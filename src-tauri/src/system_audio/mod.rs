use anyhow::{Context, Result};
use std::process::Command;
use tauri::Manager;

/// BlackHole virtual audio driver constants
pub const BLACKHOLE_DRIVER_NAME: &str = "BlackHole2ch";
#[allow(dead_code)]
pub const BLACKHOLE_BUNDLE_ID: &str = "audio.existential.BlackHole2ch";

/// Check if BlackHole virtual audio driver is installed
pub fn is_blackhole_installed() -> bool {
    // Check if BlackHole device exists in system audio devices
    match list_core_audio_devices() {
        Ok(devices) => devices
            .iter()
            .any(|name| name.contains("BlackHole") || name.contains(BLACKHOLE_DRIVER_NAME)),
        Err(_) => false,
    }
}

/// Get the BlackHole device name if installed
pub fn get_blackhole_device_name() -> Option<String> {
    match list_core_audio_devices() {
        Ok(devices) => devices
            .into_iter()
            .find(|name| name.contains("BlackHole") || name.contains(BLACKHOLE_DRIVER_NAME)),
        Err(_) => None,
    }
}

/// List all CoreAudio devices using system_profiler
fn list_core_audio_devices() -> Result<Vec<String>> {
    let output = Command::new("system_profiler")
        .args(["SPAudioDataType", "-json"])
        .output()
        .context("Failed to run system_profiler")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("system_profiler failed"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON output to extract device names
    // This is a simplified check - in production, use proper JSON parsing
    let mut devices = Vec::new();

    // Look for device names in the output
    for line in stdout.lines() {
        if line.contains("_name") {
            // Extract device name from JSON
            if let Some(name) = extract_json_string_value(line) {
                devices.push(name);
            }
        }
    }

    Ok(devices)
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

/// Install BlackHole driver from bundled resources
/// This requires administrator privileges and will prompt the user
#[allow(dead_code)]
pub fn install_blackhole_driver(app_handle: &tauri::AppHandle) -> Result<()> {
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .context("Failed to get resource directory")?;

    let pkg_path = resource_dir.join("BlackHole2ch-0.6.0.pkg");

    if !pkg_path.exists() {
        return Err(anyhow::anyhow!(
            "BlackHole installer not found at {:?}",
            pkg_path
        ));
    }

    // Install using installer command with admin privileges
    let output = Command::new("/usr/sbin/installer")
        .args(["-pkg", pkg_path.to_str().unwrap(), "-target", "/"])
        .output()
        .context("Failed to run installer")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Installation failed: {}", stderr));
    }

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
