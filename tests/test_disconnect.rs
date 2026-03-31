mod integration;
use integration::{handcrafted, pdtk_output, run_pdtk, stdout_string};

#[test]
fn disconnect_removes_matching_connection() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "disconnect",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--src",
        "0",
        "--outlet",
        "0",
        "--dst",
        "1",
        "--inlet",
        "0",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(!result.contains("#X connect 0 0 1 0;"));
    // Other connection should remain
    assert!(result.contains("#X connect 1 0 2 0;"));
}

#[test]
fn disconnect_nonexistent_exits_1() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&[
        "disconnect",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--src",
        "0",
        "--outlet",
        "0",
        "--dst",
        "2",
        "--inlet",
        "0",
    ]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn disconnect_other_connections_unchanged() {
    let f = handcrafted("simple_chain.pd");
    let out = run_pdtk(&[
        "disconnect",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--src",
        "0",
        "--outlet",
        "0",
        "--dst",
        "1",
        "--inlet",
        "0",
    ]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = stdout_string(&out);
    // The remaining connection 1→2 must be present
    assert!(stdout.contains("#X connect 1 0 2 0;"));
}

#[test]
fn disconnect_validates_after_mutation() {
    let f = handcrafted("simple_chain.pd");
    let tmp = tempfile::NamedTempFile::new().unwrap();

    pdtk_output(&[
        "disconnect",
        f.to_str().unwrap(),
        "--depth",
        "0",
        "--src",
        "0",
        "--outlet",
        "0",
        "--dst",
        "1",
        "--inlet",
        "0",
        "--output",
        tmp.path().to_str().unwrap(),
    ]);

    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}
