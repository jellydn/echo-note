use anyhow::{Context, Result};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use whisper_rs::WhisperContext;

use crate::diarization::{
    create_onnx_embedder, diarize, SegmentSpan, DEFAULT_SIMILARITY_THRESHOLD,
};

/// Maximum number of Whisper model contexts kept in memory at once. Users
/// typically switch between at most two sizes (e.g. "small" and
/// "small-q5_1"), so a tiny bounded cache is enough to reap most of the
/// benefit while still accounting for memory pressure.
pub const MAX_CACHED_WHISPER_MODELS: usize = 2;

/// A small, bounded cache keyed by model size. Keeps expensive loaded
/// resources (e.g. Whisper model contexts) alive across command calls and
/// evicts the oldest entry when the cache grows beyond capacity.
#[derive(Debug)]
pub struct ModelCache<T> {
    entries: HashMap<String, T>,
    order: VecDeque<String>,
    max_entries: usize,
}

impl<T> Default for ModelCache<T> {
    fn default() -> Self {
        Self::with_capacity(MAX_CACHED_WHISPER_MODELS)
    }
}

impl<T> ModelCache<T> {
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            max_entries: max_entries.max(1),
        }
    }

    pub fn get(&self, key: &str) -> Option<&T> {
        self.entries.get(key)
    }

    /// Insert (or replace) an entry, evicting the oldest entries beyond
    /// capacity so the cache stays memory-bounded.
    pub fn insert(&mut self, key: String, value: T) {
        if !self.entries.contains_key(&key) {
            self.order.push_back(key.clone());
        }
        self.entries.insert(key, value);
        while self.order.len() > self.max_entries {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
    }

    /// Remove a specific entry (e.g. when the selected model changes or a
    /// loaded model turns out to be stale).
    pub fn remove(&mut self, key: &str) -> Option<T> {
        self.order.retain(|k| k != key);
        self.entries.remove(key)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return a reference to the entry for `key`, loading it with `load` when
    /// absent. If `load` fails, any stale entry is dropped and the error is
    /// returned, so the next call retries cleanly.
    pub fn get_or_load<F>(&mut self, key: &str, load: F) -> Result<&T>
    where
        F: FnOnce() -> Result<T>,
    {
        if !self.entries.contains_key(key) {
            match load() {
                Ok(value) => self.insert(key.to_string(), value),
                Err(e) => {
                    self.remove(key);
                    return Err(e);
                }
            }
        }
        Ok(self.entries.get(key).expect("entry exists after load"))
    }
}

/// Cache of loaded Whisper model contexts, keyed by model size.
pub type WhisperModelCache = ModelCache<WhisperContext>;

/// Whisper model sizes supported (name, filename, expected_bytes, display_label)
///
/// ## Known-good binding combinations (issue #35)
///
/// | whisper-rs | whisper.cpp | Verified models |
/// |------------|-------------|-----------------|
/// | 0.16.x     | v1.7.x      | tiny, base, small, medium, large-v3-turbo |
///
/// Model files are selected by name and validated by expected size after
/// download; load/inference compatibility is only proven by the opt-in smoke
/// test (`ECHO_NOTE_WHISPER_SMOKE_TEST=1`). Renovate updates to `whisper-rs`
/// must run that smoke test before merge.
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

/// Validate a downloaded model file against its expected size. A model whose
/// size does not match is treated as corrupt and removed so the app re-downloads
/// it instead of failing at inference time.
pub fn validate_model_file(model_path: &PathBuf, expected_size: u64) -> Result<bool> {
    let actual_size = fs::metadata(model_path)
        .with_context(|| format!("Failed to inspect model file at {:?}", model_path))?
        .len();

    if actual_size != expected_size {
        log::warn!(
            "Model file {:?} has size {}, expected {} — removing as corrupt",
            model_path,
            actual_size,
            expected_size
        );
        fs::remove_file(model_path)
            .with_context(|| format!("Failed to remove corrupt model at {:?}", model_path))?;
        return Ok(false);
    }

    Ok(true)
}

/// Check if a model is already downloaded and passes size validation
pub fn is_model_downloaded(app_handle: &AppHandle, model_size: &str) -> Result<bool> {
    let models_dir = get_models_dir(app_handle)?;
    let filename = get_model_filename(model_size)?;
    let model_path = models_dir.join(filename);

    if !model_path.exists() {
        return Ok(false);
    }

    let expected_size = WHISPER_MODELS
        .iter()
        .find(|(size, _, _, _)| *size == model_size)
        .map(|(_, _, expected, _)| *expected)
        .unwrap_or(0);

    validate_model_file(&model_path, expected_size)
}

/// Get the full path to a model file
pub fn get_model_path(app_handle: &AppHandle, model_size: &str) -> Result<Option<PathBuf>> {
    let models_dir = get_models_dir(app_handle)?;
    let filename = get_model_filename(model_size)?;
    let model_path = models_dir.join(filename);

    if model_path.exists() {
        let expected_size = WHISPER_MODELS
            .iter()
            .find(|(size, _, _, _)| *size == model_size)
            .map(|(_, _, expected, _)| *expected)
            .unwrap_or(0);
        if validate_model_file(&model_path, expected_size)? {
            return Ok(Some(model_path));
        }
    }

    Ok(None)
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
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .context("Failed to build HTTP client")?;
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

    // Validate the downloaded file against the expected size before accepting
    // it. A mismatched file is removed so a later transcription does not fail
    // at load/inference time with a corrupt model.
    if !validate_model_file(&model_path, *expected_size)? {
        return Err(anyhow::anyhow!(
            "Downloaded model {} failed size validation and was removed",
            model_size
        ));
    }

    log::info!(
        "Successfully downloaded model {} to {:?} (validated)",
        model_size,
        model_path
    );

    Ok(model_path)
}

/// Opt-in Whisper smoke test (issue #35).
///
/// Runs a real transcription against the given audio file using the given
/// model size, proving that the downloaded model and the linked `whisper-rs`
/// binding can load and infer together. This is intended for CI / release
/// verification and is gated behind `ECHO_NOTE_WHISPER_SMOKE_TEST=1` so it
/// never runs accidentally during normal development.
#[allow(dead_code)]
pub fn run_whisper_smoke_test(app_handle: &AppHandle, audio_path: &str, model_size: &str) -> Result<()> {
    let result = transcribe_audio(app_handle, audio_path, model_size)?;
    if result.text.trim().is_empty() {
        anyhow::bail!("Smoke test produced an empty transcript");
    }
    log::info!(
        "Whisper smoke test passed: model '{}' transcribed {} chars",
        model_size,
        result.text.len()
    );
    Ok(())
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
    pub formatted_text: String,
    pub segments: Vec<TranscriptSegment>,
    pub duration_seconds: f64,
}

#[derive(Clone, serde::Serialize)]
pub struct TranscriptSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub speaker: String,
    pub text: String,
}

