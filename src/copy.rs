//! Copies directory trees without duplicate file contents.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use indicatif::ProgressBar;

use crate::datastructs::{CopyStats, FileInfo};
use crate::hasher::hash_files;
use crate::scanner::scan_directory;

/// Copies unique file contents from `source` into `destination`.
///
/// Existing destination contents and repeated source contents are skipped.
/// Symbolic links and empty directories are not copied.
///
/// # Errors
///
/// Returns an error for overlapping trees, unreadable files, destination
/// conflicts, or failed directory and file operations.
pub fn copy_unique(source: &Path, destination: &Path) -> Result<CopyStats> {
    let source = source
        .canonicalize()
        .with_context(|| format!("failed to resolve source {}", source.display()))?;
    if !source.is_dir() {
        bail!("source is not a directory: {}", source.display());
    }

    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;
    let destination = destination
        .canonicalize()
        .with_context(|| format!("failed to resolve destination {}", destination.display()))?;
    if source.starts_with(&destination) || destination.starts_with(&source) {
        bail!("source and destination must not overlap");
    }

    let destination_files = scan_directory(&destination)?;
    let mut seen: HashSet<_> = hash_files(destination_files, "Hashing destination")?
        .into_iter()
        .map(|(hash, _)| hash)
        .collect();

    let mut source_files = scan_directory(&source)?;
    source_files.sort_by(|left, right| left.path.cmp(&right.path));
    let source_count = source_files.len();
    let hashed = hash_files(source_files, "Hashing source")?;

    let mut copies: Vec<(FileInfo, PathBuf)> = Vec::new();
    for (hash, file) in hashed {
        if !seen.insert(hash) {
            continue;
        }

        let relative = file
            .path
            .strip_prefix(&source)
            .with_context(|| format!("{} is outside the source", file.path.display()))?;
        let target = destination.join(relative);
        if target.exists() {
            bail!("destination path already exists: {}", target.display());
        }
        copies.push((file, target));
    }

    let progress = ProgressBar::new(copies.len() as u64);
    progress.set_message("Copying unique files");
    let mut copied_bytes = 0_u64;
    for (file, target) in &copies {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        copied_bytes += fs::copy(&file.path, target).with_context(|| {
            format!(
                "failed to copy {} to {}",
                file.path.display(),
                target.display()
            )
        })?;
        progress.inc(1);
    }
    progress.finish_and_clear();

    Ok(CopyStats {
        copied_files: copies.len(),
        skipped_files: source_count - copies.len(),
        copied_bytes,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use anyhow::Result;

    use super::copy_unique;

    #[test]
    fn copies_each_content_once() -> Result<()> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!("dejadupe-{}-{unique}", std::process::id()));
        let source = root.join("source");
        let destination = root.join("destination");
        fs::create_dir_all(&source)?;
        fs::create_dir_all(&destination)?;
        fs::write(source.join("a"), b"same")?;
        fs::write(source.join("b"), b"same")?;
        fs::write(source.join("c"), b"existing")?;
        fs::write(destination.join("existing"), b"existing")?;

        let stats = copy_unique(&source, &destination)?;

        assert_eq!(stats.copied_files, 1);
        assert_eq!(stats.skipped_files, 2);
        assert!(destination.join("a").exists());
        assert!(!destination.join("b").exists());
        fs::remove_dir_all(root)?;
        Ok(())
    }
}

// Rust guideline compliant 2026-02-21
