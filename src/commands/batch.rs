use crate::errors::PdtkError;
use crate::io;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct FileResult {
    file: String,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct BatchReport {
    total: usize,
    succeeded: usize,
    failed: usize,
    results: Vec<FileResult>,
}

/// Run `pdtk <command> <args...>` against every `.pd` file under `dir`
/// that matches `glob_pattern`.
///
/// `command_args` is the full argument list that would follow `pdtk`, with
/// the file placeholder left out; the file is appended as the first positional
/// argument, followed by any extra flags.
///
/// When `dry_run` is true we report what would be done without executing.
/// When `continue_on_error` is false the batch stops at the first failure.
pub struct BatchResult {
    pub output: String,
    pub exit_code: i32,
}

pub fn run(
    dir: &str,
    command_args: &[&str],
    glob_pattern: &str,
    dry_run: bool,
    continue_on_error: bool,
    json: bool,
) -> Result<BatchResult, PdtkError> {
    let mut files = io::scan_pd_files(dir)?;

    // Apply glob filter if non-default
    if glob_pattern != "**/*.pd" && !glob_pattern.is_empty() {
        let pat = glob::Pattern::new(glob_pattern)
            .map_err(|e| PdtkError::Usage(format!("invalid glob: {e}")))?;
        files.retain(|f| {
            let name = f
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            pat.matches(&name)
        });
    }

    let total = files.len();
    let mut results: Vec<FileResult> = Vec::new();
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for file in &files {
        let file_str = file.display().to_string();

        if dry_run {
            results.push(FileResult {
                file: file_str.clone(),
                status: "would_run",
                error: None,
            });
            succeeded += 1;
            continue;
        }

        // Build the full argument list: command_args + [file_path]
        let mut args: Vec<String> = command_args.iter().map(|s| s.to_string()).collect();
        args.push(file_str.clone());

        // Execute pdtk as a subprocess
        let binary = current_binary()?;
        let output = std::process::Command::new(&binary)
            .args(&args)
            .output()
            .map_err(|e| PdtkError::Usage(format!("failed to spawn pdtk: {e}")))?;

        if output.status.success() {
            results.push(FileResult {
                file: file_str,
                status: "ok",
                error: None,
            });
            succeeded += 1;
        } else {
            let err = String::from_utf8_lossy(&output.stderr).to_string();
            results.push(FileResult {
                file: file_str,
                status: "error",
                error: Some(err.trim().to_string()),
            });
            failed += 1;
            if !continue_on_error {
                break;
            }
        }
    }

    let exit_code = if failed > 0 { 1 } else { 0 };

    let report = BatchReport {
        total: results.len(),
        succeeded,
        failed,
        results,
    };

    if json {
        return Ok(BatchResult {
            output: serde_json::to_string_pretty(&report)?,
            exit_code,
        });
    }

    let mut out = format!(
        "Batch: {}/{} succeeded, {} failed (total scanned: {})\n",
        succeeded, report.total, failed, total
    );
    for r in &report.results {
        match r.status {
            "ok" => out.push_str(&format!("  OK      {}\n", r.file)),
            "error" => out.push_str(&format!(
                "  ERROR   {} — {}\n",
                r.file,
                r.error.as_deref().unwrap_or("unknown error")
            )),
            "would_run" => out.push_str(&format!("  DRY-RUN {}\n", r.file)),
            _ => {}
        }
    }
    Ok(BatchResult {
        output: out.trim_end().to_string(),
        exit_code,
    })
}

/// Return the path to the currently running pdtk binary.
/// Falls back to searching PATH for "pdtk".
fn current_binary() -> Result<PathBuf, PdtkError> {
    // std::env::current_exe gives us our own path in most cases
    if let Ok(p) = std::env::current_exe() {
        return Ok(p);
    }
    // Fallback: look for pdtk on PATH
    which_pdtk()
}

fn which_pdtk() -> Result<PathBuf, PdtkError> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(':') {
        let candidate = PathBuf::from(dir).join("pdtk");
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(PdtkError::Usage("could not locate pdtk binary".to_string()))
}
