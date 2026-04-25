use crate::db;
use crate::{ApiResponse, AppStateExt};
use db::{
    create_summary, get_setting, CreateSummaryInput, DEFAULT_API_ENDPOINT, DEFAULT_API_MODEL,
    DEFAULT_LLM_PROVIDER,
};
use llm::{
    check_ollama_status, generate_summary, generate_summary_api, DEFAULT_OLLAMA_URL,
    DEFAULT_SUMMARY_MODEL, PROVIDER_API,
};
use serde::Serialize;
use tauri::State;
use url::Url;

use crate::llm;

/// Normalize API endpoint URL to ensure it ends with /chat/completions
fn normalize_api_endpoint(endpoint: &str) -> anyhow::Result<Url> {
    let mut url = Url::parse(endpoint)?;

    // Check if path already ends with chat/completions
    if url.path().ends_with("/chat/completions") {
        return Ok(url);
    }

    // Handle base URL patterns
    let path = url.path();
    let new_path = if path.ends_with("/v1") {
        format!("{}/chat/completions", path)
    } else if path.ends_with("/") {
        format!("{}chat/completions", path)
    } else if path == "/" || path.is_empty() {
        "/v1/chat/completions".to_string()
    } else {
        format!("{}/chat/completions", path)
    };

    url.set_path(&new_path);
    Ok(url)
}

#[derive(Serialize, Clone)]
pub struct GenerateSummaryResponse {
    pub summary_id: i64,
    pub key_points: String,
    pub decisions: String,
    pub action_items: String,
    pub duration_seconds: f64,
}

#[derive(Serialize, Clone)]
pub struct OllamaStatusResponse {
    pub available: bool,
    pub url: String,
}

#[tauri::command]
pub async fn check_ollama_status_command() -> Result<ApiResponse<OllamaStatusResponse>, String> {
    let url = DEFAULT_OLLAMA_URL;
    let available = check_ollama_status(url)
        .await
        .map_err(|e| format!("Failed to check Ollama status: {}", e))?;

    Ok(ApiResponse::success(OllamaStatusResponse {
        available,
        url: url.to_string(),
    }))
}

#[tauri::command]
pub async fn generate_summary_command(
    state: State<'_, AppStateExt>,
    meeting_id: i64,
    transcript: String,
) -> Result<ApiResponse<GenerateSummaryResponse>, String> {
    let llm_provider = get_setting(&state.db, "llm_provider", DEFAULT_LLM_PROVIDER)
        .await
        .map_err(|e| format!("Failed to get LLM provider setting: {}", e))?;

    let result = if llm_provider == PROVIDER_API {
        let api_key = get_setting(&state.db, "api_key", "")
            .await
            .map_err(|e| format!("Failed to get API key setting: {}", e))?;
        let api_endpoint = get_setting(&state.db, "api_endpoint", DEFAULT_API_ENDPOINT)
            .await
            .map_err(|e| format!("Failed to get API endpoint setting: {}", e))?;

        if api_key.is_empty() {
            return Err("API key is required when using API provider".to_string());
        }

        // Parse and normalize API URL using url crate
        let api_url = normalize_api_endpoint(&api_endpoint)
            .map_err(|e| format!("Invalid API endpoint: {}", e))?;

        let model = get_setting(&state.db, "api_model", DEFAULT_API_MODEL)
            .await
            .map_err(|e| format!("Failed to get API model setting: {}", e))?;

        generate_summary_api(api_url.as_str(), &api_key, &model, &transcript)
            .await
            .map_err(|e| format!("Failed to generate summary via API: {}", e))?
    } else {
        let ollama_url = DEFAULT_OLLAMA_URL.to_string();
        let model = DEFAULT_SUMMARY_MODEL;

        generate_summary(&ollama_url, model, &transcript)
            .await
            .map_err(|e| format!("Failed to generate summary: {}", e))?
    };

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
