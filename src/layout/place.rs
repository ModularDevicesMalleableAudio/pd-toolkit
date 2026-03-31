/// Width estimation and coordinate placement.
///
/// Width estimation: derive the pixel width of an object box from its content.
/// Placement: assign final X/Y pixel coordinates using layer + barycenter ordering.
use crate::model::Entry;

// Width estimation

const CHAR_WIDTH: i32 = 7;   // approximate pixels per character
const PADDING: i32 = 4;      // horizontal padding each side
const MIN_WIDTH: i32 = 25;   // minimum box width

/// Estimate the display width (pixels) of an object box.
pub fn estimate_width(entry: &Entry) -> i32 {
    use crate::model::EntryKind;

    match entry.kind {
        // Floatatom: width is encoded as the third token (#X floatatom X Y W ...)
        EntryKind::FloatAtom | EntryKind::SymbolAtom => {
            let parts: Vec<&str> = entry.raw.split_whitespace().collect();
            if let Some(w) = parts.get(4)
                && let Ok(n) = w.parse::<i32>()
                && n > 0
            {
                return n * CHAR_WIDTH + PADDING * 2;
            }
            5 * CHAR_WIDTH + PADDING * 2
        }

        EntryKind::Obj => {
            // Check for inline ", f N" width hint first
            let raw = entry.raw.trim().trim_end_matches(';');
            if let Some(comma_pos) = raw.rfind(", f ") {
                let rest = raw[comma_pos + 4..].trim();
                if let Ok(w) = rest.parse::<i32>() {
                    return w * CHAR_WIDTH + PADDING * 2;
                }
            }

            // GUI objects with a size parameter
            let class = entry.class();
            let args = entry.args();
            let gui_size = gui_size_param(class, &args);
            if let Some(sz) = gui_size {
                return sz.max(MIN_WIDTH);
            }

            // Default: estimate from class + args text length
            let text_len: usize = class.len() + args.iter().map(|a| a.len() + 1).sum::<usize>();
            (text_len as i32 * CHAR_WIDTH + PADDING * 2).max(MIN_WIDTH)
        }

        EntryKind::Msg | EntryKind::Text => {
            let parts: Vec<&str> = entry.raw.split_whitespace().collect();
            let content: Vec<&str> = parts.get(4..).unwrap_or(&[]).to_vec();
            let len: usize = content.iter().map(|t| t.len() + 1).sum::<usize>();
            (len as i32 * CHAR_WIDTH + PADDING * 2).max(MIN_WIDTH)
        }

        EntryKind::Restore => MIN_WIDTH * 3,

        _ => MIN_WIDTH,
    }
}

/// For known GUI objects, return the pixel size encoded in args.
fn gui_size_param(class: &str, args: &[String]) -> Option<i32> {
    match class {
        // tgl/bng: arg[0] is the size in pixels
        "tgl" | "bng" => args.first().and_then(|a| a.parse::<i32>().ok()),
        // nbx: arg[0]=width(chars), arg[1]=height — use width * CHAR_WIDTH
        "nbx" => args.first().and_then(|a| a.parse::<i32>().ok()).map(|w| w * CHAR_WIDTH),
        // vsl/hsl: arg[0]=width, arg[1]=height
        "vsl" => args.get(1).and_then(|a| a.parse::<i32>().ok()),
        "hsl" => args.first().and_then(|a| a.parse::<i32>().ok()),
        // vradio/hradio: arg[0]=size, arg[2]=count — total = size*count
        "vradio" => {
            let sz = args.first().and_then(|a| a.parse::<i32>().ok()).unwrap_or(15);
            Some(sz)
        }
        "hradio" => {
            let sz = args.first().and_then(|a| a.parse::<i32>().ok()).unwrap_or(15);
            let count = args.get(2).and_then(|a| a.parse::<i32>().ok()).unwrap_or(8);
            Some(sz * count)
        }
        // vu: arg[0]=width
        "vu" => args.first().and_then(|a| a.parse::<i32>().ok()),
        // cnv: arg[0]=size, arg[1]=width, arg[2]=height
        "cnv" => args.get(1).and_then(|a| a.parse::<i32>().ok()),
        _ => None,
    }
}

// Coordinate placement

