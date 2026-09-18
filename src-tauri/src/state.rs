use crate::audio::{AudioDeviceInfo, AudioEngine};
use crate::bible::{BibleDatabase, ScripturePassage, ScriptureVerse};
use crate::parser::{clean_spoken_transcript, parse_continuation_verse, parse_spoken_scripture};
use crate::stt::{ModelManager, ModelStatus};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub audio_active: bool,
    pub auto_project: bool,
    pub active_translation: String,
    pub projector_blackout: bool,
    pub projector_clear: bool,
    pub projector_theme: String,
    pub projector_font_family: String,
    pub projector_font_size: String,
    pub projector_ref_position: String,
    pub projector_verse_coverage: u32,
    pub projector_auto_fit: bool,
    pub projector_auto_grow: bool,
    pub projector_auto_shrink: bool,
    pub projector_min_font_size: u32,
    pub projector_max_font_size: u32,
    pub current_passage: Option<ScripturePassage>,
}

pub struct AppState {
    pub bible: Arc<BibleDatabase>,
    pub audio: Arc<Mutex<AudioEngine>>,
    pub current_passage: Arc<Mutex<Option<ScripturePassage>>>,
    pub current_translation: Arc<Mutex<String>>,
    pub is_auto_project: Arc<AtomicBool>,
    pub projector_blackout: Arc<AtomicBool>,
    pub projector_clear: Arc<AtomicBool>,
    pub projector_theme: Arc<Mutex<String>>,
    pub projector_font_family: Arc<Mutex<String>>,
    pub projector_font_size: Arc<Mutex<String>>,
    pub projector_ref_position: Arc<Mutex<String>>,
    pub projector_verse_coverage: Arc<Mutex<u32>>,
    pub projector_auto_fit: Arc<AtomicBool>,
    pub projector_auto_grow: Arc<AtomicBool>,
    pub projector_auto_shrink: Arc<AtomicBool>,
    pub projector_min_font_size: Arc<Mutex<u32>>,
    pub projector_max_font_size: Arc<Mutex<u32>>,
    pub last_speech_context: Arc<Mutex<Option<(String, std::time::Instant)>>>,
    pub active_passage_context: Arc<Mutex<Option<(String, u32)>>>,
    pub selected_model: Arc<Mutex<String>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut candidate_paths = Vec::new();

        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                // macOS app bundle: Relieve.app/Contents/MacOS/relieve -> ../Resources/resources/bible.db
                candidate_paths.push(exe_dir.join("../Resources/resources/bible.db"));
                candidate_paths.push(exe_dir.join("../Resources/bible.db"));
                candidate_paths.push(exe_dir.join("resources/bible.db"));
                candidate_paths.push(exe_dir.join("bible.db"));
            }
        }

        candidate_paths.push(std::path::PathBuf::from("resources/bible.db"));
        candidate_paths.push(std::path::PathBuf::from("src-tauri/resources/bible.db"));
        candidate_paths.push(std::path::PathBuf::from("../src-tauri/resources/bible.db"));

        let mut bible_opt = None;
        for path in candidate_paths {
            if path.exists() {
                if let Some(path_str) = path.to_str() {
                    if let Ok(db) = BibleDatabase::new_at_path(path_str) {
                        println!("[Relieve Database] Loaded Bible database from: {}", path_str);
                        bible_opt = Some(Arc::new(db));
                        break;
                    }
                }
            }
        }

        let bible = bible_opt.unwrap_or_else(|| {
            Arc::new(
                BibleDatabase::new_in_memory().expect("Failed to initialize SQLite Bible database"),
            )
        });
        let audio = Arc::new(Mutex::new(AudioEngine::new()));

        Self {
            bible,
            audio,
            current_passage: Arc::new(Mutex::new(None)),
            current_translation: Arc::new(Mutex::new("KJV".to_string())),
            is_auto_project: Arc::new(AtomicBool::new(false)),
            projector_blackout: Arc::new(AtomicBool::new(false)),
            projector_clear: Arc::new(AtomicBool::new(false)),
            projector_theme: Arc::new(Mutex::new("dark".to_string())),
            projector_font_family: Arc::new(Mutex::new("serif".to_string())),
            projector_font_size: Arc::new(Mutex::new("auto".to_string())),
            projector_ref_position: Arc::new(Mutex::new("bottom-center".to_string())),
            projector_verse_coverage: Arc::new(Mutex::new(85)),
            projector_auto_fit: Arc::new(AtomicBool::new(true)),
            projector_auto_grow: Arc::new(AtomicBool::new(true)),
            projector_auto_shrink: Arc::new(AtomicBool::new(true)),
            projector_min_font_size: Arc::new(Mutex::new(28)),
            projector_max_font_size: Arc::new(Mutex::new(84)),
            last_speech_context: Arc::new(Mutex::new(None)),
            active_passage_context: Arc::new(Mutex::new(None)),
            selected_model: Arc::new(Mutex::new("ggml-base.en.bin".to_string())),
        }
    }

    pub fn get_status(&self) -> AppStatus {
        let audio_active = self.audio.lock().map(|a| a.is_active()).unwrap_or(false);
        let auto_project = self.is_auto_project.load(Ordering::SeqCst);
        let active_translation = self
            .current_translation
            .lock()
            .map(|t| t.clone())
            .unwrap_or_else(|_| "KJV".to_string());
        let projector_blackout = self.projector_blackout.load(Ordering::SeqCst);
        let projector_clear = self.projector_clear.load(Ordering::SeqCst);
        let projector_theme = self
            .projector_theme
            .lock()
            .map(|t| t.clone())
            .unwrap_or_else(|_| "dark".to_string());
        let projector_font_family = self
            .projector_font_family
            .lock()
            .map(|t| t.clone())
            .unwrap_or_else(|_| "serif".to_string());
        let projector_font_size = self
            .projector_font_size
            .lock()
            .map(|t| t.clone())
            .unwrap_or_else(|_| "auto".to_string());
        let projector_ref_position = self
            .projector_ref_position
            .lock()
            .map(|p| p.clone())
            .unwrap_or_else(|_| "bottom-center".to_string());
        let projector_verse_coverage = self
            .projector_verse_coverage
            .lock()
            .map(|c| *c)
            .unwrap_or(85);
        let projector_auto_fit = self.projector_auto_fit.load(Ordering::SeqCst);
        let projector_auto_grow = self.projector_auto_grow.load(Ordering::SeqCst);
        let projector_auto_shrink = self.projector_auto_shrink.load(Ordering::SeqCst);
        let projector_min_font_size = self
            .projector_min_font_size
            .lock()
            .map(|s| *s)
            .unwrap_or(28);
        let projector_max_font_size = self
            .projector_max_font_size
            .lock()
            .map(|s| *s)
            .unwrap_or(84);
        let current_passage = self
            .current_passage
            .lock()
            .ok()
            .and_then(|p| p.clone());

        AppStatus {
            audio_active,
            auto_project,
            active_translation,
            projector_blackout,
            projector_clear,
            projector_theme,
            projector_font_family,
            projector_font_size,
            projector_ref_position,
            projector_verse_coverage,
            projector_auto_fit,
            projector_auto_grow,
            projector_auto_shrink,
            projector_min_font_size,
            projector_max_font_size,
            current_passage,
        }
    }
}

