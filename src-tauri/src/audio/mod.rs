use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;
use tauri::Manager;

/// Recording control messages
pub enum RecordingCommand {
    Stop,
}

/// Audio recording state for a single device
#[derive(Debug, Clone, Default)]
pub struct DeviceRecordingState {
    pub audio_data: Arc<Mutex<Vec<f32>>>,
    pub sample_rate: u32,
    #[allow(dead_code)]
    pub channels: u16,
}

/// Overall recording state
#[derive(Debug, Clone, Default)]
pub struct RecordingState {
    pub is_recording: bool,
    pub start_time: Option<Instant>,
    #[allow(dead_code)]
    pub mic_state: DeviceRecordingState,
    pub system_state: Option<DeviceRecordingState>,
}

/// Result of stopping a recording
#[derive(Debug, Clone)]
pub struct RecordingResult {
    pub file_path: String,
    pub duration_seconds: f64,
    pub used_system_audio: bool,
    pub system_audio_error: Option<String>,
}

/// Thread handle for a single recording
struct RecordingThreadHandle {
    command_sender: Sender<RecordingCommand>,
    audio_thread: JoinHandle<Result<DeviceRecordingState>>,
}

/// Audio recorder that manages single or dual recording threads
pub struct AudioRecorder {
    mic_handle: Option<RecordingThreadHandle>,
    system_handle: Option<RecordingThreadHandle>,
    state: Arc<Mutex<RecordingState>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            mic_handle: None,
            system_handle: None,
            state: Arc::new(Mutex::new(RecordingState::default())),
        }
    }

    pub fn is_recording(&self) -> Result<bool> {
        let state = self
            .state
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock recording state: {}", e))?;
        Ok(state.is_recording)
    }

    /// Start recording audio from the microphone and optionally from system audio
    ///
    /// # Arguments
    /// * `mic_device_id` - The microphone device ID to record from
    /// * `system_device_name` - Optional BlackHole device name for system audio capture
    pub fn start_recording(
        &mut self,
        mic_device_id: &str,
        system_device_name: Option<&str>,
    ) -> Result<()> {
        if self.is_recording()? {
            return Err(anyhow::anyhow!("Recording is already in progress"));
        }

        // Create mic recording state
        let mic_audio_data = Arc::new(Mutex::new(Vec::new()));
        let mic_state = DeviceRecordingState {
            audio_data: Arc::clone(&mic_audio_data),
            sample_rate: 16000,
            channels: 1,
        };

        // Reset overall state
        let state = Arc::new(Mutex::new(RecordingState {
            is_recording: true,
            start_time: Some(Instant::now()),
            mic_state,
            system_state: None,
        }));

        // Start microphone recording thread
        let (mic_cmd_tx, mic_cmd_rx) = channel::<RecordingCommand>();
        let mic_state_clone = Arc::clone(&state);
        let mic_device_id = mic_device_id.to_string();

        let mic_handle = thread::spawn(move || {
            run_single_recording_thread(&mic_device_id, mic_cmd_rx, mic_audio_data, mic_state_clone)
        });

        self.mic_handle = Some(RecordingThreadHandle {
            command_sender: mic_cmd_tx,
            audio_thread: mic_handle,
        });

        // Start system audio recording if BlackHole is available
        if let Some(system_name) = system_device_name {
            let system_audio_data = Arc::new(Mutex::new(Vec::new()));
            let system_state = DeviceRecordingState {
                audio_data: Arc::clone(&system_audio_data),
                sample_rate: 16000,
                channels: 1,
            };

            // Update state to include system audio
            {
                let mut state_guard = state
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Failed to lock recording state: {}", e))?;
                state_guard.system_state = Some(system_state);
            }

            let (sys_cmd_tx, sys_cmd_rx) = channel::<RecordingCommand>();
            let system_state_clone = Arc::clone(&state);
            let system_name = system_name.to_string();

            let system_handle = thread::spawn(move || {
                run_single_recording_thread(
                    &system_name,
                    sys_cmd_rx,
                    system_audio_data,
                    system_state_clone,
                )
            });

            self.system_handle = Some(RecordingThreadHandle {
                command_sender: sys_cmd_tx,
                audio_thread: system_handle,
            });
        }

        self.state = state;
        Ok(())
    }

    /// Stop recording and save to file
    /// If both mic and system audio were recorded, they are mixed together
    pub fn stop_recording(&mut self, output_dir: PathBuf) -> Result<RecordingResult> {
        if !self.is_recording()? {
            return Err(anyhow::anyhow!("No active recording to stop"));
        }

        // Send stop commands to all recording threads
        if let Some(ref handle) = self.mic_handle {
            let _ = handle.command_sender.send(RecordingCommand::Stop);
        }

        if let Some(ref handle) = self.system_handle {
            let _ = handle.command_sender.send(RecordingCommand::Stop);
        }

        // Collect results from threads
        let mut mic_data: Option<DeviceRecordingState> = None;
        let mut system_data: Option<DeviceRecordingState> = None;
        let mut system_audio_error: Option<String> = None;

        // Re-take handles to join them
        if let Some(handle) = self.mic_handle.take() {
            match handle.audio_thread.join() {
                Ok(Ok(state)) => mic_data = Some(state),
                Ok(Err(e)) => {
                    log::error!("Mic recording error: {}", e);
                    return Err(anyhow::anyhow!("Mic recording failed: {}", e));
                }
                Err(_) => {
                    return Err(anyhow::anyhow!("Mic recording thread panicked"));
                }
            }
        }

        if let Some(handle) = self.system_handle.take() {
            match handle.audio_thread.join() {
                Ok(Ok(state)) => system_data = Some(state),
                Ok(Err(e)) => {
                    let message = e.to_string();
                    log::error!("System audio recording error: {}", message);
                    // System audio failure is non-fatal — fall back to mic only
                    log::warn!("Falling back to mic-only audio");
                    system_audio_error = Some(message);
                }
                Err(_) => {
                    let message = "System audio recording thread panicked".to_string();
                    log::error!("{} — falling back to mic only", message);
                    system_audio_error = Some(message);
                }
            }
        }

        // Get timing info
        let duration = {
            let state_guard = self
                .state
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock recording state: {}", e))?;
            state_guard
                .start_time
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0)
        };

        // Mix audio data if we have both sources
        let mixed_audio = if let (Some(mic), Some(sys)) = (&mic_data, &system_data) {
            // Mix microphone and system audio
            let mic_samples = mic
                .audio_data
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock mic audio data: {}", e))?;
            let sys_samples = sys
                .audio_data
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock system audio data: {}", e))?;

            mix_audio_streams(&mic_samples, &sys_samples, mic.sample_rate, sys.sample_rate)
        } else if let Some(mic) = &mic_data {
            // Mic only
            let mic_samples = mic
                .audio_data
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock audio data: {}", e))?;
            mic_samples.clone()
        } else {
            return Err(anyhow::anyhow!("No audio data recorded"));
        };

        // Determine sample rate for output
        let output_sample_rate = mic_data.as_ref().map(|s| s.sample_rate).unwrap_or(16000);
        let used_system_audio = system_data.is_some();

        // Generate filename with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("recording_{}.wav", timestamp);
        let file_path = output_dir.join(&filename);

        // Create output directory if needed
        std::fs::create_dir_all(&output_dir)?;

        // Write WAV file
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: output_sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer =
            hound::WavWriter::create(&file_path, spec).context("Failed to create WAV writer")?;

        // Convert f32 samples to i16 and write
        for sample in mixed_audio.iter() {
            let sample_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(sample_i16)?;
        }

        writer.finalize().context("Failed to finalize WAV file")?;

        let file_path_str = file_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
            .to_string();

        // Reset state
        let mut current_state = self
            .state
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to lock recording state: {}", e))?;
        *current_state = RecordingState::default();

        Ok(RecordingResult {
            file_path: file_path_str,
            duration_seconds: duration,
            used_system_audio,
            system_audio_error,
        })
    }
}

