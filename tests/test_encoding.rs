//! Encoding / byte-level robustness cases for the `.pd` reader.
//!
//! PD is byte-oriented: it loads patches regardless of text encoding and
//! round-trips the bytes losslessly. Read-only/analysis commands therefore
//! read leniently (`parser::decode_lenient`: invalid bytes → U+FFFD, BOM
//! stripped in `parse`), while write-capable commands read strictly and refuse
//! a non-UTF-8 file rather than corrupt it (Option E). The structural PD syntax
//! (sigils, indices, connections, classes) is always ASCII, so lenient reading
//! always parses the structure correctly — only human-readable comment/label
//! bytes are lossy.
//!
//! Case matrix (all active):
//!
//!   valid UTF-8 2-byte (Latin accents), 3-byte (CJK, €), 4-byte (emoji),
//!     combining diacritics, embedded NUL — parse + byte-identical round-trip
//!   another-encoding file (UTF-16) fails gracefully, never panics
//!   mutating commands refuse a non-UTF-8 file (fail safe, don't corrupt)
//!   Latin-1 / Windows-1252 / WTF-8 surrogate emoji parse structurally
//!   lone continuation byte / truncated multibyte / overlong encoding
//!   non-ASCII inside a receive name; mixed UTF-8 + Latin-1 in one file
//!   `deps` (directory mode) does NOT silently skip a non-UTF-8 file
//!   UTF-8 BOM before `#N canvas` is tolerated

mod integration;
use integration::run_pdtk;
use std::path::Path;

/// Write raw bytes to a `.pd` temp file.
fn write_pd(bytes: &[u8]) -> tempfile::NamedTempFile {
    let tmp = tempfile::Builder::new().suffix(".pd").tempfile().unwrap();
    std::fs::write(tmp.path(), bytes).unwrap();
    tmp
}

/// A minimal 3-object patch (text[0], osc~[1], dac~[2], connect 1→2) with the
/// given bytes embedded in the comment text. The structure is pure ASCII, so
/// every case below must parse to the same 3 objects + 1 connection.
fn patch_with_comment(comment: &[u8]) -> Vec<u8> {
    let mut v = b"#N canvas 0 22 450 300 12;\n#X text 20 20 ".to_vec();
    v.extend_from_slice(comment);
    v.extend_from_slice(b";\n#X obj 20 50 osc~ 440;\n#X obj 20 80 dac~;\n#X connect 1 0 2 0;\n");
    v
}

fn json_out(args: &[&str]) -> (i32, serde_json::Value) {
    let out = run_pdtk(args);
    let code = out.status.code().unwrap_or(-1);
    let v = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    (code, v)
}

/// Assert the patch parses to the canonical 3 objects with osc~ at index 1
/// (i.e. the ASCII structure survived whatever bytes are in the comment).
fn assert_structure_ok(path: &Path) {
    let (code, v) = json_out(&["list", path.to_str().unwrap(), "--json"]);
    assert_eq!(code, 0, "list must succeed on this file");
    let arr = v.as_array().expect("list --json array");
    assert_eq!(arr.len(), 3, "expected 3 objects, got {}", arr.len());
    assert_eq!(arr[1]["class"], "osc~");
}

/// Assert `parse --output` reproduces the input bytes exactly.
fn assert_roundtrip_identical(bytes: &[u8], path: &Path) {
    let rt = tempfile::Builder::new().suffix(".pd").tempfile().unwrap();
    let out = run_pdtk(&[
        "parse",
        path.to_str().unwrap(),
        "--output",
        rt.path().to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0), "parse --output must succeed");
    let got = std::fs::read(rt.path()).unwrap();
    assert_eq!(got, bytes, "round-trip must be byte-identical");
}

// ---------------------------------------------------------------------------
// Control group — valid UTF-8. Must parse, round-trip byte-identically, today.
// ---------------------------------------------------------------------------

#[test]
fn utf8_two_byte_latin_accents() {
    // "café résumé" — U+00E9 etc. as 2-byte UTF-8.
    let bytes = patch_with_comment("café résumé".as_bytes());
    let f = write_pd(&bytes);
    assert_structure_ok(f.path());
    assert_roundtrip_identical(&bytes, f.path());
}

