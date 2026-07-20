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
fn validate_counts_array_as_object() {
    // `#X array` is an indexed gobj in Pd; the connection metro(1)->tabwrite(2)
    // is only in range when the array (index 0) is counted.
    let f = handcrafted("array_in_canvas.pd");
    let out = run_pdtk(&["validate", f.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "array must be counted as an object; stdout:\n{}",
        stdout_string(&out)
    );
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
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("unescaped $-digit token"))
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap_or("").contains("unescaped ';'"))
    );
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

#[test]
fn validate_warns_on_detached_array_data() {
    // `#A` that does not follow an array definition is orphaned data.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 print;\n\
                 #A 0 0.25 0.5 0.75;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    // A warning, not an error: exit stays 0.
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout_string(&out).contains("#A array data not attached"),
        "got:\n{}",
        stdout_string(&out)
    );
}

#[test]
fn validate_no_warning_for_attached_array_data() {
    // Classic `#X array` followed by `#A` is well-formed.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X array wf 4 float 3;\n\
                 #A 0 0.1 0.2 0.3;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(!stdout_string(&out).contains("#A array data not attached"));
}

#[test]
fn validate_warns_on_out_of_range_outlet() {
    // metro has a single outlet (index 0); a wire from outlet 1 is dropped by
    // Pd on load. validate must warn (exit stays 0).
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 metro 100;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 1 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "outlet issue is a warning, not error"
    );
    assert!(
        stdout_string(&out).contains("outlet"),
        "expected an outlet warning; got:\n{}",
        stdout_string(&out)
    );
}

#[test]
fn validate_warns_on_out_of_range_inlet() {
    // print has a single inlet (index 0); a wire into inlet 1 is invalid.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 metro 100;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 1;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout_string(&out).contains("inlet"),
        "expected an inlet warning; got:\n{}",
        stdout_string(&out)
    );
}

#[test]
fn validate_no_outlet_warning_for_unknown_class() {
    // An unknown external could have any number of outlets; do not warn.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 my_external_obj;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 5 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        !stdout_string(&out).contains("outlet"),
        "unknown class must not trigger an outlet warning; got:\n{}",
        stdout_string(&out)
    );
}

#[test]
fn validate_no_outlet_warning_for_valid_connection() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 metro 100;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(!s.contains("outlet") && !s.contains("inlet"), "got:\n{s}");
}

#[test]
fn validate_no_inlet_warning_for_bare_send_right_inlet() {
    // `[send]`/`[s]` with no argument has a second inlet that sets the
    // destination; wiring it is a common idiom and must not warn.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X msg 50 20 target;\n\
                 #X obj 50 60 s;\n\
                 #X obj 120 60 send;\n\
                 #X connect 0 0 1 1;\n\
                 #X connect 0 0 2 1;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(
        !s.contains("inlet"),
        "bare [send]/[s] right-inlet wiring is valid; got:\n{s}"
    );
}

// Feature E: a stray unescaped ';' in a message body splits the entry and
// leaves a bare fragment. validate flags the fragment (warning, not error).

#[test]
fn validate_flags_stray_fragment_from_unescaped_semicolon() {
    let input = "#N canvas 0 22 450 300 12;\n#X msg 10 10 foo ; bar;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "stray fragment is a warning");
    let s = stdout_string(&out);
    assert!(s.contains("stray content"), "got:\n{s}");
    assert!(s.contains("unescaped ';'"), "got:\n{s}");
}
