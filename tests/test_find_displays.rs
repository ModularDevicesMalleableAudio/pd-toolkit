mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk};

#[test]
fn displays_finds_connected_floatatom() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&["find-displays", f.to_str().unwrap()]);
    assert!(out.contains("floatatom"));
}

#[test]
fn displays_finds_connected_symbolatom() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&[
        "find-displays",
        f.to_str().unwrap(),
        "--include-unconnected",
    ]);
    assert!(out.contains("symbolatom"));
}

#[test]
fn displays_finds_connected_nbx() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&["find-displays", f.to_str().unwrap()]);
    assert!(out.contains(" nbx "));
}

#[test]
fn displays_does_not_report_unconnected_by_default() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&["find-displays", f.to_str().unwrap()]);
    assert!(!out.contains("symbolatom"));
}

#[test]
fn displays_include_unconnected_flag() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&[
        "find-displays",
        f.to_str().unwrap(),
        "--include-unconnected",
    ]);
    assert!(out.contains("symbolatom"));
}

#[test]
fn displays_json_output_schema() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&["find-displays", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let row = &v.as_array().unwrap()[0];
    assert!(row.get("file").is_some());
    assert!(row.get("connected").is_some());
}

#[test]
fn displays_delete_removes_and_renumbers() {
    let src = std::fs::read_to_string(handcrafted("displays.pd")).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), src).unwrap();

    pdtk_output(&[
        "find-displays",
        tmp.path().to_str().unwrap(),
        "--delete",
        "--in-place",
    ]);

    let out = pdtk_output(&["list", tmp.path().to_str().unwrap()]);
    assert!(!out.contains("floatatom"));
    assert!(!out.contains("nbx"));
}

#[test]
fn displays_delete_validates_before_write() {
    let src = std::fs::read_to_string(handcrafted("displays.pd")).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), src).unwrap();

    pdtk_output(&[
        "find-displays",
        tmp.path().to_str().unwrap(),
        "--delete",
        "--in-place",
    ]);

    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}
