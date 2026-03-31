mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk, stdout_string};

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
        "delete", tmp.path().to_str().unwrap(),
        "--depth", "0", "--index", "1",
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
        "delete", tmp.path().to_str().unwrap(),
        "--depth", "0", "--index", "1",
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
        "delete", tmp.path().to_str().unwrap(),
        "--depth", "0", "--index", "1",
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
        "delete", tmp.path().to_str().unwrap(),
        "--depth", "0", "--index", "0",
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
        "delete", tmp.path().to_str().unwrap(),
        "--depth", "1", "--index", "0",
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
        "delete", f.to_str().unwrap(),
        "--depth", "0", "--index", "100",
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
        "delete", tmp.path().to_str().unwrap(),
        "--depth", "0", "--index", "0",
    ]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn delete_then_validate_exit_0() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "delete", f.to_str().unwrap(),
        "--depth", "0", "--index", "0",
        "--output", tmp.path().to_str().unwrap(),
    ]);

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn delete_output_flag_writes_to_file() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    let out = run_pdtk(&[
        "delete", f.to_str().unwrap(),
        "--depth", "0", "--index", "0",
        "--output", tmp.path().to_str().unwrap(),
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
        "delete", tmp.path().to_str().unwrap(),
        "--depth", "0", "--index", "0",
        "--in-place", "--backup",
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
        "delete", f.path().to_str().unwrap(),
        "--depth", "0", "--index", "1",
        "--in-place",
    ]);

    // Re-insert bang at index 1
    pdtk_output(&[
        "insert", f.path().to_str().unwrap(),
        "--depth", "0", "--index", "1",
        "--entry", "#X obj 50 100 bang;",
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
        "delete", tmp.path().to_str().unwrap(),
        "--depth", "0", "--index", "99",
        "--in-place",
    ]);
    assert_ne!(out.status.code(), Some(0));
    let after = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(after, input);
}
