mod helpers;
use helpers::{handcrafted, read_fixture};
use pd_toolkit::{
    model::{EntryKind, ParseError},
    parser::parse,
    rewrite::serialize,
};

// Helpers

fn parse_fixture(name: &str) -> pd_toolkit::model::Patch {
    let input = read_fixture(&handcrafted(name));
    parse(&input).unwrap_or_else(|e| panic!("parse failed for {name}: {e}"))
}

// §7.4 Critical edge case tests (from the plan)

#[test]
fn multiline_msg_is_single_entry() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X msg 50 50 1 2 3 4 5 6 7 8\n\
                 9 10 11 12, f 40;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let patch = parse(input).unwrap();

    assert_eq!(patch.object_count_at_depth(0), 2);

    let msg = patch.object_at(0, 0).unwrap();
    assert!(
        msg.raw.contains('\n'),
        "multi-line entry must preserve newline"
    );
    assert!(
        msg.raw.contains("f 40"),
        "width hint must be preserved in raw"
    );
}

#[test]
fn standalone_width_hint_not_an_object() {
    let patch = parse_fixture("with_width_hint.pd");

    // depth 0: index 0 = restore, index 1 = print result
    assert_eq!(patch.object_count_at_depth(0), 2);

    let obj1 = patch.object_at(0, 1).unwrap();
    assert!(obj1.raw.contains("print result"));

    // #X f 38 must have no object index
    let hint = patch
        .entries
        .iter()
        .find(|e| e.raw.trim() == "#X f 38;")
        .expect("width hint entry missing");
    assert_eq!(hint.object_index, None);
}

#[test]
fn standalone_declare_not_an_object() {
    let patch = parse_fixture("with_declare.pd");

    // 10 real objects; standalone #X declare is not one of them
    assert_eq!(patch.object_count_at_depth(0), 10);

    let obj0 = patch.object_at(0, 0).unwrap();
    assert!(obj0.raw.contains("inlet"));

    let decl = patch
        .entries
        .iter()
        .find(|e| e.raw.starts_with("#X declare "))
        .expect("standalone declare missing");
    assert_eq!(decl.object_index, None);
}

#[test]
fn restore_indexed_at_parent_depth() {
    let patch = parse_fixture("nested_subpatch.pd");

    // depth 0: inlet (0), restore (1), outlet (2)
    assert_eq!(patch.object_count_at_depth(0), 3);

    let restore = patch.object_at(0, 1).unwrap();
    assert!(restore.raw.contains("restore"));

    let conns = patch.connections_at_depth(0);
    assert!(conns.iter().any(|c| c.src == 0 && c.dst == 1));
    assert!(conns.iter().any(|c| c.src == 1 && c.dst == 2));
}

#[test]
fn unknown_entry_type_handled_gracefully() {
    let patch = parse_fixture("with_c_entry.pd");

    // 7 real objects; #C restore is not one of them
    assert_eq!(patch.object_count_at_depth(0), 7);

    let c_entry = patch
        .entries
        .iter()
        .find(|e| e.raw.trim() == "#C restore;")
        .expect("#C entry missing");
    assert_eq!(c_entry.object_index, None);
    assert_eq!(c_entry.kind, EntryKind::Unknown);
}

#[test]
fn escaped_semicolon_in_message() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X msg 50 50 \\; pd dsp 1;\n\
                 #X obj 50 100 print;\n\
                 #X connect 0 0 1 0;\n";
    let patch = parse(input).unwrap();

    assert_eq!(patch.object_count_at_depth(0), 2);

    let msg = patch.object_at(0, 0).unwrap();
    assert!(
        msg.raw.contains("\\;"),
        "escaped semicolon must be preserved"
    );
}

