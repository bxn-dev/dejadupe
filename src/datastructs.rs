//! Contains scan and copy results.

use std::path::PathBuf;
use std::time::SystemTime;

/// Describes a file discovered during scanning.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FileInfo {
    /// Absolute or scan-root-relative path reported by the directory walker.
    pub path: PathBuf,
    /// File size in bytes.
    pub size: u64,
    /// Modification time used to select the newest copy.
    pub modified: SystemTime,
}

/// Describes files with identical size and contents.
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    /// BLAKE3 hash shared by every file in this group.
    pub hash: blake3::Hash,
    /// Files with identical contents.
    pub files: Vec<FileInfo>,
    /// Bytes recoverable when one copy is retained.
    pub reclaimable_bytes: u64,
}

/// Summarizes a duplicate-aware copy operation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CopyStats {
    /// Files copied into the destination.
    pub copied_files: usize,
    /// Files skipped because their content already existed.
    pub skipped_files: usize,
    /// Bytes written into the destination.
    pub copied_bytes: u64,
}
