pub mod audio;
pub mod bible;
pub mod parser;
pub mod state;
pub mod stt;

use state::{
    blackout_presentation, clear_presentation, download_model, get_audio_devices, get_model_status,
    get_status, get_system_fonts, lookup_scripture, present_passage, process_transcript, search_scripture,
    set_active_model, set_auto_project, set_projector_layout, set_projector_theme, set_projector_typography, set_translation, start_audio_capture,
    stop_audio_capture, toggle_projector_window, AppState,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_audio_devices,
            get_model_status,
            get_system_fonts,
            set_active_model,
            download_model,
            start_audio_capture,
            stop_audio_capture,
            process_transcript,
            lookup_scripture,
            search_scripture,
            present_passage,
            clear_presentation,
            blackout_presentation,
            set_projector_theme,
            set_projector_typography,
            set_projector_layout,
            set_auto_project,
            set_translation,
            toggle_projector_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
