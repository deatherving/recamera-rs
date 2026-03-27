//! File and image storage utilities for the recamera SDK.
//!
//! This crate provides helpers for persisting captured frames and arbitrary data
//! to the filesystem, as well as querying basic file metadata.

use std::fs;
use std::path::{Path, PathBuf};

use recamera_core::{Error, FrameData, ImageFormat, Result};

/// Metadata about a single file on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileInfo {
    /// The canonical path to the file.
    pub path: PathBuf,
    /// Size of the file in bytes.
    pub size: u64,
}

/// Summary of storage capacity for a mount point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageInfo {
    /// Total capacity in bytes.
    pub total_bytes: u64,
    /// Currently available (free) bytes.
    pub available_bytes: u64,
    /// The filesystem mount point (e.g. `"/mnt/sdcard"`).
    pub mount_point: String,
}

/// Write arbitrary bytes to a file, creating parent directories as needed.
///
/// # Errors
///
/// Returns an error if the parent directories cannot be created or the file
/// cannot be written.
pub fn save_file(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, data)?;
    Ok(())
}

/// Save a [`FrameData`] to disk as an image file.
///
/// JPEG frames are written directly (the encoded bytes are already valid JPEG).
/// All other formats are written as raw byte dumps.
///
/// Parent directories are created automatically if they do not exist.
///
/// # Errors
///
/// Returns an error if directories cannot be created or the file cannot be
/// written.
pub fn save_image(path: &Path, frame: &FrameData) -> Result<()> {
    match frame.format {
        ImageFormat::Jpeg => save_file(path, &frame.data),
        _ => save_file(path, &frame.data),
    }
}

/// List all files (not directories) in the given directory, sorted by path.
///
/// The listing is **not** recursive — only direct children of `dir` are
/// returned.
///
/// # Errors
///
/// Returns a [`Storage`](Error::Storage) error if `dir` does not exist or
/// cannot be read.
pub fn list_files(dir: &Path) -> Result<Vec<FileInfo>> {
    let entries = fs::read_dir(dir).map_err(|e| {
        Error::Storage(format!("failed to read directory {}: {e}", dir.display()))
    })?;

    let mut files: Vec<FileInfo> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            Error::Storage(format!("failed to read directory entry: {e}"))
        })?;
        let metadata = entry.metadata().map_err(|e| {
            Error::Storage(format!(
                "failed to read metadata for {}: {e}",
                entry.path().display()
            ))
        })?;
        if metadata.is_file() {
            files.push(FileInfo {
                path: entry.path(),
                size: metadata.len(),
            });
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use recamera_core::ImageFormat;
    use tempfile::TempDir;

    #[test]
    fn save_file_writes_bytes() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("hello.txt");
        save_file(&path, b"hello world").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"hello world");
    }

    #[test]
    fn save_file_creates_nested_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("a").join("b").join("c").join("file.bin");
        save_file(&path, b"nested").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"nested");
    }

    #[test]
    fn save_image_jpeg() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("photo.jpg");
        let frame = FrameData {
            data: vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00],
            width: 1,
            height: 1,
            format: ImageFormat::Jpeg,
            timestamp_ms: 100,
        };
        save_image(&path, &frame).unwrap();
        assert_eq!(fs::read(&path).unwrap(), frame.data);
    }

    #[test]
    fn save_image_raw_format() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("frame.raw");
        let frame = FrameData {
            data: vec![0, 1, 2, 3],
            width: 2,
            height: 2,
            format: ImageFormat::Rgb888,
            timestamp_ms: 200,
        };
        save_image(&path, &frame).unwrap();
        assert_eq!(fs::read(&path).unwrap(), frame.data);
    }

    #[test]
    fn list_files_sorted() {
        let tmp = TempDir::new().unwrap();
        // Create files in reverse alphabetical order.
        fs::write(tmp.path().join("c.txt"), b"c").unwrap();
        fs::write(tmp.path().join("a.txt"), b"a").unwrap();
        fs::write(tmp.path().join("b.txt"), b"bb").unwrap();
        // Create a subdirectory — should be excluded.
        fs::create_dir(tmp.path().join("subdir")).unwrap();

        let files = list_files(tmp.path()).unwrap();
        assert_eq!(files.len(), 3);
        assert!(files[0].path.ends_with("a.txt"));
        assert!(files[1].path.ends_with("b.txt"));
        assert!(files[2].path.ends_with("c.txt"));
        assert_eq!(files[0].size, 1);
        assert_eq!(files[1].size, 2);
        assert_eq!(files[2].size, 1);
    }

    #[test]
    fn list_files_error_on_missing_dir() {
        let result = list_files(Path::new("/nonexistent/path/that/should/not/exist"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Storage(_)));
    }
}
