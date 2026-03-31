mod integration;
use integration::{handcrafted, pdtk_output, run_pdtk};

#[test]
fn connect_adds_new_connection() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    pdtk_output(&[
        "connect", tmp.path().to_str().unwrap(),
        "--depth", "0", "--src", "0", "--outlet", "0",
        "--dst", "1", "--inlet", "0", "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("#X connect 0 0 1 0;"));
}

#[test]
fn connect_duplicate_refused() {
    let f = handcrafted("simple_chain.pd");
    // simple_chain already has 0 0 1 0
    let out = run_pdtk(&[
        "connect", f.to_str().unwrap(),
        "--depth", "0", "--src", "0", "--outlet", "0", "--dst", "1", "--inlet", "0",
    ]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn connect_out_of_range_src_refused() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&[
        "connect", f.to_str().unwrap(),
        "--depth", "0", "--src", "99", "--outlet", "0", "--dst", "1", "--inlet", "0",
    ]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn connect_out_of_range_dst_refused() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&[
        "connect", f.to_str().unwrap(),
        "--depth", "0", "--src", "0", "--outlet", "0", "--dst", "99", "--inlet", "0",
    ]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn connect_validates_after_mutation() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = run_pdtk(&[
        "connect", tmp.path().to_str().unwrap(),
        "--depth", "0", "--src", "0", "--outlet", "0", "--dst", "1", "--inlet", "0",
    ]);
    assert_eq!(out.status.code(), Some(0));
}

/// connect then disconnect at same cord → original bytes
#[test]
fn connect_then_disconnect_roundtrip() {
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 f;\n\
                 #X obj 50 100 print;\n";
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(f.path(), input).unwrap();

    pdtk_output(&[
        "connect", f.path().to_str().unwrap(),
        "--depth", "0", "--src", "0", "--outlet", "0", "--dst", "1", "--inlet", "0",
        "--in-place",
    ]);
    pdtk_output(&[
        "disconnect", f.path().to_str().unwrap(),
        "--depth", "0", "--src", "0", "--outlet", "0", "--dst", "1", "--inlet", "0",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(f.path()).unwrap();
    assert_eq!(result, input);
}
