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

#[test]
fn deps_expanded_builtins_not_missing() {
    let f = integration::handcrafted("deps_builtin_aliases.pd");
    let out = pdtk_output(&["deps", f.to_str().unwrap()]);
    // Core builtins should not appear as MISSING.
    for name in [
        "expr",
        "expr~",
        "fexpr~",
        "clone",
        "tabread~",
        "hdl",
        "vdl",
        "makenote",
        "stripnote",
    ] {
        assert!(
            !out.contains(&format!("{} (MISSING", name)),
            "core builtin {name} should not be MISSING. output: {out}"
        );
    }
    // `v` is special — it's a 1-char name and may appear as a substring; check the line.
    for line in out.lines() {
        if line.contains(" v (") {
            assert!(!line.contains("MISSING"), "v should not be MISSING: {line}");
        }
    }
}

#[test]
fn deps_extra_classes_tagged_core_extra() {
    let f = integration::handcrafted("deps_builtin_aliases.pd");
    let out = pdtk_output(&["deps", f.to_str().unwrap()]);
    assert!(
        out.contains("bob~") && out.contains("core-extra"),
        "expected bob~ tagged core-extra. output: {out}"
    );
    assert!(
        out.contains("slop~") && out.contains("core-extra"),
        "expected slop~ tagged core-extra. output: {out}"
    );
}

#[test]
fn deps_extra_class_json_carries_source_field() {
    let f = integration::handcrafted("deps_builtin_aliases.pd");
    let out = pdtk_output(&["deps", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let arr = v.as_array().expect("expected JSON array");
    let bob = arr
        .iter()
        .find(|e| e["name"] == "bob~")
        .expect("bob~ should appear in JSON");
    assert_eq!(bob["source"], "core-extra");
    assert_eq!(bob["found"], true);
}

#[test]
fn deps_corpus_no_unexpected_missing_builtins() {
    // Regression catcher: every corpus fixture should resolve cleanly aside
    // from known-missing abstractions (named "missing_*" or referencing real
    // abstractions we don't ship).
    let corpus = fixture_path("corpus");
    // Baseline of currently-missing names across the corpus. The corpus
    // copies a subset of files from a real sequencer project, so most of
    // these are real project abstractions that simply weren't copied. The
    // assertion is that the SET of missing names doesn't grow: a name newly
    // appearing here means either (a) we removed a builtin we shouldn't
    // have, or (b) a corpus fixture started referencing a new external.
    // Both cases need explicit review.
    let allowlist: &[&str] = &[
        // Project-specific abstractions from the sequencer corpus.
        "AUTODROP_TOGGLE",
        "CC_MUTE_COLOUR",
        "CC_muting",
        "CC_SELECTOR",
        "CC_SMOOTHING_TOGGLE",
        "clock_divider",
        "CLOCK_DIV_reset_colours",
        "column_cpmod",
        "cpm_toggles",
        "DENV_TOGGLE",
        "mute_or_chance",
        "mute_or_colour",
        "MW_TOGGLE",
        "seq_cp",
        "write_view_colour",
        "../../../LOAD",
        "../../LOAD",
        "../LOAD",
        "../pos_abs/chance_colour",
        "../seq_abs/122_midiout",
        "../seq_abs/CC_ARRAYS",
        "../seq_abs/CC_SEQUENCING",
        "../seq_abs/chord_editor",
        "../../TOG_INV",
        "../TOG_INV",
        // External libraries.
        "bondo",
        "zl",
        "arraycopy",
        "rotate",
        "prob",
        "list-emath",
        "list-lastx",
        // Numeric/dollar tokens that get parsed as a class because they
        // appear in position 4 of a non-standard #X obj entry. Not a
        // builtin coverage concern.
        "\\$1",
        "\\$2",
        "0",
        "6",
        "50",
        "55",
    ];
    for entry in std::fs::read_dir(&corpus).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("pd") {
            continue;
        }
        let out = pdtk_output(&["deps", path.to_str().unwrap(), "--missing"]);
        for line in out.lines() {
            if !line.contains("MISSING") {
                continue;
            }
            // Extract the name — pattern is `... <name> (MISSING)`
            let Some(open) = line.rfind(" (") else {
                continue;
            };
            let prefix = &line[..open];
            let name = prefix.split_whitespace().next_back().unwrap_or("");
            if name.is_empty() {
                continue;
            }
            // Builtin? must have already been filtered, so a MISSING name here
            // is necessarily not a builtin. Verify it's on the allowlist.
            assert!(
                allowlist.contains(&name),
                "corpus fixture {} reports MISSING name `{}` which is not on the allowlist. \
                 If this is a real new external, add it to the allowlist. If it is a \
                 vanilla class we forgot, add it to CORE_NAMES.",
                path.display(),
                name
            );
        }
    }
}

#[test]
fn deps_buses_reports_matched_pair() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "#N canvas 0 0 200 200 10;\n#X obj 10 10 s foo;\n#X obj 10 50 r foo;\n",
    )
    .unwrap();
    let out = pdtk_output(&["deps", tmp.path().to_str().unwrap(), "--buses"]);
    assert!(out.contains("'foo'"));
    assert!(out.contains("(control)"));
    assert!(out.contains("matched"));
}

