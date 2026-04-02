use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::analysis::deps::{DepEntry, analyse_file};
use std::collections::HashSet;

pub fn run(
    target: &str,
    recursive: bool,
    missing_only: bool,
    json: bool,
) -> Result<String, PdtkError> {
    let files = io::scan_pd_files(target)?;
    let mut visited = HashSet::new();
    let mut all: Vec<DepEntry> = Vec::new();

    for file in &files {
        let entries = analyse_file(file, recursive, &mut visited);
        all.extend(entries);
    }

    // Deduplicate by (file, name)
    let mut seen_pairs: HashSet<(String, String)> = HashSet::new();
    all.retain(|e| seen_pairs.insert((e.file.clone(), e.name.clone())));

    if missing_only {
        all.retain(|e| !e.found);
    }

    if json {
        return Ok(serde_json::to_string_pretty(&all)?);
    }

    if all.is_empty() {
        return Ok("No abstraction dependencies found".to_string());
    }

    let mut out = String::new();
    for e in &all {
        let status = if e.found {
            format!("found:{}", e.found_at.as_deref().unwrap_or(""))
        } else {
            "MISSING".to_string()
        };
        out.push_str(&format!(
            "{} [depth:{} index:{}] {} ({})\n",
            e.file, e.depth, e.index, e.name, status
        ));
    }
    Ok(out.trim_end().to_string())
}
