use crate::errors::PdtkError;
use std::fs;
use std::path::Path;

/// Read a .pd file from disk.
pub fn read_patch_file(path: &str) -> Result<String, PdtkError> {
    fs::read_to_string(path).map_err(PdtkError::from)
}

/// Write content to a file at the given path.
pub fn write_patch_file(path: &str, content: &str) -> Result<(), PdtkError> {
    fs::write(path, content).map_err(PdtkError::from)
}

/// Write content to a file, optionally creating a `.bak` backup first.
/// If backup is true, the original file is moved to `<path>.bak`.
pub fn write_with_backup(path: &str, content: &str, backup: bool) -> Result<(), PdtkError> {
    if backup {
        let backup_path = format!("{}.bak", path);
        // Only create backup if original exists
        if Path::new(path).exists() {
            fs::copy(path, &backup_path)?;
        }
    }
    write_patch_file(path, content)
}