/// Mix two audio streams together
/// If sample rates differ, the second stream is resampled to match the first
fn mix_audio_streams(
    stream1: &[f32],
    stream2: &[f32],
    sample_rate1: u32,
    sample_rate2: u32,
) -> Vec<f32> {
    // Determine the output length (longer of the two streams, adjusted for sample rate)
    let len1 = stream1.len();
    let len2 = if sample_rate1 == sample_rate2 {
        stream2.len()
    } else {
        // Estimate resampled length
        (stream2.len() as f64 * sample_rate1 as f64 / sample_rate2 as f64) as usize
    };

    let output_len = len1.max(len2);
    let mut mixed = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let sample1 = if i < len1 { stream1[i] } else { 0.0 };

        // Get sample from stream2, with resampling if needed
        let sample2 = if sample_rate1 == sample_rate2 {
            if i < stream2.len() {
                stream2[i]
            } else {
                0.0
            }
        } else {
            // Simple linear interpolation for resampling
            let pos = i as f64 * sample_rate2 as f64 / sample_rate1 as f64;
            let idx = pos as usize;
            let frac = pos - idx as f64;

            if idx >= stream2.len() {
                0.0
            } else if idx + 1 >= stream2.len() {
                stream2[idx]
            } else {
                let s1 = stream2[idx];
                let s2 = stream2[idx + 1];
                s1 + (s2 - s1) * frac as f32
            }
        };

        // Mix with slight attenuation to prevent clipping
        mixed.push((sample1 + sample2) * 0.75);
    }

    mixed
}

