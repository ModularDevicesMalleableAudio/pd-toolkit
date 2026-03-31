use crate::errors::PdtkError;
use pd_toolkit::{model::EntryKind, parser::parse, rewrite::serialize};
use serde::Serialize;

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
    let input = std::fs::read_to_string(file)?;
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
    out.push_str(&format!("Objects: {}\n", summary.objects));
    out.push_str(&format!("Connections: {}\n", summary.connections));
    out.push_str(&format!("Max depth: {}\n", summary.max_depth));
    out.push_str(&format!("Canvases: {}", summary.canvases));

    if summary.warnings.is_empty() {
        out.push_str("\nWarnings: 0");
    } else {
        out.push_str(&format!("\nWarnings: {}", summary.warnings.len()));
        for w in &summary.warnings {
            out.push_str(&format!("\n- {w}"));
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
        out.push_str(&format!("\n  Object entries: {obj_count}"));
        out.push_str(&format!("\n  Connect entries: {conn_count}"));
        out.push_str(&format!("\n  Other entries: {other_count}"));
        out.push_str(&format!("\n  Total entries: {}", patch.entries.len()));
    }

    Ok(out)
}
