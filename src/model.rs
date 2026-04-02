use crate::parser::tokenizer::TokenizeWarning;
use thiserror::Error;

// Errors and warnings

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("empty input: file contains no entries")]
    EmptyInput,
    #[error("missing canvas header: first entry must be #N canvas")]
    MissingCanvasHeader,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseWarning {
    UnterminatedEntry,
}

impl From<TokenizeWarning> for ParseWarning {
    fn from(w: TokenizeWarning) -> Self {
        match w {
            TokenizeWarning::UnterminatedEntry => ParseWarning::UnterminatedEntry,
        }
    }
}

// Entry kinds

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    CanvasOpen, // #N canvas
    Obj,        // #X obj
    Msg,        // #X msg
    Text,       // #X text
    FloatAtom,  // #X floatatom
    SymbolAtom, // #X symbolatom
    Restore,    // #X restore
    Connect,    // #X connect
    Coords,     // #X coords
    Array,      // #X array
    ArrayData,  // #A
    Declare,    // #X declare (standalone, NOT #X obj ... declare)
    WidthHint,  // #X f <number>
    Unknown,    // #C or anything unrecognised
}

// Entry

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub raw: String,
    pub kind: EntryKind,
    /// Internal depth.  Top-level objects are at depth 1 (the root #N canvas
    /// header lives at depth 0 and increments depth before any objects are
    /// encountered).  Use Patch methods which expose user-facing depth
    /// (user = internal − 1).
    pub depth: usize,
    pub object_index: Option<usize>,
}

// Helpers shared by Entry methods

/// Strip the trailing `;` and any trailing `, f <integer>` width hint from a
/// raw entry string.  Returns a slice of the original.
fn content_without_width_hint(raw: &str) -> &str {
    let without_semi = raw.trim().trim_end_matches(';').trim_end();
    if let Some(comma_pos) = without_semi.rfind(", f ") {
        let after = without_semi[comma_pos + 4..].trim();
        if after.parse::<i32>().is_ok() {
            return without_semi[..comma_pos].trim_end();
        }
    }
    without_semi
}

/// For GUI objects, returns the (send_arg_index, receive_arg_index) into the
/// args() slice (0-based, after class+coords have been removed).
/// Returns None for non-GUI objects, or for vu which only has a receive.
pub fn gui_send_receive_arg_indices(class: &str) -> Option<(usize, usize)> {
    match class {
        "tgl" => Some((2, 3)),
        "bng" => Some((4, 5)),
        "nbx" => Some((6, 7)),
        "vsl" | "hsl" => Some((6, 7)),
        "vradio" | "hradio" => Some((4, 5)),
        "cnv" => Some((3, 4)),
        _ => None,
    }
}

/// vu has only a receive field (no send).  Returns its arg index.
pub fn vu_receive_arg_index() -> usize {
    2
}

fn is_empty_name(s: &str) -> bool {
    s == "empty" || s == "-"
}

// Entry methods

impl Entry {
    /// The object class name.
    /// - For `#X obj`: the word after the X/Y coordinates, with any trailing
    ///   `, f N` width hint stripped first.
    /// - For other object-like kinds: the kind name itself ("msg", "text",
    ///   "floatatom", "symbolatom", "restore").
    pub fn class(&self) -> &str {
        match self.kind {
            EntryKind::Obj => content_without_width_hint(&self.raw)
                .split_whitespace()
                .nth(4)
                .unwrap_or(""),
            EntryKind::Msg => "msg",
            EntryKind::Text => "text",
            EntryKind::FloatAtom => "floatatom",
            EntryKind::SymbolAtom => "symbolatom",
            EntryKind::Restore => "restore",
            EntryKind::Connect => "connect",
            EntryKind::CanvasOpen => "canvas",
            EntryKind::Coords => "coords",
            EntryKind::Array => "array",
            EntryKind::ArrayData => "data",
            EntryKind::Declare => "declare",
            EntryKind::WidthHint => "width_hint",
            EntryKind::Unknown => "unknown",
        }
    }

    /// Arguments after the class name for `#X obj` entries, with any trailing
    /// `, f N` width hint stripped.  Empty vec for all other kinds.
    pub fn args(&self) -> Vec<String> {
        if self.kind != EntryKind::Obj {
            return Vec::new();
        }
        content_without_width_hint(&self.raw)
            .split_whitespace()
            .skip(5) // #X obj X Y class → skip 5 tokens
            .map(str::to_owned)
            .collect()
    }

    /// X canvas coordinate (pixel position).  Available for all object-like
    /// entry types.
    pub fn x(&self) -> Option<i32> {
        match self.kind {
            EntryKind::Obj
            | EntryKind::Msg
            | EntryKind::Text
            | EntryKind::FloatAtom
            | EntryKind::SymbolAtom
            | EntryKind::Restore => self.raw.split_whitespace().nth(2)?.parse().ok(),
            _ => None,
        }
    }