/// The audio recording thread function for a single device
fn run_single_recording_thread(
    device_id_or_name: &str,
    command_receiver: Receiver<RecordingCommand>,
    audio_data: Arc<Mutex<Vec<f32>>>,
    state: Arc<Mutex<RecordingState>>,
) -> Result<DeviceRecordingState> {
    let host = cpal::default_host();

    let device = resolve_input_device(&host, device_id_or_name)?;

    let config = device
        .default_input_config()
        .context("Failed to get default input config")?;

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    let sample_format = config.sample_format();

    let audio_data_clone = Arc::clone(&audio_data);
    let should_stop = Arc::new(AtomicBool::new(false));
    let should_stop_clone = Arc::clone(&should_stop);

    // Build and start the stream
    let stream = match sample_format {
        SampleFormat::F32 => {
            build_stream::<f32>(&device, &config.into(), audio_data_clone, should_stop_clone)?
        }
        SampleFormat::I16 => {
            build_stream::<i16>(&device, &config.into(), audio_data_clone, should_stop_clone)?
        }
        SampleFormat::U16 => {
            build_stream::<u16>(&device, &config.into(), audio_data_clone, should_stop_clone)?
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported sample format: {:?}",
                sample_format
            ))
        }
    };

    stream.play().context("Failed to start recording stream")?;

    // Wait for stop command
    loop {
        match command_receiver.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(RecordingCommand::Stop) => {
                should_stop.store(true, Ordering::SeqCst);
                break;
            }
            Err(_) => {
                // Timeout, check if we should continue
                let should_continue = match state.lock() {
                    Ok(guard) => guard.is_recording,
                    Err(e) => {
                        log::error!("Recording state lock poisoned: {}", e);
                        false
                    }
                };

                if !should_continue {
                    break;
                }
            }
        }
    }

    // Stream will be dropped here, which stops it
    drop(stream);

    // Return the recording state
    Ok(DeviceRecordingState {
        audio_data,
        sample_rate,
        channels,
    })
}

/// Build an audio stream for recording
fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    audio_data: Arc<Mutex<Vec<f32>>>,
    should_stop: Arc<AtomicBool>,
) -> Result<cpal::Stream>
where
    T: Sample + FromSample<f32> + SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;

    let err_fn = |err| log::error!("Audio stream error: {}", err);

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &_| {
            if should_stop.load(Ordering::SeqCst) {
                return;
            }

            // Convert samples to f32 and store
            let Ok(mut buffer) = audio_data.lock() else {
                log::error!("Audio buffer lock poisoned — dropping samples");
                return;
            };
            for frame in data.chunks(channels) {
                buffer.push(mono_sample_from_frame(frame));
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn mono_sample_from_frame<T>(frame: &[T]) -> f32
where
    T: Sample,
    f32: FromSample<T>,
{
    if frame.is_empty() {
        return 0.0;
    }

    frame.iter().map(|&s| f32::from_sample(s)).sum::<f32>() / frame.len() as f32
}

fn peak_sample_from_frame<T>(frame: &[T]) -> f32
where
    T: Sample,
    f32: FromSample<T>,
{
    if frame.is_empty() {
        return 0.0;
    }

    let average_abs = frame
        .iter()
        .map(|&s| f32::from_sample(s).abs())
        .sum::<f32>()
        / frame.len() as f32;
    average_abs.clamp(0.0, 1.0)
}

fn update_peak(peak: &std::sync::atomic::AtomicU32, sample: f32) {
    let current = f32::from_bits(peak.load(Ordering::Relaxed));
    if sample > current {
        peak.store(sample.to_bits(), Ordering::Relaxed);
    }
}

fn build_microphone_test_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    peak: Arc<std::sync::atomic::AtomicU32>,
) -> Result<cpal::Stream>
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &_| {
            for frame in data.chunks(channels) {
                update_peak(&peak, peak_sample_from_frame(frame));
            }
        },
        |err| log::error!("Mic test stream error: {}", err),
        None,
    )?;

    Ok(stream)
}

