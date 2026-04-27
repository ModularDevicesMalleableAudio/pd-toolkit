mod integration;

use integration::{fixture_path, handcrafted, pdtk_output, run_pdtk, stderr_string, stdout_string};

#[test]
fn delete_object_with_connections_removes_obj_and_conns() {
    // Delete index 1 (+ 1): both connections touch it, so both are removed.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 + 1;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 1 0 2 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // Both connections involved index 1 → both removed.
    assert!(!result.contains("#X connect"));
    assert!(!result.contains("+ 1"));
    // Remaining objects: f (0), print (1)
    assert!(result.contains("#X obj 50 50 f;"));
    assert!(result.contains("#X obj 50 150 print;"));
}

#[test]
fn delete_object_no_connections_removes_only_obj() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 bang;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 2 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // bang deleted; connection 0→2 becomes 0→1
    assert!(result.contains("#X connect 0 0 1 0;"));
    assert!(!result.contains("bang"));
}

#[test]
fn delete_last_object_no_renumbering() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // print deleted; connection 0→1 involved 1 → removed
    assert!(!result.contains("#X connect"));
    assert!(result.contains("#X obj 50 50 f;"));
}

#[test]
fn delete_first_renumbers_remaining() {
    // Delete index 0 from a chain: remaining connections should shift down.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 loadbang;\n\
                 #X obj 50 100 t b;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 1 0 2 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // loadbang removed; connection 0→1 removed (touched 0).
    // Connection 1→2 renumbered to 0→1.
    assert!(result.contains("#X connect 0 0 1 0;"));
    assert!(!result.contains("loadbang"));
}

#[test]
fn delete_only_affects_correct_depth() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 inlet;\n\
                 #N canvas 0 0 450 300 sub 0;\n\
                 #X obj 50 50 + 1;\n\
                 #X obj 50 100 outlet;\n\
                 #X connect 0 0 1 0;\n\
                 #X restore 50 100 pd sub;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 1 0 2 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    // Delete depth 1, index 0 (the + 1 inside the subpatch)
    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--index",
        "0",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // Depth 0 connections (0→1, 1→2) must be unchanged
    // (They appear after the restore line in the file)
    assert!(result.contains("inlet"));
    assert!(result.contains("print"));
}

#[test]
fn delete_out_of_range_index_exits_2() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&[
        "delete",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "100",
    ]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn delete_validates_after_mutation() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
    ]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn delete_then_validate_exit_0() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "delete",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn delete_output_flag_writes_to_file() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    let out = run_pdtk(&[
        "delete",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // loadbang was at index 0; after deletion it should be gone
    assert!(!result.contains("loadbang"));
    // stdout should be empty (written to file)
    assert!(stdout_string(&out).trim().is_empty());
}

#[test]
fn delete_backup_creates_bak_file() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();
    let backup_path = format!("{}.bak", tmp.path().display());

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--in-place",
        "--backup",
    ]);

    assert!(std::path::Path::new(&backup_path).exists());
    let backup_content = std::fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, input);
    std::fs::remove_file(&backup_path).ok();
}

/// Delete at I then insert same object text at I → original.
/// Uses an object NOT involved in any connections so connections survive.
#[test]
fn delete_then_insert_roundtrip() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 bang;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 2 0;\n";
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(f.path(), input).unwrap();

    // Delete index 1 (bang) — not connected, so connections survive
    pdtk_output(&[
        "delete",
        f.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--in-place",
    ]);

    // Re-insert bang at index 1
    pdtk_output(&[
        "insert",
        f.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--entry",
        "#X obj 50 100 bang;",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(f.path()).unwrap();
    assert_eq!(result, input);
}

/// No write should happen when validation fails.
#[test]
fn no_write_if_validation_fails_delete() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "99",
        "--in-place",
    ]);
    assert_ne!(out.status.code(), Some(0));
    let after = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(after, input);
}

