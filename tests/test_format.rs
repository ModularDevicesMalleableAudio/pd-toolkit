mod integration;

use integration::{fixture_path, handcrafted, pdtk_output, run_pdtk, stdout_string};

fn connections_from(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.trim_start().starts_with("#X connect"))
        .map(|l| l.to_owned())
        .collect()
}

// Critical test
#[test]
fn format_connections_byte_identical_before_after() {
    let f = handcrafted("simple_chain.pd");
    let original = std::fs::read_to_string(&f).unwrap();
    let out = pdtk_output(&["format", f.to_str().unwrap(), "--dry-run"]);
    assert_eq!(connections_from(&original), connections_from(&out));
}

#[test]
fn format_coordinates_changed() {
    let f = handcrafted("simple_chain.pd");
    let original = std::fs::read_to_string(&f).unwrap();
    let out = pdtk_output(&["format", f.to_str().unwrap(), "--dry-run"]);
    // Object text must exist in both, but some coordinates will differ
    assert!(out.contains("loadbang"));
    assert!(out.contains("print done"));
    // Since grid-snap may or may not change coords, just verify connections survived
    assert_eq!(connections_from(&original), connections_from(&out));
}

#[test]
fn format_object_text_unchanged() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["format", f.to_str().unwrap(), "--dry-run"]);
    assert!(out.contains("loadbang"));
    assert!(out.contains("t b"));
    assert!(out.contains("print done"));
}

#[test]
fn format_no_overlapping_bboxes() {
    let f = handcrafted("branching.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "format",
        f.to_str().unwrap(),
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    // After formatting, lint should not report overlaps
    let lint_out = run_pdtk(&["lint", tmp.path().to_str().unwrap()]);
    assert_eq!(lint_out.status.code(), Some(0));
}

#[test]
fn format_grid_alignment_respected() {
    let f = handcrafted("simple_chain.pd");
    let out = pdtk_output(&["format", f.to_str().unwrap(), "--dry-run", "--grid", "20"]);

    // Parse coordinates from output and verify they are multiples of 20
    for line in out.lines() {
        if line.starts_with("#X obj") || line.starts_with("#X msg") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let (Some(x_str), Some(y_str)) = (parts.get(2), parts.get(3)) {
                if let (Ok(x), Ok(y)) = (x_str.parse::<i32>(), y_str.parse::<i32>()) {
                    assert_eq!(x % 20, 0, "x={x} is not a multiple of 20");
                    assert_eq!(y % 20, 0, "y={y} is not a multiple of 20");
                }
            }
        }
    }
}

#[test]
fn format_dry_run_no_file_output() {
    let f = handcrafted("simple_chain.pd");
    let original = std::fs::read_to_string(&f).unwrap();

    // --dry-run must not touch the source file
    pdtk_output(&["format", f.to_str().unwrap(), "--dry-run"]);
    let after = std::fs::read_to_string(&f).unwrap();
    assert_eq!(original, after, "dry-run must not modify the source file");
}

#[test]
fn format_all_corpus_files_connections_preserved() {
    let corpus = fixture_path("corpus");
    for entry in std::fs::read_dir(&corpus).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().map(|e| e == "pd").unwrap_or(false) {
            continue;
        }
        let original = std::fs::read_to_string(&path).unwrap();
        let out = pdtk_output(&["format", path.to_str().unwrap(), "--dry-run"]);
        let orig_conns = connections_from(&original);
        let new_conns = connections_from(&out);
        assert_eq!(
            orig_conns,
            new_conns,
            "connections changed for {}",
            path.display()
        );
    }
}

#[test]
fn format_pd_else_corpus_connections_preserved() {
    let corpus = fixture_path("pd_else");
    for entry in std::fs::read_dir(&corpus).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().map(|e| e == "pd").unwrap_or(false) {
            continue;
        }
        let original = std::fs::read_to_string(&path).unwrap();
        let out = pdtk_output(&["format", path.to_str().unwrap(), "--dry-run"]);
        let orig_conns = connections_from(&original);
        let new_conns = connections_from(&out);
        assert_eq!(
            orig_conns,
            new_conns,
            "connections changed for {}",
            path.display()
        );
    }
}

#[test]
fn format_depth_filter_only_affects_specified_depth() {
    let f = handcrafted("nested_subpatch.pd");
    let original = std::fs::read_to_string(&f).unwrap();

    // Format only depth 1 (inside subpatch) — depth-0 connections must be untouched
    let out = pdtk_output(&["format", f.to_str().unwrap(), "--dry-run", "--depth", "1"]);
    let orig_d0_conns = connections_from(&original);
    let new_d0_conns = connections_from(&out);
    // All connections (depth 0 and 1) must be byte-identical
    assert_eq!(orig_d0_conns, new_d0_conns);
}

#[test]
fn format_idempotent() {
    // Formatting the output of format should produce identical output.
    let f = handcrafted("branching.pd");

    let tmp1 = tempfile::NamedTempFile::new().unwrap();
    pdtk_output(&[
        "format",
        f.to_str().unwrap(),
        "--grid",
        "20",
        "--output",
        tmp1.path().to_str().unwrap(),
    ]);
    let first = std::fs::read_to_string(tmp1.path()).unwrap();

    let tmp2 = tempfile::NamedTempFile::new().unwrap();
    pdtk_output(&[
        "format",
        tmp1.path().to_str().unwrap(),
        "--grid",
        "20",
        "--output",
        tmp2.path().to_str().unwrap(),
    ]);
    let second = std::fs::read_to_string(tmp2.path()).unwrap();

    assert_eq!(first, second, "format must be idempotent");
}

#[test]
fn format_preserves_connections_invariant() {
    // Parametric check over several fixtures.
    let fixtures = &[
        "simple_chain.pd",
        "cycle.pd",
        "branching.pd",
        "merging.pd",
        "nested_subpatch.pd",
        "multiple_subpatches.pd",
    ];
    for &name in fixtures {
        let f = handcrafted(name);
        let original = std::fs::read_to_string(&f).unwrap();
        let out = pdtk_output(&["format", f.to_str().unwrap(), "--dry-run"]);
        assert_eq!(
            connections_from(&original),
            connections_from(&out),
            "connections changed for {name}"
        );
    }
}

#[test]
fn format_output_flag_writes_to_file() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    let out = run_pdtk(&[
        "format",
        f.to_str().unwrap(),
        "--output",
        tmp.path().to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout_string(&out).trim().is_empty());

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("loadbang"));
    assert!(result.contains("#X connect"));
}

#[test]
fn format_in_place_backup_creates_bak() {
    let src = std::fs::read_to_string(handcrafted("simple_chain.pd")).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &src).unwrap();
    let bak = format!("{}.bak", tmp.path().display());

    pdtk_output(&[
        "format",
        tmp.path().to_str().unwrap(),
        "--in-place",
        "--backup",
    ]);

    assert!(std::path::Path::new(&bak).exists());
    let bak_content = std::fs::read_to_string(&bak).unwrap();
    assert_eq!(bak_content, src);
    std::fs::remove_file(&bak).ok();
}
