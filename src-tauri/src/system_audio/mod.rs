use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Command;
use tauri::Manager;

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

/// Path to the bundled BlackHole installer package
pub const BUNDLED_PKG_NAME: &str = "BlackHole2ch-0.6.0.pkg";

/// Check if the bundled BlackHole installer exists in resources
pub fn is_bundled_installer_available(app_handle: &tauri::AppHandle) -> bool {
    match get_bundled_pkg_path(app_handle) {
        Ok(path) => path.exists(),
        Err(_) => false,
    }
}

/// Get the path to the bundled BlackHole installer
fn get_bundled_pkg_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf> {
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .context("Failed to get resource directory")?;
    let pkg_path = resource_dir.join("resources").join(BUNDLED_PKG_NAME);
    Ok(pkg_path)
}

/// Install BlackHole from the bundled .pkg installer
/// Opens a privileged installer dialog for the user to complete installation
pub fn install_blackhole_from_bundle(app_handle: &tauri::AppHandle) -> Result<()> {
    let pkg_path = get_bundled_pkg_path(app_handle)?;

    if !pkg_path.exists() {
        return Err(anyhow::anyhow!(
            "Bundled installer not found at {:?}. Falling back to download method.",
            pkg_path
        ));
    }

    log::info!("Installing BlackHole from bundled package: {:?}", pkg_path);

    // Use macOS installer command with a privileged helper
    // This will prompt the user for admin password via macOS UI
    let status = Command::new("osascript")
        .args([
            "-e",
            &format!(
                r#"do shell script "installer -pkg '{}' -target /" with administrator privileges"#,
                pkg_path.to_string_lossy().replace("'", "'\"'\"'")
            ),
        ])
        .status()
        .context("Failed to execute installer script")?;

    if status.success() {
        log::info!("BlackHole installation completed successfully");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Installation failed with status: {:?}. The user may have cancelled the installation.",
            status
        ))
    }
}

/// Auto-install BlackHole using the best available method
/// 1. Try bundled installer first (if available)
/// 2. Fall back to Homebrew if available
/// 3. Fall back to opening download page as last resort
pub fn auto_install_blackhole(app_handle: &tauri::AppHandle) -> Result<BlackHoleInstallMethod> {
    // Try bundled installer first
    if is_bundled_installer_available(app_handle) {
        log::info!("Attempting to install BlackHole from bundled package...");
        match install_blackhole_from_bundle(app_handle) {
            Ok(_) => return Ok(BlackHoleInstallMethod::Bundled),
            Err(e) => {
                log::warn!("Bundled installation failed: {}. Trying Homebrew...", e);
            }
        }
    }

    // Fall back to Homebrew
    if is_homebrew_installed() {
        log::info!("Attempting to install BlackHole via Homebrew...");
        install_blackhole_via_homebrew()?;
        return Ok(BlackHoleInstallMethod::Homebrew);
    }

    // Last resort: open download page
    log::info!("No auto-install method available. Opening download page...");
    install_blackhole_driver(app_handle)?;
    Ok(BlackHoleInstallMethod::Manual)
}

/// Installation method used for BlackHole
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum BlackHoleInstallMethod {
    Bundled,
    Homebrew,
    Manual,
    #[allow(dead_code)]
    AlreadyInstalled,
}

/// Check and install BlackHole if needed on first launch
/// Returns the installation method used or AlreadyInstalled if already present
#[allow(dead_code)]
pub fn setup_blackhole_on_first_launch(
    app_handle: &tauri::AppHandle,
) -> Result<BlackHoleInstallMethod> {
    if is_blackhole_installed() {
        log::info!("BlackHole is already installed, skipping setup");
        return Ok(BlackHoleInstallMethod::AlreadyInstalled);
    }

    log::info!("BlackHole not found, attempting auto-install...");
    auto_install_blackhole(app_handle)
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