#[test]
fn utf8_three_byte_cjk_and_currency() {
    // Japanese + Euro sign — 3-byte UTF-8.
    let bytes = patch_with_comment("日本語 €100".as_bytes());
    let f = write_pd(&bytes);
    assert_structure_ok(f.path());
    assert_roundtrip_identical(&bytes, f.path());
}

#[test]
fn utf8_four_byte_emoji() {
    // 🎛 control-knobs emoji, properly encoded as 4-byte UTF-8 (F0 9F 8E 9B).
    let bytes = patch_with_comment("knob 🎛 here".as_bytes());
    let f = write_pd(&bytes);
    assert_structure_ok(f.path());
    assert_roundtrip_identical(&bytes, f.path());
}

#[test]
fn utf8_combining_diacritic() {
    // "e" + U+0301 combining acute — decomposed form.
    let bytes = patch_with_comment("cafe\u{0301}".as_bytes());
    let f = write_pd(&bytes);
    assert_structure_ok(f.path());
    assert_roundtrip_identical(&bytes, f.path());
}

#[test]
fn embedded_nul_byte_is_valid_utf8() {
    // A NUL (0x00) is a valid UTF-8 code point; a comment containing one must
    // still parse and round-trip.
    let bytes = patch_with_comment(b"before\x00after");
    let f = write_pd(&bytes);
    assert_structure_ok(f.path());
    assert_roundtrip_identical(&bytes, f.path());
}

// ---------------------------------------------------------------------------
// Graceful failure — a genuinely different encoding is not a PD patch. pdtk
// must reject it with a non-zero exit and NEVER panic.
// ---------------------------------------------------------------------------

#[test]
fn utf16le_with_bom_fails_gracefully() {
    // "#N canvas ...;" encoded as UTF-16LE with a BOM (FF FE ...).
    let mut bytes = vec![0xFF, 0xFE];
    for b in b"#N canvas 0 22 450 300 12;\n#X obj 20 50 dac~;\n" {
        bytes.push(*b);
        bytes.push(0x00);
    }
    let f = write_pd(&bytes);
    let out = run_pdtk(&["parse", f.path().to_str().unwrap()]);
    assert_ne!(out.status.code(), Some(0), "UTF-16 must be rejected");
    // No panic: a clean error, not a crash (SIGABRT would give None/136).
    assert!(matches!(out.status.code(), Some(2 | 3)));
}

// ---------------------------------------------------------------------------
// Fail-safe on mutation — until byte-preserving edit support exists, mutating
// a non-UTF-8 file must refuse rather than corrupt it.
// ---------------------------------------------------------------------------

#[test]
fn mutation_refused_on_non_utf8_file() {
    let bytes = patch_with_comment(b"divis\xe3o"); // Latin-1 (invalid UTF-8)
    let f = write_pd(&bytes);
    let out = run_pdtk(&[
        "insert",
        f.path().to_str().unwrap(),
        "--depth",
        "0",
        "--index",
        "0",
        "--entry",
        "#X obj 10 10 print;",
    ]);
    assert_ne!(
        out.status.code(),
        Some(0),
        "mutating a non-UTF-8 file must fail safe, not corrupt it"
    );
    // And the file must be untouched.
    assert_eq!(std::fs::read(f.path()).unwrap(), bytes);
}

// ---------------------------------------------------------------------------
// Pending Option E — invalid-UTF-8 files must parse *structurally* (the ASCII
// skeleton is intact) instead of hard-failing or being silently skipped.
// ---------------------------------------------------------------------------

#[test]
fn latin1_parses_structurally() {
    // ISO-8859-1: "divisão" with ã = 0xE3 (lone high byte).
    let f = write_pd(&patch_with_comment(b"divis\xe3o igual"));
    assert_structure_ok(f.path());
}

#[test]
fn windows1252_smart_punctuation_parses_structurally() {
    // CP-1252: “smart quotes” (0x93/0x94), em-dash (0x97), ellipsis (0x85),
    // bullet (0x95) — all invalid as UTF-8.
    let f = write_pd(&patch_with_comment(b"\x93quoted\x94 \x97 dash \x85 \x95"));
    assert_structure_ok(f.path());
}

