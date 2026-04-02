mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk};

#[test]
fn insert_at_beginning_renumbers_all_connections() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "insert",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--entry",
        "#X obj 50 25 loadbang;",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // Was: f(0)→print(1).  Now: loadbang(0), f(1), print(2).
    // Connection 0→1 becomes 1→2
    assert!(result.contains("#X connect 1 0 2 0;"));
    assert!(!result.contains("#X connect 0 0 1 0;"));
}

#[test]
fn insert_at_end_no_renumbering() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "insert",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "2",
        "--entry",
        "#X obj 50 150 bang;",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // Connection unchanged: 0→1
    assert!(result.contains("#X connect 0 0 1 0;"));
    assert!(result.contains("#X obj 50 150 bang;"));
}

#[test]
fn insert_in_middle_renumbers_only_affected() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 + 1;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 1 0 2 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "insert",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--entry",
        "#X obj 50 75 t f f;",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // f(0), t f f(1), + 1(2), print(3)
    // Old 0→1 becomes 0→2, old 1→2 becomes 2→3
    assert!(result.contains("#X connect 0 0 2 0;"));
    assert!(result.contains("#X connect 2 0 3 0;"));
}

#[test]
fn insert_into_subpatch_only_affects_that_depth() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 inlet;\n\
                 #N canvas 0 0 450 300 sub 0;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 outlet;\n\
                 #X connect 0 0 1 0;\n\
                 #X restore 50 100 pd sub;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 1 0 2 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "insert",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--index",
        "1",
        "--entry",
        "#X obj 50 75 + 1;",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // Depth 1: f(0) → + 1(1) → outlet(2).  Connection 0→1 becomes 0→2.
    assert!(result.contains("#X connect 0 0 2 0;"));
    // Depth 0 connections should be unchanged: 0→1, 1→2
    // (counted from depth 0 end of file)
    let lines: Vec<&str> = result.lines().collect();
    let depth0_conns: Vec<&&str> = lines
        .iter()
        .filter(|l| l.starts_with("#X connect"))
        .collect();
    // Should have 3 connections total: 1 at depth 1 (0→2), 2 at depth 0 (0→1, 1→2)
    assert_eq!(depth0_conns.len(), 3);
}

#[test]
fn insert_inserted_object_correct_position() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "insert",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--entry",
        "#X obj 50 75 + 1;",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines[0], "#N canvas 0 22 450 300 12;");
    assert_eq!(lines[1], "#X obj 50 50 f;");
    assert_eq!(lines[2], "#X obj 50 75 + 1;");
    assert_eq!(lines[3], "#X obj 50 100 print;");
}

#[test]
fn insert_out_of_range_index_exits_2() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&[
        "insert",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "10",
        "--entry",
        "#X obj 50 50 f;",
    ]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn insert_validates_after_mutation() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&[
        "insert",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--entry",
        "#X obj 50 75 bang;",
    ]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn insert_then_validate_exit_0() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "insert",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--entry",
        "#X obj 50 75 bang;",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let out = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn insert_output_flag_writes_to_file() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "insert",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--entry",
        "#X obj 50 25 bang;",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("#X obj 50 25 bang;"));
}

#[test]
fn insert_backup_creates_bak_file() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();
    let backup_path = format!("{}.bak", tmp.path().display());

    pdtk_output(&[
        "insert",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--entry",
        "#X obj 50 25 loadbang;",
        "--in-place",
        "--backup",
    ]);

    assert!(std::path::Path::new(&backup_path).exists());
    let backup_content = std::fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, input);
    std::fs::remove_file(&backup_path).ok();
}

/// Insert at I then delete at I → original (connections preserved since no
/// connections touch the inserted object).
#[test]
fn insert_then_delete_roundtrip() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(f.path(), input).unwrap();

    pdtk_output(&[
        "insert",
        f.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--entry",
        "#X obj 50 75 bang;",
        "--in-place",
    ]);

    pdtk_output(&[
        "delete",
        f.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(f.path()).unwrap();
    assert_eq!(result, input);
}

/// No write should happen when validation fails.
#[test]
fn no_write_if_validation_fails_insert() {
    // Create a file, then try an insert that would corrupt it
    // (We can't easily cause post-insert validation failure with valid input,
    // but we CAN verify out-of-range index doesn't modify the file)
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&[
        "insert",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "99",
        "--entry",
        "#X obj 50 50 bang;",
        "--in-place",
    ]);
    assert_ne!(out.status.code(), Some(0));
    // File should be unmodified
    let after = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(after, input);
}
