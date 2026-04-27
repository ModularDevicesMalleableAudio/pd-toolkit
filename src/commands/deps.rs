use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::analysis::deps::{DepEntry, analyse_file_with_extra};
use std::collections::HashSet;
use std::path::PathBuf;

/// Compute platform-specific Pd external search paths, given a HOME directory.
/// Pure function so it can be unit-tested with a synthetic `HOME`.
/// Returns paths in fallback order; non-existent paths are NOT filtered here.
pub fn pd_platform_paths(home: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if cfg!(target_os = "linux") {
        paths.push(PathBuf::from("/usr/local/lib/pd-externals"));
        paths.push(PathBuf::from("/usr/lib/pd/extra"));
        paths.push(home.join(".local/lib/pd/extra"));
        paths.push(home.join("pd-externals"));
    } else if cfg!(target_os = "macos") {
        paths.push(PathBuf::from("/Library/Pd"));
        paths.push(home.join("Library/Pd"));
    }
    paths
}

pub fn run(
    target: &str,
    recursive: bool,
    missing_only: bool,
    json: bool,
    search_paths: &[String],
    pd_path: bool,
) -> Result<String, PdtkError> {
    let files = io::scan_pd_files(target)?;
    let mut visited = HashSet::new();
    let mut all: Vec<DepEntry> = Vec::new();

    let mut extra_dirs: Vec<PathBuf> = search_paths.iter().map(PathBuf::from).collect();
    if pd_path {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/"));
        for p in pd_platform_paths(&home) {
            if p.exists() {
                extra_dirs.push(p);
            }
        }
    }

    for file in &files {
        let entries = analyse_file_with_extra(file, recursive, &mut visited, &extra_dirs);
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
