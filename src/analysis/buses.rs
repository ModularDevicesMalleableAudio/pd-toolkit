use crate::analysis::send_receive::{
    BusKey, BusKind, Location as SrLocation, collect_receives, collect_sends, is_dollar_zero_scoped,
};
use crate::model::EntryKind;
use crate::parser::parse;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

/// One location of a bus participant: which file, which canvas depth, which
/// object index. `canvas_id` is internal-only and not serialized.
#[derive(Debug, Clone, Serialize)]
pub struct BusLocation {
    pub file: String,
    pub depth: usize,
    pub index: usize,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BusStatus {
    Matched,
    OrphanSend,
    OrphanReceive,
}

/// One row in the bus audit report.
#[derive(Debug, Clone, Serialize)]
pub struct BusReport {
    pub name: String,
    pub kind: BusKind,
    pub senders: Vec<BusLocation>,
    pub receivers: Vec<BusLocation>,
    pub status: BusStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_warning: Option<&'static str>,
}

/// Per-file bus participation. The optional `file` distinguishes
/// per-file aggregation from directory-wide aggregation.
type ParticipantMap = BTreeMap<BusKey, (Vec<BusLocation>, Vec<BusLocation>)>;

fn locations_from(
    map: BTreeMap<BusKey, Vec<SrLocation>>,
    file: &str,
) -> BTreeMap<BusKey, Vec<BusLocation>> {
    let mut out: BTreeMap<BusKey, Vec<BusLocation>> = BTreeMap::new();
    for (key, locs) in map {
        let mut bls: Vec<BusLocation> = locs
            .into_iter()
            .map(|l| BusLocation {
                file: file.to_string(),
                depth: l.depth,
                index: l.index,
            })
            .collect();
        bls.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.depth.cmp(&b.depth))
                .then(a.index.cmp(&b.index))
        });
        out.insert(key, bls);
    }
    out
}

fn merge_participants(
    into: &mut ParticipantMap,
    sends: BTreeMap<BusKey, Vec<BusLocation>>,
    receives: BTreeMap<BusKey, Vec<BusLocation>>,
) {
    for (k, v) in sends {
        into.entry(k).or_default().0.extend(v);
    }
    for (k, v) in receives {
        into.entry(k).or_default().1.extend(v);
    }
}

fn audit_one_file(file: &Path) -> ParticipantMap {
    let Ok(content) = std::fs::read_to_string(file) else {
        return ParticipantMap::new();
    };
    let Ok(patch) = parse(&content) else {
        return ParticipantMap::new();
    };
    let file_str = file.display().to_string();
    let sends = locations_from(collect_sends(&patch.entries), &file_str);
    let receives = locations_from(collect_receives(&patch.entries), &file_str);
    let mut map = ParticipantMap::new();
    merge_participants(&mut map, sends, receives);
    map
}

fn build_reports(map: ParticipantMap) -> Vec<BusReport> {
    let mut rows: Vec<BusReport> = Vec::new();
    for ((kind, name), (mut senders, mut receivers)) in map {
        senders.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.depth.cmp(&b.depth))
                .then(a.index.cmp(&b.index))
        });
        receivers.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.depth.cmp(&b.depth))
                .then(a.index.cmp(&b.index))
        });
        let status = match (senders.is_empty(), receivers.is_empty()) {
            (false, false) => BusStatus::Matched,
            (false, true) => BusStatus::OrphanSend,
            (true, false) => BusStatus::OrphanReceive,
            (true, true) => continue, // shouldn't happen
        };
        let scope_warning = if is_dollar_zero_scoped(&name) {
            Some("dollar-zero-scoped")
        } else {
            None
        };
        rows.push(BusReport {
            name,
            kind,
            senders,
            receivers,
            status,
            scope_warning,
        });
    }
    rows.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.cmp(&b.name)));
    rows
}

/// Audit bus dependencies across `files`. When `per_file` is true, each
/// file is audited independently and rows from different files are kept
/// separate. Otherwise rows from all files are aggregated by `(kind, name)`.
#[must_use]
pub fn audit(files: &[PathBuf], per_file: bool) -> Vec<BusReport> {
    if per_file {
        let mut out = Vec::new();
        for f in files {
            let map = audit_one_file(f);
            out.extend(build_reports(map));
        }
        out
    } else {
        let mut combined = ParticipantMap::new();
        for f in files {
            let m = audit_one_file(f);
            for (k, (s, r)) in m {
                let entry = combined.entry(k).or_default();
                entry.0.extend(s);
                entry.1.extend(r);
            }
        }
        build_reports(combined)
    }
}

/// Direction of an unsatisfied bus contract.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractDirection {
    /// Abstraction has a receiver; caller is expected to provide a sender.
    NeedsSender,
    /// Abstraction has a sender; caller is expected to provide a receiver.
    NeedsReceiver,
}

