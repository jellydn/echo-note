use crate::{db, whisper};
use crate::{ApiResponse, AppStateExt};
use db::{create_transcript, get_setting, CreateTranscriptInput, DEFAULT_WHISPER_MODEL_SIZE};
use serde::Serialize;
use tauri::State;
use whisper::{
    download_whisper_model, get_models_info, is_model_downloaded, transcribe_audio, ModelInfo,
    TranscriptSegment,
};

#[derive(Serialize, Clone)]
pub struct TranscriptionResponse {
    pub transcript_id: i64,
    pub text: String,
    pub formatted_text: String,
    pub segments: Vec<TranscriptSegment>,
    pub duration_seconds: f64,
}

#[derive(Serialize, Clone)]
pub struct WhisperModelStatusResponse {
    pub model_size: String,
    pub is_downloaded: bool,
    pub model_path: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct WhisperModelInfoResponse {
    pub size: String,
    pub filename: String,
    pub expected_size: u64,
    pub is_downloaded: bool,
    pub actual_size: Option<u64>,
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

#[tauri::command]
pub async fn transcribe_audio_command(
    state: State<'_, AppStateExt>,
    app_handle: tauri::AppHandle,
    meeting_id: i64,
    audio_path: String,
) -> Result<ApiResponse<TranscriptionResponse>, String> {
    let model_size = get_setting(&state.db, "whisper_model_size", DEFAULT_WHISPER_MODEL_SIZE)
        .await
        .map_err(|e| format!("Failed to get model size setting: {}", e))?;

    let result = tokio::task::spawn_blocking(move || {
        transcribe_audio(&app_handle, &audio_path, &model_size)
    })
    .await
    .map_err(|e| format!("Transcription task failed: {}", e))?
    .map_err(|e| format!("Transcription failed: {}", e))?;

    let transcript_input = CreateTranscriptInput {
        meeting_id,
        content: if result.formatted_text.trim().is_empty() {
            result.text.clone()
        } else {
            result.formatted_text.clone()
        },
    };

    let transcript_id = create_transcript(&state.db, transcript_input)
        .await
        .map_err(|e| format!("Failed to save transcript: {}", e))?;

    Ok(ApiResponse::success(TranscriptionResponse {
        transcript_id,
        text: result.text,
        formatted_text: result.formatted_text,
        segments: result.segments,
        duration_seconds: result.duration_seconds,
    }))
}

#[tauri::command]
pub async fn check_whisper_model_command(
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

#[tauri::command]
pub async fn download_whisper_model_command(
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

#[tauri::command]
pub async fn list_whisper_models_command(
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<Vec<WhisperModelInfoResponse>>, String> {
    let models =
        get_models_info(&app_handle).map_err(|e| format!("Failed to get models info: {}", e))?;

    let responses: Vec<WhisperModelInfoResponse> = models.into_iter().map(|m| m.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
pub async fn open_models_folder_command(
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<String>, String> {
    let models_dir = whisper::get_models_dir(&app_handle)
        .map_err(|e| format!("Failed to get models dir: {}", e))?;

    std::fs::create_dir_all(&models_dir)
        .map_err(|e| format!("Failed to create models dir: {}", e))?;

    let path_str = models_dir.to_string_lossy().to_string();

    std::process::Command::new("open")
        .arg(&path_str)
        .spawn()
        .map_err(|e| format!("Failed to open folder: {}", e))?;

    Ok(ApiResponse::success(path_str))
}
