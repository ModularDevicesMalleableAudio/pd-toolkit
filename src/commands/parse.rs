use crate::errors::PdtkError;
use pdtk::{model::EntryKind, parser::parse, rewrite::serialize};
use serde::Serialize;
use std::fmt::Write;

#[derive(Debug, Serialize)]
struct ParseSummary {
    objects: usize,
    connections: usize,
    max_depth: usize,
    canvases: usize,
    warnings: Vec<String>,
}

pub fn run(
    file: &str,
    json: bool,
    output: Option<&str>,
    verbose: bool,
) -> Result<String, PdtkError> {
    let input = crate::io::read_patch_lenient(file)?;
    let patch = parse(&input)?;

    // When --output is given, write the re-serialized .pd file there.
    // This proves round-trip fidelity: the output file should be byte-identical
    // to the input.
    if let Some(out_path) = output {
        let serialized = serialize(&patch);
        std::fs::write(out_path, serialized)?;
    }

    let warnings: Vec<String> = patch.warnings.iter().map(|w| format!("{w:?}")).collect();

    let summary = ParseSummary {
        objects: patch
            .entries
            .iter()
            .filter(|e| e.object_index.is_some())
            .count(),
        connections: patch
            .entries
            .iter()
            .filter(|e| e.kind == EntryKind::Connect)
            .count(),
        max_depth: patch.max_depth(),
        canvases: patch.canvas_count(),
        warnings,
    };

    if json {
        return Ok(serde_json::to_string_pretty(&summary)?);
    }

    let mut out = String::new();
    let _ = writeln!(out, "Objects: {}", summary.objects);
    let _ = writeln!(out, "Connections: {}", summary.connections);
    let _ = writeln!(out, "Max depth: {}", summary.max_depth);
    let _ = write!(out, "Canvases: {}", summary.canvases);

    if summary.warnings.is_empty() {
        out.push_str("\nWarnings: 0");
    } else {
        let _ = write!(out, "\nWarnings: {}", summary.warnings.len());
        for w in &summary.warnings {
            let _ = write!(out, "\n- {w}");
        }
    }

    if verbose {
        // Count entries by kind for verbose display
        let mut obj_count = 0usize;
        let mut conn_count = 0usize;
        let mut other_count = 0usize;
        for e in &patch.entries {
            match e.kind {
                EntryKind::Connect => conn_count += 1,
                EntryKind::CanvasOpen
                | EntryKind::Coords
                | EntryKind::Array
                | EntryKind::ArrayData
                | EntryKind::Declare
                | EntryKind::WidthHint
                | EntryKind::Unknown => other_count += 1,
                _ => {
                    if e.object_index.is_some() {
                        obj_count += 1;
                    } else {
                        other_count += 1;
                    }
                }
            }
        }
        out.push_str("\n\nEntry breakdown:");
        let _ = write!(out, "\n  Object entries: {obj_count}");
        let _ = write!(out, "\n  Connect entries: {conn_count}");
        let _ = write!(out, "\n  Other entries: {other_count}");
        let _ = write!(out, "\n  Total entries: {}", patch.entries.len());
    }

    Ok(out)
}
