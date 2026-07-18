use std::error::Error;
use std::fs::{self, File, FileTimes};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct TestDir(PathBuf);

impl TestDir {
    fn new(name: &str) -> TestResult<Self> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path =
            std::env::temp_dir().join(format!("dejadupe-{name}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dejadupe"))
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_at(path: &Path, contents: &[u8], modified: SystemTime) -> TestResult {
    fs::write(path, contents)?;
    File::options()
        .write(true)
        .open(path)?
        .set_times(FileTimes::new().set_modified(modified))?;
    Ok(())
}

#[test]
fn scan_reports_duplicate_groups() -> TestResult {
    let root = TestDir::new("scan-duplicates")?;
    fs::write(root.join("a.txt"), b"same")?;
    fs::write(root.join("b.txt"), b"same")?;
    fs::write(root.join("unique.txt"), b"different")?;

    let output = command().arg(root.path()).output()?;

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 duplicate groups; 4 bytes reclaimable."));
    assert!(stdout.contains("a.txt"));
    assert!(stdout.contains("b.txt"));
    assert!(!stdout.contains("unique.txt"));
    Ok(())
}

#[test]
fn scan_reports_when_no_duplicates_exist() -> TestResult {
    let root = TestDir::new("scan-unique")?;
    fs::write(root.join("a.txt"), b"one")?;
    fs::write(root.join("b.txt"), b"two")?;

    let output = command().arg(root.path()).output()?;

    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "No duplicate files found."
    );
    Ok(())
}

#[test]
fn delete_is_a_dry_run_by_default() -> TestResult {
    let root = TestDir::new("delete-dry-run")?;
    let old = root.join("old.txt");
    let new = root.join("new.txt");
    let now = SystemTime::now();
    write_at(&old, b"same", now - Duration::from_secs(60))?;
    write_at(&new, b"same", now)?;

    let output = command().arg("delete").arg(root.path()).output()?;

    assert_success(&output);
    assert!(old.exists());
    assert!(new.exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Keep:"));
    assert!(stdout.contains("Would delete:"));
    assert!(stdout.contains("Dry run: would delete 1 files."));
    Ok(())
}

#[test]
fn delete_keeps_the_newest_copy_by_default() -> TestResult {
    let root = TestDir::new("delete-newest")?;
    let old = root.join("old.txt");
    let new = root.join("new.txt");
    let now = SystemTime::now();
    write_at(&old, b"same", now - Duration::from_secs(60))?;
    write_at(&new, b"same", now)?;

    let output = command()
        .arg("delete")
        .arg(root.path())
        .arg("--execute")
        .output()?;

    assert_success(&output);
    assert!(!old.exists());
    assert!(new.exists());
    Ok(())
}

#[test]
fn delete_can_keep_the_oldest_copy() -> TestResult {
    let root = TestDir::new("delete-oldest")?;
    let old = root.join("old.txt");
    let new = root.join("new.txt");
    let now = SystemTime::now();
    write_at(&old, b"same", now - Duration::from_secs(60))?;
    write_at(&new, b"same", now)?;

    let output = command()
        .arg("delete")
        .arg(root.path())
        .args(["--keep-oldest", "--execute"])
        .output()?;

    assert_success(&output);
    assert!(old.exists());
    assert!(!new.exists());
    Ok(())
}

#[test]
fn copy_skips_source_and_destination_duplicates() -> TestResult {
    let root = TestDir::new("copy-unique")?;
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(source.join("nested"))?;
    fs::create_dir_all(&destination)?;
    fs::write(source.join("a.txt"), b"same")?;
    fs::write(source.join("b.txt"), b"same")?;
    fs::write(source.join("nested/unique.txt"), b"unique")?;
    fs::write(destination.join("existing.txt"), b"same")?;

    let output = command()
        .arg("copy")
        .arg(&source)
        .arg(&destination)
        .output()?;

    assert_success(&output);
    assert!(!destination.join("a.txt").exists());
    assert!(!destination.join("b.txt").exists());
    assert_eq!(fs::read(destination.join("nested/unique.txt"))?, b"unique");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Copied 1 files"));
    Ok(())
}

#[test]
fn copy_rejects_conflicting_destination_paths_before_copying() -> TestResult {
    let root = TestDir::new("copy-conflict")?;
    let source = root.join("source");
    let destination = root.join("destination");
    fs::create_dir_all(&source)?;
    fs::create_dir_all(&destination)?;
    fs::write(source.join("conflict.txt"), b"source")?;
    fs::write(source.join("other.txt"), b"other")?;
    fs::write(destination.join("conflict.txt"), b"destination")?;

    let output = command()
        .arg("copy")
        .arg(&source)
        .arg(&destination)
        .output()?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("destination path already exists"));
    assert!(!destination.join("other.txt").exists());
    assert_eq!(fs::read(destination.join("conflict.txt"))?, b"destination");
    Ok(())
}

#[test]
fn copy_rejects_overlapping_trees() -> TestResult {
    let root = TestDir::new("copy-overlap")?;
    let source = root.join("source");
    let destination = source.join("destination");
    fs::create_dir_all(&source)?;
    fs::write(source.join("file.txt"), b"content")?;

    let output = command()
        .arg("copy")
        .arg(&source)
        .arg(&destination)
        .output()?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("must not overlap"));
    Ok(())
}

// Rust guideline compliant 2026-02-21
