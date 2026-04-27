use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::model::{Entry, EntryKind, gui_send_receive_arg_indices, vu_receive_arg_index};
use pd_toolkit::parser::escape::escape_pd_dollars;
use pd_toolkit::parser::{build_entries, tokenize_entries};
use pd_toolkit::rewrite::serialize;

/// Replace the token at `token_pos` (0-based, whitespace-split) in `raw`
/// with `new_token`, if and only if the existing token (stripped of any
/// trailing semicolon) equals `expected`.  Returns the new raw string, or
/// `None` if the token did not match.
fn replace_raw_token(
    raw: &str,
    token_pos: usize,
    expected: &str,
    new_token: &str,
) -> Option<String> {
    // Collect byte ranges of each whitespace-delimited token
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        // skip whitespace
        while i < raw.len()
            && (bytes[i] == b' ' || bytes[i] == b'\n' || bytes[i] == b'\t' || bytes[i] == b'\r')
        {
            i += 1;
        }
        if i >= raw.len() {
            break;
        }
        let start = i;
        while i < raw.len()
            && bytes[i] != b' '
            && bytes[i] != b'\n'
            && bytes[i] != b'\t'
            && bytes[i] != b'\r'
        {
            i += 1;
        }
        ranges.push((start, i));
    }

    if token_pos >= ranges.len() {
        return None;
    }

    let (start, end) = ranges[token_pos];
    let token = &raw[start..end];

    // Strip trailing semicolon for the comparison only
    let value = token.trim_end_matches(';');
    let has_semi = token.ends_with(';');

    if value != expected {
        return None;
    }

    let replacement = if has_semi {
        format!("{new_token};")
    } else {
        new_token.to_string()
    };

    let mut result = raw[..start].to_string();
    result.push_str(&replacement);
    result.push_str(&raw[end..]);
    Some(result)
}

/// Return true if this object class is a simple send/receive type.
fn simple_sr_class(class: &str) -> bool {
    matches!(
        class,
        "s" | "send" | "r" | "receive" | "s~" | "send~" | "r~" | "receive~" | "throw~" | "catch~"
    )
}

/// Try to rename `from` → `to` in a single raw entry.
/// Returns `Some(new_raw)` if any replacement was made, `None` otherwise.
pub fn rename_in_entry(raw: &str, kind: &EntryKind, from: &str, to: &str) -> Option<String> {
    match kind {
        EntryKind::Obj => {
            // Token positions: 0=#X 1=obj 2=X 3=Y 4=class 5+=args
            // Class is at position 4
            let tokens: Vec<&str> = raw.split_whitespace().collect();
            let class = tokens.get(4).copied().unwrap_or("");

            // Strip trailing semicolon from class if it's the last token
            let class_clean = class.trim_end_matches(';');

            if simple_sr_class(class_clean) {
                // Name is the first arg: position 5
                return replace_raw_token(raw, 5, from, to);
            }

            // GUI objects with (send_idx, recv_idx) relative to args start
            if let Some((si, ri)) = gui_send_receive_arg_indices(class_clean) {
                // args start at token position 5, so raw position = 5 + arg_index
                let send_raw_pos = 5 + si;
                let recv_raw_pos = 5 + ri;

                let after_send = replace_raw_token(raw, send_raw_pos, from, to);
                let base = after_send.as_deref().unwrap_or(raw);
                let after_recv = replace_raw_token(base, recv_raw_pos, from, to);

                return after_recv.or(after_send);
            }

            // vu has only a receive field
            if class_clean == "vu" {
                let recv_raw_pos = 5 + vu_receive_arg_index();
                return replace_raw_token(raw, recv_raw_pos, from, to);
            }

            None
        }

        EntryKind::FloatAtom | EntryKind::SymbolAtom => {
            // #X floatatom X Y width min max flag send receive label
            // Send = position 8, receive = position 9
            let after_send = replace_raw_token(raw, 8, from, to);
            let base = after_send.as_deref().unwrap_or(raw);
            let after_recv = replace_raw_token(base, 9, from, to);
            after_recv.or(after_send)
        }

        _ => None,
    }
}

