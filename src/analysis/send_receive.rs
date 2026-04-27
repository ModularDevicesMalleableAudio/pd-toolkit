use crate::model::{Entry, EntryKind, gui_send_receive_arg_indices, vu_receive_arg_index};
use std::collections::BTreeMap;

/// True if `class` is a send-like object class (writes to a named bus).
pub fn is_send_class(class: &str) -> bool {
    matches!(class, "s" | "send" | "s~" | "send~" | "throw~")
}

/// True if `class` is a receive-like object class (reads from a named bus).
pub fn is_receive_class(class: &str) -> bool {
    matches!(class, "r" | "receive" | "r~" | "receive~" | "catch~")
}

/// Location of a send/receive use: `(user_depth, object_index)`.
/// Note: `user_depth = entry.depth - 1` (entry.depth uses internal depth
/// where root canvas = 0, top-level objects = 1).
pub type Location = (usize, usize);

fn user_depth_of(entry: &Entry) -> Option<usize> {
    entry.depth.checked_sub(1)
}

fn maybe_insert(map: &mut BTreeMap<String, Vec<Location>>, name: &str, loc: Location) {
    if name == "empty" || name == "-" || name.is_empty() {
        return;
    }
    map.entry(name.to_string()).or_default().push(loc);
}

/// Collect all send-name uses (sender side) keyed by name → list of locations.
pub fn collect_sends(entries: &[Entry]) -> BTreeMap<String, Vec<Location>> {
    let mut out: BTreeMap<String, Vec<Location>> = BTreeMap::new();
    for e in entries {
        let Some(ud) = user_depth_of(e) else {
            continue;
        };
        let Some(oi) = e.object_index else {
            continue;
        };
        let loc = (ud, oi);
        let toks: Vec<&str> = e.raw.split_whitespace().collect();

        match e.kind {
            EntryKind::Obj => {
                let class = toks.get(4).copied().unwrap_or("").trim_end_matches(';');
                if is_send_class(class) {
                    if let Some(name) = toks.get(5) {
                        maybe_insert(&mut out, name.trim_end_matches(';'), loc);
                    }
                } else if let Some((si, _ri)) = gui_send_receive_arg_indices(class)
                    && let Some(tok) = toks.get(5 + si)
                {
                    maybe_insert(&mut out, tok.trim_end_matches(';'), loc);
                }
            }
            EntryKind::FloatAtom | EntryKind::SymbolAtom => {
                if let Some(tok) = toks.get(8) {
                    maybe_insert(&mut out, tok.trim_end_matches(';'), loc);
                }
            }
            _ => {}
        }
    }
    out
}

/// Collect all receive-name uses (receiver side) keyed by name → list of locations.
pub fn collect_receives(entries: &[Entry]) -> BTreeMap<String, Vec<Location>> {
    let mut out: BTreeMap<String, Vec<Location>> = BTreeMap::new();
    for e in entries {
        let Some(ud) = user_depth_of(e) else {
            continue;
        };
        let Some(oi) = e.object_index else {
            continue;
        };
        let loc = (ud, oi);
        let toks: Vec<&str> = e.raw.split_whitespace().collect();

        match e.kind {
            EntryKind::Obj => {
                let class = toks.get(4).copied().unwrap_or("").trim_end_matches(';');
                if is_receive_class(class) {
                    if let Some(name) = toks.get(5) {
                        maybe_insert(&mut out, name.trim_end_matches(';'), loc);
                    }
                } else if let Some((_si, ri)) = gui_send_receive_arg_indices(class)
                    && let Some(tok) = toks.get(5 + ri)
                {
                    maybe_insert(&mut out, tok.trim_end_matches(';'), loc);
                } else if class == "vu"
                    && let Some(tok) = toks.get(5 + vu_receive_arg_index())
                {
                    maybe_insert(&mut out, tok.trim_end_matches(';'), loc);
                }
            }
            EntryKind::FloatAtom | EntryKind::SymbolAtom => {
                if let Some(tok) = toks.get(9) {
                    maybe_insert(&mut out, tok.trim_end_matches(';'), loc);
                }
            }
            _ => {}
        }
    }
    out
}

/// Format a list of locations as `[d:i], [d:i], ...`.
pub fn format_locations(locs: &[Location]) -> String {
    let mut sorted = locs.to_vec();
    sorted.sort();
    sorted
        .iter()
        .map(|(d, i)| format!("[{d}:{i}]"))
        .collect::<Vec<_>>()
        .join(", ")
}
