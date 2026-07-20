pub mod classify;
pub mod escape;
pub mod index;
pub mod tokenizer;

pub use classify::classify_entry;
pub use index::{assign_depth_and_indices, build_entries};
pub use tokenizer::{TokenizeResult, TokenizeWarning, tokenize_entries};

use crate::model::{EntryKind, ParseError, ParseWarning, Patch};

/// Decode raw `.pd` file bytes into a `String` for parsing, tolerating
/// non-UTF-8 content: invalid byte sequences become U+FFFD
/// (`String::from_utf8_lossy`). PD's structural syntax is ASCII, so the result
/// always parses structurally; only non-ASCII comment/label text is lossy.
///
/// Because lossy decoding is not reversible, this is for read-only analysis
/// only — code that writes a file back must read strictly (mutating commands
/// refuse non-UTF-8 rather than corrupt it).
#[must_use]
pub fn decode_lenient(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Parse a Pure Data `.pd` file into a `Patch`.
///
/// Returns `Err` only for hard structural failures (empty input, missing
/// canvas header).  Soft issues (e.g. unterminated entries) are recorded in
/// `Patch::warnings`.
pub fn parse(input: &str) -> Result<Patch, ParseError> {
    // Tolerate a leading UTF-8 BOM (some editors prepend `EF BB BF`), which
    // would otherwise defeat the `#N canvas` header check.
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    if input.trim().is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let tok = tokenize_entries(input);
    let entries = build_entries(&tok.entries);

    // A valid patch contains a root `#N canvas`, but real Pd files may write
    // one or more `#N struct` data-structure template definitions before it
    // (see g_readwrite.c canvas_savetemplatesto). Accept leading templates;
    // reject a file with no canvas, or with any non-template entry before the
    // root canvas.
    match entries.iter().position(|e| e.kind == EntryKind::CanvasOpen) {
        None => return Err(ParseError::MissingCanvasHeader),
        Some(root) => {
            if entries[..root].iter().any(|e| e.kind != EntryKind::Struct) {
                return Err(ParseError::MissingCanvasHeader);
            }
        }
    }

    let warnings: Vec<ParseWarning> = tok.warnings.into_iter().map(Into::into).collect();
    Ok(Patch { entries, warnings })
}