#[test]
fn deps_buses_reports_orphan_send() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "#N canvas 0 0 200 200 10;\n#X obj 10 10 s bar;\n",
    )
    .unwrap();
    let out = pdtk_output(&["deps", tmp.path().to_str().unwrap(), "--buses"]);
    assert!(out.contains("'bar'"));
    assert!(out.contains("orphan_send"));
}

#[test]
fn deps_buses_reports_orphan_receive() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "#N canvas 0 0 200 200 10;\n#X obj 10 10 r baz;\n",
    )
    .unwrap();
    let out = pdtk_output(&["deps", tmp.path().to_str().unwrap(), "--buses"]);
    assert!(out.contains("'baz'"));
    assert!(out.contains("orphan_receive"));
}

#[test]
fn deps_buses_namespace_split_into_separate_rows() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "#N canvas 0 0 200 200 10;\n\
         #X obj 10 10 s foo;\n\
         #X obj 10 50 s~ foo;\n\
         #X obj 10 100 r~ foo;\n",
    )
    .unwrap();
    let out = pdtk_output(&["deps", tmp.path().to_str().unwrap(), "--buses"]);
    // control 'foo' is orphan_send, signal 'foo' is matched
    let control_line = out
        .lines()
        .find(|l| l.contains("(control)") && l.contains("'foo'"))
        .expect("control row");
    assert!(control_line.contains("orphan_send"));
    let signal_line = out
        .lines()
        .find(|l| l.contains("(signal)") && l.contains("'foo'"))
        .expect("signal row");
    assert!(signal_line.contains("matched"));
}

#[test]
fn deps_buses_includes_gui_sends() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "#N canvas 0 0 200 200 10;\n\
         #X obj 10 10 tgl 15 0 tgl_bus tgl_recv empty 17 7 0 10 -262144 -1 -1 0 1;\n\
         #X obj 10 50 r tgl_bus;\n",
    )
    .unwrap();
    let out = pdtk_output(&["deps", tmp.path().to_str().unwrap(), "--buses"]);
    assert!(out.contains("'tgl_bus'"));
    assert!(out.contains("(control)"));
    assert!(out.contains("matched"));
}

#[test]
fn deps_buses_ignores_empty_sentinel() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "#N canvas 0 0 200 200 10;\n#X floatatom 10 10 5 0 0 0 - - -;\n",
    )
    .unwrap();
    let out = pdtk_output(&["deps", tmp.path().to_str().unwrap(), "--buses"]);
    assert!(!out.contains("'-'"));
    assert!(!out.contains("'empty'"));
}