/// Options controlling the transcription pipeline.
#[derive(Clone, Debug)]
pub struct TranscriptionOptions {
    /// Whether to run speaker diarization on the resulting segments.
    pub diarization_enabled: bool,
    /// Cosine-similarity threshold for the diarization clustering step.
    pub diarization_threshold: f32,
}

impl Default for TranscriptionOptions {
    fn default() -> Self {
        Self {
            diarization_enabled: true,
            diarization_threshold: DEFAULT_SIMILARITY_THRESHOLD,
        }
    }
}

/// Transcribe audio file using Whisper with default options.
#[allow(dead_code)]
pub fn transcribe_audio(
    app_handle: &AppHandle,
    audio_path: &str,
    model_size: &str,
) -> Result<TranscriptionResult> {
    transcribe_audio_with_options(
        app_handle,
        audio_path,
        model_size,
        TranscriptionOptions::default(),
        None,
    )
}

/// Transcribe audio file using Whisper, controlling pipeline options
/// (e.g. whether to run speaker diarization).
///
/// When `model_cache` is provided, the loaded model context is cached by
/// model size and reused across calls, avoiding a costly reload per command.
pub fn transcribe_audio_with_options(
    app_handle: &AppHandle,
    audio_path: &str,
    model_size: &str,
    options: TranscriptionOptions,
    model_cache: Option<&std::sync::Mutex<WhisperModelCache>>,
) -> Result<TranscriptionResult> {
    use hound::WavReader;
    use std::time::Instant;
    use whisper_rs::{FullParams, WhisperContextParameters};

    let start_time = Instant::now();

    // Get model path
    let model_path = get_model_path(app_handle, model_size)?
        .ok_or_else(|| anyhow::anyhow!("Model {} not downloaded", model_size))?;

    // Emit initial progress
    let _ = app_handle.emit(
        "transcription-progress",
        TranscriptionProgress {
            percentage: 5.0,
            status: "Loading model...".to_string(),
        },
    );

    let ctx_params = WhisperContextParameters::default();
    let model_path_str = model_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Model path contains invalid UTF-8: {:?}", model_path))?;

    // Load (or reuse) the model context, then create per-call transcription
    // state. With a cache, the context is borrowed only long enough to create
    // the state, so concurrent transcriptions proceed independently.
    let mut state = match model_cache {
        Some(cache) => {
            log::info!("Using cached Whisper model context for '{}'", model_size);
            let mut guard = cache
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock model cache: {}", e))?;
            let context = guard
                .get_or_load(model_size, || {
                    log::info!("Loading Whisper model from {:?}", model_path);
                    WhisperContext::new_with_params(model_path_str, ctx_params)
                        .map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {:?}", e))
                })
                .map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {}", e))?;
            context
                .create_state()
                .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {:?}", e))?
        }
        None => {
            log::info!("Loading Whisper model from {:?}", model_path);
            let context = WhisperContext::new_with_params(model_path_str, ctx_params)
                .map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {:?}", e))?;
            context
                .create_state()
                .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {:?}", e))?
        }
    };

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

    // Stream-decode the WAV in bounded windows: for each window we read a fixed
    // number of raw i16 samples, downmix to mono, resample to 16 kHz, and feed
    // Whisper. The full recording is only ever held in memory when speaker
    // diarization is enabled (it needs random access to the whole signal).
    let channels = spec.channels;
    let window_samples_16k = MAX_AUDIO_SAMPLES;
    let window_source =
        ((window_samples_16k as f64 * f64::from(spec.sample_rate) / 16000.0) as usize).max(1);
    let total_frames = reader.duration();
    let total_windows = ((f64::from(total_frames) * 16000.0 / f64::from(spec.sample_rate))
        / window_samples_16k as f64)
        .ceil()
        .max(1.0) as usize;

    let mut raw_window: Vec<i16> = Vec::with_capacity(window_source * usize::from(channels));
    let mut samples_iter = reader.samples::<i16>();

    // Retain the full normalized signal only when diarization needs it.
    let mut full_audio: Option<Vec<f32>> = options.diarization_enabled.then(Vec::new);

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

    let mut full_text = String::new();
    let mut transcript_segments = Vec::new();
    let mut window_offset_seconds = 0.0f64;
    let mut windows_processed = 0usize;

    loop {
        raw_window.clear();
        for _ in 0..window_source * usize::from(channels) {
            match samples_iter.next() {
                Some(Ok(sample)) => raw_window.push(sample),
                Some(Err(e)) => {
                    log::warn!("Skipping unreadable WAV sample: {}", e);
                    break;
                }
                None => break,
            }
        }
        if raw_window.is_empty() {
            break;
        }

        // Downmix + resample this bounded window.
        let mono = pcm_i16_to_mono_f32(&raw_window, channels);
        let mono_16k = if spec.sample_rate != 16000 {
            resample_audio(&mono, spec.sample_rate, 16000)
        } else {
            mono
        };
        if let Some(full) = full_audio.as_mut() {
            full.extend_from_slice(&mono_16k);
        }

        // Emit progress relative to the estimated window count.
        let percentage = 20.0 + (windows_processed as f32 / total_windows as f32) * 70.0;
        let _ = app_handle.emit(
            "transcription-progress",
            TranscriptionProgress {
                percentage,
                status: format!(
                    "Transcribing (window {}/{})...",
                    windows_processed + 1,
                    total_windows
                ),
            },
        );

        // Run transcription on this window.
        state
            .full(params.clone(), &mono_16k)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {:?}", e))?;

        let num_segments = state.full_n_segments();

        for i in 0..num_segments {
            // whisper-rs 0.16: segments are accessed via `get_segment` which
            // returns a `WhisperSegment` exposing text and timestamp getters.
            // A `None` (phantom) segment is skipped rather than aborting the
            // whole transcription — mirrors the old `full_get_segment_text`
            // behaviour where an out-of-range index yields no text.
            let Some(segment) = state.get_segment(i) else {
                continue;
            };
            let segment_text = segment
                .to_str()
                .map_err(|e| anyhow::anyhow!("Failed to get segment text: {:?}", e))?;
            let segment_text = segment_text.trim();
            if segment_text.is_empty() {
                continue;
            }

            let start_seconds = window_offset_seconds + segment.start_timestamp() as f64 * 0.01;
            let end_seconds = window_offset_seconds + segment.end_timestamp() as f64 * 0.01;

            full_text.push_str(segment_text);
            full_text.push(' ');
            transcript_segments.push(TranscriptSegment {
                start_seconds,
                end_seconds,
                speaker: "Speaker 1".to_string(),
                text: segment_text.to_string(),
            });
        }

        window_offset_seconds += mono_16k.len() as f64 / 16000.0;
        windows_processed += 1;
    }

    let audio_data = full_audio.unwrap_or_default();
    log::info!(
        "Audio samples processed: {} ({} windows)",
        audio_data.len(),
        windows_processed
    );

    // Speaker diarization (best-effort: keep "Speaker 1" labels if anything goes wrong).
    if options.diarization_enabled && !transcript_segments.is_empty() {
        let _ = app_handle.emit(
            "transcription-progress",
            TranscriptionProgress {
                percentage: 92.0,
                status: "Identifying speakers...".to_string(),
            },
        );

        let spans: Vec<SegmentSpan> = transcript_segments
            .iter()
            .map(|s| SegmentSpan {
                start_seconds: s.start_seconds,
                end_seconds: s.end_seconds,
            })
            .collect();

        match create_onnx_embedder(app_handle) {
            Ok(embedder) => match diarize(
                &audio_data,
                &spans,
                &embedder,
                options.diarization_threshold,
            ) {
                Ok(labels) if labels.len() == transcript_segments.len() => {
                    // If only one speaker is detected, leave the default label so the UI
                    // doesn't imply multi-speaker analysis happened on a monologue.
                    let unique = labels
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len();
                    if unique > 1 {
                        for (segment, label) in transcript_segments.iter_mut().zip(labels.iter()) {
                            segment.speaker = label.clone();
                        }
                        log::info!("Diarization detected {} speakers", unique);
                    } else {
                        log::info!("Diarization detected a single speaker; keeping default label");
                    }
                }
                Ok(_) => {
                    log::warn!(
                        "Diarization returned a mismatched label count; keeping default speaker labels"
                    );
                }
                Err(e) => {
                    log::warn!("Diarization failed, keeping default speaker labels: {}", e);
                }
            },
            Err(e) => {
                log::warn!(
                    "Diarization model unavailable, keeping default speaker labels: {}",
                    e
                );
            }
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
        formatted_text: format_transcript_segments(&transcript_segments),
        segments: transcript_segments,
        duration_seconds: duration,
    })
}

