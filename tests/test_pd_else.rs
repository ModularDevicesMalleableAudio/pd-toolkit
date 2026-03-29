/// Integration tests using real-world pd-else patches.
///
/// Files sourced from https://github.com/porres/pd-else under WTFPL.
/// Copyright (C) 2017-2023 Alexandre Torres Porres and others.
/// See tests/fixtures/pd_else/ATTRIBUTION.md for full attribution.
mod integration;

use integration::{pdtk_output, run_pdtk, stdout_string};
use std::path::PathBuf;

fn pd_else(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pd_else")
        .join(name)
}

// Parse + validate all pd-else fixtures (regression gate)

#[test]
fn pd_else_all_fixtures_parse_without_error() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pd_else");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().map(|e| e == "pd").unwrap_or(false) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let out = run_pdtk(&["parse", path.to_str().unwrap()]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{name} must parse cleanly"
        );
    }
}

#[test]
fn pd_else_all_fixtures_validate_cleanly() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pd_else");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().map(|e| e == "pd").unwrap_or(false) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let out = run_pdtk(&["validate", path.to_str().unwrap()]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{name} must validate cleanly, stderr: {}",
            stdout_string(&out)
        );
    }
}

#[test]
fn pd_else_all_fixtures_round_trip_byte_identical() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pd_else");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().map(|e| e == "pd").unwrap_or(false) {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let original = std::fs::read_to_string(&path).unwrap();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        pdtk_output(&["parse", path.to_str().unwrap(), "--output", tmp.path().to_str().unwrap()]);
        let written = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(original, written, "{name}: round-trip must be byte-identical");
    }
}

// bpm.pd — simple flat patch with escaped chars (\; in text entries)

