mod integration;

use integration::{pdtk_output, run_pdtk, stdout_string};

#[test]
fn renumber_positive_delta_shifts_up() {
    // 4 objects, connections 0→1 and 1→2.  Shift from index 1 by +1.
    // Objects: f(0), +1(1), *2(2), print(3).  After: 0→2, 2→3.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 + 1;\n\
                 #X obj 50 150 * 2;\n\
                 #X obj 50 200 print;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 1 0 2 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "renumber",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--from",
        "1",
        "--delta",
        "1",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // src=0 stays; dst=1 >= from → 2.  Second: src=1→2, dst=2→3.
    assert!(result.contains("#X connect 0 0 2 0;"));
    assert!(result.contains("#X connect 2 0 3 0;"));
}

#[test]
fn renumber_negative_delta_shifts_down() {
    // 4 objects, connection 2→3.  Shift from index 2 by -1.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 + 1;\n\
                 #X obj 50 150 * 2;\n\
                 #X obj 50 200 print;\n\
                 #X connect 2 0 3 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "renumber",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--from",
        "2",
        "--delta",
        "-1",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // 2→3 becomes 1→2
    assert!(result.contains("#X connect 1 0 2 0;"));
}

#[test]
fn renumber_only_affects_correct_depth() {
    // Subpatch at depth 1 has 3 objects with connections 0→1, 1→2.
    // Shift depth 1 from index 1 by +1: becomes 0→2, 2→3 which is INVALID
    // (only 3 objects at depth 1). So use shift by 0 at depth 1, and verify
    // depth 0 is unaffected.
    // Actually, let's use a different approach: 4 objects at depth 1, shift +1 from 2.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 inlet;\n\
                 #N canvas 0 0 450 300 sub 0;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 + 1;\n\
                 #X obj 50 150 * 2;\n\
                 #X obj 50 200 outlet;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 2 0 3 0;\n\
                 #X restore 50 100 pd sub;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 1 0 2 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    // Shift depth 1 from index 2 by -1.  Connection 2→3 becomes 1→2.
    pdtk_output(&[
        "renumber",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--from",
        "2",
        "--delta",
        "-1",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // Depth 1: 0→1 unchanged (both < from=2), 2→3 becomes 1→2
    // Depth 0 connections: should be unchanged
    let lines: Vec<&str> = result.lines().collect();

    // Depth 0 connections are after the restore line
    let restore_idx = lines.iter().position(|l| l.contains("restore")).unwrap();
    let depth0_conns: Vec<&&str> = lines[restore_idx..]
        .iter()
        .filter(|l| l.starts_with("#X connect"))
        .collect();
    assert!(depth0_conns.iter().any(|l| l.contains("0 0 1 0")));
    assert!(depth0_conns.iter().any(|l| l.contains("1 0 2 0")));
}

#[test]
fn renumber_creates_invalid_patch_caught_by_validate() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&[
        "renumber",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--from",
        "0",
        "--delta",
        "100",
    ]);
    // Out-of-range indices → validation failure → exit 2
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn renumber_output_flag_writes_to_file() {
    // 4 objects: shift from 2 by -1 (valid)
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 + 1;\n\
                 #X obj 50 150 * 2;\n\
                 #X obj 50 200 print;\n\
                 #X connect 2 0 3 0;\n";
    let tmp_in = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp_in.path(), input).unwrap();
    let tmp_out = tempfile::NamedTempFile::new().unwrap();

    let out = run_pdtk(&[
        "renumber",
        tmp_in.path().to_str().unwrap(),
        "--depth",
        "0",
        "--from",
        "2",
        "--delta",
        "-1",
        "--output",
        tmp_out.path().to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));

    let result = std::fs::read_to_string(tmp_out.path()).unwrap();
    assert!(result.contains("#X connect 1 0 2 0;"));
    assert!(stdout_string(&out).trim().is_empty());
}

#[test]
fn renumber_backup_creates_bak_file() {
    // Valid shift: 3 objects, connection 0→1, shift from 2 by -1 (no connections affected → still valid)
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 + 1;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();
    let backup_path = format!("{}.bak", tmp.path().display());

    pdtk_output(&[
        "renumber",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--from",
        "2",
        "--delta",
        "-1",
        "--in-place",
        "--backup",
    ]);

    assert!(std::path::Path::new(&backup_path).exists());
    let backup_content = std::fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, input);
    std::fs::remove_file(&backup_path).ok();
}
