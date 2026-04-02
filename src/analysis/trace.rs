use crate::analysis::graph::DepthGraph;
use crate::model::Patch;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ForwardTrace {
    pub mode: &'static str,
    pub from: usize,
    pub to: Option<usize>,
    pub depth: usize,
    pub hops: Vec<TraceHop>,
}

#[derive(Debug, Serialize)]
pub struct TraceHop {
    pub index: usize,
    pub hops_from_start: usize,
    pub from_index: usize,
    pub src_outlet: usize,
    pub dst_inlet: usize,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct PathTrace {
    pub mode: &'static str,
    pub from: usize,
    pub to: usize,
    pub depth: usize,
    pub path: Option<Vec<PathStep>>,
}

#[derive(Debug, Serialize)]
pub struct PathStep {
    pub index: usize,
    pub text: String,
    pub via_outlet: Option<usize>,
    pub via_inlet: Option<usize>,
}

pub fn forward_trace(
    patch: &Patch,
    depth: usize,
    from: usize,
    max_hops: Option<usize>,
) -> ForwardTrace {
    let g = DepthGraph::build(patch, depth);
    let reachable = g.forward_trace(from, max_hops);

    let hops = reachable
        .into_iter()
        .map(|(index, hop_count, outlet, inlet, from_index)| {
            let text = patch
                .object_at(depth, index)
                .map(|e| e.raw.clone())
                .unwrap_or_default();
            TraceHop {
                index,
                hops_from_start: hop_count,
                from_index,
                src_outlet: outlet,
                dst_inlet: inlet,
                text,
            }
        })
        .collect();

    ForwardTrace {
        mode: "forward",
        from,
        to: None,
        depth,
        hops,
    }
}

pub fn path_trace(
    patch: &Patch,
    depth: usize,
    from: usize,
    to: usize,
    max_hops: Option<usize>,
) -> PathTrace {
    let g = DepthGraph::build(patch, depth);
    let raw_path = g.find_path(from, to, max_hops);

    let path = raw_path.map(|steps| {
        steps
            .into_iter()
            .map(|(index, via_outlet, via_inlet)| {
                let text = patch
                    .object_at(depth, index)
                    .map(|e| e.raw.clone())
                    .unwrap_or_default();
                PathStep {
                    index,
                    text,
                    via_outlet,
                    via_inlet,
                }
            })
            .collect()
    });

    PathTrace {
        mode: "path",
        from,
        to,
        depth,
        path,
    }
}
