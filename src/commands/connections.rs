use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::parser::parse;
use serde::Serialize;

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

pub fn run(file: &str, index: usize, depth: usize, json: bool) -> Result<String, PdtkError> {
    let input = io::read_patch_file(file)?;
    let patch = parse(&input)?;

    let conns = patch.connections_at_depth(depth);

    let inlets: Vec<InletEntry> = conns
        .iter()
        .filter(|c| c.dst == index)
        .map(|c| {
            let src_text = patch
                .object_at(depth, c.src)
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
                .object_at(depth, c.dst)
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
            out.push_str(&format!(
                "    ← [src:{} outlet:{}] {}\n",
                i.src, i.src_outlet, i.src_text
            ));
        }
    }
    if report.outlets.is_empty() {
        out.push_str("  Outlets: (none)");
    } else {
        out.push_str("  Outlets:");
        for o in &report.outlets {
            out.push_str(&format!(
                "\n    → [dst:{} inlet:{}] {}",
                o.dst, o.dst_inlet, o.dst_text
            ));
        }
    }
    Ok(out)
}
