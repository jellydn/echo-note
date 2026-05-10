//! Speaker embedding extraction.
//!
//! This module defines an [`Embedder`] trait so different backends can be
//! swapped in without touching the rest of the diarization pipeline.
//!
//! The production implementation, [`OnnxSpeakerEmbedder`], loads a local
//! ECAPA-TDNN ONNX speaker encoder and returns the model's speaker embedding.
//! [`AcousticFeatureEmbedder`] remains as a deterministic, dependency-light
//! implementation for unit tests and failure-isolation.
//!
//! [`AcousticFeatureEmbedder`] produces a
//! ~200-dimensional descriptor from per-frame log-mel-style energies plus
//! their first-order deltas, summarised across the segment with mean and
//! standard deviation.

use anyhow::{Context, Result};
use ndarray::Array3;
use ort::{
    inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::TensorRef,
};
use std::{path::Path, sync::Mutex};

/// Sample rate the embedder expects (mono `f32` PCM, range `[-1.0, 1.0]`).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// The downloaded ECAPA-TDNN model expects 24 kHz log-mel inputs.
const MODEL_SAMPLE_RATE: u32 = 24_000;
const MODEL_N_FFT: usize = 1024;
const MODEL_HOP_LENGTH: usize = 256;
const MODEL_MEL_BINS: usize = 128;
const MODEL_MIN_AUDIO_SAMPLES: usize = MODEL_SAMPLE_RATE as usize;
/// Pluggable embedder. Returning a fixed-dimension vector per call is a hard
/// requirement — the clustering step assumes uniform dimensionality.
pub trait Embedder: Send + Sync {
    /// Compute an embedding for `audio` (mono, [`TARGET_SAMPLE_RATE`]).
    fn embed(&self, audio: &[f32]) -> Result<Vec<f32>>;
}

/// ONNX-backed ECAPA-TDNN speaker encoder.
pub struct OnnxSpeakerEmbedder {
    session: Mutex<Session>,
    input_name: String,
    output_name: String,
}

impl OnnxSpeakerEmbedder {
    pub fn from_model_path(path: &Path) -> Result<Self> {
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level1)?
            .with_intra_threads(2)?
            .commit_from_file(path)
            .with_context(|| format!("Failed to load diarization model at {}", path.display()))?;

        let input_name = session
            .inputs
            .first()
            .map(|input| input.name.clone())
            .unwrap_or_else(|| "mel_spectrogram".to_string());
        let output_name = session
            .outputs
            .first()
            .map(|output| output.name.clone())
            .unwrap_or_else(|| "speaker_embedding".to_string());

        Ok(Self {
            session: Mutex::new(session),
            input_name,
            output_name,
        })
    }
}

impl Embedder for OnnxSpeakerEmbedder {
    fn embed(&self, audio: &[f32]) -> Result<Vec<f32>> {
        let features = log_mel_spectrogram(audio);
        let tensor = TensorRef::from_array_view(&features)?;
        let mut session = self
            .session
            .lock()
            .map_err(|_| anyhow::anyhow!("Diarization model session lock poisoned"))?;
        let outputs = session.run(inputs![self.input_name.as_str() => tensor])?;
        let (_, data) = outputs[self.output_name.as_str()].try_extract_tensor::<f32>()?;
        let mut embedding = data.to_vec();
        l2_normalize(&mut embedding);
        Ok(embedding)
    }
}

/// Minimum number of audio samples (~50 ms at 16 kHz) we consider usable for
/// an embedding. Anything shorter is padded with silence.
#[cfg(test)]
const MIN_SAMPLES: usize = 800;

/// Frame size (~25 ms at 16 kHz) and hop (~10 ms) used for short-time analysis.
#[cfg(test)]
const FRAME_SIZE: usize = 400;
#[cfg(test)]
const FRAME_HOP: usize = 160;

/// Number of mel-style energy bands. The energy block of the output has
/// dimension `4 * NUM_BANDS` (mean + std for the band itself and for its
/// first-order delta).
#[cfg(test)]
const NUM_BANDS: usize = 40;

/// Number of zero-crossing-rate (ZCR) bins per frame. ZCR is a coarse
/// frequency proxy — high-pitched voices/noise have high ZCR, low-pitched
/// content has low ZCR — and adding it lets the embedder discriminate
/// between voices that share similar amplitude envelopes.
#[cfg(test)]
const NUM_ZCR_BINS: usize = 8;

/// Default embedder: mean/std of log-energies across mel-style bands plus
/// their per-frame deltas.
#[cfg(test)]
#[derive(Default, Debug, Clone, Copy)]
pub struct AcousticFeatureEmbedder;

#[cfg(test)]
impl AcousticFeatureEmbedder {
    pub fn new() -> Self {
        Self
    }

    pub fn dim(&self) -> usize {
        NUM_BANDS * 4 + NUM_ZCR_BINS * 2
    }
}

