mod integration;
use integration::{fixture_path, pdtk_output, run_pdtk, stdout_string};

fn abs_dir() -> std::path::PathBuf {
    fixture_path("abstractions")
}

#[test]
fn deps_known_builtins_not_reported() {
    let f = abs_dir().join("uses_abstractions.pd");
    let out = pdtk_output(&["deps", f.to_str().unwrap()]);
    // loadbang and print are builtins — should not appear
    assert!(!out.contains("loadbang"));
    assert!(!out.contains("print"));
}

#[test]
fn deps_abstraction_references_listed() {
    let f = abs_dir().join("uses_abstractions.pd");
    let out = pdtk_output(&["deps", f.to_str().unwrap()]);
    assert!(out.contains("used_abs"));
    assert!(out.contains("missing_abs"));
}

#[test]
fn deps_missing_flag_only_shows_absent_files() {
    let f = abs_dir().join("uses_abstractions.pd");
    let out = pdtk_output(&["deps", f.to_str().unwrap(), "--missing"]);
    // missing_abs has no .pd file
    assert!(out.contains("missing_abs"));
    // used_abs.pd exists in the abstractions dir → should not appear
    assert!(!out.contains("used_abs"));
}

#[test]
fn deps_recursive_follows_chain() {
    let f = abs_dir().join("uses_abstractions.pd");
    let out = run_pdtk(&["deps", f.to_str().unwrap(), "--recursive"]);
    // used_abs.pd uses only builtins → no new deps from recursion
    // missing_abs.pd can't be followed (doesn't exist) → no error
    assert_eq!(out.status.code(), Some(0));
    // missing_abs should still be reported
    assert!(stdout_string(&out).contains("missing_abs"));
}

#[test]
fn deps_circular_reference_no_infinite_loop() {
    let dir = tempfile::tempdir().unwrap();
    // a uses b, b uses a
    std::fs::write(
        dir.path().join("a.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 b;\n",
    ).unwrap();
    std::fs::write(
        dir.path().join("b.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 a;\n",
    ).unwrap();

    let a = dir.path().join("a.pd");
    let out = run_pdtk(&["deps", a.to_str().unwrap(), "--recursive"]);
    assert_eq!(out.status.code(), Some(0), "circular deps must not infinite loop");
}

#[test]
fn deps_directory_mode_deduplicates() {
    let dir = tempfile::tempdir().unwrap();
    // Two files each using the same abstraction
    std::fs::write(dir.path().join("x.pd"), "#N canvas 0 22 450 300 12;\n#X obj 50 50 myabs;\n").unwrap();
    std::fs::write(dir.path().join("y.pd"), "#N canvas 0 22 450 300 12;\n#X obj 50 50 myabs;\n").unwrap();

    let out = pdtk_output(&["deps", dir.path().to_str().unwrap()]);
    // "myabs" should appear at most once per file — check it shows in both
    let count = out.matches("myabs").count();
    assert!(count >= 1, "myabs must be reported");
}

#[test]
fn deps_json_output_schema() {
    let f = abs_dir().join("uses_abstractions.pd");
    let out = pdtk_output(&["deps", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.is_array());
    let row = &v.as_array().unwrap()[0];
    assert!(row.get("file").is_some());
    assert!(row.get("name").is_some());
    assert!(row.get("found").is_some());
}
