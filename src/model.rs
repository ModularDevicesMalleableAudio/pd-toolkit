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
    Struct,     // #N struct (data-structure template definition)
    Obj,        // #X obj
    Msg,        // #X msg
    Text,       // #X text
    FloatAtom,  // #X floatatom
    SymbolAtom, // #X symbolatom
    ListAtom,   // #X listbox
    Restore,    // #X restore
    Connect,    // #X connect
    Coords,     // #X coords
    Array,      // #X array
    Scalar,     // #X scalar
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
    /// Identifies the specific canvas instance this entry belongs to.
    /// Two sibling subpatches both at the same depth have different
    /// `canvas_id`s. The root canvas is 0. `None` for entries that are
    /// not inside a canvas (shouldn't normally happen for well-formed
    /// patches).
    pub canvas_id: Option<usize>,
}

// Helpers shared by Entry methods

/// Strip the trailing `;` and any trailing `, f <integer>` width hint from a
/// raw entry string (or bare object text).  Returns a slice of the original.
#[must_use]
pub fn content_without_width_hint(raw: &str) -> &str {
    let without_semi = raw.trim().trim_end_matches(';').trim_end();
    if let Some(comma_pos) = without_semi.rfind(", f ") {
        let after = without_semi[comma_pos + 4..].trim();
        if after.parse::<i32>().is_ok() {
            return without_semi[..comma_pos].trim_end();
        }
    }
    without_semi
}

/// Parse the value of a trailing `, f <integer>` width hint from a raw entry
/// or bare object text (with or without a trailing `;`).  Returns `None` when
/// no width hint is present.
#[must_use]
pub fn trailing_width_hint(content: &str) -> Option<i32> {
    let without_semi = content.trim().trim_end_matches(';').trim_end();
    let comma_pos = without_semi.rfind(", f ")?;
    without_semi[comma_pos + 4..].trim().parse::<i32>().ok()
}

/// For GUI objects, returns the (send_arg_index, receive_arg_index) into the
/// args() slice (0-based, after class+coords have been removed).
/// Returns None for non-GUI objects, or for vu which only has a receive.
#[must_use]
pub fn gui_send_receive_arg_indices(class: &str) -> Option<(usize, usize)> {
    match class {
        "tgl" => Some((2, 3)),
        "bng" => Some((4, 5)),
        "nbx" => Some((6, 7)),
        "vsl" | "hsl" => Some((6, 7)),
        "vradio" | "hradio" | "hdl" | "vdl" => Some((4, 5)),
        "cnv" => Some((3, 4)),
        _ => None,
    }
}

/// vu has only a receive field (no send).  Returns its arg index.
#[must_use]
pub fn vu_receive_arg_index() -> usize {
    2
}

