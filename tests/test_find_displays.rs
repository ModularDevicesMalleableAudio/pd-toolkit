mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk};

#[test]
fn displays_finds_connected_floatatom() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&["find-displays", f.to_str().unwrap()]);
    assert!(out.contains("floatatom"));
}

#[test]
fn displays_finds_connected_symbolatom() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&[
        "find-displays",
        f.to_str().unwrap(),
        "--include-unconnected",
    ]);
    assert!(out.contains("symbolatom"));
}

#[test]
fn displays_finds_connected_nbx() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&["find-displays", f.to_str().unwrap()]);
    assert!(out.contains(" nbx "));
}

#[test]
fn displays_does_not_report_unconnected_by_default() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&["find-displays", f.to_str().unwrap()]);
    assert!(!out.contains("symbolatom"));
}

#[test]
fn displays_include_unconnected_flag() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&[
        "find-displays",
        f.to_str().unwrap(),
        "--include-unconnected",
    ]);
    assert!(out.contains("symbolatom"));
}

#[test]
fn displays_json_output_schema() {
    let f = handcrafted("displays.pd");
    let out = pdtk_output(&["find-displays", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let row = &v.as_array().unwrap()[0];
    assert!(row.get("file").is_some());
    assert!(row.get("connected").is_some());
}

#[test]
fn displays_delete_removes_and_renumbers() {
    let src = std::fs::read_to_string(handcrafted("displays.pd")).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), src).unwrap();

    pdtk_output(&[
        "find-displays",
        tmp.path().to_str().unwrap(),
        "--delete",
        "--in-place",
    ]);

    let out = pdtk_output(&["list", tmp.path().to_str().unwrap()]);
    assert!(!out.contains("floatatom"));
    assert!(!out.contains("nbx"));
}

#[test]
fn displays_delete_validates_before_write() {
    let src = std::fs::read_to_string(handcrafted("displays.pd")).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), src).unwrap();

    pdtk_output(&[
        "find-displays",
        tmp.path().to_str().unwrap(),
        "--delete",
        "--in-place",
    ]);

    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}

#[test]
fn displays_finds_listbox() {
    let f = handcrafted("listbox_send_receive.pd");
    let out = pdtk_output(&[
        "find-displays",
        f.to_str().unwrap(),
        "--include-unconnected",
    ]);
    assert!(out.contains("listbox"), "output was: {out}");
}

#[test]
fn displays_delete_targets_correct_sibling_canvas() {
    // Only sub_b's floatatom is connected (a display). sub_a's floatatom at
    // the same depth/index is unconnected; depth-merged detection would flag
    // it too, and a depth-scoped delete removes sub_a's objects instead.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #N canvas 0 22 200 200 sub_a 0;\n\
                 #X floatatom 30 30 5 0 0 0 keepme - -;\n\
                 #X obj 30 60 osc~ 440;\n\
                 #X restore 50 50 pd sub_a;\n\
                 #N canvas 0 22 200 200 sub_b 0;\n\
                 #X floatatom 30 30 5 0 0 0 delme - -;\n\
                 #X obj 30 60 f;\n\
                 #X connect 1 0 0 0;\n\
                 #X restore 50 100 pd sub_b;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    run_pdtk(&[
        "find-displays",
        tmp.path().to_str().unwrap(),
        "--delete",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        !result.contains("delme"),
        "sub_b's connected display must be deleted:\n{result}"
    );
    assert!(
        result.contains("keepme"),
        "sub_a's unconnected floatatom must survive:\n{result}"
    );
    assert!(
        result.contains("osc~ 440"),
        "sub_a's osc~ must survive:\n{result}"
    );
}
