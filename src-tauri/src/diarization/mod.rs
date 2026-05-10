//! Speaker diarization pipeline.
//!
//! Given Whisper's timestamped transcript segments and the corresponding mono
//! 16 kHz audio, this module extracts a speaker embedding per segment and
//! clusters them, returning a label per segment ("Speaker A", "Speaker B",
//! ...). The embedder is pluggable via the [`Embedder`] trait; production
//! transcription uses an ECAPA-TDNN ONNX speaker embedding model.

pub mod clustering;
pub mod embedding;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::{fs, io::Write, path::PathBuf};
use tauri::{AppHandle, Emitter, Manager};

pub use clustering::{cluster_speakers, speaker_label};
#[cfg(test)]
pub use embedding::AcousticFeatureEmbedder;
pub use embedding::{Embedder, OnnxSpeakerEmbedder, TARGET_SAMPLE_RATE};

/// Default cosine-similarity threshold above which two segments are treated
/// as the same speaker. Tunable per user via settings.
pub const DEFAULT_SIMILARITY_THRESHOLD: f32 = 0.75;

pub const DIARIZATION_MODEL_ID: &str = "ecapa-tdnn-speaker-encoder";
pub const DIARIZATION_MODEL_FILENAME: &str = "speaker_encoder_int8.onnx";
pub const DIARIZATION_MODEL_EXPECTED_SIZE: u64 = 9_337_463;
pub const DIARIZATION_MODEL_URL: &str = "https://huggingface.co/marksverdhei/Qwen3-Voice-Embedding-12Hz-0.6B-onnx/resolve/main/speaker_encoder_int8.onnx";

/// One transcript segment with the time range covered by its audio slice.
/// The audio passed to [`diarize`] must already be mono `f32` PCM at
/// [`TARGET_SAMPLE_RATE`].
#[derive(Debug, Clone)]
pub struct SegmentSpan {
    pub start_seconds: f64,
    pub end_seconds: f64,
}

/// Result of diarization: a label per input segment, in the same order.
pub type DiarizationLabels = Vec<String>;

#[derive(Clone, Debug)]
pub struct DiarizationModelInfo {
    pub id: String,
    pub filename: String,
    pub expected_size: u64,
    pub is_downloaded: bool,
    pub actual_size: Option<u64>,
    pub path: Option<PathBuf>,
}

#[derive(Clone, serde::Serialize)]
struct DiarizationDownloadProgress {
    model_id: String,
    bytes_downloaded: u64,
    total_bytes: u64,
    percentage: f32,
}

/// Run the full diarization pipeline over already-decoded audio and Whisper
/// segments.
///
/// Returns a label per segment. If `segments` is empty, returns an empty
/// `Vec`. If only one cluster is found, every label will be `"Speaker A"`.
pub fn diarize<E: Embedder>(
    audio_16k_mono: &[f32],
    segments: &[SegmentSpan],
    embedder: &E,
    threshold: f32,
) -> Result<DiarizationLabels> {
    if segments.is_empty() {
        return Ok(Vec::new());
    }

    let mut embeddings = Vec::with_capacity(segments.len());
    for seg in segments {
        let slice = audio_slice(audio_16k_mono, seg);
        let embedding = embedder.embed(slice)?;
        embeddings.push(embedding);
    }

    let cluster_ids = cluster_speakers(&embeddings, threshold);
    Ok(cluster_ids.into_iter().map(speaker_label).collect())
}

pub fn get_diarization_model_path(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .context("Failed to get app data directory")?;
    let models_dir = app_data_dir.join("models").join("diarization");
    fs::create_dir_all(&models_dir).context("Failed to create diarization models directory")?;
    Ok(models_dir.join(DIARIZATION_MODEL_FILENAME))
}

pub fn get_diarization_model_info(app_handle: &AppHandle) -> Result<DiarizationModelInfo> {
    let path = get_diarization_model_path(app_handle)?;
    let is_downloaded = path.exists();
    let actual_size = if is_downloaded {
        fs::metadata(&path).ok().map(|m| m.len())
    } else {
        None
    };

    Ok(DiarizationModelInfo {
        id: DIARIZATION_MODEL_ID.to_string(),
        filename: DIARIZATION_MODEL_FILENAME.to_string(),
        expected_size: DIARIZATION_MODEL_EXPECTED_SIZE,
        is_downloaded,
        actual_size,
        path: is_downloaded.then_some(path),
    })
}

pub fn create_onnx_embedder(app_handle: &AppHandle) -> Result<OnnxSpeakerEmbedder> {
    let path = get_diarization_model_path(app_handle)?;
    if !path.exists() {
        anyhow::bail!("Diarization model is not downloaded");
    }
    OnnxSpeakerEmbedder::from_model_path(&path)
}

