use crate::model::{Connection, EntryKind, Patch, parse_scalar, parse_struct};
use crate::parser::escape::has_unescaped_dollar_digit;
use crate::types::signatures::{inlet_count, outlet_count};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Problems found by `check_patch`.  Errors mean PD will not load the patch
/// as written; warnings are suspicious but loadable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Findings {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Structurally check a parsed patch — the checks `validate` reports and
/// `lint` runs before its style checks.
///
/// `path` and `source` locate the patch on disk so that connections to
/// project-local abstractions can be checked against their inlet/outlet
/// counts.  `strict` adds a warning for each duplicate connection.
#[must_use]
pub fn check_patch(patch: &Patch, path: &Path, source: &str, strict: bool) -> Findings {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = patch.warnings.iter().map(|w| format!("{w:?}")).collect();

    // 1) Depth balance check
    let mut balance = 0usize;
    for (i, entry) in patch.entries.iter().enumerate() {
        match entry.kind {
            EntryKind::CanvasOpen => balance += 1,
            EntryKind::Restore => {
                if balance == 0 {
                    errors.push(format!("entry {i}: restore without matching canvas open"));
                } else {
                    balance -= 1;
                }
            }
            _ => {}
        }
    }
    if balance != 1 {
        errors.push(format!(
            "canvas depth imbalance: expected final open depth 1 (root), got {balance}"
        ));
    }

    // 2) Connection syntax and index ranges, then outlet/inlet arity and
    //    duplicates for the in-range ones (canvas-scoped).
    let counts_by_canvas = object_counts_by_canvas(patch);
    let mut seen_by_canvas: HashMap<usize, HashSet<(usize, usize, usize, usize)>> = HashMap::new();
    // Cache of arity counts for classes resolved as project-local abstractions.
    let mut abs_io: HashMap<String, Option<(usize, usize)>> = HashMap::new();
    for (i, entry) in patch.entries.iter().enumerate() {
        if entry.kind != EntryKind::Connect {
            continue;
        }

        let Some(conn) = Connection::parse(&entry.raw) else {
            errors.push(format!("entry {i}: malformed connect line: {}", entry.raw));
            continue;
        };

        let cid = entry.canvas_id.unwrap_or(usize::MAX);
        let count = counts_by_canvas.get(&cid).copied().unwrap_or(0);
        let user_depth = entry.depth.saturating_sub(1);
        errors.extend(range_errors(&conn, count, user_depth));

        if conn.src < count
            && let Some(src_obj) = patch.object_in_canvas(cid, conn.src)
        {
            // Outlet arity: Pd silently drops a wire from a nonexistent outlet.
            let class = src_obj.class();
            let args = src_obj.args();
            let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            if let Some(n) = outlet_count(class, &args_refs) {
                if conn.src_outlet >= n {
                    warnings.push(format!(
                        "depth {user_depth}: '{class}' has {n} outlet(s) but connection uses outlet {}",
                        conn.src_outlet
                    ));
                }
            } else if class != "pd"
                && let Some((_, outs)) = abstraction_io_cached(&mut abs_io, class, path, source)
                && conn.src_outlet >= outs
            {
                warnings.push(format!(
                    "depth {user_depth}: abstraction '{class}' has {outs} outlet(s) but connection uses outlet {}",
                    conn.src_outlet
                ));
            }
        }

        if conn.dst < count
            && let Some(dst_obj) = patch.object_in_canvas(cid, conn.dst)
        {
            let class = dst_obj.class();
            let args = dst_obj.args();
            let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            if let Some(n) = inlet_count(class, &args_refs) {
                if conn.dst_inlet >= n {
                    warnings.push(format!(
                        "depth {user_depth}: '{class}' has {n} inlet(s) but connection uses inlet {}",
                        conn.dst_inlet
                    ));
                }
            } else if class != "pd"
                && let Some((ins, _)) = abstraction_io_cached(&mut abs_io, class, path, source)
                && conn.dst_inlet >= ins
            {
                warnings.push(format!(
                    "depth {user_depth}: abstraction '{class}' has {ins} inlet(s) but connection uses inlet {}",
                    conn.dst_inlet
                ));
            }
        }

        // strict: duplicate connections → warnings (not errors)
        if strict {
            let key = (conn.src, conn.src_outlet, conn.dst, conn.dst_inlet);
            let seen = seen_by_canvas.entry(cid).or_default();
            if !seen.insert(key) {
                warnings.push(format!(
                    "depth {user_depth}: duplicate connection {} {} {} {}",
                    conn.src, conn.src_outlet, conn.dst, conn.dst_inlet
                ));
            }
        }
    }

    // 3) Escaping hygiene checks for retrospective bug discovery.
    for (i, entry) in patch.entries.iter().enumerate() {
        if matches!(
            entry.kind,
            EntryKind::CanvasOpen
                | EntryKind::Connect
                | EntryKind::Coords
                | EntryKind::ArrayData
                | EntryKind::WidthHint
        ) {
            continue;
        }

        if has_unescaped_dollar_digit(&entry.raw) {
            warnings.push(format!(
                "entry {i}: unescaped $-digit token found (expected \\$N in .pd text): {}",
                entry.raw
            ));
        }
    }

    // 3b) Stray fragments from an accidental unescaped ';' in a message or
    //     comment body. PD (and pdtk's tokenizer) terminate an entry at each
    //     unescaped ';', so a mid-body ';' splits the entry and leaves a bare
    //     fragment that does not begin with a '#' sigil. Real Pd patches never
    //     contain bare entries (inline scalar data is a single '\;'-escaped
    //     entry; classic array data is '#A'), so any bare fragment is a stray.
    for (i, entry) in patch.entries.iter().enumerate() {
        if entry.kind == EntryKind::Unknown && !entry.raw.trim_start().starts_with('#') {
            warnings.push(format!(
                "entry {i}: stray content (likely an unescaped ';' in a preceding message body — use \\;): {}",
                entry.raw.trim()
            ));
        }
    }

    //     Also flag the opposite mistake: a record absorbed into the preceding
    //     entry because that entry's final ';' was escaped.
    warnings.extend(swallowed_records(patch));

    // 4) Data-structure consistency (warnings, mirroring Pd's load-time
    //    diagnostics): a `#X scalar` must reference a defined `#N struct`,
    //    and supply exactly as many flat values as the template has scalar
    //    (float/symbol) fields. Templates are patch-global in Pd, so collect
    //    them across all depths.
    let mut templates: HashMap<String, usize> = HashMap::new();
    for e in &patch.entries {
        if e.kind == EntryKind::Struct
            && let Some(t) = parse_struct(&e.raw)
        {
            templates.insert(t.name.clone(), t.scalar_field_count());
        }
    }
    for e in &patch.entries {
        if e.kind != EntryKind::Scalar {
            continue;
        }
        let Some((tmpl, flat)) = parse_scalar(&e.raw) else {
            continue;
        };
        let user_depth = e.depth.saturating_sub(1);
        match templates.get(&tmpl) {
            // A `$`-scoped template name is realized per-instance; static
            // matching is unreliable, so don't claim it is undefined.
            None if !tmpl.contains('$') => warnings.push(format!(
                "depth {user_depth}: scalar references undefined template '{tmpl}' (no matching #N struct)"
            )),
            Some(&nfields) if flat.len() != nfields => warnings.push(format!(
                "depth {user_depth}: scalar '{tmpl}' has {} value(s) but template defines {} scalar field(s)",
                flat.len(),
                nfields
            )),
            _ => {}
        }
    }

    // 5) `#A` array data PD drops or loads somewhere other than the file
    //    suggests.
    warnings.extend(crate::analysis::array_data::array_data_warnings(
        &patch.entries,
    ));

    Findings { errors, warnings }
}