/// List available audio input devices
/// Returns vector of (id, name) tuples. cpal does not expose a portable persistent
/// hardware ID, so IDs are deterministic app keys derived from device names and
/// duplicate occurrence numbers. Legacy stored display names still resolve.
pub fn list_audio_devices() -> Result<Vec<(String, String)>> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .context("Failed to access input devices")?;

    let mut device_list = Vec::new();

    // Add default device option
    device_list.push(("default".to_string(), "Default Microphone".to_string()));

    let mut name_counts = HashMap::new();
    for device in devices {
        if let Ok(name) = device.name() {
            let id = make_audio_device_id(&name, next_device_occurrence(&mut name_counts, &name));
            device_list.push((id, name));
        }
    }

    Ok(device_list)
}

fn resolve_input_device(host: &cpal::Host, device_id_or_name: &str) -> Result<cpal::Device> {
    if device_id_or_name == "default" {
        return host
            .default_input_device()
            .context("No default input device available");
    }

    let devices = collect_named_input_devices(host)?;
    find_input_device(&devices, device_id_or_name)
        .or_else(|| host.default_input_device())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No audio input device available for '{}'",
                device_id_or_name
            )
        })
}

fn collect_named_input_devices(host: &cpal::Host) -> Result<Vec<(String, String, cpal::Device)>> {
    let mut devices_with_names = Vec::new();
    let mut name_counts = HashMap::new();

    for device in host.input_devices().context("Failed to get devices")? {
        if let Ok(name) = device.name() {
            let occurrence = next_device_occurrence(&mut name_counts, &name);
            let id = make_audio_device_id(&name, occurrence);
            devices_with_names.push((id, name, device));
        }
    }

    Ok(devices_with_names)
}

fn find_input_device(
    devices: &[(String, String, cpal::Device)],
    device_id_or_name: &str,
) -> Option<cpal::Device> {
    devices
        .iter()
        .find(|(id, _, _)| id == device_id_or_name)
        .or_else(|| {
            devices
                .iter()
                .find(|(_, name, _)| name == device_id_or_name)
        })
        .or_else(|| {
            devices
                .iter()
                .find(|(_, name, _)| is_blackhole_device_match(device_id_or_name, name))
        })
        .or_else(|| {
            devices
                .iter()
                .find(|(_, name, _)| is_microphone_device_match(device_id_or_name, name))
        })
        .map(|(_, _, device)| device.clone())
}

fn next_device_occurrence(name_counts: &mut HashMap<String, usize>, name: &str) -> usize {
    let entry = name_counts.entry(name.to_string()).or_insert(0);
    let occurrence = *entry;
    *entry += 1;
    occurrence
}

fn make_audio_device_id(name: &str, occurrence: usize) -> String {
    format!(
        "coreaudio-input:{:016x}:{}",
        stable_device_name_hash(name),
        occurrence
    )
}