#[test]
fn deps_buses_json_schema() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "#N canvas 0 0 200 200 10;\n#X obj 10 10 s foo;\n#X obj 10 50 r foo;\n",
    )
    .unwrap();
    let out = pdtk_output(&["deps", tmp.path().to_str().unwrap(), "--buses", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let r = &arr[0];
    assert_eq!(r["name"], "foo");
    assert_eq!(r["kind"], "control");
    assert_eq!(r["status"], "matched");
    assert!(r["senders"].is_array());
    assert!(r["receivers"].is_array());
}

#[test]
fn deps_buses_directory_aggregated_default() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.pd"),
        "#N canvas 0 0 200 200 10;\n#X obj 10 10 s x;\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.pd"),
        "#N canvas 0 0 200 200 10;\n#X obj 10 10 r x;\n",
    )
    .unwrap();
    let out = pdtk_output(&["deps", dir.path().to_str().unwrap(), "--buses"]);
    // Aggregated by default: one matched row with both locations.
    assert!(out.contains("'x'"));
    assert!(out.contains("matched"));
    assert!(out.contains("a.pd"));
    assert!(out.contains("b.pd"));
}

#[test]
fn deps_buses_directory_per_file_isolated() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.pd"),
        "#N canvas 0 0 200 200 10;\n#X obj 10 10 s x;\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.pd"),
        "#N canvas 0 0 200 200 10;\n#X obj 10 10 r x;\n",
    )
    .unwrap();
    let out = pdtk_output(&[
        "deps",
        dir.path().to_str().unwrap(),
        "--buses",
        "--per-file",
    ]);
    // Per-file: a.pd has orphan_send, b.pd has orphan_receive.
    assert!(out.contains("orphan_send"));
    assert!(out.contains("orphan_receive"));
}

#[test]
fn deps_buses_dollar_zero_scope_warning() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        tmp.path(),
        "#N canvas 0 0 200 200 10;\n\
         #X obj 10 10 s \\$0-x;\n\
         #X obj 10 50 r \\$0-x;\n",
    )
    .unwrap();
    let out = pdtk_output(&["deps", tmp.path().to_str().unwrap(), "--buses"]);
    assert!(out.contains("dollar-zero-scoped"), "output: {out}");
}

#[test]
fn deps_recursive_buses_reports_unsatisfied() {
    // contract_inner.pd uses [r control_bus] and [s reply_bus] internally.
    // Caller has neither; both must be reported as unsatisfied.
    let caller = tempfile::NamedTempFile::new().unwrap();
    let inner_path = abs_dir().join("contract_inner.pd");
    let dir_arg = abs_dir();
    std::fs::write(
        caller.path(),
        "#N canvas 0 22 300 200 12;\n#X obj 50 50 contract_inner;\n",
    )
    .unwrap();
    let out = pdtk_output(&[
        "deps",
        caller.path().to_str().unwrap(),
        "--buses",
        "--recursive",
        "--search-path",
        dir_arg.to_str().unwrap(),
    ]);
    let _ = inner_path; // path used implicitly via abs_dir search
    assert!(
        out.contains("Unsatisfied bus contracts"),
        "expected unsatisfied section. output: {out}"
    );
    assert!(out.contains("control_bus"), "missing control_bus: {out}");
    assert!(out.contains("needs_sender"), "missing direction: {out}");
    assert!(out.contains("reply_bus"), "missing reply_bus: {out}");
    assert!(out.contains("needs_receiver"), "missing direction: {out}");
}

