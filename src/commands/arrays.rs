use crate::errors::PdtkError;
use crate::io;
use pdtk::model::EntryKind;
use pdtk::parser::escape::unescape_pd_token;
use pdtk::parser::parse;
use serde_json::{Number, Value, json};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

/// Schema version emitted by `pdtk arrays`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Schema {
    V1,
    V2,
}

/// Filter for which array kinds to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindFilter {
    Classic,
    Define,
    All,
}

/// Filter for template-named arrays (`$1`..`$9` in the name).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateFilter {
    Include,
    Exclude,
    Only,
}

/// Configuration for the `arrays` command.
#[derive(Debug, Clone)]
pub struct ArraysConfig {
    pub schema: Schema,
    pub kind: Option<KindFilter>,
    pub templates: TemplateFilter,
    pub json: bool,
    pub verbose: bool,
}

impl Default for ArraysConfig {
    fn default() -> Self {
        Self {
            schema: Schema::V2,
            kind: None,
            templates: TemplateFilter::Include,
            json: false,
            verbose: false,
        }
    }
}

#[derive(Debug, Clone)]
struct Row {
    file: String,
    depth: usize,
    index: Option<usize>,
    kind: RowKind,
    name: String,
    size: usize,
    is_template: bool,
    define: Option<DefinePayload>,
    classic: Option<ClassicPayload>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowKind {
    Classic,
    Define,
}

impl RowKind {
    fn as_str(self) -> &'static str {
        match self {
            RowKind::Classic => "classic",
            RowKind::Define => "define",
        }
    }
}

#[derive(Debug, Clone)]
struct DefinePayload {
    k: bool,
    yrange: Option<[Number; 2]>,
    pix: Option<[Number; 2]>,
    discarded: Vec<Discarded>,
}

#[derive(Debug, Clone)]
struct Discarded {
    reason: String,
    tokens: Vec<String>,
}

#[derive(Debug, Clone)]
struct ClassicPayload {
    /// Raw `K` int, or `None` if the trailing token failed to parse.
    save_flag: Option<i64>,
}

impl ClassicPayload {
    fn saveit(&self) -> Option<bool> {
        self.save_flag.map(|k| (k & 1) != 0)
    }
    fn filestyle(&self) -> Option<&'static str> {
        self.save_flag.map(|k| match (k >> 1) & 3 {
            0 => "polygon",
            1 => "points",
            2 => "bezier",
            _ => "reserved",
        })
    }
    fn hidename(&self) -> Option<bool> {
        self.save_flag.map(|k| (k & 8) != 0)
    }
}