fn stable_device_name_hash(name: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

fn is_blackhole_device_match(device_id_or_name: &str, candidate_name: &str) -> bool {
    device_id_or_name.contains("BlackHole")
        && (candidate_name.contains("BlackHole") || candidate_name.contains("BlackHole2ch"))
}

fn is_microphone_device_match(device_id_or_name: &str, candidate_name: &str) -> bool {
    device_id_or_name.contains("microphone")
        && (candidate_name.to_lowercase().contains("microphone")
            || candidate_name.to_lowercase().contains("mic"))
}

/// Test a microphone by capturing a brief sample and returning the peak audio level.
/// Returns a value between 0.0 (silence) and 1.0 (max).
pub fn test_microphone(device_id: &str) -> Result<f32> {
    let host = cpal::default_host();

    let device = if device_id == "default" {
        host.default_input_device()
            .context("No default input device available")?
    } else {
        resolve_input_device(&host, device_id)?
    };

    let config = device
        .default_input_config()
        .context("Failed to get input config")?;

    let peak = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let sample_format = config.sample_format();

    let stream_config = config.into();
    let stream = match sample_format {
        SampleFormat::F32 => {
            build_microphone_test_stream::<f32>(&device, &stream_config, Arc::clone(&peak))
        }
        SampleFormat::I16 => {
            build_microphone_test_stream::<i16>(&device, &stream_config, Arc::clone(&peak))
        }
        SampleFormat::U16 => {
            build_microphone_test_stream::<u16>(&device, &stream_config, Arc::clone(&peak))
        }
        _ => Err(anyhow::anyhow!(
            "Unsupported sample format: {:?}",
            sample_format
        )),
    }
    .context("Failed to build test stream")?;

    stream.play().context("Failed to start mic test")?;

    // Capture for 1 second
    std::thread::sleep(std::time::Duration::from_secs(1));

    drop(stream);

    let peak_val = f32::from_bits(peak.load(Ordering::Relaxed));
    log::info!("Mic test peak level: {}", peak_val);
    Ok(peak_val)
}

/// Get the default audio directory for recordings
pub fn get_recordings_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .context("Failed to get app data directory")?;

    let recordings_dir = app_dir.join("recordings");
    Ok(recordings_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_sample_from_frame_averages_f32_channels() {
        assert_eq!(mono_sample_from_frame(&[0.5_f32, -0.25_f32]), 0.125);
    }

    #[test]
    fn mono_sample_from_frame_converts_i16_channels() {
        let expected = (f32::from_sample(i16::MAX) + f32::from_sample(i16::MIN)) / 2.0;

        assert_eq!(mono_sample_from_frame(&[i16::MAX, i16::MIN]), expected);
    }

    #[test]
    fn mono_sample_from_frame_converts_u16_channels() {
        let expected = (f32::from_sample(u16::MAX) + f32::from_sample(0_u16)) / 2.0;

        assert_eq!(mono_sample_from_frame(&[u16::MAX, 0_u16]), expected);
    }

    #[test]
    fn peak_sample_from_frame_uses_absolute_mono_level() {
        assert_eq!(peak_sample_from_frame(&[-0.75_f32, 0.25_f32]), 0.5);
    }

    #[test]
    fn update_peak_keeps_largest_sample() {
        let peak = std::sync::atomic::AtomicU32::new(0);

        update_peak(&peak, 0.25);
        update_peak(&peak, 0.1);
        update_peak(&peak, 0.75);

        assert_eq!(f32::from_bits(peak.load(Ordering::Relaxed)), 0.75);
    }

    #[test]
    fn make_audio_device_id_is_stable_for_same_name_and_occurrence() {
        let first = make_audio_device_id("Studio Display Microphone", 0);
        let second = make_audio_device_id("Studio Display Microphone", 0);

        assert_eq!(first, second);
        assert!(first.starts_with("coreaudio-input:"));
    }

    #[test]
    fn make_audio_device_id_disambiguates_duplicate_names() {
        assert_ne!(
            make_audio_device_id("USB Audio Device", 0),
            make_audio_device_id("USB Audio Device", 1)
        );
    }

    #[test]
    fn next_device_occurrence_counts_per_name() {
        let mut counts = HashMap::new();

        assert_eq!(next_device_occurrence(&mut counts, "Mic"), 0);
        assert_eq!(next_device_occurrence(&mut counts, "Mic"), 1);
        assert_eq!(next_device_occurrence(&mut counts, "Other Mic"), 0);
    }

    #[test]
    fn device_name_match_helpers_preserve_legacy_fallbacks() {
        assert!(is_blackhole_device_match("BlackHole2ch", "BlackHole 2ch"));
        assert!(is_microphone_device_match(
            "external microphone",
            "External Mic"
        ));
        assert!(!is_microphone_device_match(
            "external microphone",
            "Studio Speakers"
        ));
    }
}
