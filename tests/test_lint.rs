mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk, stdout_string};

#[test]
fn lint_valid_patch_exits_0() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&["lint", f.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout_string(&out).contains("OK: patch is valid"));
}

#[test]
fn lint_invalid_connection_exits_1() {
    let f = handcrafted("malformed_bad_connection.pd");
    let out = run_pdtk(&["lint", f.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let s = stdout_string(&out);
    assert!(s.contains("ERROR:"));
}

#[test]
fn lint_detects_overlapping_objects() {
    // Craft a patch where two objects share the same x/y
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 50 print;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["lint", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0)); // valid structure, but style warning
    let s = stdout_string(&out);
    assert!(
        s.contains("STYLE:"),
        "overlap should produce a style warning"
    );
}

#[test]
fn lint_json_output_has_both_categories() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["lint", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("errors").is_some());
    assert!(v.get("warnings").is_some());
    assert!(v.get("style").is_some());
    assert!(v.get("valid").is_some());
}

#[test]
fn lint_combines_validate_and_style_results() {
    // A malformed patch should have errors AND valid is false
    let f = handcrafted("malformed_bad_connection.pd");
    let out = run_pdtk(&["lint", f.to_str().unwrap(), "--json"]);
    assert_eq!(out.status.code(), Some(1));
    let v: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();
    assert_eq!(v["valid"], false);
    assert!(!v["errors"].as_array().unwrap().is_empty());
}

#[test]
fn lint_all_valid_fixtures_exit_0() {
    let dir = integration::fixture_path("handcrafted");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !name.ends_with(".pd") || name.starts_with("malformed_") || name == "empty_file.pd" {
            continue;
        }
        let out = run_pdtk(&["lint", path.to_str().unwrap()]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{name} should lint cleanly, got: {}",
            stdout_string(&out)
        );
    }
}