#[test]
fn delete_subpatch_removes_canvas_and_inner_objects() {
    // Deleting the Restore entry at depth 0, index 1 must atomically remove
    // the matching #N canvas, all inner objects and connections, and the
    // #X restore line — not just the restore line alone.
    let f = handcrafted("subpatch_delete.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "delete",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // The #N canvas opening the subpatch must be gone
    assert!(
        !result.contains("#N canvas 0 0 450 300 inner"),
        "subpatch #N canvas must be removed; got:\n{result}"
    );
    // Inner objects must be gone
    assert!(
        !result.contains("+ 1"),
        "inner object must be removed; got:\n{result}"
    );
    // Outer objects must survive
    assert!(result.contains("loadbang"), "loadbang must survive");
    assert!(result.contains("bang"), "bang must survive");
    assert!(result.contains("print"), "print must survive");
}

#[test]
fn delete_subpatch_renumbers_connections_correctly() {
    // After deleting the subpatch at index 1, connections that touched it
    // are removed and connection 2→3 (bang→print) must renumber to 1→2.
    let f = handcrafted("subpatch_delete.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "delete",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // Inner connections are gone along with the subpatch; outer connections
    // touching the deleted index are removed; only bang→print survives.
    let connect_count = result.matches("#X connect").count();
    assert_eq!(
        connect_count, 1,
        "exactly one connection must remain; got:\n{result}"
    );
    assert!(
        result.contains("#X connect 1 0 2 0;"),
        "bang→print must be renumbered from 2→3 to 1→2; got:\n{result}"
    );
}

#[test]
fn delete_subpatch_produces_valid_output() {
    // pdtk validate must exit 0 after the delete — catches canvas depth
    // imbalance that the buggy single-line delete would leave behind.
    let f = handcrafted("subpatch_delete.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "delete",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "validate failed after subpatch delete:\n{}",
        stderr_string(&out)
    );
}

#[test]
fn delete_subpatch_with_nested_inner_canvases() {
    // deep_subpatch.pd: depth-0 index 1 is level1, which itself contains
    // level2 → level3 (three canvas levels total).  All must be removed.
    let f = handcrafted("deep_subpatch.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "delete",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        !result.contains("level1"),
        "level1 canvas must be gone; got:\n{result}"
    );
    assert!(
        !result.contains("level2"),
        "level2 canvas must be gone; got:\n{result}"
    );
    assert!(
        !result.contains("level3"),
        "level3 canvas must be gone; got:\n{result}"
    );
    // Only root canvas should remain
    let canvas_count = result.matches("#N canvas").count();
    assert_eq!(
        canvas_count, 1,
        "only root canvas must remain; got:\n{result}"
    );

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "validate failed:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn delete_first_of_two_sibling_subpatches_renumbers_second() {
    // multiple_subpatches.pd: loadbang(0) sub_a(1) sub_b(2) print_from_a(3)
    // print_from_b(4).  Deleting sub_a must shift sub_b to index 1 and leave
    // exactly root + sub_b as the remaining canvases.
    let f = handcrafted("multiple_subpatches.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "delete",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        !result.contains("sub_a 0;"),
        "sub_a canvas header must be gone; got:\n{result}"
    );
    assert!(result.contains("pd sub_b"), "sub_b must survive");
    assert!(result.contains("print from_a"), "print from_a must survive");
    assert!(result.contains("print from_b"), "print from_b must survive");

    let canvas_count = result.matches("#N canvas").count();
    assert_eq!(
        canvas_count, 2,
        "root + sub_b must be the only canvases; got:\n{result}"
    );

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "validate failed:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn delete_subpatch_corpus_width_hint_real_validate() {
    // Real-world corpus: width_hint_real.pd has a `lhs_rhs` subpatch at
    // depth 0 index 57 (57 objects precede its #N canvas header) followed
    // by a `#X f 38;` width hint.  Top-level objects after the subpatch
    // (tabread, tabwrite) must survive and the result must pass validate.
    let f = fixture_path("corpus/width_hint_real.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "delete",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "57",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        !result.contains("lhs_rhs 0;"),
        "lhs_rhs canvas header must be gone; got (truncated):\n{}",
        &result[..result.len().min(500)]
    );
    // Distinctive inner object of lhs_rhs
    assert!(
        !result.contains("LOAD 10 5"),
        "inner lhs_rhs object must be gone"
    );
    // Top-level objects that follow the deleted subpatch must survive
    assert!(
        result.contains("tabread pulse_mutes"),
        "top-level tabread objects must survive"
    );

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "validate failed after deleting lhs_rhs:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

// =====================================================================
// P2: --subpatch flag tests
// =====================================================================

const SIMPLE_SUBPATCH: &str = "#N canvas 0 22 450 300 12;\n\
#X obj 50 50 loadbang;\n\
#N canvas 0 0 450 300 sub 0;\n\
#X obj 50 50 + 1;\n\
#X obj 50 100 outlet;\n\
#X connect 0 0 1 0;\n\
#X restore 50 100 pd sub;\n\
#X obj 50 150 print;\n\
#X connect 0 0 1 0;\n\
#X connect 1 0 2 0;\n";

#[test]
fn delete_subpatch_by_depth() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), SIMPLE_SUBPATCH).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        !result.contains("pd sub"),
        "subpatch must be gone:\n{result}"
    );
    assert!(!result.contains("+ 1"), "subpatch interior must be gone");
    assert!(!result.contains("outlet"), "subpatch interior must be gone");
    assert!(result.contains("loadbang"), "loadbang must remain");
    assert!(result.contains("print"), "print must remain");
    // Both parent connections referenced the deleted sub (idx 1) and are removed.
    assert!(
        !result.contains("#X connect"),
        "all parent conns touched the sub:\n{result}"
    );
}

