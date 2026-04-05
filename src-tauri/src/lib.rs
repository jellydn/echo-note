mod audio;
mod db;
mod llm;
mod system_audio;
mod whisper;

use audio::{get_recordings_dir, list_audio_devices, AudioRecorder, RecordingResult};
use db::{
    create_meeting, create_summary, create_transcript, delete_meeting, delete_setting,
    delete_summary, delete_transcript, get_meeting, get_setting, get_summary,
    get_summary_by_meeting, get_transcript, get_transcript_by_meeting, init_default_settings,
    list_meetings, list_settings, list_summaries, list_transcripts, set_setting, update_summary,
    update_transcript, CreateMeetingInput, CreateSummaryInput, CreateTranscriptInput,
    DEFAULT_API_ENDPOINT, DEFAULT_API_KEY, DEFAULT_AUDIO_DEVICE, DEFAULT_LLM_PROVIDER,
    DEFAULT_WHISPER_MODEL_SIZE,
};
use llm::{check_ollama_status, generate_summary, DEFAULT_OLLAMA_URL, DEFAULT_SUMMARY_MODEL};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use system_audio::{get_blackhole_device_name, is_blackhole_installed};
use tauri::{Manager, State};
use whisper::{
    download_whisper_model, get_models_info, is_model_downloaded, transcribe_audio, ModelInfo,
};

/// Extended app state that includes audio recording
pub struct AppStateExt {
    pub db: sqlx::Pool<sqlx::Sqlite>,
    pub audio_recorder: Mutex<AudioRecorder>,
}

/// Response wrapper for consistent API responses
#[derive(Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Input for creating a meeting (mirrors CreateMeetingInput for Tauri)
#[derive(Deserialize)]
struct CreateMeetingRequest {
    title: String,
    date: String, // ISO 8601 format
    duration_seconds: i64,
    audio_path: String,
}

/// Meeting response for Tauri commands
#[derive(Serialize, Clone)]
struct MeetingResponse {
    id: i64,
    title: String,
    date: String,
    duration_seconds: i64,
    audio_path: String,
    created_at: String,
}

impl From<db::Meeting> for MeetingResponse {
    fn from(meeting: db::Meeting) -> Self {
        Self {
            id: meeting.id,
            title: meeting.title,
            date: meeting.date.to_rfc3339(),
            duration_seconds: meeting.duration_seconds,
            audio_path: meeting.audio_path,
            created_at: meeting.created_at.to_rfc3339(),
        }
    }
}

#[tauri::command]
async fn create_meeting_command(
    state: State<'_, AppStateExt>,
    request: CreateMeetingRequest,
) -> Result<ApiResponse<MeetingResponse>, String> {
    let date = chrono::DateTime::parse_from_rfc3339(&request.date)
        .map_err(|e| format!("Invalid date format: {}", e))?
        .with_timezone(&chrono::Utc);

    let input = CreateMeetingInput {
        title: request.title,
        date,
        duration_seconds: request.duration_seconds,
        audio_path: request.audio_path,
    };

    let id = create_meeting(&state.db, input)
        .await
        .map_err(|e| format!("Failed to create meeting: {}", e))?;

    // Fetch the created meeting to return full data
    let meeting = get_meeting(&state.db, id)
        .await
        .map_err(|e| format!("Failed to fetch created meeting: {}", e))?
        .ok_or_else(|| "Created meeting not found".to_string())?;

    Ok(ApiResponse::success(meeting.into()))
}

#[tauri::command]
async fn get_meeting_command(
    state: State<'_, AppStateExt>,
    id: i64,
) -> Result<ApiResponse<MeetingResponse>, String> {
    let meeting = get_meeting(&state.db, id)
        .await
        .map_err(|e| format!("Failed to get meeting: {}", e))?;

    match meeting {
        Some(m) => Ok(ApiResponse::success(m.into())),
        None => Ok(ApiResponse::error(format!(
            "Meeting with id {} not found",
            id
        ))),
    }
}