/// Extract the named send targets embedded in a `#X msg` entry.
///
/// A PD message box may contain sub-messages separated by an escaped `\;`.
/// Each `\;`-introduced sub-message is delivered to the named receiver given
/// by its first token (e.g. `\; pitch 60` sends `60` to receiver `pitch`).
/// This is PD's standard state-broadcast idiom and is otherwise invisible to
/// send/receive analysis and `rename-send`.
///
/// Returns `(token_position, name)` pairs, where `token_position` is the index
/// into `raw.split_whitespace()` of the target token (so callers can rewrite
/// it in place). The returned `name` has any trailing `;`/`,` stripped. The
/// leading sub-message (before the first `\;`) is emitted from the box's
/// outlet, not to a named receiver, so it is never a target.
///
/// Returns an empty vec for non-message entries.
#[must_use]
pub fn message_send_targets(raw: &str) -> Vec<(usize, String)> {
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    if tokens.len() < 5 || tokens[0] != "#X" || tokens[1] != "msg" {
        return Vec::new();
    }
    let mut out = Vec::new();
    // Content tokens start after `#X msg X Y`.
    let mut i = 4;
    while i < tokens.len() {
        if tokens[i] == r"\;" {
            if let Some(&next) = tokens.get(i + 1) {
                let name = next.trim_end_matches([';', ',']);
                // Skip empties and further separators.
                if !name.is_empty() && name != r"\;" && name != r"\," {
                    out.push((i + 1, name.to_string()));
                }
            }
        }
        i += 1;
    }
    out
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
    #[must_use]
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
            EntryKind::ListAtom => "listbox",
            EntryKind::Restore => "restore",
            EntryKind::Connect => "connect",
            EntryKind::CanvasOpen => "canvas",
            EntryKind::Struct => "struct",
            EntryKind::Coords => "coords",
            EntryKind::Array => "array",
            EntryKind::Scalar => "scalar",
            EntryKind::ArrayData => "data",
            EntryKind::Declare => "declare",
            EntryKind::WidthHint => "width_hint",
            EntryKind::Unknown => "unknown",
        }
    }

    /// Arguments after the class name for `#X obj` entries, with any trailing
    /// `, f N` width hint stripped.  Empty vec for all other kinds.
    #[must_use]
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

    /// The inline `, f N` width-hint value, if this entry carries one.
    #[must_use]
    pub fn width_hint(&self) -> Option<i32> {
        trailing_width_hint(&self.raw)
    }

    /// X canvas coordinate (pixel position).  Available for all object-like
    /// entry types.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
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
            EntryKind::FloatAtom | EntryKind::SymbolAtom | EntryKind::ListAtom => {
                // #X floatatom/symbolatom/listbox X Y width min max flag send receive label
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
    #[must_use]
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
            EntryKind::FloatAtom | EntryKind::SymbolAtom | EntryKind::ListAtom => {
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

// Data-structure templates and scalars

/// The type of a `#N struct` template field (Pd `g_template.c`:
/// DT_FLOAT / DT_SYMBOL / DT_TEXT / DT_ARRAY; the legacy `list` type name maps
/// to `Text`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateFieldType {
    Float,
    Symbol,
    Text,
    Array,
}

/// A single field of a `#N struct` data-structure template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateField {
    pub field_type: TemplateFieldType,
    pub name: String,
    /// Element sub-template name for an `array` field; `None` otherwise.
    pub array_template: Option<String>,
}

/// A parsed `#N struct` template: its name and typed fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    pub name: String,
    pub fields: Vec<TemplateField>,
}

impl Template {
    /// Number of scalar (flat) fields — `float` and `symbol`. This is the count
    /// of values a `#X scalar` for this template carries before its first `\;`
    /// separator; `array`/`text` fields are stored separately and don't count.
    #[must_use]
    pub fn scalar_field_count(&self) -> usize {
        self.fields
            .iter()
            .filter(|f| matches!(f.field_type, TemplateFieldType::Float | TemplateFieldType::Symbol))
            .count()
    }
}

/// Strip exactly one trailing entry terminator `;` (preserving any escaped
/// `\;`), after trimming trailing whitespace/newlines.
fn strip_terminator(raw: &str) -> &str {
    let t = raw.trim_end();
    t.strip_suffix(';').unwrap_or(t)
}

/// Parse a `#N struct` template definition into its name and typed fields.
/// Returns `None` for any other entry.
#[must_use]
pub fn parse_struct(raw: &str) -> Option<Template> {
    let toks: Vec<&str> = strip_terminator(raw).split_whitespace().collect();
    if toks.len() < 3 || toks[0] != "#N" || toks[1] != "struct" {
        return None;
    }
    let name = toks[2].to_string();
    let mut fields = Vec::new();
    let mut i = 3;
    while i < toks.len() {
        match toks[i] {
            "float" | "symbol" | "text" | "list" => {
                let Some(fname) = toks.get(i + 1) else { break };
                let field_type = match toks[i] {
                    "float" => TemplateFieldType::Float,
                    "symbol" => TemplateFieldType::Symbol,
                    _ => TemplateFieldType::Text,
                };
                fields.push(TemplateField {
                    field_type,
                    name: (*fname).to_string(),
                    array_template: None,
                });
                i += 2;
            }
            // `array <name> <element-template>` (Pd g_template.c).
            "array" => {
                let Some(fname) = toks.get(i + 1) else { break };
                fields.push(TemplateField {
                    field_type: TemplateFieldType::Array,
                    name: (*fname).to_string(),
                    array_template: toks.get(i + 2).map(|s| (*s).to_string()),
                });
                i += 3;
            }
            // Unknown type token: skip defensively rather than misparse.
            _ => i += 1,
        }
    }
    Some(Template { name, fields })
}

