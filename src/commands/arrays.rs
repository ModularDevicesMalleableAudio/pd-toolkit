use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::model::EntryKind;
use pd_toolkit::parser::parse;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Clone)]
struct ArrayRow {
    file: String,
    depth: usize,
    name: String,
    size: usize,
}

#[derive(Debug, Serialize)]
struct ArraysReport {
    arrays: Vec<ArrayRow>,
    duplicate_names: BTreeMap<String, Vec<String>>,
}

pub fn run(target: &str, json: bool) -> Result<String, PdtkError> {
    let files = io::scan_pd_files(target)?;
    let mut rows = Vec::new();

    for file in files {
        let Ok(input) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(patch) = parse(&input) else { continue };
        for e in &patch.entries {
            if e.kind != EntryKind::Array {
                continue;
            }
            let parts: Vec<&str> = e
                .raw
                .trim()
                .trim_end_matches(';')
                .split_whitespace()
                .collect();
            // #X array <name> <size> ...
            if parts.len() < 5 {
                continue;
            }
            let name = parts[2].to_string();
            let size = parts[3].parse::<usize>().unwrap_or(0);
            rows.push(ArrayRow {
                file: file.display().to_string(),
                depth: e.depth.saturating_sub(1),
                name,
                size,
            });
        }
    }

    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in &rows {
        by_name
            .entry(r.name.clone())
            .or_default()
            .push(r.file.clone());
    }
    let duplicate_names: BTreeMap<String, Vec<String>> = by_name
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .collect();

    if json {
        return Ok(serde_json::to_string_pretty(&ArraysReport {
            arrays: rows,
            duplicate_names,
        })?);
    }

    if rows.is_empty() {
        return Ok("No arrays found".to_string());
    }

    let mut out = String::new();
    for r in &rows {
        out.push_str(&format!(
            "{} [depth:{}] array {} size {}\n",
            r.file, r.depth, r.name, r.size
        ));
    }
    if !duplicate_names.is_empty() {
        out.push_str("Duplicate array names:\n");
        for (name, files) in duplicate_names {
            out.push_str(&format!("- {}: {}\n", name, files.join(", ")));
        }
    }
    Ok(out.trim_end().to_string())
}
