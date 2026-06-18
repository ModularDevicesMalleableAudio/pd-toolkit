use crate::analysis::send_receive::{BusKind, collect_receives, collect_sends};
use crate::model::{Connection, Patch};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet, VecDeque};

type PathNode = (NodeIndex, Option<usize>, Option<EdgeKind>);
type ObjectPathStep = (usize, Option<usize>, Option<EdgeKind>);

/// Edge classification in the depth graph. A `Wire` edge is a literal
/// `#X connect` line; a `Bus` edge is a synthetic edge inferred from
/// matching send/receive names within the same canvas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeKind {
    Wire {
        outlet: usize,
        inlet: usize,
    },
    Bus {
        kind: BusKind,
        name: String,
        /// True if the name starts with `$0-`. Static name matching may
        /// produce false positives across subpatch instances; users see
        /// this flag in trace output without having to opt in.
        dollar_zero_scoped: bool,
    },
}

impl EdgeKind {
    #[must_use]
    pub fn is_wire(&self) -> bool {
        matches!(self, EdgeKind::Wire { .. })
    }
    #[must_use]
    pub fn is_bus(&self) -> bool {
        matches!(self, EdgeKind::Bus { .. })
    }
}

/// Filter applied to graph traversal: which edge kinds to follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeFilter {
    /// Follow only `Wire` edges (default behavior).
    WiresOnly,
    /// Follow both `Wire` and `Bus` edges.
    All,
}

/// A directed graph of one depth-level's connection topology.
pub struct DepthGraph {
    /// node weight = `(object_index, canvas_id)`; edge weight = `EdgeKind`.
    pub graph: DiGraph<(usize, usize), EdgeKind>,
    /// `(object_index, canvas_id) → NodeIndex`. Required because sibling
    /// subpatches at the same depth share object_index counters.
    pub node_map: HashMap<(usize, usize), NodeIndex>,
    /// `NodeIndex → object_index` (for callers that only know depth).
    pub idx_map: HashMap<NodeIndex, usize>,
    pub connections: Vec<Connection>,
}

impl DepthGraph {
    #[must_use]
    pub fn build(patch: &Patch, depth: usize) -> Self {
        let internal = depth + 1;
        let conns = patch.connections_at_depth(depth);

        let mut graph = DiGraph::new();
        let mut node_map: HashMap<(usize, usize), NodeIndex> = HashMap::new();
        let mut idx_map = HashMap::new();
        // Track which (canvas_id, object_index) pairs exist at this depth.
        // Primary keying is on canvas_id since sibling canvases share index
        // space.
        for e in &patch.entries {
            if e.depth != internal {
                continue;
            }
            let Some(idx) = e.object_index else { continue };
            let cid = e.canvas_id.unwrap_or(0);
            if node_map.contains_key(&(idx, cid)) {
                continue;
            }
            let n = graph.add_node((idx, cid));
            node_map.insert((idx, cid), n);
            idx_map.insert(n, idx);
        }

        // Wire edges: a `#X connect` belongs to its canvas. Connections in
        // sibling canvases must not cross. We disambiguate by looking up the
        // connect entry's canvas_id and using that for both endpoints.
        for e in &patch.entries {
            if e.depth != internal || e.kind != crate::model::EntryKind::Connect {
                continue;
            }
            let Some(c) = Connection::parse(&e.raw) else {
                continue;
            };
            let cid = e.canvas_id.unwrap_or(0);
            if let (Some(&s), Some(&d)) = (node_map.get(&(c.src, cid)), node_map.get(&(c.dst, cid)))
            {
                graph.add_edge(
                    s,
                    d,
                    EdgeKind::Wire {
                        outlet: c.src_outlet,
                        inlet: c.dst_inlet,
                    },
                );
            }
        }

        // Bus edges: scoped per canvas. We collect all send/receive uses for
        // the whole patch, then restrict to those at this depth and connect
        // pairs that share `(canvas_id, BusKind, name)`.
        let sends = collect_sends(&patch.entries);
        let receives = collect_receives(&patch.entries);
        for ((kind, name), send_locs) in &sends {
            let Some(recv_locs) = receives.get(&(*kind, name.clone())) else {
                continue;
            };
            let dollar_zero = crate::analysis::send_receive::is_dollar_zero_scoped(name);
            for sl in send_locs {
                if sl.depth != depth {
                    continue;
                }
                for rl in recv_locs {
                    if rl.depth != depth {
                        continue;
                    }
                    if sl.canvas_id != rl.canvas_id {
                        continue;
                    }
                    // Don't double an edge a wire connection already makes.
                    // (A `[s foo]` is a sender, never wired to a `[r foo]`
                    // directly, so this is mostly a no-op, but harmless.)
                    let (Some(&s_node), Some(&r_node)) = (
                        node_map.get(&(sl.index, sl.canvas_id)),
                        node_map.get(&(rl.index, rl.canvas_id)),
                    ) else {
                        continue;
                    };
                    graph.add_edge(
                        s_node,
                        r_node,
                        EdgeKind::Bus {
                            kind: *kind,
                            name: name.clone(),
                            dollar_zero_scoped: dollar_zero,
                        },
                    );
                }
            }
        }

        DepthGraph {
            graph,
            node_map,
            idx_map,
            connections: conns,
        }
    }

