use crate::commands::common::{object_insert_pos, validate_patch};
use crate::errors::PdtkError;
use crate::io;
use pdtk::model::{Entry, EntryKind};
use pdtk::parser::escape::escape_pd_dollars;
use pdtk::parser::{assign_depth_and_indices, build_entries, classify_entry, tokenize_entries};
use pdtk::rewrite::serialize;

/// Inputs for the `subpatch` command.
pub struct RunArgs<'a> {
    /// Path to the input patch file.
    pub file: &'a str,
    /// Parent user-visible depth to create the subpatch in (0 = top-level).
    pub depth: usize,
    /// Nth sibling canvas at this depth (0 = first).
    pub canvas: usize,
    /// Object index the new subpatch's restore box takes in the parent.
    pub index: usize,
    /// Subpatch name (appears in `pd <name>`).
    pub name: &'a str,
    /// Number of inlet objects to create inside the subpatch.
    pub inlets: usize,
    /// Number of outlet objects to create inside the subpatch.
    pub outlets: usize,
    /// Overwrite the input file.
    pub in_place: bool,
    /// Create a `.bak` backup before overwriting.
    pub backup: bool,
    /// Optional output file when not writing in place.
    pub output: Option<&'a str>,
}

/// Create a `#N canvas … #X restore … pd <name>;` subpatch block inside an
/// existing patch and renumber the parent canvas's connections.
pub fn run(args: RunArgs<'_>) -> Result<(String, i32), PdtkError> {
    let RunArgs {
        file,
        depth,
        canvas,
        index,
        name,
        inlets,
        outlets,
        in_place,
        backup,
        output,
    } = args;

    // The name lands verbatim in the 6-arg subwindow header and the restore
    // line; whitespace or entry punctuation would corrupt both.
    if name.is_empty()
        || name
            .chars()
            .any(|c| c.is_whitespace() || c == ';' || c == ',')
    {
        return Err(PdtkError::Usage(
            "subpatch name must be a single non-empty token (no whitespace, ';' or ',')"
                .to_string(),
        ));
    }

    let input = io::read_patch_file(file)?;
    let tok = tokenize_entries(&input);
    let mut entries = build_entries(&tok.entries);

    let internal_depth = depth + 1;

    // Resolve the parent canvas among siblings at this depth.
    let canvas_id = pdtk::model::resolve_canvas_id(&entries, depth, canvas).ok_or_else(|| {
        PdtkError::Usage(format!(
            "no canvas {canvas} at depth {depth} ({} at this depth)",
            pdtk::model::canvas_ids_at_depth(&entries, depth).len()
        ))
    })?;

    let obj_count = entries
        .iter()
        .filter(|e| e.canvas_id == Some(canvas_id) && e.object_index.is_some())
        .count();
    if index > obj_count {
        return Err(PdtkError::Usage(format!(
            "index {index} out of range for depth {depth}, canvas {canvas} (object count: {obj_count})"
        )));
    }

    // Build the subpatch block: subwindow canvas header, inlet/outlet objects,
    // then the restore that becomes the parent object.
    let esc_name = escape_pd_dollars(name);
    let block = build_block(&esc_name, inlets, outlets, internal_depth);

    // Find the insertion position within the parent canvas (before the whole
    // span of the object currently at `index`, or after the last object when
    // appending).
    let insert_pos = object_insert_pos(&entries, canvas_id, index);

    // Insert the whole block at the insertion point.
    for (offset, e) in block.into_iter().enumerate() {
        entries.insert(insert_pos + offset, e);
    }

    // Renumber parent connections in this canvas: src/dst >= index get +1
    // (the restore box takes object index `index`).
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
        let (Ok(mut src), Ok(outlet), Ok(mut dst), Ok(inlet)) = (
            parts[2].parse::<usize>(),
            parts[3].parse::<usize>(),
            parts[4].parse::<usize>(),
            parts[5].parse::<usize>(),
        ) else {
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

    // Normalise all depth/index/canvas metadata.
    assign_depth_and_indices(&mut entries);

    let patch = pdtk::model::Patch {
        entries,
        warnings: Vec::new(),
    };
    let serialized = serialize(&patch);

    let errors = validate_patch(&pdtk::parser::parse(&serialized)?);
    if !errors.is_empty() {
        return Err(PdtkError::Usage(format!(
            "validation failed after subpatch: {}",
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

/// Build the entry list for a subpatch block. Depth/canvas metadata is
/// placeholder — the caller re-runs `assign_depth_and_indices` afterwards.
fn build_block(
    name: &str,
    inlets: usize,
    outlets: usize,
    parent_internal_depth: usize,
) -> Vec<Entry> {
    let mut raws: Vec<String> = Vec::new();
    // Subwindow header: `#N canvas X Y W H NAME VIS;` (vis = 0, closed).
    raws.push(format!("#N canvas 0 22 450 300 {name} 0;"));
    for i in 0..inlets {
        let x = 30 + (i as i32) * 80;
        raws.push(format!("#X obj {x} 30 inlet;"));
    }
    for i in 0..outlets {
        let x = 30 + (i as i32) * 80;
        raws.push(format!("#X obj {x} 220 outlet;"));
    }
    raws.push(format!("#X restore 50 50 pd {name};"));

    raws.into_iter()
        .map(|raw| {
            let kind = classify_entry(&raw);
            Entry {
                raw,
                kind,
                depth: parent_internal_depth,
                object_index: None,
                canvas_id: None,
            }
        })
        .collect()
}
