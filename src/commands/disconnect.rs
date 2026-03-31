use crate::commands::common::validate_patch;
use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::model::EntryKind;
use pd_toolkit::parser::parse;
use pd_toolkit::rewrite::serialize;

/// Inputs for the `disconnect` command.
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
    let target_raw = format!("#X connect {src} {outlet} {dst} {inlet};");

    // Find the connection to remove
    let pos = patch
        .entries
        .iter()
        .position(|e| {
            e.kind == EntryKind::Connect && e.depth == internal_depth && e.raw.trim() == target_raw
        })
        .ok_or_else(|| {
            PdtkError::Usage(format!(
                "connection {src} {outlet} {dst} {inlet} not found at depth {depth}"
            ))
        })?;

    patch.entries.remove(pos);

    let serialized = serialize(&patch);
    let errors = validate_patch(&parse(&serialized)?);
    if !errors.is_empty() {
        return Err(PdtkError::Usage(format!(
            "validation failed after disconnect: {}",
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
