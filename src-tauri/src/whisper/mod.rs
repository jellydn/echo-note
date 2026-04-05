use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

/// Whisper model sizes supported (name, filename, expected_bytes, display_label)
pub const WHISPER_MODELS: &[(&str, &str, u64, &str)] = &[
    ("tiny", "ggml-tiny.bin", 78_000_000, "Tiny"),
    ("tiny-q5_1", "ggml-tiny-q5_1.bin", 33_000_000, "Tiny (Q5)"),
    ("base", "ggml-base.bin", 149_000_000, "Base"),
    ("base-q5_1", "ggml-base-q5_1.bin", 60_000_000, "Base (Q5)"),
    ("small", "ggml-small.bin", 489_000_000, "Small"),
    (
        "small-q5_1",
        "ggml-small-q5_1.bin",
        190_000_000,
        "Small (Q5)",
    ),
    ("medium", "ggml-medium.bin", 1_572_000_000, "Medium"),
    (
        "medium-q5_0",
        "ggml-medium-q5_0.bin",
        539_000_000,
        "Medium (Q5)",
    ),
    (
        "large-v3-turbo",
        "ggml-large-v3-turbo.bin",
        1_572_000_000,
        "Large v3 Turbo",
    ),
    (
        "large-v3-turbo-q5_0",
        "ggml-large-v3-turbo-q5_0.bin",
        574_000_000,
        "Large v3 Turbo (Q5)",
    ),
];

/// Default model size (used for transcription)
#[allow(dead_code)]
pub const DEFAULT_MODEL_SIZE: &str = "small";

/// Maximum audio samples to process at once (30 seconds at 16kHz)
const MAX_AUDIO_SAMPLES: usize = 30 * 16000;

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
        .find(|(size, _, _, _)| *size == model_size)
        .map(|(_, filename, _, _)| *filename)
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
    let (_, filename, expected_size, _) = WHISPER_MODELS
        .iter()
        .find(|(size, _, _, _)| *size == model_size)
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

    for (size, filename, expected_size, _) in WHISPER_MODELS {
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

/// Transcription progress event payload
#[derive(Clone, serde::Serialize)]
pub struct TranscriptionProgress {
    pub percentage: f32,
    pub status: String,
}

/// Transcription result
#[derive(Clone, serde::Serialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_seconds: f64,
}

/// Transcribe audio file using Whisper
pub fn transcribe_audio(
    app_handle: &AppHandle,
    audio_path: &str,
    model_size: &str,
) -> Result<TranscriptionResult> {
    use hound::WavReader;
    use std::time::Instant;
    use whisper_rs::{FullParams, WhisperContext, WhisperContextParameters};

    let start_time = Instant::now();

    // Get model path
    let model_path = get_model_path(app_handle, model_size)?
        .ok_or_else(|| anyhow::anyhow!("Model {} not downloaded", model_size))?;

    log::info!("Loading Whisper model from {:?}", model_path);

    // Emit initial progress
    let _ = app_handle.emit(
        "transcription-progress",
        TranscriptionProgress {
            percentage: 5.0,
            status: "Loading model...".to_string(),
        },
    );

    // Load the model
    let ctx_params = WhisperContextParameters::default();
    let ctx = WhisperContext::new_with_params(model_path.to_str().unwrap(), ctx_params)
        .map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {:?}", e))?;

    // Create state for transcription
    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {:?}", e))?;

    // Emit progress
    let _ = app_handle.emit(
        "transcription-progress",
        TranscriptionProgress {
            percentage: 10.0,
            status: "Reading audio file...".to_string(),
        },
    );

    // Read the WAV file
    let mut reader = WavReader::open(audio_path)
        .map_err(|e| anyhow::anyhow!("Failed to open audio file: {}", e))?;

    let spec = reader.spec();
    log::info!(
        "Audio file: channels={}, sample_rate={}, bits_per_sample={}",
        spec.channels,
        spec.sample_rate,
        spec.bits_per_sample
    );

    // Read samples and convert to mono f32 at 16kHz
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .filter_map(|s| s.ok())
        .enumerate()
        .map(|(i, sample)| {
            // Convert to mono by averaging channels
            if spec.channels > 1 && i % spec.channels as usize == 0 {
                // This is the first channel of a frame
                sample as f32 / 32768.0
            } else if spec.channels > 1 {
                // Skip additional channels for now (we'll average later)
                0.0
            } else {
                sample as f32 / 32768.0
            }
        })
        .collect();

    // If stereo, we need to properly average
    let audio_data: Vec<f32> = if spec.channels > 1 {
        samples
            .chunks(spec.channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / spec.channels as f32)
            .collect()
    } else {
        samples
    };

    // Resample to 16kHz if needed
    let audio_data = if spec.sample_rate != 16000 {
        resample_audio(&audio_data, spec.sample_rate, 16000)
    } else {
        audio_data
    };

    log::info!("Audio samples after processing: {}", audio_data.len());

    // Set up transcription parameters
    let mut params = FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // Emit progress
    let _ = app_handle.emit(
        "transcription-progress",
        TranscriptionProgress {
            percentage: 20.0,
            status: "Transcribing...".to_string(),
        },
    );

    // Process audio in chunks if needed (for very long files)
    let mut full_text = String::new();
    let total_samples = audio_data.len();
    let chunk_size = MAX_AUDIO_SAMPLES;

    for (chunk_idx, chunk) in audio_data.chunks(chunk_size).enumerate() {
        // Calculate progress
        let base_progress = 20.0;
        let chunk_progress = (chunk_idx as f32 * chunk_size as f32 / total_samples as f32) * 70.0;
        let percentage = base_progress + chunk_progress;

        let _ = app_handle.emit(
            "transcription-progress",
            TranscriptionProgress {
                percentage,
                status: format!(
                    "Transcribing (chunk {}/{})...",
                    chunk_idx + 1,
                    total_samples.div_ceil(chunk_size)
                ),
            },
        );

        // Run transcription on this chunk
        state
            .full(params.clone(), chunk)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {:?}", e))?;

        // Get the text
        let num_segments = state
            .full_n_segments()
            .map_err(|e| anyhow::anyhow!("Failed to get segment count: {:?}", e))?;

        for i in 0..num_segments {
            let segment_text = state
                .full_get_segment_text(i)
                .map_err(|e| anyhow::anyhow!("Failed to get segment text: {:?}", e))?;
            full_text.push_str(&segment_text);
            full_text.push(' ');
        }
    }

    // Emit final progress
    let _ = app_handle.emit(
        "transcription-progress",
        TranscriptionProgress {
            percentage: 100.0,
            status: "Complete".to_string(),
        },
    );

    let duration = start_time.elapsed().as_secs_f64();
    log::info!("Transcription completed in {:.2} seconds", duration);

    Ok(TranscriptionResult {
        text: full_text.trim().to_string(),
        duration_seconds: duration,
    })
}

/// Resample audio using linear interpolation
fn resample_audio(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return input.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let output_len = (input.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 / ratio;
        let src_idx_floor = src_idx.floor() as usize;
        let src_idx_ceil = (src_idx.ceil() as usize).min(input.len() - 1);
        let frac = src_idx - src_idx_floor as f64;

        let val = input[src_idx_floor] * (1.0 - frac as f32) + input[src_idx_ceil] * frac as f32;
        output.push(val);
    }

    output
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

    #[test]
    fn test_resample_audio() {
        let input = vec![0.0, 0.5, 1.0, 0.5, 0.0];
        let output = resample_audio(&input, 16000, 8000);
        assert_eq!(output.len(), 3); // Approximately half the size
    }
}
