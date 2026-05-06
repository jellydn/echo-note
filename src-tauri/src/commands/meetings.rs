use crate::{ApiResponse, AppStateExt};
use db::{
    create_meeting, delete_meeting, get_meeting, list_meetings, update_meeting, CreateMeetingInput,
};
use serde::{Deserialize, Deserializer, Serialize};
use tauri::State;

use crate::db;

#[derive(Deserialize)]
pub struct CreateMeetingRequest {
    pub title: String,
    #[serde(deserialize_with = "deserialize_meeting_date")]
    pub date: chrono::DateTime<chrono::Utc>,
    pub duration_seconds: i64,
    pub audio_path: String,
}

fn deserialize_meeting_date<'de, D>(
    deserializer: D,
) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum MeetingDate {
        Rfc3339(String),
        UnixSeconds(i64),
    }

    match MeetingDate::deserialize(deserializer)? {
        MeetingDate::Rfc3339(value) => chrono::DateTime::parse_from_rfc3339(&value)
            .map(|date| date.with_timezone(&chrono::Utc))
            .map_err(serde::de::Error::custom),
        MeetingDate::UnixSeconds(value) => chrono::DateTime::from_timestamp(value, 0)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid Unix timestamp: {value}"))),
    }
}

#[derive(Serialize, Clone)]
pub struct MeetingResponse {
    pub id: i64,
    pub title: String,
    pub date: String,
    pub duration_seconds: i64,
    pub audio_path: String,
    pub created_at: String,
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
pub async fn create_meeting_command(
    state: State<'_, AppStateExt>,
    request: CreateMeetingRequest,
) -> Result<ApiResponse<MeetingResponse>, String> {
    let input = CreateMeetingInput {
        title: request.title,
        date: request.date,
        duration_seconds: request.duration_seconds,
        audio_path: request.audio_path,
    };

    let id = create_meeting(&state.db, input)
        .await
        .map_err(|e| format!("Failed to create meeting: {}", e))?;

    let meeting = get_meeting(&state.db, id)
        .await
        .map_err(|e| format!("Failed to fetch created meeting: {}", e))?
        .ok_or_else(|| "Created meeting not found".to_string())?;

    Ok(ApiResponse::success(meeting.into()))
}

#[tauri::command]
pub async fn get_meeting_command(
    state: State<'_, AppStateExt>,
    id: i64,
) -> Result<ApiResponse<MeetingResponse>, String> {
    log::info!("Getting meeting with id: {}", id);

    let meeting = get_meeting(&state.db, id).await.map_err(|e| {
        log::error!("Database error fetching meeting {}: {}", id, e);
        format!("Database error: {}", e)
    })?;

    match meeting {
        Some(m) => {
            log::info!("Found meeting: {} ({})", m.id, m.title);
            Ok(ApiResponse::success(m.into()))
        }
        None => {
            log::warn!("Meeting with id {} not found", id);
            Ok(ApiResponse::error(format!(
                "Meeting with id {} not found",
                id
            )))
        }
    }
}

#[tauri::command]
pub async fn list_meetings_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<Vec<MeetingResponse>>, String> {
    let meetings = list_meetings(&state.db)
        .await
        .map_err(|e| format!("Failed to list meetings: {}", e))?;

    let responses: Vec<MeetingResponse> = meetings.into_iter().map(|m| m.into()).collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
pub async fn delete_meeting_command(
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

#[tauri::command]
pub async fn update_meeting_command(
    state: State<'_, AppStateExt>,
    id: i64,
    title: String,
) -> Result<ApiResponse<MeetingResponse>, String> {
    let updated = update_meeting(&state.db, id, title)
        .await
        .map_err(|e| format!("Failed to update meeting: {}", e))?;

    if updated {
        let meeting = get_meeting(&state.db, id)
            .await
            .map_err(|e| format!("Failed to fetch updated meeting: {}", e))?
            .ok_or_else(|| "Updated meeting not found".to_string())?;

        Ok(ApiResponse::success(meeting.into()))
    } else {
        Ok(ApiResponse::error(format!(
            "Meeting with id {} not found",
            id
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_meeting_request_accepts_rfc3339_date() {
        let request: CreateMeetingRequest = serde_json::from_str(
            r#"{
                "title": "Test Meeting",
                "date": "2026-05-06T15:56:00.000Z",
                "duration_seconds": 3,
                "audio_path": "/tmp/recording.wav"
            }"#,
        )
        .unwrap();

        assert_eq!(request.date.to_rfc3339(), "2026-05-06T15:56:00+00:00");
    }

    #[test]
    fn create_meeting_request_accepts_unix_seconds_for_compatibility() {
        let request: CreateMeetingRequest = serde_json::from_str(
            r#"{
                "title": "Test Meeting",
                "date": 1778082960,
                "duration_seconds": 3,
                "audio_path": "/tmp/recording.wav"
            }"#,
        )
        .unwrap();

        assert_eq!(request.date.to_rfc3339(), "2026-05-06T15:56:00+00:00");
    }
}
