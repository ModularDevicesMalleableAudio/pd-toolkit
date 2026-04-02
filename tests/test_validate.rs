mod integration;

use integration::{corpus_dir, handcrafted, run_pdtk, stdout_string};

#[test]
fn validate_simple_chain_exits_0() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&["validate", f.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout_string(&out).contains("OK: patch is valid"));
}

#[test]
fn validate_all_handcrafted_valid_fixtures_exit_0() {
    let dir = integration::fixture_path("handcrafted");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if !name.ends_with(".pd") {
            continue;
        }
        if name.starts_with("malformed_") || name == "empty_file.pd" {
            continue;
        }

        let out = run_pdtk(&["validate", path.to_str().unwrap()]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "expected valid fixture {} to pass, stdout:\n{}",
            name,
            stdout_string(&out)
        );
    }
}

#[test]
fn validate_all_corpus_files_exit_0() {
    let dir = corpus_dir();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().map(|e| e == "pd").unwrap_or(false) {
            continue;
        }

        let out = run_pdtk(&["validate", path.to_str().unwrap()]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "expected corpus fixture {} to pass, stdout:\n{}",
            path.display(),
            stdout_string(&out)
        );
    }
}

#[test]
fn validate_malformed_bad_connection_exits_1() {
    let f = handcrafted("malformed_bad_connection.pd");
    let out = run_pdtk(&["validate", f.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout_string(&out).contains("INVALID"));
}

#[test]
fn validate_bad_connection_message_includes_index() {
    let f = handcrafted("malformed_bad_connection.pd");
    let out = run_pdtk(&["validate", f.to_str().unwrap()]);
    let stdout = stdout_string(&out);
    assert!(stdout.contains("dst 5 out of range"));
}

#[test]
fn validate_depth_imbalance_exits_1() {
    let input = "#N canvas 0 22 450 300 12;\n#X obj 10 10 f;\n#X restore 10 10 pd nope;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout_string(&out).contains("depth imbalance"));
}

#[test]
fn validate_strict_duplicate_is_warning_not_error() {
    // --strict duplicates are WARNINGS (plan §2.3): valid patch, exit 0, but warning emitted
    let input = "#N canvas 0 22 450 300 12;\n#X obj 10 10 f;\n#X obj 10 40 print;\n#X connect 0 0 1 0;\n#X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap(), "--strict"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "duplicate connections are warnings, not errors"
    );
    let stdout = stdout_string(&out);
    assert!(
        stdout.contains("duplicate connection"),
        "warning text must appear"
    );
    assert!(stdout.contains("OK: patch is valid"), "patch still valid");
}

#[test]
fn validate_strict_duplicate_json_is_in_warnings_array() {
    let input = "#N canvas 0 22 450 300 12;\n#X obj 10 10 f;\n#X obj 10 40 print;\n#X connect 0 0 1 0;\n#X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&[
        "validate",
        tmp.path().to_str().unwrap(),
        "--strict",
        "--json",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();
    assert_eq!(json["valid"], true);
    assert!(json["errors"].as_array().unwrap().is_empty());
    let warnings = json["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("duplicate connection"))
    );
}

#[test]
fn validate_json_output_includes_error_list() {
    let f = handcrafted("malformed_bad_connection.pd");
    let out = run_pdtk(&["validate", f.to_str().unwrap(), "--json"]);
    assert_eq!(out.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();
    assert_eq!(json["valid"], false);
    assert!(json["errors"].as_array().unwrap().len() >= 1);
}

#[test]
fn validate_empty_file_exits_2() {
    let f = handcrafted("empty_file.pd");
    let out = run_pdtk(&["validate", f.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn validate_nonexistent_file_exits_3() {
    let out = run_pdtk(&["validate", "does_not_exist.pd"]);
    assert_eq!(out.status.code(), Some(3), "IO errors must exit 3");
}

#[test]
fn validate_warns_on_unescaped_dollar_digit() {
    let input = "#N canvas 0 22 450 300 12;\n#X obj 10 10 $1;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = stdout_string(&out);
    assert!(stdout.contains("warning"));
    assert!(stdout.contains("unescaped $-digit token"));
}

#[test]
fn validate_warns_on_unescaped_semicolon_in_entry_body() {
    let input = "#N canvas 0 22 450 300 12;\n#X msg 10 10 foo ; bar;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = stdout_string(&out);
    assert!(stdout.contains("warning"));
    assert!(stdout.contains("unescaped ';'"));
}

#[test]
fn validate_json_reports_escaping_warnings() {
    let input = "#N canvas 0 22 450 300 12;\n#X obj 10 10 $1;\n#X msg 10 40 foo ; bar;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap(), "--json"]);
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_str(&stdout_string(&out)).unwrap();
    let warnings = json["warnings"].as_array().unwrap();
    assert!(warnings
        .iter()
        .any(|w| w.as_str().unwrap_or("").contains("unescaped $-digit token")));
    assert!(warnings
        .iter()
        .any(|w| w.as_str().unwrap_or("").contains("unescaped ';'")));
}

#[test]
fn validate_output_flag_writes_to_file() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let out = run_pdtk(&[
        "validate",
        f.to_str().unwrap(),
        "--output",
        tmp.path().to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout_string(&out).trim().is_empty(),
        "stdout must be empty when --output is used"
    );
    let content = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(content.contains("OK: patch is valid"));
}
