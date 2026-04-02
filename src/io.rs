use crate::errors::PdtkError;
use std::fs;
use std::path::{Path, PathBuf};

/// Read a .pd file from disk.
pub fn read_patch_file(path: &str) -> Result<String, PdtkError> {
    fs::read_to_string(path).map_err(PdtkError::from)
}

/// Write content to a file at the given path.
pub fn write_patch_file(path: &str, content: &str) -> Result<(), PdtkError> {
    fs::write(path, content).map_err(PdtkError::from)
}

/// Write content to a file, optionally creating a `.bak` backup first.
pub fn write_with_backup(path: &str, content: &str, backup: bool) -> Result<(), PdtkError> {
    if backup {
        let backup_path = format!("{}.bak", path);
        if Path::new(path).exists() {
            fs::copy(path, &backup_path)?;
        }
    }
    write_patch_file(path, content)
}

/// Scan either a single .pd file or a directory tree for .pd files.
pub fn scan_pd_files(path: &str) -> Result<Vec<PathBuf>, PdtkError> {
    let p = Path::new(path);
    if p.is_file() {
        return Ok(vec![p.to_path_buf()]);
    }

    if !p.is_dir() {
        return Err(PdtkError::Usage(format!("path not found: {path}")));
    }

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(p).into_iter().filter_map(Result::ok) {
        let ep = entry.path();
        if ep.is_file() && ep.extension().map(|e| e == "pd").unwrap_or(false) {
            files.push(ep.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}
