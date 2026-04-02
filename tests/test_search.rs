mod integration;

use integration::{fixture_path, handcrafted, pdtk_output, run_pdtk};

#[test]
fn search_by_type_finds_matching_objects() {
    let f = handcrafted("send_receive.pd");
    let out = pdtk_output(&["search", f.to_str().unwrap(), "--type", "s"]);
    assert!(out.contains("class:s"));
}

#[test]
fn search_by_text_glob_matches() {
    let f = handcrafted("send_receive.pd");
    let out = pdtk_output(&["search", f.to_str().unwrap(), "--text", "*clock_*"]);
    assert!(out.contains("clock_main"));
}

#[test]
fn search_by_text_regex_flag_matches() {
    let f = handcrafted("send_receive.pd");
    let out = pdtk_output(&[
        "search",
        f.to_str().unwrap(),
        "--regex",
        "--text",
        "reverb_.*",
    ]);
    assert!(out.contains("reverb_bus"));
}

#[test]
fn search_combined_type_and_text() {
    let f = handcrafted("send_receive.pd");
    let out = pdtk_output(&[
        "search",
        f.to_str().unwrap(),
        "--type",
        "s",
        "--text",
        "*clock_*",
    ]);
    assert!(out.contains("clock_main"));
    assert!(!out.contains("audio_bus"));
}

#[test]
fn search_depth_filter() {
    let f = handcrafted("nested_subpatch.pd");
    let out = pdtk_output(&[
        "search",
        f.to_str().unwrap(),
        "--type",
        "inlet",
        "--depth",
        "1",
    ]);
    assert!(out.contains("depth:1"));
}

#[test]
fn search_no_matches_exits_0_empty_result() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&[
        "search",
        f.to_str().unwrap(),
        "--type",
        "definitely_not_a_class",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("No matches"));
}

#[test]
fn search_json_output_schema() {
    let f = handcrafted("send_receive.pd");
    let out = pdtk_output(&["search", f.to_str().unwrap(), "--type", "s", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let row = &v.as_array().unwrap()[0];
    assert!(row.get("file").is_some());
    assert!(row.get("class").is_some());
}

#[test]
fn search_directory_mode_counts_per_file() {
    let dir = fixture_path("handcrafted");
    let out = pdtk_output(&["search", dir.to_str().unwrap(), "--type", "loadbang"]);
    assert!(out.contains("simple_chain.pd"));
}

#[test]
fn search_case_insensitive_by_default() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["search", f.to_str().unwrap(), "--type", "LOADBANG"]);
    assert!(out.contains("loadbang"));
}

#[test]
fn search_case_sensitive_flag() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&[
        "search",
        f.to_str().unwrap(),
        "--type",
        "LOADBANG",
        "--case-sensitive",
    ]);
    assert!(out.contains("No matches"));
}
