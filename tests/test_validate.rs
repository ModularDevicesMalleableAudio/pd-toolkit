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
        if path.extension().is_none_or(|e| e != "pd") {
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
    assert!(!json["errors"].as_array().unwrap().is_empty());
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

// Feature B: validate resolves unknown classes as project-local abstractions
// and checks connection inlet/outlet indices against the abstraction's real
// I/O count (number of top-level inlet/outlet objects).

/// Write `abs.pd` and `main.pd` into a fresh dir and return the dir + main path.
fn abstraction_project(abs: &str, main: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("voice.pd"), abs).unwrap();
    let main_path = dir.path().join("main.pd");
    std::fs::write(&main_path, main).unwrap();
    (dir, main_path)
}

#[test]
fn validate_warns_on_abstraction_inlet_out_of_range() {
    // voice.pd has exactly 1 inlet; wiring into inlet 1 is out of range.
    let abs = "#N canvas 0 22 450 300 12;\n\
               #X obj 20 20 inlet;\n\
               #X obj 20 200 outlet;\n";
    let main = "#N canvas 0 22 450 300 12;\n\
                #X obj 10 10 loadbang;\n\
                #X obj 10 60 voice;\n\
                #X connect 0 0 1 1;\n";
    let (_dir, main_path) = abstraction_project(abs, main);

    let out = run_pdtk(&["validate", main_path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "arity mismatch is a warning");
    let s = stdout_string(&out);
    assert!(
        s.contains("abstraction 'voice' has 1 inlet(s)") && s.contains("inlet 1"),
        "got:\n{s}"
    );
}

#[test]
fn validate_warns_on_abstraction_outlet_out_of_range() {
    let abs = "#N canvas 0 22 450 300 12;\n\
               #X obj 20 20 inlet;\n\
               #X obj 20 200 outlet;\n";
    let main = "#N canvas 0 22 450 300 12;\n\
                #X obj 10 10 voice;\n\
                #X obj 10 60 print;\n\
                #X connect 0 2 1 0;\n";
    let (_dir, main_path) = abstraction_project(abs, main);

    let out = run_pdtk(&["validate", main_path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(
        s.contains("abstraction 'voice' has 1 outlet(s)") && s.contains("outlet 2"),
        "got:\n{s}"
    );
}

#[test]
fn validate_no_warning_for_valid_abstraction_connection() {
    // Two inlets, two outlets: wiring outlet 1 -> inlet 1 is valid.
    let abs = "#N canvas 0 22 450 300 12;\n\
               #X obj 20 20 inlet;\n\
               #X obj 60 20 inlet;\n\
               #X obj 20 200 outlet;\n\
               #X obj 60 200 outlet;\n";
    let main = "#N canvas 0 22 450 300 12;\n\
                #X obj 10 10 voice;\n\
                #X obj 10 60 voice;\n\
                #X connect 0 1 1 1;\n";
    let (_dir, main_path) = abstraction_project(abs, main);

    let out = run_pdtk(&["validate", main_path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(
        !s.contains("abstraction"),
        "valid abstraction wiring must not warn; got:\n{s}"
    );
}

#[test]
fn validate_abstraction_arity_counts_top_level_only() {
    // 1 top-level inlet + 1 inlet nested in a subpatch = arity 1. Wiring
    // inlet 1 must warn (the nested inlet is not part of the interface).
    let abs = "#N canvas 0 22 450 300 12;\n\
               #X obj 20 20 inlet;\n\
               #N canvas 0 22 200 200 sub 0;\n\
               #X obj 10 10 inlet;\n\
               #X restore 100 100 pd sub;\n\
               #X obj 20 200 outlet;\n";
    let main = "#N canvas 0 22 450 300 12;\n\
                #X obj 10 10 loadbang;\n\
                #X obj 10 60 voice;\n\
                #X connect 0 0 1 1;\n";
    let (_dir, main_path) = abstraction_project(abs, main);

    let out = run_pdtk(&["validate", main_path.to_str().unwrap()]);
    let s = stdout_string(&out);
    assert!(
        s.contains("abstraction 'voice' has 1 inlet(s)"),
        "nested inlet must not count toward arity; got:\n{s}"
    );
}

#[test]
fn validate_no_arity_warning_for_unresolvable_external() {
    // No sibling file named my_external.pd -> cannot introspect -> no warning
    // even for a large outlet index (could be a compiled external).
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.pd");
    std::fs::write(
        &main_path,
        "#N canvas 0 22 450 300 12;\n\
         #X obj 10 10 my_external;\n\
         #X obj 10 60 print;\n\
         #X connect 0 7 1 0;\n",
    )
    .unwrap();

    let out = run_pdtk(&["validate", main_path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(
        !s.contains("outlet") && !s.contains("abstraction"),
        "unresolvable external must not warn; got:\n{s}"
    );
}

#[test]
fn validate_no_stray_warning_for_inline_scalar_data() {
    // Real inline scalars are a SINGLE `\;`-escaped entry (Pd escapes the
    // internal separators), not bare data lines. So a data-structure patch
    // has no bare fragments and the stray-fragment check must stay silent.
    let input = "#N struct holder float x float y array z element;\n\
                 #N struct element float v;\n\
                 #N canvas 0 22 450 300 12;\n\
                 #X scalar holder 5 6 \\; 1 \\; 2 \\;;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(
        !s.contains("stray content"),
        "inline scalar data is one entry, not a stray fragment; got:\n{s}"
    );
}

// D1: template/scalar consistency checks (warnings, never errors) that mirror
// Pd's own load-time diagnostics.

#[test]
fn validate_warns_on_scalar_undefined_template() {
    // A `#X scalar ghost` with no `#N struct ghost` — Pd raises
    // "no such template" and drops the scalar at load.
    let input = "#N canvas 0 22 450 300 12;\n#X scalar ghost 1 2;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "consistency issue is a warning");
    let s = stdout_string(&out);
    assert!(
        s.contains("undefined template") && s.contains("ghost"),
        "got:\n{s}"
    );
}

#[test]
fn validate_warns_on_scalar_field_count_mismatch() {
    // Template `point` has 2 scalar fields; the scalar supplies 3 flat values.
    let input = "#N struct point float x float y;\n\
                 #N canvas 0 22 450 300 12;\n\
                 #X scalar point 1 2 3;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(
        s.contains("point") && s.contains("field"),
        "expected a field-count warning; got:\n{s}"
    );
}

#[test]
fn validate_no_warning_for_consistent_scalar() {
    let input = "#N struct point float x float y;\n\
                 #N canvas 0 22 450 300 12;\n\
                 #X scalar point 10 20;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(
        !s.contains("template") && !s.contains("field"),
        "consistent scalar must not warn; got:\n{s}"
    );
}

#[test]
fn validate_scalar_with_array_field_counts_flat_only() {
    // holder has 2 scalar fields (x, y) plus an array field (z). The flat
    // value block (before the first `\;`) is `5 6` = 2, which matches; the
    // array element data must NOT be counted toward the scalar field count.
    let input = "#N struct holder float x float y array z element;\n\
                 #N struct element float v;\n\
                 #N canvas 0 22 450 300 12;\n\
                 #X scalar holder 5 6 \\; 1 \\; 2 \\;;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(
        !s.contains("field") && !s.contains("undefined template"),
        "array-field scalar must validate cleanly; got:\n{s}"
    );
}

#[test]
fn validate_no_dangling_warning_for_dollar_template() {
    // A `$`-scoped template name is realized per-instance; static matching is
    // unreliable, so it must not produce a false "undefined template" warning.
    let input = "#N canvas 0 22 450 300 12;\n#X scalar \\$0-foo 1 2;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(!s.contains("undefined template"), "got:\n{s}");
}
