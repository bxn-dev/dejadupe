//! Discovers regular files below a directory.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use indicatif::ProgressBar;
use walkdir::WalkDir;

use crate::datastructs::FileInfo;

/// Recursively scans `path` for regular files.
///
/// Symbolic links are not followed.
///
/// # Errors
///
/// Returns an error when a directory entry or its metadata cannot be read.
pub fn scan_directory(path: &Path) -> Result<Vec<FileInfo>> {
    let progress = ProgressBar::new_spinner();
    progress.set_message("Scanning files");
    progress.enable_steady_tick(Duration::from_millis(100));

    let mut files = Vec::new();
    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to walk {}", path.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let metadata = entry
            .metadata()
            .with_context(|| format!("failed to read metadata for {}", entry.path().display()))?;
        let modified = metadata.modified().with_context(|| {
            format!(
                "failed to read modification time for {}",
                entry.path().display()
            )
        })?;

        files.push(FileInfo {
            path: entry.into_path(),
            size: metadata.len(),
            modified,
        });
        progress.inc(1);
    }

    progress.finish_and_clear();
    Ok(files)
}

// Rust guideline compliant 2026-02-21
