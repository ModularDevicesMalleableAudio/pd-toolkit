mod integration;
use integration::{handcrafted, pdtk_output};

#[test]
fn connections_lists_inlets_and_outlets() {
    let f = handcrafted("simple_chain.pd");
    // index 1 (t b): fed by loadbang (0), feeds print (2)
    let out = pdtk_output(&["connections", f.to_str().unwrap(), "--index", "1"]);
    assert!(out.contains("← [src:0 outlet:0]"));
    assert!(out.contains("→ [dst:2 inlet:0]"));
}

#[test]
fn connections_orphan_returns_empty() {
    let f = handcrafted("orphans.pd");
    // osc~ is at index 2, no connections
    let out = pdtk_output(&["connections", f.to_str().unwrap(), "--index", "2"]);
    assert!(out.contains("(none)"));
}

#[test]
fn connections_grouped_by_direction() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["connections", f.to_str().unwrap(), "--index", "1"]);
    let inlet_pos = out.find("Inlets:").unwrap();
    let outlet_pos = out.find("Outlets:").unwrap();
    assert!(
        inlet_pos < outlet_pos,
        "inlets section must come before outlets section"
    );
}

#[test]
fn connections_json_output_schema() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["connections", f.to_str().unwrap(), "--index", "1", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("inlets").is_some());
    assert!(v.get("outlets").is_some());
    assert_eq!(v["index"], 1);
    assert_eq!(v["depth"], 0);
}

#[test]
fn connections_shows_connected_object_text() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["connections", f.to_str().unwrap(), "--index", "1", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let inlet_src_text = v["inlets"][0]["src_text"].as_str().unwrap();
    assert!(inlet_src_text.contains("loadbang"));
    let outlet_dst_text = v["outlets"][0]["dst_text"].as_str().unwrap();
    assert!(outlet_dst_text.contains("print"));
}
