use crate::{ApiResponse, AppStateExt};
use audio::{get_recordings_dir, list_audio_devices, RecordingResult};
use db::{get_setting, set_setting, DEFAULT_AUDIO_DEVICE};
use serde::Serialize;
use system_audio::{
    auto_install_blackhole, get_blackhole_device_name, install_blackhole_driver,
    install_blackhole_from_bundle, is_blackhole_installed, BlackHoleInstallMethod,
};
use tauri::State;

use crate::{audio, db, system_audio};

#[derive(Serialize, Clone)]
pub struct RecordingResponse {
    pub file_path: String,
    pub duration_seconds: f64,
    pub used_system_audio: bool,
    pub system_audio_error: Option<String>,
}

impl From<RecordingResult> for RecordingResponse {
    fn from(result: RecordingResult) -> Self {
        Self {
            file_path: result.file_path,
            duration_seconds: result.duration_seconds,
            used_system_audio: result.used_system_audio,
            system_audio_error: result.system_audio_error,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct BlackHoleStatusResponse {
    pub installed: bool,
    pub device_name: Option<String>,
}

#[tauri::command]
pub async fn start_recording_command(
    state: State<'_, AppStateExt>,
    _app_handle: tauri::AppHandle,
) -> Result<ApiResponse<bool>, String> {
    let device_id = get_setting(&state.db, "audio_device", DEFAULT_AUDIO_DEVICE)
        .await
        .map_err(|e| format!("Failed to get audio device setting: {}", e))?;

    let system_device_name = if is_blackhole_installed() {
        get_blackhole_device_name()
    } else {
        log::warn!("BlackHole not installed - recording microphone only");
        None
    };

    let mut recorder = state
        .audio_recorder
        .lock()
        .map_err(|e| format!("Failed to lock audio recorder: {}", e))?;

    recorder
        .start_recording(&device_id, system_device_name.as_deref())
        .map_err(|e| format!("Failed to start recording: {}", e))?;

    Ok(ApiResponse::success(true))
}

#[tauri::command]
pub async fn stop_recording_command(
    state: State<'_, AppStateExt>,
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<RecordingResponse>, String> {
    let recordings_dir = get_recordings_dir(&app_handle)
        .map_err(|e| format!("Failed to get recordings directory: {}", e))?;

    let mut recorder = state
        .audio_recorder
        .lock()
        .map_err(|e| format!("Failed to lock audio recorder: {}", e))?;

    let result = recorder
        .stop_recording(recordings_dir)
        .map_err(|e| format!("Failed to stop recording: {}", e))?;

    Ok(ApiResponse::success(result.into()))
}

#[tauri::command]
pub async fn list_audio_devices_command() -> Result<ApiResponse<Vec<AudioDeviceInfo>>, String> {
    let devices = list_audio_devices().map_err(|e| format!("Failed to list devices: {}", e))?;

    let device_infos: Vec<AudioDeviceInfo> = devices
        .into_iter()
        .map(|(id, name)| AudioDeviceInfo { id, name })
        .collect();

    Ok(ApiResponse::success(device_infos))
}

#[tauri::command]
pub async fn test_microphone_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<f32>, String> {
    let device_id = get_setting(&state.db, "audio_device", DEFAULT_AUDIO_DEVICE)
        .await
        .map_err(|e| format!("Failed to get audio device setting: {}", e))?;

    let peak = tokio::task::spawn_blocking(move || audio::test_microphone(&device_id))
        .await
        .map_err(|e| format!("Mic test task failed: {}", e))?
        .map_err(|e| format!("Mic test failed: {}", e))?;

    Ok(ApiResponse::success(peak))
}

#[tauri::command]
pub async fn check_blackhole_status_command() -> Result<ApiResponse<BlackHoleStatusResponse>, String>
{
    let installed = is_blackhole_installed();
    let device_name = if installed {
        get_blackhole_device_name()
    } else {
        None
    };

    Ok(ApiResponse::success(BlackHoleStatusResponse {
        installed,
        device_name,
    }))
}

#[tauri::command]
pub async fn install_blackhole_command(
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<bool>, String> {
    install_blackhole_driver(&app_handle)
        .map_err(|e| format!("{} Please visit https://github.com/ExistentialAudio/BlackHole manually to download the installer.", e))?;

    Ok(ApiResponse::success(true))
}

#[tauri::command]
pub async fn check_homebrew_status_command() -> Result<ApiResponse<bool>, String> {
    Ok(ApiResponse::success(system_audio::is_homebrew_installed()))
}

#[tauri::command]
pub async fn install_blackhole_homebrew_command() -> Result<ApiResponse<bool>, String> {
    system_audio::install_blackhole_via_homebrew().map_err(|e| {
        format!(
            "Homebrew installation failed: {}. Try the manual download option instead.",
            e
        )
    })?;

    Ok(ApiResponse::success(true))
}

#[derive(Serialize, Clone)]
pub struct BlackHoleInstallResponse {
    pub success: bool,
    pub method: String,
    pub message: String,
}

#[tauri::command]
pub async fn install_blackhole_bundled_command(
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<BlackHoleInstallResponse>, String> {
    install_blackhole_from_bundle(&app_handle).map_err(|e| {
        format!(
            "Bundled installation failed: {}. Try Homebrew or manual installation instead.",
            e
        )
    })?;

    let response = BlackHoleInstallResponse {
        success: true,
        method: "bundled".to_string(),
        message: "BlackHole installed successfully from bundled package".to_string(),
    };

    log::info!("BlackHole installed via bundled package");
    Ok(ApiResponse::success(response))
}

#[tauri::command]
pub async fn auto_install_blackhole_command(
    app_handle: tauri::AppHandle,
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<BlackHoleInstallResponse>, String> {
    let method = auto_install_blackhole(&app_handle).map_err(|e| {
        format!(
            "Auto-installation failed: {}. Please install manually from Settings.",
            e
        )
    })?;

    // Mark that we've attempted BlackHole installation
    set_setting(&state.db, "blackhole_install_attempted", "true")
        .await
        .map_err(|e| format!("Failed to record installation attempt: {}", e))?;

    let method_str = match method {
        BlackHoleInstallMethod::Bundled => "bundled",
        BlackHoleInstallMethod::Homebrew => "homebrew",
        BlackHoleInstallMethod::Manual => "manual",
        BlackHoleInstallMethod::AlreadyInstalled => "already_installed",
    };

    let message = match method {
        BlackHoleInstallMethod::Bundled => {
            "BlackHole installed successfully from bundled package".to_string()
        }
        BlackHoleInstallMethod::Homebrew => {
            "BlackHole installation started via Homebrew. Check Terminal for progress.".to_string()
        }
        BlackHoleInstallMethod::Manual => {
            "Download page opened. Please download and install manually, then restart the app."
                .to_string()
        }
        BlackHoleInstallMethod::AlreadyInstalled => "BlackHole was already installed".to_string(),
    };

    let response = BlackHoleInstallResponse {
        success: method != BlackHoleInstallMethod::Manual,
        method: method_str.to_string(),
        message,
    };

    log::info!("BlackHole auto-install result: {:?}", method);
    Ok(ApiResponse::success(response))
}

#[tauri::command]
pub async fn complete_first_launch_setup_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<bool>, String> {
    set_setting(&state.db, "first_launch_completed", "true")
        .await
        .map_err(|e| format!("Failed to mark first launch as completed: {}", e))?;

    log::info!("First launch setup marked as completed");
    Ok(ApiResponse::success(true))
}

#[tauri::command]
pub async fn check_first_launch_status_command(
    state: State<'_, AppStateExt>,
) -> Result<ApiResponse<bool>, String> {
    let is_first_launch = get_setting(&state.db, "first_launch_completed", "false")
        .await
        .map_err(|e| format!("Failed to check first launch status: {}", e))?;

    // Return true if this IS the first launch (first_launch_completed is "false")
    Ok(ApiResponse::success(is_first_launch == "false"))
}
