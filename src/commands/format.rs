use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::layout::{
    crossing::{group_by_layer, reorder},
    graph::LayoutGraph,
    layer::assign_layers,
    place::{estimate_width, place_nodes, LayoutOptions},
};
use pd_toolkit::model::{Entry, EntryKind};
use pd_toolkit::parser::parse;
use pd_toolkit::rewrite::serialize;

/// Inputs for the `format` command.
pub struct RunArgs<'a> {
    /// Path to the input patch file.
    pub file: &'a str,
    /// Optional depth to format; None formats all depths.
    pub depth: Option<usize>,
    /// Snap-to-grid size.
    pub grid: i32,
    /// Horizontal padding between nodes.
    pub hpad: i32,
    /// Outer margin.
    pub margin: i32,
    /// Only print output; do not write files.
    pub dry_run: bool,
    /// Overwrite input file.
    pub in_place: bool,
    /// Create backup when overwriting.
    pub backup: bool,
    /// Optional output file when not writing in place.
    pub output: Option<&'a str>,
}

pub fn run(args: RunArgs<'_>) -> Result<String, PdtkError> {
    let RunArgs {
        file,
        depth,
        grid,
        hpad,
        margin,
        dry_run,
        in_place,
        backup,
        output,
    } = args;

    let input = io::read_patch_file(file)?;
    let mut patch = parse(&input)?;

    let max_d = patch.max_depth();

    // Which depths to format
    let depths_to_format: Vec<usize> = if let Some(d) = depth {
        vec![d]
    } else {
        (0..=max_d).collect()
    };

    let opts = LayoutOptions { grid, hpad, vpad: grid + 10, margin };

    for d in depths_to_format {
        let internal = d + 1;
        let g = LayoutGraph::build(&patch, d);

        if g.node_count == 0 {
            continue;
        }

        // Width estimates for every object at this depth
        let mut widths: Vec<i32> = vec![25; g.node_count];
        for e in &patch.entries {
            if e.depth == internal
                && let Some(idx) = e.object_index
                && idx < widths.len()
            {
                widths[idx] = estimate_width(e);
            }
        }

        // Layering + crossing minimisation
        let layers = assign_layers(&g);
        let groups = group_by_layer(&layers);
        let ordered = reorder(&g, groups, 4);

        // Coordinate assignment
        let coords = place_nodes(&ordered, &widths, &opts);

        // Rewrite only X/Y in the object entries — nothing else changes
        for e in patch.entries.iter_mut() {
            if e.depth != internal {
                continue;
            }
            let Some(idx) = e.object_index else { continue };
            if idx >= coords.len() {
                continue;
            }
            let (new_x, new_y) = coords[idx];
            rewrite_coords(e, new_x, new_y);
        }
    }

    let serialized = serialize(&patch);

    // Verify that no connection lines were touched (safety check)
    let orig_conns: Vec<String> = input
        .lines()
        .filter(|l| l.trim_start().starts_with("#X connect"))
        .map(str::to_owned)
        .collect();
    let new_conns: Vec<String> = serialized
        .lines()
        .filter(|l| l.trim_start().starts_with("#X connect"))
        .map(str::to_owned)
        .collect();
    if orig_conns != new_conns {
        return Err(PdtkError::Usage(
            "BUG: format modified connection lines — refusing to write".to_string(),
        ));
    }

    if dry_run {
        return Ok(serialized);
    }

    if in_place {
        io::write_with_backup(file, &serialized, backup)?;
    } else if let Some(out_path) = output {
        io::write_patch_file(out_path, &serialized)?;
    }

    Ok(serialized)
}

/// Rewrite the X and Y coordinate fields (tokens 2 and 3) of an object entry.
/// Only touches #X obj/msg/text/floatatom/symbolatom/restore entries.
fn rewrite_coords(entry: &mut Entry, new_x: i32, new_y: i32) {
    match entry.kind {
        EntryKind::Obj
        | EntryKind::Msg
        | EntryKind::Text
        | EntryKind::FloatAtom
        | EntryKind::SymbolAtom
        | EntryKind::Restore => {}
        _ => return,
    }

    // Find the byte ranges of token[2] and token[3]
    let raw = entry.raw.as_bytes();
    let mut token_idx = 0;
    let mut i = 0;
    let mut x_start = None;
    let mut x_end = None;
    let mut y_start = None;
    let mut y_end = None;

    while i < raw.len() {
        // skip whitespace
        while i < raw.len() && (raw[i] == b' ' || raw[i] == b'\t') {
            i += 1;
        }
        if i >= raw.len() {
            break;
        }
        let tok_start = i;
        while i < raw.len() && raw[i] != b' ' && raw[i] != b'\t' && raw[i] != b'\n' {
            i += 1;
        }
        match token_idx {
            2 => {
                x_start = Some(tok_start);
                x_end = Some(i);
            }
            3 => {
                y_start = Some(tok_start);
                y_end = Some(i);
                break;
            }
            _ => {}
        }
        token_idx += 1;
    }

    if let (Some(xs), Some(xe), Some(ys), Some(ye)) = (x_start, x_end, y_start, y_end) {
        let prefix = entry.raw[..xs].to_string();
        let between = entry.raw[xe..ys].to_string();
        let suffix = entry.raw[ye..].to_string();
        entry.raw = format!("{}{}{}{}{}", prefix, new_x, between, new_y, suffix);
    }
}
