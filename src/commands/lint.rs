use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::layout::place::estimate_width;
use pd_toolkit::layout::{
    crossing::{group_by_layer, reorder},
    graph::LayoutGraph,
    layer::assign_layers,
    place::place_nodes,
};
use pd_toolkit::model::{Connection, EntryKind};
use pd_toolkit::parser::parse;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct LintReport {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
    style: Vec<String>,
}

#[derive(Debug)]
pub struct LintResult {
    pub output: String,
    pub exit_code: i32,
}

pub fn run(
    file: &str,
    json: bool,
    send_receive: bool,
    fan_out: bool,
    dsp_loop: bool,
) -> Result<LintResult, PdtkError> {
    let input = io::read_patch_file(file)?;
    let patch = parse(&input)?;

    let mut errors: Vec<String> = Vec::new();
    let warnings: Vec<String> = patch.warnings.iter().map(|w| format!("{w:?}")).collect();
    let mut style: Vec<String> = Vec::new();

    // --- Structural validation (same as validate command) ---

    let mut balance = 0usize;
    for (i, e) in patch.entries.iter().enumerate() {
        match e.kind {
            EntryKind::CanvasOpen => balance += 1,
            EntryKind::Restore => {
                if balance == 0 {
                    errors.push(format!("entry {i}: restore without canvas open"));
                } else {
                    balance -= 1;
                }
            }
            _ => {}
        }
    }
    if balance != 1 {
        errors.push(format!(
            "canvas depth imbalance: expected 1 open canvas, got {balance}"
        ));
    }

    let mut object_counts: std::collections::HashMap<usize, usize> = Default::default();
    for e in &patch.entries {
        if e.object_index.is_some() {
            *object_counts.entry(e.depth).or_insert(0) += 1;
        }
    }

    for e in &patch.entries {
        if e.kind != EntryKind::Connect {
            continue;
        }
        let Some(conn) = Connection::parse(&e.raw) else {
            errors.push(format!("malformed connect: {}", e.raw.trim()));
            continue;
        };
        let count = object_counts.get(&e.depth).copied().unwrap_or(0);
        let ud = e.depth.saturating_sub(1);
        if conn.src >= count {
            errors.push(format!("depth {ud}: src {} out of range", conn.src));
        }
        if conn.dst >= count {
            errors.push(format!("depth {ud}: dst {} out of range", conn.dst));
        }
    }

    // --- Style / layout checks ---

    for d in 0..=patch.max_depth() {
        let internal = d + 1;
        let g = LayoutGraph::build(&patch, d);
        if g.node_count == 0 {
            continue;
        }

        let opts = pd_toolkit::layout::place::LayoutOptions::default();
        let mut widths = vec![25i32; g.node_count];
        for e in &patch.entries {
            if e.depth == internal
                && let Some(idx) = e.object_index
                && idx < widths.len()
            {
                widths[idx] = estimate_width(e);
            }
        }

        // Bounding-box overlap check using *existing* coordinates
        let mut coords: Vec<(i32, i32)> = vec![(0, 0); g.node_count];
        for e in &patch.entries {
            if e.depth == internal
                && let Some(idx) = e.object_index
                && idx < coords.len()
            {
                coords[idx] = (e.x().unwrap_or(0), e.y().unwrap_or(0));
            }
        }

        // Group by their actual Y position as a rough "layer"
        let layers = assign_layers(&g);
        let groups = group_by_layer(&layers);
        let ordered = reorder(&g, groups, 1);

        // Check overlap using actual coords
        let actual_groups: Vec<Vec<usize>> = ordered.clone();

        // Use real x-coordinates to check for overlap
        let mut by_layer: std::collections::HashMap<i32, Vec<usize>> = Default::default();
        for e in &patch.entries {
            if e.depth == internal
                && let Some(idx) = e.object_index
            {
                by_layer.entry(e.y().unwrap_or(0)).or_default().push(idx);
            }
        }

        for row in by_layer.values() {
            let mut boxes: Vec<(i32, i32)> = row
                .iter()
                .filter(|&&n| n < coords.len() && n < widths.len())
                .map(|&n| (coords[n].0, coords[n].0 + widths[n]))
                .collect();
            boxes.sort_by_key(|b| b.0);
            for pair in boxes.windows(2) {
                if pair[0].1 > pair[1].0 {
                    style.push(format!(
                        "depth {d}: objects overlap at y={}",
                        by_layer
                            .iter()
                            .find(|(_, v)| v.contains(&row[0]))
                            .map(|(y, _)| *y)
                            .unwrap_or(0)
                    ));
                    break;
                }
            }
        }

        // Check layout using recomputed coordinates vs existing (non-overlap)
        let recomputed = place_nodes(&ordered, &widths, &opts);
        let _ = (recomputed, actual_groups); // used for potential future checks
    }

    if send_receive {
        run_send_receive_lint(&patch, &mut style);
    }
    if fan_out {
        run_fan_out_lint(&patch, &mut style);
    }
    if dsp_loop {
        run_dsp_loop_lint(&patch, &mut style);
    }

    let valid = errors.is_empty();
    let exit_code = if valid { 0 } else { 1 };

    let report = LintReport {
        valid,
        errors: errors.clone(),
        warnings: warnings.clone(),
        style: style.clone(),
    };

    if json {
        return Ok(LintResult {
            output: serde_json::to_string_pretty(&report)?,
            exit_code,
        });
    }

    let mut out = if valid {
        "OK: patch is valid\n".to_string()
    } else {
        format!("INVALID: {} error(s)\n", errors.len())
    };

    for e in &errors {
        out.push_str(&format!("  ERROR: {e}\n"));
    }
    for w in &warnings {
        out.push_str(&format!("  WARN: {w}\n"));
    }
    for s in &style {
        out.push_str(&format!("  STYLE: {s}\n"));
    }

    Ok(LintResult {
        output: out.trim_end().to_string(),
        exit_code,
    })
}

