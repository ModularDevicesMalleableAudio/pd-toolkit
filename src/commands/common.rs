use crate::errors::PdtkError;
use pdtk::model::{Entry, EntryKind, Patch};
use pdtk::parser::{TokenizeWarning, assign_depth_and_indices, tokenize_entries};

/// Check that `raw` is exactly one `;`-terminated entry before it is spliced
/// into a patch.  An entry whose final `;` is escaped (`\;`) or missing would
/// run on into the next line and silently absorb the following record.
pub fn ensure_single_entry(raw: &str) -> Result<(), PdtkError> {
    let tok = tokenize_entries(raw);
    if tok.warnings.contains(&TokenizeWarning::UnterminatedEntry) {
        return Err(PdtkError::Usage(format!(
            "entry is not terminated by an unescaped ';' (a trailing \\; is message \
             content, not a terminator): {raw}"
        )));
    }
    if tok.entries.len() != 1 {
        return Err(PdtkError::Usage(format!(
            "expected exactly one entry, found {}: {raw}",
            tok.entries.len()
        )));
    }
    Ok(())
}

/// Post-mutation validation: checks that all connection src/dst indices are
/// in range for their own canvas.  Returns a list of error strings (empty =
/// valid).
pub fn validate_patch(patch: &Patch) -> Vec<String> {
    pdtk::analysis::check::connection_range_errors(patch)
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