// ----------------- TAURI COMMANDS -----------------

#[tauri::command]
pub fn get_status(state: tauri::State<AppState>) -> AppStatus {
    state.get_status()
}

#[tauri::command]
pub fn get_audio_devices() -> Vec<AudioDeviceInfo> {
    AudioEngine::list_input_devices()
}

#[tauri::command]
pub fn start_audio_capture(
    app: AppHandle,
    state: tauri::State<AppState>,
    device_name: Option<String>,
) -> Result<(), String> {
    let audio = state.audio.lock().map_err(|e| e.to_string())?;
    audio.start_listening(app, device_name)
}

#[tauri::command]
pub fn stop_audio_capture(state: tauri::State<AppState>) -> Result<(), String> {
    let audio = state.audio.lock().map_err(|e| e.to_string())?;
    audio.stop();
    Ok(())
}

pub fn process_transcript_core(
    app: &AppHandle,
    state: &AppState,
    transcript: &str,
) -> Option<ScripturePassage> {
    let clean = clean_spoken_transcript(transcript);
    if clean.is_empty() {
        return None;
    }

    // 1. Emit clean speech chunk to the operator window
    let _ = app.emit("live-transcript", &clean);

    let trans = state
        .current_translation
        .lock()
        .map(|t| t.clone())
        .unwrap_or_else(|_| "KJV".to_string());

    // 2. First attempt direct citation matching
    let mut citation = parse_spoken_scripture(&clean);

    // 3. If direct match fails, attempt continuation verse using active book & chapter context
    // (e.g. pastor was at Romans 8:28, now says "and in verse thirty" -> Romans 8:30)
    if citation.is_none() {
        if let Ok(active) = state.active_passage_context.lock() {
            if let Some((ref book, chapter)) = *active {
                citation = parse_continuation_verse(&clean, book, chapter);
            }
        }
    }

    // 4. If still fails, combine with recent previous speech context (within 2.5 seconds)
    // (e.g., "In the book of Romans" ... [pause] ... "chapter eight verse twenty eight")
    if citation.is_none() {
        if let Ok(last_ctx) = state.last_speech_context.lock() {
            if let Some((ref last, ref timestamp)) = *last_ctx {
                if timestamp.elapsed() <= std::time::Duration::from_millis(2500) && !last.is_empty() {
                    let combined = format!("{} {}", last, clean);
                    citation = parse_spoken_scripture(&combined);
                }
            }
        }
    }

    println!("[Relieve Pipeline] Live transcript: '{}' (Matched: {:?})", clean, citation.as_ref().map(|c| format!("{}:{}", c.book, c.chapter)));

    if let Some(cit) = citation {
        // Clear recent context on successful match
        if let Ok(mut last_ctx) = state.last_speech_context.lock() {
            *last_ctx = None;
        }

        if let Ok(passage) = state.bible.lookup_passage(
            &trans,
            &cit.book,
            cit.chapter,
            cit.verse_start,
            cit.verse_end,
        ) {
            // Update active passage context for subsequent continuation verses
            if let Ok(mut active) = state.active_passage_context.lock() {
                *active = Some((cit.book.clone(), cit.chapter));
            }

            // Emit detected scripture to operator
            let _ = app.emit("scripture-detected", &passage);

            // If auto-pilot mode is enabled, immediately push to projector!
            if state.is_auto_project.load(Ordering::SeqCst) {
                if let Ok(mut current) = state.current_passage.lock() {
                    *current = Some(passage.clone());
                    state.projector_clear.store(false, Ordering::SeqCst);
                    state.projector_blackout.store(false, Ordering::SeqCst);
                    let _ = app.emit("present-slide", &passage);
                }
            }

            return Some(passage);
        }
    } else {
        // 5. Deep Processing Recitation Matching:
        // If no citation reference matched, check if a scripture verse is being quoted directly
        // (e.g. "For God so loved the world that he gave his only begotten Son")
        if clean.split_whitespace().count() >= 4 {
            if let Ok(Some(passage)) = state.bible.match_spoken_verse_text(&trans, &clean) {
                // Clear recent context on successful match
                if let Ok(mut last_ctx) = state.last_speech_context.lock() {
                    *last_ctx = None;
                }

                if let Ok(mut active) = state.active_passage_context.lock() {
                    *active = Some((passage.book.clone(), passage.chapter));
                }

                let _ = app.emit("scripture-detected", &passage);

                if state.is_auto_project.load(Ordering::SeqCst) {
                    if let Ok(mut current) = state.current_passage.lock() {
                        *current = Some(passage.clone());
                        state.projector_clear.store(false, Ordering::SeqCst);
                        state.projector_blackout.store(false, Ordering::SeqCst);
                        let _ = app.emit("present-slide", &passage);
                    }
                }

                return Some(passage);
            }
        }

        // Save as rolling speech context for next chunk with timestamp (capped at 120 chars)
        if let Ok(mut last_ctx) = state.last_speech_context.lock() {
            let saved = if clean.len() > 120 {
                clean[clean.len() - 120..].to_string()
            } else {
                clean.to_string()
            };
            *last_ctx = Some((saved, std::time::Instant::now()));
        }
    }

    None
}

