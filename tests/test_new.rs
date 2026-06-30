mod integration;

use integration::{pdtk_output, run_pdtk};

// ── stdout ───────────────────────────────────────────────────────────────────

/// Platform-specific Y default: 22 on macOS, 50 elsewhere (mirrors GLIST_DEFCANVASYLOC).
#[cfg(target_os = "macos")]
const DEFAULT_Y: u32 = 22;
#[cfg(not(target_os = "macos"))]
const DEFAULT_Y: u32 = 50;

#[test]
fn new_to_stdout_uses_pd_defaults() {
    let out = pdtk_output(&["new"]);
    let expected = format!("#N canvas 0 {DEFAULT_Y} 450 300 12;\n");
    assert_eq!(out, expected);
}

#[test]
fn new_custom_dimensions_to_stdout() {
    let out = pdtk_output(&["new", "--width", "800", "--height", "600"]);
    let expected = format!("#N canvas 0 {DEFAULT_Y} 800 600 12;\n");
    assert_eq!(out, expected);
}

#[test]
fn new_custom_position_to_stdout() {
    let out = pdtk_output(&["new", "--canvas-x", "100", "--canvas-y", "50"]);
    assert_eq!(out, "#N canvas 100 50 450 300 12;\n");
}

#[test]
fn new_custom_font_to_stdout() {
    let out = pdtk_output(&["new", "--font", "10"]);
    let expected = format!("#N canvas 0 {DEFAULT_Y} 450 300 10;\n");
    assert_eq!(out, expected);
}

// ── file output ──────────────────────────────────────────────────────────────

#[test]
fn new_writes_to_file() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap();

    // NamedTempFile creates the file, so we need --force to overwrite it
    pdtk_output(&["new", "--force", path]);

    let content = std::fs::read_to_string(path).unwrap();
    let expected = format!("#N canvas 0 {DEFAULT_Y} 450 300 12;\n");
    assert_eq!(content, expected);
}

#[test]
fn new_writes_custom_patch_to_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blank.pd");
    let path_str = path.to_str().unwrap();

    pdtk_output(&[
        "new", path_str, "--width", "800", "--height", "600", "--font", "10",
    ]);

    let content = std::fs::read_to_string(&path).unwrap();
    let expected = format!("#N canvas 0 {DEFAULT_Y} 800 600 10;\n");
    assert_eq!(content, expected);
}

#[test]
fn new_refuses_existing_file_without_force() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_str().unwrap();

    let out = run_pdtk(&["new", path]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("already exists"),
        "expected error, got: {stderr}"
    );
}

#[test]
fn new_force_overwrites_existing_file() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "old content").unwrap();
    let path = tmp.path().to_str().unwrap();

    pdtk_output(&["new", "--force", path]);

    let content = std::fs::read_to_string(tmp.path()).unwrap();
    let expected = format!("#N canvas 0 {DEFAULT_Y} 450 300 12;\n");
    assert_eq!(content, expected);
}

// ── round-trip ───────────────────────────────────────────────────────────────

#[test]
fn new_output_is_parseable() {
    // The generated patch must be accepted by `pdtk parse`
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blank.pd");
    let path_str = path.to_str().unwrap();

    pdtk_output(&["new", path_str]);
    // If parse succeeds the output has exit code 0 (pdtk_output asserts that)
    pdtk_output(&["parse", path_str]);
}

#[test]
fn new_output_is_valid() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("blank.pd");
    let path_str = path.to_str().unwrap();

    pdtk_output(&["new", path_str]);
    pdtk_output(&["validate", path_str]);
}
