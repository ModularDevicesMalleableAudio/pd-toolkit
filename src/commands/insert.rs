use crate::commands::common::{object_insert_pos, validate_patch};
use crate::errors::PdtkError;
use crate::io;
use pdtk::model::{Entry, EntryKind};
use pdtk::parser::{assign_depth_and_indices, build_entries, classify_entry, tokenize_entries};
use pdtk::rewrite::serialize;

#[allow(clippy::too_many_arguments)]
pub fn run(
    file: &str,
    depth: usize,
    canvas: usize,
    index: usize,
    entry: &str,
    in_place: bool,
    backup: bool,
    output: Option<&str>,
) -> Result<(String, i32), PdtkError> {
    let input = io::read_patch_file(file)?;
    let tok = tokenize_entries(&input);
    let mut entries = build_entries(&tok.entries);

    let internal_depth = depth + 1;

    // Resolve the target canvas among siblings at this depth.
    let canvas_id = pdtk::model::resolve_canvas_id(&entries, depth, canvas).ok_or_else(|| {
        PdtkError::Usage(format!(
            "no canvas {canvas} at depth {depth} ({} at this depth)",
            pdtk::model::canvas_ids_at_depth(&entries, depth).len()
        ))
    })?;

    // Validate: index is in range for the target canvas
    let obj_count = entries
        .iter()
        .filter(|e| e.canvas_id == Some(canvas_id) && e.object_index.is_some())
        .count();

    if index > obj_count {
        return Err(PdtkError::Usage(format!(
            "index {index} out of range for depth {depth}, canvas {canvas} (object count: {obj_count})"
        )));
    }

    // Create the new entry
    let new_entry = Entry {
        raw: entry.to_string(),
        kind: classify_entry(entry),
        depth: internal_depth,
        object_index: Some(index),
        canvas_id: Some(canvas_id),
    };

    let insert_pos = object_insert_pos(&entries, canvas_id, index);

    entries.insert(insert_pos, new_entry);

    // Renumber connections in this canvas: src/dst >= original index get +1
    for e in &mut entries {
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

        if src >= index {
            src += 1;
        }
        if dst >= index {
            dst += 1;
        }

        e.raw = format!("#X connect {src} {outlet} {dst} {inlet};");
    }

    // Re-assign depth/index to make the model consistent
    assign_depth_and_indices(&mut entries);

    let patch = pdtk::model::Patch {
        entries,
        warnings: Vec::new(),
    };
    let serialized = serialize(&patch);

    // Validate before writing
    let errors = validate_patch(&pdtk::parser::parse(&serialized)?);
    if !errors.is_empty() {
        return Err(PdtkError::Usage(format!(
            "validation failed after insert: {}",
            errors.join("; ")
        )));
    }

    // Write
    if in_place {
        io::write_with_backup(file, &serialized, backup)?;
    } else if let Some(out_path) = output {
        io::write_patch_file(out_path, &serialized)?;
    }

    Ok((serialized, 0))
}
