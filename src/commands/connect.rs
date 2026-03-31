use crate::commands::common::validate_patch;
use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::model::{Entry, EntryKind};
use pd_toolkit::parser::parse;
use pd_toolkit::rewrite::serialize;

/// Inputs for the `connect` command.
pub struct RunArgs<'a> {
    /// Path to the input patch file.
    pub file: &'a str,
    /// User-visible depth (0 = top-level).
    pub depth: usize,
    /// Source object index.
    pub src: usize,
    /// Source outlet index.
    pub outlet: usize,
    /// Destination object index.
    pub dst: usize,
    /// Destination inlet index.
    pub inlet: usize,
    /// Whether to overwrite the input file.
    pub in_place: bool,
    /// Whether to create a backup when overwriting.
    pub backup: bool,
    /// Optional output file when not writing in place.
    pub output: Option<&'a str>,
}

pub fn run(args: RunArgs<'_>) -> Result<(String, i32), PdtkError> {
    let RunArgs {
        file,
        depth,
        src,
        outlet,
        dst,
        inlet,
        in_place,
        backup,
        output,
    } = args;

    let input = io::read_patch_file(file)?;
    let mut patch = parse(&input)?;

    let internal_depth = depth + 1;
    let obj_count = patch.object_count_at_depth(depth);

    if src >= obj_count {
        return Err(PdtkError::Usage(format!(
            "src {src} out of range (object count {obj_count} at depth {depth})"
        )));
    }
    if dst >= obj_count {
        return Err(PdtkError::Usage(format!(
            "dst {dst} out of range (object count {obj_count} at depth {depth})"
        )));
    }

    // Refuse duplicate connections
    let already_exists = patch
        .connections_at_depth(depth)
        .iter()
        .any(|c| c.src == src && c.src_outlet == outlet && c.dst == dst && c.dst_inlet == inlet);
    if already_exists {
        return Err(PdtkError::Usage(format!(
            "connection {src} {outlet} {dst} {inlet} already exists at depth {depth}"
        )));
    }

    // Find insertion point: after the last Connect at this depth, or after the
    // last object at this depth if there are no connections yet.
    let insert_pos = patch
        .entries
        .iter()
        .rposition(|e| e.kind == EntryKind::Connect && e.depth == internal_depth)
        .map(|p| p + 1)
        .or_else(|| {
            patch
                .entries
                .iter()
                .rposition(|e| e.depth == internal_depth && e.object_index.is_some())
                .map(|p| p + 1)
        })
        .unwrap_or(patch.entries.len());

    let new_conn = Entry {
        raw: format!("#X connect {src} {outlet} {dst} {inlet};"),
        kind: EntryKind::Connect,
        depth: internal_depth,
        object_index: None,
    };
    patch.entries.insert(insert_pos, new_conn);

    let serialized = serialize(&patch);
    let errors = validate_patch(&parse(&serialized)?);
    if !errors.is_empty() {
        return Err(PdtkError::Usage(format!(
            "validation failed after connect: {}",
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
