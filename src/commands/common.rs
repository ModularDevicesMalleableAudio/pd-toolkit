use pdtk::model::{Entry, EntryKind, Patch};
use pdtk::parser::assign_depth_and_indices;
use std::collections::HashMap;

/// Whether an entry is an `array define` / `array d` object, which can carry
/// `#A` saved data.
pub(crate) fn is_array_define(entry: &Entry) -> bool {
    entry.kind == EntryKind::Obj
        && entry.class() == "array"
        && matches!(
            entry.args().first().map(String::as_str),
            Some("define" | "d")
        )
}

/// Detect `#A` array-data records that are not attached to an array
/// definition. In PD's file format an `#A` record binds to the most recent
/// array (`#X array` or `array define`); a record whose immediately preceding
/// entry is neither an array definition nor another `#A` is orphaned, so its
/// data will be lost or bound to the wrong array. Returns human-readable
/// messages (empty = no problems found).
pub fn detached_array_data(patch: &Patch) -> Vec<String> {
    let mut msgs = Vec::new();
    for (i, e) in patch.entries.iter().enumerate() {
        if e.kind != EntryKind::ArrayData {
            continue;
        }
        let attached = i.checked_sub(1).is_some_and(|p| {
            let prev = &patch.entries[p];
            matches!(prev.kind, EntryKind::ArrayData | EntryKind::Array) || is_array_define(prev)
        });
        if !attached {
            msgs.push(format!(
                "entry {i}: #A array data not attached to an array definition: {}",
                e.raw.trim()
            ));
        }
    }
    msgs
}

/// Post-mutation validation: checks that all connection src/dst indices are
/// in range for their own canvas.  Sibling subpatches at the same depth have
/// independent index spaces, so counts are per `canvas_id`, not per depth.
/// Returns a list of error strings (empty = valid).
pub fn validate_patch(patch: &Patch) -> Vec<String> {
    let mut errors = Vec::new();

    let mut counts_by_canvas: HashMap<usize, usize> = HashMap::new();
    for e in &patch.entries {
        if e.object_index.is_some()
            && let Some(cid) = e.canvas_id
        {
            *counts_by_canvas.entry(cid).or_insert(0) += 1;
        }
    }

    for e in &patch.entries {
        if e.kind != EntryKind::Connect {
            continue;
        }

        let parts: Vec<&str> = e
            .raw
            .trim()
            .trim_end_matches(';')
            .split_whitespace()
            .collect();
        if parts.len() != 6 {
            continue;
        }

        if let (Ok(src), Ok(dst)) = (parts[2].parse::<usize>(), parts[4].parse::<usize>()) {
            let cid = e.canvas_id.unwrap_or(usize::MAX);
            let count = counts_by_canvas.get(&cid).copied().unwrap_or(0);
            let user_depth = e.depth.saturating_sub(1);
            if src >= count {
                errors.push(format!(
                    "depth {user_depth}: connect src {src} out of range (object count {count})"
                ));
            }
            if dst >= count {
                errors.push(format!(
                    "depth {user_depth}: connect dst {dst} out of range (object count {count})"
                ));
            }
        }
    }

    errors
}

