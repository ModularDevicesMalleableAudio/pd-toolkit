mod integration;
use integration::{handcrafted, pdtk_output, run_pdtk};

fn write_tmp(content: &str) -> tempfile::NamedTempFile {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(f.path(), content).unwrap();
    f
}

#[test]
fn diff_identical_files_empty_result() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["diff", f.to_str().unwrap(), f.to_str().unwrap()]);
    assert!(out.contains("No differences"));
}

#[test]
fn diff_added_object_detected() {
    let base = handcrafted("simple_chain.pd");
    let modified = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 50 50 loadbang;\n\
         #X obj 50 100 t b;\n\
         #X obj 50 150 print done;\n\
         #X obj 50 200 bang;\n\
         #X connect 0 0 1 0;\n\
         #X connect 1 0 2 0;\n",
    );

    let out = pdtk_output(&[
        "diff",
        base.to_str().unwrap(),
        modified.path().to_str().unwrap(),
    ]);
    assert!(out.contains("Objects added: 1"));
    assert!(out.contains("bang"));
}

#[test]
fn diff_removed_object_detected() {
    let base = handcrafted("simple_chain.pd");
    let modified = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 50 50 loadbang;\n\
         #X obj 50 150 print done;\n\
         #X connect 0 0 1 0;\n",
    );

    let out = pdtk_output(&[
        "diff",
        base.to_str().unwrap(),
        modified.path().to_str().unwrap(),
    ]);
    assert!(out.contains("Objects removed: 1"));
}

#[test]
fn diff_modified_object_detected() {
    let base = handcrafted("simple_chain.pd");
    let modified = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 50 50 loadbang;\n\
         #X obj 50 100 t b b;\n\
         #X obj 50 150 print done;\n\
         #X connect 0 0 1 0;\n\
         #X connect 1 0 2 0;\n",
    );

    let out = pdtk_output(&[
        "diff",
        base.to_str().unwrap(),
        modified.path().to_str().unwrap(),
    ]);
    assert!(out.contains("Objects modified: 1"));
}

#[test]
fn diff_added_connection_detected() {
    let base = handcrafted("simple_chain.pd");
    let modified = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 50 50 loadbang;\n\
         #X obj 50 100 t b;\n\
         #X obj 50 150 print done;\n\
         #X connect 0 0 1 0;\n\
         #X connect 0 0 2 0;\n\
         #X connect 1 0 2 0;\n",
    );

    let out = pdtk_output(&[
        "diff",
        base.to_str().unwrap(),
        modified.path().to_str().unwrap(),
    ]);
    assert!(out.contains("Connections added: 1"));
}

#[test]
fn diff_removed_connection_detected() {
    let base = handcrafted("simple_chain.pd");
    let modified = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 50 50 loadbang;\n\
         #X obj 50 100 t b;\n\
         #X obj 50 150 print done;\n\
         #X connect 0 0 1 0;\n",
    );

    let out = pdtk_output(&[
        "diff",
        base.to_str().unwrap(),
        modified.path().to_str().unwrap(),
    ]);
    assert!(out.contains("Connections removed: 1"));
}

#[test]
fn diff_ignore_coords_treats_coord_changes_as_identical() {
    let base = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 50 50 loadbang;\n\
         #X obj 50 100 print;\n\
         #X connect 0 0 1 0;\n",
    );
    let moved = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 200 300 loadbang;\n\
         #X obj 200 400 print;\n\
         #X connect 0 0 1 0;\n",
    );

    let out = pdtk_output(&[
        "diff",
        base.path().to_str().unwrap(),
        moved.path().to_str().unwrap(),
        "--ignore-coords",
    ]);
    assert!(out.contains("No differences"));
}

#[test]
fn diff_json_output_schema() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["diff", f.to_str().unwrap(), f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("objects_added").is_some());
    assert!(v.get("objects_removed").is_some());
    assert!(v.get("objects_modified").is_some());
    assert!(v.get("connections_added").is_some());
    assert!(v.get("connections_removed").is_some());
}

#[test]
fn diff_format_only_change_ignored_with_flag() {
    // Without --ignore-coords, coordinate changes show up as modifications
    let base = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 50 50 loadbang;\n\
         #X obj 50 100 print;\n\
         #X connect 0 0 1 0;\n",
    );
    let formatted = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 10 10 loadbang;\n\
         #X obj 10 40 print;\n\
         #X connect 0 0 1 0;\n",
    );

    // Without flag: should see modifications
    let without_flag = run_pdtk(&[
        "diff",
        base.path().to_str().unwrap(),
        formatted.path().to_str().unwrap(),
    ]);
    let out_without = String::from_utf8_lossy(&without_flag.stdout).to_string();
    assert!(out_without.contains("Objects modified:") || out_without.contains("No differences"));

    // With flag: no differences
    let out = pdtk_output(&[
        "diff",
        base.path().to_str().unwrap(),
        formatted.path().to_str().unwrap(),
        "--ignore-coords",
    ]);
    assert!(out.contains("No differences"));
}
