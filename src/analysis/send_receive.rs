use crate::model::{
    Entry, EntryKind, gui_send_receive_arg_indices, message_send_targets, vu_receive_arg_index,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Identifies one of the three disjoint PD bus namespaces. `[s foo]` and
/// `[s~ foo]` and `[throw~ foo]` live in three separate symbol tables and
/// never route to each other at runtime, so static analysis must distinguish
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BusKind {
    /// Control-rate bus: `s`/`send`/`r`/`receive`, plus all GUI send/receive
    /// fields (which carry control messages).
    Control,
    /// Signal-rate bus: `s~`/`send~`/`r~`/`receive~`.
    Signal,
    /// Audio-sum bus: `throw~`/`catch~` (multiple `throw~`s sum into one
    /// `catch~`).
    AudioSum,
}

/// Returns the bus namespace this class writes to as a sender, or `None`
/// if not a sender class.
#[must_use]
pub fn send_bus_kind(class: &str) -> Option<BusKind> {
    match class {
        "s" | "send" => Some(BusKind::Control),
        "s~" | "send~" => Some(BusKind::Signal),
        "throw~" => Some(BusKind::AudioSum),
        _ => None,
    }
}

/// Returns the bus namespace this class reads from as a receiver, or `None`
/// if not a receiver class.
#[must_use]
pub fn receive_bus_kind(class: &str) -> Option<BusKind> {
    match class {
        "r" | "receive" => Some(BusKind::Control),
        "r~" | "receive~" => Some(BusKind::Signal),
        "catch~" => Some(BusKind::AudioSum),
        _ => None,
    }
}

/// True if `class` writes to a named bus (any kind).
#[must_use]
pub fn is_send_class(class: &str) -> bool {
    send_bus_kind(class).is_some()
}

/// True if `class` reads from a named bus (any kind).
#[must_use]
pub fn is_receive_class(class: &str) -> bool {
    receive_bus_kind(class).is_some()
}

/// Location of a send/receive use: `(user_depth, object_index, canvas_id)`.
/// `canvas_id` distinguishes sibling subpatches at the same depth so bus
/// matching can be scoped per canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    pub depth: usize,
    pub index: usize,
    pub canvas_id: usize,
}

impl Location {
    #[must_use]
    pub fn new(depth: usize, index: usize, canvas_id: usize) -> Self {
        Self {
            depth,
            index,
            canvas_id,
        }
    }
}

/// Key for the send/receive maps: `(BusKind, name)`. Names within different
/// kinds are distinct.
pub type BusKey = (BusKind, String);

fn user_depth_of(entry: &Entry) -> Option<usize> {
    entry.depth.checked_sub(1)
}

fn maybe_insert(
    map: &mut BTreeMap<BusKey, Vec<Location>>,
    kind: BusKind,
    name: &str,
    loc: Location,
) {
    if name == "empty" || name == "-" || name.is_empty() {
        return;
    }
    map.entry((kind, name.to_string())).or_default().push(loc);
}

fn entry_location(entry: &Entry) -> Option<Location> {
    let ud = user_depth_of(entry)?;
    let oi = entry.object_index?;
    let cid = entry.canvas_id?;
    Some(Location::new(ud, oi, cid))
}

/// Collect all sender uses keyed by `(BusKind, name)` → list of locations.
///
/// Returns a separate row per occurrence: a `[s foo]` and a `[tgl ... send=foo]`
/// at different indices both contribute to `(Control, "foo")`.
#[must_use]
pub fn collect_sends(entries: &[Entry]) -> BTreeMap<BusKey, Vec<Location>> {
    let mut out: BTreeMap<BusKey, Vec<Location>> = BTreeMap::new();
    for e in entries {
        let Some(loc) = entry_location(e) else {
            continue;
        };
        let toks: Vec<&str> = e.raw.split_whitespace().collect();

        match e.kind {
            EntryKind::Obj => {
                let class = toks.get(4).copied().unwrap_or("").trim_end_matches(';');
                if let Some(kind) = send_bus_kind(class) {
                    if let Some(name) = toks.get(5) {
                        maybe_insert(&mut out, kind, name.trim_end_matches(';'), loc);
                    }
                } else if let Some((si, _ri)) = gui_send_receive_arg_indices(class)
                    && let Some(tok) = toks.get(5 + si)
                {
                    // GUI send fields always carry control messages.
                    maybe_insert(&mut out, BusKind::Control, tok.trim_end_matches(';'), loc);
                }
            }
            EntryKind::FloatAtom | EntryKind::SymbolAtom | EntryKind::ListAtom => {
                if let Some(tok) = toks.get(8) {
                    maybe_insert(&mut out, BusKind::Control, tok.trim_end_matches(';'), loc);
                }
            }
            EntryKind::Msg => {
                // Message boxes send to named receivers via `\;`-introduced
                // sub-messages. These always carry control messages. Engine /
                // canvas control targets (`pd`, `pd-<name>`) are not user buses
                // and would only create false orphan-send noise, so skip them.
                for (_pos, name) in message_send_targets(&e.raw) {
                    if name == "pd" || name.starts_with("pd-") {
                        continue;
                    }
                    maybe_insert(&mut out, BusKind::Control, &name, loc);
                }
            }
            _ => {}
        }
    }
    out
}