/// Options controlling layout geometry.
#[derive(Debug, Clone)]
pub struct LayoutOptions {
    pub grid: i32,
    pub hpad: i32,   // extra horizontal gap between boxes in the same layer
    pub vpad: i32,   // vertical gap between layers (pixels)
    pub margin: i32, // left/top margin
}

impl Default for LayoutOptions {
    fn default() -> Self {
        LayoutOptions { grid: 30, hpad: 10, vpad: 40, margin: 20 }
    }
}

/// Snap a value to the nearest grid multiple.
fn snap(v: i32, grid: i32) -> i32 {
    if grid <= 1 {
        return v;
    }
    ((v + grid / 2) / grid) * grid
}

/// Compute (x, y) pixel positions for every node, given:
/// - the ordered-within-layer groups from `crossing::reorder`
/// - the width of each node
/// - layout options
///
/// Returns a `Vec<(i32, i32)>` indexed by node id.
pub fn place_nodes(
    groups: &[Vec<usize>],
    widths: &[i32],
    opts: &LayoutOptions,
) -> Vec<(i32, i32)> {
    let n = widths.len();
    let mut coords = vec![(opts.margin, opts.margin); n];

    let mut y = opts.margin;

    for layer_nodes in groups {
        if layer_nodes.is_empty() {
            y += opts.vpad;
            continue;
        }

        let layer_height = 20; // assume 20px object box height
        let mut x = opts.margin;

        for &node in layer_nodes {
            if node >= n {
                continue;
            }
            let sx = snap(x, opts.grid);
            let sy = snap(y, opts.grid);
            coords[node] = (sx, sy);
            x = sx + widths[node] + opts.hpad;
        }

        y += layer_height + opts.vpad;
    }

    coords
}

// Overlap / bounding-box check

/// Returns `true` if any two bounding boxes at the same layer overlap
/// horizontally (they can't overlap vertically because layers are stacked).
pub fn has_overlaps(
    groups: &[Vec<usize>],
    coords: &[(i32, i32)],
    widths: &[i32],
) -> bool {
    for layer in groups {
        let mut boxes: Vec<(i32, i32)> = layer
            .iter()
            .filter(|&&n| n < coords.len() && n < widths.len())
            .map(|&n| (coords[n].0, coords[n].0 + widths[n]))
            .collect();
        boxes.sort_by_key(|b| b.0);
        for pair in boxes.windows(2) {
            if pair[0].1 > pair[1].0 {
                return true;
            }
        }
    }
    false
}

// Unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntryKind;

    fn make_obj(class_args: &str) -> Entry {
        Entry {
            raw: format!("#X obj 50 50 {};", class_args),
            kind: EntryKind::Obj,
            depth: 1,
            object_index: Some(0),
        }
    }

    fn make_floatatom(width_chars: i32) -> Entry {
        Entry {
            raw: format!("#X floatatom 50 50 {} 0 0 0 - - -;", width_chars),
            kind: EntryKind::FloatAtom,
            depth: 1,
            object_index: Some(0),
        }
    }

    #[test]
    fn width_plain_object_text_length_plus_padding() {
        // "loadbang" = 8 chars → 8*7 + 8 = 64
        let e = make_obj("loadbang");
        let w = estimate_width(&e);
        assert!(w >= MIN_WIDTH);
        assert_eq!(w, 8 * CHAR_WIDTH + PADDING * 2);
    }

    #[test]
    fn width_inline_f_hint_uses_hint_value() {
        // ", f 12" at end → width = 12 * CHAR_WIDTH + 8
        let e = Entry {
            raw: "#X obj 50 50 t f f, f 12;".to_string(),
            kind: EntryKind::Obj,
            depth: 1,
            object_index: Some(0),
        };
        let w = estimate_width(&e);
        assert_eq!(w, 12 * CHAR_WIDTH + PADDING * 2);
    }

    #[test]
    fn width_gui_objects_use_size_param() {
        // tgl with size 25 → width = 25
        let e = make_obj("tgl 25 0 s r empty 17 7 0 10 -262144 -1 -1 0 1");
        let w = estimate_width(&e);
        assert_eq!(w, 25);
    }

    #[test]
    fn width_floatatom_uses_width_field() {
        // floatatom with 5-char width → 5*7 + 8 = 43
        let e = make_floatatom(5);
        let w = estimate_width(&e);
        assert_eq!(w, 5 * CHAR_WIDTH + PADDING * 2);
    }
}
