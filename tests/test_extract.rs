mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk};

/// Run extract and return the extracted file contents.
fn extract_to_tmp(fixture: &str, depth: usize) -> (tempfile::NamedTempFile, String) {
    let f = handcrafted(fixture);
    let tmp = tempfile::NamedTempFile::new().unwrap();
    pdtk_output(&[
        "extract",
        f.to_str().unwrap(),
        "--depth",
        &depth.to_string(),
        "--output",
        tmp.path().to_str().unwrap(),
    ]);
    let content = std::fs::read_to_string(tmp.path()).unwrap();
    (tmp, content)
}

// Output is a valid .pd file

#[test]
fn extract_output_is_valid_pd_file() {
    let (_tmp, content) = extract_to_tmp("nested_subpatch.pd", 1);
    assert!(content.trim_start().starts_with("#N canvas"), "must start with canvas header");
}

#[test]
fn extract_output_passes_validate() {
    let f = handcrafted("nested_subpatch.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    pdtk_output(&[
        "extract",
        f.to_str().unwrap(),
        "--depth", "1",
        "--output", tmp.path().to_str().unwrap(),
    ]);
    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}

// In-place: source file validates after extraction

#[test]
fn extract_source_passes_validate_after_in_place() {
    let src = std::fs::read_to_string(handcrafted("nested_subpatch.pd")).unwrap();
    let src_tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(src_tmp.path(), &src).unwrap();
    let out_tmp = tempfile::NamedTempFile::new().unwrap();

    run_pdtk(&[
        "extract",
        src_tmp.path().to_str().unwrap(),
        "--depth", "1",
        "--output", out_tmp.path().to_str().unwrap(),
        "--in-place",
    ]);

    let v = run_pdtk(&["validate", src_tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}

#[test]
fn extract_in_place_replaces_subpatch_with_abstraction_ref() {
    let src = std::fs::read_to_string(handcrafted("nested_subpatch.pd")).unwrap();
    let src_tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(src_tmp.path(), &src).unwrap();
    // Name the output file so we can predict the abstraction name
    let out_dir = tempfile::tempdir().unwrap();
    let out_path = out_dir.path().join("my_sub.pd");

    run_pdtk(&[
        "extract",
        src_tmp.path().to_str().unwrap(),
        "--depth", "1",
        "--output", out_path.to_str().unwrap(),
        "--in-place",
    ]);

    let modified = std::fs::read_to_string(src_tmp.path()).unwrap();
    // The subpatch block (#N canvas … #X restore) must be gone
    assert!(!modified.contains("#X restore"), "restore entry should be replaced");
    // An abstraction reference to my_sub should exist
    assert!(modified.contains("my_sub"), "abstraction reference must appear");
    // No more nested canvas header
    let canvas_count = modified.lines().filter(|l| l.starts_with("#N canvas")).count();
    assert_eq!(canvas_count, 1, "only the root canvas should remain");
}

// Boundary connections become inlet/outlet objects

#[test]
fn extract_boundary_connections_become_inlets_outlets() {
    let (_tmp, content) = extract_to_tmp("nested_subpatch.pd", 1);
    // nested_subpatch feeds in from parent (1 inlet) and out to parent (1 outlet)
    assert!(content.contains("inlet"), "extracted file must have inlet");
    assert!(content.contains("outlet"), "extracted file must have outlet");
}

#[test]
fn extract_preserves_interior_objects() {
    let (_tmp, content) = extract_to_tmp("nested_subpatch.pd", 1);
    // The subpatch contained + 1
    assert!(content.contains("+ 1"), "interior object must appear in extracted file");
}

#[test]
fn extract_preserves_interior_connections() {
    let (_tmp, content) = extract_to_tmp("nested_subpatch.pd", 1);
    // The subpatch had #X connect 0 0 1 0; and #X connect 1 0 2 0; (offset by n_inlets)
    assert!(content.contains("#X connect"), "interior connections must be preserved");
}

// Refuses malformed / missing subpatch

#[test]
fn extract_refuses_malformed_subpatch() {
    // minimal.pd has no subpatches
    let f = handcrafted("minimal.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let out = run_pdtk(&[
        "extract",
        f.to_str().unwrap(),
        "--depth", "1",
        "--output", tmp.path().to_str().unwrap(),
    ]);
    assert_ne!(out.status.code(), Some(0));
}

// Works on deeper subpatches

#[test]
fn extract_works_on_deeply_nested_subpatch() {
    let f = handcrafted("deep_subpatch.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let out = run_pdtk(&[
        "extract",
        f.to_str().unwrap(),
        "--depth", "1",
        "--output", tmp.path().to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));

    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}

// Backup flag

#[test]
fn extract_in_place_backup_creates_bak() {
    let src = std::fs::read_to_string(handcrafted("nested_subpatch.pd")).unwrap();
    let src_tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(src_tmp.path(), &src).unwrap();
    let bak_path = format!("{}.bak", src_tmp.path().display());

    let out_tmp = tempfile::NamedTempFile::new().unwrap();
    run_pdtk(&[
        "extract",
        src_tmp.path().to_str().unwrap(),
        "--depth", "1",
        "--output", out_tmp.path().to_str().unwrap(),
        "--in-place",
        "--backup",
    ]);

    assert!(std::path::Path::new(&bak_path).exists(), ".bak must be created");
    let bak_content = std::fs::read_to_string(&bak_path).unwrap();
    assert_eq!(bak_content, src, ".bak must match original");
    std::fs::remove_file(&bak_path).ok();
}
