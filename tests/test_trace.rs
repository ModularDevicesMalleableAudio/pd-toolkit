mod integration;
use integration::{handcrafted, pdtk_output, run_pdtk};

#[test]
fn trace_forward_reaches_downstream_objects() {
    let f = handcrafted("simple_chain.pd");
    // loadbang(0) → t b(1) → print done(2)
    let out = pdtk_output(&["trace", f.to_str().unwrap(), "--from", "0"]);
    assert!(out.contains("index:1"));
    assert!(out.contains("index:2"));
}

#[test]
fn trace_forward_correct_hop_order() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["trace", f.to_str().unwrap(), "--from", "0"]);
    let hop1_pos = out.find("hop 1:").unwrap();
    let hop2_pos = out.find("hop 2:").unwrap();
    assert!(hop1_pos < hop2_pos);
    // hop 1 should reach t b (index 1)
    assert!(out[hop1_pos..hop2_pos].contains("index:1"));
}

#[test]
fn trace_path_between_connected_objects() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["trace", f.to_str().unwrap(), "--from", "0", "--to", "2"]);
    assert!(out.contains("index:0"));
    assert!(out.contains("index:1"));
    assert!(out.contains("index:2"));
}

#[test]
fn trace_path_no_path_returns_empty() {
    let f = handcrafted("simple_chain.pd");
    // print done (2) has no outgoing connections → no path 2→0
    let out = pdtk_output(&["trace", f.to_str().unwrap(), "--from", "2", "--to", "0"]);
    assert!(out.contains("no path"));
}

#[test]
fn trace_cycle_does_not_infinite_loop() {
    let f = handcrafted("cycle.pd");
    // Should complete without hanging
    let out = run_pdtk(&["trace", f.to_str().unwrap(), "--from", "0"]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn trace_max_hops_limits_depth() {
    let f = handcrafted("simple_chain.pd");
    // With max-hops 1, from 0 should only reach index 1, not index 2
    let out = pdtk_output(&[
        "trace",
        f.to_str().unwrap(),
        "--from",
        "0",
        "--max-hops",
        "1",
    ]);
    assert!(out.contains("index:1"));
    assert!(!out.contains("index:2"));
}

#[test]
fn trace_json_output_schema() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["trace", f.to_str().unwrap(), "--from", "0", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("mode").is_some());
    assert!(v.get("from").is_some());
    assert!(v.get("hops").is_some());
    assert!(v["hops"].as_array().unwrap().len() >= 1);
}
