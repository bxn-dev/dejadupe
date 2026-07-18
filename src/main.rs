use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::ProgressBar;

#[derive(Parser)]
#[command(author="Bxn", version, about = "DéjàDupe is a fast Rust CLI for finding, reviewing, deleting, and copying files without duplicates. It scans directories recursively, groups files by size and content hash, reports reclaimable disk space, and can copy directory trees while automatically skipping duplicate content.", long_about = None)]
struct Cli {
    /// Directory to scan; defaults to the current directory.
    path: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Deletes duplicates while retaining the newest copy.
    Delete {
        /// Directory to scan; defaults to the current directory.
        path: Option<PathBuf>,
        /// Performs deletion; without this flag, only shows planned changes.
        #[arg(long)]
        execute: bool,
    },
    /// Copies a tree while skipping duplicate file contents.
    Copy {
        /// Directory tree to copy.
        source: PathBuf,
        /// Destination directory.
        destination: PathBuf,
    },
}

fn scan(path: &Path) -> Result<()> {
    let groups =
        dejadupe::scan(path).with_context(|| format!("failed to scan {}", path.display()))?;

    if groups.is_empty() {
        println!("No duplicate files found.");
        return Ok(());
    }

    for group in &groups {
        println!("\n{} ({} bytes each)", group.hash, group.files[0].size);
        for file in &group.files {
            println!("  {}", file.path.display());
        }
    }

    let total_reclaimable: u64 = groups.iter().map(|group| group.reclaimable_bytes).sum();
    println!(
        "\n{} duplicate groups; {} bytes reclaimable.",
        groups.len(),
        total_reclaimable
    );
    Ok(())
}

fn keeper_index(files: &[dejadupe::FileInfo]) -> Option<usize> {
    files
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            left.modified
                .cmp(&right.modified)
                .then_with(|| right.path.cmp(&left.path))
        })
        .map(|(index, _)| index)
}

fn delete_duplicates(path: &Path, execute: bool) -> Result<()> {
    let groups =
        dejadupe::scan(path).with_context(|| format!("failed to scan {}", path.display()))?;
    let delete_count: usize = groups.iter().map(|group| group.files.len() - 1).sum();

    if delete_count == 0 {
        println!("No duplicate files found.");
        return Ok(());
    }

    let progress = execute.then(|| {
        let progress = ProgressBar::new(delete_count as u64);
        progress.set_message("Deleting duplicates");
        progress
    });
    let mut reclaimed = 0_u64;

    for group in &groups {
        let keeper = keeper_index(&group.files).context("duplicate group contains no files")?;
        if !execute {
            println!("Keep: {}", group.files[keeper].path.display());
        }

        for (index, file) in group.files.iter().enumerate() {
            if index == keeper {
                continue;
            }

            if execute {
                fs::remove_file(&file.path)
                    .with_context(|| format!("failed to delete {}", file.path.display()))?;
                reclaimed += file.size;
                if let Some(progress) = &progress {
                    progress.inc(1);
                }
            } else {
                println!("Would delete: {}", file.path.display());
            }
        }
    }

    if let Some(progress) = progress {
        progress.finish_and_clear();
        println!("Deleted {delete_count} files; reclaimed {reclaimed} bytes.");
    } else {
        println!("\nDry run: would delete {delete_count} files. Pass --execute to apply.");
    }
    Ok(())
}

fn copy_unique(source: &Path, destination: &Path) -> Result<()> {
    let stats = dejadupe::copy_unique(source, destination).with_context(|| {
        format!(
            "failed to copy {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    println!(
        "Copied {} files ({} bytes); skipped {} duplicates.",
        stats.copied_files, stats.copied_bytes, stats.skipped_files
    );
    Ok(())
}

fn run(cli: &Cli) -> Result<()> {
    match &cli.command {
        Some(Commands::Delete { path, execute }) => {
            delete_duplicates(path.as_deref().unwrap_or_else(|| Path::new(".")), *execute)
        }
        Some(Commands::Copy {
            source,
            destination,
        }) => copy_unique(source, destination),
        None => scan(cli.path.as_deref().unwrap_or_else(|| Path::new("."))),
    }
}

fn main() -> Result<()> {
    run(&Cli::parse()).context("failed to run command")
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use super::keeper_index;

    #[test]
    fn keeps_newest_file() {
        let files = [
            dejadupe::FileInfo {
                path: "old".into(),
                size: 1,
                modified: SystemTime::UNIX_EPOCH,
            },
            dejadupe::FileInfo {
                path: "new".into(),
                size: 1,
                modified: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            },
        ];

        assert_eq!(keeper_index(&files), Some(1));
    }
}

// Rust guideline compliant 2026-02-21
