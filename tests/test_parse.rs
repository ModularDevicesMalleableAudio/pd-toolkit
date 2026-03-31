mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk, stderr_string, stdout_string};

#[test]
fn parse_minimal_prints_zero_objects() {
    let f = handcrafted("minimal.pd");
    let out = run_pdtk(&["parse", f.to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    assert!(stdout.contains("Objects: 0"));
    assert!(stdout.contains("Connections: 0"));
}

#[test]
fn parse_simple_chain_prints_3_objects_2_connections() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&["parse", f.to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    assert!(stdout.contains("Objects: 3"));
    assert!(stdout.contains("Connections: 2"));
}

#[test]
fn parse_nested_subpatch_reports_depth_1() {
    let f = handcrafted("nested_subpatch.pd");
    let out = run_pdtk(&["parse", f.to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    assert!(stdout.contains("Max depth: 1"));
}

#[test]
fn parse_large_patch_reports_120_objects() {
    let f = handcrafted("large_patch.pd");
    let out = run_pdtk(&["parse", f.to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    assert!(stdout.contains("Objects: 120"));
}

#[test]
fn parse_nonexistent_file_exits_3() {
    let out = run_pdtk(&["parse", "does_not_exist.pd"]);
    assert_eq!(out.status.code(), Some(3), "IO errors must exit 3");
    assert!(stderr_string(&out).contains("io error"));
}

#[test]
fn parse_empty_file_exits_2_with_message() {
    let f = handcrafted("empty_file.pd");
    let out = run_pdtk(&["parse", f.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr_string(&out).contains("empty input"));
}

#[test]
fn parse_json_output_is_valid_json() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&["parse", f.to_str().unwrap(), "--json"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();
    assert!(json.is_object());
}

#[test]
fn parse_json_output_object_count_matches() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&["parse", f.to_str().unwrap(), "--json"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();
    assert_eq!(json["objects"], 3);
    assert_eq!(json["connections"], 2);
}

#[test]
fn parse_output_flag_round_trips_file() {
    // pdtk parse <file> --output <tmp> must produce a byte-identical copy
    let f = handcrafted("simple_chain.pd");
    let original = std::fs::read_to_string(&f).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let _summary = pdtk_output(&["parse", f.to_str().unwrap(), "--output", tmp.path().to_str().unwrap()]);
    let written = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(original, written, "--output must produce byte-identical round-trip");
}

#[test]
fn parse_output_flag_round_trips_corpus_files() {
    let dir = integration::fixture_path("corpus");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().map(|e| e == "pd").unwrap_or(false) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let original = std::fs::read_to_string(&path).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        pdtk_output(&["parse", path.to_str().unwrap(), "--output", tmp.path().to_str().unwrap()]);
        let written = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(original, written, "round-trip failed for {name}");
    }
}

#[test]
fn parse_verbose_flag_accepted() {
    let f = handcrafted("simple_chain.pd");
    let stdout = pdtk_output(&["--verbose", "parse", f.to_str().unwrap()]);
    // verbose adds an entry breakdown section
    assert!(stdout.contains("Objects: 3"));
    assert!(stdout.contains("Entry breakdown"));
}

#[test]
fn parse_malformed_missing_semicolon_emits_warning() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang\n").unwrap();

    let out = run_pdtk(&["parse", tmp.path().to_str().unwrap()]);
    assert!(out.status.success());
    let stdout = stdout_string(&out);
    assert!(stdout.contains("Warnings: 1"));
    assert!(stdout.contains("UnterminatedEntry"));
}
