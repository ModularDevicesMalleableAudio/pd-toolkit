use crate::commands::common::validate_patch;
use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::model::EntryKind;
use pd_toolkit::parser::{assign_depth_and_indices, build_entries, tokenize_entries};
use pd_toolkit::rewrite::serialize;

pub fn run(
    file: &str,
    depth: usize,
    index: usize,
    in_place: bool,
    backup: bool,
    output: Option<&str>,
) -> Result<(String, i32), PdtkError> {
    let input = io::read_patch_file(file)?;
    let tok = tokenize_entries(&input);
    let mut entries = build_entries(&tok.entries);

    let internal_depth = depth + 1;

    // Find and validate the entry to delete
    let _delete_pos = entries
        .iter()
        .position(|e| e.depth == internal_depth && e.object_index == Some(index))
        .ok_or_else(|| PdtkError::Usage(format!("no object at depth {depth}, index {index}")))?;

    // Remove the entry
    entries.remove(_delete_pos);

    // Remove all connections that reference the deleted object at this depth
    entries.retain(|e| {
        if e.kind != EntryKind::Connect || e.depth != internal_depth {
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

    // Renumber remaining connections: src > index → src - 1, dst > index → dst - 1
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

        if src > index {
            src -= 1;
        }
        if dst > index {
            dst -= 1;
        }

        e.raw = format!("#X connect {src} {outlet} {dst} {inlet};");
    }

    // Re-assign depth/index
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
            "validation failed after delete: {}",
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