fn run_send_receive_lint(patch: &pd_toolkit::model::Patch, style: &mut Vec<String>) {
    use pd_toolkit::analysis::send_receive::{collect_receives, collect_sends, format_locations};
    let sends = collect_sends(&patch.entries);
    let receives = collect_receives(&patch.entries);

    for (name, locs) in &sends {
        if !receives.contains_key(name) {
            style.push(format!(
                "orphan send: '{name}' at {} — no matching receive",
                format_locations(locs)
            ));
        }
    }
    for (name, locs) in &receives {
        if !sends.contains_key(name) {
            style.push(format!(
                "dead receive: '{name}' at {} — no matching send",
                format_locations(locs)
            ));
        }
    }
    for (name, locs) in &receives {
        if locs.len() > 1 {
            style.push(format!(
                "broadcast receive: '{name}' has {} receivers at {}",
                locs.len(),
                format_locations(locs)
            ));
        }
    }
}

fn run_fan_out_lint(patch: &pd_toolkit::model::Patch, style: &mut Vec<String>) {
    use std::collections::HashMap;
    // For each depth, group connections by (src, src_outlet) and report
    // groups of >= 2 destinations whose source is non-signal.
    let max_depth = patch.max_depth();
    for d in 0..=max_depth {
        let internal = d + 1;
        let mut groups: HashMap<(usize, usize), usize> = HashMap::new();
        for e in &patch.entries {
            if e.kind != EntryKind::Connect || e.depth != internal {
                continue;
            }
            if let Some(c) = Connection::parse(&e.raw) {
                *groups.entry((c.src, c.src_outlet)).or_insert(0) += 1;
            }
        }
        // Look up source entries by (depth, object_index)
        let mut src_class: HashMap<usize, (EntryKind, String)> = HashMap::new();
        for e in &patch.entries {
            if e.depth == internal
                && let Some(idx) = e.object_index
            {
                src_class.insert(idx, (e.kind.clone(), e.class().to_string()));
            }
        }
        let mut sorted: Vec<((usize, usize), usize)> =
            groups.into_iter().filter(|(_, c)| *c > 1).collect();
        sorted.sort();
        for ((src, outlet), count) in sorted {
            if let Some((kind, class)) = src_class.get(&src)
                && *kind == EntryKind::Obj
                && class.ends_with('~')
            {
                continue; // signal-rate fan-out is fine
            }
            style.push(format!(
                "fan-out: depth {d}, obj {src} outlet {outlet} connects to {count} destinations — consider [trigger]"
            ));
        }
    }
}

fn run_dsp_loop_lint(patch: &pd_toolkit::model::Patch, style: &mut Vec<String>) {
    use petgraph::algo::tarjan_scc;
    use petgraph::graph::{DiGraph, NodeIndex};
    use std::collections::HashMap;

    let max_depth = patch.max_depth();
    for d in 0..=max_depth {
        let internal = d + 1;
        // Collect signal objects at this depth
        let mut signal_objs: HashMap<usize, String> = HashMap::new();
        for e in &patch.entries {
            if e.depth == internal
                && e.kind == EntryKind::Obj
                && let Some(idx) = e.object_index
            {
                let class = e.class().to_string();
                if class.ends_with('~') {
                    signal_objs.insert(idx, class);
                }
            }
        }
        if signal_objs.is_empty() {
            continue;
        }
        let mut g: DiGraph<usize, ()> = DiGraph::new();
        let mut node_for: HashMap<usize, NodeIndex> = HashMap::new();
        for &idx in signal_objs.keys() {
            let n = g.add_node(idx);
            node_for.insert(idx, n);
        }
        for e in &patch.entries {
            if e.kind != EntryKind::Connect || e.depth != internal {
                continue;
            }
            if let Some(c) = Connection::parse(&e.raw)
                && let (Some(&src), Some(&dst)) = (node_for.get(&c.src), node_for.get(&c.dst))
            {
                g.add_edge(src, dst, ());
            }
        }
        for scc in tarjan_scc(&g) {
            let is_cycle =
                scc.len() > 1 || (scc.len() == 1 && g.find_edge(scc[0], scc[0]).is_some());
            if !is_cycle {
                continue;
            }
            let mut indices: Vec<usize> = scc.iter().map(|n| *g.node_weight(*n).unwrap()).collect();
            indices.sort();
            let list = indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            style.push(format!(
                "dsp-loop: depth {d}, cycle involving objects {list}"
            ));
        }
    }
}
