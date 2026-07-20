use crate::errors::PdtkError;
use std::fs;
use std::path::{Path, PathBuf};

/// Read a .pd file from disk as strict UTF-8.
///
/// Used by write-capable commands: a non-UTF-8 file is refused with a clear
/// message rather than silently lossy-decoded, so an in-place edit can never
/// corrupt non-UTF-8 comment/label content. Read-only commands should use
/// [`read_patch_lenient`] instead.
pub fn read_patch_file(path: &str) -> Result<String, PdtkError> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => Err(PdtkError::Usage(format!(
            "{path}: not valid UTF-8 — pdtk can analyse this file (e.g. `parse`, `list`, \
             `deps`) but refuses to edit it to avoid corrupting non-UTF-8 content"
        ))),
        Err(e) => Err(PdtkError::from(e)),
    }
}

/// Read a .pd file for read-only analysis, tolerating non-UTF-8 content.
///
/// Invalid byte sequences are replaced with U+FFFD (see
/// [`pdtk::parser::decode_lenient`]); PD structure is ASCII, so parsing stays
/// correct. Accepts any path type so directory-scan loops (which hold
/// `PathBuf`s) can use it directly.
pub fn read_patch_lenient(path: impl AsRef<Path>) -> Result<String, PdtkError> {
    let bytes = fs::read(path)?;
    Ok(pdtk::parser::decode_lenient(&bytes))
}

/// Write content to a file at the given path.
pub fn write_patch_file(path: &str, content: &str) -> Result<(), PdtkError> {
    fs::write(path, content).map_err(PdtkError::from)
}

/// Write content to a file, optionally creating a `.bak` backup first.
pub fn write_with_backup(path: &str, content: &str, backup: bool) -> Result<(), PdtkError> {
    if backup {
        let backup_path = format!("{path}.bak");
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
        if ep.is_file() && ep.extension().is_some_and(|e| e == "pd") {
            files.push(ep.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}