    /// Look up a node by object_index. Returns the first match (canvas_id is
    /// not disambiguated — callers querying a depth with multiple sibling
    /// canvases should use `node_for(index, canvas_id)` instead).
    #[must_use]
    pub fn node_for_index(&self, object_index: usize) -> Option<NodeIndex> {
        self.node_map
            .iter()
            .find_map(|((i, _), n)| if *i == object_index { Some(*n) } else { None })
    }

    /// BFS forward from `start_obj`, stopping at `max_hops`. Wire-only by
    /// default; pass `EdgeFilter::All` to include bus edges.
    /// Returns `(object_index, hop_count, edge_kind, from_index)` for
    /// every reachable node except the start.
    #[must_use]
    pub fn forward_trace(
        &self,
        start_obj: usize,
        max_hops: Option<usize>,
        filter: EdgeFilter,
    ) -> Vec<(usize, usize, EdgeKind, usize)> {
        let Some(start) = self.node_for_index(start_obj) else {
            return Vec::new();
        };

        // visited: NodeIndex → (hop_count, from_obj, edge_kind)
        let mut visited: HashMap<NodeIndex, (usize, usize, Option<EdgeKind>)> = HashMap::new();
        visited.insert(start, (0, start_obj, None));

        let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
        queue.push_back((start, 0));

        while let Some((node, hops)) = queue.pop_front() {
            if max_hops.is_some_and(|m| hops >= m) {
                continue;
            }
            for edge in self.graph.edges(node) {
                let ek = edge.weight();
                if matches!(filter, EdgeFilter::WiresOnly) && ek.is_bus() {
                    continue;
                }
                let neighbor = edge.target();
                if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(neighbor) {
                    let from_obj = self.idx_map[&node];
                    e.insert((hops + 1, from_obj, Some(ek.clone())));
                    queue.push_back((neighbor, hops + 1));
                }
            }
        }

        let mut result: Vec<(usize, usize, EdgeKind, usize)> = visited
            .into_iter()
            .filter(|(n, _)| *n != start)
            .filter_map(|(n, (hops, from, ek))| ek.map(|k| (self.idx_map[&n], hops, k, from)))
            .collect();
        result.sort_by_key(|(_, hops, _, _)| *hops);
        result
    }

