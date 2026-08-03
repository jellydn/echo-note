//! Audio file retention and cleanup.
//!
//! Recordings are stored as WAV files in the app data directory. Without a
//! retention policy these files accumulate forever, growing storage
//! unboundedly (issue #12). This module provides:
//!
//! - [`cleanup_expired_recordings`] — delete WAV files older than a retention
//!   window.
//! - [`storage_usage`] — report how much disk the recordings directory uses,
//!   so the UI can surface storage pressure.
//!
//! Cleanup is opt-in per retention setting (`audio_retention_days`, default
//! 30). A value of `0` disables automatic cleanup entirely.

use anyhow::{Context, Result};
use std::path::Path;
use std::time::{Duration, SystemTime};

/// File extensions treated as recordings.
const RECORDING_EXTENSIONS: &[&str] = &["wav"];

/// Summary of a cleanup pass.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CleanupSummary {
    pub deleted_count: usize,
    pub freed_bytes: u64,
    pub retention_days: u64,
}

/// Disk usage of the recordings directory.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct StorageUsage {
    pub file_count: usize,
    pub total_bytes: u64,
    pub recordings_dir: String,
}

/// Delete recording files older than `retention_days`. Returns how many files
/// were removed and how much space was freed. `retention_days == 0` disables
/// cleanup (returns an empty summary without touching anything).
pub fn cleanup_expired_recordings(
    recordings_dir: &Path,
    retention_days: u64,
) -> Result<CleanupSummary> {
    if retention_days == 0 {
        log::info!("Audio retention disabled (0 days) — skipping cleanup");
        return Ok(CleanupSummary {
            retention_days,
            ..CleanupSummary::default()
        });
    }

    if !recordings_dir.exists() {
        return Ok(CleanupSummary {
            retention_days,
            ..CleanupSummary::default()
        });
    }

    let retention = Duration::from_secs(retention_days.saturating_mul(24 * 60 * 60));
    let now = SystemTime::now();
    let mut summary = CleanupSummary {
        retention_days,
        ..CleanupSummary::default()
    };

    for entry in std::fs::read_dir(recordings_dir)
        .with_context(|| format!("Failed to read recordings dir {:?}", recordings_dir))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_recording_file(&path) {
            continue;
        }

        // Skip files whose metadata can't be read rather than guessing an age:
        // falling back to UNIX_EPOCH would make an unreadable file look ~56
        // years old and delete it without inspection.
        let Ok(metadata) = entry.metadata() else {
            log::warn!("Skipping {:?}: failed to read metadata", path);
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            log::warn!("Skipping {:?}: failed to read modified time", path);
            continue;
        };

        let age = now.duration_since(modified).unwrap_or(Duration::ZERO);
        if age > retention {
            let size = metadata.len();
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    summary.deleted_count += 1;
                    summary.freed_bytes += size;
                    log::info!("Removed expired recording {:?} (age {:?})", path, age);
                }
                Err(e) => log::warn!("Failed to remove {:?}: {}", path, e),
            }
        }
    }

    log::info!(
        "Cleanup pass complete: {} files, {} bytes freed",
        summary.deleted_count,
        summary.freed_bytes
    );
    Ok(summary)
}

/// Compute the total size and count of recording files in the directory.
pub fn storage_usage(recordings_dir: &Path) -> Result<StorageUsage> {
    let mut usage = StorageUsage::default();
    if !recordings_dir.exists() {
        usage.recordings_dir = recordings_dir.display().to_string();
        return Ok(usage);
    }

    for entry in std::fs::read_dir(recordings_dir)
        .with_context(|| format!("Failed to read recordings dir {:?}", recordings_dir))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_recording_file(&path) {
            continue;
        }
        if let Ok(metadata) = entry.metadata() {
            usage.file_count += 1;
            usage.total_bytes += metadata.len();
        }
    }

    usage.recordings_dir = recordings_dir.display().to_string();
    Ok(usage)
}

fn is_recording_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| RECORDING_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("echo-note-cleanup-test-{}", uuid::Uuid::new_v4()))
    }

    fn write_file(dir: &Path, name: &str, age_hours: u64) -> u64 {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        let bytes = vec![0u8; 1024];
        std::fs::write(&path, &bytes).unwrap();
        let mtime = SystemTime::now() - Duration::from_secs(age_hours * 60 * 60);
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        file.set_times(std::fs::FileTimes::new().set_modified(mtime))
            .unwrap();
        bytes.len() as u64
    }

    #[test]
    fn cleanup_removes_only_expired_files() {
        let dir = temp_dir();
        write_file(&dir, "old.wav", 40 * 24); // 40 days
        write_file(&dir, "new.wav", 1); // 1 hour

        let summary = cleanup_expired_recordings(&dir, 30).unwrap();

        assert_eq!(summary.deleted_count, 1);
        assert_eq!(summary.freed_bytes, 1024);
        assert!(!dir.join("old.wav").exists());
        assert!(dir.join("new.wav").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cleanup_ignores_non_recording_files() {
        let dir = temp_dir();
        write_file(&dir, "notes.txt", 40 * 24);
        write_file(&dir, "keep.wav", 40 * 24);

        let summary = cleanup_expired_recordings(&dir, 30).unwrap();

        assert_eq!(summary.deleted_count, 1);
        assert!(dir.join("notes.txt").exists(), "non-audio files must be kept");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cleanup_zero_days_disables_cleanup() {
        let dir = temp_dir();
        write_file(&dir, "old.wav", 40 * 24);

        let summary = cleanup_expired_recordings(&dir, 0).unwrap();
        assert_eq!(summary.deleted_count, 0);
        assert!(dir.join("old.wav").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn cleanup_missing_dir_returns_empty_summary() {
        let dir = temp_dir();
        let summary = cleanup_expired_recordings(&dir, 30).unwrap();
        assert_eq!(summary.deleted_count, 0);
    }

    #[test]
    fn storage_usage_counts_recording_bytes() {
        let dir = temp_dir();
        write_file(&dir, "a.wav", 1);
        write_file(&dir, "b.wav", 1);
        write_file(&dir, "notes.txt", 1);

        let usage = storage_usage(&dir).unwrap();

        assert_eq!(usage.file_count, 2);
        assert_eq!(usage.total_bytes, 2048);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn storage_usage_missing_dir_is_zero() {
        let usage = storage_usage(&temp_dir()).unwrap();
        assert_eq!(usage.file_count, 0);
        assert_eq!(usage.total_bytes, 0);
    }
}
