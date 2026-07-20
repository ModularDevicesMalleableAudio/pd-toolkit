use crate::errors::PdtkError;
use pdtk::parser::parse;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ListObject {
    depth: usize,
    /// `--canvas N` ordinal among sibling subpatches at this depth.
    canvas: Option<usize>,
    index: usize,
    class: String,
    args: Vec<String>,
    x: Option<i32>,
    y: Option<i32>,
    raw: String,
}

pub fn run(
    file: &str,
    depth: Option<usize>,
    json: bool,
    output: Option<&str>,
) -> Result<String, PdtkError> {
    let input = crate::io::read_patch_lenient(file)?;
    let patch = parse(&input)?;

    let mut rows: Vec<ListObject> = patch
        .entries
        .iter()
        .filter_map(|e| {
            let idx = e.object_index?;
            let user_depth = e.depth.saturating_sub(1);
            if let Some(wanted) = depth
                && user_depth != wanted
            {
                return None;
            }

            let canvas = e
                .canvas_id
                .and_then(|cid| patch.canvas_ordinal(user_depth, cid));
            Some(ListObject {
                depth: user_depth,
                canvas,
                index: idx,
                class: e.class().to_owned(),
                args: e.args(),
                x: e.x(),
                y: e.y(),
                raw: e.raw.clone(),
            })
        })
        .collect();

    rows.sort_by_key(|r| (r.depth, r.canvas, r.index));

    let text = if json {
        serde_json::to_string_pretty(&rows)?
    } else {
        let lines = rows
            .into_iter()
            .map(|r| {
                if r.args.is_empty() {
                    format!("[{}:{}] {}", r.depth, r.index, r.class)
                } else {
                    format!("[{}:{}] {} {}", r.depth, r.index, r.class, r.args.join(" "))
                }
            })
            .collect::<Vec<_>>();
        lines.join("\n")
    };

    if let Some(out_path) = output {
        std::fs::write(out_path, &text)?;
        return Ok(String::new());
    }

    Ok(text)
}