/// Delete one object at `index` inside `canvas_id` from a raw entry list,
/// including connection cleanup and renumbering within that canvas.
pub fn delete_object(entries: &mut Vec<Entry>, canvas_id: usize, index: usize) -> bool {
    let Some(pos) = entries
        .iter()
        .position(|e| e.canvas_id == Some(canvas_id) && e.object_index == Some(index))
    else {
        return false;
    };

    entries.remove(pos);

    // Remove touched connections
    entries.retain(|e| {
        if e.kind != EntryKind::Connect || e.canvas_id != Some(canvas_id) {
            return true;
        }
        let parts: Vec<&str> = e
            .raw
            .trim()
            .trim_end_matches(';')
            .split_whitespace()
            .collect();
        if parts.len() != 6 {
            return true;
        }
        let src = parts[2].parse::<usize>().unwrap_or(usize::MAX);
        let dst = parts[4].parse::<usize>().unwrap_or(usize::MAX);
        src != index && dst != index
    });

    // Renumber remaining connections
    for e in entries.iter_mut() {
        if e.kind != EntryKind::Connect || e.canvas_id != Some(canvas_id) {
            continue;
        }
        let parts: Vec<&str> = e
            .raw
            .trim()
            .trim_end_matches(';')
            .split_whitespace()
            .collect();
        if parts.len() != 6 || parts[0] != "#X" || parts[1] != "connect" {
            continue;
        }
        let Ok(mut src) = parts[2].parse::<usize>() else {
            continue;
        };
        let Ok(outlet) = parts[3].parse::<usize>() else {
            continue;
        };
        let Ok(mut dst) = parts[4].parse::<usize>() else {
            continue;
        };
        let Ok(inlet) = parts[5].parse::<usize>() else {
            continue;
        };

        if src > index {
            src -= 1;
        }
        if dst > index {
            dst -= 1;
        }
        e.raw = format!("#X connect {src} {outlet} {dst} {inlet};");
    }

    assign_depth_and_indices(entries);
    true
}

/// Position at which a new object taking parent index `index` must be
/// inserted into `canvas_id`.
///
/// Three cases: inserting before an existing object (a restore box is
/// backtracked to the start of its whole `#N canvas … #X restore` span, so
/// the new entry lands beside the subpatch, not inside it); appending after
/// the last object and any tail entries glued to it (`#A` array data,
/// `#X f N` width hints); and appending into a canvas with no objects
/// (immediately before the canvas's closing restore — never the end of the
/// file, which belongs to the root canvas).
pub fn object_insert_pos(entries: &[Entry], canvas_id: usize, index: usize) -> usize {
    let existing = entries
        .iter()
        .position(|e| e.canvas_id == Some(canvas_id) && e.object_index.is_some_and(|i| i >= index));
    if let Some(pos) = existing {
        return object_span_start(entries, pos);
    }

    match entries
        .iter()
        .rposition(|e| e.canvas_id == Some(canvas_id) && e.object_index.is_some())
    {
        Some(pos) => {
            let mut after = pos + 1;
            while after < entries.len()
                && entries[after].canvas_id == Some(canvas_id)
                && matches!(
                    entries[after].kind,
                    EntryKind::ArrayData | EntryKind::WidthHint
                )
            {
                after += 1;
            }
            after
        }
        None => canvas_close_pos(entries, canvas_id),
    }
}

/// The start of the entry span occupied by the object at `pos`: for a restore
/// box, the matching `#N canvas` that opens it; otherwise `pos` itself.
fn object_span_start(entries: &[Entry], pos: usize) -> usize {
    if entries[pos].kind != EntryKind::Restore {
        return pos;
    }
    let mut balance = 0usize;
    for i in (0..pos).rev() {
        match entries[i].kind {
            EntryKind::Restore => balance += 1,
            EntryKind::CanvasOpen => {
                if balance == 0 {
                    return i;
                }
                balance -= 1;
            }
            _ => {}
        }
    }
    pos
}

/// The position of the `#X restore` that closes `canvas_id`, or the end of
/// the entry list for the root (unclosed) canvas.
fn canvas_close_pos(entries: &[Entry], canvas_id: usize) -> usize {
    let Some(open_pos) = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind == EntryKind::CanvasOpen)
        .map(|(i, _)| i)
        .nth(canvas_id)
    else {
        return entries.len();
    };
    let mut balance = 0usize;
    for (i, e) in entries.iter().enumerate().skip(open_pos + 1) {
        match e.kind {
            EntryKind::CanvasOpen => balance += 1,
            EntryKind::Restore => {
                if balance == 0 {
                    return i;
                }
                balance -= 1;
            }
            _ => {}
        }
    }
    entries.len()
}
