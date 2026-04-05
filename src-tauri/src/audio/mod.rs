use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample};
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

    pub fn is_recording(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.is_recording
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
        if self.is_recording() {
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
                let mut state_guard = state.lock().unwrap();
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
        if !self.is_recording() {
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

        // Re-take handles to join them
        if let Some(handle) = self.mic_handle.take() {
            match handle.audio_thread.join() {
                Ok(Ok(state)) => mic_data = Some(state),
                Ok(Err(e)) => eprintln!("Mic recording error: {}", e),
                Err(_) => eprintln!("Mic recording thread panicked"),
            }
        }

        if let Some(handle) = self.system_handle.take() {
            match handle.audio_thread.join() {
                Ok(Ok(state)) => system_data = Some(state),
                Ok(Err(e)) => eprintln!("System audio recording error: {}", e),
                Err(_) => eprintln!("System audio recording thread panicked"),
            }
        }

        // Get timing info
        let duration = {
            let state_guard = self.state.lock().unwrap();
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
        let mut current_state = self.state.lock().unwrap();
        *current_state = RecordingState::default();

        Ok(RecordingResult {
            file_path: file_path_str,
            duration_seconds: duration,
            used_system_audio,
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

    // Find the device - try exact match first, then partial match
    let device = if device_id_or_name == "default" {
        host.default_input_device()
            .context("No default input device available")?
    } else {
        let devices = host.input_devices().context("Failed to get devices")?;
        let mut found_device = None;

        for device in devices {
            if let Ok(name) = device.name() {
                // Exact match
                if name == device_id_or_name {
                    found_device = Some(device);
                    break;
                }
                // Partial match for BlackHole
                if device_id_or_name.contains("BlackHole")
                    && (name.contains("BlackHole") || name.contains("BlackHole2ch"))
                {
                    found_device = Some(device);
                    break;
                }
                // Partial match for microphone
                if device_id_or_name.contains("microphone")
                    && (name.to_lowercase().contains("microphone")
                        || name.to_lowercase().contains("mic"))
                {
                    found_device = Some(device);
                    break;
                }
            }
        }

        found_device.unwrap_or_else(|| {
            host.default_input_device()
                .expect("No default input device")
        })
    };

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
                if !{
                    let guard = state.lock().unwrap();
                    guard.is_recording
                } {
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

    let err_fn = |err| eprintln!("Stream error: {}", err);

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &_| {
            if should_stop.load(Ordering::SeqCst) {
                return;
            }

            // Convert samples to f32 and store
            let mut buffer = audio_data.lock().unwrap();
            for frame in data.chunks(channels) {
                // Mix channels to mono if needed
                let sample: f32 =
                    frame.iter().map(|&s| f32::from_sample(s)).sum::<f32>() / channels as f32;
                buffer.push(sample);
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

/// List available audio input devices
/// Returns vector of (id, name) tuples where id is the device name (for stable identification)
pub fn list_audio_devices() -> Result<Vec<(String, String)>> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .context("Failed to access input devices")?;

    let mut device_list = Vec::new();

    // Add default device option
    device_list.push(("default".to_string(), "Default Microphone".to_string()));

    for device in devices {
        if let Ok(name) = device.name() {
            // Use the actual device name as the ID for stable identification
            // This ensures the stored device setting matches what cpal returns
            device_list.push((name.clone(), name));
        }
    }

    Ok(device_list)
}

/// Test a microphone by capturing a brief sample and returning the peak audio level.
/// Returns a value between 0.0 (silence) and 1.0 (max).
pub fn test_microphone(device_id: &str) -> Result<f32> {
    let host = cpal::default_host();

    let device = if device_id == "default" {
        host.default_input_device()
            .context("No default input device available")?
    } else {
        let mut found = None;
        for d in host.input_devices().context("Failed to get devices")? {
            if let Ok(name) = d.name() {
                if name == device_id {
                    found = Some(d);
                    break;
                }
            }
        }
        found.unwrap_or(
            host.default_input_device()
                .context("No default input device")?,
        )
    };

    let config = device
        .default_input_config()
        .context("Failed to get input config")?;

    let peak = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let peak_clone = Arc::clone(&peak);
    let channels = config.channels() as usize;

    let stream = device
        .build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| {
                for frame in data.chunks(channels) {
                    let sample: f32 = frame.iter().map(|s| s.abs()).sum::<f32>() / channels as f32;
                    let current = f32::from_bits(peak_clone.load(Ordering::Relaxed));
                    if sample > current {
                        peak_clone.store(sample.to_bits(), Ordering::Relaxed);
                    }
                }
            },
            |err| log::error!("Mic test stream error: {}", err),
            None,
        )
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
