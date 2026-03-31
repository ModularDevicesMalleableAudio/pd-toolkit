use crate::errors::PdtkError;
use crate::io;
use glob::Pattern;
use pd_toolkit::parser::parse;
use regex::RegexBuilder;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct SearchRow {
    file: String,
    depth: usize,
    index: usize,
    class: String,
    text: String,
}

pub fn run(
    target: &str,
    obj_type: Option<&str>,
    text: Option<&str>,
    depth: Option<usize>,
    json: bool,
    regex: bool,
    case_sensitive: bool,
) -> Result<String, PdtkError> {
    let files = io::scan_pd_files(target)?;

    let regex_matcher = if let Some(p) = text {
        if regex {
            Some(
                RegexBuilder::new(p)
                    .case_insensitive(!case_sensitive)
                    .build()
                    .map_err(|e| PdtkError::Usage(format!("invalid regex: {e}")))?,
            )
        } else {
            None
        }
    } else {
        None
    };

    let glob_matcher = if let Some(p) = text {
        if !regex {
            Some(Pattern::new(p).map_err(|e| PdtkError::Usage(format!("invalid glob: {e}")))?)
        } else {
            None
        }
    } else {
        None
    };

    let mut rows = Vec::new();

    for file in files {
        let Ok(input) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(patch) = parse(&input) else { continue };
        for e in &patch.entries {
            let Some(index) = e.object_index else {
                continue;
            };
            let d = e.depth.saturating_sub(1);
            if let Some(wanted) = depth
                && wanted != d
            {
                continue;
            }

            let class = e.class().to_string();
            if let Some(t) = obj_type {
                let ok = if case_sensitive {
                    class == t
                } else {
                    class.eq_ignore_ascii_case(t)
                };
                if !ok {
                    continue;
                }
            }

            let full_text = if e.kind == pd_toolkit::model::EntryKind::Obj {
                let mut s = class.clone();
                let args = e.args();
                if !args.is_empty() {
                    s.push(' ');
                    s.push_str(&args.join(" "));
                }
                s
            } else {
                e.raw.clone()
            };

            if let Some(re) = &regex_matcher
                && !re.is_match(&full_text)
            {
                continue;
            }
            if let Some(g) = &glob_matcher {
                let candidate = if case_sensitive {
                    full_text.clone()
                } else {
                    full_text.to_lowercase()
                };
                let pat = if case_sensitive {
                    g.clone()
                } else {
                    Pattern::new(&g.as_str().to_lowercase())
                        .map_err(|e| PdtkError::Usage(format!("invalid glob: {e}")))?
                };
                if !pat.matches(&candidate) {
                    continue;
                }
            }

            rows.push(SearchRow {
                file: file.display().to_string(),
                depth: d,
                index,
                class,
                text: full_text,
            });
        }
    }

    if json {
        return Ok(serde_json::to_string_pretty(&rows)?);
    }

    if rows.is_empty() {
        return Ok("No matches".to_string());
    }

    Ok(rows
        .into_iter()
        .map(|r| {
            format!(
                "{} [depth:{} index:{} class:{}] {}",
                r.file, r.depth, r.index, r.class, r.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}