    /// BFS path find from `from_obj` to `to_obj`. Wire-only by default.
    /// Returns `Some(Vec<(obj_index, via_outlet, via_edge_kind)>)` or `None`.
    /// The first step has `None` for outlet and edge_kind (the start node).
    #[must_use]
    pub fn find_path(
        &self,
        from_obj: usize,
        to_obj: usize,
        max_hops: Option<usize>,
        filter: EdgeFilter,
    ) -> Option<Vec<ObjectPathStep>> {
        let start = self.node_for_index(from_obj)?;
        let end = self.node_for_index(to_obj)?;

        if start == end {
            return Some(vec![(from_obj, None, None)]);
        }

        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut queue: VecDeque<Vec<PathNode>> = VecDeque::new();
        queue.push_back(vec![(start, None, None)]);
        visited.insert(start);

        while let Some(path) = queue.pop_front() {
            let (node, _, _) = path.last().unwrap().clone();
            if max_hops.is_some_and(|m| path.len() > m) {
                continue;
            }
            for edge in self.graph.edges(node) {
                let ek = edge.weight();
                if matches!(filter, EdgeFilter::WiresOnly) && ek.is_bus() {
                    continue;
                }
                let neighbor = edge.target();
                let outlet = match ek {
                    EdgeKind::Wire { outlet, .. } => Some(*outlet),
                    EdgeKind::Bus { .. } => None,
                };
                if neighbor == end {
                    let mut full_path = path.clone();
                    full_path.push((neighbor, outlet, Some(ek.clone())));
                    return Some(
                        full_path
                            .into_iter()
                            .map(|(n, o, e)| (self.idx_map[&n], o, e))
                            .collect(),
                    );
                }
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    let mut new_path = path.clone();
                    new_path.push((neighbor, outlet, Some(ek.clone())));
                    queue.push_back(new_path);
                }
            }
        }
        None
    }

    /// All bus edges in the graph as `(src_obj, dst_obj, EdgeKind)`. Useful
    /// for tests and audit reporting.
    #[must_use]
    pub fn bus_edges(&self) -> Vec<(usize, usize, EdgeKind)> {
        self.graph
            .edge_references()
            .filter_map(|e| {
                if e.weight().is_bus() {
                    Some((
                        self.idx_map[&e.source()],
                        self.idx_map[&e.target()],
                        e.weight().clone(),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Inlet/outlet of a wire edge step, returned as legacy tuple. Returns
    /// `None` for bus edges. Helper for callers maintaining the old API.
    #[must_use]
    pub fn wire_endpoints(ek: &EdgeKind) -> Option<(usize, usize)> {
        match ek {
            EdgeKind::Wire { outlet, inlet } => Some((*outlet, *inlet)),
            EdgeKind::Bus { .. } => None,
        }
    }
}

/// Build a simple adjacency list per depth from patch connections.
#[must_use]
pub fn adjacency_by_depth(patch: &Patch, depth: usize) -> Vec<(usize, usize)> {
    patch
        .connections_at_depth(depth)
        .into_iter()
        .map(|c| (c.src, c.dst))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn build_for(source: &str, depth: usize) -> DepthGraph {
        let patch = parse(source).expect("parse");
        DepthGraph::build(&patch, depth)
    }

    fn count_bus_edges_named(g: &DepthGraph, name: &str) -> usize {
        g.bus_edges()
            .iter()
            .filter(|(_, _, ek)| matches!(ek, EdgeKind::Bus { name: n, .. } if n == name))
            .count()
    }

    #[test]
    fn bus_edge_control_pair() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 s foo;\n\
                   #X obj 10 50 r foo;\n";
        let g = build_for(src, 0);
        let buses = g.bus_edges();
        assert_eq!(buses.len(), 1);
        match &buses[0].2 {
            EdgeKind::Bus { kind, name, .. } => {
                assert_eq!(*kind, BusKind::Control);
                assert_eq!(name, "foo");
            }
            _ => panic!("expected bus edge"),
        }
    }

    #[test]
    fn bus_edge_orphan_send_has_no_edge() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 s foo;\n";
        let g = build_for(src, 0);
        assert!(g.bus_edges().is_empty());
    }

    #[test]
    fn bus_namespace_control_does_not_link_signal() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 s foo;\n\
                   #X obj 10 50 r~ foo;\n";
        let g = build_for(src, 0);
        assert_eq!(count_bus_edges_named(&g, "foo"), 0);
    }

    #[test]
    fn bus_namespace_signal_does_not_link_control() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 s~ bar;\n\
                   #X obj 10 50 r bar;\n";
        let g = build_for(src, 0);
        assert_eq!(count_bus_edges_named(&g, "bar"), 0);
    }

    #[test]
    fn bus_namespace_throw_does_not_link_r_tilde() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 throw~ x;\n\
                   #X obj 10 50 r~ x;\n";
        let g = build_for(src, 0);
        assert_eq!(count_bus_edges_named(&g, "x"), 0);
    }

    #[test]
    fn bus_namespace_signal_pair() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 s~ baz;\n\
                   #X obj 10 50 r~ baz;\n";
        let g = build_for(src, 0);
        let buses = g.bus_edges();
        assert_eq!(buses.len(), 1);
        match &buses[0].2 {
            EdgeKind::Bus { kind, .. } => assert_eq!(*kind, BusKind::Signal),
            _ => panic!("expected bus edge"),
        }
    }

    #[test]
    fn bus_namespace_audiosum_pair() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 throw~ q;\n\
                   #X obj 10 50 catch~ q;\n";
        let g = build_for(src, 0);
        let buses = g.bus_edges();
        assert_eq!(buses.len(), 1);
        match &buses[0].2 {
            EdgeKind::Bus { kind, .. } => assert_eq!(*kind, BusKind::AudioSum),
            _ => panic!("expected bus edge"),
        }
    }

    #[test]
    fn bus_does_not_cross_depth() {
        // [s foo] at depth 0, [r foo] inside subpatch at depth 1
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 s foo;\n\
                   #N canvas 0 0 100 100 10 sub;\n\
                   #X obj 10 10 r foo;\n\
                   #X restore 50 50 pd sub;\n";
        let g0 = build_for(src, 0);
        assert_eq!(count_bus_edges_named(&g0, "foo"), 0);
        let g1 = build_for(src, 1);
        assert_eq!(count_bus_edges_named(&g1, "foo"), 0);
    }

    #[test]
    fn bus_does_not_cross_sibling_canvases() {
        // Two sibling subpatches, each with [s foo] and [r foo]
        let src = "#N canvas 0 0 300 300 10;\n\
                   #N canvas 0 0 100 100 10 sub_a;\n\
                   #X obj 10 10 s foo;\n\
                   #X obj 10 50 r foo;\n\
                   #X connect 0 0 1 0;\n\
                   #X restore 50 50 pd sub_a;\n\
                   #N canvas 0 0 100 100 10 sub_b;\n\
                   #X obj 10 10 s foo;\n\
                   #X obj 10 50 r foo;\n\
                   #X connect 0 0 1 0;\n\
                   #X restore 150 50 pd sub_b;\n";
        let g = build_for(src, 1);
        // Two bus edges total — one per canvas — and they don't cross.
        assert_eq!(count_bus_edges_named(&g, "foo"), 2);
    }

    #[test]
    fn bus_edge_dollar_zero_flagged() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 s \\$0-x;\n\
                   #X obj 10 50 r \\$0-x;\n";
        let g = build_for(src, 0);
        let buses = g.bus_edges();
        assert_eq!(buses.len(), 1);
        match &buses[0].2 {
            EdgeKind::Bus {
                dollar_zero_scoped, ..
            } => {
                assert!(dollar_zero_scoped, "expected dollar_zero_scoped flag");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn forward_trace_wires_only_by_default() {
        // chain → [s foo] then unrelated [r foo]: wire-only trace stops at s.
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 loadbang;\n\
                   #X obj 10 50 s foo;\n\
                   #X obj 10 100 r foo;\n\
                   #X obj 10 150 print;\n\
                   #X connect 0 0 1 0;\n\
                   #X connect 2 0 3 0;\n";
        let g = build_for(src, 0);
        let reached = g.forward_trace(0, None, EdgeFilter::WiresOnly);
        let ixs: Vec<usize> = reached.iter().map(|(i, _, _, _)| *i).collect();
        assert_eq!(ixs, vec![1]); // only reaches s, not r or print
    }

    #[test]
    fn forward_trace_with_bus_filter_follows_bus() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 loadbang;\n\
                   #X obj 10 50 s foo;\n\
                   #X obj 10 100 r foo;\n\
                   #X obj 10 150 print;\n\
                   #X connect 0 0 1 0;\n\
                   #X connect 2 0 3 0;\n";
        let g = build_for(src, 0);
        let reached = g.forward_trace(0, None, EdgeFilter::All);
        let ixs: Vec<usize> = reached.iter().map(|(i, _, _, _)| *i).collect();
        // Reaches s, then bus-hops to r, then wires to print.
        assert!(ixs.contains(&1));
        assert!(ixs.contains(&2));
        assert!(ixs.contains(&3));
    }

    #[test]
    fn find_path_through_bus() {
        let src = "#N canvas 0 0 200 200 10;\n\
                   #X obj 10 10 loadbang;\n\
                   #X obj 10 50 s foo;\n\
                   #X obj 10 100 r foo;\n\
                   #X obj 10 150 print;\n\
                   #X connect 0 0 1 0;\n\
                   #X connect 2 0 3 0;\n";
        let g = build_for(src, 0);
        let path_wires = g.find_path(0, 3, None, EdgeFilter::WiresOnly);
        assert!(path_wires.is_none(), "wire-only should not connect 0 → 3");
        let path_all = g.find_path(0, 3, None, EdgeFilter::All).expect("path");
        let indices: Vec<usize> = path_all.iter().map(|(i, _, _)| *i).collect();
        assert_eq!(indices, vec![0, 1, 2, 3]);
    }
}
