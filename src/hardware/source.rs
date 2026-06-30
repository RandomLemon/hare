use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Backend abstraction for accessing kernel-provided hardware data.
///
/// All metric implementations read and write through a `Source` instead of
/// touching `std::fs` directly. This keeps them pure and testable: real
/// deployments use [`SysfsSource`], while tests can supply a fake source that
/// serves in-memory contents.
pub trait Source: Send + Sync {
    /// Read a file's full contents into a `String`.
    fn read_to_string(&self, path: &Path) -> Result<String>;

    /// Write `content` to a file, truncating it.
    fn write(&self, path: &Path, content: &str) -> Result<()>;

    /// Return whether `path` exists.
    fn exists(&self, path: &Path) -> bool;

    /// List the direct children of a directory, returning their paths.
    /// Implementations should return paths in a stable (e.g. sorted) order.
    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
}

/// `Source` backed by the real sysfs/procfs filesystem.
#[derive(Default, Clone, Copy, Debug)]
pub struct SysfsSource;

impl SysfsSource {
    pub fn new() -> Self {
        Self
    }
}

impl Source for SysfsSource {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))
    }

    fn write(&self, path: &Path, content: &str) -> Result<()> {
        fs::write(path, content)
            .with_context(|| format!("failed to write {}", path.display()))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let entries = fs::read_dir(path)
            .with_context(|| format!("failed to read dir {}", path.display()))?;
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        paths.sort();
        Ok(paths)
    }
}
