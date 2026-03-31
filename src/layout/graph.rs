/// Layout-specific connection graph.
///
/// Distinct from `analysis::graph` which uses petgraph.  Here we need a
/// simple representation that the layering and placement passes can work with
/// directly, without the overhead of the full petgraph API.
use crate::model::Patch;

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub src: usize,
    pub dst: usize,
}

#[derive(Debug, Clone)]
pub struct LayoutGraph {
    pub node_count: usize,
    pub edges: Vec<LayoutEdge>,
}

impl LayoutGraph {
    pub fn build(patch: &Patch, depth: usize) -> Self {
        let node_count = patch.object_count_at_depth(depth);
        let edges = patch
            .connections_at_depth(depth)
            .into_iter()
            .map(|c| LayoutEdge {
                src: c.src,
                dst: c.dst,
            })
            .collect();
        LayoutGraph { node_count, edges }
    }

    /// Predecessors of each node (nodes that have an edge *into* it).
    pub fn predecessors(&self, node: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|e| e.dst == node)
            .map(|e| e.src)
            .collect()
    }

    /// Successors of each node.
    pub fn successors(&self, node: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|e| e.src == node)
            .map(|e| e.dst)
            .collect()
    }

    /// Back edges according to a DFS post-order.  Returns the set of (src,dst)
    /// pairs that form cycles.  Removing these makes the graph a DAG.
    pub fn back_edges(&self) -> std::collections::HashSet<(usize, usize)> {
        let mut visited = vec![false; self.node_count];
        let mut in_stack = vec![false; self.node_count];
        let mut back = std::collections::HashSet::new();

        for start in 0..self.node_count {
            if !visited[start] {
                self.dfs(start, &mut visited, &mut in_stack, &mut back);
            }
        }
        back
    }

    fn dfs(
        &self,
        node: usize,
        visited: &mut Vec<bool>,
        in_stack: &mut Vec<bool>,
        back: &mut std::collections::HashSet<(usize, usize)>,
    ) {
        visited[node] = true;
        in_stack[node] = true;
        for succ in self.successors(node) {
            if succ >= self.node_count {
                continue;
            }
            if !visited[succ] {
                self.dfs(succ, visited, in_stack, back);
            } else if in_stack[succ] {
                back.insert((node, succ));
            }
        }
        in_stack[node] = false;
    }
}
