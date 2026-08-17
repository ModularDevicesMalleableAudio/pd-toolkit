use crate::commands::arrays::{
    ArraysConfig, KindFilter, Row, Schema, TemplateFilter, collect_rows, data_to_json,
    format_number,
};
use crate::errors::PdtkError;
use serde_json::json;
use std::fmt::Write;

/// Configuration for the `array-data` command.
#[derive(Debug, Clone, Default)]
pub struct ArrayDataConfig {
    pub json: bool,
    pub verbose: bool,
}

/// Run `pdtk array-data`: dump the saved contents of one named array.
pub fn run(target: &str, name: &str, cfg: ArrayDataConfig) -> Result<String, PdtkError> {
    let scan = ArraysConfig {
        schema: Schema::V2,
        kind: Some(KindFilter::All),
        templates: TemplateFilter::Include,
        json: false,
        verbose: cfg.verbose,
        data: true,
    };
    let rows = collect_rows(target, &scan, KindFilter::All)?;
    let matches: Vec<&Row> = rows.iter().filter(|r| r.name == name).collect();

    let row = match matches.as_slice() {
        [] => {
            return Err(PdtkError::Usage(format!(
                "no array named `{name}` in {target}"
            )));
        }
        [only] => *only,
        many => {
            let mut msg = format!("array name `{name}` is ambiguous ({} matches):", many.len());
            for r in many {
                let _ = write!(
                    msg,
                    "\n  {} [depth:{}] {}",
                    r.file,
                    r.depth,
                    r.kind.as_str()
                );
            }
            msg.push_str("\nnarrow the target to a single file");
            return Err(PdtkError::Usage(msg));
        }
    };

    // The scan above requests data, so every row carries contents — an array
    // that saves nothing yields contents with `saved() == false`.
    let Some(data) = row.data.as_ref() else {
        return Err(PdtkError::Usage(format!(
            "internal: no contents read for array `{name}` in {}",
            row.file
        )));
    };
    if !data.saved() {
        return Err(PdtkError::Usage(format!(
            "array `{name}` in {} saves no contents \
             (declared without -k / with an even save flag), \
             so PD loads it as {} zeros",
            row.file, row.size
        )));
    }

    if cfg.json {
        let payload = json!({
            "file": row.file,
            "name": row.name,
            "kind": row.kind.as_str(),
            "depth": row.depth,
            "index": row.index,
            "size": row.size,
            "data": data_to_json(data),
        });
        return Ok(serde_json::to_string_pretty(&payload)?);
    }

    if cfg.verbose && data.values.len() < row.size {
        eprintln!(
            "note: {}: `{name}` saves {} of {} elements; PD loads the rest as 0",
            row.file,
            data.values.len(),
            row.size
        );
    }
    let mut out = String::new();
    for (i, v) in data.values.iter().enumerate() {
        let _ = writeln!(out, "{i} {}", format_number(*v));
    }
    Ok(out.trim_end().to_string())
}
