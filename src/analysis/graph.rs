use crate::model::Patch;

/// Build a simple adjacency list per depth from patch connections.
pub fn adjacency_by_depth(patch: &Patch, depth: usize) -> Vec<(usize, usize)> {
    patch
        .connections_at_depth(depth)
        .into_iter()
        .map(|c| (c.src, c.dst))
        .collect()
}