#[cfg(test)]
impl Embedder for AcousticFeatureEmbedder {
    fn embed(&self, audio: &[f32]) -> Result<Vec<f32>> {
        // Pad very short slices with silence so framing always produces at
        // least one frame.
        let mut buf: Vec<f32>;
        let samples: &[f32] = if audio.len() < MIN_SAMPLES {
            buf = Vec::with_capacity(MIN_SAMPLES);
            buf.extend_from_slice(audio);
            buf.resize(MIN_SAMPLES, 0.0);
            &buf
        } else {
            audio
        };

        // Per-frame band energies and zero-crossing-rate bins.
        let mut energy_frames: Vec<[f32; NUM_BANDS]> = Vec::new();
        let mut zcr_frames: Vec<[f32; NUM_ZCR_BINS]> = Vec::new();
        let mut start = 0usize;
        while start + FRAME_SIZE <= samples.len() {
            let frame = &samples[start..start + FRAME_SIZE];
            energy_frames.push(band_energies(frame));
            zcr_frames.push(zero_crossing_rate_bins(frame));
            start += FRAME_HOP;
        }
        if energy_frames.is_empty() {
            // Single frame from the (padded) buffer.
            let mut frame = [0.0f32; FRAME_SIZE];
            let n = samples.len().min(FRAME_SIZE);
            frame[..n].copy_from_slice(&samples[..n]);
            energy_frames.push(band_energies(&frame));
            zcr_frames.push(zero_crossing_rate_bins(&frame));
        }

        // First-order deltas between consecutive energy frames.
        let mut deltas: Vec<[f32; NUM_BANDS]> = Vec::with_capacity(energy_frames.len());
        for i in 0..energy_frames.len() {
            let prev = if i == 0 {
                &energy_frames[0]
            } else {
                &energy_frames[i - 1]
            };
            let next = if i + 1 == energy_frames.len() {
                &energy_frames[i]
            } else {
                &energy_frames[i + 1]
            };
            let mut d = [0.0f32; NUM_BANDS];
            for b in 0..NUM_BANDS {
                d[b] = (next[b] - prev[b]) * 0.5;
            }
            deltas.push(d);
        }

        // Summarise across time with mean and std.
        let band_mean = mean_across_frames::<NUM_BANDS>(&energy_frames);
        let band_std = std_across_frames::<NUM_BANDS>(&energy_frames, &band_mean);
        let delta_mean = mean_across_frames::<NUM_BANDS>(&deltas);
        let delta_std = std_across_frames::<NUM_BANDS>(&deltas, &delta_mean);
        let zcr_mean = mean_across_frames::<NUM_ZCR_BINS>(&zcr_frames);
        let zcr_std = std_across_frames::<NUM_ZCR_BINS>(&zcr_frames, &zcr_mean);

        let mut out = Vec::with_capacity(self.dim());
        out.extend_from_slice(&band_mean);
        out.extend_from_slice(&band_std);
        out.extend_from_slice(&delta_mean);
        out.extend_from_slice(&delta_std);
        out.extend_from_slice(&zcr_mean);
        out.extend_from_slice(&zcr_std);

        l2_normalize(&mut out);
        Ok(out)
    }
}

/// Compute log-energy in `NUM_BANDS` linearly-spaced bands of the squared
/// signal. We deliberately avoid an FFT dependency: a windowed energy
/// projection captures enough timbre information to distinguish voices for
/// the MVP, and ECAPA-TDNN will replace this entirely later.
#[cfg(test)]
fn band_energies(frame: &[f32]) -> [f32; NUM_BANDS] {
    let mut out = [0.0f32; NUM_BANDS];
    if frame.is_empty() {
        return out;
    }
    let len = frame.len();
    let band_size = (len / NUM_BANDS).max(1);
    for (b, slot) in out.iter_mut().enumerate() {
        let start = b * band_size;
        let end = if b + 1 == NUM_BANDS {
            len
        } else {
            (start + band_size).min(len)
        };
        if start >= end {
            continue;
        }
        let mut sum_sq = 0.0f32;
        for &x in &frame[start..end] {
            sum_sq += x * x;
        }
        let mean_energy = sum_sq / (end - start) as f32;
        // log(1+x) keeps things finite for silence and compresses dynamic range.
        *slot = (1.0 + mean_energy).ln();
    }
    out
}

/// Zero-crossing rate computed in `NUM_ZCR_BINS` evenly-sized chunks of the
/// frame. The output is the per-chunk fraction of sign changes between
/// adjacent samples (in `[0, 1]`), which scales roughly with the dominant
/// frequency in that chunk.
#[cfg(test)]
fn zero_crossing_rate_bins(frame: &[f32]) -> [f32; NUM_ZCR_BINS] {
    let mut out = [0.0f32; NUM_ZCR_BINS];
    if frame.len() < 2 {
        return out;
    }
    let chunk = frame.len() / NUM_ZCR_BINS;
    if chunk < 2 {
        return out;
    }
    for (b, slot) in out.iter_mut().enumerate() {
        let start = b * chunk;
        let end = if b + 1 == NUM_ZCR_BINS {
            frame.len()
        } else {
            start + chunk
        };
        let slice = &frame[start..end];
        if slice.len() < 2 {
            continue;
        }
        let mut crossings = 0usize;
        for window in slice.windows(2) {
            if (window[0] >= 0.0) != (window[1] >= 0.0) {
                crossings += 1;
            }
        }
        *slot = crossings as f32 / (slice.len() - 1) as f32;
    }
    out
}