#[test]
fn float_object_not_confused_with_width_hint() {
    let patch = parse_fixture("float_vs_width.pd");

    let obj0 = patch.object_at(0, 0).unwrap();
    assert_eq!(obj0.class(), "f", "bare float box class must be 'f'");

    let obj1 = patch.object_at(0, 1).unwrap();
    assert_eq!(obj1.class(), "f", "float box with arg class must be 'f'");

    let obj2 = patch.object_at(0, 2).unwrap();
    assert_eq!(
        obj2.class(),
        "t",
        "trigger class must be 't', not confused by ', f 8'"
    );

    let obj3 = patch.object_at(0, 3).unwrap();
    assert_eq!(
        obj3.class(),
        "+",
        "plus class must be '+', not confused by ', f 6'"
    );
}

#[test]
fn graph_restore_is_object() {
    let patch = parse_fixture("with_graph.pd");

    // depth 0: loadbang (0), graph restore (1), tabread (2), print (3)
    assert_eq!(patch.object_count_at_depth(0), 4);

    let graph = patch.object_at(0, 1).unwrap();
    assert!(graph.raw.contains("graph"));

    let conns = patch.connections_at_depth(0);
    assert!(conns.iter().any(|c| c.src == 0 && c.dst == 2));
    assert!(conns.iter().any(|c| c.src == 2 && c.dst == 3));
}

#[test]
fn gui_send_receive_extraction() {
    let patch = parse_fixture("all_gui_types.pd");

    let tgl = patch.object_at(0, 0).unwrap();
    assert_eq!(tgl.gui_send(), Some("tgl_send".to_owned()));
    assert_eq!(tgl.gui_receive(), Some("tgl_recv".to_owned()));

    let bng = patch.object_at(0, 1).unwrap();
    assert_eq!(bng.gui_send(), Some("bng_send".to_owned()));
    assert_eq!(bng.gui_receive(), Some("bng_recv".to_owned()));

    let floatatom = patch.object_at(0, 2).unwrap();
    assert_eq!(floatatom.gui_send(), None); // dash
    assert_eq!(floatatom.gui_receive(), Some("float_recv".to_owned()));

    let symbolatom = patch.object_at(0, 3).unwrap();
    assert_eq!(symbolatom.gui_send(), None);
    assert_eq!(symbolatom.gui_receive(), Some("sym_recv".to_owned()));

    let nbx = patch.object_at(0, 4).unwrap();
    assert_eq!(nbx.gui_send(), Some("nbx_send".to_owned()));
    assert_eq!(nbx.gui_receive(), Some("nbx_recv".to_owned()));

    let vsl = patch.object_at(0, 5).unwrap();
    assert_eq!(vsl.gui_send(), Some("vsl_send".to_owned()));
    assert_eq!(vsl.gui_receive(), Some("vsl_recv".to_owned()));

    let hsl = patch.object_at(0, 6).unwrap();
    assert_eq!(hsl.gui_send(), Some("hsl_send".to_owned()));
    assert_eq!(hsl.gui_receive(), Some("hsl_recv".to_owned()));

    let vradio = patch.object_at(0, 7).unwrap();
    assert_eq!(vradio.gui_send(), Some("vrad_send".to_owned()));
    assert_eq!(vradio.gui_receive(), Some("vrad_recv".to_owned()));

    let hradio = patch.object_at(0, 8).unwrap();
    assert_eq!(hradio.gui_send(), Some("hrad_send".to_owned()));
    assert_eq!(hradio.gui_receive(), Some("hrad_recv".to_owned()));

    let vu = patch.object_at(0, 9).unwrap();
    assert_eq!(vu.gui_send(), None); // vu has no send
    assert_eq!(vu.gui_receive(), Some("vu_recv".to_owned()));

    let cnv = patch.object_at(0, 10).unwrap();
    assert_eq!(cnv.gui_send(), Some("cnv_send".to_owned()));
    assert_eq!(cnv.gui_receive(), Some("cnv_recv".to_owned()));
}

// Parse — structural tests

#[test]
fn parse_minimal_pd_zero_objects() {
    let patch = parse_fixture("minimal.pd");
    assert_eq!(patch.object_count_at_depth(0), 0);
    assert_eq!(patch.canvas_count(), 1);
    assert_eq!(patch.max_depth(), 0);
}

