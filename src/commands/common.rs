use pd_toolkit::model::{EntryKind, Patch};

/// Post-mutation validation: checks that all connection src/dst indices are in
/// range for their depth.  Returns a list of error strings (empty = valid).
pub fn validate_patch(patch: &Patch) -> Vec<String> {
    let mut errors = Vec::new();

    // Object counts per internal depth
    let mut object_counts: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    for e in &patch.entries {
        if e.object_index.is_some() {
            *object_counts.entry(e.depth).or_insert(0) += 1;
        }
    }

    for e in &patch.entries {
        if e.kind != EntryKind::Connect {
            continue;
        }

        let parts: Vec<&str> = e
            .raw
            .trim()
            .trim_end_matches(';')
            .split_whitespace()
            .collect();
        if parts.len() != 6 {
            continue;
        }

        if let (Ok(src), Ok(dst)) = (parts[2].parse::<usize>(), parts[4].parse::<usize>()) {
            let count = object_counts.get(&e.depth).copied().unwrap_or(0);
            let user_depth = e.depth.saturating_sub(1);
            if src >= count {
                errors.push(format!(
                    "depth {user_depth}: connect src {src} out of range (object count {count})"
                ));
            }
            if dst >= count {
                errors.push(format!(
                    "depth {user_depth}: connect dst {dst} out of range (object count {count})"
                ));
            }
        }
    }

    errors
}