#[cfg(test)]
fn mean_across_frames<const N: usize>(frames: &[[f32; N]]) -> [f32; N] {
    let mut acc = [0.0f32; N];
    if frames.is_empty() {
        return acc;
    }
    for frame in frames {
        for b in 0..N {
            acc[b] += frame[b];
        }
    }
    let n = frames.len() as f32;
    for v in &mut acc {
        *v /= n;
    }
    acc
}

#[cfg(test)]
fn std_across_frames<const N: usize>(frames: &[[f32; N]], mean: &[f32; N]) -> [f32; N] {
    let mut acc = [0.0f32; N];
    if frames.len() < 2 {
        return acc;
    }
    for frame in frames {
        for b in 0..N {
            let d = frame[b] - mean[b];
            acc[b] += d * d;
        }
    }
    let n = frames.len() as f32;
    for v in &mut acc {
        *v = (*v / n).sqrt();
    }
    acc
}

fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > f32::EPSILON {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

fn log_mel_spectrogram(audio_16k: &[f32]) -> Array3<f32> {
    let mut audio = resample_linear(audio_16k, TARGET_SAMPLE_RATE, MODEL_SAMPLE_RATE);
    if audio.len() < MODEL_MIN_AUDIO_SAMPLES {
        audio.resize(MODEL_MIN_AUDIO_SAMPLES, 0.0);
    }

    let frame_count = ((audio.len().saturating_sub(MODEL_N_FFT)) / MODEL_HOP_LENGTH + 1).max(1);
    let mut features = Array3::<f32>::zeros((1, frame_count, MODEL_MEL_BINS));
    let window: Vec<f32> = (0..MODEL_N_FFT)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / MODEL_N_FFT as f32).cos())
        .collect();

    for frame_idx in 0..frame_count {
        let start = frame_idx * MODEL_HOP_LENGTH;
        let mut spectrum = vec![0.0f32; MODEL_N_FFT / 2 + 1];
        for (bin, slot) in spectrum.iter_mut().enumerate() {
            let mut re = 0.0f32;
            let mut im = 0.0f32;
            for (n, window_value) in window.iter().enumerate() {
                let sample = audio.get(start + n).copied().unwrap_or(0.0) * window_value;
                let angle =
                    -2.0 * std::f32::consts::PI * bin as f32 * n as f32 / MODEL_N_FFT as f32;
                re += sample * angle.cos();
                im += sample * angle.sin();
            }
            *slot = re.mul_add(re, im * im);
        }

        for mel_bin in 0..MODEL_MEL_BINS {
            let start_bin = mel_bin * spectrum.len() / MODEL_MEL_BINS;
            let end_bin = ((mel_bin + 1) * spectrum.len() / MODEL_MEL_BINS).max(start_bin + 1);
            let energy = spectrum[start_bin..end_bin.min(spectrum.len())]
                .iter()
                .sum::<f32>()
                / (end_bin - start_bin) as f32;
            features[[0, frame_idx, mel_bin]] = energy.max(1e-5).ln();
        }
    }

    features
}

fn resample_linear(audio: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if audio.is_empty() || from_rate == to_rate {
        return audio.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let out_len = (audio.len() as f64 * ratio).ceil() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let src_idx = src_pos.floor() as usize;
        let frac = (src_pos - src_idx as f64) as f32;
        let a = audio.get(src_idx).copied().unwrap_or(0.0);
        let b = audio.get(src_idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, samples: usize, sample_rate: f32) -> Vec<f32> {
        (0..samples)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin() * 0.5)
            .collect()
    }

    #[test]
    fn embedder_dim_is_consistent() {
        let e = AcousticFeatureEmbedder::new();
        let audio = sine(220.0, 16_000, 16_000.0);
        let v = e.embed(&audio).unwrap();
        assert_eq!(v.len(), e.dim());
    }

    #[test]
    fn very_short_input_still_returns_embedding() {
        let e = AcousticFeatureEmbedder::new();
        let v = e.embed(&[0.1, -0.1, 0.2]).unwrap();
        assert_eq!(v.len(), e.dim());
    }

    #[test]
    fn embeddings_are_l2_normalized() {
        let e = AcousticFeatureEmbedder::new();
        let v = e.embed(&sine(440.0, 16_000, 16_000.0)).unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-3, "expected unit norm, got {norm}");
    }

    #[test]
    fn distinct_signals_produce_distinguishable_embeddings() {
        // Two very different "voices": low sine vs. high sine.
        let e = AcousticFeatureEmbedder::new();
        let low = e.embed(&sine(110.0, 16_000, 16_000.0)).unwrap();
        let high = e.embed(&sine(2_000.0, 16_000, 16_000.0)).unwrap();
        let dot: f32 = low.iter().zip(high.iter()).map(|(a, b)| a * b).sum();
        // Cosine similarity should be clearly below 1.0 — these are different.
        assert!(dot < 0.95, "expected dissimilar embeddings, got cos={dot}");
    }
}
