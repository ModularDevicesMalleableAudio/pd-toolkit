mod integration;

use integration::{handcrafted, pdtk_output, run_pdtk, stderr_string, stdout_string};
use serde_json::Value;

fn fixture() -> String {
    handcrafted("array_saved_data.pd")
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn dumps_saved_values_one_per_line() {
    let out = pdtk_output(&["array-data", &fixture(), "saved_notes"]);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 8);
    assert_eq!(lines[0], "0 0");
    assert_eq!(lines[1], "1 2");
    assert_eq!(lines[7], "7 12");
}

#[test]
fn joins_chunked_records() {
    let out = pdtk_output(&["array-data", &fixture(), "chunked"]);
    let values: Vec<&str> = out.lines().map(|l| l.split(' ').nth(1).unwrap()).collect();
    assert_eq!(values, vec!["1", "2", "3", "4", "5", "6"]);
}

#[test]
fn reads_classic_array_contents() {
    let out = pdtk_output(&["array-data", &fixture(), "classic_saved"]);
    assert_eq!(out.lines().next().unwrap(), "0 10");
    assert_eq!(out.lines().last().unwrap(), "3 40");
}

#[test]
fn json_output_carries_declaration_and_values() {
    let out = pdtk_output(&["array-data", &fixture(), "saved_notes", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["name"], "saved_notes");
    assert_eq!(v["kind"], "define");
    assert_eq!(v["size"], 8);
    assert_eq!(v["data"]["saved"], true);
    assert_eq!(v["data"]["records"], 1);
    assert_eq!(
        v["data"]["values"],
        serde_json::json!([0, 2, 4, 5, 7, 9, 11, 12])
    );
}

#[test]
fn unsaved_array_is_an_error_not_empty_output() {
    let out = run_pdtk(&["array-data", &fixture(), "unsaved"]);
    assert!(!out.status.success());
    assert!(stdout_string(&out).trim().is_empty());
    assert!(stderr_string(&out).contains("saves no contents"));
}

#[test]
fn unknown_array_name_is_an_error() {
    let out = run_pdtk(&["array-data", &fixture(), "not_here"]);
    assert!(!out.status.success());
    assert!(stderr_string(&out).contains("no array named `not_here`"));
}

#[test]
fn ambiguous_name_across_files_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    for (file, value) in [("a.pd", 1), ("b.pd", 2)] {
        std::fs::write(
            dir.path().join(file),
            format!(
                "#N canvas 0 22 450 300 12;\n#X obj 20 20 array define -k dup 2;\n#A 0 {value} {value};\n"
            ),
        )
        .unwrap();
    }
    let out = run_pdtk(&["array-data", dir.path().to_str().unwrap(), "dup"]);
    assert!(!out.status.success());
    let err = stderr_string(&out);
    assert!(err.contains("ambiguous"), "unexpected stderr: {err}");
    assert!(err.contains("a.pd") && err.contains("b.pd"));
}

#[test]
fn arrays_data_flag_includes_contents() {
    let out = pdtk_output(&["arrays", &fixture(), "--kind", "all", "--data"]);
    assert!(out.contains("data: 0 2 4 5 7 9 11 12"));
    assert!(out.contains("data: 1 2 3 4 5 6"));
    assert!(out.contains("data: none saved"));
}

#[test]
fn arrays_data_flag_json() {
    let out = pdtk_output(&["arrays", &fixture(), "--kind", "all", "--data", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let rows = v["arrays"].as_array().unwrap();
    let chunked = rows.iter().find(|r| r["name"] == "chunked").unwrap();
    assert_eq!(chunked["data"]["records"], 2);
    assert_eq!(
        chunked["data"]["values"],
        serde_json::json!([1, 2, 3, 4, 5, 6])
    );
    let unsaved = rows.iter().find(|r| r["name"] == "unsaved").unwrap();
    assert_eq!(unsaved["data"]["saved"], false);
}

#[test]
fn arrays_without_data_flag_omits_contents() {
    let out = pdtk_output(&["arrays", &fixture(), "--kind", "all", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    for row in v["arrays"].as_array().unwrap() {
        assert!(row.get("data").is_none());
    }
}

#[test]
fn data_is_rejected_for_the_frozen_v1_schema() {
    let out = run_pdtk(&["arrays", &fixture(), "--data", "--schema", "1"]);
    assert!(!out.status.success());
    assert!(stderr_string(&out).contains("--data is not available with --schema 1"));
}

#[test]
fn values_beyond_the_declared_size_are_discarded() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("overflow.pd");
    std::fs::write(
        &f,
        "#N canvas 0 22 450 300 12;\n#X obj 20 20 array define -k small 2;\n#A 0 1 2 3 4;\n",
    )
    .unwrap();
    let out = pdtk_output(&["array-data", f.to_str().unwrap(), "small", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"]["values"], serde_json::json!([1, 2]));
    assert_eq!(v["data"]["overflow"], 2);
}

#[test]
fn fractional_values_survive_the_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("floats.pd");
    std::fs::write(
        &f,
        "#N canvas 0 22 450 300 12;\n#X obj 20 20 array define -k floats 3;\n#A 0 0.5 -1.25 2;\n",
    )
    .unwrap();
    let out = pdtk_output(&["array-data", f.to_str().unwrap(), "floats"]);
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        vec!["0 0.5", "1 -1.25", "2 2"]
    );
}