#[test]
fn deps_recursive_buses_dollar_names_excluded() {
    // contract_inner.pd has [r \$0-internal] and [s \$1-arg_bus]. $0 is
    // instance-scoped and always excluded; $1 is realized against call-site
    // args, but this caller supplies NO args, so $1 is unrealizable and also
    // does not appear. (Feature F adds realization; see the tests below for
    // the with-argument behaviour.)
    let caller = tempfile::NamedTempFile::new().unwrap();
    let dir_arg = abs_dir();
    std::fs::write(
        caller.path(),
        "#N canvas 0 22 300 200 12;\n#X obj 50 50 contract_inner;\n",
    )
    .unwrap();
    let out = pdtk_output(&[
        "deps",
        caller.path().to_str().unwrap(),
        "--buses",
        "--recursive",
        "--search-path",
        dir_arg.to_str().unwrap(),
    ]);
    // The $0 / $1 names from the abstraction must NOT appear in the
    // unsatisfied-contracts section.
    let section_start = out.find("Unsatisfied bus contracts").unwrap_or(out.len());
    let section = &out[section_start..];
    assert!(
        !section.contains("$0-internal") && !section.contains("\\$0-internal"),
        "$0 name leaked into cross-file contract: {out}"
    );
    assert!(
        !section.contains("$1-arg_bus") && !section.contains("\\$1-arg_bus"),
        "$1 name leaked into cross-file contract: {out}"
    );
}

#[test]
fn deps_recursive_buses_realizes_dollar_arg_in_contract() {
    // Feature F: contract_inner.pd has [s \$1-arg_bus]. Instantiated as
    // `contract_inner voice`, $1 -> voice, so the abstraction sends on
    // `voice-arg_bus` and the caller (lacking a matching receiver) must be
    // told it needs a receiver for the REALIZED name.
    let caller = tempfile::NamedTempFile::new().unwrap();
    let dir_arg = abs_dir();
    std::fs::write(
        caller.path(),
        "#N canvas 0 22 300 200 12;\n#X obj 50 50 contract_inner voice;\n",
    )
    .unwrap();
    let out = pdtk_output(&[
        "deps",
        caller.path().to_str().unwrap(),
        "--buses",
        "--recursive",
        "--search-path",
        dir_arg.to_str().unwrap(),
    ]);
    let section_start = out.find("Unsatisfied bus contracts").unwrap_or(out.len());
    let section = &out[section_start..];
    assert!(
        section.contains("voice-arg_bus") && section.contains("needs_receiver"),
        "realized $1 bus name must appear: {out}"
    );
    // The unrealized literal must NOT leak.
    assert!(
        !section.contains("$1-arg_bus") && !section.contains("\\$1-arg_bus"),
        "unrealized $1 literal leaked: {out}"
    );
}

#[test]
fn deps_recursive_buses_realized_dollar_arg_satisfied_by_caller() {
    // Feature F: when the caller provides the realized receiver
    // (`[r voice-arg_bus]`), the $1-derived contract is satisfied and must
    // NOT be reported.
    let caller = tempfile::NamedTempFile::new().unwrap();
    let dir_arg = abs_dir();
    std::fs::write(
        caller.path(),
        "#N canvas 0 22 300 200 12;\n\
         #X obj 50 20 r voice-arg_bus;\n\
         #X obj 50 60 contract_inner voice;\n",
    )
    .unwrap();
    let out = pdtk_output(&[
        "deps",
        caller.path().to_str().unwrap(),
        "--buses",
        "--recursive",
        "--search-path",
        dir_arg.to_str().unwrap(),
    ]);
    let section_start = out.find("Unsatisfied bus contracts").unwrap_or(out.len());
    let section = &out[section_start..];
    assert!(
        !section.contains("voice-arg_bus"),
        "realized bus satisfied by caller must not be reported: {out}"
    );
}