#[tauri::command]
async fn list_meetings_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<Vec<MeetingResponse>>, String> {
    let meetings = list_meetings(&state.db)
        .await
        .map_err(|e| format!("Failed to list meetings: {}", e))?;

    let responses: Vec<MeetingResponse> = meetings.into_iter().map(|m| m.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
async fn delete_meeting_command(
    state: State<'_, AppStateExt>,
    id: i64,
) -> Result<ApiResponse<bool>, String> {
    let deleted = delete_meeting(&state.db, id)
        .await
        .map_err(|e| format!("Failed to delete meeting: {}", e))?;

    if deleted {
        Ok(ApiResponse::success(true))
    } else {
        Ok(ApiResponse::error(format!(
            "Meeting with id {} not found",
            id
        )))
    }
}

// ==================== TRANSCRIPT COMMANDS ====================

/// Input for creating a transcript
#[derive(Deserialize)]
struct CreateTranscriptRequest {
    meeting_id: i64,
    content: String,
}

/// Transcript response for Tauri commands
#[derive(Serialize, Clone)]
struct TranscriptResponse {
    id: i64,
    meeting_id: i64,
    content: String,
    created_at: String,
}

impl From<db::Transcript> for TranscriptResponse {
    fn from(transcript: db::Transcript) -> Self {
        Self {
            id: transcript.id,
            meeting_id: transcript.meeting_id,
            content: transcript.content,
            created_at: transcript.created_at.to_rfc3339(),
        }
    }
}

#[tauri::command]
async fn create_transcript_command(
    state: State<'_, AppStateExt>,
    request: CreateTranscriptRequest,
) -> Result<ApiResponse<TranscriptResponse>, String> {
    let input = CreateTranscriptInput {
        meeting_id: request.meeting_id,
        content: request.content,
    };

    let id = create_transcript(&state.db, input)
        .await
        .map_err(|e| format!("Failed to create transcript: {}", e))?;

    let transcript = get_transcript(&state.db, id)
        .await
        .map_err(|e| format!("Failed to fetch created transcript: {}", e))?
        .ok_or_else(|| "Created transcript not found".to_string())?;

    Ok(ApiResponse::success(transcript.into()))
}

#[tauri::command]
async fn get_transcript_command(
    state: State<'_, AppStateExt>,
    id: i64,
) -> Result<ApiResponse<TranscriptResponse>, String> {
    let transcript = get_transcript(&state.db, id)
        .await
        .map_err(|e| format!("Failed to get transcript: {}", e))?;

    match transcript {
        Some(t) => Ok(ApiResponse::success(t.into())),
        None => Ok(ApiResponse::error(format!(
            "Transcript with id {} not found",
            id
        ))),
    }
}

#[tauri::command]
async fn get_transcript_by_meeting_command(
    state: State<'_, AppStateExt>,
    meeting_id: i64,
) -> Result<ApiResponse<Option<TranscriptResponse>>, String> {
    let transcript = get_transcript_by_meeting(&state.db, meeting_id)
        .await
        .map_err(|e| format!("Failed to get transcript by meeting: {}", e))?;

    Ok(ApiResponse::success(transcript.map(|t| t.into())))
}

#[tauri::command]
async fn list_transcripts_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<Vec<TranscriptResponse>>, String> {
    let transcripts = list_transcripts(&state.db)
        .await
        .map_err(|e| format!("Failed to list transcripts: {}", e))?;

    let responses: Vec<TranscriptResponse> = transcripts.into_iter().map(|t| t.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
async fn update_transcript_command(
    state: State<'_, AppStateExt>,
    id: i64,
    content: String,
) -> Result<ApiResponse<bool>, String> {
    let updated = update_transcript(&state.db, id, content)
        .await
        .map_err(|e| format!("Failed to update transcript: {}", e))?;

    if updated {
        Ok(ApiResponse::success(true))
    } else {
        Ok(ApiResponse::error(format!(
            "Transcript with id {} not found",
            id
        )))
    }
}

#[tauri::command]
async fn delete_transcript_command(
    state: State<'_, AppStateExt>,
    id: i64,
) -> Result<ApiResponse<bool>, String> {
    let deleted = delete_transcript(&state.db, id)
        .await
        .map_err(|e| format!("Failed to delete transcript: {}", e))?;

    if deleted {
        Ok(ApiResponse::success(true))
    } else {
        Ok(ApiResponse::error(format!(
            "Transcript with id {} not found",
            id
        )))
    }
}

// ==================== SUMMARY COMMANDS ====================

/// Input for creating a summary
#[derive(Deserialize)]
struct CreateSummaryRequest {
    meeting_id: i64,
    key_points: String,
    decisions: String,
    action_items: String,
}

/// Summary response for Tauri commands
#[derive(Serialize, Clone)]
struct SummaryResponse {
    id: i64,
    meeting_id: i64,
    key_points: String,
    decisions: String,
    action_items: String,
    created_at: String,
}

impl From<db::Summary> for SummaryResponse {
    fn from(summary: db::Summary) -> Self {
        Self {
            id: summary.id,
            meeting_id: summary.meeting_id,
            key_points: summary.key_points,
            decisions: summary.decisions,
            action_items: summary.action_items,
            created_at: summary.created_at.to_rfc3339(),
        }
    }
}

#[tauri::command]
async fn create_summary_command(
    state: State<'_, AppStateExt>,
    request: CreateSummaryRequest,
) -> Result<ApiResponse<SummaryResponse>, String> {
    let input = CreateSummaryInput {
        meeting_id: request.meeting_id,
        key_points: request.key_points,
        decisions: request.decisions,
        action_items: request.action_items,
    };

    let id = create_summary(&state.db, input)
        .await
        .map_err(|e| format!("Failed to create summary: {}", e))?;

    let summary = get_summary(&state.db, id)
        .await
        .map_err(|e| format!("Failed to fetch created summary: {}", e))?
        .ok_or_else(|| "Created summary not found".to_string())?;

    Ok(ApiResponse::success(summary.into()))
}

#[tauri::command]
async fn get_summary_command(
    state: State<'_, AppStateExt>,
    id: i64,
) -> Result<ApiResponse<SummaryResponse>, String> {
    let summary = get_summary(&state.db, id)
        .await
        .map_err(|e| format!("Failed to get summary: {}", e))?;

    match summary {
        Some(s) => Ok(ApiResponse::success(s.into())),
        None => Ok(ApiResponse::error(format!(
            "Summary with id {} not found",
            id
        ))),
    }
}

#[tauri::command]
async fn get_summary_by_meeting_command(
    state: State<'_, AppStateExt>,
    meeting_id: i64,
) -> Result<ApiResponse<Option<SummaryResponse>>, String> {
    let summary = get_summary_by_meeting(&state.db, meeting_id)
        .await
        .map_err(|e| format!("Failed to get summary by meeting: {}", e))?;

    Ok(ApiResponse::success(summary.map(|s| s.into())))
}

#[tauri::command]
async fn list_summaries_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<Vec<SummaryResponse>>, String> {
    let summaries = list_summaries(&state.db)
        .await
        .map_err(|e| format!("Failed to list summaries: {}", e))?;

    let responses: Vec<SummaryResponse> = summaries.into_iter().map(|s| s.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
async fn update_summary_command(
    state: State<'_, AppStateExt>,
    id: i64,
    key_points: String,
    decisions: String,
    action_items: String,
) -> Result<ApiResponse<bool>, String> {
    let updated = update_summary(&state.db, id, key_points, decisions, action_items)
        .await
        .map_err(|e| format!("Failed to update summary: {}", e))?;

    if updated {
        Ok(ApiResponse::success(true))
    } else {
        Ok(ApiResponse::error(format!(
            "Summary with id {} not found",
            id
        )))
    }
}

#[tauri::command]
async fn delete_summary_command(
    state: State<'_, AppStateExt>,
    id: i64,
) -> Result<ApiResponse<bool>, String> {
    let deleted = delete_summary(&state.db, id)
        .await
        .map_err(|e| format!("Failed to delete summary: {}", e))?;

    if deleted {
        Ok(ApiResponse::success(true))
    } else {
        Ok(ApiResponse::error(format!(
            "Summary with id {} not found",
            id
        )))
    }
}

// ==================== SETTINGS COMMANDS ====================

/// Settings response for Tauri commands
#[derive(Serialize, Clone)]
struct SettingResponse {
    id: i64,
    key: String,
    value: String,
    created_at: String,
    updated_at: String,
}

impl From<db::Setting> for SettingResponse {
    fn from(setting: db::Setting) -> Self {
        Self {
            id: setting.id,
            key: setting.key,
            value: setting.value,
            created_at: setting.created_at.to_rfc3339(),
            updated_at: setting.updated_at.to_rfc3339(),
        }
    }
}

/// Input for getting a setting
#[derive(Deserialize)]
struct GetSettingRequest {
    key: String,
}

/// Get a setting by key, returning default value if not found
#[tauri::command]
async fn get_setting_command(
    state: State<'_, AppStateExt>,
    request: GetSettingRequest,
) -> Result<ApiResponse<String>, String> {
    // Determine default value based on key
    let default_value = match request.key.as_str() {
        "audio_device" => DEFAULT_AUDIO_DEVICE,
        "whisper_model_size" => DEFAULT_WHISPER_MODEL_SIZE,
        "llm_provider" => DEFAULT_LLM_PROVIDER,
        "api_key" => DEFAULT_API_KEY,
        "api_endpoint" => DEFAULT_API_ENDPOINT,
        _ => "",
    };

    let value = get_setting(&state.db, &request.key, default_value)
        .await
        .map_err(|e| format!("Failed to get setting: {}", e))?;

    Ok(ApiResponse::success(value))
}

/// Input for setting a value
#[derive(Deserialize)]
struct SetSettingRequest {
    key: String,
    value: String,
}

/// Set a setting value (insert or update)
#[tauri::command]
async fn set_setting_command(
    state: State<'_, AppStateExt>,
    request: SetSettingRequest,
) -> Result<ApiResponse<bool>, String> {
    let success = set_setting(&state.db, &request.key, &request.value)
        .await
        .map_err(|e| format!("Failed to set setting: {}", e))?;

    Ok(ApiResponse::success(success))
}

/// List all settings
#[tauri::command]
async fn list_settings_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<Vec<SettingResponse>>, String> {
    let settings = list_settings(&state.db)
        .await
        .map_err(|e| format!("Failed to list settings: {}", e))?;

    let responses: Vec<SettingResponse> = settings.into_iter().map(|s| s.into()).collect();
    Ok(ApiResponse::success(responses))
}

/// Delete a setting by key
#[tauri::command]
async fn delete_setting_command(
    state: State<'_, AppStateExt>,
    key: String,
) -> Result<ApiResponse<bool>, String> {
    let deleted = delete_setting(&state.db, &key)
        .await
        .map_err(|e| format!("Failed to delete setting: {}", e))?;

    if deleted {
        Ok(ApiResponse::success(true))
    } else {
        Ok(ApiResponse::error(format!(
            "Setting with key '{}' not found",
            key
        )))
    }
}

// ==================== AUDIO RECORDING COMMANDS ====================

/// Response for recording result
#[derive(Serialize, Clone)]
struct RecordingResponse {
    file_path: String,
    duration_seconds: f64,
    used_system_audio: bool,
}

impl From<RecordingResult> for RecordingResponse {
    fn from(result: RecordingResult) -> Self {
        Self {
            file_path: result.file_path,
            duration_seconds: result.duration_seconds,
            used_system_audio: result.used_system_audio,
        }
    }
}

/// Audio device info
#[derive(Serialize, Clone)]
struct AudioDeviceInfo {
    id: String,
    name: String,
}

/// Start recording audio
/// Records from microphone and optionally from BlackHole (system audio) if available
#[tauri::command]
async fn start_recording_command(
    state: State<'_, AppStateExt>,
    _app_handle: tauri::AppHandle,
) -> Result<ApiResponse<bool>, String> {
    // Get the audio device from settings
    let device_id = get_setting(&state.db, "audio_device", DEFAULT_AUDIO_DEVICE)
        .await
        .map_err(|e| format!("Failed to get audio device setting: {}", e))?;

    // Check if BlackHole is available for system audio capture
    let system_device_name = if is_blackhole_installed() {
        get_blackhole_device_name()
    } else {
        log::warn!("BlackHole not installed - recording microphone only");
        None
    };

    // Start recording
    let mut recorder = state
        .audio_recorder
        .lock()
        .map_err(|e| format!("Failed to lock audio recorder: {}", e))?;

    recorder
        .start_recording(&device_id, system_device_name.as_deref())
        .map_err(|e| format!("Failed to start recording: {}", e))?;

    Ok(ApiResponse::success(true))
}

/// Stop recording audio and save to file
#[tauri::command]
async fn stop_recording_command(
    state: State<'_, AppStateExt>,
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<RecordingResponse>, String> {
    // Get the recordings directory
    let recordings_dir = get_recordings_dir(&app_handle)
        .map_err(|e| format!("Failed to get recordings directory: {}", e))?;

    // Stop recording
    let mut recorder = state
        .audio_recorder
        .lock()
        .map_err(|e| format!("Failed to lock audio recorder: {}", e))?;

    let result = recorder
        .stop_recording(recordings_dir)
        .map_err(|e| format!("Failed to stop recording: {}", e))?;

    Ok(ApiResponse::success(result.into()))
}

/// List available audio input devices
#[tauri::command]
async fn list_audio_devices_command() -> Result<ApiResponse<Vec<AudioDeviceInfo>>, String> {
    let devices = list_audio_devices().map_err(|e| format!("Failed to list devices: {}", e))?;

    let device_infos: Vec<AudioDeviceInfo> = devices
        .into_iter()
        .map(|(id, name)| AudioDeviceInfo { id, name })
        .collect();

    Ok(ApiResponse::success(device_infos))
}

// ==================== BLACKHOLE SYSTEM AUDIO COMMANDS ====================

/// Response for BlackHole status
#[derive(Serialize, Clone)]
struct BlackHoleStatusResponse {
    installed: bool,
    device_name: Option<String>,
}

/// Check if BlackHole virtual audio driver is installed
#[tauri::command]
async fn check_blackhole_status_command() -> Result<ApiResponse<BlackHoleStatusResponse>, String> {
    let installed = is_blackhole_installed();
    let device_name = if installed {
        get_blackhole_device_name()
    } else {
        None
    };

    Ok(ApiResponse::success(BlackHoleStatusResponse {
        installed,
        device_name,
    }))
}

// ==================== WHISPER MODEL COMMANDS ====================

/// Response for model download status
#[derive(Serialize, Clone)]
struct WhisperModelStatusResponse {
    model_size: String,
    is_downloaded: bool,
    model_path: Option<String>,
}

/// Response for model info
#[derive(Serialize, Clone)]
struct WhisperModelInfoResponse {
    size: String,
    filename: String,
    expected_size: u64,
    is_downloaded: bool,
    actual_size: Option<u64>,
}

impl From<ModelInfo> for WhisperModelInfoResponse {
    fn from(info: ModelInfo) -> Self {
        Self {
            size: info.size,
            filename: info.filename,
            expected_size: info.expected_size,
            is_downloaded: info.is_downloaded,
            actual_size: info.actual_size,
        }
    }
}

/// Check if a Whisper model is downloaded
#[tauri::command]
async fn check_whisper_model_command(
    app_handle: tauri::AppHandle,
    model_size: String,
) -> Result<ApiResponse<WhisperModelStatusResponse>, String> {
    let is_downloaded = is_model_downloaded(&app_handle, &model_size)
        .map_err(|e| format!("Failed to check model status: {}", e))?;

    let model_path = if is_downloaded {
        whisper::get_model_path(&app_handle, &model_size)
            .map_err(|e| format!("Failed to get model path: {}", e))?
            .map(|p| p.to_string_lossy().to_string())
    } else {
        None
    };

    Ok(ApiResponse::success(WhisperModelStatusResponse {
        model_size,
        is_downloaded,
        model_path,
    }))
}

/// Download a Whisper model
#[tauri::command]
async fn download_whisper_model_command(
    app_handle: tauri::AppHandle,
    model_size: String,
) -> Result<ApiResponse<String>, String> {
    let model_path = download_whisper_model(&app_handle, &model_size)
        .await
        .map_err(|e| format!("Failed to download model: {}", e))?;

    Ok(ApiResponse::success(
        model_path.to_string_lossy().to_string(),
    ))
}

/// Get information about all available Whisper models
#[tauri::command]
async fn list_whisper_models_command(
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<Vec<WhisperModelInfoResponse>>, String> {
    let models =
        get_models_info(&app_handle).map_err(|e| format!("Failed to get models info: {}", e))?;

    let responses: Vec<WhisperModelInfoResponse> = models.into_iter().map(|m| m.into()).collect();
    Ok(ApiResponse::success(responses))
}

// ==================== TRANSCRIPTION COMMANDS ====================

/// Transcription response for Tauri commands
#[derive(Serialize, Clone)]
struct TranscriptionResponse {
    transcript_id: i64,
    text: String,
    duration_seconds: f64,
}

/// Transcribe audio file and save transcript to database
#[tauri::command]
async fn transcribe_audio_command(
    state: State<'_, AppStateExt>,
    app_handle: tauri::AppHandle,
    meeting_id: i64,
    audio_path: String,
) -> Result<ApiResponse<TranscriptionResponse>, String> {
    // Get the model size from settings
    let model_size = get_setting(&state.db, "whisper_model_size", DEFAULT_WHISPER_MODEL_SIZE)
        .await
        .map_err(|e| format!("Failed to get model size setting: {}", e))?;

    // Run transcription
    let result = tokio::task::spawn_blocking(move || {
        transcribe_audio(&app_handle, &audio_path, &model_size)
    })
    .await
    .map_err(|e| format!("Transcription task failed: {}", e))?
    .map_err(|e| format!("Transcription failed: {}", e))?;

    // Save transcript to database
    let transcript_input = CreateTranscriptInput {
        meeting_id,
        content: result.text.clone(),
    };

    let transcript_id = create_transcript(&state.db, transcript_input)
        .await
        .map_err(|e| format!("Failed to save transcript: {}", e))?;

    Ok(ApiResponse::success(TranscriptionResponse {
        transcript_id,
        text: result.text,
        duration_seconds: result.duration_seconds,
    }))
}

// ==================== LLM / SUMMARY COMMANDS ====================

/// Summary generation response
#[derive(Serialize, Clone)]
struct GenerateSummaryResponse {
    summary_id: i64,
    key_points: String,
    decisions: String,
    action_items: String,
    duration_seconds: f64,
}

/// Ollama status response
#[derive(Serialize, Clone)]
struct OllamaStatusResponse {
    available: bool,
    url: String,
}

/// Check if Ollama is available
#[tauri::command]
async fn check_ollama_status_command() -> Result<ApiResponse<OllamaStatusResponse>, String> {
    let url = DEFAULT_OLLAMA_URL;
    let available = check_ollama_status(url)
        .await
        .map_err(|e| format!("Failed to check Ollama status: {}", e))?;

    Ok(ApiResponse::success(OllamaStatusResponse {
        available,
        url: url.to_string(),
    }))
}

/// Generate a summary from a transcript and save to database
#[tauri::command]
async fn generate_summary_command(
    state: State<'_, AppStateExt>,
    meeting_id: i64,
    transcript: String,
) -> Result<ApiResponse<GenerateSummaryResponse>, String> {
    // Get LLM provider and settings
    let llm_provider = get_setting(&state.db, "llm_provider", DEFAULT_LLM_PROVIDER)
        .await
        .map_err(|e| format!("Failed to get LLM provider setting: {}", e))?;

    // Currently only Ollama is supported (US-011 will add API fallback)
    if llm_provider != "ollama" {
        return Err(format!(
            "LLM provider '{}' not yet implemented. Use 'ollama'.",
            llm_provider
        ));
    }

    // Get Ollama URL from settings or use default
    let ollama_url = DEFAULT_OLLAMA_URL.to_string();

    // For now, use default model. US-016 will allow configuring this via settings
    let model = DEFAULT_SUMMARY_MODEL;

    // Generate summary
    let result = generate_summary(&ollama_url, model, &transcript)
        .await
        .map_err(|e| format!("Failed to generate summary: {}", e))?;

    // Save summary to database
    let summary_input = CreateSummaryInput {
        meeting_id,
        key_points: result.key_points.clone(),
        decisions: result.decisions.clone(),
        action_items: result.action_items.clone(),
    };

    let summary_id = create_summary(&state.db, summary_input)
        .await
        .map_err(|e| format!("Failed to save summary: {}", e))?;

    Ok(ApiResponse::success(GenerateSummaryResponse {
        summary_id,
        key_points: result.key_points,
        decisions: result.decisions,
        action_items: result.action_items,
        duration_seconds: result.duration_seconds,
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let db_pool = db::init_db(&app_handle)
                    .await
                    .expect("Failed to initialize database");

                // Initialize default settings
                init_default_settings(&db_pool)
                    .await
                    .expect("Failed to initialize default settings");

                app_handle.manage(AppStateExt {
                    db: db_pool,
                    audio_recorder: Mutex::new(AudioRecorder::new()),
                });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_meeting_command,
            get_meeting_command,
            list_meetings_command,
            delete_meeting_command,
            create_transcript_command,
            get_transcript_command,
            get_transcript_by_meeting_command,
            list_transcripts_command,
            update_transcript_command,
            delete_transcript_command,
            create_summary_command,
            get_summary_command,
            get_summary_by_meeting_command,
            list_summaries_command,
            update_summary_command,
            delete_summary_command,
            get_setting_command,
            set_setting_command,
            list_settings_command,
            delete_setting_command,
            start_recording_command,
            stop_recording_command,
            list_audio_devices_command,
            check_blackhole_status_command,
            check_whisper_model_command,
            download_whisper_model_command,
            list_whisper_models_command,
            transcribe_audio_command,
            check_ollama_status_command,
            generate_summary_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
