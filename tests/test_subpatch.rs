mod integration;

use integration::{pdtk_output, run_pdtk, stdout_string};

fn write_tmp(content: &str) -> tempfile::NamedTempFile {
    let tmp = tempfile::Builder::new().suffix(".pd").tempfile().unwrap();
    std::fs::write(tmp.path(), content).unwrap();
    tmp
}

#[test]
fn subpatch_creates_block_at_top_level() {
    let tmp = write_tmp("#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang;\n");
    pdtk_output(&[
        "subpatch",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--name",
        "foo",
        "--inlets",
        "1",
        "--outlets",
        "1",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    // 6-arg subwindow header (name + vis flag), not a font header.
    assert!(
        out.contains("#N canvas 0 22 450 300 foo 0;"),
        "subwindow header missing; got:\n{out}"
    );
    assert!(out.contains("#X restore"), "restore missing; got:\n{out}");
    assert!(out.contains("pd foo;"), "restore name missing; got:\n{out}");
    assert!(out.contains("inlet;"), "inlet missing; got:\n{out}");
    assert!(out.contains("outlet;"), "outlet missing; got:\n{out}");
    // Result validates and round-trips through the parser.
    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0), "stdout:\n{}", stdout_string(&v));
}

#[test]
fn subpatch_appears_as_object_at_index() {
    let tmp = write_tmp("#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang;\n");
    pdtk_output(&[
        "subpatch",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--name",
        "foo",
        "--in-place",
    ]);
    // The subpatch's restore box is object [0:1].
    let list = pdtk_output(&["list", tmp.path().to_str().unwrap(), "--depth", "0"]);
    assert!(list.contains("[0:0] loadbang"), "got:\n{list}");
    assert!(list.contains("[0:1] restore"), "got:\n{list}");
}

#[test]
fn subpatch_renumbers_parent_connections() {
    let tmp = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 50 50 loadbang;\n\
         #X obj 50 100 print;\n\
         #X connect 0 0 1 0;\n",
    );
    // Insert a subpatch at index 0 (before loadbang): everything shifts +1.
    pdtk_output(&[
        "subpatch",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--name",
        "head",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        out.contains("#X connect 1 0 2 0;"),
        "parent connection should be renumbered 0->1,1->2; got:\n{out}"
    );
    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0), "stdout:\n{}", stdout_string(&v));
}

#[test]
fn subpatch_zero_io_is_valid() {
    let tmp = write_tmp("#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang;\n");
    pdtk_output(&[
        "subpatch",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--name",
        "empty_sub",
        "--inlets",
        "0",
        "--outlets",
        "0",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(out.contains("pd empty_sub;"), "got:\n{out}");
    assert!(!out.contains("inlet;"), "no inlets expected; got:\n{out}");
    assert!(!out.contains("outlet;"), "no outlets expected; got:\n{out}");
    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0), "stdout:\n{}", stdout_string(&v));
}

#[test]
fn subpatch_nested_inside_existing_subpatch() {
    let tmp = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #N canvas 0 22 200 200 outer 0;\n\
         #X obj 30 30 inlet;\n\
         #X restore 50 50 pd outer;\n",
    );
    // Create a subpatch inside `outer` (depth 1), at index 1.
    pdtk_output(&[
        "subpatch",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--index",
        "1",
        "--name",
        "inner",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        out.contains("pd inner;"),
        "nested subpatch missing; got:\n{out}"
    );
    // `inner` lives inside `outer`: its restore precedes outer's restore.
    let inner = out.find("pd inner;").unwrap();
    let outer = out.find("pd outer;").unwrap();
    assert!(inner < outer, "inner must close before outer; got:\n{out}");
    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0), "stdout:\n{}", stdout_string(&v));
}