#[test]
fn delete_subpatch_default_index_zero() {
    // Same as above but verifies that omitting --index works
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), SIMPLE_SUBPATCH).unwrap();
    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(!result.contains("pd sub"));
}

#[test]
fn delete_subpatch_explicit_index_zero_works() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), SIMPLE_SUBPATCH).unwrap();
    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--index",
        "0",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(!result.contains("pd sub"));
}

#[test]
fn delete_subpatch_second_at_depth() {
    let input = "#N canvas 0 22 450 300 12;\n\
#N canvas 0 0 450 300 first 0;\n\
#X obj 10 10 nop1;\n\
#X restore 50 50 pd first;\n\
#N canvas 0 0 450 300 second 0;\n\
#X obj 10 10 nop2;\n\
#X restore 50 100 pd second;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--index",
        "1",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        result.contains("pd first"),
        "first subpatch must remain:\n{result}"
    );
    assert!(
        !result.contains("pd second"),
        "second subpatch must be gone:\n{result}"
    );
    assert!(result.contains("nop1"));
    assert!(!result.contains("nop2"));
}

#[test]
fn delete_subpatch_renumbers_parent() {
    let input = "#N canvas 0 22 450 300 12;\n\
#X obj 10 10 a;\n\
#N canvas 0 0 450 300 sub 0;\n\
#X obj 10 10 inner;\n\
#X restore 50 50 pd sub;\n\
#X obj 10 50 b;\n\
#X obj 10 90 c;\n\
#X connect 0 0 1 0;\n\
#X connect 2 0 3 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // a(0), b(1), c(2) remain; b->c connection is 1->2 (renumbered from 2->3)
    assert!(
        result.contains("#X connect 1 0 2 0;"),
        "b->c renumbered:\n{result}"
    );
    // a -> sub connection is gone (touched deleted index 1)
    assert!(!result.contains("#X connect 0 0 1 0;"));
}

#[test]
fn delete_subpatch_with_width_hint() {
    let input = "#N canvas 0 22 450 300 12;\n\
#N canvas 0 0 450 300 sub 0;\n\
#X obj 10 10 inner;\n\
#X restore 50 50 pd sub;\n\
#X f 38;\n\
#X obj 10 50 keep;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        !result.contains("#X f 38"),
        "width hint must be gone:\n{result}"
    );
    assert!(!result.contains("pd sub"));
    assert!(result.contains("keep"));
}

#[test]
fn delete_subpatch_nested() {
    let input = "#N canvas 0 22 450 300 12;\n\
#N canvas 0 0 450 300 outer 0;\n\
#X obj 10 10 a;\n\
#N canvas 0 0 450 300 inner 0;\n\
#X obj 10 10 deep;\n\
#X restore 50 50 pd inner;\n\
#X restore 50 100 pd outer;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(!result.contains("pd outer"));
    assert!(!result.contains("pd inner"));
    assert!(!result.contains("deep"));
}

#[test]
fn delete_subpatch_no_match_exits_2() {
    let input = "#N canvas 0 22 450 300 12;\n#X obj 50 50 nop;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();
    let out = run_pdtk(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "5",
        "--subpatch",
        "--in-place",
    ]);
    assert!(!out.status.success());
    assert!(stderr_string(&out).contains("no subpatch found"));
}

#[test]
fn delete_subpatch_root_depth_rejected() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), SIMPLE_SUBPATCH).unwrap();
    let out = run_pdtk(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--subpatch",
        "--in-place",
    ]);
    assert!(!out.status.success());
    assert!(stderr_string(&out).contains("root canvas"));
}

#[test]
fn delete_subpatch_index_out_of_range_exits_2() {
    let input = "#N canvas 0 22 450 300 12;\n\
#N canvas 0 0 450 300 a 0;\n#X restore 50 50 pd a;\n\
#N canvas 0 0 450 300 b 0;\n#X restore 50 80 pd b;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();
    let out = run_pdtk(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--index",
        "99",
        "--in-place",
    ]);
    assert!(!out.status.success());
}

