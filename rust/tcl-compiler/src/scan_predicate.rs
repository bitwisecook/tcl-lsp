//! Static `scan` no-match proof — `scan_provably_no_match`.
//!
//! Determines whether `scan INPUT FORMAT ?vars...?` is statically guaranteed
//! to leave at least the **first** output variable unset (the first
//! conversion can't consume any input).  Only returns `true` when fully
//! confident; any uncertain case returns `false` so callers never fire a
//! spurious W210.  Ported from `compiler/scan_format.py`.

/// Tcl scan whitespace set (the C `isspace` characters).
fn is_scan_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0c}' | '\u{0b}')
}

/// Advance over input whitespace.
fn skip_leading_whitespace(s: &[char], mut i: usize) -> usize {
    while i < s.len() && is_scan_ws(s[i]) {
        i += 1;
    }
    i
}

/// Conversion-leader class for a `scan` conversion character, or `None` for
/// an unmodelled conversion.
fn conv_leader(conv: char) -> Option<&'static str> {
    Some(match conv {
        'd' | 'i' | 'u' => "decimal-int",
        'o' => "octal-int",
        'x' | 'X' => "hex-int",
        'b' => "binary-int",
        'c' => "any",
        's' => "non-space",
        'e' | 'E' | 'f' | 'g' | 'G' => "float",
        'n' => "always",
        _ => return None,
    })
}

/// True iff `s[i..]` starts with an optional sign followed by ≥1 digit from
/// `digits`.
fn starts_with_int(s: &[char], mut i: usize, digits: &str) -> bool {
    if i >= s.len() {
        return false;
    }
    if s[i] == '+' || s[i] == '-' {
        i += 1;
    }
    i < s.len() && digits.contains(s[i])
}

/// Can the input at offset `i` satisfy a single conversion `conv`?
/// `Some(true)`/`Some(false)`, or `None` when unmodelled.
fn input_satisfies_conv(conv: char, s: &[char], i: usize) -> Option<bool> {
    let kind = conv_leader(conv)?;
    Some(match kind {
        // `%n` writes the consumed count and never consumes input.
        "always" => true,
        "any" => i < s.len(),
        "non-space" => i < s.len() && !is_scan_ws(s[i]),
        "decimal-int" => starts_with_int(s, i, "0123456789"),
        "octal-int" => starts_with_int(s, i, "01234567"),
        "binary-int" => starts_with_int(s, i, "01"),
        "hex-int" => starts_with_int(s, i, "0123456789abcdefABCDEF"),
        "float" => {
            if i >= s.len() {
                return Some(false);
            }
            let mut j = i;
            if s[j] == '+' || s[j] == '-' {
                j += 1;
                if j >= s.len() {
                    return Some(false);
                }
            }
            let c = s[j];
            if c.is_ascii_digit() {
                return Some(true);
            }
            if c == '.' && j + 1 < s.len() && s[j + 1].is_ascii_digit() {
                return Some(true);
            }
            // `Inf` / `Infinity` / `NaN` (case-insensitive prefix).
            let tail: String = s[j..].iter().collect::<String>().to_ascii_lowercase();
            tail.starts_with("inf") || tail.starts_with("nan")
        }
        _ => return None,
    })
}

/// Return `Some(true)`/`Some(false)` if we can prove the first conversion
/// will / will not consume input; `None` when outside the modelled subset.
fn can_consume_first_conversion(format_str: &[char], input_str: &[char]) -> Option<bool> {
    let mut fi = 0;
    let mut si = 0;
    let n_fmt = format_str.len();
    let n_inp = input_str.len();
    while fi < n_fmt {
        let fc = format_str[fi];
        if fc == '%' {
            fi += 1;
            if fi >= n_fmt {
                return None;
            }
            if format_str[fi] == '%' {
                // `%%` matches a literal `%`.
                if si >= n_inp || input_str[si] != '%' {
                    return Some(false);
                }
                si += 1;
                fi += 1;
                continue;
            }
            // Assignment-suppression `*`.
            if format_str[fi] == '*' {
                fi += 1;
                if fi >= n_fmt {
                    return None;
                }
            }
            // Width digits.
            while fi < n_fmt && format_str[fi].is_ascii_digit() {
                fi += 1;
            }
            if fi >= n_fmt {
                return None;
            }
            // Size modifier (`h`, `l`, `ll`).
            while fi < n_fmt && (format_str[fi] == 'h' || format_str[fi] == 'l') {
                fi += 1;
            }
            if fi >= n_fmt {
                return None;
            }
            let conv = format_str[fi];
            if conv == '[' {
                // Character-set conversion — unmodelled, stay conservative.
                return None;
            }
            // Non-`[` conversions skip leading whitespace except `%c`/`%n`.
            if conv != 'c' && conv != 'n' {
                si = skip_leading_whitespace(input_str, si);
            }
            return input_satisfies_conv(conv, input_str, si);
        }
        // Whitespace directive in the format matches an input ws run.
        if is_scan_ws(fc) {
            fi += 1;
            si = skip_leading_whitespace(input_str, si);
            continue;
        }
        // Plain literal character: must match verbatim.
        if si >= n_inp || input_str[si] != fc {
            return Some(false);
        }
        si += 1;
        fi += 1;
    }
    // No conversion in the format — nothing to write, no-match is meaningless.
    None
}

/// True iff `scan INPUT FORMAT ?vars...?` is statically guaranteed to leave
/// the first output variable unset.  `format_str` / `input_str` are RAW
/// source slices (no backslash processing); a `\`, `$`, or `[` bails.
#[must_use]
pub fn scan_provably_no_match(format_str: &str, input_str: &str) -> bool {
    if format_str.contains(['\\', '$', '[']) || input_str.contains(['\\', '$', '[']) {
        return false;
    }
    let fmt: Vec<char> = format_str.chars().collect();
    let inp: Vec<char> = input_str.chars().collect();
    can_consume_first_conversion(&fmt, &inp) == Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provable_no_match_cases() {
        assert!(scan_provably_no_match("%d", "abc")); // digits vs letters
        assert!(scan_provably_no_match("%d", "")); // empty input
        assert!(scan_provably_no_match("x%d", "y3")); // literal mismatch
    }

    #[test]
    fn matchable_or_uncertain_cases() {
        assert!(!scan_provably_no_match("%d", "42"));
        assert!(!scan_provably_no_match("%s", "abc"));
        assert!(!scan_provably_no_match("%n", "")); // %n always succeeds
        assert!(!scan_provably_no_match("%f", "Inf"));
        assert!(!scan_provably_no_match("\\r%d", " 123")); // backslash bail
        assert!(!scan_provably_no_match("%[abc]", "x")); // char-set bail
        assert!(!scan_provably_no_match("plain", "plain")); // no conversion
    }
}