/// Collect all receiver uses keyed by `(BusKind, name)` → list of locations.
#[must_use]
pub fn collect_receives(entries: &[Entry]) -> BTreeMap<BusKey, Vec<Location>> {
    let mut out: BTreeMap<BusKey, Vec<Location>> = BTreeMap::new();
    for e in entries {
        let Some(loc) = entry_location(e) else {
            continue;
        };
        let toks: Vec<&str> = e.raw.split_whitespace().collect();

        match e.kind {
            EntryKind::Obj => {
                let class = toks.get(4).copied().unwrap_or("").trim_end_matches(';');
                if let Some(kind) = receive_bus_kind(class) {
                    if let Some(name) = toks.get(5) {
                        maybe_insert(&mut out, kind, name.trim_end_matches(';'), loc);
                    }
                } else if let Some((_si, ri)) = gui_send_receive_arg_indices(class)
                    && let Some(tok) = toks.get(5 + ri)
                {
                    maybe_insert(&mut out, BusKind::Control, tok.trim_end_matches(';'), loc);
                } else if class == "vu"
                    && let Some(tok) = toks.get(5 + vu_receive_arg_index())
                {
                    maybe_insert(&mut out, BusKind::Control, tok.trim_end_matches(';'), loc);
                }
            }
            EntryKind::FloatAtom | EntryKind::SymbolAtom | EntryKind::ListAtom => {
                if let Some(tok) = toks.get(9) {
                    maybe_insert(&mut out, BusKind::Control, tok.trim_end_matches(';'), loc);
                }
            }
            _ => {}
        }
    }
    out
}

/// True if a bus name should carry a `$0-scoped` warning (i.e. its scope is
/// instance-local in PD's runtime, so static name matching may produce
/// false positives across subpatch instances).
#[must_use]
pub fn is_dollar_zero_scoped(name: &str) -> bool {
    name.starts_with("$0-") || name.starts_with("\\$0-")
}

/// Format a list of locations as `[d:i], [d:i], ...` (canvas_id elided).
#[must_use]
pub fn format_locations(locs: &[Location]) -> String {
    let mut sorted = locs.to_vec();
    sorted.sort();
    sorted
        .iter()
        .map(|l| format!("[{}:{}]", l.depth, l.index))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn keys(map: &BTreeMap<BusKey, Vec<Location>>) -> Vec<(BusKind, String)> {
        map.keys().cloned().collect()
    }

    #[test]
    fn collect_sends_includes_message_box_targets() {
        // A message box driving [r pitch] via `\; pitch 60` is a real sender,
        // matched to the `[r]` on the control bus.
        let input = "#N canvas 0 22 450 300 12;\n\
                     #X msg 10 10 \\; pitch 60 \\; velocity 100;\n\
                     #X obj 10 60 r pitch;\n";
        let patch = parse(input).unwrap();
        let sends = collect_sends(&patch.entries);
        assert!(sends.contains_key(&(BusKind::Control, "pitch".to_string())));
        assert!(sends.contains_key(&(BusKind::Control, "velocity".to_string())));
        // The message box's outlet message is not a named send.
        assert_eq!(sends.len(), 2);
    }

    #[test]
    fn collect_sends_skips_engine_message_targets() {
        // `pd`/`pd-<name>` are engine/canvas control receivers, not user buses.
        let input = "#N canvas 0 22 450 300 12;\n\
                     #X msg 10 10 \\; pd dsp 1;\n\
                     #X msg 10 40 \\; pd-sub clear;\n\
                     #X msg 10 70 \\; realbus go;\n";
        let patch = parse(input).unwrap();
        let sends = collect_sends(&patch.entries);
        assert_eq!(
            keys(&sends),
            vec![(BusKind::Control, "realbus".to_string())]
        );
    }

    #[test]
    fn collect_sends_message_target_matches_object_send() {
        // Both a `[s foo]` object and a `\; foo` message contribute to `foo`.
        let input = "#N canvas 0 22 450 300 12;\n\
                     #X obj 10 10 s foo;\n\
                     #X msg 10 40 \\; foo 1;\n";
        let patch = parse(input).unwrap();
        let sends = collect_sends(&patch.entries);
        let locs = &sends[&(BusKind::Control, "foo".to_string())];
        assert_eq!(locs.len(), 2, "object send + message send both counted");
    }

    #[test]
    fn collect_receives_ignores_message_boxes() {
        // Message boxes only send; they never receive on a named bus.
        let input = "#N canvas 0 22 450 300 12;\n\
                     #X msg 10 10 \\; foo 1;\n";
        let patch = parse(input).unwrap();
        assert!(collect_receives(&patch.entries).is_empty());
    }
}
