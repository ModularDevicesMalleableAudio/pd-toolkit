use crate::commands::common::{delete_object, validate_patch};
use crate::errors::PdtkError;
use crate::io;
use pdtk::model::EntryKind;
use pdtk::parser::{build_entries, parse, tokenize_entries};
use pdtk::rewrite::serialize;
use serde::Serialize;
use std::fmt::Write;

#[derive(Debug, Serialize, Clone)]
struct DisplayRow {
    file: String,
    depth: usize,
    index: usize,
    text: String,
    connected: bool,
    /// Canvas the object belongs to — the deletion scope. Sibling subpatches
    /// at the same depth have independent index spaces.
    #[serde(skip)]
    canvas_id: usize,
}

fn is_display(entry: &pdtk::model::Entry, include_labels: bool) -> bool {
    match entry.kind {
        EntryKind::FloatAtom | EntryKind::SymbolAtom | EntryKind::ListAtom => true,
        EntryKind::Obj => {
            let class = entry.class();
            class == "nbx" || class == "vu" || (include_labels && class == "cnv")
        }
        _ => false,
    }
}

/// Inputs for the `find-displays` command.
pub struct RunArgs<'a> {
    /// File or directory to scan.
    pub target: &'a str,
    /// Optional depth filter.
    pub depth: Option<usize>,
    /// Emit JSON output.
    pub json: bool,
    /// Delete matching objects.
    pub delete: bool,
    /// Write changes to disk.
    pub in_place: bool,
    /// Create backups before overwrite.
    pub backup: bool,
    /// Include unconnected displays.
    pub include_unconnected: bool,
    /// Include label canvases (`cnv`) as displays.
    pub include_labels: bool,
}

pub fn run(args: RunArgs<'_>) -> Result<String, PdtkError> {
    let RunArgs {
        target,
        depth,
        json,
        delete,
        in_place,
        backup,
        include_unconnected,
        include_labels,
    } = args;

    let files = io::scan_pd_files(target)?;
    let mut rows = Vec::new();

    for file in &files {
        let Ok(input) = std::fs::read_to_string(file) else {
            continue;
        };
        let Ok(patch) = parse(&input) else { continue };

        for e in &patch.entries {
            let Some(idx) = e.object_index else { continue };
            let user_depth = e.depth.saturating_sub(1);
            if let Some(d) = depth
                && d != user_depth
            {
                continue;
            }
            if !is_display(e, include_labels) {
                continue;
            }

            let Some(cid) = e.canvas_id else { continue };
            // Connectivity is per canvas: a same-index connection in a
            // sibling subpatch must not make this object look connected.
            let connected = patch
                .connections_in_canvas(cid)
                .iter()
                .any(|c| c.src == idx || c.dst == idx);
            if connected || include_unconnected {
                rows.push(DisplayRow {
                    file: file.display().to_string(),
                    depth: user_depth,
                    index: idx,
                    text: e.raw.clone(),
                    connected,
                    canvas_id: cid,
                });
            }
        }
    }

    if delete {
        use std::collections::BTreeMap;
        if !in_place {
            return Err(PdtkError::Usage(
                "--delete requires --in-place for find-displays".to_string(),
            ));
        }

        let mut per_file: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
        for r in &rows {
            per_file
                .entry(r.file.clone())
                .or_default()
                .push((r.canvas_id, r.index));
        }

        for (file, mut dels) in per_file {
            let input = std::fs::read_to_string(&file)?;
            let tok = tokenize_entries(&input);
            let mut entries = build_entries(&tok.entries);

            // delete highest canvas/index first to avoid index drift
            dels.sort_by(|a, b| b.cmp(a));
            for (cid, i) in dels {
                let _ = delete_object(&mut entries, cid, i);
            }

            let patch = pdtk::model::Patch {
                entries,
                warnings: Vec::new(),
            };
            let errors = validate_patch(&patch);
            if !errors.is_empty() {
                return Err(PdtkError::Usage(format!(
                    "validation failed after deleting displays in {}: {}",
                    file,
                    errors.join("; ")
                )));
            }

            io::write_with_backup(&file, &serialize(&patch), backup)?;
        }
    }

    if json {
        return Ok(serde_json::to_string_pretty(&rows)?);
    }

    if rows.is_empty() {
        return Ok("No display objects found".to_string());
    }

    let mut out = String::new();
    for r in rows {
        let _ = writeln!(
            out,
            "{} [depth:{} index:{} connected:{}] {}",
            r.file, r.depth, r.index, r.connected, r.text
        );
    }
    if delete {
        out.push_str("Deleted display objects in-place\n");
    }
    Ok(out.trim_end().to_string())
}
