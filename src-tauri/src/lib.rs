mod audio;
mod commands;
mod db;
mod llm;
mod system_audio;
mod whisper;

use audio::AudioRecorder;
use db::init_default_settings;
use serde::Serialize;
use std::sync::Mutex;
use tauri::Manager;

/// Extended app state that includes audio recording
pub struct AppStateExt {
    pub db: sqlx::Pool<sqlx::Sqlite>,
    pub audio_recorder: Mutex<AudioRecorder>,
}

/// Response wrapper for consistent API responses
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use commands::{
        audio::*, llm::*, meetings::*, settings::*, summaries::*, transcription::*, transcripts::*,
    };

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let db_pool = db::init_db(&app_handle)
                    .await
                    .map_err(|e| format!("Failed to initialize database: {}", e))?;

                init_default_settings(&db_pool)
                    .await
                    .map_err(|e| format!("Failed to initialize default settings: {}", e))?;

                app_handle.manage(AppStateExt {
                    db: db_pool,
                    audio_recorder: Mutex::new(AudioRecorder::new()),
                });
                Ok::<(), String>(())
            })
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Meetings
            create_meeting_command,
            get_meeting_command,
            list_meetings_command,
            delete_meeting_command,
            update_meeting_command,
            // Transcripts
            create_transcript_command,
            get_transcript_command,
            get_transcript_by_meeting_command,
            list_transcripts_command,
            update_transcript_command,
            delete_transcript_command,
            // Summaries
            create_summary_command,
            get_summary_command,
            get_summary_by_meeting_command,
            list_summaries_command,
            update_summary_command,
            delete_summary_command,
            // Settings
            get_setting_command,
            set_setting_command,
            list_settings_command,
            delete_setting_command,
            // Audio & devices
            start_recording_command,
            stop_recording_command,
            list_audio_devices_command,
            test_microphone_command,
            check_blackhole_status_command,
            install_blackhole_command,
            check_homebrew_status_command,
            install_blackhole_homebrew_command,
            install_blackhole_bundled_command,
            auto_install_blackhole_command,
            complete_first_launch_setup_command,
            check_first_launch_status_command,
            // Whisper models & transcription
            check_whisper_model_command,
            download_whisper_model_command,
            list_whisper_models_command,
            open_models_folder_command,
            transcribe_audio_command,
            // LLM / summaries
            check_ollama_status_command,
            generate_summary_command,
        ]);

    if let Err(err) = app.run(tauri::generate_context!()) {
        log::error!("Error while running Tauri application: {}", err);
    }
}
