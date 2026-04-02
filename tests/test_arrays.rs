mod integration;

use integration::{fixture_path, handcrafted, pdtk_output};

#[test]
fn arrays_lists_all_defined_arrays() {
    let f = handcrafted("arrays.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap()]);
    assert!(out.contains("waveform_a"));
    assert!(out.contains("waveform_b"));
}

#[test]
fn arrays_shows_name_and_size() {
    let f = handcrafted("arrays.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap()]);
    assert!(out.contains("size 256"));
    assert!(out.contains("size 128"));
}

#[test]
fn arrays_json_output_schema() {
    let f = handcrafted("arrays.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("arrays").is_some());
    assert!(v.get("duplicate_names").is_some());
}

#[test]
fn arrays_directory_deduplication() {
    let dir = fixture_path("handcrafted");
    let out = pdtk_output(&["arrays", dir.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["arrays"].as_array().unwrap().len() >= 2);
}

#[test]
fn arrays_detects_duplicate_names_across_files() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pd");
    let b = dir.path().join("b.pd");

    std::fs::write(
        &a,
        "#N canvas 0 0 100 100 10;\n#N canvas 0 0 100 100 (subpatch) 0;\n#X array same 16 float 3;\n#A 0 0 0;\n#X coords 0 1 15 -1 100 50 1 0 0;\n#X restore 10 10 graph;\n",
    )
    .unwrap();
    std::fs::write(
        &b,
        "#N canvas 0 0 100 100 10;\n#N canvas 0 0 100 100 (subpatch) 0;\n#X array same 32 float 3;\n#A 0 0 0;\n#X coords 0 1 31 -1 100 50 1 0 0;\n#X restore 10 10 graph;\n",
    )
    .unwrap();

    let out = pdtk_output(&["arrays", dir.path().to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["duplicate_names"]["same"].as_array().unwrap().len() == 2);
}
