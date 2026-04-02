mod integration;

use integration::{fixture_path, handcrafted, pdtk_output, run_pdtk};

#[test]
fn orphans_finds_unconnected_objects() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap()]);
    assert!(out.contains("osc~ 440"));
    assert!(out.contains("f 42"));
}

#[test]
fn orphans_excludes_text_by_default() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap()]);
    assert!(!out.contains("this is a comment"));
}

#[test]
fn orphans_include_comments_flag_shows_text() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap(), "--include-comments"]);
    assert!(out.contains("this is a comment"));
}

#[test]
fn orphans_depth_filter() {
    let f = handcrafted("nested_subpatch.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap(), "--depth", "0"]);
    // nested_subpatch has no obvious orphans at depth 0
    assert!(out.contains("No orphan") || !out.contains("[depth:1"));
}

#[test]
fn orphans_json_output_schema() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.is_array());
    let row = &v.as_array().unwrap()[0];
    assert!(row.get("file").is_some());
    assert!(row.get("depth").is_some());
    assert!(row.get("index").is_some());
    assert!(row.get("text").is_some());
}

#[test]
fn orphans_connected_loadbang_not_orphan() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap()]);
    assert!(!out.contains("loadbang"));
}

#[test]
fn orphans_delete_removes_and_renumbers() {
    let src = std::fs::read_to_string(handcrafted("orphans.pd")).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), src).unwrap();

    pdtk_output(&[
        "find-orphans",
        tmp.path().to_str().unwrap(),
        "--delete",
        "--in-place",
    ]);

    let out = pdtk_output(&["list", tmp.path().to_str().unwrap()]);
    assert!(!out.contains("osc~ 440"));
    assert!(!out.contains("f 42"));
}

#[test]
fn orphans_delete_validates_before_write() {
    let src = std::fs::read_to_string(handcrafted("orphans.pd")).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), src).unwrap();

    pdtk_output(&[
        "find-orphans",
        tmp.path().to_str().unwrap(),
        "--delete",
        "--in-place",
    ]);

    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}

#[test]
fn orphans_directory_mode_scans_recursively() {
    let dir = fixture_path("handcrafted");
    let out = pdtk_output(&["find-orphans", dir.to_str().unwrap()]);
    assert!(out.contains("orphans.pd"));
}