#[test]
fn parse_simple_chain_correct_indices_and_connections() {
    let patch = parse_fixture("simple_chain.pd");
    assert_eq!(patch.object_count_at_depth(0), 3);

    assert_eq!(patch.object_at(0, 0).unwrap().class(), "loadbang");
    assert_eq!(patch.object_at(0, 1).unwrap().class(), "t");
    assert_eq!(patch.object_at(0, 2).unwrap().class(), "print");

    let conns = patch.connections_at_depth(0);
    assert_eq!(conns.len(), 2);
    assert!(conns.iter().any(|c| c.src == 0 && c.dst == 1));
    assert!(conns.iter().any(|c| c.src == 1 && c.dst == 2));
}

#[test]
fn parse_nested_subpatch_depth_tracking() {
    let patch = parse_fixture("nested_subpatch.pd");
    assert_eq!(patch.canvas_count(), 2);
    assert_eq!(patch.object_count_at_depth(0), 3);
    assert_eq!(patch.object_count_at_depth(1), 3);
    assert_eq!(patch.max_depth(), 1);
}

#[test]
fn parse_deep_subpatch_correct_per_depth_counts() {
    let patch = parse_fixture("deep_subpatch.pd");
    assert_eq!(patch.canvas_count(), 4);
    assert_eq!(patch.max_depth(), 3);
    for d in 0..=3 {
        assert_eq!(
            patch.object_count_at_depth(d),
            3,
            "depth {d} should have 3 objects"
        );
    }
}

#[test]
fn parse_empty_file_returns_error() {
    let result = parse("");
    assert_eq!(result.unwrap_err(), ParseError::EmptyInput);
}

#[test]
fn parse_whitespace_only_returns_empty_error() {
    let result = parse("   \n\n  ");
    assert_eq!(result.unwrap_err(), ParseError::EmptyInput);
}

#[test]
fn parse_missing_canvas_header_returns_error() {
    let result = parse("#X obj 10 10 f;\n");
    assert_eq!(result.unwrap_err(), ParseError::MissingCanvasHeader);
}

#[test]
fn parse_multiline_msg_single_entry() {
    let patch = parse_fixture("multiline_obj.pd");
    assert_eq!(patch.object_count_at_depth(0), 3);

    let msg = patch.object_at(0, 0).unwrap();
    assert_eq!(msg.kind, EntryKind::Msg);
    assert!(msg.raw.contains('\n'));
}

#[test]
fn parse_escaped_semicolons_not_split() {
    let patch = parse_fixture("escaped_semicolons.pd");
    // 4 objects: msg (0), msg (1), loadbang (2), t b b (3)
    assert_eq!(patch.object_count_at_depth(0), 4);

    let msg0 = patch.object_at(0, 0).unwrap();
    assert!(msg0.raw.contains("\\;"));
}

#[test]
fn parse_cycle_has_back_edge_connection() {
    let patch = parse_fixture("cycle.pd");
    let conns = patch.connections_at_depth(0);
    // cycle.pd has: f→+1→mod→f (back) + metro→f + loadbang→metro
    assert!(
        conns.iter().any(|c| c.src == 2 && c.dst == 0),
        "back edge mod→f must be present"
    );
}

#[test]
fn parse_large_patch_120_objects() {
    let patch = parse_fixture("large_patch.pd");
    assert_eq!(patch.object_count_at_depth(0), 120);
    assert!(patch.connections_at_depth(0).len() >= 119);
}

#[test]
fn parse_multiple_subpatches_independent_indexing() {
    let patch = parse_fixture("multiple_subpatches.pd");
    // depth 0: loadbang(0), restore_sub_a(1), restore_sub_b(2), print_a(3), print_b(4)
    assert_eq!(patch.object_count_at_depth(0), 5);
    // Two sibling subpatches both live at depth 1 — 3 objects each = 6 total
    assert_eq!(patch.object_count_at_depth(1), 6);

    // But their indices reset per canvas: both have a local index 0 (inlet)
    let depth1_inlet_indices: Vec<usize> = patch
        .entries
        .iter()
        .filter(|e| e.depth == 2 && e.raw.contains(" inlet;"))
        .filter_map(|e| e.object_index)
        .collect();
    assert_eq!(
        depth1_inlet_indices,
        vec![0, 0],
        "each sub resets its own index counter"
    );
}

