use crate::model::EntryKind;

pub fn classify_entry(raw: &str) -> EntryKind {
    let trimmed = raw.trim();

    if trimmed.starts_with("#N canvas ") {
        return EntryKind::CanvasOpen;
    }

    if trimmed.starts_with("#A ") {
        return EntryKind::ArrayData;
    }

    if trimmed.starts_with("#C ") {
        return EntryKind::Unknown;
    }

    if !trimmed.starts_with("#X ") {
        return EntryKind::Unknown;
    }

    let after_x = &trimmed[3..];
    let mut parts = after_x.split_whitespace();
    let Some(head) = parts.next() else {
        return EntryKind::Unknown;
    };

    match head {
        "obj" => EntryKind::Obj,
        "msg" => EntryKind::Msg,
        "text" => EntryKind::Text,
        "floatatom" => EntryKind::FloatAtom,
        "symbolatom" => EntryKind::SymbolAtom,
        "restore" => EntryKind::Restore,
        "connect" => EntryKind::Connect,
        "coords" => EntryKind::Coords,
        "array" => EntryKind::Array,
        "declare" => EntryKind::Declare,
        "f" => {
            // Standalone width hint: #X f <number>;
            let rest = after_x[head.len()..].trim();
            let value = rest.trim_end_matches(';').trim();
            if value.parse::<i32>().is_ok() {
                EntryKind::WidthHint
            } else {
                EntryKind::Unknown
            }
        }
        _ => EntryKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_canvas_open() {
        assert_eq!(
            classify_entry("#N canvas 0 22 450 300 12;"),
            EntryKind::CanvasOpen
        );
    }

    #[test]
    fn classify_obj() {
        assert_eq!(classify_entry("#X obj 10 10 f;"), EntryKind::Obj);
    }

    #[test]
    fn classify_msg() {
        assert_eq!(classify_entry("#X msg 10 10 bang;"), EntryKind::Msg);
    }

    #[test]
    fn classify_text() {
        assert_eq!(
            classify_entry("#X text 10 10 comment here;"),
            EntryKind::Text
        );
    }

    #[test]
    fn classify_floatatom() {
        assert_eq!(
            classify_entry("#X floatatom 10 10 5 0 0 0 - - -;"),
            EntryKind::FloatAtom
        );
    }

    #[test]
    fn classify_symbolatom() {
        assert_eq!(
            classify_entry("#X symbolatom 10 10 10 0 0 0 - - -;"),
            EntryKind::SymbolAtom
        );
    }

    #[test]
    fn classify_restore_pd() {
        assert_eq!(
            classify_entry("#X restore 50 50 pd my_sub;"),
            EntryKind::Restore
        );
    }

    #[test]
    fn classify_restore_graph() {
        assert_eq!(
            classify_entry("#X restore 50 50 graph;"),
            EntryKind::Restore
        );
    }

    #[test]
    fn classify_connect() {
        assert_eq!(classify_entry("#X connect 0 0 1 0;"), EntryKind::Connect);
    }

    #[test]
    fn classify_coords() {
        assert_eq!(
            classify_entry("#X coords 0 1 127 -1 200 140 1 0 0;"),
            EntryKind::Coords
        );
    }

    #[test]
    fn classify_array_def() {
        assert_eq!(
            classify_entry("#X array my_array 100 float 3;"),
            EntryKind::Array
        );
    }

    #[test]
    fn classify_array_data() {
        assert_eq!(classify_entry("#A 0 0 1 2 3;"), EntryKind::ArrayData);
    }

    #[test]
    fn classify_standalone_declare() {
        assert_eq!(
            classify_entry("#X declare -path pos_abs;"),
            EntryKind::Declare
        );
    }

    #[test]
    fn classify_obj_declare_is_obj() {
        assert_eq!(
            classify_entry("#X obj 10 10 declare -path pos_abs;"),
            EntryKind::Obj
        );
    }

    #[test]
    fn classify_standalone_width_hint() {
        assert_eq!(classify_entry("#X f 38;"), EntryKind::WidthHint);
    }

    #[test]
    fn classify_obj_with_inline_f_is_obj() {
        assert_eq!(classify_entry("#X obj 100 100 t f f, f 8;"), EntryKind::Obj);
    }

    #[test]
    fn classify_c_entry() {
        assert_eq!(classify_entry("#C restore;"), EntryKind::Unknown);
    }

    #[test]
    fn classify_unknown_entry() {
        assert_eq!(classify_entry("#X foobar 10 10 ???;"), EntryKind::Unknown);
    }
}
