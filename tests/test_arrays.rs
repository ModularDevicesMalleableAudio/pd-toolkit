mod integration;

use integration::{fixture_path, handcrafted, pdtk_output, run_pdtk, stderr_string};
use serde_json::Value;

#[test]
fn arrays_lists_all_defined_arrays() {
    let f = handcrafted("arrays.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap()]);
    assert!(out.contains("waveform_a"));
    assert!(out.contains("waveform_b"));
}

#[test]
fn arrays_shows_name_and_size() {
    let f = handcrafted("arrays.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap()]);
    assert!(out.contains("size 256"));
    assert!(out.contains("size 128"));
}

#[test]
fn arrays_json_output_schema() {
    let f = handcrafted("arrays.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("arrays").is_some());
    assert!(v.get("duplicate_names").is_some());
    assert_eq!(v["schema_version"], 2);
}

#[test]
fn arrays_directory_deduplication() {
    let dir = fixture_path("handcrafted");
    let out = pdtk_output(&["arrays", dir.to_str().unwrap(), "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v["arrays"].as_array().unwrap().len() >= 2);
}

#[test]
fn arrays_detects_duplicate_names_across_files() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pd");
    let b = dir.path().join("b.pd");

    std::fs::write(
        &a,
        "#N canvas 0 0 100 100 10;\n#N canvas 0 0 100 100 (subpatch) 0;\n#X array same 16 float 3;\n#A 0 0 0;\n#X coords 0 1 15 -1 100 50 1 0 0;\n#X restore 10 10 graph;\n",
    )
    .unwrap();
    std::fs::write(
        &b,
        "#N canvas 0 0 100 100 10;\n#N canvas 0 0 100 100 (subpatch) 0;\n#X array same 32 float 3;\n#A 0 0 0;\n#X coords 0 1 31 -1 100 50 1 0 0;\n#X restore 10 10 graph;\n",
    )
    .unwrap();

    let out = pdtk_output(&["arrays", dir.path().to_str().unwrap(), "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v["duplicate_names"]["same"].as_array().unwrap().len() == 2);
}

// ---------- v1 schema back-compat ----------

#[test]
fn arrays_schema_v1_envelope_matches_legacy_shape() {
    let f = handcrafted("arrays.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--schema=1", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v.get("schema_version").is_none());
    assert!(v.get("arrays").is_some());
    assert!(v.get("duplicate_names").is_some());
    let row = &v["arrays"][0];
    // v1 row keys only.
    for absent in ["kind", "is_template", "classic", "define", "index"] {
        assert!(
            row.get(absent).is_none(),
            "v1 row should not contain `{absent}`"
        );
    }
    assert!(row.get("name").is_some());
    assert!(row.get("size").is_some());
    assert!(row.get("file").is_some());
    assert!(row.get("depth").is_some());
}

#[test]
fn arrays_schema_v1_preserves_consumer_query_results() {
    let f = handcrafted("arrays.pd");
    let v1: Value = serde_json::from_str(&pdtk_output(&[
        "arrays",
        f.to_str().unwrap(),
        "--schema=1",
        "--json",
    ]))
    .unwrap();
    let v2: Value = serde_json::from_str(&pdtk_output(&[
        "arrays",
        f.to_str().unwrap(),
        "--schema=2",
        "--json",
    ]))
    .unwrap();
    let names_v1: Vec<&str> = v1["arrays"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    let names_v2: Vec<&str> = v2["arrays"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert_eq!(names_v1, names_v2);
}

// ---------- classic save_flag decoding ----------

#[test]
fn arrays_classic_decodes_save_flag_bits() {
    let f = handcrafted("classic_array_save_flags.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let by_name: std::collections::HashMap<&str, &Value> = v["arrays"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["name"].as_str().unwrap(), r))
        .collect();

    let nosave = &by_name["nosave_default"]["classic"];
    assert_eq!(nosave["save_flag"], 0);
    assert_eq!(nosave["saveit"], false);
    assert_eq!(nosave["filestyle"], "polygon");
    assert_eq!(nosave["hidename"], false);

    let saved = &by_name["saved_points"]["classic"];
    assert_eq!(saved["save_flag"], 3);
    assert_eq!(saved["saveit"], true);
    assert_eq!(saved["filestyle"], "points");
    assert_eq!(saved["hidename"], false);

    let hidden = &by_name["hidden_polygon"]["classic"];
    assert_eq!(hidden["save_flag"], 9);
    assert_eq!(hidden["saveit"], true);
    assert_eq!(hidden["filestyle"], "polygon");
    assert_eq!(hidden["hidename"], true);

    let bad = &by_name["malformed_save"]["classic"];
    assert!(bad["save_flag"].is_null());
    assert!(bad["saveit"].is_null());
    assert!(bad["filestyle"].is_null());
    assert!(bad["hidename"].is_null());
}

// ---------- Real corpus regression ----------

#[test]
fn arrays_real_corpus_aggregate_counts() {
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--kind=all", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let arrays = v["arrays"].as_array().unwrap();

    let mut classic = 0;
    let mut define = 0;
    let mut clean = 0;
    let mut partial = 0;
    let mut templates = 0;
    for r in arrays {
        match r["kind"].as_str().unwrap() {
            "classic" => classic += 1,
            "define" => define += 1,
            _ => unreachable!(),
        }
        if r["is_template"].as_bool().unwrap() {
            templates += 1;
        }
        if let Some(d) = r.get("define").and_then(|x| x.as_object()) {
            match d["parse_status"].as_str().unwrap() {
                "clean" => clean += 1,
                "partial" => partial += 1,
                _ => unreachable!(),
            }
        }
    }
    assert_eq!(define, 340, "expected 340 array-define rows");
    assert_eq!(classic, 1, "expected 1 classic row");
    assert_eq!(clean, 336, "expected 336 clean parse_status rows");
    assert_eq!(partial, 4, "expected 4 partial parse_status rows");
    assert_eq!(templates, 114, "expected 114 templated names");
}

#[test]
fn arrays_real_corpus_unknown_flag_guard_preserves_name_and_size() {
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let row = v["arrays"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "sample_unknown_guard")
        .expect("sample_unknown_guard row must exist");
    assert_eq!(row["size"], 32);
    let d = &row["define"];
    assert_eq!(d["parse_status"], "partial");
    assert_eq!(d["k"], false);
    assert!(d["yrange"].is_null());
    assert!(d["pix"].is_null());
    let dt = d["discarded_tokens"].as_array().unwrap();
    assert_eq!(dt.len(), 1);
    assert_eq!(dt[0]["reason"], "unknown_flag");
    assert_eq!(
        dt[0]["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["-newflag", "arg1", "arg2"]
    );
}

#[test]
fn arrays_real_corpus_synthetic_payloads() {
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let by_name: std::collections::HashMap<String, &Value> = v["arrays"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["name"].as_str().unwrap().to_string(), r))
        .collect();

    let all_flags = &by_name["sample_all_flags"]["define"];
    assert_eq!(all_flags["k"], true);
    assert_eq!(all_flags["yrange"], serde_json::json!([0, 128]));
    assert_eq!(all_flags["pix"], serde_json::json!([600, 400]));
    assert_eq!(all_flags["parse_status"], "clean");

    let float_yr = &by_name["sample_float_yrange"]["define"];
    assert_eq!(float_yr["yrange"], serde_json::json!([0.5, 1.5]));
    assert_eq!(float_yr["parse_status"], "clean");

    let degen = &by_name["sample_degenerate_yrange"]["define"];
    assert_eq!(degen["yrange"], serde_json::json!([5, 5]));

    let superseded = &by_name["sample_superseded"]["define"];
    assert_eq!(superseded["yrange"], serde_json::json!([0, 128]));
    let dt = superseded["discarded_tokens"].as_array().unwrap();
    assert_eq!(dt[0]["reason"], "superseded_yrange");
    assert_eq!(dt[0]["tokens"], serde_json::json!(["-yrange", "0", "64"]));

    let mal = &by_name["sample_malformed_yrange"]["define"];
    assert!(mal["yrange"].is_null());
    let dt = mal["discarded_tokens"].as_array().unwrap();
    assert_eq!(dt[0]["reason"], "malformed_yrange");
    assert_eq!(
        dt[0]["tokens"],
        serde_json::json!(["-yrange", "notanum", "nope"])
    );

    let partial = &by_name["sample_partial_yrange"]["define"];
    assert_eq!(partial["parse_status"], "partial");
    let dt = partial["discarded_tokens"].as_array().unwrap();
    assert_eq!(dt[0]["reason"], "unknown_flag");
    assert_eq!(dt[0]["tokens"], serde_json::json!(["-yrange", "5"]));

    let pix_sub = &by_name["sample_pix_subclamp"]["define"];
    assert_eq!(pix_sub["pix"], serde_json::json!([5, 5]));

    let neg_yr = &by_name["sample_negative_yrange"]["define"];
    assert_eq!(neg_yr["yrange"], serde_json::json!([-1, 1]));

    let neg_float = &by_name["sample_negative_float_yrange"]["define"];
    assert_eq!(neg_float["yrange"], serde_json::json!([-10.5, 10.5]));

    let rep_k = &by_name["sample_repeated_k"]["define"];
    assert_eq!(rep_k["k"], true);
    assert_eq!(rep_k["parse_status"], "clean");

    let dash = &by_name["-dashname_divergence"]["define"];
    assert_eq!(dash["parse_status"], "clean");
    assert!(dash["yrange"].is_null());

    // `array d` synonym entries.
    assert!(by_name.contains_key("sample_d_form"));
    assert_eq!(by_name["sample_d_form"]["define"]["k"], false);
    assert_eq!(by_name["sample_d_with_k"]["define"]["k"], true);
}

#[test]
fn arrays_real_corpus_classic_define_cross_kind_duplicate() {
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--kind=all", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let dup = &v["duplicate_names"]["sample_d_form"];
    let entries = dup.as_array().expect("sample_d_form must be a duplicate");
    let kinds: Vec<&str> = entries
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"classic"));
    assert!(kinds.contains(&"define"));
}

#[test]
fn arrays_real_corpus_subpatch_depth() {
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    for name in ["sub_array_in_subpatch", "sub_d_synonym"] {
        let row = v["arrays"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap_or_else(|| panic!("subpatch row {name} missing"));
        assert_eq!(row["depth"], 1, "{name} should be at depth 1");
    }
}

#[test]
fn arrays_real_corpus_block_clean_invariant() {
    // The first 319 #X obj entries (the verbatim corpus block) must all be
    // parse_status="clean" with empty discarded_tokens.  Walks the file in
    // declaration order and slices off the corpus block.
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let arrays = v["arrays"].as_array().unwrap();
    // We sort by (file, depth, index) so depth=0 corpus entries come first
    // (319 of them, indices 0..319), then depth=1 subpatch (2), then the
    // synthetic depth=0 entries.  The 2 subpatch entries have depth=1, so
    // depth=0 with index in 0..319 captures the pure corpus block.
    let corpus: Vec<&Value> = arrays
        .iter()
        .filter(|r| r["depth"] == 0 && r["index"].as_u64().unwrap() < 319)
        .collect();
    assert_eq!(
        corpus.len(),
        319,
        "corpus block should yield exactly 319 rows"
    );
    for r in corpus {
        let d = &r["define"];
        assert_eq!(
            d["parse_status"], "clean",
            "corpus row {} should be clean",
            r["name"]
        );
        assert_eq!(
            d["discarded_tokens"].as_array().unwrap().len(),
            0,
            "corpus row {} should have no discarded tokens",
            r["name"]
        );
    }
}

// ---------- Filtering ----------

#[test]
fn arrays_kind_classic_excludes_defines() {
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--kind=classic", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let arrays = v["arrays"].as_array().unwrap();
    assert_eq!(arrays.len(), 1);
    assert_eq!(arrays[0]["kind"], "classic");
}

#[test]
fn arrays_kind_define_excludes_classic() {
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let arrays = v["arrays"].as_array().unwrap();
    assert_eq!(arrays.len(), 340);
    for r in arrays {
        assert_eq!(r["kind"], "define");
    }
}

#[test]
fn arrays_templates_exclude_drops_dollar_named() {
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&[
        "arrays",
        f.to_str().unwrap(),
        "--kind=define",
        "--templates=exclude",
        "--json",
    ]);
    let v: Value = serde_json::from_str(&out).unwrap();
    for r in v["arrays"].as_array().unwrap() {
        assert_eq!(r["is_template"], false);
    }
}

#[test]
fn arrays_templates_only_keeps_dollar_named() {
    let f = fixture_path("corpus/array_define_real.pd");
    let out = pdtk_output(&[
        "arrays",
        f.to_str().unwrap(),
        "--kind=define",
        "--templates=only",
        "--json",
    ]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let arrays = v["arrays"].as_array().unwrap();
    assert!(!arrays.is_empty());
    for r in arrays {
        assert_eq!(r["is_template"], true);
    }
}

// ---------- Non-.pd file ergonomics ----------

#[test]
fn arrays_non_pd_file_returns_empty_list() {
    let f = handcrafted("not_a_patch.pd_lua");
    let out = pdtk_output(&["arrays", f.to_str().unwrap(), "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v["arrays"].as_array().unwrap().is_empty());
    assert!(v["duplicate_names"].as_object().unwrap().is_empty());
}

#[test]
fn arrays_non_pd_file_warns_under_verbose() {
    let f = handcrafted("not_a_patch.pd_lua");
    let out = run_pdtk(&["--verbose", "arrays", f.to_str().unwrap(), "--json"]);
    assert!(out.status.success());
    let stderr = stderr_string(&out);
    assert!(
        stderr.contains("not a .pd file"),
        "expected verbose warning about non-.pd file, got stderr:\n{stderr}"
    );
}

// ---------- Schema flag validation ----------

#[test]
fn arrays_unknown_schema_value_errors_cleanly() {
    let f = handcrafted("arrays.pd");
    let out = run_pdtk(&["arrays", f.to_str().unwrap(), "--schema=99"]);
    assert!(!out.status.success());
}

// ---------- `array d` synonym ----------

#[test]
fn arrays_d_synonym_parses_identically() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("syn.pd");
    std::fs::write(
        &p,
        "#N canvas 0 0 100 100 10;\n#X obj 50 50 array define -k foo 16;\n#X obj 50 80 array d -k bar 16;\n",
    )
    .unwrap();
    let out = pdtk_output(&["arrays", p.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let arrays = v["arrays"].as_array().unwrap();
    assert_eq!(arrays.len(), 2);
    for r in arrays {
        assert_eq!(r["define"]["k"], true);
        assert_eq!(r["size"], 16);
    }
}

// ---------- malformed inputs ----------

#[test]
fn arrays_define_missing_size_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("bad.pd");
    std::fs::write(
        &p,
        "#N canvas 0 0 100 100 10;\n#X obj 50 50 array define lonely;\n",
    )
    .unwrap();
    let out = pdtk_output(&["arrays", p.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v["arrays"].as_array().unwrap().is_empty());
}

#[test]
fn arrays_define_non_integer_size_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("bad.pd");
    std::fs::write(
        &p,
        "#N canvas 0 0 100 100 10;\n#X obj 50 50 array define foo 16 -k;\n",
    )
    .unwrap();
    // Right-anchored: trailing -k means size_tok = "-k", not an integer.
    let out = pdtk_output(&["arrays", p.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    assert!(v["arrays"].as_array().unwrap().is_empty());
}

// ---------- escape handling ----------

#[test]
fn arrays_define_unescapes_dollar_in_name() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("tpl.pd");
    std::fs::write(
        &p,
        "#N canvas 0 0 100 100 10;\n#X obj 50 50 array define \\$1_stepvelo 16;\n",
    )
    .unwrap();
    let out = pdtk_output(&["arrays", p.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let row = &v["arrays"][0];
    assert_eq!(row["name"], "$1_stepvelo");
    assert_eq!(row["is_template"], true);
}

#[test]
fn arrays_define_dollar_zero_is_not_template() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("buf.pd");
    std::fs::write(
        &p,
        "#N canvas 0 0 100 100 10;\n#X obj 50 50 array define synth_\\$0_buffer 16;\n",
    )
    .unwrap();
    let out = pdtk_output(&["arrays", p.to_str().unwrap(), "--kind=define", "--json"]);
    let v: Value = serde_json::from_str(&out).unwrap();
    let row = &v["arrays"][0];
    assert_eq!(row["name"], "synth_$0_buffer");
    assert_eq!(row["is_template"], false);
}
