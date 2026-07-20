use crate::errors::PdtkError;
use crate::types::signatures::{inlet_count, outlet_count};
use pdtk::{
    model::{Connection, EntryKind},
    parser::{escape::has_unescaped_dollar_digit, parse},
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

#[derive(Debug)]
pub struct ValidateResult {
    pub output: String,
    pub exit_code: i32,
}

#[derive(Debug, Serialize)]
struct ValidateJson {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

pub fn run(
    file: &str,
    strict: bool,
    json: bool,
    output: Option<&str>,
) -> Result<ValidateResult, PdtkError> {
    let input = std::fs::read_to_string(file)?;
    let patch = parse(&input)?;

    let mut errors: Vec<String> = Vec::new();
    // Warnings: parser warnings + --strict soft findings (do NOT cause exit 1)
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

    // 2) Object counts per canvas (sibling subpatches at the same depth have
    //    independent index spaces, so counts must be per canvas, not per depth).
    let mut counts_by_canvas: HashMap<usize, usize> = HashMap::new();
    for e in &patch.entries {
        if e.object_index.is_some()
            && let Some(cid) = e.canvas_id
        {
            *counts_by_canvas.entry(cid).or_insert(0) += 1;
        }
    }

    // 3) Connection range checks + outlet/inlet arity checks (canvas-scoped)
    let mut seen_by_canvas: HashMap<usize, HashSet<(usize, usize, usize, usize)>> = HashMap::new();
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

        if conn.src >= count {
            errors.push(format!(
                "depth {user_depth}: connect src {} out of range (object count {count})",
                conn.src
            ));
        } else if let Some(src_obj) = patch.object_in_canvas(cid, conn.src) {
            // Outlet arity: Pd silently drops a wire from a nonexistent outlet.
            let class = src_obj.class();
            let args = src_obj.args();
            let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            if let Some(n) = outlet_count(class, &args_refs)
                && conn.src_outlet >= n
            {
                warnings.push(format!(
                    "depth {user_depth}: '{class}' has {n} outlet(s) but connection uses outlet {}",
                    conn.src_outlet
                ));
            }
        }

        if conn.dst >= count {
            errors.push(format!(
                "depth {user_depth}: connect dst {} out of range (object count {count})",
                conn.dst
            ));
        } else if let Some(dst_obj) = patch.object_in_canvas(cid, conn.dst) {
            let class = dst_obj.class();
            let args = dst_obj.args();
            let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            if let Some(n) = inlet_count(class, &args_refs)
                && conn.dst_inlet >= n
            {
                warnings.push(format!(
                    "depth {user_depth}: '{class}' has {n} inlet(s) but connection uses inlet {}",
                    conn.dst_inlet
                ));
            }
        }

        // --strict: duplicate connections → warnings (not errors)
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

    // 4) Escaping hygiene checks for retrospective bug discovery.
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

    // 4b) Stray fragments from an accidental unescaped ';' in a message or
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

    // 5) Detached `#A` array data (data bound to the wrong array or dropped).
    warnings.extend(crate::commands::common::detached_array_data(&patch));

    let valid = errors.is_empty();
    let exit_code = i32::from(!valid);

    let text = if json {
        serde_json::to_string_pretty(&ValidateJson {
            valid,
            errors,
            warnings,
        })?
    } else if valid {
        if warnings.is_empty() {
            "OK: patch is valid".to_string()
        } else {
            let mut s = format!("OK: patch is valid ({} warning(s))", warnings.len());
            for w in &warnings {
                let _ = write!(s, "\n- WARNING: {w}");
            }
            s
        }
    } else {
        let mut s = format!("INVALID: {} error(s)", errors.len());
        for e in &errors {
            let _ = write!(s, "\n- {e}");
        }
        if !warnings.is_empty() {
            let _ = write!(s, "\n{} warning(s)", warnings.len());
            for w in &warnings {
                let _ = write!(s, "\n- WARNING: {w}");
            }
        }
        s
    };

    if let Some(out_path) = output {
        std::fs::write(out_path, &text)?;
        return Ok(ValidateResult {
            output: String::new(),
            exit_code,
        });
    }

    Ok(ValidateResult {
        output: text,
        exit_code,
    })
}
