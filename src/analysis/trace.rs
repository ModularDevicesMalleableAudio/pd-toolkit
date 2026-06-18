use crate::analysis::graph::{DepthGraph, EdgeFilter, EdgeKind};
use crate::analysis::send_receive::BusKind;
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
    /// `"wire"` or `"bus"`. Always present, regardless of `--show-bus-hops`.
    pub hop_kind: &'static str,
    /// Outlet of the source object for wire hops; absent for bus hops.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src_outlet: Option<usize>,
    /// Inlet of the destination object for wire hops; absent for bus hops.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dst_inlet: Option<usize>,
    /// Bus name for bus hops.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus_name: Option<String>,
    /// `"control"`, `"signal"`, or `"audio_sum"` for bus hops.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus_kind: Option<&'static str>,
    /// Set to `"dollar-zero-scoped"` if the bus name starts with `$0-`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_warning: Option<&'static str>,
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
    pub hop_kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via_outlet: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub via_inlet: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bus_kind: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_warning: Option<&'static str>,
}

pub(crate) fn bus_kind_label(k: BusKind) -> &'static str {
    match k {
        BusKind::Control => "control",
        BusKind::Signal => "signal",
        BusKind::AudioSum => "audio_sum",
    }
}

fn make_hop(
    index: usize,
    hops_from_start: usize,
    from_index: usize,
    ek: &EdgeKind,
    text: String,
) -> TraceHop {
    match ek {
        EdgeKind::Wire { outlet, inlet } => TraceHop {
            index,
            hops_from_start,
            from_index,
            hop_kind: "wire",
            src_outlet: Some(*outlet),
            dst_inlet: Some(*inlet),
            bus_name: None,
            bus_kind: None,
            scope_warning: None,
            text,
        },
        EdgeKind::Bus {
            kind,
            name,
            dollar_zero_scoped,
        } => TraceHop {
            index,
            hops_from_start,
            from_index,
            hop_kind: "bus",
            src_outlet: None,
            dst_inlet: None,
            bus_name: Some(name.clone()),
            bus_kind: Some(bus_kind_label(*kind)),
            scope_warning: if *dollar_zero_scoped {
                Some("dollar-zero-scoped")
            } else {
                None
            },
            text,
        },
    }
}

fn make_step(index: usize, ek: Option<&EdgeKind>, text: String) -> PathStep {
    match ek {
        None => PathStep {
            index,
            text,
            hop_kind: "wire",
            via_outlet: None,
            via_inlet: None,
            bus_name: None,
            bus_kind: None,
            scope_warning: None,
        },
        Some(EdgeKind::Wire { outlet, inlet }) => PathStep {
            index,
            text,
            hop_kind: "wire",
            via_outlet: Some(*outlet),
            via_inlet: Some(*inlet),
            bus_name: None,
            bus_kind: None,
            scope_warning: None,
        },
        Some(EdgeKind::Bus {
            kind,
            name,
            dollar_zero_scoped,
        }) => PathStep {
            index,
            text,
            hop_kind: "bus",
            via_outlet: None,
            via_inlet: None,
            bus_name: Some(name.clone()),
            bus_kind: Some(bus_kind_label(*kind)),
            scope_warning: if *dollar_zero_scoped {
                Some("dollar-zero-scoped")
            } else {
                None
            },
        },
    }
}

#[must_use]
pub fn forward_trace(
    patch: &Patch,
    depth: usize,
    from: usize,
    max_hops: Option<usize>,
    filter: EdgeFilter,
) -> ForwardTrace {
    let g = DepthGraph::build(patch, depth);
    let reachable = g.forward_trace(from, max_hops, filter);

    let hops = reachable
        .into_iter()
        .map(|(index, hop_count, ek, from_index)| {
            let text = patch
                .object_at(depth, index)
                .map(|e| e.raw.clone())
                .unwrap_or_default();
            make_hop(index, hop_count, from_index, &ek, text)
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

#[must_use]
pub fn path_trace(
    patch: &Patch,
    depth: usize,
    from: usize,
    to: usize,
    max_hops: Option<usize>,
    filter: EdgeFilter,
) -> PathTrace {
    let g = DepthGraph::build(patch, depth);
    let raw_path = g.find_path(from, to, max_hops, filter);

    let path = raw_path.map(|steps| {
        steps
            .into_iter()
            .map(|(index, _via_outlet, via_ek)| {
                let text = patch
                    .object_at(depth, index)
                    .map(|e| e.raw.clone())
                    .unwrap_or_default();
                make_step(index, via_ek.as_ref(), text)
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
