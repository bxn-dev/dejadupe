//! Groups files by size and content hash.

use std::collections::HashMap;

use anyhow::Result;

use crate::datastructs::{DuplicateGroup, FileInfo};
use crate::hasher::hash_files;

/// Finds files with identical contents.
///
/// Files are grouped by size before hashing to avoid unnecessary disk reads.
///
/// # Errors
///
/// Returns an error when a candidate file cannot be opened or read.
pub fn find_duplicates(files: Vec<FileInfo>) -> Result<Vec<DuplicateGroup>> {
    let mut size_groups: HashMap<u64, Vec<FileInfo>> = HashMap::new();
    for file in files {
        size_groups.entry(file.size).or_default().push(file);
    }

    let candidates: Vec<_> = size_groups
        .into_values()
        .filter(|files| files.len() > 1)
        .flatten()
        .collect();
    let hashed = hash_files(candidates, "Hashing candidates")?;

    let mut hash_groups: HashMap<blake3::Hash, Vec<FileInfo>> = HashMap::new();
    for (hash, file) in hashed {
        hash_groups.entry(hash).or_default().push(file);
    }

    let mut duplicates = Vec::new();
    for (hash, mut files) in hash_groups.into_iter().filter(|(_, files)| files.len() > 1) {
        files.sort_by(|left, right| left.path.cmp(&right.path));
        let reclaimable_bytes = files[0].size * (files.len() as u64 - 1);
        duplicates.push(DuplicateGroup {
            hash,
            files,
            reclaimable_bytes,
        });
    }

    duplicates.sort_by(|left, right| {
        right
            .reclaimable_bytes
            .cmp(&left.reclaimable_bytes)
            .then_with(|| left.files[0].path.cmp(&right.files[0].path))
    });
    Ok(duplicates)
}

// Rust guideline compliant 2026-02-21
