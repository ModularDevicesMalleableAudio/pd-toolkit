use crate::model::{Connection, Patch};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};

type PathNode = (NodeIndex, Option<usize>, Option<usize>);
type ObjectPathStep = (usize, Option<usize>, Option<usize>);

/// A directed graph of one depth-level's connection topology.
pub struct DepthGraph {
    pub graph: DiGraph<usize, (usize, usize)>, // node=object_index, edge=(outlet,inlet)
    pub node_map: HashMap<usize, NodeIndex>,   // object_index → NodeIndex
    pub idx_map: HashMap<NodeIndex, usize>,    // NodeIndex → object_index
    pub connections: Vec<Connection>,
}

impl DepthGraph {
    pub fn build(patch: &Patch, depth: usize) -> Self {
        let conns = patch.connections_at_depth(depth);
        let obj_count = patch.object_count_at_depth(depth);

        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();
        let mut idx_map = HashMap::new();

        for i in 0..obj_count {
            let n = graph.add_node(i);
            node_map.insert(i, n);
            idx_map.insert(n, i);
        }

        for c in &conns {
            if let (Some(&s), Some(&d)) = (node_map.get(&c.src), node_map.get(&c.dst)) {
                graph.add_edge(s, d, (c.src_outlet, c.dst_inlet));
            }
        }

        DepthGraph {
            graph,
            node_map,
            idx_map,
            connections: conns,
        }
    }

    /// BFS forward from `start_obj`, stopping at `max_hops`.
    /// Returns (object_index, hop_count, via_src_outlet, via_dst_inlet, from_index) for
    /// every reachable node except the start.
    pub fn forward_trace(
        &self,
        start_obj: usize,
        max_hops: Option<usize>,
    ) -> Vec<(usize, usize, usize, usize, usize)> {
        let Some(&start) = self.node_map.get(&start_obj) else {
            return Vec::new();
        };

        // visited: NodeIndex → (hop_count, from_obj, outlet, inlet)
        let mut visited: HashMap<NodeIndex, (usize, usize, usize, usize)> = HashMap::new();
        visited.insert(start, (0, start_obj, 0, 0));

        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
        queue.push_back((start, 0));

        while let Some((node, hops)) = queue.pop_front() {
            if max_hops.is_some_and(|m| hops >= m) {
                continue;
            }
            for edge in self.graph.edges(node) {
                let neighbor = edge.target();
                if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(neighbor) {
                    let (outlet, inlet) = *edge.weight();
                    let from_obj = self.idx_map[&node];
                    e.insert((hops + 1, from_obj, outlet, inlet));
                    queue.push_back((neighbor, hops + 1));
                }
            }
        }

        let mut result: Vec<(usize, usize, usize, usize, usize)> = visited
            .into_iter()
            .filter(|(n, _)| *n != start)
            .map(|(n, (hops, from, outlet, inlet))| (self.idx_map[&n], hops, outlet, inlet, from))
            .collect();
        result.sort_by_key(|(_, hops, _, _, _)| *hops);
        result
    }

    /// BFS path find from `from_obj` to `to_obj`.
    /// Returns Some(Vec<(obj_index, via_outlet, via_inlet)>) or None.
    /// The first step has via_outlet=None semantically (represented as 0).
    pub fn find_path(
        &self,
        from_obj: usize,
        to_obj: usize,
        max_hops: Option<usize>,
    ) -> Option<Vec<ObjectPathStep>> {
        let (&start, &end) = (self.node_map.get(&from_obj)?, self.node_map.get(&to_obj)?);

        if start == end {
            return Some(vec![(from_obj, None, None)]);
        }

        // BFS with path tracking: queue of (current_node, path_so_far)
        // path_so_far = Vec<(NodeIndex, Option<outlet>, Option<inlet>)>
        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut queue: VecDeque<Vec<PathNode>> = VecDeque::new();
        queue.push_back(vec![(start, None, None)]);
        visited.insert(start);

        while let Some(path) = queue.pop_front() {
            let (node, _, _) = *path.last().unwrap();
            if max_hops.is_some_and(|m| path.len() > m) {
                continue;
            }
            for edge in self.graph.edges(node) {
                let neighbor = edge.target();
                let (outlet, inlet) = *edge.weight();
                if neighbor == end {
                    let mut full_path = path.clone();
                    full_path.push((neighbor, Some(outlet), Some(inlet)));
                    return Some(
                        full_path
                            .into_iter()
                            .map(|(n, o, i)| (self.idx_map[&n], o, i))
                            .collect(),
                    );
                }
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    let mut new_path = path.clone();
                    new_path.push((neighbor, Some(outlet), Some(inlet)));
                    queue.push_back(new_path);
                }
            }
        }
        None
    }
}

/// Build a simple adjacency list per depth from patch connections.
pub fn adjacency_by_depth(patch: &Patch, depth: usize) -> Vec<(usize, usize)> {
    patch
        .connections_at_depth(depth)
        .into_iter()
        .map(|c| (c.src, c.dst))
        .collect()
}
