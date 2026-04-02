mod integration;

use integration::{fixture_path, handcrafted, pdtk_output};

#[test]
fn stats_object_count_matches_list() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["stats", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["objects"], 3);
}

#[test]
fn stats_connection_count_matches_validate() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["stats", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["connections"], 2);
}

#[test]
fn stats_max_fanin_correct() {
    let f = handcrafted("merging.pd");
    let out = pdtk_output(&["stats", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["files"][0]["max_fanin"].as_u64().unwrap() >= 2);
}

#[test]
fn stats_max_fanout_correct() {
    let f = handcrafted("branching.pd");
    let out = pdtk_output(&["stats", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["files"][0]["max_fanout"].as_u64().unwrap() >= 2);
}

#[test]
fn stats_class_histogram_tallies_correctly() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["stats", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["class_histogram"]["loadbang"], 1);
}

#[test]
fn stats_json_output_schema() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["stats", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("files").is_some());
    assert!(v.get("total_files").is_some());
}

#[test]
fn stats_directory_aggregate() {
    let dir = fixture_path("handcrafted");
    let out = pdtk_output(&["stats", dir.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v["total_files"].as_u64().unwrap() > 1);
}

#[test]
fn stats_zero_object_patch() {
    let f = handcrafted("minimal.pd");
    let out = pdtk_output(&["stats", f.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["files"][0]["objects"], 0);
}
