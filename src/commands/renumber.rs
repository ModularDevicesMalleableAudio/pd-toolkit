use crate::commands::common::validate_patch;
use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::model::EntryKind;
use pd_toolkit::parser::parse;
use pd_toolkit::rewrite::serialize;

pub fn run(
    file: &str,
    depth: usize,
    from: usize,
    delta: i32,
    in_place: bool,
    backup: bool,
    output: Option<&str>,
) -> Result<(String, i32), PdtkError> {
    let input = io::read_patch_file(file)?;
    let mut patch = parse(&input)?;

    let internal_depth = depth + 1;

    // Shift connection indices at this depth where src >= from or dst >= from
    for e in patch.entries.iter_mut() {
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

        let Ok(mut src) = parts[2].parse::<i64>() else {
            continue;
        };
        let Ok(outlet) = parts[3].parse::<usize>() else {
            continue;
        };
        let Ok(mut dst) = parts[4].parse::<i64>() else {
            continue;
        };
        let Ok(inlet) = parts[5].parse::<usize>() else {
            continue;
        };

        if src >= from as i64 {
            src += delta as i64;
        }
        if dst >= from as i64 {
            dst += delta as i64;
        }

        // Clamp to 0 minimum (negative indices are invalid)
        src = src.max(0);
        dst = dst.max(0);

        e.raw = format!("#X connect {src} {outlet} {dst} {inlet};");
    }

    let serialized = serialize(&patch);

    // Validate the result
    let errors = validate_patch(&parse(&serialized)?);
    if !errors.is_empty() {
        return Err(PdtkError::Usage(format!(
            "validation failed after renumber: {}",
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
