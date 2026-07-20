use crate::errors::PdtkError;
use crate::io;
use pdtk::model::{EntryKind, TemplateFieldType, parse_scalar, parse_struct};
use pdtk::parser::parse;
use serde::Serialize;
use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

#[derive(Debug, Serialize)]
struct FieldJson {
    #[serde(rename = "type")]
    field_type: &'static str,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    array_template: Option<String>,
}

#[derive(Debug, Serialize)]
struct TemplateJson {
    name: String,
    fields: Vec<FieldJson>,
    /// Number of scalar (float/symbol) fields.
    scalar_fields: usize,
}

#[derive(Debug, Serialize)]
struct ScalarJson {
    template: String,
    depth: usize,
    index: usize,
    /// Number of flat values supplied (before the first `\;`).
    values: usize,
    /// Whether a matching `#N struct` was found in the same file.
    template_found: bool,
}

#[derive(Debug, Serialize)]
struct FileStructs {
    file: String,
    templates: Vec<TemplateJson>,
    scalars: Vec<ScalarJson>,
}

fn field_type_str(t: TemplateFieldType) -> &'static str {
    match t {
        TemplateFieldType::Float => "float",
        TemplateFieldType::Symbol => "symbol",
        TemplateFieldType::Text => "text",
        TemplateFieldType::Array => "array",
    }
}

fn analyse(file: &Path) -> Option<FileStructs> {
    let content = crate::io::read_patch_lenient(file).ok()?;
    let patch = parse(&content).ok()?;

    let mut templates = Vec::new();
    let mut template_names: HashSet<String> = HashSet::new();
    for e in &patch.entries {
        if e.kind == EntryKind::Struct
            && let Some(t) = parse_struct(&e.raw)
        {
            template_names.insert(t.name.clone());
            templates.push(TemplateJson {
                name: t.name.clone(),
                scalar_fields: t.scalar_field_count(),
                fields: t
                    .fields
                    .iter()
                    .map(|f| FieldJson {
                        field_type: field_type_str(f.field_type),
                        name: f.name.clone(),
                        array_template: f.array_template.clone(),
                    })
                    .collect(),
            });
        }
    }

    let mut scalars = Vec::new();
    for e in &patch.entries {
        if e.kind != EntryKind::Scalar {
            continue;
        }
        let Some((tmpl, flat)) = parse_scalar(&e.raw) else {
            continue;
        };
        scalars.push(ScalarJson {
            template_found: template_names.contains(&tmpl),
            template: tmpl,
            depth: e.depth.saturating_sub(1),
            index: e.object_index.unwrap_or(0),
            values: flat.len(),
        });
    }

    Some(FileStructs {
        file: file.display().to_string(),
        templates,
        scalars,
    })
}

fn render_field(f: &FieldJson) -> String {
    match &f.array_template {
        Some(sub) => format!("array {} ({sub})", f.name),
        None => format!("{} {}", f.field_type, f.name),
    }
}

/// Run the `structs` command: list `#N struct` templates and `#X scalar`
/// instances in a file or directory tree.
pub fn run(target: &str, json: bool) -> Result<String, PdtkError> {
    let files = io::scan_pd_files(target)?;
    let mut all: Vec<FileStructs> = Vec::new();
    for f in &files {
        if let Some(fs) = analyse(f)
            && (!fs.templates.is_empty() || !fs.scalars.is_empty())
        {
            all.push(fs);
        }
    }

    if json {
        return Ok(serde_json::to_string_pretty(&all)?);
    }

    if all.is_empty() {
        return Ok("No data-structure templates or scalars found".to_string());
    }

    let mut out = String::new();
    for fs in &all {
        let _ = writeln!(out, "{}:", fs.file);
        if !fs.templates.is_empty() {
            let _ = writeln!(out, "  templates ({}):", fs.templates.len());
            for t in &fs.templates {
                let fields: Vec<String> = t.fields.iter().map(render_field).collect();
                let _ = writeln!(out, "    {}: {}", t.name, fields.join(", "));
            }
        }
        if !fs.scalars.is_empty() {
            let _ = writeln!(out, "  scalars ({}):", fs.scalars.len());
            for s in &fs.scalars {
                let missing = if s.template_found {
                    ""
                } else {
                    " (undefined template)"
                };
                let _ = writeln!(
                    out,
                    "    [{}:{}] {} ({} value{}){}",
                    s.depth,
                    s.index,
                    s.template,
                    s.values,
                    if s.values == 1 { "" } else { "s" },
                    missing
                );
            }
        }
    }
    Ok(out.trim_end().to_string())
}
