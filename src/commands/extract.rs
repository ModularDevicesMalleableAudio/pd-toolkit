use crate::commands::common::validate_patch;
use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::model::{Connection, Entry, EntryKind};
use pd_toolkit::parser::{assign_depth_and_indices, build_entries, parse, tokenize_entries};
use pd_toolkit::rewrite::serialize;

pub fn run(
    file: &str,
    user_depth: usize,
    output_path: &str,
    in_place: bool,
    backup: bool,
) -> Result<(), PdtkError> {
    let input = io::read_patch_file(file)?;
    let tok = tokenize_entries(&input);
    let entries = build_entries(&tok.entries);

    // The canvas that opens the target subpatch lives at internal depth == user_depth.
    // (Root canvas is at depth 0 and opens to depth 1, so depth-1 subpatch has its
    // CanvasOpen at internal depth 1.)
    let sub_canvas_internal_depth = user_depth;

    // Skip the root canvas (position 0) when searching.
    let canvas_pos = entries[1..]
        .iter()
        .position(|e| e.kind == EntryKind::CanvasOpen && e.depth == sub_canvas_internal_depth)
        .map(|p| p + 1)   // adjust back to full-slice index
        .ok_or_else(|| {
            PdtkError::Usage(format!(
                "no subpatch found at depth {user_depth}"
            ))
        })?;

    // The matching Restore closes this subpatch.
    let restore_pos = entries[canvas_pos + 1..]
        .iter()
        .position(|e| e.kind == EntryKind::Restore && e.depth == sub_canvas_internal_depth)
        .map(|p| p + canvas_pos + 1)
        .ok_or_else(|| {
            PdtkError::Usage(format!(
                "subpatch at depth {user_depth} has no matching #X restore"
            ))
        })?;

    let restore_obj_index = entries[restore_pos].object_index.ok_or_else(|| {
        PdtkError::Usage("restore entry has no object index".to_string())
    })?;

    // -----------------------------------------------------------------------
    // Boundary connection analysis: which parent connections reference the
    // restore box?  These tell us how many inlets/outlets to add.
    // -----------------------------------------------------------------------
    let parent_conns: Vec<Connection> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Connect && e.depth == sub_canvas_internal_depth)
        .filter_map(|e| Connection::parse(&e.raw))
        .collect();

    let n_inlets = parent_conns
        .iter()
        .filter(|c| c.dst == restore_obj_index)
        .map(|c| c.dst_inlet + 1)
        .max()
        .unwrap_or(0);

    let n_outlets = parent_conns
        .iter()
        .filter(|c| c.src == restore_obj_index)
        .map(|c| c.src_outlet + 1)
        .max()
        .unwrap_or(0);

    // -----------------------------------------------------------------------
    // Collect interior entries (everything between canvas open and restore).
    // We preserve the raw text of ALL interior entries — including nested
    // canvas/restore pairs — unchanged.  Only direct (top-level) connections
    // need to be renumbered by the number of inlets we prepend.
    // -----------------------------------------------------------------------
    let interior: &[Entry] = &entries[canvas_pos + 1..restore_pos];

    // Depth of objects that sit directly inside this subpatch (one level deeper
    // than the canvas header).
    let direct_depth = sub_canvas_internal_depth + 1;

    // The maximum Y among direct-level objects, used to place the outlet row.
    let max_y: i32 = interior
        .iter()
        .filter(|e| e.depth == direct_depth && e.object_index.is_some())
        .filter_map(|e| e.y())
        .max()
        .unwrap_or(100);

    // -----------------------------------------------------------------------
    // Build extracted patch text
    // -----------------------------------------------------------------------

    // Standalone canvas header derived from the subpatch header.
    let canvas_raw = &entries[canvas_pos].raw;
    let cp: Vec<&str> = canvas_raw.split_whitespace().collect();
    let ex = cp.get(2).copied().unwrap_or("0");
    let ey = cp.get(3).copied().unwrap_or("22");
    let ew = cp.get(4).copied().unwrap_or("450");
    let eh = cp.get(5).copied().unwrap_or("300");
    let extracted_canvas = format!("#N canvas {ex} {ey} {ew} {eh} 12;");

    let mut raw_lines: Vec<String> = vec![extracted_canvas];

    // Prepend inlet objects (one per inlet needed)
    let mut ix = 30i32;
    for _ in 0..n_inlets {
        raw_lines.push(format!("#X obj {ix} 30 inlet;"));
        ix += 60;
    }

    // Interior entries: all preserved verbatim, EXCEPT direct-level connections
    // which get their src/dst offset by n_inlets.
    for e in interior {
        let raw = if e.kind == EntryKind::Connect && e.depth == direct_depth {
            if let Some(c) = Connection::parse(&e.raw) {
                format!(
                    "#X connect {} {} {} {};",
                    c.src + n_inlets,
                    c.src_outlet,
                    c.dst + n_inlets,
                    c.dst_inlet
                )
            } else {
                e.raw.clone()
            }
        } else {
            e.raw.clone()
        };
        raw_lines.push(raw);
    }

    // Append outlet objects below the existing content.
    let mut ox = 30i32;
    let outlet_y = max_y + 60;
    for _ in 0..n_outlets {
        raw_lines.push(format!("#X obj {ox} {outlet_y} outlet;"));
        ox += 60;
    }

    let extracted_text = format!("{}\n", raw_lines.join("\n"));

    // Validate the extracted patch.
    let extracted_patch = parse(&extracted_text)?;
    let errs = validate_patch(&extracted_patch);
    if !errs.is_empty() {
        return Err(PdtkError::Usage(format!(
            "extracted patch failed validation: {}",
            errs.join("; ")
        )));
    }

    // Write extracted file.
    io::write_patch_file(output_path, &extracted_text)?;

    // -----------------------------------------------------------------------
    // Optionally modify the source file in-place
    // -----------------------------------------------------------------------
    if in_place {
        let mut src_entries = build_entries(&tok.entries);

        let abs_name = std::path::Path::new(output_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(output_path)
            .to_string();

        let restore_x = src_entries[restore_pos].x().unwrap_or(50);
        let restore_y = src_entries[restore_pos].y().unwrap_or(100);

        // Range to remove: canvas open up to and including the restore entry,
        // and any trailing WidthHint belonging to this canvas.
        let end = if restore_pos + 1 < src_entries.len()
            && src_entries[restore_pos + 1].kind == EntryKind::WidthHint
            && src_entries[restore_pos + 1].depth == sub_canvas_internal_depth
        {
            restore_pos + 2
        } else {
            restore_pos + 1
        };

        let replacement = Entry {
            raw: format!("#X obj {restore_x} {restore_y} {abs_name};"),
            kind: EntryKind::Obj,
            depth: sub_canvas_internal_depth,
            object_index: None,
        };

        src_entries.drain(canvas_pos..end);
        src_entries.insert(canvas_pos, replacement);
        assign_depth_and_indices(&mut src_entries);

        let modified_patch = pd_toolkit::model::Patch {
            entries: src_entries,
            warnings: Vec::new(),
        };
        let mod_text = serialize(&modified_patch);

        let mod_parsed = parse(&mod_text)?;
        let errs = validate_patch(&mod_parsed);
        if !errs.is_empty() {
            return Err(PdtkError::Usage(format!(
                "modified source failed validation: {}",
                errs.join("; ")
            )));
        }

        io::write_with_backup(file, &mod_text, backup)?;
    }

    Ok(())
}
