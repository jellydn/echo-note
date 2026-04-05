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

/// Audio recording state
#[derive(Debug, Clone)]
pub struct RecordingState {
    pub is_recording: bool,
    pub start_time: Option<Instant>,
    pub audio_data: Arc<Mutex<Vec<f32>>>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            is_recording: false,
            start_time: None,
            audio_data: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
            channels: 1,
        }
    }
}

/// Result of stopping a recording
#[derive(Debug, Clone)]
pub struct RecordingResult {
    pub file_path: String,
    pub duration_seconds: f64,
}

/// Audio recorder that manages the recording thread
pub struct AudioRecorder {
    command_sender: Option<Sender<RecordingCommand>>,
    audio_thread: Option<JoinHandle<Result<RecordingState>>>,
    state: Arc<Mutex<RecordingState>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            command_sender: None,
            audio_thread: None,
            state: Arc::new(Mutex::new(RecordingState::default())),
        }
    }

    pub fn is_recording(&self) -> bool {
        let state = self.state.lock().unwrap();
        state.is_recording
    }

    /// Start recording audio from the specified device
    pub fn start_recording(&mut self, device_id: &str) -> Result<()> {
        if self.is_recording() {
            return Err(anyhow::anyhow!("Recording is already in progress"));
        }

        // Create channels for communication
        let (cmd_tx, cmd_rx) = channel::<RecordingCommand>();

        // Reset state
        let audio_data = Arc::new(Mutex::new(Vec::new()));
        let state = Arc::new(Mutex::new(RecordingState {
            is_recording: true,
            start_time: Some(Instant::now()),
            audio_data: Arc::clone(&audio_data),
            sample_rate: 16000,
            channels: 1,
        }));

        let state_clone = Arc::clone(&state);
        let device_id = device_id.to_string();

        // Spawn the audio recording thread
        let handle = thread::spawn(move || run_recording_thread(&device_id, cmd_rx, state_clone));

        self.command_sender = Some(cmd_tx);
        self.audio_thread = Some(handle);
        self.state = state;

        Ok(())
    }

    /// Stop recording and save to file
    pub fn stop_recording(&mut self, output_dir: PathBuf) -> Result<RecordingResult> {
        if !self.is_recording() {
            return Err(anyhow::anyhow!("No active recording to stop"));
        }

        // Send stop command
        if let Some(sender) = self.command_sender.take() {
            sender.send(RecordingCommand::Stop)?;
        }

        // Wait for thread to complete
        if let Some(handle) = self.audio_thread.take() {
            let final_state = handle
                .join()
                .map_err(|_| anyhow::anyhow!("Audio thread panicked"))??;

            // Save the recording
            let duration = final_state
                .start_time
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0);

            // Get the audio data
            let audio_data = final_state
                .audio_data
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to lock audio data: {}", e))?;

            // Generate filename with timestamp
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
            let filename = format!("recording_{}.wav", timestamp);
            let file_path = output_dir.join(&filename);

            // Create output directory if needed
            std::fs::create_dir_all(&output_dir)?;

            // Write WAV file
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: final_state.sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut writer = hound::WavWriter::create(&file_path, spec)
                .context("Failed to create WAV writer")?;

            // Convert f32 samples to i16 and write
            for sample in audio_data.iter() {
                let sample_i16 =
                    (sample * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                writer.write_sample(sample_i16)?;
            }

            writer.finalize().context("Failed to finalize WAV file")?;

            let file_path_str = file_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?
                .to_string();

            // Update our state
            let mut current_state = self.state.lock().unwrap();
            *current_state = RecordingState::default();

            return Ok(RecordingResult {
                file_path: file_path_str,
                duration_seconds: duration,
            });
        }

        Err(anyhow::anyhow!("Failed to stop recording"))
    }
}

/// The audio recording thread function
fn run_recording_thread(
    device_id: &str,
    command_receiver: Receiver<RecordingCommand>,
    state: Arc<Mutex<RecordingState>>,
) -> Result<RecordingState> {
    let host = cpal::default_host();

    let device = if device_id == "default" {
        host.default_input_device()
            .context("No default input device available")?
    } else {
        let devices = host.input_devices().context("Failed to get devices")?;
        let mut found_device = None;
        for device in devices {
            if let Ok(name) = device.name() {
                if device_id.contains(&name) || name.to_lowercase().contains("microphone") {
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

    // Update state with actual values
    {
        let mut state_guard = state.lock().unwrap();
        state_guard.sample_rate = sample_rate;
        state_guard.channels = channels;
    }

    let audio_data = {
        let state_guard = state.lock().unwrap();
        Arc::clone(&state_guard.audio_data)
    };

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

    // Return the final state
    let final_state = state.lock().unwrap().clone();
    Ok(final_state)
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
pub fn list_audio_devices() -> Result<Vec<(String, String)>> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .context("Failed to access input devices")?;

    let mut device_list = Vec::new();

    // Add default device
    device_list.push(("default".to_string(), "Default Microphone".to_string()));

    for (idx, device) in devices.enumerate() {
        if let Ok(name) = device.name() {
            let id = format!("device_{}", idx);
            device_list.push((id, name));
        }
    }

    Ok(device_list)
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