/// Collect all send/receive names currently in use across the entry list.
fn collect_sr_names(entries: &[Entry]) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();

    for e in entries {
        match e.kind {
            EntryKind::Obj => {
                let tokens: Vec<&str> = e.raw.split_whitespace().collect();
                let class = tokens.get(4).copied().unwrap_or("").trim_end_matches(';');
                if simple_sr_class(class) {
                    if let Some(name) = tokens.get(5) {
                        let n = name.trim_end_matches(';');
                        if n != "empty" && n != "-" {
                            names.insert(n.to_string());
                        }
                    }
                } else if let Some((si, ri)) = gui_send_receive_arg_indices(class) {
                    for idx in [5 + si, 5 + ri] {
                        if let Some(tok) = tokens.get(idx) {
                            let n = tok.trim_end_matches(';');
                            if n != "empty" && n != "-" {
                                names.insert(n.to_string());
                            }
                        }
                    }
                } else if class == "vu" {
                    let idx = 5 + vu_receive_arg_index();
                    if let Some(tok) = tokens.get(idx) {
                        let n = tok.trim_end_matches(';');
                        if n != "empty" && n != "-" {
                            names.insert(n.to_string());
                        }
                    }
                }
            }
            EntryKind::FloatAtom | EntryKind::SymbolAtom => {
                for pos in [8, 9] {
                    let toks: Vec<&str> = e.raw.split_whitespace().collect();
                    if let Some(tok) = toks.get(pos) {
                        let n = tok.trim_end_matches(';');
                        if n != "empty" && n != "-" {
                            names.insert(n.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    names
}

pub fn run(
    target: &str,
    from: &str,
    to: &str,
    in_place: bool,
    backup: bool,
    dry_run: bool,
    force: bool,
) -> Result<String, PdtkError> {
    let files = io::scan_pd_files(target)?;
    let from_escaped = escape_pd_dollars(from);
    let to_escaped = escape_pd_dollars(to);

    // --- Collect all entries from all files to check if `to` already exists ---
    if !force {
        for file in &files {
            let Ok(input) = std::fs::read_to_string(file) else {
                continue;
            };
            let tok = tokenize_entries(&input);
            let entries = build_entries(&tok.entries);
            let names = collect_sr_names(&entries);
            if names.contains(&to_escaped) {
                return Err(PdtkError::Usage(format!(
                    "target name '{}' already exists in {} — use --force to override",
                    to,
                    file.display()
                )));
            }
        }
    }

    let mut report_lines: Vec<String> = Vec::new();
    let mut total_replacements = 0usize;

    for file in &files {
        let Ok(input) = std::fs::read_to_string(file) else {
            continue;
        };
        let tok = tokenize_entries(&input);
        let mut entries = build_entries(&tok.entries);

        let mut file_replacements = 0usize;
        for e in entries.iter_mut() {
            if let Some(new_raw) = rename_in_entry(&e.raw, &e.kind, &from_escaped, &to_escaped) {
                report_lines.push(format!(
                    "{}: {} → {}",
                    file.display(),
                    e.raw.trim(),
                    new_raw.trim()
                ));
                e.raw = new_raw;
                file_replacements += 1;
            }
        }

        total_replacements += file_replacements;

        if file_replacements == 0 {
            // No changes — do not write (byte-identical guarantee)
            continue;
        }

        if !dry_run {
            if !in_place {
                return Err(PdtkError::Usage(
                    "rename-send requires --in-place to write changes".to_string(),
                ));
            }

            let patch = pd_toolkit::model::Patch {
                entries,
                warnings: Vec::new(),
            };
            let serialized = serialize(&patch);
            io::write_with_backup(file.to_str().unwrap_or(""), &serialized, backup)?;
        }
    }

    let mut out = if dry_run {
        format!(
            "DRY RUN — {} replacement(s) would be made:\n",
            total_replacements
        )
    } else {
        format!("{} replacement(s) made:\n", total_replacements)
    };
    for line in &report_lines {
        out.push_str(&format!("  {line}\n"));
    }
    Ok(out.trim_end().to_string())
}
