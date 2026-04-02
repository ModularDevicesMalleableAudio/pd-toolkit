/// Barycenter crossing-minimisation for within-layer node ordering.
///
/// For each layer we order nodes by the median position of their neighbours
/// in the adjacent layer.  One sweep down (top-to-bottom) is sufficient for
/// a single pass; the `reorder` function runs a configurable number of passes.
use crate::layout::graph::LayoutGraph;

/// Given the layer assignment, group nodes by layer.
/// Returns a `Vec<Vec<usize>>` where `result\[layer\]` is the ordered list of node ids.
pub fn group_by_layer(layers: &[usize]) -> Vec<Vec<usize>> {
    if layers.is_empty() {
        return Vec::new();
    }
    let max_layer = *layers.iter().max().unwrap();
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); max_layer + 1];
    for (node, &l) in layers.iter().enumerate() {
        groups[l].push(node);
    }
    groups
}

/// Run barycenter median reordering for `passes` iterations.
/// Returns an updated `Vec<Vec<usize>>` with nodes sorted within each layer.
pub fn reorder(graph: &LayoutGraph, groups: Vec<Vec<usize>>, passes: usize) -> Vec<Vec<usize>> {
    if groups.is_empty() {
        return groups;
    }

    // Build a position map: node → (layer, pos_within_layer)
    let mut groups = groups;

    for _ in 0..passes {
        // Top-to-bottom pass: for each layer > 0, sort by median predecessor position
        for l in 1..groups.len() {
            let prev_pos: std::collections::HashMap<usize, f64> = groups[l - 1]
                .iter()
                .enumerate()
                .map(|(i, &n)| (n, i as f64))
                .collect();

            let mut scores: Vec<(usize, f64)> = groups[l]
                .iter()
                .map(|&node| {
                    let preds = graph.predecessors(node);
                    let positions: Vec<f64> = preds
                        .iter()
                        .filter_map(|p| prev_pos.get(p))
                        .copied()
                        .collect();
                    let score = if positions.is_empty() {
                        node as f64 // stable fallback
                    } else {
                        median(&positions)
                    };
                    (node, score)
                })
                .collect();

            scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            groups[l] = scores.into_iter().map(|(n, _)| n).collect();
        }

        // Bottom-to-top pass: sort by median successor position
        if groups.len() >= 2 {
            for l in (0..groups.len() - 1).rev() {
                let next_pos: std::collections::HashMap<usize, f64> = groups[l + 1]
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| (n, i as f64))
                    .collect();

                let mut scores: Vec<(usize, f64)> = groups[l]
                    .iter()
                    .map(|&node| {
                        let succs = graph.successors(node);
                        let positions: Vec<f64> = succs
                            .iter()
                            .filter_map(|s| next_pos.get(s))
                            .copied()
                            .collect();
                        let score = if positions.is_empty() {
                            node as f64
                        } else {
                            median(&positions)
                        };
                        (node, score)
                    })
                    .collect();

                scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                groups[l] = scores.into_iter().map(|(n, _)| n).collect();
            }
        }
    }

    groups
}

fn median(v: &[f64]) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}
