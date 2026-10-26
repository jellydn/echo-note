use crate::db::{get_meeting, get_summary_by_meeting, get_transcript_by_meeting};
use crate::export::{export_filename, render_meeting_markdown, render_meeting_text};
use crate::{ApiResponse, AppStateExt};
use serde::Serialize;
use tauri::State;

/// Payload returned to the frontend with the rendered export.
#[derive(Serialize, Clone)]
pub struct ExportResponse {
    pub format: String,
    pub content: String,
    pub filename: String,
}

/// Export a meeting's notes (metadata + summary + transcript) as Markdown or
/// plain text so users can archive or share them (issue #13).
#[tauri::command]
pub async fn export_meeting_command(
    state: State<'_, AppStateExt>,
    meeting_id: i64,
    format: Option<String>,
) -> Result<ApiResponse<ExportResponse>, String> {
    let meeting = get_meeting(&state.db, meeting_id)
        .await
        .map_err(|e| format!("Failed to fetch meeting: {}", e))?
        .ok_or_else(|| format!("Meeting with id {} not found", meeting_id))?;

    let transcript = get_transcript_by_meeting(&state.db, meeting_id)
        .await
        .map_err(|e| format!("Failed to fetch transcript: {}", e))?;

    let summary = get_summary_by_meeting(&state.db, meeting_id)
        .await
        .map_err(|e| format!("Failed to fetch summary: {}", e))?;

    let is_markdown = format.as_deref().unwrap_or("markdown").eq_ignore_ascii_case("markdown");

    let (content, ext) = if is_markdown {
        (render_meeting_markdown(&meeting, transcript.as_ref(), summary.as_ref()), "md")
    } else {
        (render_meeting_text(&meeting, transcript.as_ref(), summary.as_ref()), "txt")
    };

    let filename = export_filename(&meeting, ext);

    Ok(ApiResponse::success(ExportResponse {
        format: if is_markdown { "markdown".to_string() } else { "text".to_string() },
        content,
        filename,
    }))
}