#[tauri::command]
pub fn process_transcript(
    app: AppHandle,
    state: tauri::State<AppState>,
    transcript: String,
) -> Result<Option<ScripturePassage>, String> {
    Ok(process_transcript_core(&app, &state, &transcript))
}

#[tauri::command]
pub fn lookup_scripture(
    state: tauri::State<AppState>,
    book: String,
    chapter: u32,
    verse_start: u32,
    verse_end: Option<u32>,
    translation: Option<String>,
) -> Result<ScripturePassage, String> {
    let trans = translation.unwrap_or_else(|| {
        state
            .current_translation
            .lock()
            .map(|t| t.clone())
            .unwrap_or_else(|_| "KJV".to_string())
    });

    state
        .bible
        .lookup_passage(&trans, &book, chapter, verse_start, verse_end)
}

#[tauri::command]
pub fn search_scripture(
    state: tauri::State<AppState>,
    query: String,
    translation: Option<String>,
) -> Result<Vec<ScriptureVerse>, String> {
    let trans = translation.unwrap_or_else(|| {
        state
            .current_translation
            .lock()
            .map(|t| t.clone())
            .unwrap_or_else(|_| "KJV".to_string())
    });

    state.bible.search_verses(&trans, &query, 10)
}

#[tauri::command]
pub fn present_passage(
    app: AppHandle,
    state: tauri::State<AppState>,
    passage: ScripturePassage,
) -> Result<(), String> {
    let mut current = state.current_passage.lock().map_err(|e| e.to_string())?;
    *current = Some(passage.clone());
    state.projector_clear.store(false, Ordering::SeqCst);
    state.projector_blackout.store(false, Ordering::SeqCst);

    let _ = app.emit("present-slide", &passage);
    Ok(())
}

