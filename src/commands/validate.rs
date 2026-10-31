use crate::errors::PdtkError;
use pdtk::{analysis::check::check_patch, parser::parse};
use serde::Serialize;
use std::fmt::Write;
use std::path::Path;

#[derive(Debug)]
pub struct ValidateResult {
    pub output: String,
    pub exit_code: i32,
}

#[derive(Debug, Serialize)]
struct ValidateJson {
    valid: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
}

pub fn run(
    file: &str,
    strict: bool,
    json: bool,
    output: Option<&str>,
) -> Result<ValidateResult, PdtkError> {
    let input = crate::io::read_patch_lenient(file)?;
    let patch = parse(&input)?;
    // Warnings (parser warnings, suspicious-but-loadable findings, and the
    // --strict duplicate check) do NOT cause exit 1.
    let findings = check_patch(&patch, Path::new(file), &input, strict);
    let (errors, warnings) = (findings.errors, findings.warnings);

    let valid = errors.is_empty();
    let exit_code = i32::from(!valid);

    let text = if json {
        serde_json::to_string_pretty(&ValidateJson {
            valid,
            errors,
            warnings,
        })?
    } else if valid {
        if warnings.is_empty() {
            "OK: patch is valid".to_string()
        } else {
            let mut s = format!("OK: patch is valid ({} warning(s))", warnings.len());
            for w in &warnings {
                let _ = write!(s, "\n- WARNING: {w}");
            }
            s
        }
    } else {
        let mut s = format!("INVALID: {} error(s)", errors.len());
        for e in &errors {
            let _ = write!(s, "\n- {e}");
        }
        if !warnings.is_empty() {
            let _ = write!(s, "\n{} warning(s)", warnings.len());
            for w in &warnings {
                let _ = write!(s, "\n- WARNING: {w}");
            }
        }
        s
    };

    if let Some(out_path) = output {
        std::fs::write(out_path, &text)?;
        return Ok(ValidateResult {
            output: String::new(),
            exit_code,
        });
    }

    Ok(ValidateResult {
        output: text,
        exit_code,
    })
}
