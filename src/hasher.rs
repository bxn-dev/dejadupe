//! Computes content hashes for duplicate detection.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};
use blake3::Hash;
use indicatif::ProgressBar;
use rayon::prelude::*;

use crate::datastructs::FileInfo;

const HASH_BUFFER_SIZE: usize = 64 * 1024;

/// Computes the BLAKE3 hash of a file.
///
/// # Errors
///
/// Returns an error when the file cannot be opened or read.
pub(crate) fn hash_file(path: &Path) -> Result<Hash> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    // This size balances syscall overhead and stack usage for sequential reads.
    let mut buffer = [0_u8; HASH_BUFFER_SIZE];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize())
}

/// Hashes files in parallel while reporting progress.
///
/// # Errors
///
/// Returns an error when any file cannot be opened or read.
pub(crate) fn hash_files(
    files: Vec<FileInfo>,
    message: &'static str,
) -> Result<Vec<(Hash, FileInfo)>> {
    let progress = ProgressBar::new(files.len() as u64);
    progress.set_message(message);

    let hashed = files
        .into_par_iter()
        .map(|file| {
            let hash = hash_file(&file.path)?;
            progress.inc(1);
            Ok((hash, file))
        })
        .collect();
    progress.finish_and_clear();
    hashed
}

// Rust guideline compliant 2026-02-21
