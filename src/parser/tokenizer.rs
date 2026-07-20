#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizeWarning {
    UnterminatedEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenizeResult {
    pub entries: Vec<String>,
    pub warnings: Vec<TokenizeWarning>,
}

/// Split raw `.pd` text into entries, terminated at each unescaped `;`.
///
/// PD's binbuf reader (`binbuf_text` in `m_binbuf.c`) treats every unescaped
/// `;` as a message terminator regardless of line position, and `binbuf_write`
/// emits a newline only after a real terminator (no column wrapping).  So a
/// terminator can appear mid-line (two entries on one physical line) and an
/// entry can span multiple physical lines.  This tokenizer therefore splits on
/// every unescaped `;`, not just those at end-of-line: continuation lines are
/// joined until a terminator is found, and any trailing content after a
/// mid-line terminator begins a new entry.
///
/// A `;` is escaped (and so kept as message content) when preceded by an odd
/// number of backslashes, matching PD's `\;` convention.
#[must_use]
pub fn tokenize_entries(input: &str) -> TokenizeResult {
    if input.is_empty() {
        return TokenizeResult::default();
    }

    let mut result = TokenizeResult::default();
    let mut current = String::new();

    for line in input.lines() {
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);

        // Emit every complete entry the buffer now contains.  A single line
        // may hold several terminators (`a; b;`); a terminator may also be the
        // final character (the common one-entry-per-line case).
        while let Some(pos) = first_unescaped_semicolon(&current) {
            let entry = current[..=pos].to_string();
            result.entries.push(entry);
            // Whatever follows the terminator starts the next entry.  Leading
            // separator whitespace (spaces/newlines between messages) is not
            // entry content, so drop it.
            current = current[pos + 1..].trim_start().to_string();
        }
    }

    if !current.trim().is_empty() {
        result.entries.push(current);
        result.warnings.push(TokenizeWarning::UnterminatedEntry);
    }

    result
}

/// Byte position of the first unescaped `;` in `text`, or `None`.
///
/// A `;` is unescaped when preceded by an even number (including zero) of
/// consecutive backslashes.
fn first_unescaped_semicolon(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b';' {
            continue;
        }
        let mut backslashes = 0usize;
        let mut idx = i;
        while idx > 0 && bytes[idx - 1] == b'\\' {
            backslashes += 1;
            idx -= 1;
        }
        if backslashes.is_multiple_of(2) {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_single_line_entry() {
        let input = "#N canvas 0 22 450 300 12;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.entries[0], input);
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_multiline_entry_joins_lines() {
        let input = "#X msg 50 50 1 2 3\n4 5 6, f 40;\n#X obj 50 100 print;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 2);
        assert_eq!(out.entries[0], "#X msg 50 50 1 2 3\n4 5 6, f 40;");
        assert_eq!(out.entries[1], "#X obj 50 100 print;");
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_escaped_semicolon_does_not_split() {
        let input = "#X msg 50 50 \\; pd dsp 1\n;\n#X obj 50 100 print;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 2);
        assert_eq!(out.entries[0], "#X msg 50 50 \\; pd dsp 1\n;");
        assert_eq!(out.entries[1], "#X obj 50 100 print;");
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_double_backslash_before_semicolon_does_split() {
        let input = "#X msg 50 50 path \\\\;\n#X obj 10 10 print;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 2);
        assert_eq!(out.entries[0], "#X msg 50 50 path \\\\;");
        assert_eq!(out.entries[1], "#X obj 10 10 print;");
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_empty_input_returns_empty_vec() {
        let out = tokenize_entries("");
        assert!(out.entries.is_empty());
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_unterminated_entry_returns_warning() {
        let input = "#X obj 10 10 loadbang";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.entries[0], "#X obj 10 10 loadbang");
        assert_eq!(out.warnings, vec![TokenizeWarning::UnterminatedEntry]);
    }

    #[test]
    fn tokenize_multiple_entries_correct_count() {
        let input = "#N canvas 0 0 100 100 10;\n#X obj 10 10 f;\n#X connect 0 0 0 0;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 3);
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_preserves_inline_width_hint() {
        let input = "#X obj 100 100 t f f, f 8;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.entries[0], input);
        assert!(out.entries[0].contains(", f 8"));
        assert!(out.warnings.is_empty());
    }

    // --- Feature E: split on any unescaped `;`, not just end-of-line ---

    #[test]
    fn tokenize_two_entries_on_one_line_split() {
        // PD's binbuf_text terminates at each unescaped `;` regardless of
        // newlines. A hand-written / generated file may pack several entries
        // onto one physical line; we must match PD and split them.
        let input = "#X obj 10 10 print; #X obj 10 20 print;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 2);
        assert_eq!(out.entries[0], "#X obj 10 10 print;");
        assert_eq!(out.entries[1], "#X obj 10 20 print;");
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_three_entries_on_one_line_split() {
        let input = "#X connect 0 0 1 0;#X connect 1 0 2 0;#X connect 2 0 3 0;";
        let out = tokenize_entries(input);

        assert_eq!(
            out.entries,
            vec![
                "#X connect 0 0 1 0;",
                "#X connect 1 0 2 0;",
                "#X connect 2 0 3 0;",
            ]
        );
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_mid_line_terminator_then_continuation() {
        // First entry ends mid-line; the remainder is an unterminated entry
        // that continues onto the next physical line before terminating.
        let input = "#X obj 10 10 print; #X msg 10 20 hello\nworld;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 2);
        assert_eq!(out.entries[0], "#X obj 10 10 print;");
        assert_eq!(out.entries[1], "#X msg 10 20 hello\nworld;");
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_escaped_semicolon_not_split_when_followed_by_content() {
        // The `\;` inside a message must NOT split even though a real
        // terminator follows later on the same line.
        let input = "#X msg 19 89 \\; target read foo.mseq;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.entries[0], input);
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_message_with_multiple_escaped_sends_one_entry() {
        // A single message box with several `\;`-introduced sub-messages is
        // one entry (only the final unescaped `;` terminates it).
        let input = "#X msg 10 10 \\; a 1 \\; b 2 \\; c 3;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.entries[0], input);
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_roundtrip_normal_file_unchanged() {
        // The common one-entry-per-line layout must tokenize identically to
        // the pre-change behaviour (byte-for-byte round trip via join).
        let input = "#N canvas 0 0 100 100 10;\n#X obj 10 10 osc~ 440;\n#X obj 10 40 dac~;\n#X connect 0 0 1 0;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 4);
        assert_eq!(out.entries.join("\n"), input);
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn tokenize_trailing_terminator_after_multiline_then_new_entry_same_line() {
        // Multi-line entry that terminates, immediately followed by another
        // entry on the terminating line.
        let input = "#X msg 50 50 1 2 3\n4 5 6; #X obj 50 100 print;";
        let out = tokenize_entries(input);

        assert_eq!(out.entries.len(), 2);
        assert_eq!(out.entries[0], "#X msg 50 50 1 2 3\n4 5 6;");
        assert_eq!(out.entries[1], "#X obj 50 100 print;");
        assert!(out.warnings.is_empty());
    }
}
