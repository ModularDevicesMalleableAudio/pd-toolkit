use std::path::PathBuf;

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub fn handcrafted(name: &str) -> PathBuf {
    fixtures_dir().join("handcrafted").join(name)
}

pub fn _corpus(name: &str) -> PathBuf {
    fixtures_dir().join("corpus").join(name)
}

pub fn read_fixture(path: &PathBuf) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e))
}