#[test]
fn wtf8_surrogate_emoji_parses_structurally() {
    // CESU-8/WTF-8: 💩 stored as encoded UTF-16 surrogate pair
    // (ED A0 BD  ED B2 A9) — the real pd-else "Merda" case.
    let f = write_pd(&patch_with_comment(b"poop \xed\xa0\xbd\xed\xb2\xa9 here"));
    assert_structure_ok(f.path());
}

#[test]
fn lone_continuation_byte_parses_structurally() {
    // A stray continuation byte (0xBF) with no lead byte.
    let f = write_pd(&patch_with_comment(b"stray \xbf byte"));
    assert_structure_ok(f.path());
}

#[test]
fn truncated_multibyte_sequence_parses_structurally() {
    // A 2-byte lead (0xC3) with its continuation missing before the ';'.
    let f = write_pd(&patch_with_comment(b"truncated \xc3"));
    assert_structure_ok(f.path());
}

#[test]
fn overlong_encoding_parses_structurally() {
    // Overlong encoding of '/' (0xC0 0xAF) — invalid, sometimes emitted by
    // buggy exporters.
    let f = write_pd(&patch_with_comment(b"overlong \xc0\xaf"));
    assert_structure_ok(f.path());
}

#[test]
fn non_ascii_in_receive_name_parses() {
    // High byte inside an object argument (a receive name), not a comment:
    // `[r café]` with é = 0xE9. The class token stays ASCII and parses.
    let bytes = b"#N canvas 0 22 450 300 12;\n#X obj 20 50 r caf\xe9;\n#X obj 20 80 print;\n\
                  #X connect 0 0 1 0;\n"
        .to_vec();
    let f = write_pd(&bytes);
    let (code, v) = json_out(&["list", f.path().to_str().unwrap(), "--json"]);
    assert_eq!(code, 0);
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["class"], "r");
}

#[test]
fn mixed_utf8_and_latin1_in_one_file() {
    // One valid-UTF-8 comment and one Latin-1 comment in the same patch.
    let mut bytes = b"#N canvas 0 22 450 300 12;\n".to_vec();
    bytes.extend_from_slice("#X text 20 20 café utf8;\n".as_bytes());
    bytes.extend_from_slice(b"#X text 20 40 lat\xedn one;\n");
    bytes.extend_from_slice(b"#X obj 20 80 dac~;\n");
    let f = write_pd(&bytes);
    let (code, v) = json_out(&["list", f.path().to_str().unwrap(), "--json"]);
    assert_eq!(code, 0);
    assert_eq!(v.as_array().unwrap().len(), 3);
}

#[test]
fn deps_does_not_silently_skip_non_utf8_files() {
    // In directory mode, `deps` must include dependencies from a non-UTF-8
    // file, not silently drop it (which currently corrupts the report).
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("clean.pd"),
        b"#N canvas 0 22 450 300 12;\n#X obj 10 10 abstraction_a;\n",
    )
    .unwrap();
    // A Latin-1 comment + a reference to a different abstraction.
    std::fs::write(
        dir.path().join("dirty.pd"),
        b"#N canvas 0 22 450 300 12;\n#X text 10 10 lat\xedn;\n#X obj 10 40 abstraction_b;\n",
    )
    .unwrap();
    let out = run_pdtk(&["deps", dir.path().to_str().unwrap()]);
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("abstraction_a"), "clean file dep present: {s}");
    assert!(
        s.contains("abstraction_b"),
        "non-UTF-8 file must not be silently skipped: {s}"
    );
}

// ---------------------------------------------------------------------------
// Pending BOM handling — a UTF-8 BOM is valid UTF-8 but currently defeats the
// `#N canvas` header check.
// ---------------------------------------------------------------------------

#[test]
fn utf8_bom_before_canvas_is_tolerated() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(b"#N canvas 0 22 450 300 12;\n#X obj 20 50 dac~;\n");
    let f = write_pd(&bytes);
    let out = run_pdtk(&["parse", f.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "BOM must not defeat parsing");
}
