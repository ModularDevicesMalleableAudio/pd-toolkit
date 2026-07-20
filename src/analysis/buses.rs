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

/// Realize `$1..$9` in an abstraction bus name using the call-site arguments
/// (`$1` → first arg, `$2` → second, …).
///
/// Returns `None` when the name references `$0` (instance-unique, and so
/// unmatchable across files) or an argument index beyond those supplied at the
/// call site (unrealizable — skip rather than risk a false positive). Names
/// with no dollar args pass through unchanged. A single leading escape
/// backslash before `$` (some contexts save `\$1-foo`) is accepted and
/// dropped, since the realized value is a plain symbol.
fn realize_dollar_name(name: &str, args: &[&str]) -> Option<String> {
    let bytes = name.as_bytes();
    let mut out = String::with_capacity(name.len());
    let mut i = 0;
    while i < bytes.len() {
        let dollar_at = if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'$' {
            i + 1
        } else if bytes[i] == b'$' {
            i
        } else {
            out.push(bytes[i] as char);
            i += 1;
            continue;
        };
        let mut k = dollar_at + 1;
        if k >= bytes.len() || !bytes[k].is_ascii_digit() {
            // `$` not followed by a digit (e.g. `$foo`) — emit verbatim.
            out.push(bytes[i] as char);
            i += 1;
            continue;
        }
        let mut num = 0usize;
        while k < bytes.len() && bytes[k].is_ascii_digit() {
            num = num * 10 + (bytes[k] - b'0') as usize;
            k += 1;
        }
        if num == 0 {
            return None; // $0 is instance-scoped; cannot match cross-file.
        }
        let arg = args.get(num - 1)?; // beyond supplied args — unrealizable.
        out.push_str(arg);
        i = k;
    }
    Some(out)
}

/// Bus contract of an abstraction: the set of `(BusKind, name)` pairs it
/// sends and receives internally.
///
/// `$0`-scoped names are dropped (instance-unique, unmatchable across files),
/// but `$1..$9` names are retained so callers can realize them against their
/// per-call-site arguments (see `realize_dollar_name`).
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
            .filter(|((_k, n), _)| !is_dollar_zero_scoped(n))
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

        // Realize the abstraction's `$1..$9` bus names against this call
        // site's arguments (`#X obj X Y <abs> arg1 arg2 ...`). A plain name
        // realizes to itself; a `$0`/out-of-range name is skipped.
        let call_args = e.args();
        let arg_refs: Vec<&str> = call_args.iter().map(String::as_str).collect();

        // Abstraction has a receiver → caller must provide a sender.
        for (kind, name) in &contract.receives {
            let Some(realized) = realize_dollar_name(name, &arg_refs) else {
                continue;
            };
            if !caller_sends.contains(&(*kind, realized.clone())) {
                out.push(UnsatisfiedBus {
                    caller_file: caller_str.clone(),
                    caller_depth: depth,
                    caller_index: idx,
                    abstraction: class.clone(),
                    abstraction_path: abs_path.display().to_string(),
                    bus_kind: *kind,
                    bus_name: realized,
                    direction: ContractDirection::NeedsSender,
                });
            }
        }
        // Abstraction has a sender → caller must provide a receiver.
        for (kind, name) in &contract.sends {
            let Some(realized) = realize_dollar_name(name, &arg_refs) else {
                continue;
            };
            if !caller_receives.contains(&(*kind, realized.clone())) {
                out.push(UnsatisfiedBus {
                    caller_file: caller_str.clone(),
                    caller_depth: depth,
                    caller_index: idx,
                    abstraction: class.clone(),
                    abstraction_path: abs_path.display().to_string(),
                    bus_kind: *kind,
                    bus_name: realized,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realize_plain_name_unchanged() {
        assert_eq!(
            realize_dollar_name("clock", &["a", "b"]),
            Some("clock".to_string())
        );
    }

    #[test]
    fn realize_dollar_one_prefix() {
        // The canonical abstraction pattern: `$1-foo` fed by call arg `voice3`.
        assert_eq!(
            realize_dollar_name("$1-foo", &["voice3"]),
            Some("voice3-foo".to_string())
        );
    }

    #[test]
    fn realize_multiple_and_escaped_dollars() {
        assert_eq!(
            realize_dollar_name("$1-$2", &["a", "b"]),
            Some("a-b".to_string())
        );
        // A leading escape backslash is accepted and dropped.
        assert_eq!(
            realize_dollar_name(r"\$1-bus", &["x"]),
            Some("x-bus".to_string())
        );
    }

    #[test]
    fn realize_dollar_zero_is_none() {
        // $0 is instance-unique; not matchable across files.
        assert_eq!(realize_dollar_name("$0-local", &["a"]), None);
    }

    #[test]
    fn realize_out_of_range_arg_is_none() {
        // References $2 but only one arg supplied.
        assert_eq!(realize_dollar_name("$2-foo", &["a"]), None);
    }

    #[test]
    fn realize_dollar_non_digit_is_literal() {
        // `$foo` is not a numeric arg (e.g. expr-style); pass through.
        assert_eq!(
            realize_dollar_name("$foo", &["a"]),
            Some("$foo".to_string())
        );
    }
}
