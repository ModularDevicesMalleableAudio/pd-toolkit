use crate::commands::common::validate_patch;
use crate::errors::PdtkError;
use crate::io;
use crate::types::signatures::outlet_count;
use pdtk::model::{EntryKind, content_without_width_hint, trailing_width_hint};
use pdtk::parser::escape::{escape_pd_dollars, has_unescaped_semicolon};
use pdtk::parser::parse;
use pdtk::rewrite::serialize;

#[allow(clippy::too_many_arguments)]
pub fn run(
    file: &str,
    depth: usize,
    canvas: usize,
    index: usize,
    new_text: &str,
    in_place: bool,
    backup: bool,
    output: Option<&str>,
) -> Result<(String, i32), PdtkError> {
    let input = io::read_patch_file(file)?;
    let mut patch = parse(&input)?;

    let canvas_id = patch.resolve_canvas(depth, canvas).ok_or_else(|| {
        PdtkError::Usage(format!(
            "no canvas {canvas} at depth {depth} ({} at this depth)",
            patch.canvas_ids_at_depth(depth).len()
        ))
    })?;

    // Find the entry to modify (scoped to the selected canvas)
    let entry_pos = patch
        .entries
        .iter()
        .position(|e| e.canvas_id == Some(canvas_id) && e.object_index == Some(index))
        .ok_or_else(|| {
            PdtkError::Usage(format!(
                "no object at depth {depth}, canvas {canvas}, index {index}"
            ))
        })?;

    let entry = &patch.entries[entry_pos];

    // Only Obj and Msg can be modified (not canvas, connect, coords, etc.)
    match entry.kind {
        EntryKind::Obj
        | EntryKind::Msg
        | EntryKind::Text
        | EntryKind::FloatAtom
        | EntryKind::SymbolAtom
        | EntryKind::ListAtom => {}
        EntryKind::Connect => {
            return Err(PdtkError::Usage(
                "cannot modify a #X connect entry".to_string(),
            ));
        }
        EntryKind::CanvasOpen => {
            return Err(PdtkError::Usage(
                "cannot modify a #N canvas entry".to_string(),
            ));
        }
        EntryKind::Coords => {
            return Err(PdtkError::Usage(
                "cannot modify a #X coords entry".to_string(),
            ));
        }
        _ => {
            return Err(PdtkError::Usage(format!(
                "cannot modify entry of kind {:?}",
                entry.kind
            )));
        }
    }

    // Extract X Y coordinates from the current raw entry
    let parts: Vec<&str> = entry.raw.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(PdtkError::Usage(
            "entry too short to extract coordinates".to_string(),
        ));
    }
    let x = parts[2];
    let y = parts[3];

    // Outlet warning: check if existing connections use outlets the new object might not have
    let old_class = entry.class().to_string();
    let old_args: Vec<String> = entry.args();
    let old_width_hint = entry.width_hint();
    let _ = (old_args, old_class); // used only for outlet-count warning lookup

    if has_unescaped_semicolon(new_text) {
        return Err(PdtkError::Usage(
            "text contains an unescaped ';' — use \\; for literal semicolons".to_string(),
        ));
    }

    let escaped_text = escape_pd_dollars(new_text);

    // Separate the object content from any inline `, f N` width hint. If the
    // new text does not carry its own hint, preserve the entry's existing one
    // so editing an object's contents does not silently reset its box width.
    let content = content_without_width_hint(&escaped_text);
    let effective_width_hint = trailing_width_hint(&escaped_text).or(old_width_hint);

    let new_parts: Vec<&str> = content.split_whitespace().collect();
    let new_class = new_parts.first().copied().unwrap_or("");
    let new_args: Vec<&str> = new_parts.get(1..).unwrap_or(&[]).to_vec();

    let mut warning: Option<String> = None;
    if let Some(new_outlets) = outlet_count(new_class, &new_args) {
        // Find the max outlet index used by connections from this object
        let max_used_outlet = patch
            .connections_in_canvas(canvas_id)
            .iter()
            .filter(|c| c.src == index)
            .map(|c| c.src_outlet)
            .max();
        if let Some(max_outlet) = max_used_outlet
            && max_outlet >= new_outlets
        {
            warning = Some(format!(
                "warning: new object '{new_class}' has {new_outlets} outlet(s) but connection uses outlet {max_outlet}"
            ));
        }
    }

    // Build the new raw entry
    let kind = entry.kind.clone();
    let prefix = match kind {
        EntryKind::Obj => "obj",
        EntryKind::Msg => "msg",
        EntryKind::Text => "text",
        EntryKind::FloatAtom => "floatatom",
        EntryKind::SymbolAtom => "symbolatom",
        EntryKind::ListAtom => "listbox",
        _ => "obj",
    };
    let width_suffix = match effective_width_hint {
        Some(w) => format!(", f {w}"),
        None => String::new(),
    };
    let new_raw = format!("#X {prefix} {x} {y} {content}{width_suffix};");
    patch.entries[entry_pos].raw = new_raw;

    let serialized = serialize(&patch);

    // Validate
    let errors = validate_patch(&parse(&serialized)?);
    if !errors.is_empty() {
        return Err(PdtkError::Usage(format!(
            "validation failed after modify: {}",
            errors.join("; ")
        )));
    }

    if let Some(w) = warning {
        eprintln!("{w}");
    }

    // Write
    if in_place {
        io::write_with_backup(file, &serialized, backup)?;
    } else if let Some(out_path) = output {
        io::write_patch_file(out_path, &serialized)?;
    }

    Ok((serialized, 0))
}
