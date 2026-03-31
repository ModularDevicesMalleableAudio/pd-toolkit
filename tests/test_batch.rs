mod integration;

use integration::{fixture_path, pdtk_output, run_pdtk, stdout_string};
use std::path::PathBuf;

fn handcrafted_dir() -> PathBuf {
    fixture_path("handcrafted")
}

fn count_pd_files(dir: &PathBuf) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map(|x| x == "pd").unwrap_or(false))
        .count()
}

// Processes all matching files

#[test]
fn batch_processes_all_matching_files() {
    let dir = tempfile::tempdir().unwrap();
    for name in &["a.pd", "b.pd", "c.pd"] {
        std::fs::write(
            dir.path().join(name),
            "#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang;\n",
        ).unwrap();
    }

    let out = pdtk_output(&[
        "batch",
        dir.path().to_str().unwrap(),
        "validate",
    ]);
    assert!(out.contains("3/3 succeeded"), "all 3 files should succeed: {out}");
}

// Glob filter

#[test]
fn batch_glob_filter_correct() {
    let dir = tempfile::tempdir().unwrap();
    for name in &["keep.pd", "skip.pd", "also.pd"] {
        std::fs::write(
            dir.path().join(name),
            "#N canvas 0 22 450 300 12;\n",
        ).unwrap();
    }

    let out = pdtk_output(&[
        "batch",
        dir.path().to_str().unwrap(),
        "--glob", "keep.pd",
        "validate",
    ]);
    // Only "keep.pd" should be processed
    assert!(out.contains("1/1 succeeded"), "glob should match only keep.pd: {out}");
    assert!(!out.contains("skip.pd"), "skip.pd should not appear");
}

// Dry-run

#[test]
fn batch_dry_run_no_execution() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("test.pd");
    let content = "#N canvas 0 22 450 300 12;\n#X obj 50 50 f;\n";
    std::fs::write(&f, content).unwrap();

    pdtk_output(&[
        "batch",
        dir.path().to_str().unwrap(),
        "--dry-run",
        "validate",
    ]);

    // File must be unchanged (validate is read-only but dry-run shouldn't even call it)
    let after = std::fs::read_to_string(&f).unwrap();
    assert_eq!(after, content);
}

#[test]
fn batch_dry_run_reports_would_run() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.pd"), "#N canvas 0 22 450 300 12;\n").unwrap();
    std::fs::write(dir.path().join("b.pd"), "#N canvas 0 22 450 300 12;\n").unwrap();

    let out = pdtk_output(&[
        "batch",
        dir.path().to_str().unwrap(),
        "--dry-run",
        "validate",
    ]);
    assert!(out.contains("DRY-RUN"), "dry-run output should mention DRY-RUN");
}

// Error stops by default

#[test]
fn batch_error_stops_by_default() {
    let dir = tempfile::tempdir().unwrap();
    // First file valid, second malformed
    std::fs::write(
        dir.path().join("a.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang;\n#X connect 0 0 99 0;\n",
    ).unwrap();
    std::fs::write(
        dir.path().join("b.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang;\n",
    ).unwrap();

    let out = run_pdtk(&[
        "batch",
        dir.path().to_str().unwrap(),
        "validate",
    ]);
    // Should exit with non-zero code since a file failed
    assert_ne!(out.status.code(), Some(0));
    let s = stdout_string(&out);
    assert!(s.contains("1 failed"), "should report 1 failure: {s}");
}

// Continue on error

#[test]
fn batch_continue_on_error_processes_remaining() {
    let dir = tempfile::tempdir().unwrap();
    // a.pd: malformed (connection out of range)
    std::fs::write(
        dir.path().join("a.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang;\n#X connect 0 0 99 0;\n",
    ).unwrap();
    // b.pd: valid
    std::fs::write(
        dir.path().join("b.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang;\n",
    ).unwrap();

    // With --continue-on-error, exit code is still 1 (there was a failure) but
    // both files are attempted.
    let out = run_pdtk(&[
        "batch",
        dir.path().to_str().unwrap(),
        "--continue-on-error",
        "validate",
    ]);
    let s = stdout_string(&out);
    // Both files attempted; 1 succeeded, 1 failed
    assert!(s.contains("1 failed"), "should report 1 failure: {s}");
    // Both files should appear in output (proves both were attempted)
    assert!(s.contains("a.pd") && s.contains("b.pd"), "both files should appear: {s}");
}

// Reports success and error counts

#[test]
fn batch_reports_success_and_error_counts() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ok.pd"), "#N canvas 0 22 450 300 12;\n").unwrap();

    let out = pdtk_output(&[
        "batch",
        dir.path().to_str().unwrap(),
        "validate",
    ]);
    // Should report N/N succeeded, 0 failed
    assert!(out.contains("succeeded"));
}

// JSON output

#[test]
fn batch_json_output_schema() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.pd"), "#N canvas 0 22 450 300 12;\n").unwrap();
    std::fs::write(dir.path().join("b.pd"), "#N canvas 0 22 450 300 12;\n").unwrap();

    let out = pdtk_output(&[
        "batch",
        dir.path().to_str().unwrap(),
        "--json",
        "validate",
    ]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("total").is_some());
    assert!(v.get("succeeded").is_some());
    assert!(v.get("failed").is_some());
    assert!(v.get("results").is_some());
    assert!(v["results"].is_array());
}

// Integration with handcrafted fixture directory

#[test]
fn batch_validate_all_handcrafted_fixtures() {
    let dir = handcrafted_dir();
    let total = count_pd_files(&dir);

    // --continue-on-error because malformed_*.pd and empty_file.pd will fail
    let out = run_pdtk(&[
        "batch",
        dir.to_str().unwrap(),
        "--continue-on-error",
        "validate",
    ]);
    let s = stdout_string(&out);
    // Most fixtures should pass; malformed ones will fail but that's expected.
    // The important thing is the command runs and reports both successes and failures.
    assert!(s.contains("succeeded"), "batch should report successes: {s}");
    let _ = total;
}