pub async fn download_diarization_model(app_handle: &AppHandle) -> Result<PathBuf> {
    let model_path = get_diarization_model_path(app_handle)?;
    if model_path.exists() {
        return Ok(model_path);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .context("Failed to build HTTP client")?;
    let response = client
        .get(DIARIZATION_MODEL_URL)
        .send()
        .await
        .context("Failed to start diarization model download")?
        .error_for_status()
        .context("Diarization model download failed")?;
    let total_size = response
        .content_length()
        .unwrap_or(DIARIZATION_MODEL_EXPECTED_SIZE);
    let mut file = fs::File::create(&model_path)
        .with_context(|| format!("Failed to create file at {}", model_path.display()))?;
    let mut bytes_downloaded = 0u64;
    let mut last_percentage = 0.0f32;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.context("Failed to download diarization model chunk")?;
        file.write_all(&chunk)
            .context("Failed to write diarization model chunk")?;
        bytes_downloaded += chunk.len() as u64;
        let percentage = (bytes_downloaded as f64 / total_size as f64 * 100.0) as f32;
        if percentage - last_percentage >= 5.0 || percentage >= 99.0 {
            let progress = DiarizationDownloadProgress {
                model_id: DIARIZATION_MODEL_ID.to_string(),
                bytes_downloaded,
                total_bytes: total_size,
                percentage,
            };
            if let Err(e) = app_handle.emit("diarization-download-progress", &progress) {
                log::warn!("Failed to emit diarization download progress: {}", e);
            }
            last_percentage = percentage;
        }
    }

    let progress = DiarizationDownloadProgress {
        model_id: DIARIZATION_MODEL_ID.to_string(),
        bytes_downloaded,
        total_bytes: total_size,
        percentage: 100.0,
    };
    if let Err(e) = app_handle.emit("diarization-download-progress", &progress) {
        log::warn!("Failed to emit final diarization download progress: {}", e);
    }

    Ok(model_path)
}

/// Extract the audio samples that fall inside `seg`. The returned slice is
/// clamped to the bounds of `audio` so out-of-range timestamps degrade
/// gracefully.
fn audio_slice<'a>(audio: &'a [f32], seg: &SegmentSpan) -> &'a [f32] {
    let sr = TARGET_SAMPLE_RATE as f64;
    let start = (seg.start_seconds.max(0.0) * sr) as usize;
    let end = (seg.end_seconds.max(0.0) * sr) as usize;
    let start = start.min(audio.len());
    let end = end.min(audio.len()).max(start);
    &audio[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 1-second mono signal at 16 kHz with the given frequency.
    fn sine_segment(freq: f32, seconds: f64, sample_rate: u32) -> Vec<f32> {
        let n = (seconds * sample_rate as f64) as usize;
        (0..n)
            .map(|i| {
                (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin() * 0.4
            })
            .collect()
    }

    #[test]
    fn empty_segments_returns_empty_labels() {
        let labels = diarize(&[], &[], &AcousticFeatureEmbedder::new(), 0.75).unwrap();
        assert!(labels.is_empty());
    }

    #[test]
    fn single_segment_yields_speaker_a() {
        let audio = sine_segment(220.0, 1.0, TARGET_SAMPLE_RATE);
        let segments = vec![SegmentSpan {
            start_seconds: 0.0,
            end_seconds: 1.0,
        }];
        let labels = diarize(
            &audio,
            &segments,
            &AcousticFeatureEmbedder::new(),
            DEFAULT_SIMILARITY_THRESHOLD,
        )
        .unwrap();
        assert_eq!(labels, vec!["Speaker A".to_string()]);
    }

    #[test]
    fn two_alternating_voices_are_labelled_distinctly() {
        // Build a 4-second track with alternating "voices" (very different
        // frequencies). Each segment is 1 s long.
        let mut audio = Vec::new();
        audio.extend(sine_segment(110.0, 1.0, TARGET_SAMPLE_RATE));
        audio.extend(sine_segment(2_000.0, 1.0, TARGET_SAMPLE_RATE));
        audio.extend(sine_segment(110.0, 1.0, TARGET_SAMPLE_RATE));
        audio.extend(sine_segment(2_000.0, 1.0, TARGET_SAMPLE_RATE));

        let segments = vec![
            SegmentSpan {
                start_seconds: 0.0,
                end_seconds: 1.0,
            },
            SegmentSpan {
                start_seconds: 1.0,
                end_seconds: 2.0,
            },
            SegmentSpan {
                start_seconds: 2.0,
                end_seconds: 3.0,
            },
            SegmentSpan {
                start_seconds: 3.0,
                end_seconds: 4.0,
            },
        ];

        // Use a low threshold; the synthetic embeddings are not normalised to
        // ECAPA-TDNN's distribution.
        let labels = diarize(&audio, &segments, &AcousticFeatureEmbedder::new(), 0.5).unwrap();
        assert_eq!(labels.len(), 4);
        assert_eq!(labels[0], "Speaker A");
        assert_eq!(labels[1], "Speaker B");
        assert_eq!(labels[0], labels[2]);
        assert_eq!(labels[1], labels[3]);
    }

    #[test]
    fn out_of_range_segment_does_not_panic() {
        let audio = sine_segment(440.0, 0.5, TARGET_SAMPLE_RATE);
        let segments = vec![SegmentSpan {
            start_seconds: 10.0,
            end_seconds: 11.0,
        }];
        let labels = diarize(&audio, &segments, &AcousticFeatureEmbedder::new(), 0.75).unwrap();
        assert_eq!(labels, vec!["Speaker A".to_string()]);
    }
}