#[test]
fn bpm_correct_object_count() {
    let out = pdtk_output(&["parse", pd_else("bpm.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // bpm.pd has 16 objects and 16 connections
    assert_eq!(v["objects"], 16);
    assert_eq!(v["connections"], 16);
}

#[test]
fn bpm_is_flat_patch_max_depth_0() {
    let out = pdtk_output(&["parse", pd_else("bpm.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["max_depth"], 0);
}

// tremolo~.pd — flat signal-rate patch

#[test]
fn tremolo_flat_signal_chain() {
    let out = pdtk_output(&["parse", pd_else("tremolo~.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["max_depth"], 0);
    assert_eq!(v["objects"], 25);
    assert_eq!(v["connections"], 26);
}

#[test]
fn tremolo_list_shows_signal_objects() {
    let out = pdtk_output(&["list", pd_else("tremolo~.pd").to_str().unwrap()]);
    assert!(out.contains("*~"));
    assert!(out.contains("inlet~"));
    assert!(out.contains("outlet~"));
}

// euclid.pd — floatatom, 2 subpatches

#[test]
fn euclid_has_subpatch() {
    let out = pdtk_output(&["parse", pd_else("euclid.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["max_depth"], 1);
    assert_eq!(v["canvases"], 2);
}

#[test]
fn euclid_floatatom_listed() {
    let out = pdtk_output(&["list", pd_else("euclid.pd").to_str().unwrap()]);
    assert!(out.contains("floatatom"));
}

#[test]
fn euclid_no_orphaned_index_gaps() {
    // Validate confirms all connections reference valid indices
    let out = run_pdtk(&["validate", pd_else("euclid.pd").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}

// glide.pd — inline width hints, escaped $0, 2 subpatches, 52 connections

#[test]
fn glide_inline_width_hints_parsed_correctly() {
    // glide.pd has 42 objects (not 2 more from width hints being counted)
    let out = pdtk_output(&["parse", pd_else("glide.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["objects"], 42);
}

#[test]
fn glide_connections_intact() {
    let out = pdtk_output(&["parse", pd_else("glide.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["connections"], 52);
}

#[test]
fn glide_list_depth_1_has_objects() {
    let out = pdtk_output(&["list", pd_else("glide.pd").to_str().unwrap(), "--depth", "1"]);
    assert!(!out.is_empty(), "depth 1 must have objects");
}

// compress~.pd — route with inline width hint

#[test]
fn compress_route_width_hint_not_counted_as_object() {
    // If the ", f 12" in route was wrongly counted, object count would be off
    let out = pdtk_output(&["parse", pd_else("compress~.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["objects"], 42);
    assert_eq!(v["connections"], 46);
}

#[test]
fn compress_has_subpatch() {
    let out = pdtk_output(&["parse", pd_else("compress~.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["max_depth"], 1);
    assert_eq!(v["canvases"], 2);
}

// chorus~.pd — signal chain with subpatch

#[test]
fn chorus_signal_chain_validates() {
    let out = run_pdtk(&["validate", pd_else("chorus~.pd").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn chorus_has_inlet_and_outlet_tilde() {
    let out = pdtk_output(&["list", pd_else("chorus~.pd").to_str().unwrap()]);
    assert!(out.contains("inlet~"));
    assert!(out.contains("outlet~"));
}

// crusher~.pd — bit crusher, 2 subpatches

#[test]
fn crusher_two_subpatches() {
    let out = pdtk_output(&["parse", pd_else("crusher~.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["canvases"], 2);
}

// arpeggiator.pd — 5 subpatches, floatatoms, width hints, $0 namespacing

#[test]
fn arpeggiator_five_subpatches() {
    let out = pdtk_output(&["parse", pd_else("arpeggiator.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["canvases"], 5);
    assert_eq!(v["max_depth"], 1); // subpatches at depth 1
}

#[test]
fn arpeggiator_object_and_connection_counts() {
    let out = pdtk_output(&["parse", pd_else("arpeggiator.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["objects"], 110);
    assert_eq!(v["connections"], 136);
}

#[test]
fn arpeggiator_width_hint_not_counted_as_object() {
    // arpeggiator.pd has a #X f N width hint — must not consume an index
    let out = run_pdtk(&["validate", pd_else("arpeggiator.pd").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn arpeggiator_depth_0_connections_valid() {
    let out = pdtk_output(&["list", pd_else("arpeggiator.pd").to_str().unwrap(), "--depth", "0", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(!v.as_array().unwrap().is_empty());
}

// clock.pd — 7 subpatches, $0-namespaced sends, 133 connections

#[test]
fn clock_seven_subpatches() {
    let out = pdtk_output(&["parse", pd_else("clock.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["canvases"], 7);
}

#[test]
fn clock_connection_count_correct() {
    let out = pdtk_output(&["parse", pd_else("clock.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["connections"], 133);
}

#[test]
fn clock_zero_namespaced_sends_listed() {
    let out = pdtk_output(&["search", pd_else("clock.pd").to_str().unwrap(), "--type", "s"]);
    // $0-prefixed sends should appear
    assert!(out.contains("class:s"));
}

// pvoc~.pd — phase vocoder, 5 subpatches, FFT objects

#[test]
fn pvoc_five_subpatches() {
    let out = pdtk_output(&["parse", pd_else("pvoc~.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["canvases"], 5);
}

#[test]
fn pvoc_fft_objects_present() {
    let out = pdtk_output(&["search", pd_else("pvoc~.pd").to_str().unwrap(), "--type", "rfft~"]);
    assert!(out.contains("class:rfft~"));
}

// gran~.pd — granular synthesis, 6 subpatches, complex signal routing

#[test]
fn gran_six_subpatches() {
    let out = pdtk_output(&["parse", pd_else("gran~.pd").to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["canvases"], 6);
    assert_eq!(v["connections"], 111);
}

#[test]
fn gran_validates_cleanly() {
    let out = run_pdtk(&["validate", pd_else("gran~.pd").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}

// Cross-file analysis tests

#[test]
fn pd_else_stats_directory() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pd_else");
    let out = pdtk_output(&["stats", dir.to_str().unwrap(), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["total_files"], 11);
    // Aggregate: should have hundreds of objects total
    assert!(v["total_objects"].as_u64().unwrap() > 500);
}

#[test]
fn pd_else_search_for_send_objects() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pd_else");
    let out = pdtk_output(&["search", dir.to_str().unwrap(), "--type", "s"]);
    assert!(out.contains("class:s"));
}

#[test]
fn pd_else_find_orphans_scans_directory() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pd_else");
    let out = run_pdtk(&["find-orphans", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn pd_else_diff_identical_file_is_empty() {
    let f = pd_else("bpm.pd");
    let out = pdtk_output(&["diff", f.to_str().unwrap(), f.to_str().unwrap()]);
    assert!(out.contains("No differences"));
}