#[test]
fn delete_subpatch_index_skips_deeper_canvases() {
    // sub_a (depth 1) contains nested (depth 2), then sub_b (depth 1).
    let input = "#N canvas 0 22 450 300 12;\n\
#N canvas 0 0 450 300 sub_a 0;\n\
#N canvas 0 0 450 300 nested 0;\n\
#X obj 10 10 deep;\n\
#X restore 50 50 pd nested;\n\
#X restore 50 50 pd sub_a;\n\
#N canvas 0 0 450 300 sub_b 0;\n\
#X obj 10 10 b_inner;\n\
#X restore 50 100 pd sub_b;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    // --depth 1 --index 1 should remove sub_b, NOT nested
    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--index",
        "1",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("pd sub_a"), "sub_a must remain:\n{result}");
    assert!(
        result.contains("pd nested"),
        "nested must remain:\n{result}"
    );
    assert!(
        !result.contains("pd sub_b"),
        "sub_b must be gone:\n{result}"
    );

    // Now test --depth 2 --index 0 removes nested only
    let tmp2 = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp2.path(), input).unwrap();
    pdtk_output(&[
        "delete",
        tmp2.path().to_str().unwrap(),
        "--depth",
        "2",
        "--subpatch",
        "--in-place",
    ]);
    let r2 = std::fs::read_to_string(tmp2.path()).unwrap();
    assert!(r2.contains("pd sub_a"));
    assert!(r2.contains("pd sub_b"));
    assert!(!r2.contains("pd nested"));
}

#[test]
fn delete_subpatch_connection_depth_not_off_by_one() {
    // Parent has 4 objects: a(0), sub(1), b(2), c(3).
    // Subpatch interior has connection 0->1 at internal depth 2.
    // Parent connections at internal depth 1: a->sub (0->1), b->c (2->3).
    let input = "#N canvas 0 22 450 300 12;\n\
#X obj 10 10 a;\n\
#N canvas 0 0 450 300 sub 0;\n\
#X obj 10 10 inner1;\n\
#X obj 10 50 inner2;\n\
#X connect 0 0 1 0;\n\
#X restore 50 50 pd sub;\n\
#X obj 10 90 b;\n\
#X obj 10 130 c;\n\
#X connect 0 0 1 0;\n\
#X connect 2 0 3 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--subpatch",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // (a) interior connection gone with the span
    assert!(!result.contains("inner1"));
    assert!(!result.contains("inner2"));
    // (b) parent b->c (2->3) renumbered to 1->2
    assert!(
        result.contains("#X connect 1 0 2 0;"),
        "b->c renumbered:\n{result}"
    );
    // (c) parent connection touching deleted index 1 (a->sub) removed
    let connect_lines: Vec<&str> = result
        .lines()
        .filter(|l| l.starts_with("#X connect"))
        .collect();
    assert_eq!(
        connect_lines.len(),
        1,
        "exactly one connect remains:\n{result}"
    );
    // (d) no connect references a non-existent index (we have 3 objects: a(0), b(1), c(2))
    for line in &connect_lines {
        for tok in line.split_whitespace() {
            if let Ok(n) = tok.parse::<usize>() {
                if n > 2 && n != 0 {
                    // check it's not referring to an object index >=3
                    // simpler: src/dst at positions 2 and 4 of "#X connect SRC O DST I;"
                }
            }
        }
        let parts: Vec<&str> = line.trim_end_matches(';').split_whitespace().collect();
        let src: usize = parts[2].parse().unwrap();
        let dst: usize = parts[4].parse().unwrap();
        assert!(src < 3 && dst < 3, "connection out of range: {line}");
    }
}

#[test]
fn delete_restore_drains_trailing_width_hint() {
    // Pre-existing bug fix: deleting via --depth 0 --index I where I points
    // at a Restore should also drain the trailing #X f N width hint.
    let input = "#N canvas 0 22 450 300 12;\n\
#X obj 10 10 a;\n\
#N canvas 0 0 450 300 sub 0;\n\
#X obj 10 10 inner;\n\
#X restore 50 50 pd sub;\n\
#X f 42;\n\
#X obj 10 90 b;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    // Restore is object index 1 at depth 0
    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        !result.contains("#X f 42"),
        "width hint must be gone:\n{result}"
    );
    assert!(!result.contains("pd sub"));
    assert!(result.contains(" b;"));
}

#[test]
fn delete_index_required_without_subpatch_flag() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), SIMPLE_SUBPATCH).unwrap();
    let out = run_pdtk(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--in-place",
    ]);
    assert!(!out.status.success(), "should fail without --index");
    let err = stderr_string(&out);
    assert!(
        err.contains("--index") || err.contains("required"),
        "error should mention --index requirement:\n{err}"
    );
}
