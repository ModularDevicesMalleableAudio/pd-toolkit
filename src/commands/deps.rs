use crate::errors::PdtkError;
use crate::io;
use pdtk::analysis::buses;
use pdtk::analysis::deps::{BuiltinSource, DepEntry, analyse_file_with_extra};
use std::collections::HashSet;
use std::fmt::Write;
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

#[allow(clippy::too_many_arguments)]
pub fn run(
    target: &str,
    recursive: bool,
    missing_only: bool,
    json: bool,
    search_paths: &[String],
    pd_path: bool,
    buses_mode: bool,
    per_file: bool,
) -> Result<String, PdtkError> {
    let files = io::scan_pd_files(target)?;

    if buses_mode {
        return run_buses(&files, per_file, json, recursive, search_paths, pd_path);
    }

    let mut visited = HashSet::new();
    let mut all: Vec<DepEntry> = Vec::new();

    let mut extra_dirs: Vec<PathBuf> = search_paths.iter().map(PathBuf::from).collect();
    if pd_path {
        let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
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
        // A class covered by a declared library cannot be confirmed missing,
        // so it is excluded from --missing (only genuinely unresolvable
        // classes with no library declared remain).
        all.retain(|e| !e.found && e.declared_libs.is_empty());
    }

    if json {
        return Ok(serde_json::to_string_pretty(&all)?);
    }

    if all.is_empty() {
        return Ok("No abstraction dependencies found".to_string());
    }

    let mut out = String::new();
    for e in &all {
        let status = if matches!(e.source, Some(BuiltinSource::CoreExtra)) {
            "core-extra (declare may be required)".to_string()
        } else if e.found {
            format!("found:{}", e.found_at.as_deref().unwrap_or(""))
        } else if !e.declared_libs.is_empty() {
            format!(
                "unresolved (declared lib: {} — cannot verify)",
                e.declared_libs.join(", ")
            )
        } else {
            "MISSING".to_string()
        };
        let _ = writeln!(
            out,
            "{} [depth:{} index:{}] {} ({})",
            e.file, e.depth, e.index, e.name, status
        );
    }
    Ok(out.trim_end().to_string())
}

fn run_buses(
    files: &[PathBuf],
    per_file: bool,
    json: bool,
    recursive: bool,
    search_paths: &[String],
    pd_path: bool,
) -> Result<String, PdtkError> {
    let report = buses::audit(files, per_file);

    // Recursive mode: also derive per-abstraction bus contracts and report
    // unsatisfied buses at each call site.
    let unsatisfied = if recursive {
        let mut extra_dirs: Vec<PathBuf> = search_paths.iter().map(PathBuf::from).collect();
        if pd_path {
            let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
            for p in pd_platform_paths(&home) {
                if p.exists() {
                    extra_dirs.push(p);
                }
            }
        }
        let mut all = Vec::new();
        for f in files {
            let file_dir = f.parent().unwrap_or(std::path::Path::new("."));
            let search: Vec<PathBuf> = std::iter::once(file_dir.to_path_buf())
                .chain(extra_dirs.iter().cloned())
                .collect();
            let resolve = |name: &str| -> Option<PathBuf> {
                let fname = format!("{name}.pd");
                for d in &search {
                    let p = d.join(&fname);
                    if p.exists() {
                        return Some(p);
                    }
                }
                None
            };
            all.extend(buses::unsatisfied_contracts(f, resolve));
        }
        all
    } else {
        Vec::new()
    };

    if json {
        #[derive(serde::Serialize)]
        struct Combined<'a> {
            buses: &'a [buses::BusReport],
            #[serde(skip_serializing_if = "Vec::is_empty")]
            unsatisfied_contracts: Vec<buses::UnsatisfiedBus>,
        }
        if recursive {
            let c = Combined {
                buses: &report,
                unsatisfied_contracts: unsatisfied,
            };
            return Ok(serde_json::to_string_pretty(&c)?);
        } else {
            return Ok(serde_json::to_string_pretty(&report)?);
        }
    }

    if report.is_empty() && unsatisfied.is_empty() {
        return Ok("No send/receive buses found".to_string());
    }

    let mut out = String::new();
    for row in &report {
        let kind = match row.kind {
            pdtk::analysis::send_receive::BusKind::Control => "control",
            pdtk::analysis::send_receive::BusKind::Signal => "signal",
            pdtk::analysis::send_receive::BusKind::AudioSum => "audio_sum",
        };
        let status = match row.status {
            buses::BusStatus::Matched => "matched",
            buses::BusStatus::OrphanSend => "orphan_send (no receivers)",
            buses::BusStatus::OrphanReceive => "orphan_receive (no senders)",
        };
        let warn = row
            .scope_warning
            .map(|w| format!(" [{w}]"))
            .unwrap_or_default();
        let _ = writeln!(out, "bus '{}' ({}) {}{}", row.name, kind, status, warn);
        for s in &row.senders {
            let _ = writeln!(
                out,
                "  send:    {} [depth:{} index:{}]",
                s.file, s.depth, s.index
            );
        }
        for r in &row.receivers {
            let _ = writeln!(
                out,
                "  receive: {} [depth:{} index:{}]",
                r.file, r.depth, r.index
            );
        }
    }
    if !unsatisfied.is_empty() {
        out.push_str("\nUnsatisfied bus contracts:\n");
        for u in &unsatisfied {
            let kind = match u.bus_kind {
                pdtk::analysis::send_receive::BusKind::Control => "control",
                pdtk::analysis::send_receive::BusKind::Signal => "signal",
                pdtk::analysis::send_receive::BusKind::AudioSum => "audio_sum",
            };
            let dir = match u.direction {
                buses::ContractDirection::NeedsSender => "needs_sender",
                buses::ContractDirection::NeedsReceiver => "needs_receiver",
            };
            let _ = writeln!(
                out,
                "  {} [depth:{} index:{}] calls '{}' — {} '{}' ({}) [{}]",
                u.caller_file,
                u.caller_depth,
                u.caller_index,
                u.abstraction,
                dir,
                u.bus_name,
                kind,
                u.abstraction_path,
            );
        }
    }
    Ok(out.trim_end().to_string())
}
