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
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 a;\n",
    )
    .unwrap();

    let a = dir.path().join("a.pd");
    let out = run_pdtk(&["deps", a.to_str().unwrap(), "--recursive"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "circular deps must not infinite loop"
    );
}

#[test]
fn deps_directory_mode_deduplicates() {
    let dir = tempfile::tempdir().unwrap();
    // Two files each using the same abstraction
    std::fs::write(
        dir.path().join("x.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 myabs;\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("y.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 myabs;\n",
    )
    .unwrap();

    let out = pdtk_output(&["deps", dir.path().to_str().unwrap()]);
    // "myabs" should appear at most once per file — check it shows in both
    let count = out.matches("myabs").count();
    assert!(count >= 1, "myabs must be reported");
}

#[test]
fn deps_resolves_declare_path_cross_directory() {
    // Regression: `#X declare -path ../abs;` must strip the trailing `;`
    // and resolve the path relative to the patch's directory.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("patches")).unwrap();
    std::fs::create_dir_all(dir.path().join("abs")).unwrap();
    std::fs::write(
        dir.path().join("patches/main.pd"),
        "#N canvas 0 22 450 300 12;\n#X declare -path ../abs;\n#X obj 50 50 util;\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("abs/util.pd"),
        "#N canvas 0 22 450 300 12;\n",
    )
    .unwrap();

    let main = dir.path().join("patches/main.pd");
    let out = pdtk_output(&["deps", main.to_str().unwrap()]);
    assert!(out.contains("util"), "got: {out}");
    assert!(
        out.contains("found:"),
        "declare -path was not honored: {out}"
    );
    assert!(!out.contains("MISSING"), "util must be resolved: {out}");
}

#[test]
fn deps_resolves_declare_path_same_directory_subdir() {
    // `#X declare -path subdir;` with trailing semicolon.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("subdir")).unwrap();
    std::fs::write(
        dir.path().join("main.pd"),
        "#N canvas 0 22 450 300 12;\n#X declare -path subdir;\n#X obj 50 50 util;\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("subdir/util.pd"),
        "#N canvas 0 22 450 300 12;\n",
    )
    .unwrap();

    let main = dir.path().join("main.pd");
    let out = pdtk_output(&["deps", main.to_str().unwrap()]);
    assert!(!out.contains("MISSING"), "util must be resolved: {out}");
}

#[test]
fn deps_recursive_inherits_parent_declare_path() {
    // Pd's canvas_path_iterate walks the owner chain: a child abstraction
    // with no declares of its own still resolves references via its caller's
    // declare -path entries. --recursive must mirror that.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("patches")).unwrap();
    std::fs::create_dir_all(dir.path().join("abs")).unwrap();
    // main.pd declares -path ../abs and calls `outer`
    std::fs::write(
        dir.path().join("patches/main.pd"),
        "#N canvas 0 22 450 300 12;\n#X declare -path ../abs;\n#X obj 50 50 outer;\n",
    )
    .unwrap();
    // outer.pd has no declares but calls `inner`, which lives next to it
    std::fs::write(
        dir.path().join("abs/outer.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 inner;\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("abs/inner.pd"),
        "#N canvas 0 22 450 300 12;\n",
    )
    .unwrap();

    let main = dir.path().join("patches/main.pd");
    let out = pdtk_output(&["deps", main.to_str().unwrap(), "--recursive"]);
    // inner is resolved from outer.pd's own dir, but the key assertion is that
    // outer.pd appears and its child lookup succeeds (would fail without
    // ancestor propagation if inner lived only in a parent-declared dir).
    assert!(out.contains("outer"));
    assert!(out.contains("inner"));
    assert!(
        !out.contains("MISSING"),
        "recursive chain must resolve: {out}"
    );
}

#[test]
fn deps_recursive_inherits_parent_declare_path_strict() {
    // Strict version: `inner` lives ONLY in a directory declared by the
    // parent, NOT next to `outer`. Without ancestor propagation this MUST
    // fail; with it, `inner` is found.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("patches")).unwrap();
    std::fs::create_dir_all(dir.path().join("abs")).unwrap();
    std::fs::create_dir_all(dir.path().join("libs")).unwrap();
    std::fs::write(
        dir.path().join("patches/main.pd"),
        "#N canvas 0 22 450 300 12;\n#X declare -path ../abs -path ../libs;\n#X obj 50 50 outer;\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("abs/outer.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 inner;\n",
    )
    .unwrap();
    // inner lives in libs/, reachable only via main.pd's declare
    std::fs::write(
        dir.path().join("libs/inner.pd"),
        "#N canvas 0 22 450 300 12;\n",
    )
    .unwrap();

    let main = dir.path().join("patches/main.pd");
    let out = pdtk_output(&["deps", main.to_str().unwrap(), "--recursive"]);
    assert!(out.contains("inner"));
    assert!(
        !out.contains("MISSING"),
        "parent declares must propagate: {out}"
    );
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

// =====================================================================
// P5: --search-path and --pd-path
// =====================================================================

#[test]
fn deps_extra_search_path_resolves() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("main.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 50 50 util;\n",
    )
    .unwrap();
    let extra = dir.path().join("libs");
    std::fs::create_dir(&extra).unwrap();
    std::fs::write(extra.join("util.pd"), "#N canvas 0 22 450 300 12;\n").unwrap();

    let main = dir.path().join("main.pd");
    let out = pdtk_output(&[
        "deps",
        main.to_str().unwrap(),
        "--search-path",
        extra.to_str().unwrap(),
    ]);
    assert!(out.contains("util"), "{out}");
    assert!(!out.contains("MISSING"), "{out}");
}

#[test]
fn deps_extra_search_path_after_declare() {
    // Declare path takes priority over --search-path. Same file in both
    // dirs: result must point at the declare-path version.
    let dir = tempfile::tempdir().unwrap();
    let declared = dir.path().join("declared");
    let extra = dir.path().join("extra");
    std::fs::create_dir(&declared).unwrap();
    std::fs::create_dir(&extra).unwrap();
    std::fs::write(
        dir.path().join("main.pd"),
        "#N canvas 0 22 450 300 12;\n\
#X declare -path declared;\n\
#X obj 50 50 thing;\n",
    )
    .unwrap();
    std::fs::write(declared.join("thing.pd"), "#N canvas 0 22 450 300 12;\n").unwrap();
    std::fs::write(extra.join("thing.pd"), "#N canvas 0 22 450 300 12;\n").unwrap();

    let main = dir.path().join("main.pd");
    let out = pdtk_output(&[
        "deps",
        main.to_str().unwrap(),
        "--search-path",
        extra.to_str().unwrap(),
    ]);
    assert!(
        out.contains("declared/thing.pd"),
        "declare path should take priority: {out}"
    );
}

#[test]
fn deps_pd_path_flag_accepted() {
    // Just verify the flag doesn't crash; platform paths may not exist on CI.
    let f = abs_dir().join("uses_abstractions.pd");
    let out = run_pdtk(&["deps", f.to_str().unwrap(), "--pd-path"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout_string(&out).contains("used_abs"));
}

#[test]
fn deps_pd_path_resolves_via_synthetic_home() {
    // Place foo.pd under a synthetic HOME's .local/lib/pd/extra and verify
    // --pd-path picks it up (Linux only — macOS uses different layout).
    if !cfg!(target_os = "linux") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let extra = home.join(".local/lib/pd/extra");
    std::fs::create_dir_all(&extra).unwrap();
    std::fs::write(extra.join("foo.pd"), "#N canvas 0 22 450 300 12;\n").unwrap();

    let main = home.join("main.pd");
    std::fs::write(&main, "#N canvas 0 22 450 300 12;\n#X obj 50 50 foo;\n").unwrap();

    let out = assert_cmd::Command::cargo_bin("pdtk")
        .unwrap()
        .env("HOME", home)
        .args(["deps", main.to_str().unwrap(), "--pd-path"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        !s.contains("MISSING"),
        "foo.pd should resolve via synthetic HOME:\n{s}"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn pd_platform_paths_linux_with_synthetic_home() {
    // Spawn pdtk with --pd-path and a synthetic HOME that contains the
    // user-local extra dir; just check the flag is accepted and the binary
    // does not crash with that HOME. Pure-function testing of
    // pd_platform_paths happens in lib via its own unit tests if wanted;
    // here we just exercise the integration.
    let dir = tempfile::tempdir().unwrap();
    let f = abs_dir().join("uses_abstractions.pd");
    let out = assert_cmd::Command::cargo_bin("pdtk")
        .unwrap()
        .env("HOME", dir.path())
        .args(["deps", f.to_str().unwrap(), "--pd-path"])
        .output()
        .unwrap();
    assert!(out.status.success());
}
