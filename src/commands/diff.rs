use crate::errors::PdtkError;
use crate::io;
use pdtk::analysis::diff::diff_patches;
use pdtk::parser::parse;
use std::fmt::Write;

pub fn run(
    file_a: &str,
    file_b: &str,
    json: bool,
    ignore_coords: bool,
) -> Result<String, PdtkError> {
    let input_a = io::read_patch_file(file_a)?;
    let input_b = io::read_patch_file(file_b)?;
    let patch_a = parse(&input_a)?;
    let patch_b = parse(&input_b)?;

    let result = diff_patches(&patch_a, &patch_b, ignore_coords);

    if json {
        return Ok(serde_json::to_string_pretty(&result)?);
    }

    if result.is_empty() {
        return Ok("No differences".to_string());
    }

    let mut out = String::new();

    if !result.objects_removed.is_empty() {
        let _ = writeln!(out, "Objects removed: {}", result.objects_removed.len());
        for c in &result.objects_removed {
            let _ = writeln!(out, "  - [depth:{} index:{}] {}", c.depth, c.index, c.text);
        }
    }
    if !result.objects_added.is_empty() {
        let _ = writeln!(out, "Objects added: {}", result.objects_added.len());
        for c in &result.objects_added {
            let _ = writeln!(out, "  + [depth:{} index:{}] {}", c.depth, c.index, c.text);
        }
    }
    if !result.objects_modified.is_empty() {
        let _ = writeln!(out, "Objects modified: {}", result.objects_modified.len());
        for c in &result.objects_modified {
            let _ = writeln!(
                out,
                "  ~ [depth:{} index:{}]\n    - {}\n    + {}",
                c.depth,
                c.index,
                c.old_text.as_deref().unwrap_or(""),
                c.new_text.as_deref().unwrap_or(""),
            );
        }
    }
    if !result.connections_removed.is_empty() {
        let _ = writeln!(
            out,
            "Connections removed: {}",
            result.connections_removed.len()
        );
        for c in &result.connections_removed {
            let _ = writeln!(
                out,
                "  - [depth:{}] {} {} → {} {}",
                c.depth, c.src, c.src_outlet, c.dst, c.dst_inlet
            );
        }
    }
    if !result.connections_added.is_empty() {
        let _ = writeln!(out, "Connections added: {}", result.connections_added.len());
        for c in &result.connections_added {
            let _ = writeln!(
                out,
                "  + [depth:{}] {} {} → {} {}",
                c.depth, c.src, c.src_outlet, c.dst, c.dst_inlet
            );
        }
    }

    Ok(out.trim_end().to_string())
}
