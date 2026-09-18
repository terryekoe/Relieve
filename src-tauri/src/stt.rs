use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub is_ready: bool,
    pub is_downloading: bool,
    pub model_name: String,
    pub path: Option<String>,
    pub available_models: Vec<String>,
}

pub struct SpeechTranscriber {
    _ctx: WhisperContext,
    state: Mutex<WhisperState>,
}

impl SpeechTranscriber {
    pub fn new_from_paths(possible_paths: &[&Path]) -> Result<Self, String> {
        for path in possible_paths {
            if path.exists() {
                if let Some(path_str) = path.to_str() {
                    return Self::new(path_str);
                }
            }
        }
        Err(format!(
            "No Whisper model found at any of the paths: {:?}",
            possible_paths
        ))
    }

    pub fn new(model_path: &str) -> Result<Self, String> {
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(model_path, params)
            .map_err(|e| format!("Failed to load Whisper model from {}: {:?}", model_path, e))?;
        let state = ctx
            .create_state()
            .map_err(|e| format!("Failed to initialize Whisper state: {:?}", e))?;
        Ok(Self {
            _ctx: ctx,
            state: Mutex::new(state),
        })
    }

    pub fn transcribe_16khz(&self, audio_samples: &[f32]) -> Result<String, String> {
        if audio_samples.is_empty() {
            return Ok(String::new());
        }

        // 1. Dual RMS & Peak silence gate:
        // Calculate both energy metrics over the entire audio buffer
        let mut sum_sq = 0.0f32;
        let mut peak = 0.0f32;
        for &s in audio_samples {
            let abs = s.abs();
            if abs > peak {
                peak = abs;
            }
            sum_sq += s * s;
        }
        let rms = (sum_sq / audio_samples.len() as f32).sqrt();

        // Real human speech into a mic has sustained RMS >= 0.014 and Peak >= 0.035.
        // If the buffer is just ambient room hiss, breathing, or fan noise, drop it immediately
        // without waking Whisper.
        if rms < 0.014 || peak < 0.035 {
            return Ok(String::new());
        }

        // 2. Dynamic range normalization: boost soft mic speech to healthy Whisper operating amplitude
        // Cap gain at 3.0x max to avoid inflating low-level background noise into high-amplitude distortion
        let normalized: Vec<f32> = if peak < 0.60 {
            let gain = (0.65 / peak).clamp(1.0, 3.0);
            audio_samples.iter().map(|&s| (s * gain).clamp(-1.0, 1.0)).collect()
        } else {
            audio_samples.to_vec()
        };

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        let threads = std::thread::available_parallelism()
            .map(|p| p.get().clamp(2, 6))
            .unwrap_or(4);

        params.set_n_threads(threads as i32);
        params.set_language(Some("en"));
        params.set_translate(false);
        params.set_no_context(true);
        // CRUCIAL: Allow multi-segment decoding so full phrases are never truncated
        params.set_single_segment(false);
        params.set_max_tokens(128);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        // Lock temperature to 0.0 and disable progressive loosening/fallback retries
        params.set_temperature(0.0);
        params.set_temperature_inc(0.0);
        // Aggressive no-speech threshold (0.65) to suppress decoding on ambient room noise
        params.set_no_speech_thold(0.65);

        // Domain conditioning prompt: primes Whisper on Bible citation syntax WITHOUT priming
        // specific book names or numbers (prevents hallucinating 'Psalm 23' on quiet audio)
        params.set_initial_prompt("The following is a Scripture reading, church sermon, chapter, and verse citations: Book chapter, verses, numbers.");

        let mut state = self
            .state
            .lock()
            .map_err(|e| format!("Whisper state lock error: {}", e))?;

        state
            .full(params, &normalized)
            .map_err(|e| format!("{:?}", e))?;

        let num_segments = state.full_n_segments();
        let mut text = String::new();
        for i in 0..num_segments {
            if let Some(seg) = state.get_segment(i) {
                // Whisper produces an explicit no-speech probability per segment.
                // If Whisper is not confident that speech was actually present (> 0.40), discard the segment!
                if seg.no_speech_probability() > 0.40 {
                    continue;
                }
                if let Ok(s) = seg.to_str_lossy() {
                    text.push_str(&s);
                    text.push(' ');
                }
            }
        }

        let mut clean = text.trim();

        // 3. Hallucination guards: reject non-speech bracketed descriptions like (waves crashing), [music], etc.
        if (clean.starts_with('(') && clean.ends_with(')'))
            || (clean.starts_with('[') && clean.ends_with(']'))
            || (clean.starts_with('*') && clean.ends_with('*'))
        {
            return Ok(String::new());
        }

        // Reject common Whisper silence hallucinations
        let lower_clean = clean.to_lowercase();
        let stripped_clean = lower_clean.trim_matches(|c: char| !c.is_alphanumeric());
        if matches!(
            stripped_clean,
            "thank you"
                | "thank you very much"
                | "thank you for watching"
                | "thanks for watching"
                | "silence"
                | "subtitles by"
                | "subscribe"
                | "you"
                | "bye"
                | "goodbye"
                | "the end"
                | "watching"
        ) {
            return Ok(String::new());
        }

        // Strip inline bracketed tokens if any leaked through
        let mut cleaned_str = clean.to_string();
        if cleaned_str.contains('(') || cleaned_str.contains('[') {
            let re_brackets = regex::Regex::new(r"\[.*?\]|\(.*?\)")
                .map_err(|e| e.to_string())?;
            cleaned_str = re_brackets.replace_all(&cleaned_str, "").trim().to_string();
            clean = &cleaned_str;
        }

        // Reject degenerate single-character repetitions (e.g. "p f f f f f")
        let words: Vec<&str> = clean.split_whitespace().collect();
        if words.len() >= 3 && words.iter().all(|&w| w.len() <= 1) {
            return Ok(String::new());
        }

        // Reject if less than 2 alphabetic characters
        let alpha_count = clean.chars().filter(|c| c.is_alphabetic()).count();
        if alpha_count < 2 {
            return Ok(String::new());
        }

        println!("[Whisper Output] Clean: '{}'", clean);
        Ok(clean.to_string())
    }
}

