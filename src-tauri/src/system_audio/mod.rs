use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Command;

/// BlackHole virtual audio driver constants
#[allow(dead_code)]
pub const BLACKHOLE_DRIVER_NAME: &str = "BlackHole2ch";
#[allow(dead_code)]
pub const BLACKHOLE_BUNDLE_ID: &str = "audio.existential.BlackHole2ch";

/// Check if BlackHole virtual audio driver is installed
pub fn is_blackhole_installed() -> bool {
    // First try the system_profiler JSON method
    match list_core_audio_devices() {
        Ok(devices) => {
            log::info!("Found audio devices: {:?}", devices);
            let found = devices.iter().any(|name| {
                let name_lower = name.to_lowercase();
                name_lower.contains("blackhole")
            });
            log::info!("BlackHole detection result: {}", found);
            found
        }
        Err(e) => {
            log::warn!(
                "Failed to list audio devices: {}. Trying fallback method.",
                e
            );
            // Fallback to simple grep method
            is_blackhole_installed_fallback()
        }
    }
}

/// Fallback detection method using system_profiler without JSON
fn is_blackhole_installed_fallback() -> bool {
    match Command::new("system_profiler")
        .args(["SPAudioDataType"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let found = stdout.to_lowercase().contains("blackhole");
            log::info!("Fallback detection result: {}", found);
            if found {
                return true;
            }
        }
        _ => {
            log::error!("system_profiler detection failed");
        }
    }
    // Check for the HAL driver directly on disk
    is_blackhole_hal_driver_present()
}

/// Check if the BlackHole HAL audio driver is installed on disk
fn is_blackhole_hal_driver_present() -> bool {
    let hal_dir = std::path::Path::new("/Library/Audio/Plug-Ins/HAL");
    if let Ok(entries) = std::fs::read_dir(hal_dir) {
        for entry in entries.flatten() {
            if entry
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .contains("blackhole")
            {
                log::info!("Found BlackHole HAL driver at {:?}", entry.path());
                return true;
            }
        }
    }
    log::info!("No BlackHole HAL driver found in /Library/Audio/Plug-Ins/HAL");
    false
}

/// Get the BlackHole device name if installed
pub fn get_blackhole_device_name() -> Option<String> {
    match list_core_audio_devices() {
        Ok(devices) => devices
            .into_iter()
            .find(|name| name.to_lowercase().contains("blackhole")),
        Err(_) => None,
    }
}

/// System profiler audio data structure
#[derive(Debug, Deserialize)]
struct SystemProfilerAudioData {
    #[serde(rename = "SPAudioDataType")]
    audio_data: Vec<AudioDeviceList>,
}

#[derive(Debug, Deserialize)]
struct AudioDeviceList {
    #[serde(rename = "_items")]
    items: Option<Vec<AudioDevice>>,
}

#[derive(Debug, Deserialize)]
struct AudioDevice {
    #[serde(rename = "_name")]
    name: String,
}

/// List all CoreAudio devices using system_profiler with proper JSON parsing
fn list_core_audio_devices() -> Result<Vec<String>> {
    let output = Command::new("system_profiler")
        .args(["SPAudioDataType", "-json"])
        .output()
        .context("Failed to run system_profiler")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "system_profiler failed with status: {:?}",
            output.status
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Try to parse as JSON first
    match serde_json::from_str::<SystemProfilerAudioData>(&stdout) {
        Ok(data) => {
            let mut devices = Vec::new();
            for device_list in data.audio_data {
                if let Some(items) = device_list.items {
                    for device in items {
                        devices.push(device.name);
                    }
                }
            }
            log::info!("Parsed {} audio devices from JSON", devices.len());
            Ok(devices)
        }
        Err(e) => {
            log::warn!(
                "Failed to parse system_profiler JSON: {}. Falling back to line parsing.",
                e
            );
            // Fallback to line-by-line parsing
            let mut devices = Vec::new();
            for line in stdout.lines() {
                if line.contains("_name") {
                    if let Some(name) = extract_json_string_value(line) {
                        devices.push(name);
                    }
                }
            }
            Ok(devices)
        }
    }
}

/// Extract a string value from a JSON key-value line
fn extract_json_string_value(line: &str) -> Option<String> {
    // Look for pattern: "_name" : "Device Name"
    if let Some(start) = line.find(':') {
        let value_part = &line[start + 1..];
        let trimmed = value_part.trim();
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            let value = &trimmed[1..trimmed.len() - 1];
            return Some(value.to_string());
        }
    }
    None
}

/// Install BlackHole driver by opening official download page
pub fn install_blackhole_driver(_app_handle: &tauri::AppHandle) -> Result<()> {
    // Open the official BlackHole GitHub releases page
    std::process::Command::new("open")
        .arg("https://github.com/ExistentialAudio/BlackHole")
        .spawn()
        .context("Failed to open BlackHole download page. Please visit https://github.com/ExistentialAudio/BlackHole manually.")?;

    Ok(())
}

/// Check if Homebrew is installed
pub fn is_homebrew_installed() -> bool {
    Command::new("which")
        .arg("brew")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Install BlackHole via Homebrew by opening Terminal
/// The pkg installation requires admin privileges, so we open Terminal
/// for the user to enter their password interactively.
pub fn install_blackhole_via_homebrew() -> Result<()> {
    // Check if Homebrew is available
    if !is_homebrew_installed() {
        return Err(anyhow::anyhow!(
            "Homebrew is not installed. Please install Homebrew first (https://brew.sh) or use the manual download option."
        ));
    }

    // Open Terminal with the brew reinstall command so the user can enter their password
    Command::new("osascript")
        .args([
            "-e",
            r#"tell application "Terminal"
    activate
    do script "brew reinstall blackhole-2ch && echo '✅ BlackHole installed! You can close this window.' || echo '❌ Installation failed.'"
end tell"#,
        ])
        .spawn()
        .context("Failed to open Terminal. Please run 'brew reinstall blackhole-2ch' manually in Terminal.")?;

    Ok(())
}

/// Check if we should use BlackHole for system audio capture
/// Returns true if BlackHole is available and should be used
#[allow(dead_code)]
pub fn should_use_blackhole() -> bool {
    is_blackhole_installed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_string_value() {
        let line = r#""_name" : "MacBook Pro Speakers""#;
        assert_eq!(
            extract_json_string_value(line),
            Some("MacBook Pro Speakers".to_string())
        );
    }
}
