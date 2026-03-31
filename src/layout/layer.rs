/// Topological longest-path layering.
///
/// Each node is placed in the earliest layer possible given its predecessors
/// (longest-path from sources).  Cycles are broken by removing back-edges
/// before layering; those back-edges are re-added transparently to the caller.
use crate::layout::graph::LayoutGraph;

/// Assign a layer (vertical rank) to every node in the graph.
/// Layer 0 = sources (no non-back-edge predecessors).
/// Nodes with predecessors get `max(predecessor_layer) + 1`.
///
/// The returned `Vec<usize>` has `graph.node_count` entries; `result[i]` is the
/// layer of node `i`.
pub fn assign_layers(graph: &LayoutGraph) -> Vec<usize> {
    let n = graph.node_count;
    if n == 0 {
        return Vec::new();
    }

    // Break cycles so we can do a topological traversal
    let back = graph.back_edges();

    let mut layers = vec![0usize; n];
    let mut changed = true;
    let max_iters = n * n + 1; // safety cap
    let mut iters = 0;

    while changed && iters < max_iters {
        changed = false;
        iters += 1;
        for e in &graph.edges {
            if back.contains(&(e.src, e.dst)) || e.src >= n || e.dst >= n {
                continue;
            }
            let new_layer = layers[e.src] + 1;
            if new_layer > layers[e.dst] {
                layers[e.dst] = new_layer;
                changed = true;
            }
        }
    }

    layers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::graph::LayoutEdge;

    fn graph_from_edges(n: usize, edges: &[(usize, usize)]) -> LayoutGraph {
        LayoutGraph {
            node_count: n,
            edges: edges.iter().map(|&(s, d)| LayoutEdge { src: s, dst: d }).collect(),
        }
    }

    #[test]
    fn layer_linear_chain_correct_layers() {
        // 0→1→2→3
        let g = graph_from_edges(4, &[(0, 1), (1, 2), (2, 3)]);
        let layers = assign_layers(&g);
        assert_eq!(layers, vec![0, 1, 2, 3]);
    }

    #[test]
    fn layer_branching_correct_layers() {
        // 0→1, 0→2, 1→3, 2→3
        let g = graph_from_edges(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let layers = assign_layers(&g);
        assert_eq!(layers[0], 0);
        assert_eq!(layers[1], 1);
        assert_eq!(layers[2], 1);
        assert_eq!(layers[3], 2);
    }

    #[test]
    fn layer_merging_correct_layers() {
        // Two sources merging: 0→2, 1→2, 2→3
        let g = graph_from_edges(4, &[(0, 2), (1, 2), (2, 3)]);
        let layers = assign_layers(&g);
        assert_eq!(layers[0], 0);
        assert_eq!(layers[1], 0);
        assert_eq!(layers[2], 1);
        assert_eq!(layers[3], 2);
    }

    #[test]
    fn layer_cycle_does_not_panic() {
        // 0→1→2→0 (cycle), plus 0→3
        let g = graph_from_edges(4, &[(0, 1), (1, 2), (2, 0), (0, 3)]);
        let layers = assign_layers(&g); // must not hang or panic
        assert_eq!(layers.len(), 4);
    }

    #[test]
    fn layer_disconnected_objects_get_layer_0() {
        // 0→1, node 2 is isolated
        let g = graph_from_edges(3, &[(0, 1)]);
        let layers = assign_layers(&g);
        assert_eq!(layers[2], 0);
    }
}
