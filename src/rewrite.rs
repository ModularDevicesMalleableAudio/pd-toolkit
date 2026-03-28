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
