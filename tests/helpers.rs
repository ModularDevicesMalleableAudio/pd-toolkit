// Shared test-support module included via `mod helpers;`. Each test binary
// uses only a subset of these helpers, so unused ones are expected per-binary.
#![allow(dead_code)]

use std::path::PathBuf;

#[must_use]
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[must_use]
pub fn handcrafted(name: &str) -> PathBuf {
    fixtures_dir().join("handcrafted").join(name)
}

#[must_use]
pub fn _corpus(name: &str) -> PathBuf {
    fixtures_dir().join("corpus").join(name)
}

#[must_use]
pub fn read_fixture(path: &PathBuf) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e))
}
