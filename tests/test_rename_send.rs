mod integration;
use integration::{handcrafted, pdtk_output, run_pdtk, stdout_string};

fn with_copy(name: &str) -> (tempfile::NamedTempFile, String) {
    let src = std::fs::read_to_string(handcrafted(name)).unwrap();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &src).unwrap();
    (tmp, src)
}

#[test]
fn rename_send_renames_s_and_r_pair() {
    let (tmp, _) = with_copy("send_receive.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "clock_main",
        "--to",
        "clock_renamed",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("s clock_renamed"));
    assert!(result.contains("r clock_renamed"));
    assert!(!result.contains("clock_main"));
}

#[test]
fn rename_send_renames_s_tilde_r_tilde_pair() {
    let (tmp, _) = with_copy("send_receive.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "audio_bus",
        "--to",
        "audio_renamed",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("s~ audio_renamed"));
    assert!(result.contains("r~ audio_renamed"));
    assert!(!result.contains("audio_bus"));
}

#[test]
fn rename_send_renames_throw_catch_pair() {
    let (tmp, _) = with_copy("send_receive.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "reverb_bus",
        "--to",
        "reverb_renamed",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("throw~ reverb_renamed"));
    assert!(result.contains("catch~ reverb_renamed"));
    assert!(!result.contains("reverb_bus"));
}

#[test]
fn rename_send_renames_tgl_send_receive_fields() {
    let (tmp, _) = with_copy("all_gui_types.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "tgl_send",
        "--to",
        "tgl_renamed",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("tgl_renamed"));
    assert!(!result.contains("tgl_send"));
}

#[test]
fn rename_send_renames_bng_send_receive_fields() {
    let (tmp, _) = with_copy("all_gui_types.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "bng_recv",
        "--to",
        "bng_renamed",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("bng_renamed"));
    assert!(!result.contains("bng_recv"));
}

#[test]
fn rename_send_renames_nbx_send_receive_fields() {
    let (tmp, _) = with_copy("all_gui_types.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "nbx_send",
        "--to",
        "nbx_renamed",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("nbx_renamed"));
    assert!(!result.contains("nbx_send"));
}

#[test]
fn rename_send_renames_vsl_hsl_fields() {
    let (tmp, _) = with_copy("all_gui_types.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "vsl_send",
        "--to",
        "vsl_renamed",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("vsl_renamed"));
    assert!(!result.contains("vsl_send"));
}

#[test]
fn rename_send_dry_run_no_writes() {
    let (tmp, original) = with_copy("send_receive.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "clock_main",
        "--to",
        "clock_renamed",
        "--dry-run",
    ]);

    let after = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(after, original, "dry-run must not modify the file");
}

#[test]
fn rename_send_refuses_if_target_exists() {
    let (tmp, _) = with_copy("send_receive.pd");
    // audio_bus already exists — renaming clock_main to audio_bus should fail
    let out = run_pdtk(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "clock_main",
        "--to",
        "audio_bus",
        "--in-place",
    ]);
    assert_ne!(out.status.code(), Some(0));
}

#[test]
fn rename_send_force_flag_overrides_refusal() {
    let (tmp, _) = with_copy("send_receive.pd");
    let out = run_pdtk(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "clock_main",
        "--to",
        "audio_bus",
        "--in-place",
        "--force",
    ]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn rename_send_unmatched_files_byte_identical() {
    let (tmp, original) = with_copy("simple_chain.pd");
    // simple_chain has no sends/receives
    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "nonexistent_send",
        "--to",
        "whatever",
        "--in-place",
    ]);
    let after = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(after, original, "unmatched file must not be touched");
}

#[test]
fn rename_send_directory_mode() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pd");
    let b = dir.path().join("b.pd");
    std::fs::write(&a, "#N canvas 0 22 450 300 12;\n#X obj 50 50 s my_bus;\n").unwrap();
    std::fs::write(&b, "#N canvas 0 22 450 300 12;\n#X obj 50 50 r my_bus;\n").unwrap();

    pdtk_output(&[
        "rename-send",
        dir.path().to_str().unwrap(),
        "--from",
        "my_bus",
        "--to",
        "renamed_bus",
        "--in-place",
    ]);

    assert!(std::fs::read_to_string(&a).unwrap().contains("renamed_bus"));
    assert!(std::fs::read_to_string(&b).unwrap().contains("renamed_bus"));
}

/// Rename A→B then B→A returns original (round-trip invariant)
#[test]
fn rename_send_ab_then_ba_roundtrip() {
    let (tmp, original) = with_copy("send_receive.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "clock_main",
        "--to",
        "clock_temp",
        "--in-place",
    ]);
    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "clock_temp",
        "--to",
        "clock_main",
        "--in-place",
    ]);

    let after = std::fs::read_to_string(tmp.path()).unwrap();
    assert_eq!(after, original);
}

#[test]
fn rename_send_validates_after_mutation() {
    let (tmp, _) = with_copy("send_receive.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "clock_main",
        "--to",
        "clock_renamed",
        "--in-place",
    ]);

    let v = run_pdtk(&["validate", tmp.path().to_str().unwrap()]);
    assert_eq!(v.status.code(), Some(0));
}
