use crate::errors::PdtkError;
use crate::io;
use pdtk::parser::parse;
use serde::Serialize;
use std::fmt::Write;

#[derive(Debug, Serialize)]
struct ConnectionsReport {
    index: usize,
    depth: usize,
    inlets: Vec<InletEntry>,
    outlets: Vec<OutletEntry>,
}

#[derive(Debug, Serialize)]
struct InletEntry {
    src: usize,
    src_outlet: usize,
    src_text: String,
}

#[derive(Debug, Serialize)]
struct OutletEntry {
    dst: usize,
    dst_inlet: usize,
    dst_text: String,
}

pub fn run(
    file: &str,
    index: usize,
    depth: usize,
    canvas: usize,
    json: bool,
) -> Result<String, PdtkError> {
    let input = io::read_patch_lenient(file)?;
    let patch = parse(&input)?;

    let canvas_id = patch.resolve_canvas(depth, canvas).ok_or_else(|| {
        PdtkError::Usage(format!(
            "no canvas {canvas} at depth {depth} ({} at this depth)",
            patch.canvas_ids_at_depth(depth).len()
        ))
    })?;
    let conns = patch.connections_in_canvas(canvas_id);

    let inlets: Vec<InletEntry> = conns
        .iter()
        .filter(|c| c.dst == index)
        .map(|c| {
            let src_text = patch
                .object_in_canvas(canvas_id, c.src)
                .map(|e| e.raw.clone())
                .unwrap_or_default();
            InletEntry {
                src: c.src,
                src_outlet: c.src_outlet,
                src_text,
            }
        })
        .collect();

    let outlets: Vec<OutletEntry> = conns
        .iter()
        .filter(|c| c.src == index)
        .map(|c| {
            let dst_text = patch
                .object_in_canvas(canvas_id, c.dst)
                .map(|e| e.raw.clone())
                .unwrap_or_default();
            OutletEntry {
                dst: c.dst,
                dst_inlet: c.dst_inlet,
                dst_text,
            }
        })
        .collect();

    let report = ConnectionsReport {
        index,
        depth,
        inlets,
        outlets,
    };

    if json {
        return Ok(serde_json::to_string_pretty(&report)?);
    }

    let mut out = format!("Object [depth:{depth} index:{index}]\n");
    if report.inlets.is_empty() {
        out.push_str("  Inlets: (none)\n");
    } else {
        out.push_str("  Inlets:\n");
        for i in &report.inlets {
            let _ = writeln!(
                out,
                "    ← [src:{} outlet:{}] {}",
                i.src, i.src_outlet, i.src_text
            );
        }
    }
    if report.outlets.is_empty() {
        out.push_str("  Outlets: (none)");
    } else {
        out.push_str("  Outlets:");
        for o in &report.outlets {
            let _ = write!(
                out,
                "\n    → [dst:{} inlet:{}] {}",
                o.dst, o.dst_inlet, o.dst_text
            );
        }
    }
    Ok(out)
}
