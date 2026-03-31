use pd_toolkit::model::{Entry, EntryKind, Patch};
use pd_toolkit::parser::assign_depth_and_indices;

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

/// Delete one object at `user_depth`/`index` from raw entry list, including
/// connection cleanup and renumbering at that depth.
pub fn delete_object(entries: &mut Vec<Entry>, user_depth: usize, index: usize) -> bool {
    let internal_depth = user_depth + 1;
    let Some(pos) = entries
        .iter()
        .position(|e| e.depth == internal_depth && e.object_index == Some(index))
    else {
        return false;
    };

    entries.remove(pos);

    // Remove touched connections
    entries.retain(|e| {
        if e.kind != EntryKind::Connect || e.depth != internal_depth {
            return true;
        }
        let parts: Vec<&str> = e
            .raw
            .trim()
            .trim_end_matches(';')
            .split_whitespace()
            .collect();
        if parts.len() != 6 {
            return true;
        }
        let src = parts[2].parse::<usize>().unwrap_or(usize::MAX);
        let dst = parts[4].parse::<usize>().unwrap_or(usize::MAX);
        src != index && dst != index
    });

    // Renumber remaining connections
    for e in entries.iter_mut() {
        if e.kind != EntryKind::Connect || e.depth != internal_depth {
            continue;
        }
        let parts: Vec<&str> = e
            .raw
            .trim()
            .trim_end_matches(';')
            .split_whitespace()
            .collect();
        if parts.len() != 6 || parts[0] != "#X" || parts[1] != "connect" {
            continue;
        }
        let Ok(mut src) = parts[2].parse::<usize>() else {
            continue;
        };
        let Ok(outlet) = parts[3].parse::<usize>() else {
            continue;
        };
        let Ok(mut dst) = parts[4].parse::<usize>() else {
            continue;
        };
        let Ok(inlet) = parts[5].parse::<usize>() else {
            continue;
        };

        if src > index {
            src -= 1;
        }
        if dst > index {
            dst -= 1;
        }
        e.raw = format!("#X connect {src} {outlet} {dst} {inlet};");
    }

    assign_depth_and_indices(entries);
    true
}
