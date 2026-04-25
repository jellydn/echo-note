use crate::db;
use crate::{ApiResponse, AppStateExt};
use db::{
    create_transcript, delete_transcript, get_transcript, get_transcript_by_meeting,
    list_transcripts, update_transcript, CreateTranscriptInput,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize)]
pub struct CreateTranscriptRequest {
    pub meeting_id: i64,
    pub content: String,
}

#[derive(Serialize, Clone)]
pub struct TranscriptResponse {
    pub id: i64,
    pub meeting_id: i64,
    pub content: String,
    pub created_at: String,
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
pub async fn create_transcript_command(
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
pub async fn get_transcript_command(
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
pub async fn get_transcript_by_meeting_command(
    state: State<'_, AppStateExt>,
    meeting_id: i64,
) -> Result<ApiResponse<Option<TranscriptResponse>>, String> {
    let transcript = get_transcript_by_meeting(&state.db, meeting_id)
        .await
        .map_err(|e| format!("Failed to get transcript by meeting: {}", e))?;

    Ok(ApiResponse::success(transcript.map(|t| t.into())))
}

#[tauri::command]
pub async fn list_transcripts_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<Vec<TranscriptResponse>>, String> {
    let transcripts = list_transcripts(&state.db)
        .await
        .map_err(|e| format!("Failed to list transcripts: {}", e))?;

    let responses: Vec<TranscriptResponse> = transcripts.into_iter().map(|t| t.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
pub async fn update_transcript_command(
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
pub async fn delete_transcript_command(
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
