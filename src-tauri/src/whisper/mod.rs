use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

/// Whisper model sizes supported
pub const WHISPER_MODELS: &[(&str, &str, u64)] = &[
    ("tiny", "ggml-tiny.bin", 39_000_000),
    ("base", "ggml-base.bin", 74_000_000),
    ("small", "ggml-small.bin", 244_000_000),
    ("medium", "ggml-medium.bin", 769_000_000),
];

/// Default model size (used for transcription)
#[allow(dead_code)]
pub const DEFAULT_MODEL_SIZE: &str = "small";

/// Get the directory where Whisper models are stored
pub fn get_models_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .context("Failed to get app data directory")?;

    let models_dir = app_data_dir.join("models");
    fs::create_dir_all(&models_dir).context("Failed to create models directory")?;

    Ok(models_dir)
}

/// Get the filename for a model size
pub fn get_model_filename(model_size: &str) -> Result<&'static str> {
    WHISPER_MODELS
        .iter()
        .find(|(size, _, _)| *size == model_size)
        .map(|(_, filename, _)| *filename)
        .ok_or_else(|| anyhow::anyhow!("Invalid model size: {}", model_size))
}

/// Check if a model is already downloaded
pub fn is_model_downloaded(app_handle: &AppHandle, model_size: &str) -> Result<bool> {
    let models_dir = get_models_dir(app_handle)?;
    let filename = get_model_filename(model_size)?;
    let model_path = models_dir.join(filename);

    Ok(model_path.exists())
}

/// Get the full path to a model file
pub fn get_model_path(app_handle: &AppHandle, model_size: &str) -> Result<Option<PathBuf>> {
    let models_dir = get_models_dir(app_handle)?;
    let filename = get_model_filename(model_size)?;
    let model_path = models_dir.join(filename);

    if model_path.exists() {
        Ok(Some(model_path))
    } else {
        Ok(None)
    }
}

/// Download progress event payload
#[derive(Clone, serde::Serialize)]
struct DownloadProgress {
    model_size: String,
    bytes_downloaded: u64,
    total_bytes: u64,
    percentage: f32,
}

/// Download a Whisper model with progress reporting
pub async fn download_whisper_model(app_handle: &AppHandle, model_size: &str) -> Result<PathBuf> {
    // Validate model size
    let (_, filename, expected_size) = WHISPER_MODELS
        .iter()
        .find(|(size, _, _)| *size == model_size)
        .ok_or_else(|| anyhow::anyhow!("Invalid model size: {}", model_size))?;

    let models_dir = get_models_dir(app_handle)?;
    let model_path = models_dir.join(filename);

    // Check if already downloaded
    if model_path.exists() {
        log::info!("Model {} already exists at {:?}", model_size, model_path);
        return Ok(model_path);
    }

    // Download URL from Hugging Face
    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
        filename
    );

    log::info!("Downloading Whisper model {} from {}", model_size, url);

    // Download with streaming
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to start download")?;

    let total_size = response.content_length().unwrap_or(*expected_size);

    // Create the file
    let mut file = fs::File::create(&model_path)
        .with_context(|| format!("Failed to create file at {:?}", model_path))?;

    // Stream the download and report progress
    let mut bytes_downloaded: u64 = 0;
    let mut last_percentage: f32 = 0.0;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.context("Failed to download chunk")?;
        file.write_all(&chunk)
            .context("Failed to write chunk to file")?;

        bytes_downloaded += chunk.len() as u64;

        // Calculate percentage
        let percentage = (bytes_downloaded as f64 / total_size as f64 * 100.0) as f32;

        // Emit progress event every 5% or on completion
        if percentage - last_percentage >= 5.0 || percentage >= 99.0 {
            let progress = DownloadProgress {
                model_size: model_size.to_string(),
                bytes_downloaded,
                total_bytes: total_size,
                percentage,
            };

            // Emit event to frontend
            if let Err(e) = app_handle.emit("whisper-download-progress", &progress) {
                log::warn!("Failed to emit download progress: {}", e);
            }

            last_percentage = percentage;
            log::info!(
                "Download progress: {}% ({}/{} bytes)",
                percentage,
                bytes_downloaded,
                total_size
            );
        }
    }

    // Emit final progress
    let final_progress = DownloadProgress {
        model_size: model_size.to_string(),
        bytes_downloaded,
        total_bytes: total_size,
        percentage: 100.0,
    };

    if let Err(e) = app_handle.emit("whisper-download-progress", &final_progress) {
        log::warn!("Failed to emit final download progress: {}", e);
    }

    log::info!(
        "Successfully downloaded model {} to {:?}",
        model_size,
        model_path
    );

    Ok(model_path)
}

/// Get information about all available models and their download status
pub fn get_models_info(app_handle: &AppHandle) -> Result<Vec<ModelInfo>> {
    let models_dir = get_models_dir(app_handle)?;

    let mut infos = Vec::new();

    for (size, filename, expected_size) in WHISPER_MODELS {
        let model_path = models_dir.join(filename);
        let is_downloaded = model_path.exists();

        let actual_size = if is_downloaded {
            fs::metadata(&model_path).ok().map(|m| m.len())
        } else {
            None
        };

        infos.push(ModelInfo {
            size: size.to_string(),
            filename: filename.to_string(),
            expected_size: *expected_size,
            is_downloaded,
            actual_size,
        });
    }

    Ok(infos)
}

/// Information about a Whisper model
#[derive(Clone, serde::Serialize)]
pub struct ModelInfo {
    pub size: String,
    pub filename: String,
    pub expected_size: u64,
    pub is_downloaded: bool,
    pub actual_size: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_model_filename() {
        assert_eq!(get_model_filename("tiny").unwrap(), "ggml-tiny.bin");
        assert_eq!(get_model_filename("small").unwrap(), "ggml-small.bin");
        assert!(get_model_filename("invalid").is_err());
    }

    #[test]
    fn test_default_model() {
        assert_eq!(DEFAULT_MODEL_SIZE, "small");
    }
}
