use crate::db;
use crate::{ApiResponse, AppStateExt};
use db::{
    create_summary, delete_summary, get_summary, get_summary_by_meeting, list_summaries,
    update_summary, CreateSummaryInput,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize)]
pub struct CreateSummaryRequest {
    pub meeting_id: i64,
    pub key_points: String,
    pub decisions: String,
    pub action_items: String,
}

#[derive(Serialize, Clone)]
pub struct SummaryResponse {
    pub id: i64,
    pub meeting_id: i64,
    pub key_points: String,
    pub decisions: String,
    pub action_items: String,
    pub created_at: String,
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
pub async fn create_summary_command(
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
pub async fn get_summary_command(
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
pub async fn get_summary_by_meeting_command(
    state: State<'_, AppStateExt>,
    meeting_id: i64,
) -> Result<ApiResponse<Option<SummaryResponse>>, String> {
    let summary = get_summary_by_meeting(&state.db, meeting_id)
        .await
        .map_err(|e| format!("Failed to get summary by meeting: {}", e))?;

    Ok(ApiResponse::success(summary.map(|s| s.into())))
}

#[tauri::command]
pub async fn list_summaries_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<Vec<SummaryResponse>>, String> {
    let summaries = list_summaries(&state.db)
        .await
        .map_err(|e| format!("Failed to list summaries: {}", e))?;

    let responses: Vec<SummaryResponse> = summaries.into_iter().map(|s| s.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
pub async fn update_summary_command(
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
pub async fn delete_summary_command(
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
