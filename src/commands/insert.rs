use crate::commands::common::validate_patch;
use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::model::{Entry, EntryKind};
use pd_toolkit::parser::{assign_depth_and_indices, build_entries, classify_entry, tokenize_entries};
use pd_toolkit::rewrite::serialize;

pub fn run(
    file: &str,
    depth: usize,
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

    // Validate: check the target depth exists and index is in range
    let obj_count = entries
        .iter()
        .filter(|e| e.depth == internal_depth && e.object_index.is_some())
        .count();

    if index > obj_count {
        return Err(PdtkError::Usage(format!(
            "index {index} out of range for depth {depth} (object count: {obj_count})"
        )));
    }

    // Create the new entry
    let new_entry = Entry {
        raw: entry.to_string(),
        kind: classify_entry(entry),
        depth: internal_depth,
        object_index: Some(index),
    };

    // Find insertion position: before the first entry at this depth with
    // object_index >= index.  If inserting at the end, place after all objects
    // at this depth (but before connections at this depth).
    let insert_pos = if index < obj_count {
        // Insert before the existing object at this index
        entries
            .iter()
            .position(|e| {
                e.depth == internal_depth && e.object_index.is_some_and(|idx| idx >= index)
            })
            .unwrap_or(entries.len())
    } else {
        // Appending: find the last object at this depth and insert after it.
        // We want to be after all objects but before connections at this depth.
        let last_obj = entries
            .iter()
            .rposition(|e| e.depth == internal_depth && e.object_index.is_some());
        match last_obj {
            Some(pos) => pos + 1,
            None => entries.len(),
        }
    };

    entries.insert(insert_pos, new_entry);

    // Renumber connections at this depth: src/dst >= original index get +1
    for e in entries.iter_mut() {
        if e.kind != EntryKind::Connect || e.depth != internal_depth {
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

    let patch = pd_toolkit::model::Patch {
        entries,
        warnings: Vec::new(),
    };
    let serialized = serialize(&patch);

    // Validate before writing
    let errors = validate_patch(&pd_toolkit::parser::parse(&serialized)?);
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
