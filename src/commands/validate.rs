use crate::errors::PdtkError;
use pd_toolkit::{
    model::{Connection, EntryKind},
    parser::parse,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

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

    // 2) Object counts per internal depth
    let mut object_counts: HashMap<usize, usize> = HashMap::new();
    for e in &patch.entries {
        if e.object_index.is_some() {
            *object_counts.entry(e.depth).or_insert(0) += 1;
        }
    }

    // 3) Connection range checks
    let mut seen_by_depth: HashMap<usize, HashSet<(usize, usize, usize, usize)>> = HashMap::new();
    for (i, entry) in patch.entries.iter().enumerate() {
        if entry.kind != EntryKind::Connect {
            continue;
        }

        let Some(conn) = Connection::parse(&entry.raw) else {
            errors.push(format!("entry {i}: malformed connect line: {}", entry.raw));
            continue;
        };

        let count = object_counts.get(&entry.depth).copied().unwrap_or(0);
        let user_depth = entry.depth.saturating_sub(1);

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

        // --strict: duplicate connections → warnings (not errors)
        if strict {
            let key = (conn.src, conn.src_outlet, conn.dst, conn.dst_inlet);
            let seen = seen_by_depth.entry(entry.depth).or_default();
            if !seen.insert(key) {
                warnings.push(format!(
                    "depth {user_depth}: duplicate connection {} {} {} {}",
                    conn.src, conn.src_outlet, conn.dst, conn.dst_inlet
                ));
            }
        }
    }

    let valid = errors.is_empty();
    let exit_code = if valid { 0 } else { 1 };

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
                s.push_str(&format!("\n- WARNING: {w}"));
            }
            s
        }
    } else {
        let mut s = format!("INVALID: {} error(s)", errors.len());
        for e in &errors {
            s.push_str(&format!("\n- {e}"));
        }
        if !warnings.is_empty() {
            s.push_str(&format!("\n{} warning(s)", warnings.len()));
            for w in &warnings {
                s.push_str(&format!("\n- WARNING: {w}"));
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