#[test]
fn parse_graph_and_pd_subpatches_coexist() {
    let patch = parse_fixture("graph_and_pd_subpatches.pd");
    assert_eq!(patch.object_count_at_depth(0), 4);
    let graph_obj = patch.object_at(0, 1).unwrap();
    assert!(graph_obj.raw.contains("graph"));
    let pd_obj = patch.object_at(0, 2).unwrap();
    assert!(pd_obj.raw.contains("pd processor"));
}

#[test]
fn parse_entry_x_y_coordinates() {
    let patch = parse_fixture("simple_chain.pd");
    let lb = patch.object_at(0, 0).unwrap();
    assert_eq!(lb.x(), Some(50));
    assert_eq!(lb.y(), Some(50));
}

#[test]
fn parse_args_strips_width_hint() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 t f b, f 8;\n";
    let patch = parse(input).unwrap();
    let t = patch.object_at(0, 0).unwrap();
    assert_eq!(t.class(), "t");
    assert_eq!(t.args(), vec!["f", "b"]);
}

// Round-trip tests — parse then serialize must be byte-identical

fn assert_roundtrip(name: &str, path: std::path::PathBuf) {
    let input = read_fixture(&path);
    let patch = parse(&input).unwrap_or_else(|e| panic!("parse failed for {name}: {e}"));
    let output = serialize(&patch);
    assert_eq!(input, output, "round-trip failed for {name}");
}

#[test]
fn roundtrip_minimal() {
    assert_roundtrip("minimal.pd", handcrafted("minimal.pd"));
}

#[test]
fn roundtrip_simple_chain() {
    assert_roundtrip("simple_chain.pd", handcrafted("simple_chain.pd"));
}

#[test]
fn roundtrip_nested_subpatch() {
    assert_roundtrip("nested_subpatch.pd", handcrafted("nested_subpatch.pd"));
}

#[test]
fn roundtrip_multiline_obj() {
    assert_roundtrip("multiline_obj.pd", handcrafted("multiline_obj.pd"));
}

#[test]
fn roundtrip_escaped_semicolons() {
    assert_roundtrip(
        "escaped_semicolons.pd",
        handcrafted("escaped_semicolons.pd"),
    );
}

#[test]
fn roundtrip_escaped_chars() {
    assert_roundtrip("escaped_chars.pd", handcrafted("escaped_chars.pd"));
}

#[test]
fn roundtrip_with_declare() {
    assert_roundtrip("with_declare.pd", handcrafted("with_declare.pd"));
}

#[test]
fn roundtrip_with_width_hint() {
    assert_roundtrip("with_width_hint.pd", handcrafted("with_width_hint.pd"));
}

#[test]
fn roundtrip_with_c_entry() {
    assert_roundtrip("with_c_entry.pd", handcrafted("with_c_entry.pd"));
}

#[test]
fn roundtrip_all_handcrafted_fixtures() {
    let dir = helpers::fixtures_dir().join("handcrafted");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "pd").unwrap_or(false) {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            // empty_file.pd is intentionally invalid — skip round-trip
            if name == "empty_file.pd" {
                continue;
            }
            assert_roundtrip(&name, path);
        }
    }
}

#[test]
fn roundtrip_all_corpus_files() {
    let dir = helpers::fixtures_dir().join("corpus");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "pd").unwrap_or(false) {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            assert_roundtrip(&name, path);
        }
    }
}

// All corpus files parse without error

#[test]
fn parse_all_corpus_files_no_error() {
    let dir = helpers::fixtures_dir().join("corpus");
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "pd").unwrap_or(false) {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let input = read_fixture(&path);
            parse(&input).unwrap_or_else(|e| panic!("parse failed for {name}: {e}"));
        }
    }
}
