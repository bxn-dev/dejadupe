# DéjàDupe

Fast CLI for finding, deleting, and copying files without duplicate contents.

## What it does

- scans directories recursively and groups identical files
- deletes duplicates while keeping the newest file; dry-run by default
- copies a directory tree while skipping contents already present in the destination

Symbolic links are not followed.

## Install

Download the archive for Windows, Linux, or macOS from **GitHub Releases**, verify it with `SHA256SUMS`, extract it, and place `dejadupe` (`dejadupe.exe` on Windows) on your `PATH`.

From source, with stable Rust installed:

```sh
cargo install --path .
```

## Usage

```sh
# Scan the current directory
dejadupe

# Scan another directory
dejadupe /path/to/files

# Preview duplicate deletion
dejadupe delete /path/to/files

# Delete duplicates, keeping the newest copy
dejadupe delete /path/to/files --execute

# Copy only contents not already present at the destination
dejadupe copy /path/to/source /path/to/destination
```

Copying aborts rather than overwriting a destination path containing different data.

> `delete --execute` permanently removes files. Review the dry-run output first.

## License

[MIT](LICENSE)
