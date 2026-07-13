mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk, stderr_string, stdout_string};

/// Copy the sibling fixture to a temp file so in-place edits don't touch the
/// shared fixture.
fn temp_sibling() -> tempfile::NamedTempFile {
    let src = std::fs::read_to_string(handcrafted("sibling_subpatches.pd")).unwrap();
    let tmp = tempfile::Builder::new().suffix(".pd").tempfile().unwrap();
    std::fs::write(tmp.path(), src).unwrap();
    tmp
}

#[test]
fn modify_canvas_0_targets_first_sibling() {
    let tmp = temp_sibling();
    pdtk_output(&[
        "modify",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--canvas",
        "0",
        "--index",
        "1",
        "--text",
        "osc~ 999",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        out.contains("osc~ 999"),
        "sub_a osc~ should change; got:\n{out}"
    );
    assert!(
        out.contains("phasor~ 220"),
        "sub_b must be untouched; got:\n{out}"
    );
}

#[test]
fn modify_canvas_1_targets_second_sibling() {
    let tmp = temp_sibling();
    pdtk_output(&[
        "modify",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--canvas",
        "1",
        "--index",
        "1",
        "--text",
        "phasor~ 111",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        out.contains("phasor~ 111"),
        "sub_b phasor~ should change; got:\n{out}"
    );
    assert!(
        out.contains("osc~ 440"),
        "sub_a must be untouched; got:\n{out}"
    );
}

#[test]
fn modify_defaults_to_canvas_0() {
    let tmp = temp_sibling();
    // No --canvas given: must behave as canvas 0 (first sibling).
    pdtk_output(&[
        "modify",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--index",
        "1",
        "--text",
        "osc~ 777",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(out.contains("osc~ 777"), "got:\n{out}");
    assert!(out.contains("phasor~ 220"), "got:\n{out}");
}

#[test]
fn modify_canvas_out_of_range_errors() {
    let tmp = temp_sibling();
    let out = run_pdtk(&[
        "modify",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--canvas",
        "5",
        "--index",
        "1",
        "--text",
        "osc~ 1",
        "--in-place",
    ]);
    assert_ne!(out.status.code(), Some(0));
    assert!(
        stderr_string(&out).to_lowercase().contains("canvas"),
        "expected a canvas-out-of-range error; got:\n{}",
        stderr_string(&out)
    );
}

#[test]
fn connect_scopes_to_selected_canvas() {
    let tmp = temp_sibling();
    // Add inlet(0) -> outlet~(2) inside sub_b only.
    pdtk_output(&[
        "connect",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--canvas",
        "1",
        "--src",
        "0",
        "--outlet",
        "0",
        "--dst",
        "2",
        "--inlet",
        "0",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    // The new connection must land inside sub_b, i.e. after phasor~.
    let sub_b_start = out.find("phasor~").expect("sub_b present");
    let tail = &out[sub_b_start..];
    assert!(
        tail.contains("#X connect 0 0 2 0;"),
        "new connection should be inside sub_b; got:\n{out}"
    );
    // sub_a should still have exactly its original two connections (no 0->2).
    let sub_a = &out[out.find("osc~").unwrap()..sub_b_start];
    assert!(
        !sub_a.contains("#X connect 0 0 2 0;"),
        "sub_a must not gain a connection; got:\n{out}"
    );
}

#[test]
fn delete_scopes_to_selected_canvas() {
    let tmp = temp_sibling();
    // Delete phasor~ (index 1) in sub_b.
    pdtk_output(&[
        "delete",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--canvas",
        "1",
        "--index",
        "1",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        !out.contains("phasor~"),
        "phasor~ should be gone; got:\n{out}"
    );
    assert!(
        out.contains("osc~ 440"),
        "sub_a osc~ must remain; got:\n{out}"
    );
}

#[test]
fn insert_scopes_to_selected_canvas() {
    let tmp = temp_sibling();
    // Insert a new object at index 1 in sub_b.
    pdtk_output(&[
        "insert",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--canvas",
        "1",
        "--index",
        "1",
        "--entry",
        "#X obj 30 45 lop~ 100;",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    let restore_a = out.find("pd sub_a").expect("sub_a restore present");
    let lop_pos = out.find("lop~ 100").expect("lop~ inserted");
    assert!(
        lop_pos > restore_a,
        "lop~ must be inside sub_b (after sub_a's restore); got:\n{out}"
    );
    assert!(
        !out[..restore_a].contains("lop~"),
        "sub_a must not gain the object; got:\n{out}"
    );
    // sub_b's own connections were renumbered (inlet 0 -> phasor~ now at 2).
    assert!(out.contains("#X connect 0 0 2 0;"), "got:\n{out}");
    // Validate still passes after the scoped insert + renumber.
    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0), "stdout:\n{}", stdout_string(&v));
}

#[test]
fn connections_scopes_to_selected_canvas() {
    let f = handcrafted("sibling_subpatches.pd");
    // phasor~ is index 1 in sub_b (canvas 1). Its inlet comes from inlet(0),
    // its outlet feeds outlet~(2).
    let out = pdtk_output(&[
        "connections",
        f.to_str().unwrap(),
        "--depth",
        "1",
        "--canvas",
        "1",
        "--index",
        "1",
    ]);
    // Scoped to sub_b, phasor~(1) has exactly one inlet (from inlet 0) and one
    // outlet (to outlet~ 2). Without canvas scoping the merged depth-1 view
    // would double these (sub_a and sub_b have identical wiring).
    assert_eq!(
        out.matches('\u{2190}').count(),
        1,
        "one inlet expected; got:\n{out}"
    );
    assert_eq!(
        out.matches('\u{2192}').count(),
        1,
        "one outlet expected; got:\n{out}"
    );
}

#[test]
fn extract_index_selects_nth_sibling() {
    let tmp = temp_sibling();
    let abs = tempfile::Builder::new().suffix(".pd").tempfile().unwrap();
    // Extract the SECOND subpatch (sub_b, index 1).
    pdtk_output(&[
        "extract",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--index",
        "1",
        "--output",
        abs.path().to_str().unwrap(),
    ]);
    let extracted = std::fs::read_to_string(abs.path()).unwrap();
    assert!(
        extracted.contains("phasor~ 220"),
        "extracted abstraction should be sub_b; got:\n{extracted}"
    );
    assert!(
        !extracted.contains("osc~ 440"),
        "must not extract sub_a; got:\n{extracted}"
    );
}

#[test]
fn extract_boundary_connections_are_scoped_to_parent_canvas() {
    // The inner subpatch in outer_b has no parent connections. A same-depth
    // sibling parent (outer_a) does have a connection into its own inner
    // restore; extract must not use outer_a's connection to add inlets to
    // outer_b's extracted abstraction.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #N canvas 0 22 250 250 outer_a 0;\n\
                 #N canvas 0 22 150 150 inner_a 0;\n\
                 #X obj 20 20 print a;\n\
                 #X restore 30 30 pd inner_a;\n\
                 #X obj 30 80 loadbang;\n\
                 #X connect 1 0 0 3;\n\
                 #X restore 50 50 pd outer_a;\n\
                 #N canvas 0 22 250 250 outer_b 0;\n\
                 #N canvas 0 22 150 150 inner_b 0;\n\
                 #X obj 20 20 print b;\n\
                 #X restore 30 30 pd inner_b;\n\
                 #X restore 120 50 pd outer_b;\n";
    let tmp = tempfile::Builder::new().suffix(".pd").tempfile().unwrap();
    std::fs::write(tmp.path(), input).unwrap();
    let abs = tempfile::Builder::new().suffix(".pd").tempfile().unwrap();

    pdtk_output(&[
        "extract",
        tmp.path().to_str().unwrap(),
        "--depth",
        "2",
        "--index",
        "1",
        "--output",
        abs.path().to_str().unwrap(),
    ]);

    let extracted = std::fs::read_to_string(abs.path()).unwrap();
    assert!(
        extracted.contains("print b"),
        "must extract inner_b; got:\n{extracted}"
    );
    assert!(
        !extracted.lines().any(|line| line.contains(" inlet;")),
        "sibling parent connections must not create extracted inlets:\n{extracted}"
    );
}

#[test]
fn list_json_includes_canvas_ordinal() {
    let f = handcrafted("sibling_subpatches.pd");
    let out = pdtk_output(&["list", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let rows = v.as_array().expect("list --json is an array");
    // Every depth-1 row must carry a `canvas` ordinal; sub_a rows = 0, sub_b = 1.
    let depth1: Vec<&serde_json::Value> = rows.iter().filter(|r| r["depth"] == 1).collect();
    assert!(!depth1.is_empty(), "expected depth-1 rows; got:\n{out}");
    assert!(
        depth1.iter().all(|r| r.get("canvas").is_some()),
        "each depth-1 row needs a canvas ordinal; got:\n{out}"
    );
    let canvases: std::collections::HashSet<i64> =
        depth1.iter().filter_map(|r| r["canvas"].as_i64()).collect();
    assert!(
        canvases.contains(&0) && canvases.contains(&1),
        "expected canvas ordinals 0 and 1; got:\n{out}"
    );
}
