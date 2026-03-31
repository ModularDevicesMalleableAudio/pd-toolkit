mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk, stdout_string};

#[test]
fn list_simple_chain_shows_3_objects() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&["list", f.to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    assert!(stdout.contains("[0:0] loadbang"));
    assert!(stdout.contains("[0:1] t b"));
    assert!(stdout.contains("[0:2] print done"));
}

#[test]
fn list_nested_shows_restore_at_depth_0() {
    let f = handcrafted("nested_subpatch.pd");
    let out = run_pdtk(&["list", f.to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    assert!(stdout.contains("[0:1] restore"));
}

#[test]
fn list_depth_filter_only_shows_requested_depth() {
    let f = handcrafted("nested_subpatch.pd");
    let out = run_pdtk(&["list", f.to_str().unwrap(), "--depth", "1"]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    assert!(stdout.contains("[1:0] inlet"));
    assert!(stdout.contains("[1:1] + 1"));
    assert!(stdout.contains("[1:2] outlet"));
    assert!(!stdout.contains("[0:0]"));
}

#[test]
fn list_with_declare_skips_directive() {
    let f = handcrafted("with_declare.pd");
    let out = run_pdtk(&["list", f.to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    // Standalone #X declare must not consume index 0.
    assert!(stdout.contains("[0:0] inlet"));
    // Object-form declare should still be present as a normal object.
    assert!(stdout.contains("declare -path pos_abs"));
}

#[test]
fn list_with_width_hint_skips_hint() {
    let f = handcrafted("with_width_hint.pd");
    let out = run_pdtk(&["list", f.to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    assert!(!stdout.contains("width_hint"));
    assert!(stdout.contains("[0:0] restore"));
    assert!(stdout.contains("[0:1] print result"));
}

#[test]
fn list_json_output_valid_json() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&["list", f.to_str().unwrap(), "--json"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();
    assert!(json.is_array());
}

#[test]
fn list_json_indices_match_expected() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&["list", f.to_str().unwrap(), "--json"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();

    assert_eq!(json.as_array().unwrap().len(), 3);
    assert_eq!(json[0]["index"], 0);
    assert_eq!(json[1]["index"], 1);
    assert_eq!(json[2]["index"], 2);
}

#[test]
fn list_corpus_declare_real_indices() {
    let f = integration::fixture_path("corpus/declare_real.pd");
    let out = run_pdtk(&["list", f.to_str().unwrap(), "--depth", "0", "--json"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();
    let first = &json.as_array().unwrap()[0];
    assert_eq!(first["index"], 0);
    assert_ne!(first["class"], "declare");
}

#[test]
fn list_corpus_width_hint_real_indices() {
    let f = integration::fixture_path("corpus/width_hint_real.pd");
    let out = run_pdtk(&["list", f.to_str().unwrap(), "--depth", "0", "--json"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();
    // ensure no width hint pseudo-object slipped in
    assert!(json
        .as_array()
        .unwrap()
        .iter()
        .all(|o| o["class"] != "width_hint"));
}

#[test]
fn list_output_flag_writes_to_file() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    // With --output, nothing should go to stdout
    let out = run_pdtk(&["list", f.to_str().unwrap(), "--output", tmp.path().to_str().unwrap()]);
    assert!(out.status.success());
    assert!(stdout_string(&out).trim().is_empty(), "stdout must be empty when --output is used");
    let content = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(content.contains("[0:0] loadbang"));
}

#[test]
fn list_output_flag_json_writes_to_file() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    pdtk_output(&["list", f.to_str().unwrap(), "--json", "--output", tmp.path().to_str().unwrap()]);
    let content = std::fs::read_to_string(tmp.path()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 3);
}
