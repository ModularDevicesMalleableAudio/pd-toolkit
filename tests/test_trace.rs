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
    assert!(!v["hops"].as_array().unwrap().is_empty());
}

#[test]
fn trace_default_does_not_follow_bus() {
    let f = handcrafted("trace_send_receive_hop.pd");
    // Without --show-bus-hops, trace from loadbang (0) should reach [s foo] (1)
    // but NOT [r foo] (2) or downstream print (3).
    let out = pdtk_output(&["trace", f.to_str().unwrap(), "--from", "0"]);
    assert!(out.contains("index:1"), "should reach [s foo]: {out}");
    assert!(!out.contains("index:2"), "must not reach [r foo]: {out}");
    assert!(!out.contains("index:3"), "must not reach downstream: {out}");
}

#[test]
fn trace_show_bus_hops_follows_control_bus() {
    let f = handcrafted("trace_send_receive_hop.pd");
    let out = pdtk_output(&[
        "trace",
        f.to_str().unwrap(),
        "--from",
        "0",
        "--show-bus-hops",
    ]);
    assert!(out.contains("index:1"), "should reach [s foo]: {out}");
    assert!(out.contains("index:2"), "should reach [r foo]: {out}");
    assert!(out.contains("index:3"), "should reach downstream: {out}");
    assert!(
        out.contains("bus \"foo\""),
        "should mention bus name: {out}"
    );
    assert!(out.contains("(control)"), "should mention bus kind: {out}");
}

#[test]
fn trace_show_bus_hops_signal_namespace() {
    let f = handcrafted("trace_send_receive_hop.pd");
    // osc~ (4) → s~ audio (5) → r~ audio (6) → dac~ (7)
    let out = pdtk_output(&[
        "trace",
        f.to_str().unwrap(),
        "--from",
        "4",
        "--show-bus-hops",
    ]);
    assert!(out.contains("index:5"));
    assert!(out.contains("index:6"));
    assert!(out.contains("index:7"));
    assert!(out.contains("(signal)"), "should mark signal bus: {out}");
}

#[test]
fn trace_dollar_zero_warning_in_output() {
    let f = handcrafted("trace_send_receive_hop.pd");
    let out = pdtk_output(&[
        "trace",
        f.to_str().unwrap(),
        "--from",
        "8",
        "--show-bus-hops",
    ]);
    assert!(
        out.contains("dollar-zero-scoped"),
        "should flag $0 scope warning: {out}"
    );
}

#[test]
fn trace_json_default_has_hop_kind_wire() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["trace", f.to_str().unwrap(), "--from", "0", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let hops = v["hops"].as_array().expect("hops array");
    assert!(!hops.is_empty());
    for h in hops {
        assert_eq!(h["hop_kind"], "wire", "default hops must be hop_kind=wire");
    }
}

#[test]
fn trace_json_bus_hop_carries_bus_metadata() {
    let f = handcrafted("trace_send_receive_hop.pd");
    let out = pdtk_output(&[
        "trace",
        f.to_str().unwrap(),
        "--from",
        "0",
        "--show-bus-hops",
        "--json",
    ]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let hops = v["hops"].as_array().expect("hops array");
    let bus_hop = hops
        .iter()
        .find(|h| h["hop_kind"] == "bus")
        .expect("expected a bus hop");
    assert_eq!(bus_hop["bus_name"], "foo");
    assert_eq!(bus_hop["bus_kind"], "control");
}

#[test]
fn trace_namespace_isolation_no_cross_link() {
    // [s foo] (control) is at idx 1; [r~ audio] is at idx 6. They share
    // no bus name, but verify the trace from a control-send doesn't reach
    // an unrelated signal-receive even with --show-bus-hops.
    let f = handcrafted("trace_send_receive_hop.pd");
    let out = pdtk_output(&[
        "trace",
        f.to_str().unwrap(),
        "--from",
        "1",
        "--show-bus-hops",
    ]);
    // From [s foo] we should bus-hop to [r foo] (2) then wire to print (3),
    // but not into the signal-bus chain.
    assert!(out.contains("index:2"));
    assert!(out.contains("index:3"));
    assert!(
        !out.contains("index:6"),
        "must not cross to signal bus: {out}"
    );
    assert!(!out.contains("index:7"), "must not cross to dac~: {out}");
}
