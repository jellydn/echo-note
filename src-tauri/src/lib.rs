mod db;

use db::{
    create_meeting, create_summary, create_transcript, delete_meeting, delete_summary,
    delete_transcript, get_meeting, get_summary, get_summary_by_meeting, get_transcript,
    get_transcript_by_meeting, list_meetings, list_summaries, list_transcripts, update_summary,
    update_transcript, AppState, CreateMeetingInput, CreateSummaryInput, CreateTranscriptInput,
};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

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
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<MeetingResponse>>, String> {
    let meetings = list_meetings(&state.db)
        .await
        .map_err(|e| format!("Failed to list meetings: {}", e))?;

    let responses: Vec<MeetingResponse> = meetings.into_iter().map(|m| m.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
async fn delete_meeting_command(
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
    meeting_id: i64,
) -> Result<ApiResponse<Option<TranscriptResponse>>, String> {
    let transcript = get_transcript_by_meeting(&state.db, meeting_id)
        .await
        .map_err(|e| format!("Failed to get transcript by meeting: {}", e))?;

    Ok(ApiResponse::success(transcript.map(|t| t.into())))
}

#[tauri::command]
async fn list_transcripts_command(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<TranscriptResponse>>, String> {
    let transcripts = list_transcripts(&state.db)
        .await
        .map_err(|e| format!("Failed to list transcripts: {}", e))?;

    let responses: Vec<TranscriptResponse> = transcripts.into_iter().map(|t| t.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
async fn update_transcript_command(
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
    meeting_id: i64,
) -> Result<ApiResponse<Option<SummaryResponse>>, String> {
    let summary = get_summary_by_meeting(&state.db, meeting_id)
        .await
        .map_err(|e| format!("Failed to get summary by meeting: {}", e))?;

    Ok(ApiResponse::success(summary.map(|s| s.into())))
}

#[tauri::command]
async fn list_summaries_command(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<SummaryResponse>>, String> {
    let summaries = list_summaries(&state.db)
        .await
        .map_err(|e| format!("Failed to list summaries: {}", e))?;

    let responses: Vec<SummaryResponse> = summaries.into_iter().map(|s| s.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
async fn update_summary_command(
    state: State<'_, AppState>,
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
    state: State<'_, AppState>,
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

                app_handle.manage(AppState { db: db_pool });
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
            delete_summary_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
