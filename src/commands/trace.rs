use crate::errors::PdtkError;
use crate::io;
use pdtk::analysis::graph::EdgeFilter;
use pdtk::analysis::trace as t;
use pdtk::parser::parse;
use std::fmt::Write;

pub fn run(
    file: &str,
    from: usize,
    to: Option<usize>,
    depth: usize,
    max_hops: Option<usize>,
    json: bool,
    show_bus_hops: bool,
) -> Result<String, PdtkError> {
    let input = io::read_patch_lenient(file)?;
    let patch = parse(&input)?;
    let filter = if show_bus_hops {
        EdgeFilter::All
    } else {
        EdgeFilter::WiresOnly
    };

    if let Some(dst) = to {
        let result = t::path_trace(&patch, depth, from, dst, max_hops, filter);

        if json {
            return Ok(serde_json::to_string_pretty(&result)?);
        }

        let mut out = format!("Path from {from} to {dst} at depth {depth}:\n");
        match &result.path {
            None => out.push_str("  (no path found)"),
            Some(steps) => {
                for (i, step) in steps.iter().enumerate() {
                    if i == 0 {
                        let _ = writeln!(out, "  [index:{}] {}", step.index, step.text.trim());
                    } else if step.hop_kind == "bus" {
                        let kind = step.bus_kind.unwrap_or("control");
                        let name = step.bus_name.as_deref().unwrap_or("");
                        let warn = step
                            .scope_warning
                            .map(|w| format!(" [{w}]"))
                            .unwrap_or_default();
                        let _ = writeln!(
                            out,
                            "  → bus \"{name}\" ({kind}){warn}\n  [index:{}] {}",
                            step.index,
                            step.text.trim()
                        );
                    } else {
                        let _ = writeln!(
                            out,
                            "  → outlet {} → inlet {}\n  [index:{}] {}",
                            step.via_outlet.unwrap_or(0),
                            step.via_inlet.unwrap_or(0),
                            step.index,
                            step.text.trim()
                        );
                    }
                }
                out = out.trim_end().to_string();
            }
        }
        return Ok(out);
    }

    // Forward trace
    let result = t::forward_trace(&patch, depth, from, max_hops, filter);

    if json {
        return Ok(serde_json::to_string_pretty(&result)?);
    }

    let mut out = format!("Forward trace from index {from} at depth {depth}:\n");
    if result.hops.is_empty() {
        out.push_str("  (no downstream objects)");
    } else {
        for hop in &result.hops {
            if hop.hop_kind == "bus" {
                let kind = hop.bus_kind.unwrap_or("control");
                let name = hop.bus_name.as_deref().unwrap_or("");
                let warn = hop
                    .scope_warning
                    .map(|w| format!(" [{w}]"))
                    .unwrap_or_default();
                let _ = writeln!(
                    out,
                    "  hop {}: [index:{}] {} (via bus \"{name}\" ({kind}){warn} from index {})",
                    hop.hops_from_start,
                    hop.index,
                    hop.text.trim(),
                    hop.from_index,
                );
            } else {
                let _ = writeln!(
                    out,
                    "  hop {}: [index:{}] {} (via outlet {} → inlet {} from index {})",
                    hop.hops_from_start,
                    hop.index,
                    hop.text.trim(),
                    hop.src_outlet.unwrap_or(0),
                    hop.dst_inlet.unwrap_or(0),
                    hop.from_index,
                );
            }
        }
        out = out.trim_end().to_string();
    }
    Ok(out)
}
