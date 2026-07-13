mod integration;
use integration::{handcrafted, pdtk_output, run_pdtk};

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
fn rename_send_auto_escapes_dollar_args() {
    let (tmp, _) = with_copy("dollar_signs.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "s$1_output",
        "--to",
        "s$2_output",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains(r"s\$2_output"));
    assert!(!result.contains(r"s\$1_output"));
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

#[test]
fn rename_send_handles_hdl_vdl_compat_radios() {
    // hdl and vdl are old-compat names for hradio/vradio; same arg layout.
    let (tmp, _) = with_copy("radio_compat_hdl_vdl.pd");

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "hdl_send",
        "--to",
        "hdl_renamed",
        "--in-place",
    ]);

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("hdl_renamed"));
    assert!(!result.contains("hdl_send"));

    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "vdl_recv",
        "--to",
        "vdl_recv_renamed",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("vdl_recv_renamed"));
}

#[test]
fn rename_send_handles_listbox_send_field() {
    let (tmp, _) = with_copy("listbox_send_receive.pd");
    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "list_send",
        "--to",
        "list_send_renamed",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("list_send_renamed"));
    assert!(!result.contains(" list_send "));
}

#[test]
fn rename_send_handles_listbox_receive_field() {
    let (tmp, _) = with_copy("listbox_send_receive.pd");
    pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "list_recv",
        "--to",
        "list_recv_renamed",
        "--in-place",
    ]);
    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(result.contains("list_recv_renamed"));
}

#[test]
fn rename_send_handles_inline_width_hint() {
    // A send/receive object whose name is the last token before a `, f N`
    // width hint must still be renamed, and the hint must be preserved.
    let input = "#N canvas 0 22 450 300 12;\n\
                 #X obj 50 50 s clock_bus, f 42;\n\
                 #X obj 200 200 r clock_bus, f 20;\n";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), input).unwrap();

    let out = pdtk_output(&[
        "rename-send",
        tmp.path().to_str().unwrap(),
        "--from",
        "clock_bus",
        "--to",
        "master_clock",
        "--in-place",
    ]);
    assert!(out.contains("2 replacement"), "got:\n{out}");

    let result = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(
        result.contains("#X obj 50 50 s master_clock, f 42;"),
        "got:\n{result}"
    );
    assert!(
        result.contains("#X obj 200 200 r master_clock, f 20;"),
        "got:\n{result}"
    );
    assert!(
        !result.contains("clock_bus"),
        "old name should be gone, got:\n{result}"
    );
}