fn format_transcript_segments(segments: &[TranscriptSegment]) -> String {
    segments
        .iter()
        .map(|segment| {
            format!(
                "[{}] {}: {}",
                format_timestamp(segment.start_seconds),
                segment.speaker,
                segment.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_timestamp(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0).floor() as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// Resample audio using linear interpolation
fn resample_audio(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }

    if from_rate == to_rate {
        return input.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let output_len = (input.len() as f64 * ratio).round().max(1.0) as usize;
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

fn pcm_i16_to_mono_f32(samples: &[i16], channels: u16) -> Vec<f32> {
    let channels = usize::from(channels.max(1));

    samples
        .chunks(channels)
        .map(|frame| {
            frame
                .iter()
                .map(|sample| f32::from(*sample) / 32768.0)
                .sum::<f32>()
                / frame.len() as f32
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_cache_loads_once_and_reuses() {
        let mut cache = ModelCache::<u32>::default();
        let mut loads = 0;

        for _ in 0..3 {
            let value = cache.get_or_load("small", || {
                loads += 1;
                Ok(42)
            });
            assert_eq!(value.unwrap(), &42);
        }

        assert_eq!(loads, 1);
    }

    #[test]
    fn model_cache_evicts_oldest_beyond_capacity() {
        let mut cache = ModelCache::<u32>::with_capacity(2);

        cache.insert("tiny".to_string(), 1);
        cache.insert("base".to_string(), 2);
        cache.insert("small".to_string(), 3);

        assert!(cache.get("tiny").is_none(), "oldest entry should be evicted");
        assert_eq!(cache.get("base"), Some(&2));
        assert_eq!(cache.get("small"), Some(&3));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn model_cache_replaces_existing_key_without_duplicating() {
        let mut cache = ModelCache::<u32>::with_capacity(2);

        cache.insert("small".to_string(), 1);
        cache.insert("small".to_string(), 2);

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get("small"), Some(&2));
    }

    #[test]
    fn model_cache_hit_does_not_reload() {
        let mut cache = ModelCache::<u32>::default();
        cache.insert("small".to_string(), 1);

        // A hit never invokes the loader, so a failing loader is harmless.
        let result = cache.get_or_load("small", || Err(anyhow::anyhow!("should not load")));
        assert_eq!(result.unwrap(), &1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn model_cache_load_failure_returns_error_and_keeps_cache_clean() {
        let mut cache = ModelCache::<u32>::default();

        let result = cache.get_or_load("small", || Err(anyhow::anyhow!("model file corrupt")));
        assert!(result.is_err());
        assert!(cache.is_empty(), "failed load must not leave an entry");

        // A subsequent load succeeds and repopulates.
        let value = cache.get_or_load("small", || Ok(7));
        assert_eq!(value.unwrap(), &7);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn validate_model_file_accepts_matching_size() {
        let dir = std::env::temp_dir().join(format!("echo-note-whisper-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ggml-tiny.bin");
        std::fs::write(&path, vec![0u8; 78_000_000]).unwrap();

        assert!(validate_model_file(&path, 78_000_000).unwrap());
        assert!(path.exists(), "matching file must be kept");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn validate_model_file_removes_mismatched_size() {
        let dir = std::env::temp_dir().join(format!("echo-note-whisper-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ggml-tiny.bin");
        std::fs::write(&path, vec![0u8; 1024]).unwrap();

        assert!(!validate_model_file(&path, 78_000_000).unwrap());
        assert!(!path.exists(), "corrupt file must be removed");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn validate_model_file_errors_on_missing_file() {
        let path = std::env::temp_dir().join("echo-note-does-not-exist.bin");
        assert!(validate_model_file(&path, 100).is_err());
    }

    #[test]
    fn model_cache_remove_and_clear() {
        let mut cache = ModelCache::<u32>::default();
        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);

        assert_eq!(cache.remove("a"), Some(1));
        assert!(cache.get("a").is_none());

        cache.clear();
        assert!(cache.is_empty());
    }

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

    #[test]
    fn test_resample_audio_empty_input() {
        assert!(resample_audio(&[], 48000, 16000).is_empty());
    }

    #[test]
    fn test_pcm_i16_to_mono_f32_averages_stereo_channels() {
        let samples = vec![32767, 0, 0, -32768];

        let mono = pcm_i16_to_mono_f32(&samples, 2);

        assert!((mono[0] - 0.49998474).abs() < 0.00001);
        assert!((mono[1] - -0.5).abs() < 0.00001);
    }

    #[test]
    fn test_pcm_i16_to_mono_f32_empty_input() {
        assert!(pcm_i16_to_mono_f32(&[], 2).is_empty());
    }

    #[test]
    fn test_pcm_i16_to_mono_f32_partial_frame_is_averaged() {
        // A trailing incomplete frame (1 of 2 channels) must still be handled
        // without panicking, mirroring the bounded-window decode path.
        let mono = pcm_i16_to_mono_f32(&[32767], 2);
        assert_eq!(mono.len(), 1);
        assert!((mono[0] - 32767.0 / 32768.0).abs() < 0.00001);
    }

    #[test]
    fn test_pcm_i16_to_mono_f32_handles_mono_channel() {
        let samples = vec![32767, 0, -32768];
        let mono = pcm_i16_to_mono_f32(&samples, 1);
        assert_eq!(mono, vec![32767.0 / 32768.0, 0.0, -1.0]);
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0.0), "00:00");
        assert_eq!(format_timestamp(65.9), "01:05");
        assert_eq!(format_timestamp(3661.2), "01:01:01");
    }

    #[test]
    fn test_format_transcript_segments() {
        let segments = vec![
            TranscriptSegment {
                start_seconds: 0.0,
                end_seconds: 1.5,
                speaker: "Speaker 1".to_string(),
                text: "Hello".to_string(),
            },
            TranscriptSegment {
                start_seconds: 62.0,
                end_seconds: 64.0,
                speaker: "Speaker 1".to_string(),
                text: "Follow up tomorrow".to_string(),
            },
        ];

        assert_eq!(
            format_transcript_segments(&segments),
            "[00:00] Speaker 1: Hello\n[01:02] Speaker 1: Follow up tomorrow"
        );
    }
}
