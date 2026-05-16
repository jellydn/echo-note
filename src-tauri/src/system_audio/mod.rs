use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashSet;
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
            let found = devices.iter().any(|name| is_blackhole_device_name(name));
            log::info!("BlackHole detection result: {}", found);
            if found {
                true
            } else {
                log::info!("BlackHole not found in JSON device list. Trying fallback method.");
                is_blackhole_installed_fallback()
            }
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
            .find(|name| is_blackhole_device_name(name)),
        Err(_) => None,
    }
}

const AUDIO_DEVICE_NAME_KEYS: &[&str] = &[
    "_name",
    "name",
    "device_name",
    "coreaudio_device_name",
    "spaudio_device_name",
    "spaudio_name",
];

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
    match serde_json::from_str::<Value>(&stdout) {
        Ok(data) => {
            let devices = parse_system_profiler_audio_devices_from_json(&data);
            log::info!("Parsed {} audio devices from JSON", devices.len());
            if devices.is_empty() {
                return Err(anyhow::anyhow!(
                    "system_profiler JSON did not contain audio device names"
                ));
            }
            Ok(devices)
        }
        Err(e) => {
            log::warn!(
                "Failed to parse system_profiler JSON: {}. Falling back to line parsing.",
                e
            );
            Ok(parse_system_profiler_audio_devices_from_text(&stdout))
        }
    }
}

fn parse_system_profiler_audio_devices_from_json(data: &Value) -> Vec<String> {
    let mut devices = Vec::new();
    collect_audio_device_names(data, &mut devices);
    dedupe_device_names(devices)
}

fn collect_audio_device_names(value: &Value, devices: &mut Vec<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_audio_device_names(item, devices);
            }
        }
        Value::Object(map) => {
            for (key, value) in map {
                if is_audio_device_name_key(key) {
                    if let Some(name) = value
                        .as_str()
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                    {
                        devices.push(name.to_string());
                    }
                }
                collect_audio_device_names(value, devices);
            }
        }
        _ => {}
    }
}

fn parse_system_profiler_audio_devices_from_text(stdout: &str) -> Vec<String> {
    let mut devices = Vec::new();

    for line in stdout.lines() {
        if let Some(name) = extract_json_string_value(line) {
            devices.push(name);
            continue;
        }

        if let Some(name) = extract_text_device_heading(line) {
            devices.push(name);
        }
    }

    dedupe_device_names(devices)
}

fn is_audio_device_name_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    AUDIO_DEVICE_NAME_KEYS.contains(&key.as_str())
}

fn is_blackhole_device_name(name: &str) -> bool {
    name.to_ascii_lowercase().contains("blackhole")
}

fn extract_text_device_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let heading = trimmed.strip_suffix(':')?.trim();

    if heading.is_empty() || heading.contains(':') {
        return None;
    }

    let heading_lower = heading.to_ascii_lowercase();
    if matches!(
        heading_lower.as_str(),
        "audio" | "devices" | "input" | "output" | "system profiler"
    ) {
        return None;
    }

    Some(heading.to_string())
}

fn dedupe_device_names(devices: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for device in devices {
        let key = device.to_ascii_lowercase();
        if seen.insert(key) {
            deduped.push(device);
        }
    }

    deduped
}

/// Extract a string value from a JSON key-value line
fn extract_json_string_value(line: &str) -> Option<String> {
    let (key, value_part) = line.split_once(':')?;
    let key = key.trim().trim_matches('"');
    if !is_audio_device_name_key(key) {
        return None;
    }

    let trimmed = value_part.trim().trim_end_matches(',');
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        let value = &trimmed[1..trimmed.len() - 1];
        return serde_json::from_str::<String>(&format!("\"{value}\"")).ok();
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
    // This will prompt the user for admin password via macOS UI.
    // Use AppleScript's `quoted form of` for proper shell escaping.
    let escaped_path = pkg_path
        .to_string_lossy()
        .replace("\\", "\\\\")
        .replace("\"", "\\\"");
    let status = Command::new("osascript")
        .args([
            "-e",
            &format!(
                r#"do shell script "installer -pkg " & quoted form of "{}" & " -target /" with administrator privileges"#,
                escaped_path
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

    #[test]
    fn test_extract_json_string_value_ignores_non_name_keys() {
        let line = r#""manufacturer" : "BlackHole Audio""#;
        assert_eq!(extract_json_string_value(line), None);
    }

    #[test]
    fn test_parse_system_profiler_current_items_shape() {
        let data: Value = serde_json::from_str(
            r#"{
                "SPAudioDataType": [
                    {
                        "_items": [
                            { "_name": "MacBook Pro Speakers" },
                            { "_name": "BlackHole 2ch" }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(
            parse_system_profiler_audio_devices_from_json(&data),
            vec!["MacBook Pro Speakers", "BlackHole 2ch"]
        );
    }

    #[test]
    fn test_parse_system_profiler_named_items_shape() {
        let data: Value = serde_json::from_str(
            r#"{
                "SPAudioDataType": [
                    {
                        "items": [
                            { "name": "Studio Display Speakers" },
                            { "spaudio_device_name": "BlackHole 16ch" }
                        ]
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(
            parse_system_profiler_audio_devices_from_json(&data),
            vec!["Studio Display Speakers", "BlackHole 16ch"]
        );
    }

    #[test]
    fn test_parse_system_profiler_deeply_nested_shape() {
        let data: Value = serde_json::from_str(
            r#"{
                "SPAudioDataType": [
                    {
                        "audio": {
                            "devices": [
                                {
                                    "coreaudio_device_name": "External Headphones",
                                    "channels": 2
                                },
                                {
                                    "device_name": "BlackHole2ch",
                                    "channels": 2
                                }
                            ]
                        }
                    }
                ]
            }"#,
        )
        .unwrap();

        assert_eq!(
            parse_system_profiler_audio_devices_from_json(&data),
            vec!["External Headphones", "BlackHole2ch"]
        );
    }

    #[test]
    fn test_parse_system_profiler_text_output_shape() {
        let stdout = r#"
Audio:

    Devices:

        MacBook Pro Speakers:

          Default Output Device: Yes
          Manufacturer: Apple Inc.

        BlackHole 2ch:

          Input Channels: 2
          Output Channels: 2
"#;

        assert_eq!(
            parse_system_profiler_audio_devices_from_text(stdout),
            vec!["MacBook Pro Speakers", "BlackHole 2ch"]
        );
    }

    #[test]
    fn test_parse_system_profiler_text_json_line_shape() {
        let stdout = r#"
          "_name" : "MacBook Pro Speakers",
          "_name" : "BlackHole \"2ch\"",
"#;

        assert_eq!(
            parse_system_profiler_audio_devices_from_text(stdout),
            vec!["MacBook Pro Speakers", "BlackHole \"2ch\""]
        );
    }
}
