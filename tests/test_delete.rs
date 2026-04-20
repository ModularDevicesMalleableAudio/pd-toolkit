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
