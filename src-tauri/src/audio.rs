use crate::state::{process_transcript_core, AppState};
use crate::stt::{ModelManager, SpeechTranscriber};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

enum AudioCommand {
    Start {
        app_handle: AppHandle,
        device_name: Option<String>,
    },
    Stop,
}

pub struct AudioEngine {
    tx: Sender<AudioCommand>,
    is_running: Arc<AtomicBool>,
}

impl AudioEngine {
    pub fn new() -> Self {
        let (tx, rx) = channel::<AudioCommand>();
        let is_running = Arc::new(AtomicBool::new(false));
        let is_running_clone = is_running.clone();

        // Spawn dedicated audio thread where cpal::Stream and inference live
        thread::spawn(move || {
            #[allow(unused_assignments, unused_variables)]
            let mut current_stream: Option<cpal::Stream> = None;
            let mut inference_stop_flag: Option<Arc<AtomicBool>> = None;
            let mut cached_transcriber: Option<Arc<SpeechTranscriber>> = None;
            let mut cached_model_name: Option<String> = None;

            while let Ok(cmd) = rx.recv() {
                match cmd {
                    AudioCommand::Start {
                        app_handle,
                        device_name,
                    } => {
                        let _ = current_stream.take();
                        if let Some(flag) = inference_stop_flag.take() {
                            flag.store(true, Ordering::SeqCst);
                        }

                        // Determine active model from AppState
                        let target_model_name = if let Some(state) = app_handle.try_state::<AppState>() {
                            state
                                .selected_model
                                .lock()
                                .map(|m| m.clone())
                                .unwrap_or_else(|_| "ggml-base.en.bin".to_string())
                        } else {
                            "ggml-base.en.bin".to_string()
                        };

                        // Reload if transcriber not cached or user switched models
                        if cached_transcriber.is_none() || cached_model_name.as_deref() != Some(&target_model_name) {
                            let model_path = ModelManager::find_existing_model(&app_handle, &target_model_name)
                                .or_else(|| ModelManager::find_existing_model(&app_handle, "ggml-base.en.bin"))
                                .or_else(|| ModelManager::find_existing_model(&app_handle, "ggml-tiny.en.bin"))
                                .or_else(|| {
                                    ModelManager::download_model_if_missing(&app_handle, &target_model_name).ok()
                                });

                            if let Some(ref path) = model_path {
                                println!("Initializing Whisper model ({}) from {:?}", target_model_name, path);
                                if let Some(path_str) = path.to_str() {
                                    if let Ok(stt) = SpeechTranscriber::new(path_str) {
                                        cached_transcriber = Some(Arc::new(stt));
                                        cached_model_name = Some(target_model_name);
                                    } else {
                                        eprintln!("Failed to initialize Whisper context from {:?}", path_str);
                                    }
                                }
                            } else {
                                println!("Whisper model not found. Simulation mode available.");
                            }
                        }

                        let transcriber = cached_transcriber.clone();

                        let host = cpal::default_host();
                        let device = if let Some(ref name) = device_name {
                            host.input_devices()
                                .ok()
                                .and_then(|mut devs| devs.find(|d| d.name().map(|n| &n == name).unwrap_or(false)))
                        } else {
                            host.default_input_device()
                        };

                        if let Some(device) = device {
                            if let Ok(config) = device.default_input_config() {
                                let sample_rate = config.sample_rate().0 as usize;
                                let channels = config.channels() as usize;

                                let app_clone = app_handle.clone();
                                let err_fn = |err| eprintln!("Audio stream error: {}", err);

                                // 300ms pre-roll circular buffer (~4800 samples at 16kHz)
                                let preroll_capacity = 4800;
                                let preroll_buffer = Arc::new(Mutex::new(VecDeque::<f32>::with_capacity(preroll_capacity)));
                                let speech_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

                                let preroll_for_stream = preroll_buffer.clone();
                                let speech_for_stream = speech_buffer.clone();
                                let speech_for_worker = speech_buffer.clone();

                                let last_speech_instant = Arc::new(Mutex::new(Instant::now()));
                                let speech_time_for_stream = last_speech_instant.clone();
                                let speech_time_for_worker = last_speech_instant.clone();

                                let is_speaking = Arc::new(AtomicBool::new(false));
                                let is_speaking_for_stream = is_speaking.clone();
                                let is_speaking_for_worker = is_speaking.clone();

                                let resample_phase = Arc::new(Mutex::new(0.0f32));
                                let resample_phase_stream = resample_phase.clone();

                                let stop_inference = Arc::new(AtomicBool::new(false));
                                let stop_inference_clone = stop_inference.clone();
                                inference_stop_flag = Some(stop_inference);

                                // Background Whisper worker thread with Voice Activity Detection
                                if let Some(ref stt) = transcriber {
                                    let stt_clone = stt.clone();
                                    let app_worker_clone = app_handle.clone();

                                    thread::spawn(move || {
                                        while !stop_inference_clone.load(Ordering::SeqCst) {
                                            thread::sleep(Duration::from_millis(35));

                                            let mut samples_to_transcribe = Vec::new();
                                            let is_currently_speaking = is_speaking_for_worker.load(Ordering::SeqCst);

                                            if is_currently_speaking {
                                                let elapsed = speech_time_for_worker
                                                    .lock()
                                                    .map(|t| t.elapsed())
                                                    .unwrap_or_default();

                                                if let Ok(mut buf) = speech_for_worker.lock() {
                                                    // Condition 1: Natural end-of-utterance pause of >= 450ms after speaking at least 0.5s (8,000 samples)
                                                    if elapsed >= Duration::from_millis(450) && buf.len() >= 8000 {
                                                        samples_to_transcribe = buf.clone();
                                                        buf.clear();
                                                        is_speaking_for_worker.store(false, Ordering::SeqCst);
                                                    }
                                                    // Condition 2: Continuous speech threshold reduced to 2.5s (40,000 samples) for faster scripture turnaround
                                                    else if buf.len() >= 40000 {
                                                        samples_to_transcribe = buf.clone();
                                                        // Retain tail 0.75s (12,000 samples) to prevent word boundary clipping on continuous speech
                                                        let overlap = 12000.min(buf.len());
                                                        let start_idx = buf.len() - overlap;
                                                        let tail = buf[start_idx..].to_vec();
                                                        *buf = tail;
                                                    }
                                                }
                                            }

                                            if !samples_to_transcribe.is_empty() {
                                                // Gate the flush on sustained voice energy: calculate whole-buffer RMS
                                                let mut sum_sq = 0.0f32;
                                                for &s in &samples_to_transcribe {
                                                    sum_sq += s * s;
                                                }
                                                let buffer_rms = (sum_sq / samples_to_transcribe.len() as f32).sqrt();

                                                // If the accumulated buffer has voice energy (>= 0.010 RMS), transcribe it
                                                if buffer_rms >= 0.010 {
                                                    let dur_sec = samples_to_transcribe.len() as f32 / 16000.0;
                                                    println!("[Audio Engine] Transcribing {:.2}s utterance (RMS: {:.4})...", dur_sec, buffer_rms);
                                                    if let Ok(text) = stt_clone.transcribe_16khz(&samples_to_transcribe) {
                                                        let trimmed = text.trim();
                                                        // Ignore hallucinated single characters, parenthesized noise tags, or silence tokens
                                                        if !trimmed.is_empty()
                                                            && trimmed.len() >= 3
                                                            && !trimmed.starts_with('[')
                                                            && !trimmed.starts_with('(')
                                                            && !trimmed.ends_with(']')
                                                            && !trimmed.ends_with(')')
                                                        {
                                                            let words: Vec<&str> = trimmed.split_whitespace().collect();
                                                            let is_degenerate = words.len() >= 3 && words.iter().all(|&w| w.len() <= 1);

                                                            if !is_degenerate {
                                                                if let Some(state) = app_worker_clone.try_state::<AppState>() {
                                                                    if let Some(_passage) = process_transcript_core(&app_worker_clone, &state, trimmed) {
                                                                        // Scripture recognized! Clear buffer so it doesn't re-trigger
                                                                        if let Ok(mut buf) = speech_for_worker.lock() {
                                                                            buf.clear();
                                                                        }
                                                                        is_speaking_for_worker.store(false, Ordering::SeqCst);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    });
                                }

                                // Shared sample processor across F32 and I16 formats
                                let process_f32_samples = move |samples: &[f32]| {
                                    if samples.is_empty() {
                                        return;
                                    }

                                    // 1. RMS level calculation
                                    let mut sum = 0.0f32;
                                    for &sample in samples {
                                        sum += sample * sample;
                                    }
                                    let rms = (sum / (samples.len() as f32).max(1.0)).sqrt();
                                    let level = (rms * 6.0).min(1.0);
                                    let _ = app_clone.emit("audio-level", level);

                                    // 2. High-quality linear interpolated 16kHz resampler with persistent phase
                                    let frames = samples.len() / channels;
                                    let mut mono_chunk = Vec::new();
                                    if frames > 0 {
                                        let step = sample_rate as f32 / 16000.0;
                                        if let Ok(mut phase) = resample_phase_stream.lock() {
                                            while *phase < frames as f32 {
                                                let i0 = (*phase).floor() as usize;
                                                let frac = *phase - i0 as f32;
                                                let i1 = (i0 + 1).min(frames - 1);

                                                let mut s0 = 0.0f32;
                                                let mut s1 = 0.0f32;
                                                for c in 0..channels {
                                                    s0 += samples[i0 * channels + c];
                                                    s1 += samples[i1 * channels + c];
                                                }
                                                s0 /= channels as f32;
                                                s1 /= channels as f32;

                                                mono_chunk.push(s0 + frac * (s1 - s0));
                                                *phase += step;
                                            }
                                            *phase -= frames as f32;
                                        }
                                    }

                                    // 3. VAD Thresholding: calibrated for clear voice sensitivity without clipping quiet consonants
                                    let vad_active = rms > 0.012;

                                    if vad_active {
                                        if let Ok(mut t) = speech_time_for_stream.lock() {
                                            *t = Instant::now();
                                        }

                                        // If entering speech state, prepend pre-roll buffer
                                        if !is_speaking_for_stream.load(Ordering::SeqCst) {
                                            is_speaking_for_stream.store(true, Ordering::SeqCst);
                                            if let Ok(mut speech) = speech_for_stream.lock() {
                                                if let Ok(preroll) = preroll_for_stream.lock() {
                                                    speech.extend(preroll.iter());
                                                }
                                            }
                                        }
                                    }

                                    // If speech is active, accumulate samples
                                    if is_speaking_for_stream.load(Ordering::SeqCst) {
                                        if let Ok(mut speech) = speech_for_stream.lock() {
                                            speech.extend_from_slice(&mono_chunk);
                                            // Cap at 10 seconds max
                                            if speech.len() > 16000 * 10 {
                                                let excess = speech.len() - 16000 * 10;
                                                speech.drain(0..excess);
                                            }
                                        }
                                    } else {
                                        // Maintain pre-roll ring buffer during silence
                                        if let Ok(mut preroll) = preroll_for_stream.lock() {
                                            for &s in &mono_chunk {
                                                if preroll.len() >= preroll_capacity {
                                                    preroll.pop_front();
                                                }
                                                preroll.push_back(s);
                                            }
                                        }
                                    }
                                };

                                let proc_arc: Arc<dyn Fn(&[f32]) + Send + Sync> = Arc::new(process_f32_samples);

                                let stream_result = match config.sample_format() {
                                    cpal::SampleFormat::F32 => {
                                        let proc = proc_arc.clone();
                                        device.build_input_stream(
                                            &config.into(),
                                            move |data: &[f32], _| {
                                                proc(data);
                                            },
                                            err_fn,
                                            None,
                                        )
                                    }
                                    cpal::SampleFormat::I16 => {
                                        let proc = proc_arc.clone();
                                        device.build_input_stream(
                                            &config.into(),
                                            move |data: &[i16], _| {
                                                let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                                                proc(&f32_data);
                                            },
                                            err_fn,
                                            None,
                                        )
                                    }
                                    _ => {
                                        eprintln!("Unsupported audio format");
                                        continue;
                                    }
                                };

                                if let Ok(stream) = stream_result {
                                    if stream.play().is_ok() {
                                        current_stream = Some(stream);
                                        is_running_clone.store(true, Ordering::SeqCst);
                                    }
                                }
                            }
                        }
                    }
                    AudioCommand::Stop => {
                        let _ = current_stream.take();
                        if let Some(flag) = inference_stop_flag.take() {
                            flag.store(true, Ordering::SeqCst);
                        }
                        is_running_clone.store(false, Ordering::SeqCst);
                    }
                }
            }
            drop(current_stream);
        });

        Self { tx, is_running }
    }

    /// List all available audio input devices (microphones / audio interfaces)
    pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
        let host = cpal::default_host();
        let default_device_name = host.default_input_device().and_then(|d| d.name().ok());

        let mut devices = Vec::new();
        if let Ok(input_devices) = host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    let is_default = default_device_name.as_ref() == Some(&name);
                    devices.push(AudioDeviceInfo { name, is_default });
                }
            }
        }
        devices
    }

    pub fn start_listening(
        &self,
        app_handle: AppHandle,
        device_name: Option<String>,
    ) -> Result<(), String> {
        self.tx
            .send(AudioCommand::Start {
                app_handle,
                device_name,
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn stop(&self) {
        let _ = self.tx.send(AudioCommand::Stop);
    }

    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}
