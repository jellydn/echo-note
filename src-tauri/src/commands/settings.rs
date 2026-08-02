use crate::db;
use crate::keychain::{is_secret_setting, API_KEY_ACCOUNT};
use crate::{ApiResponse, AppStateExt};
use db::{
    delete_setting, get_setting, list_settings, set_setting, DEFAULT_API_ENDPOINT,
    DEFAULT_AUDIO_DEVICE, DEFAULT_LLM_PROVIDER, DEFAULT_WHISPER_MODEL_SIZE,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Serialize, Clone)]
pub struct SettingResponse {
    pub id: i64,
    pub key: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
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

#[derive(Deserialize)]
pub struct GetSettingRequest {
    pub key: String,
}

#[derive(Deserialize)]
pub struct SetSettingRequest {
    pub key: String,
    pub value: String,
}

#[tauri::command]
pub async fn get_setting_command(
    state: State<'_, AppStateExt>,
    request: GetSettingRequest,
) -> Result<ApiResponse<String>, String> {
    // Secrets (e.g. the API key) live in the OS credential store, never SQLite.
    if is_secret_setting(&request.key) {
        let value = state
            .secret_store
            .get(API_KEY_ACCOUNT)
            .map_err(|e| format!("Failed to read secret: {}", e))?
            .unwrap_or_default();
        return Ok(ApiResponse::success(value));
    }

    let default_value = match request.key.as_str() {
        "audio_device" => DEFAULT_AUDIO_DEVICE,
        "whisper_model_size" => DEFAULT_WHISPER_MODEL_SIZE,
        "llm_provider" => DEFAULT_LLM_PROVIDER,
        "api_endpoint" => DEFAULT_API_ENDPOINT,
        _ => "",
    };

    let value = get_setting(&state.db, &request.key, default_value)
        .await
        .map_err(|e| format!("Failed to get setting: {}", e))?;

    Ok(ApiResponse::success(value))
}

#[tauri::command]
pub async fn set_setting_command(
    state: State<'_, AppStateExt>,
    request: SetSettingRequest,
) -> Result<ApiResponse<bool>, String> {
    // Secrets (e.g. the API key) live in the OS credential store, never SQLite.
    if is_secret_setting(&request.key) {
        state
            .secret_store
            .set(API_KEY_ACCOUNT, &request.value)
            .map_err(|e| format!("Failed to store secret: {}", e))?;
        return Ok(ApiResponse::success(true));
    }

    let success = set_setting(&state.db, &request.key, &request.value)
        .await
        .map_err(|e| format!("Failed to set setting: {}", e))?;

    Ok(ApiResponse::success(success))
}

#[tauri::command]
pub async fn list_settings_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<Vec<SettingResponse>>, String> {
    let settings = list_settings(&state.db)
        .await
        .map_err(|e| format!("Failed to list settings: {}", e))?;

    // Mask secret values (e.g. the API key) so they are never exposed through
    // the settings listing. The UI reads secrets via `get_setting_command`.
    let responses: Vec<SettingResponse> = settings
        .into_iter()
        .map(|s| {
            let mut response: SettingResponse = s.into();
            if is_secret_setting(&response.key) && !response.value.is_empty() {
                response.value = "••••••••".to_string();
            }
            response
        })
        .collect();
    Ok(ApiResponse::success(responses))
}

#[tauri::command]
pub async fn delete_setting_command(
    state: State<'_, AppStateExt>,
    key: String,
) -> Result<ApiResponse<bool>, String> {
    // Secrets (e.g. the API key) live in the OS credential store, never SQLite.
    if is_secret_setting(&key) {
        let deleted = state
            .secret_store
            .delete(API_KEY_ACCOUNT)
            .map_err(|e| format!("Failed to delete secret: {}", e))?;
        return if deleted {
            Ok(ApiResponse::success(true))
        } else {
            Ok(ApiResponse::error(format!(
                "Setting with key '{}' not found",
                key
            )))
        };
    }

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
