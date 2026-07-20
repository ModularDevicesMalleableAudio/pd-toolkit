mod integration;
use integration::{handcrafted, pdtk_output, run_pdtk, stdout_string};

#[test]
fn structs_lists_templates_with_typed_fields() {
    let f = handcrafted("multi_struct_before_canvas.pd");
    let out = pdtk_output(&["structs", f.to_str().unwrap()]);
    assert!(out.contains("templates (2)"), "got:\n{out}");
    assert!(
        out.contains("array_holder: float x, float y, array data (element)"),
        "got:\n{out}"
    );
    assert!(out.contains("element: float val"), "got:\n{out}");
}

#[test]
fn structs_lists_scalars_with_template_and_value_count() {
    let f = handcrafted("struct_before_canvas.pd");
    let out = pdtk_output(&["structs", f.to_str().unwrap()]);
    assert!(out.contains("point: float x, float y"), "got:\n{out}");
    assert!(out.contains("[0:1] point (2 values)"), "got:\n{out}");
}

#[test]
fn structs_flags_scalar_with_undefined_template() {
    let input = "#N canvas 0 22 450 300 12;\n#X scalar ghost 1 2;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();
    let out = pdtk_output(&["structs", tmp.path().to_str().unwrap()]);
    assert!(out.contains("ghost"), "got:\n{out}");
    assert!(out.contains("undefined template"), "got:\n{out}");
}

#[test]
fn structs_json_schema() {
    let f = handcrafted("struct_before_canvas.pd");
    let out = pdtk_output(&["structs", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let file0 = &v.as_array().unwrap()[0];
    assert!(file0.get("file").is_some());
    let tmpl0 = &file0["templates"].as_array().unwrap()[0];
    assert_eq!(tmpl0["name"], "point");
    assert_eq!(tmpl0["scalar_fields"], serde_json::json!(2));
    assert_eq!(tmpl0["fields"].as_array().unwrap()[0]["type"], "float");
    let sc0 = &file0["scalars"].as_array().unwrap()[0];
    assert_eq!(sc0["template"], "point");
    assert_eq!(sc0["values"], serde_json::json!(2));
    assert_eq!(sc0["template_found"], serde_json::json!(true));
}

#[test]
fn structs_json_array_field_carries_subtemplate() {
    let f = handcrafted("multi_struct_before_canvas.pd");
    let out = pdtk_output(&["structs", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let fields = v.as_array().unwrap()[0]["templates"].as_array().unwrap()[0]["fields"]
        .as_array()
        .unwrap()
        .clone();
    let arr = fields
        .iter()
        .find(|f| f["type"] == "array")
        .expect("array field");
    assert_eq!(arr["name"], "data");
    assert_eq!(arr["array_template"], "element");
}

#[test]
fn structs_reports_none_for_plain_patch() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["structs", f.to_str().unwrap()]);
    assert!(
        out.contains("No data-structure templates or scalars found"),
        "got:\n{out}"
    );
}

#[test]
fn structs_directory_mode_scans_recursively() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.pd"),
        "#N struct pt float x float y;\n#N canvas 0 22 450 300 12;\n#X scalar pt 1 2;\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.pd"),
        "#N canvas 0 22 450 300 12;\n#X obj 10 10 print;\n",
    )
    .unwrap();
    let out = pdtk_output(&["structs", dir.path().to_str().unwrap()]);
    assert!(out.contains("pt: float x, float y"), "got:\n{out}");
    // b.pd has no data structures → not listed.
    assert!(
        !out.contains("b.pd"),
        "plain file should not appear; got:\n{out}"
    );
}

#[test]
fn structs_json_output_is_valid_for_empty_result() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["structs", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v.as_array().unwrap().len(), 0);
}

#[test]
fn structs_missing_file_errors() {
    let out = run_pdtk(&["structs", "/nonexistent/nope.pd"]);
    assert_ne!(out.status.code(), Some(0));
    let _ = stdout_string(&out);
}
