/// Escape unescaped `$` that introduce PD creation args (`$0`..`$9`).
///
/// In `.pd` files, `$` followed by a digit must be written as `\$`.
/// Existing escapes are preserved, and `$` followed by non-digits (e.g. `$f1`
/// in `expr`) is left unchanged.
pub fn escape_pd_dollars(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len() + 4);
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\\' {
            out.push('\\');
            i += 1;
            if i < bytes.len() {
                out.push(bytes[i] as char);
                i += 1;
            }
        } else if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            out.push('\\');
            out.push('$');
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }

    out
}

/// Return true if `text` contains any semicolon that is not escaped.
///
/// A semicolon is considered escaped only when preceded by an odd number of
/// consecutive backslashes.
pub fn has_unescaped_semicolon(text: &str) -> bool {
    let bytes = text.as_bytes();

    for i in 0..bytes.len() {
        if bytes[i] != b';' {
            continue;
        }

        let mut backslashes = 0usize;
        let mut j = i;
        while j > 0 && bytes[j - 1] == b'\\' {
            backslashes += 1;
            j -= 1;
        }

        if backslashes.is_multiple_of(2) {
            return true;
        }
    }

    false
}

/// Return true if `text` contains any unescaped `$` followed by a digit.
pub fn has_unescaped_dollar_digit(text: &str) -> bool {
    let bytes = text.as_bytes();

    for i in 0..bytes.len() {
        if bytes[i] != b'$' {
            continue;
        }
        if i + 1 >= bytes.len() || !bytes[i + 1].is_ascii_digit() {
            continue;
        }

        let mut backslashes = 0usize;
        let mut j = i;
        while j > 0 && bytes[j - 1] == b'\\' {
            backslashes += 1;
            j -= 1;
        }

        if backslashes.is_multiple_of(2) {
            return true;
        }
    }

    false
}

/// Return true if the entry body (excluding a trailing terminator `;`) contains
/// an unescaped semicolon.
pub fn has_unescaped_semicolon_in_body(entry_raw: &str) -> bool {
    let trimmed = entry_raw.trim_end();
    if let Some(body) = trimmed.strip_suffix(';') {
        has_unescaped_semicolon(body)
    } else {
        has_unescaped_semicolon(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        escape_pd_dollars, has_unescaped_dollar_digit, has_unescaped_semicolon,
        has_unescaped_semicolon_in_body,
    };

    #[test]
    fn escapes_bare_dollar_digit() {
        assert_eq!(escape_pd_dollars("$1"), r"\$1");
        assert_eq!(escape_pd_dollars("s$1_out"), r"s\$1_out");
    }

    #[test]
    fn does_not_double_escape_existing_dollar() {
        assert_eq!(escape_pd_dollars(r"\$1"), r"\$1");
        assert_eq!(escape_pd_dollars(r"\$1 $2"), r"\$1 \$2");
    }

    #[test]
    fn does_not_escape_expr_dollar_f() {
        assert_eq!(escape_pd_dollars("expr $f1 + $f2"), "expr $f1 + $f2");
    }

    #[test]
    fn detects_unescaped_semicolon() {
        assert!(has_unescaped_semicolon("foo ; bar"));
        assert!(has_unescaped_semicolon(r"foo \\; bar"));
        assert!(!has_unescaped_semicolon(r"foo \; bar"));
    }

    #[test]
    fn detects_unescaped_dollar_digit() {
        assert!(has_unescaped_dollar_digit("$1"));
        assert!(has_unescaped_dollar_digit("foo $2 bar"));
        assert!(!has_unescaped_dollar_digit(r"\$1"));
        assert!(!has_unescaped_dollar_digit("expr $f1 + $f2"));
    }

    #[test]
    fn detects_unescaped_semicolon_in_body_only() {
        assert!(has_unescaped_semicolon_in_body("#X msg 10 10 foo ; bar;"));
        assert!(!has_unescaped_semicolon_in_body(
            r"#X msg 10 10 foo \; bar;"
        ));
    }
}
