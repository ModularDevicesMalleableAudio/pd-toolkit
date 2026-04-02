use crate::errors::PdtkError;
use crate::io;
use pd_toolkit::parser::parse;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Serialize)]
struct FileStats {
    file: String,
    objects: usize,
    connections: usize,
    max_depth: usize,
    class_histogram: BTreeMap<String, usize>,
    max_fanin: usize,
    max_fanout: usize,
    orphans: usize,
    displays: usize,
    arrays: usize,
}

#[derive(Debug, Serialize)]
struct StatsReport {
    files: Vec<FileStats>,
    total_files: usize,
    total_objects: usize,
    total_connections: usize,
}

fn is_display(class: &str) -> bool {
    matches!(class, "floatatom" | "symbolatom" | "nbx" | "vu")
}

pub fn run(target: &str, json: bool) -> Result<String, PdtkError> {
    let files = io::scan_pd_files(target)?;
    let mut report_rows = Vec::new();

    for file in files {
        let Ok(input) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(patch) = parse(&input) else { continue };
        let objects: Vec<_> = patch
            .entries
            .iter()
            .filter(|e| e.object_index.is_some())
            .collect();
        let connections: Vec<_> = patch
            .entries
            .iter()
            .filter(|e| e.kind == pd_toolkit::model::EntryKind::Connect)
            .collect();

        let mut class_hist = BTreeMap::new();
        let mut fanin: HashMap<(usize, usize), usize> = HashMap::new(); // (depth,index)
        let mut fanout: HashMap<(usize, usize), usize> = HashMap::new();

        for e in &objects {
            *class_hist.entry(e.class().to_string()).or_insert(0) += 1;
        }

        for d in 0..=patch.max_depth() {
            for c in patch.connections_at_depth(d) {
                *fanout.entry((d, c.src)).or_insert(0) += 1;
                *fanin.entry((d, c.dst)).or_insert(0) += 1;
            }
        }

        let mut orphans = 0usize;
        let mut displays = 0usize;
        for e in &objects {
            let idx = e.object_index.unwrap();
            let d = e.depth.saturating_sub(1);
            let key = (d, idx);
            let degree =
                fanin.get(&key).copied().unwrap_or(0) + fanout.get(&key).copied().unwrap_or(0);
            if degree == 0 && e.kind != pd_toolkit::model::EntryKind::Text {
                orphans += 1;
            }
            if is_display(e.class()) {
                displays += 1;
            }
        }

        let arrays = patch
            .entries
            .iter()
            .filter(|e| e.kind == pd_toolkit::model::EntryKind::Array)
            .count();

        report_rows.push(FileStats {
            file: file.display().to_string(),
            objects: objects.len(),
            connections: connections.len(),
            max_depth: patch.max_depth(),
            class_histogram: class_hist,
            max_fanin: fanin.values().copied().max().unwrap_or(0),
            max_fanout: fanout.values().copied().max().unwrap_or(0),
            orphans,
            displays,
            arrays,
        });
    }

    let total_files = report_rows.len();
    let total_objects = report_rows.iter().map(|r| r.objects).sum();
    let total_connections = report_rows.iter().map(|r| r.connections).sum();

    let report = StatsReport {
        files: report_rows,
        total_files,
        total_objects,
        total_connections,
    };

    if json {
        return Ok(serde_json::to_string_pretty(&report)?);
    }

    let mut out = String::new();
    for f in &report.files {
        out.push_str(&format!(
            "{}: objects={} connections={} max_depth={} max_fanin={} max_fanout={} orphans={} displays={} arrays={}\n",
            f.file, f.objects, f.connections, f.max_depth, f.max_fanin, f.max_fanout, f.orphans, f.displays, f.arrays
        ));
    }
    out.push_str(&format!(
        "TOTAL files={} objects={} connections={}",
        report.total_files, report.total_objects, report.total_connections
    ));
    Ok(out)
}