/// Errors for connections whose source or destination index is past the last
/// object of their own canvas.  Sibling subpatches at the same depth have
/// independent index spaces, so counts are per `canvas_id`, not per depth.
/// Malformed connect lines are skipped (`check_patch` reports those).
#[must_use]
pub fn connection_range_errors(patch: &Patch) -> Vec<String> {
    let counts_by_canvas = object_counts_by_canvas(patch);
    let mut errors = Vec::new();
    for e in &patch.entries {
        if e.kind != EntryKind::Connect {
            continue;
        }
        let Some(conn) = Connection::parse(&e.raw) else {
            continue;
        };
        let cid = e.canvas_id.unwrap_or(usize::MAX);
        let count = counts_by_canvas.get(&cid).copied().unwrap_or(0);
        errors.extend(range_errors(&conn, count, e.depth.saturating_sub(1)));
    }
    errors
}

fn object_counts_by_canvas(patch: &Patch) -> HashMap<usize, usize> {
    let mut counts: HashMap<usize, usize> = HashMap::new();
    for e in &patch.entries {
        if e.object_index.is_some()
            && let Some(cid) = e.canvas_id
        {
            *counts.entry(cid).or_insert(0) += 1;
        }
    }
    counts
}

fn range_errors(conn: &Connection, count: usize, user_depth: usize) -> Vec<String> {
    let mut errors = Vec::new();
    if conn.src >= count {
        errors.push(format!(
            "depth {user_depth}: connect src {} out of range (object count {count})",
            conn.src
        ));
    }
    if conn.dst >= count {
        errors.push(format!(
            "depth {user_depth}: connect dst {} out of range (object count {count})",
            conn.dst
        ));
    }
    errors
}

/// Detect entries that have absorbed a following record: a continuation line
/// that starts with a `#X`/`#N`/`#A` sigil.  This is what a message whose
/// final `;` is escaped looks like on disk — PD reads the next record as part
/// of the message, so the object vanishes and later indices shift.
fn swallowed_records(patch: &Patch) -> Vec<String> {
    let mut msgs = Vec::new();
    for (i, e) in patch.entries.iter().enumerate() {
        let swallowed = e.raw.lines().skip(1).find(|line| {
            let line = line.trim_start();
            ["#X ", "#N ", "#A "].iter().any(|s| line.starts_with(s))
        });
        if let Some(line) = swallowed {
            msgs.push(format!(
                "entry {i}: contains what looks like another record ({}) — the preceding \
                 text probably ends in an escaped \\; with no terminating ';'",
                line.trim()
            ));
        }
    }
    msgs
}

/// Memoized `(inlets, outlets)` for an object class resolved as an abstraction
/// relative to the patch being checked. `None` means "not resolvable as an
/// introspectable `.pd` abstraction" (unknown external, missing file, etc.) —
/// in which case no arity check is applied, matching the conservative
/// never-undercount policy of the builtin signature table.
fn abstraction_io_cached(
    cache: &mut HashMap<String, Option<(usize, usize)>>,
    class: &str,
    file_path: &Path,
    content: &str,
) -> Option<(usize, usize)> {
    if let Some(cached) = cache.get(class) {
        return *cached;
    }
    let resolved = crate::analysis::deps::resolve_abstraction(class, file_path, content)
        .and_then(|p| crate::analysis::deps::abstraction_io_counts(&p));
    cache.insert(class.to_string(), resolved);
    resolved
}