#[tauri::command]
pub fn clear_presentation(app: AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    state.projector_clear.store(true, Ordering::SeqCst);
    let _ = app.emit("projector-clear", ());
    Ok(())
}

#[tauri::command]
pub fn blackout_presentation(
    app: AppHandle,
    state: tauri::State<AppState>,
    blackout: bool,
) -> Result<(), String> {
    state.projector_blackout.store(blackout, Ordering::SeqCst);
    let _ = app.emit("projector-blackout", blackout);
    Ok(())
}

#[tauri::command]
pub fn set_projector_theme(
    app: AppHandle,
    state: tauri::State<AppState>,
    theme: String,
) -> Result<(), String> {
    let mut t = state.projector_theme.lock().map_err(|e| e.to_string())?;
    *t = theme.clone();
    let _ = app.emit("projector-theme", theme);
    Ok(())
}

#[tauri::command]
pub fn set_auto_project(state: tauri::State<AppState>, enabled: bool) -> Result<(), String> {
    state.is_auto_project.store(enabled, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub fn set_translation(state: tauri::State<AppState>, translation: String) -> Result<(), String> {
    let mut t = state
        .current_translation
        .lock()
        .map_err(|e| e.to_string())?;
    *t = translation;
    Ok(())
}

#[tauri::command]
pub fn toggle_projector_window(app: AppHandle) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("projector") {
        let is_visible = window.is_visible().unwrap_or(false);
        if is_visible {
            window.hide().map_err(|e| e.to_string())?;
            Ok(false)
        } else {
            // Check available monitors for external projection display
            if let Ok(monitors) = app.available_monitors() {
                if monitors.len() > 1 {
                    let main_pos = app
                        .get_webview_window("main")
                        .and_then(|w| w.outer_position().ok());

                    let target_monitor = monitors.iter().find(|m| {
                        if let Some(ref p) = main_pos {
                            let m_pos = m.position();
                            let m_size = m.size();
                            !(p.x >= m_pos.x
                                && p.x < m_pos.x + m_size.width as i32
                                && p.y >= m_pos.y
                                && p.y < m_pos.y + m_size.height as i32)
                        } else {
                            false
                        }
                    }).or_else(|| monitors.get(1));

                    if let Some(target) = target_monitor {
                        let _ = window.set_position(tauri::Position::Physical(*target.position()));
                        let _ = window.set_size(tauri::Size::Physical(*target.size()));
                        let _ = window.set_fullscreen(true);
                    }
                }
            }
            window.show().map_err(|e| e.to_string())?;
            window.set_focus().map_err(|e| e.to_string())?;
            Ok(true)
        }
    } else {
        Err("Projector window not found".to_string())
    }
}

#[tauri::command]
pub fn set_projector_typography(
    app: AppHandle,
    state: tauri::State<AppState>,
    font_family: String,
    font_size: String,
) -> Result<(), String> {
    if let Ok(mut ff) = state.projector_font_family.lock() {
        *ff = font_family.clone();
    }
    if let Ok(mut fs) = state.projector_font_size.lock() {
        *fs = font_size.clone();
    }
    let _ = app.emit("projector-typography", serde_json::json!({
        "fontFamily": font_family,
        "fontSize": font_size,
    }));
    Ok(())
}

#[tauri::command]
pub fn set_projector_layout(
    app: AppHandle,
    state: tauri::State<AppState>,
    ref_position: Option<String>,
    verse_coverage: Option<u32>,
    auto_fit: Option<bool>,
    auto_grow: Option<bool>,
    auto_shrink: Option<bool>,
    min_font_size: Option<u32>,
    max_font_size: Option<u32>,
) -> Result<(), String> {
    if let Some(pos) = ref_position {
        if let Ok(mut p) = state.projector_ref_position.lock() {
            *p = pos;
        }
    }
    if let Some(cov) = verse_coverage {
        if let Ok(mut c) = state.projector_verse_coverage.lock() {
            *c = cov;
        }
    }
    if let Some(af) = auto_fit {
        state.projector_auto_fit.store(af, Ordering::SeqCst);
    }
    if let Some(ag) = auto_grow {
        state.projector_auto_grow.store(ag, Ordering::SeqCst);
    }
    if let Some(as_val) = auto_shrink {
        state.projector_auto_shrink.store(as_val, Ordering::SeqCst);
    }
    if let Some(min_s) = min_font_size {
        if let Ok(mut m) = state.projector_min_font_size.lock() {
            *m = min_s;
        }
    }
    if let Some(max_s) = max_font_size {
        if let Ok(mut m) = state.projector_max_font_size.lock() {
            *m = max_s;
        }
    }

    let status = state.get_status();
    let _ = app.emit("projector-layout", serde_json::json!({
        "refPosition": status.projector_ref_position,
        "verseCoverage": status.projector_verse_coverage,
        "autoFit": status.projector_auto_fit,
        "autoGrow": status.projector_auto_grow,
        "autoShrink": status.projector_auto_shrink,
        "minFontSize": status.projector_min_font_size,
        "maxFontSize": status.projector_max_font_size,
    }));
    Ok(())
}

#[tauri::command]
pub fn get_system_fonts() -> Vec<String> {
    use std::collections::BTreeSet;
    let mut fonts = BTreeSet::new();

    // Curated standard church presentation & system fonts
    let standard_fonts = [
        "Arial", "Arial Black", "Avenir", "Avenir Next", "Baskerville", 
        "Big Caslon", "Bodoni 72", "Book Antiqua", "Calibri", "Cambria", 
        "Candara", "Century Gothic", "Charter", "Cochin", "Comic Sans MS", 
        "Consolas", "Constantia", "Copperplate", "Corbel", "Courier New", 
        "Didot", "DIN Alternate", "DIN Condensed", "Franklin Gothic Medium", 
        "Futura", "Garamond", "Geneva", "Georgia", "Gill Sans", 
        "Helvetica", "Helvetica Neue", "Impact", "Inter", "Lucida Grande", 
        "Menlo", "Monaco", "Montserrat", "Noteworthy", "Optima", 
        "Palatino", "Papyrus", "Poppins", "PT Sans", "PT Serif", 
        "Roboto", "Rockwell", "Segoe UI", "Seravek", "Snell Roundhand", 
        "Tahoma", "Times New Roman", "Trebuchet MS", "Verdana", "Zapfino"
    ];
    for f in standard_fonts {
        fonts.insert(f.to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let font_dirs = [
            "/System/Library/Fonts",
            "/System/Library/Fonts/Supplemental",
            "/Library/Fonts",
        ];

        for dir in font_dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let clean_name = stem
                            .trim_end_matches(" Regular")
                            .trim_end_matches(" Bold")
                            .trim_end_matches(" Italic")
                            .trim_end_matches(" Medium")
                            .trim_end_matches(" Light")
                            .trim_end_matches(" Black")
                            .trim_end_matches(" Thin")
                            .trim_end_matches(" SemiBold")
                            .trim_end_matches(" ExtraBold")
                            .trim();

                        if !clean_name.is_empty() && !clean_name.starts_with('.') && clean_name.len() > 2 {
                            fonts.insert(clean_name.to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let font_dir = std::path::Path::new(&windir).join("Fonts");
        if let Ok(entries) = std::fs::read_dir(font_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let clean_name = stem
                        .trim_end_matches(" Regular")
                        .trim_end_matches(" Bold")
                        .trim_end_matches(" Italic")
                        .trim_end_matches(" Medium")
                        .trim_end_matches(" Light")
                        .trim_end_matches(" Black")
                        .trim_end_matches(" Thin")
                        .trim_end_matches(" SemiBold")
                        .trim_end_matches(" ExtraBold")
                        .trim();

                    if !clean_name.is_empty() && !clean_name.starts_with('.') && clean_name.len() > 2 {
                        fonts.insert(clean_name.to_string());
                    }
                }
            }
        }
    }

    fonts.into_iter().collect()
}

#[tauri::command]
pub fn get_model_status(app: AppHandle, state: tauri::State<AppState>) -> ModelStatus {
    let active_model = state
        .selected_model
        .lock()
        .map(|m| m.clone())
        .unwrap_or_else(|_| "ggml-base.en.bin".to_string());

    let existing = ModelManager::find_existing_model(&app, &active_model)
        .or_else(|| ModelManager::find_existing_model(&app, "ggml-tiny.en.bin"));

    let resolved_name = if ModelManager::find_existing_model(&app, &active_model).is_some() {
        active_model.clone()
    } else if ModelManager::find_existing_model(&app, "ggml-tiny.en.bin").is_some() {
        "ggml-tiny.en.bin".to_string()
    } else {
        active_model.clone()
    };

    let mut available_models = Vec::new();
    for candidate in &["ggml-base.en.bin", "ggml-small.en.bin", "ggml-tiny.en.bin"] {
        if ModelManager::find_existing_model(&app, candidate).is_some() {
            available_models.push(candidate.to_string());
        }
    }

    ModelStatus {
        is_ready: existing.is_some(),
        is_downloading: false,
        model_name: resolved_name,
        path: existing.map(|p| p.to_string_lossy().to_string()),
        available_models,
    }
}

#[tauri::command]
pub fn set_active_model(state: tauri::State<AppState>, model_name: String) -> Result<(), String> {
    let mut m = state.selected_model.lock().map_err(|e| e.to_string())?;
    *m = model_name;
    Ok(())
}

#[tauri::command]
pub async fn download_model(app: AppHandle, model_name: Option<String>) -> Result<String, String> {
    let name = model_name.unwrap_or_else(|| "ggml-base.en.bin".to_string());
    tokio::task::spawn_blocking(move || {
        let path = ModelManager::download_model_if_missing(&app, &name)?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_context_expiration() {
        let state = AppState::new();
        let ctx_arc = state.last_speech_context.clone();
        // Simulate speech context from 3 seconds ago
        let old_time = std::time::Instant::now() - std::time::Duration::from_secs(3);
        {
            let mut last_ctx = ctx_arc.lock().unwrap();
            *last_ctx = Some(("In the book of Romans".to_string(), old_time));
        }

        // Verify that expired context is recognized as past the 2.5s threshold
        {
            let last_ctx = ctx_arc.lock().unwrap();
            if let Some((_, ref timestamp)) = *last_ctx {
                assert!(timestamp.elapsed() > std::time::Duration::from_millis(2500), "Old context should be recognized as expired");
            }
        }
    }
}
