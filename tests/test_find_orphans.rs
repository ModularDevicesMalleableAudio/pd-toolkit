mod integration;

use integration::{fixture_path, handcrafted, pdtk_output, run_pdtk};

#[test]
fn orphans_finds_unconnected_objects() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap()]);
    assert!(out.contains("osc~ 440"));
    assert!(out.contains("f 42"));
}

#[test]
fn orphans_excludes_arrays_and_scalars() {
    // An `#X array` is a gobj but is referenced by name (tabread/tabwrite),
    // not by patch cords, so it must not be reported as an orphan even though
    // no wire connects to it.
    let f = handcrafted("array_in_canvas.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap()]);
    // array(0) is unconnected but excluded by kind; metro(1)->tabwrite(2) are
    // both connected once the array is correctly counted, so nothing is orphan.
    assert!(
        out.contains("No orphan objects found"),
        "arrays must be excluded and connection indices correct; got:\n{out}"
    );
    assert!(
        !out.contains("data_arr"),
        "array must not be reported as an orphan; got:\n{out}"
    );
}

#[test]
fn orphans_excludes_text_by_default() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap()]);
    assert!(!out.contains("this is a comment"));
}

#[test]
fn orphans_include_comments_flag_shows_text() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap(), "--include-comments"]);
    assert!(out.contains("this is a comment"));
}

#[test]
fn orphans_depth_filter() {
    let f = handcrafted("nested_subpatch.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap(), "--depth", "0"]);
    // nested_subpatch has no obvious orphans at depth 0
    assert!(out.contains("No orphan") || !out.contains("[depth:1"));
}

#[test]
fn orphans_json_output_schema() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.is_array());
    let row = &v.as_array().unwrap()[0];
    assert!(row.get("file").is_some());
    assert!(row.get("depth").is_some());
    assert!(row.get("index").is_some());
    assert!(row.get("text").is_some());
}

#[test]
fn orphans_connected_loadbang_not_orphan() {
    let f = handcrafted("orphans.pd");
    let out = pdtk_output(&["find-orphans", f.to_str().unwrap()]);
    assert!(!out.contains("loadbang"));
}

#[test]
fn orphans_delete_removes_and_renumbers() {
    let src = std::fs::read_to_string(handcrafted("orphans.pd")).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), src).unwrap();

    pdtk_output(&[
        "find-orphans",
        tmp.path().to_str().unwrap(),
        "--delete",
        "--in-place",
    ]);

    let out = pdtk_output(&["list", tmp.path().to_str().unwrap()]);
    assert!(!out.contains("osc~ 440"));
    assert!(!out.contains("f 42"));
}

#[test]
fn orphans_delete_validates_before_write() {
    let src = std::fs::read_to_string(handcrafted("orphans.pd")).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), src).unwrap();

    pdtk_output(&[
        "find-orphans",
        tmp.path().to_str().unwrap(),
        "--delete",
        "--in-place",
    ]);

    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}

#[test]
fn orphans_directory_mode_scans_recursively() {
    let dir = fixture_path("handcrafted");
    let out = pdtk_output(&["find-orphans", dir.to_str().unwrap()]);
    assert!(out.contains("orphans.pd"));
}

#[test]
fn orphans_are_detected_per_sibling_canvas() {
    // sub_b's phasor~ is index 1 and has no connections. sub_a also has an
    // index 1 object that is connected; depth-merged orphan detection would
    // incorrectly treat sub_b's index 1 as connected too.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #N canvas 0 22 200 200 sub_a 0;\n\
                 #X obj 30 30 inlet;\n\
                 #X obj 30 60 osc~ 440;\n\
                 #X obj 30 90 outlet~;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 1 0 2 0;\n\
                 #X restore 50 50 pd sub_a;\n\
                 #N canvas 0 22 200 200 sub_b 0;\n\
                 #X obj 30 30 inlet;\n\
                 #X obj 30 60 phasor~ 220;\n\
                 #X obj 30 90 outlet~;\n\
                 #X connect 0 0 2 0;\n\
                 #X restore 50 100 pd sub_b;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = pdtk_output(&["find-orphans", tmp.path().to_str().unwrap(), "--depth", "1"]);
    assert!(
        out.contains("phasor~ 220"),
        "sub_b's unconnected object must be reported despite sub_a's same-index connections:\n{out}"
    );
}

#[test]
fn orphans_delete_targets_correct_sibling_canvas() {
    // The orphan is sub_b's phasor~ (index 1 in its own canvas). sub_a has a
    // connected osc~ at the same depth/index; a depth-scoped delete would
    // remove the osc~ instead.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #N canvas 0 22 200 200 sub_a 0;\n\
                 #X obj 30 30 inlet;\n\
                 #X obj 30 60 osc~ 440;\n\
                 #X obj 30 90 outlet~;\n\
                 #X connect 0 0 1 0;\n\
                 #X connect 1 0 2 0;\n\
                 #X restore 50 50 pd sub_a;\n\
                 #N canvas 0 22 200 200 sub_b 0;\n\
                 #X obj 30 30 inlet;\n\
                 #X obj 30 60 phasor~ 220;\n\
                 #X obj 30 90 outlet~;\n\
                 #X connect 0 0 2 0;\n\
                 #X restore 50 100 pd sub_b;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    run_pdtk(&[
        "find-orphans",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--delete",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        !result.contains("phasor~ 220"),
        "the orphan in sub_b must be deleted:\n{result}"
    );
    assert!(
        result.contains("osc~ 440"),
        "sub_a's connected osc~ must survive:\n{result}"
    );
}