pub struct ModelManager;

impl ModelManager {
    /// Return potential paths to the model in priority order:
    /// 1. App data directory (`$APP_DATA/models/ggml-tiny.en.bin`)
    /// 2. Bundled application resources (`resources/models/...`)
    pub fn get_candidate_paths(app: &AppHandle, model_name: &str) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. App Data directory
        if let Ok(app_dir) = app.path().app_data_dir() {
            paths.push(app_dir.join("models").join(model_name));
        }

        // 2. Bundled application resource directory
        if let Ok(res_dir) = app.path().resource_dir() {
            paths.push(res_dir.join("resources/models").join(model_name));
            paths.push(res_dir.join("models").join(model_name));
        }

        // 3. Relative project/bundle locations
        paths.push(PathBuf::from("resources/models").join(model_name));
        paths.push(PathBuf::from("src-tauri/resources/models").join(model_name));
        paths.push(PathBuf::from("../src-tauri/resources/models").join(model_name));

        paths
    }

    /// Check if the model is locally available
    pub fn find_existing_model(app: &AppHandle, model_name: &str) -> Option<PathBuf> {
        for path in Self::get_candidate_paths(app, model_name) {
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    /// Download model from Hugging Face if not already present
    pub fn download_model_if_missing(app: &AppHandle, model_name: &str) -> Result<PathBuf, String> {
        if let Some(existing) = Self::find_existing_model(app, model_name) {
            return Ok(existing);
        }

        let app_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;
        let models_dir = app_dir.join("models");
        create_dir_all(&models_dir).map_err(|e| format!("Failed to create models directory: {}", e))?;

        let dest_path = models_dir.join(model_name);
        let temp_path = models_dir.join(format!("{}.download", model_name));

        let download_url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
            model_name
        );

        println!("Downloading Whisper model from {}...", download_url);

        let resp = ureq::get(&download_url)
            .call()
            .map_err(|e| format!("Download request failed: {}", e))?;

        let total_size: u64 = resp
            .header("content-length")
            .and_then(|l| l.parse().ok())
            .unwrap_or(77_000_000);

        let mut reader = resp.into_reader();
        let mut file = File::create(&temp_path)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;

        let mut buffer = [0u8; 65536];
        let mut downloaded = 0u64;
        let mut last_emit_percent = 0;

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .map_err(|e| format!("Error reading download stream: {}", e))?;
            if bytes_read == 0 {
                break;
            }
            file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Error writing model file: {}", e))?;

            downloaded += bytes_read as u64;
            let percent = ((downloaded as f64 / total_size.max(1) as f64) * 100.0) as u32;

            if percent != last_emit_percent {
                last_emit_percent = percent;
                let _ = app.emit(
                    "model-download-progress",
                    serde_json::json!({
                        "model": model_name,
                        "percent": percent,
                        "downloaded": downloaded,
                        "total": total_size
                    }),
                );
            }
        }

        file.flush().map_err(|e| e.to_string())?;
        drop(file);

        // Rename temp file to final destination
        std::fs::rename(&temp_path, &dest_path)
            .map_err(|e| format!("Failed to rename downloaded model: {}", e))?;

        let _ = app.emit(
            "model-download-progress",
            serde_json::json!({
                "model": model_name,
                "percent": 100,
                "downloaded": total_size,
                "total": total_size,
                "complete": true
            }),
        );

        println!("Whisper model saved to {:?}", dest_path);
        Ok(dest_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_and_run_transcriber() {
        let possible_paths = [
            Path::new("resources/models/ggml-tiny.en.bin"),
            Path::new("src-tauri/resources/models/ggml-tiny.en.bin"),
            Path::new("../src-tauri/resources/models/ggml-tiny.en.bin"),
        ];
        if let Ok(stt) = SpeechTranscriber::new_from_paths(&possible_paths) {
            // 1. True silence should be rejected by the RMS gate without running Whisper
            let silence = vec![0.0f32; 16000];
            let res_silence = stt.transcribe_16khz(&silence).expect("Should succeed");
            assert_eq!(res_silence, "", "Silence must return empty transcript");

            // 2. Ambient room hiss / fan noise (peak ~0.008, RMS ~0.005) must also be rejected
            let hiss = vec![0.006f32; 16000];
            let res_hiss = stt.transcribe_16khz(&hiss).expect("Should succeed");
            assert_eq!(res_hiss, "", "Low-level noise floor must return empty transcript");

            // 3. Moderate room ambient noise / breathing (peak ~0.025, RMS ~0.012) must be dropped by pre-inference gate
            let room_noise = vec![0.012f32; 16000];
            let res_room = stt.transcribe_16khz(&room_noise).expect("Should succeed");
            assert_eq!(res_room, "", "Moderate ambient noise must return empty transcript without triggering Whisper");
        }
    }
}
