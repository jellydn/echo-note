mod db;

use db::{
    create_meeting, delete_meeting, get_meeting, list_meetings, AppState, CreateMeetingInput,
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
            delete_meeting_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