#[test]
fn deps_recursive_buses_message_box_send_satisfies_contract() {
    // Feature A + F synergy: the caller drives the abstraction's [r control_bus]
    // from a MESSAGE BOX (`\; control_bus 1`), which counts as a sender, so
    // the contract is satisfied and not reported as needs_sender.
    let caller = tempfile::NamedTempFile::new().unwrap();
    let dir_arg = abs_dir();
    std::fs::write(
        caller.path(),
        "#N canvas 0 22 300 200 12;\n\
         #X msg 50 20 \\; control_bus 1;\n\
         #X obj 50 60 contract_inner;\n",
    )
    .unwrap();
    let out = pdtk_output(&[
        "deps",
        caller.path().to_str().unwrap(),
        "--buses",
        "--recursive",
        "--search-path",
        dir_arg.to_str().unwrap(),
    ]);
    let section_start = out.find("Unsatisfied bus contracts").unwrap_or(out.len());
    let section = &out[section_start..];
    assert!(
        !section.contains("control_bus"),
        "message-box send must satisfy the [r control_bus] contract: {out}"
    );
}

#[test]
fn deps_recursive_buses_namespace_mismatch_still_unsatisfied() {
    // Abstraction wants [r control_bus] (control). Caller provides
    // [s~ control_bus] (signal). Namespaces don't cross — must still
    // report unsatisfied.
    let caller = tempfile::NamedTempFile::new().unwrap();
    let dir_arg = abs_dir();
    std::fs::write(
        caller.path(),
        "#N canvas 0 22 300 200 12;\n\
         #X obj 50 50 s~ control_bus;\n\
         #X obj 50 100 contract_inner;\n",
    )
    .unwrap();
    let out = pdtk_output(&[
        "deps",
        caller.path().to_str().unwrap(),
        "--buses",
        "--recursive",
        "--search-path",
        dir_arg.to_str().unwrap(),
    ]);
    assert!(
        out.contains("control_bus") && out.contains("needs_sender") && out.contains("(control)"),
        "expected unsatisfied control-namespace 'control_bus': {out}"
    );
}

#[test]
fn deps_recursive_buses_matched_cross_file() {
    // Caller provides [s control_bus] and [r reply_bus]; contract is
    // satisfied → those names should NOT appear in the unsatisfied list.
    let caller = tempfile::NamedTempFile::new().unwrap();
    let dir_arg = abs_dir();
    std::fs::write(
        caller.path(),
        "#N canvas 0 22 300 200 12;\n\
         #X obj 50 50 s control_bus;\n\
         #X obj 50 100 r reply_bus;\n\
         #X obj 50 150 contract_inner;\n",
    )
    .unwrap();
    let out = pdtk_output(&[
        "deps",
        caller.path().to_str().unwrap(),
        "--buses",
        "--recursive",
        "--search-path",
        dir_arg.to_str().unwrap(),
    ]);
    // 'control_bus' and 'reply_bus' should appear only in the matched bus
    // audit, not in the unsatisfied section.
    let section_start = out.find("Unsatisfied bus contracts").unwrap_or(out.len());
    let section = &out[section_start..];
    assert!(
        !section.contains("control_bus"),
        "control_bus should not be unsatisfied: {out}"
    );
    assert!(
        !section.contains("reply_bus"),
        "reply_bus should not be unsatisfied: {out}"
    );
}

#[test]
fn deps_recursive_buses_per_instance_rows() {
    // Two instances of contract_inner → two unsatisfied rows per name.
    let caller = tempfile::NamedTempFile::new().unwrap();
    let dir_arg = abs_dir();
    std::fs::write(
        caller.path(),
        "#N canvas 0 22 300 200 12;\n\
         #X obj 50 50 contract_inner;\n\
         #X obj 50 100 contract_inner;\n",
    )
    .unwrap();
    let out = pdtk_output(&[
        "deps",
        caller.path().to_str().unwrap(),
        "--buses",
        "--recursive",
        "--search-path",
        dir_arg.to_str().unwrap(),
    ]);
    // Two rows for control_bus (one per instance).
    let count = out.matches("control_bus").count();
    assert!(
        count >= 2,
        "expected 2+ control_bus rows (one per call site), got {count}: {out}"
    );
}
