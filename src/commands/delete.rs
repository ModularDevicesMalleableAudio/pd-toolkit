use crate::commands::common::validate_patch;
use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::model::EntryKind;
use pd_toolkit::parser::{assign_depth_and_indices, build_entries, tokenize_entries};
use pd_toolkit::rewrite::serialize;

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
pub fn run(
    file: &str,
    depth: usize,
    index: Option<usize>,
    subpatch: bool,
    in_place: bool,
    backup: bool,
    output: Option<&str>,
) -> Result<(String, i32), PdtkError> {
    let input = io::read_patch_file(file)?;
    let tok = tokenize_entries(&input);
    let mut entries = build_entries(&tok.entries);

    let (span_start, delete_pos, target_index, connection_depth) = if subpatch {
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
        find_object_span(&entries, depth, idx)?
    };

    // Drain trailing WidthHint after the deleted span (applies to both
    // paths: an `#X f N` immediately following the restore belongs to
    // it and must be removed too).
    let end_inclusive = if delete_pos + 1 < entries.len()
        && entries[delete_pos + 1].kind == EntryKind::WidthHint
        && entries[delete_pos].kind == EntryKind::Restore
        && entries[delete_pos + 1].depth == entries[delete_pos].depth
    {
        delete_pos + 1
    } else {
        delete_pos
    };

    entries.drain(span_start..=end_inclusive);

    // Remove parent-depth connections that reference target_index
    entries.retain(|e| {
        if e.kind != EntryKind::Connect || e.depth != connection_depth {
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

    // Renumber: src/dst > target_index → -1
    for e in entries.iter_mut() {
        if e.kind != EntryKind::Connect || e.depth != connection_depth {
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

    let patch = pd_toolkit::model::Patch {
        entries,
        warnings: Vec::new(),
    };
    let serialized = serialize(&patch);

    let errors = validate_patch(&pd_toolkit::parser::parse(&serialized)?);
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
/// Returns `(span_start, delete_pos, target_index, connection_depth)`.
fn find_object_span(
    entries: &[pd_toolkit::model::Entry],
    user_depth: usize,
    index: usize,
) -> Result<(usize, usize, usize, usize), PdtkError> {
    let internal_depth = user_depth + 1;
    let delete_pos = entries
        .iter()
        .position(|e| e.depth == internal_depth && e.object_index == Some(index))
        .ok_or_else(|| {
            PdtkError::Usage(format!("no object at depth {user_depth}, index {index}"))
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

    // connection_depth = target entry's depth (for both Restore and
    // non-Restore targets, this equals user_depth + 1 in this path).
    let connection_depth = entries[delete_pos].depth;
    Ok((span_start, delete_pos, index, connection_depth))
}

/// Find the span for the `--subpatch` path.
/// Returns `(span_start, delete_pos, target_index, connection_depth)`.
fn find_subpatch_span(
    entries: &[pd_toolkit::model::Entry],
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
    let connection_depth = restore.depth;

    Ok((canvas_pos, restore_pos, target_index, connection_depth))
}
