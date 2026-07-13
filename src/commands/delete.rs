use crate::commands::common::{is_array_define, validate_patch};
use crate::errors::PdtkError;
use crate::io;
use pdtk::model::EntryKind;
use pdtk::parser::{assign_depth_and_indices, build_entries, tokenize_entries};
use pdtk::rewrite::serialize;

/// Run the `delete` command.
///
/// When `subpatch` is true, `depth` identifies the subpatch's own canvas
/// depth (matching `extract --depth`) and `index` selects the Nth subpatch
/// at that depth (default 0). The full span from `#N canvas` through
/// `#X restore` (and any trailing `#X f N` width hint) is removed, and
/// parent connections referencing the restore are filtered/renumbered.
///
/// When `subpatch` is false, behavior is the existing "delete object at
/// `--depth D --index I`" semantics.
#[allow(clippy::too_many_arguments)]
pub fn run(
    file: &str,
    depth: usize,
    canvas: usize,
    index: Option<usize>,
    subpatch: bool,
    in_place: bool,
    backup: bool,
    output: Option<&str>,
) -> Result<(String, i32), PdtkError> {
    let input = io::read_patch_file(file)?;
    let tok = tokenize_entries(&input);
    let mut entries = build_entries(&tok.entries);

    let (span_start, delete_pos, target_index, conn_canvas_id) = if subpatch {
        if depth == 0 {
            return Err(PdtkError::Usage(
                "cannot delete the root canvas".to_string(),
            ));
        }
        find_subpatch_span(&entries, depth, index.unwrap_or(0))?
    } else {
        let idx = index.ok_or_else(|| {
            PdtkError::Usage("--index is required unless --subpatch is given".to_string())
        })?;
        find_object_span(&entries, depth, canvas, idx)?
    };

    // Drain trailing WidthHint after the deleted span (applies to both
    // paths: an `#X f N` immediately following the restore belongs to
    // it and must be removed too).
    let mut end_inclusive = if delete_pos + 1 < entries.len()
        && entries[delete_pos + 1].kind == EntryKind::WidthHint
        && entries[delete_pos].kind == EntryKind::Restore
        && entries[delete_pos + 1].depth == entries[delete_pos].depth
    {
        delete_pos + 1
    } else {
        delete_pos
    };

    // `#A` records bind to the immediately preceding array definition
    // (classic `#X array` or `array define`); leaving them behind detaches
    // the saved samples, so they are deleted with the array.
    if entries[delete_pos].kind == EntryKind::Array || is_array_define(&entries[delete_pos]) {
        while end_inclusive + 1 < entries.len()
            && entries[end_inclusive + 1].kind == EntryKind::ArrayData
        {
            end_inclusive += 1;
        }
    }

    entries.drain(span_start..=end_inclusive);

    // Remove connections in the target canvas that reference target_index
    entries.retain(|e| {
        if e.kind != EntryKind::Connect || e.canvas_id != Some(conn_canvas_id) {
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
        src != target_index && dst != target_index
    });

    // Renumber: src/dst > target_index → -1 (within the target canvas)
    for e in &mut *entries {
        if e.kind != EntryKind::Connect || e.canvas_id != Some(conn_canvas_id) {
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
        if src > target_index {
            src -= 1;
        }
        if dst > target_index {
            dst -= 1;
        }
        e.raw = format!("#X connect {src} {outlet} {dst} {inlet};");
    }

    assign_depth_and_indices(&mut entries);

    let patch = pdtk::model::Patch {
        entries,
        warnings: Vec::new(),
    };
    let serialized = serialize(&patch);

    let errors = validate_patch(&pdtk::parser::parse(&serialized)?);
    if !errors.is_empty() {
        return Err(PdtkError::Usage(format!(
            "validation failed after delete: {}",
            errors.join("; ")
        )));
    }

    if in_place {
        io::write_with_backup(file, &serialized, backup)?;
    } else if let Some(out_path) = output {
        io::write_patch_file(out_path, &serialized)?;
    }

    Ok((serialized, 0))
}

/// Find the span for the default (non-subpatch) delete path.
/// Returns `(span_start, delete_pos, target_index, conn_canvas_id)`, where
/// `conn_canvas_id` is the canvas whose connections must be adjusted.
fn find_object_span(
    entries: &[pdtk::model::Entry],
    user_depth: usize,
    canvas: usize,
    index: usize,
) -> Result<(usize, usize, usize, usize), PdtkError> {
    let canvas_id =
        pdtk::model::resolve_canvas_id(entries, user_depth, canvas).ok_or_else(|| {
            PdtkError::Usage(format!(
                "no canvas {canvas} at depth {user_depth} ({} at this depth)",
                pdtk::model::canvas_ids_at_depth(entries, user_depth).len()
            ))
        })?;
    let delete_pos = entries
        .iter()
        .position(|e| e.canvas_id == Some(canvas_id) && e.object_index == Some(index))
        .ok_or_else(|| {
            PdtkError::Usage(format!(
                "no object at depth {user_depth}, canvas {canvas}, index {index}"
            ))
        })?;

    let span_start = if entries[delete_pos].kind == EntryKind::Restore {
        let mut balance: i32 = 0;
        let mut canvas_pos = None;
        for i in (0..delete_pos).rev() {
            match entries[i].kind {
                EntryKind::Restore => balance += 1,
                EntryKind::CanvasOpen => {
                    if balance == 0 {
                        canvas_pos = Some(i);
                        break;
                    }
                    balance -= 1;
                }
                _ => {}
            }
        }
        canvas_pos.ok_or_else(|| {
            PdtkError::Usage(format!(
                "no matching #N canvas found for restore at depth {user_depth}, index {index}"
            ))
        })?
    } else {
        delete_pos
    };

    // The connections to adjust live in the deleted entry's own canvas
    // (its parent canvas when the target is a restore box).
    let conn_canvas_id = entries[delete_pos].canvas_id.ok_or_else(|| {
        PdtkError::Usage(format!(
            "object at depth {user_depth}, canvas {canvas}, index {index} has no canvas id"
        ))
    })?;
    Ok((span_start, delete_pos, index, conn_canvas_id))
}

/// Find the span for the `--subpatch` path.
/// Returns `(span_start, delete_pos, target_index, conn_canvas_id)`.
fn find_subpatch_span(
    entries: &[pdtk::model::Entry],
    user_depth: usize,
    n: usize,
) -> Result<(usize, usize, usize, usize), PdtkError> {
    // CanvasOpens whose own internal depth equals user_depth.
    let candidates: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            (e.kind == EntryKind::CanvasOpen && e.depth == user_depth).then_some(i)
        })
        .collect();

    let canvas_pos = *candidates.get(n).ok_or_else(|| {
        PdtkError::Usage(format!(
            "no subpatch found at depth {user_depth} index {n} ({} subpatches at this depth)",
            candidates.len()
        ))
    })?;

    // Walk forward, balancing CanvasOpen/Restore, to find the matching restore.
    let mut balance: i32 = 0;
    let mut restore_pos = None;
    for (i, e) in entries.iter().enumerate().skip(canvas_pos) {
        match e.kind {
            EntryKind::CanvasOpen => balance += 1,
            EntryKind::Restore => {
                balance -= 1;
                if balance == 0 {
                    restore_pos = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let restore_pos = restore_pos.ok_or_else(|| {
        PdtkError::Usage(format!(
            "subpatch at depth {user_depth} index {n} has no matching #X restore"
        ))
    })?;

    let restore = &entries[restore_pos];
    let target_index = restore.object_index.ok_or_else(|| {
        PdtkError::Usage(format!(
            "restore at depth {user_depth} index {n} has no object_index"
        ))
    })?;
    // Parent connections that reference the restore box live in the restore's
    // own (parent) canvas.
    let conn_canvas_id = restore.canvas_id.ok_or_else(|| {
        PdtkError::Usage(format!(
            "restore at depth {user_depth} index {n} has no canvas id"
        ))
    })?;

    Ok((canvas_pos, restore_pos, target_index, conn_canvas_id))
}
