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

/// Derive a recording's age and size from a metadata read, or `None` when the
/// read fails. Callers must skip files whose metadata can't be read rather
/// than guessing an age — a failed read is not evidence the file is old, and
/// the old UNIX_EPOCH fallback deleted unreadable files without inspection.
fn age_and_size_from_metadata(
    metadata: std::io::Result<std::fs::Metadata>,
    now: SystemTime,
) -> Option<(Duration, u64)> {
    let metadata = metadata.ok()?;
    let modified = metadata.modified().ok()?;
    let age = now.duration_since(modified).unwrap_or(Duration::ZERO);
    Some((age, metadata.len()))
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
        let Some((age, size)) = age_and_size_from_metadata(entry.metadata(), now) else {
            log::warn!("Skipping {:?}: failed to read metadata or modified time", path);
            continue;
        };

        if age > retention {
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

    #[cfg(unix)]
    #[test]
    fn cleanup_skips_entries_whose_metadata_cannot_be_read() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();

        // A dangling symlink resolves to no target, so follow-based reads of
        // it fail (is_file() on Unix; DirEntry::metadata() on Windows, where
        // it traverses links). Either way it must be skipped, never deleted.
        // On Unix this entry is intercepted by the is_file() pre-check rather
        // than the metadata guard, so this test locks the observable contract
        // end-to-end; the deterministic guard behavior is covered by
        // metadata_read_failure_skips_instead_of_guessing_epoch_age below.
        std::os::unix::fs::symlink(dir.join("missing-target.wav"), dir.join("broken.wav"))
            .unwrap();
        // A genuinely expired recording is still cleaned up in the same pass.
        write_file(&dir, "old.wav", 40 * 24);

        let summary = cleanup_expired_recordings(&dir, 30).unwrap();

        assert_eq!(summary.deleted_count, 1);
        assert!(
            dir.join("broken.wav").symlink_metadata().is_ok(),
            "entries whose metadata can't be read must be skipped, not deleted"
        );
        assert!(!dir.join("old.wav").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn metadata_read_failure_skips_instead_of_guessing_epoch_age() {
        // A failed stat must yield None (skip), never a ~56-year-old age that
        // would delete the file without inspection.
        let err = std::io::Error::other("simulated stat failure");
        assert_eq!(age_and_size_from_metadata(Err(err), SystemTime::now()), None);
    }

    #[test]
    fn readable_metadata_yields_age_and_size() {
        // Companion to the failure test above: readable metadata must yield
        // Some(age, size), proving the helper skips only on genuine failures
        // and doesn't over-skip valid recordings.
        let dir = temp_dir();
        let size = write_file(&dir, "probe.wav", 1);
        let metadata = std::fs::metadata(dir.join("probe.wav")).unwrap();
        let age = age_and_size_from_metadata(Ok(metadata), SystemTime::now());
        assert!(age.is_some(), "readable metadata should yield an age");
        assert_eq!(age.unwrap().1, size);
        std::fs::remove_dir_all(&dir).unwrap();
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