/// Parse a `#X scalar` entry into `(template_name, flat_values)`.
///
/// The flat values are the atoms before the first `\;` separator — Pd writes a
/// scalar's `float`/`symbol` fields there, with `array`/`text` data following
/// after `\;`. Returns `None` for any other entry.
#[must_use]
pub fn parse_scalar(raw: &str) -> Option<(String, Vec<String>)> {
    let toks: Vec<&str> = strip_terminator(raw).split_whitespace().collect();
    if toks.len() < 3 || toks[0] != "#X" || toks[1] != "scalar" {
        return None;
    }
    let name = toks[2].to_string();
    let mut flat = Vec::new();
    for &t in &toks[3..] {
        if t == r"\;" {
            break;
        }
        flat.push(t.to_string());
    }
    Some((name, flat))
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
    #[must_use]
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

/// Free-function form of `Patch::canvas_ids_at_depth`, for callers working on
/// a raw entry slice (before wrapping in a `Patch`).  Returns the `canvas_id`s
/// of the canvases whose direct contents live at `user_depth`, in document
/// order.
#[must_use]
pub fn canvas_ids_at_depth(entries: &[Entry], user_depth: usize) -> Vec<usize> {
    let mut ids = Vec::new();
    let mut open_counter = 0usize;
    for e in entries {
        if e.kind == EntryKind::CanvasOpen {
            let own_id = open_counter;
            open_counter += 1;
            if e.depth == user_depth {
                ids.push(own_id);
            }
        }
    }
    ids
}

/// Free-function form of `Patch::resolve_canvas`, for callers working on a raw
/// entry slice.
#[must_use]
pub fn resolve_canvas_id(entries: &[Entry], user_depth: usize, nth: usize) -> Option<usize> {
    canvas_ids_at_depth(entries, user_depth).get(nth).copied()
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
    #[must_use]
    pub fn object_count_at_depth(&self, user_depth: usize) -> usize {
        let internal = user_depth + 1;
        self.entries
            .iter()
            .filter(|e| e.depth == internal && e.object_index.is_some())
            .count()
    }

    /// The entry at user-facing depth `d` with object index `idx`.
    #[must_use]
    pub fn object_at(&self, user_depth: usize, idx: usize) -> Option<&Entry> {
        let internal = user_depth + 1;
        self.entries
            .iter()
            .find(|e| e.depth == internal && e.object_index == Some(idx))
    }

    /// All connections at user-facing depth `d`.
    #[must_use]
    pub fn connections_at_depth(&self, user_depth: usize) -> Vec<Connection> {
        let internal = user_depth + 1;
        self.entries
            .iter()
            .filter(|e| e.kind == EntryKind::Connect && e.depth == internal)
            .filter_map(|e| Connection::parse(&e.raw))
            .collect()
    }

    /// The `canvas_id`s of the canvases whose direct contents live at
    /// `user_depth`, in document order.  The position in this vec is the
    /// user-facing `--canvas N` selector (0 = first sibling at that depth).
    ///
    /// A canvas's own id equals its 0-based position among all `#N canvas`
    /// entries in document order, and its `#N canvas` entry sits at internal
    /// depth == `user_depth` (its contents are one level deeper).
    #[must_use]
    pub fn canvas_ids_at_depth(&self, user_depth: usize) -> Vec<usize> {
        canvas_ids_at_depth(&self.entries, user_depth)
    }

    /// Resolve the `canvas_id` of the `nth` canvas at `user_depth` (document
    /// order).  Returns `None` if there is no such canvas.
    #[must_use]
    pub fn resolve_canvas(&self, user_depth: usize, nth: usize) -> Option<usize> {
        self.canvas_ids_at_depth(user_depth).get(nth).copied()
    }

    /// The `--canvas N` ordinal of `canvas_id` among its siblings at
    /// `user_depth`, or `None` if it is not found at that depth.
    #[must_use]
    pub fn canvas_ordinal(&self, user_depth: usize, canvas_id: usize) -> Option<usize> {
        self.canvas_ids_at_depth(user_depth)
            .iter()
            .position(|&id| id == canvas_id)
    }

    /// Number of indexed objects belonging to a specific canvas.
    #[must_use]
    pub fn object_count_in_canvas(&self, canvas_id: usize) -> usize {
        self.entries
            .iter()
            .filter(|e| e.canvas_id == Some(canvas_id) && e.object_index.is_some())
            .count()
    }

    /// The entry with object index `idx` belonging to a specific canvas.
    #[must_use]
    pub fn object_in_canvas(&self, canvas_id: usize, idx: usize) -> Option<&Entry> {
        self.entries
            .iter()
            .find(|e| e.canvas_id == Some(canvas_id) && e.object_index == Some(idx))
    }

    /// All connections belonging to a specific canvas.
    #[must_use]
    pub fn connections_in_canvas(&self, canvas_id: usize) -> Vec<Connection> {
        self.entries
            .iter()
            .filter(|e| e.kind == EntryKind::Connect && e.canvas_id == Some(canvas_id))
            .filter_map(|e| Connection::parse(&e.raw))
            .collect()
    }

    /// Maximum user-facing depth that contains at least one object.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.object_index.is_some())
            .map(|e| e.depth.saturating_sub(1))
            .max()
            .unwrap_or(0)
    }

    /// Total number of `#N canvas` entries (root + subpatches).
    #[must_use]
    pub fn canvas_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.kind == EntryKind::CanvasOpen)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_indices_hradio_vradio() {
        assert_eq!(gui_send_receive_arg_indices("hradio"), Some((4, 5)));
        assert_eq!(gui_send_receive_arg_indices("vradio"), Some((4, 5)));
    }

    #[test]
    fn gui_indices_hdl_vdl_compat() {
        assert_eq!(gui_send_receive_arg_indices("hdl"), Some((4, 5)));
        assert_eq!(gui_send_receive_arg_indices("vdl"), Some((4, 5)));
    }

    #[test]
    fn gui_indices_unknown_class() {
        assert_eq!(gui_send_receive_arg_indices("osc~"), None);
    }

    #[test]
    fn width_hint_parsed_from_raw_entry() {
        let e = Entry {
            raw: "#X obj 50 50 t b b b, f 154;".to_string(),
            kind: EntryKind::Obj,
            depth: 1,
            object_index: Some(0),
            canvas_id: Some(0),
        };
        assert_eq!(e.width_hint(), Some(154));
        assert_eq!(e.class(), "t");
        assert_eq!(e.args(), vec!["b", "b", "b"]);
    }

    #[test]
    fn width_hint_absent_returns_none() {
        let e = Entry {
            raw: "#X obj 50 50 print;".to_string(),
            kind: EntryKind::Obj,
            depth: 1,
            object_index: Some(0),
            canvas_id: Some(0),
        };
        assert_eq!(e.width_hint(), None);
    }

    #[test]
    fn trailing_width_hint_on_bare_text() {
        assert_eq!(trailing_width_hint("t b b b b, f 200"), Some(200));
        assert_eq!(trailing_width_hint("t b b b b"), None);
        assert_eq!(content_without_width_hint("t b b b b, f 200"), "t b b b b");
    }

    #[test]
    fn message_targets_single_leading_send() {
        // `\;`-introduced sub-message: target is `pitch`.
        let t = message_send_targets(r"#X msg 19 89 \; pitch 60;");
        assert_eq!(t, vec![(5, "pitch".to_string())]);
    }

    #[test]
    fn message_targets_multiple_sends() {
        let t = message_send_targets(r"#X msg 10 10 \; pitch 60 \; velocity 100 \; gate 1;");
        let names: Vec<&str> = t.iter().map(|(_, n)| n.as_str()).collect();
        assert_eq!(names, vec!["pitch", "velocity", "gate"]);
    }

    #[test]
    fn message_targets_leading_outlet_message_is_not_a_target() {
        // `set 1` goes out the box outlet; only `foo` (after `\;`) is a target.
        let t = message_send_targets(r"#X msg 10 10 set 1 \; foo 2;");
        assert_eq!(t, vec![(7, "foo".to_string())]);
    }

    #[test]
    fn message_targets_none_without_escaped_semicolon() {
        assert!(message_send_targets("#X msg 10 10 bang;").is_empty());
    }

    #[test]
    fn message_targets_ignores_non_message_entries() {
        assert!(message_send_targets(r"#X obj 10 10 s foo;").is_empty());
        assert!(message_send_targets(r"#X text 10 10 \; not a send;").is_empty());
    }

    #[test]
    fn parse_struct_typed_fields() {
        let t = parse_struct("#N struct point float x float y symbol name;").unwrap();
        assert_eq!(t.name, "point");
        assert_eq!(t.fields.len(), 3);
        assert_eq!(t.fields[0].field_type, TemplateFieldType::Float);
        assert_eq!(t.fields[0].name, "x");
        assert_eq!(t.fields[2].field_type, TemplateFieldType::Symbol);
        assert_eq!(t.scalar_field_count(), 3);
    }

    #[test]
    fn parse_struct_array_field_has_subtemplate_and_is_not_scalar_field() {
        let t = parse_struct("#N struct holder float x float y array z element;").unwrap();
        assert_eq!(t.fields.len(), 3);
        let arr = &t.fields[2];
        assert_eq!(arr.field_type, TemplateFieldType::Array);
        assert_eq!(arr.name, "z");
        assert_eq!(arr.array_template.as_deref(), Some("element"));
        // Array/text fields do not count toward the scalar (flat) field count.
        assert_eq!(t.scalar_field_count(), 2);
    }

    #[test]
    fn parse_struct_rejects_non_struct() {
        assert!(parse_struct("#N canvas 0 22 450 300 12;").is_none());
        assert!(parse_struct("#X obj 10 10 struct foo;").is_none());
    }

    #[test]
    fn parse_scalar_flat_values_stop_at_first_escaped_semi() {
        // Real inline scalar: flat fields, then `\;`-separated array data.
        let (tmpl, flat) = parse_scalar(r"#X scalar holder 5 6 \; 1 \; 2 \;;").unwrap();
        assert_eq!(tmpl, "holder");
        assert_eq!(flat, vec!["5", "6"]);
    }

    #[test]
    fn parse_scalar_without_array_data() {
        let (tmpl, flat) = parse_scalar("#X scalar point 10 20;").unwrap();
        assert_eq!(tmpl, "point");
        assert_eq!(flat, vec!["10", "20"]);
    }

    #[test]
    fn parse_scalar_single_field_with_trailing_escaped_semi() {
        // Matches a real single-field data-structure patch shape.
        let (tmpl, flat) = parse_scalar(r"#X scalar 1017-DS-1 66.6 \;;").unwrap();
        assert_eq!(tmpl, "1017-DS-1");
        assert_eq!(flat, vec!["66.6"]);
    }

    #[test]
    fn parse_scalar_rejects_non_scalar() {
        assert!(parse_scalar("#X obj 10 10 print;").is_none());
    }

    #[test]
    fn message_targets_position_is_whitespace_token_index() {
        // Position must index into split_whitespace() so callers can rewrite it.
        let raw = r"#X msg 19 89 \; 01_arrays read foo.mseq;";
        let t = message_send_targets(raw);
        assert_eq!(t.len(), 1);
        let (pos, name) = &t[0];
        assert_eq!(name, "01_arrays");
        let toks: Vec<&str> = raw.split_whitespace().collect();
        assert_eq!(toks[*pos], "01_arrays");
    }
}
