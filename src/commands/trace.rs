use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::analysis::trace as t;
use pd_toolkit::parser::parse;

pub fn run(
    file: &str,
    from: usize,
    to: Option<usize>,
    depth: usize,
    max_hops: Option<usize>,
    json: bool,
) -> Result<String, PdtkError> {
    let input = io::read_patch_file(file)?;
    let patch = parse(&input)?;

    if let Some(dst) = to {
        let result = t::path_trace(&patch, depth, from, dst, max_hops);

        if json {
            return Ok(serde_json::to_string_pretty(&result)?);
        }

        let mut out = format!("Path from {from} to {dst} at depth {depth}:\n");
        match &result.path {
            None => out.push_str("  (no path found)"),
            Some(steps) => {
                for (i, step) in steps.iter().enumerate().collect::<Vec<_>>().into_iter() {
                    if i == 0 {
                        out.push_str(&format!("  [index:{}] {}\n", step.index, step.text.trim()));
                    } else {
                        out.push_str(&format!(
                            "  → outlet {} → inlet {}\n  [index:{}] {}\n",
                            step.via_outlet.unwrap_or(0),
                            step.via_inlet.unwrap_or(0),
                            step.index,
                            step.text.trim()
                        ));
                    }
                }
                out = out.trim_end().to_string();
            }
        }
        return Ok(out);
    }

    // Forward trace
    let result = t::forward_trace(&patch, depth, from, max_hops);

    if json {
        return Ok(serde_json::to_string_pretty(&result)?);
    }

    let mut out = format!("Forward trace from index {from} at depth {depth}:\n");
    if result.hops.is_empty() {
        out.push_str("  (no downstream objects)");
    } else {
        for hop in &result.hops {
            out.push_str(&format!(
                "  hop {}: [index:{}] {} (via outlet {} → inlet {} from index {})\n",
                hop.hops_from_start,
                hop.index,
                hop.text.trim(),
                hop.src_outlet,
                hop.dst_inlet,
                hop.from_index,
            ));
        }
        out = out.trim_end().to_string();
    }
    Ok(out)
}
