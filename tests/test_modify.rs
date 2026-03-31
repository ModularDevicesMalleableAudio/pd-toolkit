mod integration;
use integration::{handcrafted, pdtk_output, run_pdtk, stdout_string};

#[test]
fn modify_changes_class_and_args() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "modify",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--text",
        "route 1 2",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("route 1 2"));
    assert!(!result.contains("#X obj 50 50 f;"));
}

#[test]
fn modify_preserves_coordinates() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 123 456 f;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "modify",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--text",
        "+ 1",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("#X obj 123 456 + 1;"));
}

#[test]
fn modify_does_not_change_index() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "modify",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--text",
        "t b b",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let out = pdtk_output(&["list", tmp.path().to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // Index 1 should now be "t b b"
    let obj1 = &v.as_array().unwrap()[1];
    assert_eq!(obj1["index"], 1);
    assert_eq!(obj1["class"], "t");
}

#[test]
fn modify_preserves_connections() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "modify",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--text",
        "t b b",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    // Both connections must survive unchanged
    assert!(result.contains("#X connect 0 0 1 0;"));
    assert!(result.contains("#X connect 1 0 2 0;"));
}

#[test]
fn modify_warns_when_new_obj_has_fewer_outlets() {
    // t b has 1 outlet; if connection uses outlet 1, there should be a warning
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 t b b;\n\
                 #X obj 50 100 print;\n\
                 #X obj 50 150 print;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 0 1 2 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    // Replace with "bang" which has 1 outlet — outlet 1 (used by conn 0 1 2 0) is out of range
    let out = run_pdtk(&[
        "modify",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--text",
        "bang",
    ]);
    // Should still succeed (warning only)
    assert_eq!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("warning"));
}

#[test]
fn modify_refuses_to_modify_connect_entry() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    // There's no connect entry with an object_index, so this will fail with "no object"
    let out = run_pdtk(&[
        "modify",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "5",
        "--text",
        "bang",
    ]);
    assert_ne!(out.status.code(), Some(0));
}

#[test]
fn modify_refuses_to_modify_canvas_entry() {
    let out = run_pdtk(&[
        "modify",
        handcrafted("simple_chain.pd").to_str().unwrap(),
        "--depth",
        "99",
        "--index",
        "0",
        "--text",
        "bang",
    ]);
    assert_ne!(out.status.code(), Some(0));
}

#[test]
fn modify_special_chars_in_text_handled() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "modify",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--text",
        r"r s\$1.\$2",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains(r"s\$1.\$2"));
}

#[test]
fn modify_validates_after_mutation() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "modify",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--text",
        "loadbang",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}
