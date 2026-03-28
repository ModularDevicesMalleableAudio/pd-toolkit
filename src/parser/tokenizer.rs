#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizeWarning {
    UnterminatedEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenizeResult {
    pub entries: Vec<String>,
    pub warnings: Vec<TokenizeWarning>,
}

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

        if has_unescaped_terminator_at_end(&current) {
            result.entries.push(current.clone());
            current.clear();
        }
    }

    if !current.trim().is_empty() {
        result.entries.push(current);
        result.warnings.push(TokenizeWarning::UnterminatedEntry);
    }

    result
}

fn has_unescaped_terminator_at_end(text: &str) -> bool {
    let trimmed = text.trim_end();
    if !trimmed.ends_with(';') {
        return false;
    }

    let bytes = trimmed.as_bytes();
    let semicolon_pos = bytes.len() - 1;

    let mut backslashes = 0;
    let mut idx = semicolon_pos;
    while idx > 0 && bytes[idx - 1] == b'\\' {
        backslashes += 1;
        idx -= 1;
    }

    // odd number of backslashes => escaped semicolon
    backslashes % 2 == 0
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
}
