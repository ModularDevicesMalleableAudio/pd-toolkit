use crate::model::Patch;

/// Serialize a `Patch` back to its `.pd` text representation.
///
/// The output is guaranteed to be byte-identical to the original input
/// provided that:
/// - the input was well-formed (all entries terminated, Unix line endings)
/// - no entries have been mutated
///
/// Each entry is written on its own "block" (multi-line entries preserve their
/// internal newlines).  Entries are separated by a single `\n`, and the file
/// ends with a trailing `\n`.
///
/// Trailing-newline normalization: the output always ends with exactly one
/// `\n`, matching Pd's own writer (`binbuf_write` emits `\n` after every
/// terminating `;`). An input that is missing its final newline is therefore
/// normalized to include one — the single intentional exception to byte-exact
/// round-tripping, and a move toward Pd-canonical form rather than away from it.
#[must_use]
pub fn serialize(patch: &Patch) -> String {
    let mut out = String::new();
    for (i, entry) in patch.entries.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&entry.raw);
    }
    out.push('\n');
    out
}