/// One unsatisfied bus contract at a specific call site.
#[derive(Debug, Clone, Serialize)]
pub struct UnsatisfiedBus {
    pub caller_file: String,
    pub caller_depth: usize,
    /// Object index of the `#X obj <abstraction_name>` entry that called
    /// the abstraction.
    pub caller_index: usize,
    pub abstraction: String,
    pub abstraction_path: String,
    pub bus_kind: BusKind,
    pub bus_name: String,
    pub direction: ContractDirection,
}

fn name_is_dollar_excluded(name: &str) -> bool {
    // Exclude $0-* (instance-scoped) and $N-* (caller parameter expansion)
    // from cross-file contracts.
    name.starts_with('$') || name.starts_with("\\$")
}

/// Bus contract of an abstraction: the set of `(BusKind, name)` pairs it
/// sends and receives internally, excluding dollar-prefixed names.
struct BusContract {
    sends: HashSet<(BusKind, String)>,
    receives: HashSet<(BusKind, String)>,
}

fn derive_contract(path: &Path) -> Option<BusContract> {
    let content = std::fs::read_to_string(path).ok()?;
    let patch = parse(&content).ok()?;
    let sends = collect_sends(&patch.entries);
    let receives = collect_receives(&patch.entries);
    let filter = |m: BTreeMap<BusKey, Vec<SrLocation>>| -> HashSet<(BusKind, String)> {
        m.into_iter()
            .filter(|((_k, n), _)| !name_is_dollar_excluded(n))
            .map(|((k, n), _)| (k, n))
            .collect()
    };
    Some(BusContract {
        sends: filter(sends),
        receives: filter(receives),
    })
}

/// For each `[obj <abstraction>]` call site in `caller_path`, find the
/// abstraction file via `resolve` and check whether the caller's own
/// send/receive set satisfies the abstraction's bus contract.
///
/// Each call-site instance is reported separately (16 `[looper]` instances
/// in the same caller produce up to 16 rows per unsatisfied bus).
pub fn unsatisfied_contracts<F>(caller_path: &Path, resolve: F) -> Vec<UnsatisfiedBus>
where
    F: Fn(&str) -> Option<PathBuf>,
{
    let Ok(content) = std::fs::read_to_string(caller_path) else {
        return Vec::new();
    };
    let Ok(patch) = parse(&content) else {
        return Vec::new();
    };

    // Caller's own send/receive sets.
    let caller_sends: HashSet<(BusKind, String)> =
        collect_sends(&patch.entries).into_keys().collect();
    let caller_receives: HashSet<(BusKind, String)> =
        collect_receives(&patch.entries).into_keys().collect();

    let mut out = Vec::new();
    let caller_str = caller_path.display().to_string();

    for e in &patch.entries {
        if e.kind != EntryKind::Obj {
            continue;
        }
        let Some(idx) = e.object_index else { continue };
        let class = e.class().to_string();
        if class.is_empty() || class == "pd" {
            continue;
        }
        let Some(abs_path) = resolve(&class) else {
            continue;
        };
        let Some(contract) = derive_contract(&abs_path) else {
            continue;
        };
        let depth = e.depth.saturating_sub(1);

        // Abstraction has a receiver → caller must provide a sender.
        for (kind, name) in &contract.receives {
            if !caller_sends.contains(&(*kind, name.clone())) {
                out.push(UnsatisfiedBus {
                    caller_file: caller_str.clone(),
                    caller_depth: depth,
                    caller_index: idx,
                    abstraction: class.clone(),
                    abstraction_path: abs_path.display().to_string(),
                    bus_kind: *kind,
                    bus_name: name.clone(),
                    direction: ContractDirection::NeedsSender,
                });
            }
        }
        // Abstraction has a sender → caller must provide a receiver.
        for (kind, name) in &contract.sends {
            if !caller_receives.contains(&(*kind, name.clone())) {
                out.push(UnsatisfiedBus {
                    caller_file: caller_str.clone(),
                    caller_depth: depth,
                    caller_index: idx,
                    abstraction: class.clone(),
                    abstraction_path: abs_path.display().to_string(),
                    bus_kind: *kind,
                    bus_name: name.clone(),
                    direction: ContractDirection::NeedsReceiver,
                });
            }
        }
    }

    out.sort_by(|a, b| {
        a.caller_file
            .cmp(&b.caller_file)
            .then(a.caller_depth.cmp(&b.caller_depth))
            .then(a.caller_index.cmp(&b.caller_index))
            .then(a.bus_kind.cmp(&b.bus_kind))
            .then(a.bus_name.cmp(&b.bus_name))
    });
    out
}