/// Run `pdtk arrays`.
pub fn run(target: &str, cfg: ArraysConfig) -> Result<String, PdtkError> {
    // Resolve effective kind filter (default depends on schema).
    let kind_filter = match cfg.kind {
        Some(k) => k,
        None => match cfg.schema {
            Schema::V1 => KindFilter::Classic,
            // First v2 release: default to `classic` with deprecation warning.
            Schema::V2 => {
                if cfg.verbose {
                    eprintln!(
                        "warning: --kind defaults to 'classic' for compatibility; \
                         pass --kind all to include `array define` rows"
                    );
                }
                KindFilter::Classic
            }
        },
    };

    let files = io::scan_pd_files(target)?;

    // Single-file ergonomics: warn (under --verbose) when the input file is
    // not `.pd`, and emit an empty list with exit 0.
    if files.len() == 1 {
        let p = &files[0];
        let is_pd = p.extension().is_some_and(|e| e == "pd");
        if !is_pd && Path::new(target).is_file() {
            if cfg.verbose {
                eprintln!("warning: {}: not a .pd file, skipped", p.display());
            }
            return Ok(render(&[], &cfg, kind_filter));
        }
    }

    let mut rows: Vec<Row> = Vec::new();
    for file in files {
        let Ok(input) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(patch) = parse(&input) else { continue };
        let file_str = file.display().to_string();

        for e in &patch.entries {
            match e.kind {
                EntryKind::Array => {
                    if let Some(row) = parse_classic(&e.raw, &file_str, e.depth) {
                        rows.push(row);
                    } else if cfg.verbose {
                        eprintln!(
                            "warning: {}: malformed #X array entry: {}",
                            file_str,
                            e.raw.trim()
                        );
                    }
                }
                EntryKind::Obj => {
                    if let Some(parsed) =
                        parse_define(&e.raw, &file_str, e.depth, e.object_index, cfg.verbose)
                    {
                        match parsed {
                            DefineParse::Row(row) => rows.push(row),
                            DefineParse::Malformed(msg) => {
                                if cfg.verbose {
                                    eprintln!("warning: {file_str}: {msg}");
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Apply filters.
    let mut rows: Vec<Row> = rows
        .into_iter()
        .filter(|r| match kind_filter {
            KindFilter::All => true,
            KindFilter::Classic => r.kind == RowKind::Classic,
            KindFilter::Define => r.kind == RowKind::Define,
        })
        .filter(|r| match cfg.templates {
            TemplateFilter::Include => true,
            TemplateFilter::Exclude => !r.is_template,
            TemplateFilter::Only => r.is_template,
        })
        .collect();

    // Stable order: file, then (depth, index, name).
    rows.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.depth.cmp(&b.depth))
            .then(match (a.index, b.index) {
                (Some(ai), Some(bi)) => ai.cmp(&bi),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then(a.name.cmp(&b.name))
    });

    Ok(render(&rows, &cfg, kind_filter))
}

enum DefineParse {
    Row(Row),
    Malformed(String),
}

/// Parse an `#X array name N float K;` entry.
fn parse_classic(raw: &str, file: &str, internal_depth: usize) -> Option<Row> {
    let parts: Vec<&str> = raw
        .trim()
        .trim_end_matches(';')
        .split_whitespace()
        .collect();
    // #X array <name> <size> [float K]
    if parts.len() < 4 || parts[0] != "#X" || parts[1] != "array" {
        return None;
    }
    let name_raw = parts[2];
    let size = parts[3].parse::<usize>().ok()?;
    let save_flag: Option<i64> = parts.get(5).and_then(|s| s.parse::<i64>().ok());

    let name = unescape_pd_token(name_raw);
    let is_template = name_is_template(&name);
    Some(Row {
        file: file.to_string(),
        depth: internal_depth.saturating_sub(1),
        index: None,
        kind: RowKind::Classic,
        name,
        size,
        is_template,
        define: None,
        classic: Some(ClassicPayload { save_flag }),
    })
}

/// Try to parse an `#X obj ... array (define|d) ...` entry.  Returns `None` if
/// this entry is not an `array define`/`array d` declaration.
fn parse_define(
    raw: &str,
    file: &str,
    internal_depth: usize,
    object_index: Option<usize>,
    verbose: bool,
) -> Option<DefineParse> {
    // Strip trailing `;` and split.
    let body = raw.trim().trim_end_matches(';').trim_end();
    let toks: Vec<&str> = body.split_whitespace().collect();
    // #X obj X Y array (define|d) [flags...] <name> <size>
    if toks.len() < 4 || toks[0] != "#X" || toks[1] != "obj" {
        return None;
    }
    if toks.len() < 6 {
        return None;
    }
    if toks[4] != "array" {
        return None;
    }
    if toks[5] != "define" && toks[5] != "d" {
        return None;
    }
    // After the `#X obj X Y array (define|d)` prefix we have the args.
    let args = &toks[6..];
    if args.len() < 2 {
        return Some(DefineParse::Malformed(format!(
            "malformed `array {}` (need at least <name> <size>): {}",
            toks[5],
            raw.trim()
        )));
    }
    let size_tok = args[args.len() - 1];
    let name_tok = args[args.len() - 2];
    let flag_toks = &args[..args.len() - 2];

    // Right-anchor: size must parse as integer.
    let size: usize = match size_tok.parse::<usize>() {
        Ok(n) => n,
        Err(_) => {
            return Some(DefineParse::Malformed(format!(
                "malformed `array {}` (size `{}` is not an integer): {}",
                toks[5],
                size_tok,
                raw.trim()
            )));
        }
    };
    let name = unescape_pd_token(name_tok);
    let is_template = name_is_template(&name);

    let mut payload = DefinePayload {
        k: false,
        yrange: None,
        pix: None,
        discarded: Vec::new(),
    };
    let mut seen_yrange: Option<[Number; 2]> = None;
    let mut seen_pix: Option<[Number; 2]> = None;

    let mut i = 0;
    while i < flag_toks.len() {
        let tok = flag_toks[i];
        match tok {
            "-k" => {
                if payload.k {
                    // Repeated 0-arg flag: idempotent, NOT in discarded_tokens.
                }
                payload.k = true;
                i += 1;
            }
            "-yrange" | "-pix" => {
                let arity = 2;
                if i + arity >= flag_toks.len() {
                    // Not enough arguments left before name/size.  Treat the
                    // remainder as an unknown-flag stop (cannot guess arity).
                    payload.discarded.push(Discarded {
                        reason: "unknown_flag".to_string(),
                        tokens: flag_toks[i..].iter().map(ToString::to_string).collect(),
                    });
                    break;
                }
                let a = flag_toks[i + 1];
                let b = flag_toks[i + 2];
                let parsed = (parse_number(a), parse_number(b));
                let kind_name = if tok == "-yrange" { "yrange" } else { "pix" };
                match parsed {
                    (Some(na), Some(nb)) => {
                        let pair = [na, nb];
                        match tok {
                            "-yrange" => {
                                if let Some(prev) = seen_yrange.take() {
                                    payload.discarded.push(Discarded {
                                        reason: "superseded_yrange".to_string(),
                                        tokens: vec![
                                            "-yrange".to_string(),
                                            prev[0].to_string(),
                                            prev[1].to_string(),
                                        ],
                                    });
                                    if verbose {
                                        eprintln!(
                                            "warning: {}: superseded -yrange in `{}`",
                                            file,
                                            raw.trim()
                                        );
                                    }
                                }
                                seen_yrange = Some(pair.clone());
                                payload.yrange = Some(pair);
                                if verbose
                                    && parse_number(a).map(|n| n.to_string())
                                        == parse_number(b).map(|n| n.to_string())
                                {
                                    eprintln!(
                                        "note: {}: -yrange ylo == yhi in `{}` (PD will treat as (-1, 1) at runtime)",
                                        file,
                                        raw.trim()
                                    );
                                }
                            }
                            "-pix" => {
                                if let Some(prev) = seen_pix.take() {
                                    payload.discarded.push(Discarded {
                                        reason: "superseded_pix".to_string(),
                                        tokens: vec![
                                            "-pix".to_string(),
                                            prev[0].to_string(),
                                            prev[1].to_string(),
                                        ],
                                    });
                                    if verbose {
                                        eprintln!(
                                            "warning: {}: superseded -pix in `{}`",
                                            file,
                                            raw.trim()
                                        );
                                    }
                                }
                                seen_pix = Some(pair.clone());
                                payload.pix = Some(pair);
                            }
                            _ => unreachable!(),
                        }
                    }
                    _ => {
                        payload.discarded.push(Discarded {
                            reason: format!("malformed_{kind_name}"),
                            tokens: vec![tok.to_string(), a.to_string(), b.to_string()],
                        });
                        if verbose {
                            eprintln!(
                                "warning: {}: malformed {} args in `{}`",
                                file,
                                tok,
                                raw.trim()
                            );
                        }
                    }
                }
                i += 1 + arity;
            }
            _ => {
                // Unknown flag — cannot guess arity.  Record from here to end
                // and stop parsing flags.
                payload.discarded.push(Discarded {
                    reason: "unknown_flag".to_string(),
                    tokens: flag_toks[i..].iter().map(ToString::to_string).collect(),
                });
                if verbose {
                    eprintln!(
                        "warning: {}: unknown flag `{}` in `{}`",
                        file,
                        tok,
                        raw.trim()
                    );
                }
                break;
            }
        }
    }

    Some(DefineParse::Row(Row {
        file: file.to_string(),
        depth: internal_depth.saturating_sub(1),
        index: object_index,
        kind: RowKind::Define,
        name,
        size,
        is_template,
        define: Some(payload),
        classic: None,
    }))
}

/// Parse a numeric token for `-yrange`/`-pix`.  Emits an integer-typed
/// `Number` when both criteria hold: parses as `i64` AND has no decimal point.
fn parse_number(tok: &str) -> Option<Number> {
    if !tok.contains('.')
        && !tok.contains('e')
        && !tok.contains('E')
        && let Ok(n) = tok.parse::<i64>()
    {
        return Some(Number::from(n));
    }
    let f = tok.parse::<f64>().ok()?;
    if !f.is_finite() {
        return None;
    }
    Number::from_f64(f)
}

fn name_is_template(name: &str) -> bool {
    // Strict: contains `$N` for N in 1..=9 (substring match), preceded by an
    // even number of backslashes (i.e. unescaped) — but unescape_pd_token
    // has already collapsed `\$` to `$`, so a plain substring scan is correct.
    let bytes = name.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != b'$' {
            continue;
        }
        if i + 1 < bytes.len() {
            let d = bytes[i + 1];
            if (b'1'..=b'9').contains(&d) {
                return true;
            }
        }
    }
    false
}

fn render(rows: &[Row], cfg: &ArraysConfig, kind_filter: KindFilter) -> String {
    if cfg.json {
        return render_json(rows, cfg);
    }
    render_text(rows, cfg, kind_filter)
}

fn render_json(rows: &[Row], cfg: &ArraysConfig) -> String {
    match cfg.schema {
        Schema::V1 => render_json_v1(rows),
        Schema::V2 => render_json_v2(rows),
    }
}

fn render_json_v1(rows: &[Row]) -> String {
    // v1 envelope — exactly the historical shape.
    let arrays: Vec<Value> = rows
        .iter()
        .filter(|r| r.kind == RowKind::Classic)
        .map(|r| {
            json!({
                "file": r.file,
                "depth": r.depth,
                "name": r.name,
                "size": r.size,
            })
        })
        .collect();

    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.kind == RowKind::Classic) {
        by_name
            .entry(r.name.clone())
            .or_default()
            .push(r.file.clone());
    }
    let dups: BTreeMap<String, Vec<String>> =
        by_name.into_iter().filter(|(_, fs)| fs.len() > 1).collect();

    serde_json::to_string_pretty(&json!({
        "arrays": arrays,
        "duplicate_names": dups,
    }))
    .unwrap()
}

fn render_json_v2(rows: &[Row]) -> String {
    let arrays: Vec<Value> = rows.iter().map(row_to_json_v2).collect();

    // duplicate_names: name -> [{file, kind, depth, index}]
    let mut by_name: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for r in rows {
        by_name.entry(r.name.clone()).or_default().push(json!({
            "file": r.file,
            "kind": r.kind.as_str(),
            "depth": r.depth,
            "index": r.index,
        }));
    }
    let dups: BTreeMap<String, Vec<Value>> =
        by_name.into_iter().filter(|(_, v)| v.len() > 1).collect();

    serde_json::to_string_pretty(&json!({
        "schema_version": 2,
        "arrays": arrays,
        "duplicate_names": dups,
    }))
    .unwrap()
}

fn row_to_json_v2(r: &Row) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("file".into(), json!(r.file));
    obj.insert("depth".into(), json!(r.depth));
    obj.insert("index".into(), json!(r.index));
    obj.insert("kind".into(), json!(r.kind.as_str()));
    obj.insert("name".into(), json!(r.name));
    obj.insert("size".into(), json!(r.size));
    obj.insert("is_template".into(), json!(r.is_template));

    if let Some(d) = &r.define {
        let discarded: Vec<Value> = d
            .discarded
            .iter()
            .map(|x| json!({"reason": x.reason, "tokens": x.tokens}))
            .collect();
        let parse_status = if d.discarded.is_empty() {
            "clean"
        } else {
            "partial"
        };
        let yrange = match &d.yrange {
            Some(p) => json!([p[0], p[1]]),
            None => Value::Null,
        };
        let pix = match &d.pix {
            Some(p) => json!([p[0], p[1]]),
            None => Value::Null,
        };
        obj.insert(
            "define".into(),
            json!({
                "k": d.k,
                "yrange": yrange,
                "pix": pix,
                "discarded_tokens": discarded,
                "parse_status": parse_status,
            }),
        );
    }
    if let Some(c) = &r.classic {
        let save_flag = match c.save_flag {
            Some(k) => json!(k),
            None => Value::Null,
        };
        let saveit = match c.saveit() {
            Some(b) => json!(b),
            None => Value::Null,
        };
        let filestyle = match c.filestyle() {
            Some(s) => json!(s),
            None => Value::Null,
        };
        let hidename = match c.hidename() {
            Some(b) => json!(b),
            None => Value::Null,
        };
        obj.insert(
            "classic".into(),
            json!({
                "save_flag": save_flag,
                "saveit": saveit,
                "filestyle": filestyle,
                "hidename": hidename,
            }),
        );
    }
    Value::Object(obj)
}

fn render_text(rows: &[Row], cfg: &ArraysConfig, kind_filter: KindFilter) -> String {
    if rows.is_empty() {
        return match kind_filter {
            KindFilter::Classic => "No arrays found".to_string(),
            KindFilter::Define => "No `array define` declarations found".to_string(),
            KindFilter::All => "No arrays found".to_string(),
        };
    }

    let mut out = String::new();
    match cfg.schema {
        Schema::V1 => {
            // Historic format: `<file> [depth:N] array <name> size <size>`
            for r in rows.iter().filter(|r| r.kind == RowKind::Classic) {
                let _ = writeln!(
                    out,
                    "{} [depth:{}] array {} size {}",
                    r.file, r.depth, r.name, r.size
                );
            }
            let dups = collect_duplicates_v1(rows);
            if !dups.is_empty() {
                out.push_str("Duplicate array names:\n");
                for (name, files) in dups {
                    let _ = writeln!(out, "- {}: {}", name, files.join(", "));
                }
            }
        }
        Schema::V2 => {
            for r in rows {
                let kind = r.kind.as_str();
                let mut line = format!(
                    "{} [depth:{}] {} {} size {}",
                    r.file, r.depth, kind, r.name, r.size
                );
                if let Some(c) = &r.classic {
                    let mut tail = Vec::new();
                    if c.saveit() == Some(true) {
                        tail.push("saveit".to_string());
                    }
                    if let Some(fs) = c.filestyle() {
                        tail.push(fs.to_string());
                    }
                    if c.hidename() == Some(true) {
                        tail.push("hidename".to_string());
                    }
                    if !tail.is_empty() {
                        let _ = write!(line, " ({})", tail.join(", "));
                    }
                }
                if let Some(d) = &r.define {
                    if d.k {
                        line.push_str(" -k");
                    }
                    if let Some(p) = &d.yrange {
                        let _ = write!(line, " -yrange {} {}", p[0], p[1]);
                    }
                    if let Some(p) = &d.pix {
                        let _ = write!(line, " -pix {} {}", p[0], p[1]);
                    }
                }
                out.push_str(&line);
                out.push('\n');
            }
            let dups = collect_duplicates_v2(rows);
            if !dups.is_empty() {
                out.push_str("Duplicate array names:\n");
                for (name, entries) in dups {
                    let parts: Vec<String> = entries
                        .iter()
                        .map(|(file, kind)| format!("{file} [{kind}]"))
                        .collect();
                    let _ = writeln!(out, "- {}: {}", name, parts.join(", "));
                }
            }
        }
    }
    out.trim_end().to_string()
}

fn collect_duplicates_v1(rows: &[Row]) -> BTreeMap<String, Vec<String>> {
    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.kind == RowKind::Classic) {
        by_name
            .entry(r.name.clone())
            .or_default()
            .push(r.file.clone());
    }
    by_name.into_iter().filter(|(_, v)| v.len() > 1).collect()
}

fn collect_duplicates_v2(rows: &[Row]) -> BTreeMap<String, Vec<(String, &'static str)>> {
    let mut by_name: BTreeMap<String, Vec<(String, &'static str)>> = BTreeMap::new();
    for r in rows {
        by_name
            .entry(r.name.clone())
            .or_default()
            .push((r.file.clone(), r.kind.as_str()));
    }
    by_name.into_iter().filter(|(_, v)| v.len() > 1).collect()
}