    /// Y canvas coordinate (pixel position).
    pub fn y(&self) -> Option<i32> {
        match self.kind {
            EntryKind::Obj
            | EntryKind::Msg
            | EntryKind::Text
            | EntryKind::FloatAtom
            | EntryKind::SymbolAtom
            | EntryKind::Restore => self.raw.split_whitespace().nth(3)?.parse().ok(),
            _ => None,
        }
    }

    /// The embedded send name for GUI objects, or None if absent / set to the
    /// sentinel value "empty" or "-".
    pub fn gui_send(&self) -> Option<String> {
        match self.kind {
            EntryKind::Obj => {
                let class = self.class();
                // vu has no send field
                let (send_idx, _) = gui_send_receive_arg_indices(class)?;
                let args = self.args();
                let val = args.get(send_idx)?;
                if is_empty_name(val) {
                    None
                } else {
                    Some(val.clone())
                }
            }
            EntryKind::FloatAtom | EntryKind::SymbolAtom => {
                // #X floatatom/symbolatom X Y width min max flag send receive label
                // send is word[8]
                let words: Vec<&str> = self.raw.split_whitespace().collect();
                let val = words.get(8)?;
                if is_empty_name(val) {
                    None
                } else {
                    Some((*val).to_owned())
                }
            }
            _ => None,
        }
    }

    /// The embedded receive name for GUI objects.
    pub fn gui_receive(&self) -> Option<String> {
        match self.kind {
            EntryKind::Obj => {
                let class = self.class();
                if class == "vu" {
                    let args = self.args();
                    let val = args.get(vu_receive_arg_index())?;
                    return if is_empty_name(val) {
                        None
                    } else {
                        Some(val.clone())
                    };
                }
                let (_, recv_idx) = gui_send_receive_arg_indices(class)?;
                let args = self.args();
                let val = args.get(recv_idx)?;
                if is_empty_name(val) {
                    None
                } else {
                    Some(val.clone())
                }
            }
            EntryKind::FloatAtom | EntryKind::SymbolAtom => {
                // receive is word[9]
                let words: Vec<&str> = self.raw.split_whitespace().collect();
                let val = words.get(9)?;
                if is_empty_name(val) {
                    None
                } else {
                    Some((*val).to_owned())
                }
            }
            _ => None,
        }
    }
}

// Connection

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub src: usize,
    pub src_outlet: usize,
    pub dst: usize,
    pub dst_inlet: usize,
}

impl Connection {
    /// Parse a raw `#X connect src outlet dst inlet;` entry.
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim().trim_end_matches(';');
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() != 6 || parts[0] != "#X" || parts[1] != "connect" {
            return None;
        }
        Some(Connection {
            src: parts[2].parse().ok()?,
            src_outlet: parts[3].parse().ok()?,
            dst: parts[4].parse().ok()?,
            dst_inlet: parts[5].parse().ok()?,
        })
    }
}

// Patch

#[derive(Debug, Clone)]
pub struct Patch {
    pub entries: Vec<Entry>,
    pub warnings: Vec<ParseWarning>,
}

impl Patch {
    /// Number of object-indexed entries at user-facing depth `d`.
    /// User depth 0 = top-level canvas, 1 = first level of subpatch, etc.
    pub fn object_count_at_depth(&self, user_depth: usize) -> usize {
        let internal = user_depth + 1;
        self.entries
            .iter()
            .filter(|e| e.depth == internal && e.object_index.is_some())
            .count()
    }

    /// The entry at user-facing depth `d` with object index `idx`.
    pub fn object_at(&self, user_depth: usize, idx: usize) -> Option<&Entry> {
        let internal = user_depth + 1;
        self.entries
            .iter()
            .find(|e| e.depth == internal && e.object_index == Some(idx))
    }

    /// All connections at user-facing depth `d`.
    pub fn connections_at_depth(&self, user_depth: usize) -> Vec<Connection> {
        let internal = user_depth + 1;
        self.entries
            .iter()
            .filter(|e| e.kind == EntryKind::Connect && e.depth == internal)
            .filter_map(|e| Connection::parse(&e.raw))
            .collect()
    }

    /// Maximum user-facing depth that contains at least one object.
    pub fn max_depth(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.object_index.is_some())
            .map(|e| e.depth.saturating_sub(1))
            .max()
            .unwrap_or(0)
    }

    /// Total number of `#N canvas` entries (root + subpatches).
    pub fn canvas_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.kind == EntryKind::CanvasOpen)
            .count()
    }
}