#[test]
fn subpatch_before_existing_subpatch_preserves_parent_connections() {
    // Index 1 is currently `pd child`. The new block must be inserted before
    // child's whole `#N canvas ... #X restore` span; inserting immediately
    // before the restore line nests `newsub` inside `child` while still
    // renumbering the parent's connections (silent corruption).
    let tmp = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 10 10 loadbang;\n\
         #N canvas 0 22 200 200 child 0;\n\
         #X obj 20 20 inlet;\n\
         #X restore 50 50 pd child;\n\
         #X obj 80 80 print;\n\
         #X obj 80 120 f;\n\
         #X connect 0 0 2 0;\n",
    );
    run_pdtk(&[
        "subpatch",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--name",
        "newsub",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    let newsub = out
        .find("#N canvas 0 22 450 300 newsub 0;")
        .expect("newsub block missing");
    let child = out.find("#N canvas 0 22 200 200 child 0;").unwrap();
    assert!(
        newsub < child,
        "newsub must be a parent-canvas sibling before child, not nested inside it:\n{out}"
    );
    // Root now has 5 objects: loadbang, newsub, child, print, f.
    let list = pdtk_output(&["list", tmp.path().to_str().unwrap(), "--depth", "0"]);
    assert!(
        list.contains("[0:4]"),
        "root must have 5 objects; got:\n{list}"
    );
    // loadbang -> print survives as 0 0 3 0 (print shifted 2 -> 3).
    assert!(out.contains("#X connect 0 0 3 0;"), "got:\n{out}");
}

#[test]
fn subpatch_into_empty_subpatch_lands_inside_it() {
    // `shell` has no objects; the append path must still place the new block
    // inside shell's canvas, not at the end of the file (root canvas).
    let tmp = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 10 10 loadbang;\n\
         #N canvas 0 22 200 200 shell 0;\n\
         #X restore 50 50 pd shell;\n",
    );
    run_pdtk(&[
        "subpatch",
        tmp.path().to_str().unwrap(),
        "--depth",
        "1",
        "--index",
        "0",
        "--name",
        "inner",
        "--in-place",
    ]);
    let out = std::fs::read_to_string(tmp.path()).unwrap();
    let inner_restore = out.find("pd inner;").expect("inner block missing");
    let shell_restore = out.find("pd shell;").unwrap();
    assert!(
        inner_restore < shell_restore,
        "inner must close before shell (i.e. live inside it):\n{out}"
    );
}

#[test]
fn subpatch_rejects_name_with_whitespace() {
    // A space in the name breaks the 6-arg subwindow header
    // (`#N canvas X Y W H NAME VIS;`) — the command must refuse it.
    let input = "#N canvas 0 22 450 300 12;\n#X obj 50 50 loadbang;\n";
    let tmp = write_tmp(input);
    let out = run_pdtk(&[
        "subpatch",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--name",
        "bad name",
        "--in-place",
    ]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "whitespace name must be rejected"
    );
    let after = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(after, input, "failed subpatch must not write the file");
}

#[test]
fn subpatch_before_existing_subpatch_stays_in_parent_canvas() {
    // Creating a subpatch before an existing restore object must insert the
    // whole new block before the existing child block, not inside it.
    let tmp = write_tmp(
        "#N canvas 0 22 450 300 12;\n\
         #X obj 10 10 loadbang;\n\
         #N canvas 0 22 200 200 child 0;\n\
         #X obj 20 20 inlet;\n\
         #X restore 50 50 pd child;\n\
         #X obj 80 80 print;\n",
    );

    pdtk_output(&[
        "subpatch",
        tmp.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "1",
        "--name",
        "inserted",
        "--in-place",
    ]);

    let out = std::fs::read_to_string(tmp.path()).unwrap();
    let inserted_restore = out.find("pd inserted;").unwrap();
    let old_child_canvas = out.find("#N canvas 0 22 200 200 child 0;").unwrap();
    assert!(
        inserted_restore < old_child_canvas,
        "new subpatch must stay in the parent canvas before the old child block:\n{out}"
    );
}
