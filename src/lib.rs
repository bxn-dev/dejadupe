//! Powers DéjàDupe duplicate-file operations.

mod copy;

mod datastructs;
mod duplicate;
mod hasher;
mod scanner;

use std::path::Path;

use anyhow::Result;

#[doc(inline)]
pub use datastructs::{CopyStats, DuplicateGroup, FileInfo};

/// Scans a directory and returns duplicate groups.
///
/// Results are ordered by reclaimable bytes in descending order.
///
/// # Errors
///
/// Returns an error when directories or candidate files cannot be read.
pub fn scan(path: &Path) -> Result<Vec<DuplicateGroup>> {
    duplicate::find_duplicates(scanner::scan_directory(path)?)
}

/// Copies unique file contents between directory trees.
///
/// Files already present by content are skipped.
///
/// # Errors
///
/// Returns an error when paths overlap, files cannot be read, destination
/// paths conflict, or copying fails.
pub fn copy_unique(source: &Path, destination: &Path) -> Result<CopyStats> {
    copy::copy_unique(source, destination)
}

// Rust guideline compliant 2026-02-21
