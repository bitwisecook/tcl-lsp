//! Parameter-list parser for Tcl proc declarations.
//!
//! Splits a parameter-list
//! string (the literal `args` argument to `proc`) into [`ParamDef`]
//! records, recognising the bare-word and `{name default}` forms.

use super::types::ParamDef;

/// Parse a Tcl proc argument list string into [`ParamDef`] records.
///
/// Handles both bare-word (`a b c`) and braced-with-default
/// (`{name default}`) forms. The input is the verbatim text of the
/// proc's parameter argument; outer whitespace is tolerated.
///
/// ```
/// use tcl_compiler::signature_scan::params::parse_param_list;
/// let params = parse_param_list("a {b 1} c");
/// assert_eq!(params.len(), 3);
/// assert_eq!(params[1].name, "b");
/// assert_eq!(params[1].default_value.as_deref(), Some("1"));
/// ```
#[must_use]
pub fn parse_param_list(param_str: &str) -> Vec<ParamDef> {
    let mut params: Vec<ParamDef> = Vec::new();
    let text = param_str.trim();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while let Some(len) = separator_len(bytes, i) {
            i += len;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'{' {
            let mut level: u32 = 1;
            i += 1;
            let start = i;
            while i < bytes.len() && level > 0 {
                match bytes[i] {
                    b'{' => level += 1,
                    b'}' => level -= 1,
                    _ => {}
                }
                i += 1;
            }
            // `i` now points one past the matching '}' (or end-of-input
            // for an unbalanced brace, which we tolerate by treating
            // the entire remainder as the inner text).
            let inner_end = if level == 0 { i - 1 } else { i };
            let inner = text[start..inner_end].trim();
            if inner.is_empty() {
                continue;
            }
            if let Some((name, default)) = split_first_whitespace(inner) {
                params.push(ParamDef {
                    name: name.to_string(),
                    has_default: true,
                    default_value: Some(default.to_string()),
                });
            } else {
                params.push(ParamDef {
                    name: inner.to_string(),
                    has_default: false,
                    default_value: None,
                });
            }
        } else {
            let start = i;
            i = scan_bare_word(bytes, i);
            let word = &text[start..i];
            if !word.is_empty() {
                params.push(ParamDef {
                    name: word.to_string(),
                    has_default: false,
                    default_value: None,
                });
            }
        }
    }
    params
}

/// Source spans of each parameter *name*, in declaration order, within the
/// **raw** param-list literal `raw` (exactly as it appears in source, e.g.
/// `{a b}` or `{a {b 1}}` — one optional outer brace layer is stripped). Each
/// returned [`tcl_lexer::Span`] is offset by `base` (the literal's start byte
/// offset in the document), so the spans point at the parameter names in the
/// original source. The order matches [`parse_param_list`], so the two can be
/// zipped.
///
/// This exists so go-to-definition / references / rename on a formal parameter
/// resolve to the parameter *name* in the declaration, not the proc name or the
/// whole method body (issue #727).
#[must_use]
pub fn param_name_spans(raw: &str, base: u32) -> Vec<tcl_lexer::Span> {
    let bytes = raw.as_bytes();
    let n = bytes.len();
    // Strip exactly one outer `{…}` brace layer if the whole literal is braced.
    let (lo, hi) = if n >= 2 && bytes[0] == b'{' && bytes[n - 1] == b'}' {
        (1, n - 1)
    } else {
        (0, n)
    };
    // Absolute source offset of a byte index within `raw`.
    let abs = |off: usize| base.saturating_add(u32::try_from(off).unwrap_or(u32::MAX));
    let mut out = Vec::new();
    let mut i = lo;
    while i < hi {
        while let Some(len) = separator_len(&bytes[..hi], i) {
            i += len;
        }
        if i >= hi {
            break;
        }
        if bytes[i] == b'{' {
            // `{name default}` — the name is the first word inside the braces.
            let inner_start = i + 1;
            let mut level: u32 = 1;
            let mut j = inner_start;
            while j < hi && level > 0 {
                match bytes[j] {
                    b'{' => level += 1,
                    b'}' => level -= 1,
                    _ => {}
                }
                j += 1;
            }
            let inner_end = if level == 0 { j - 1 } else { j };
            let mut ns = inner_start;
            while ns < inner_end && is_whitespace_byte(bytes[ns]) {
                ns += 1;
            }
            let mut ne = ns;
            while ne < inner_end && !is_whitespace_byte(bytes[ne]) {
                ne += 1;
            }
            if ne > ns {
                out.push(tcl_lexer::Span::new(abs(ns), abs(ne)));
            }
            i = j;
        } else {
            let start = i;
            i = scan_bare_word(&bytes[..hi], i);
            if i > start {
                out.push(tcl_lexer::Span::new(abs(start), abs(i)));
            }
        }
    }
    out
}

#[inline]
fn is_whitespace_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// Length of the element separator starting at `bytes[i]`, if any.
///
/// A separator is either a single whitespace byte, or a **backslash-newline
/// line continuation** (`\<LF>`, `\<CR><LF>`, or `\<CR>`).  In Tcl list
/// parsing — which is how a `proc` / method parameter list is split — a
/// backslash-newline collapses to element-separating whitespace, so a long
/// parameter list wrapped across lines with `\` yields distinct parameters
/// (`{a b\<newline>c}` → `a`, `b`, `c`), not a bogus `b\` name (issue #743).
/// Every other backslash escape keeps the following byte inside the element
/// (see [`scan_bare_word`]).
#[inline]
fn separator_len(bytes: &[u8], i: usize) -> Option<usize> {
    let b = *bytes.get(i)?;
    if is_whitespace_byte(b) {
        return Some(1);
    }
    if b == b'\\' {
        return match bytes.get(i + 1) {
            Some(b'\n') => Some(2),
            Some(b'\r') if bytes.get(i + 2) == Some(&b'\n') => Some(3),
            Some(b'\r') => Some(2),
            _ => None,
        };
    }
    None
}

/// Scan one bare (unbraced) list element starting at `bytes[i]`, returning the
/// index one past its final byte.  Stops at the next [`separator_len`]
/// separator; a backslash that escapes any non-newline byte (`a\ b`, `a\tb`)
/// consumes both bytes so the escaped whitespace stays within the element,
/// mirroring Tcl's `TclFindElement`.
#[inline]
fn scan_bare_word(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && separator_len(bytes, i).is_none() {
        // A backslash before a non-newline byte escapes it into the element.
        i += if bytes[i] == b'\\' && i + 1 < bytes.len() {
            2
        } else {
            1
        };
    }
    i
}

/// Split on the first run of whitespace. Returns `None` when the input
/// contains no whitespace.
fn split_first_whitespace(s: &str) -> Option<(&str, &str)> {
    let bytes = s.as_bytes();
    let split_at = bytes.iter().position(|b| is_whitespace_byte(*b))?;
    let name = &s[..split_at];
    let rest = s[split_at..].trim_start();
    Some((name, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_params() {
        assert!(parse_param_list("").is_empty());
        assert!(parse_param_list("   \t\n  ").is_empty());
    }

    #[test]
    fn three_bare_names() {
        let params = parse_param_list("a b c");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "a");
        assert!(!params[0].has_default);
        assert_eq!(params[1].name, "b");
        assert_eq!(params[2].name, "c");
        assert!(params.iter().all(|p| p.default_value.is_none()));
    }

    #[test]
    fn braced_with_default() {
        let params = parse_param_list("{a default}");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "a");
        assert!(params[0].has_default);
        assert_eq!(params[0].default_value.as_deref(), Some("default"));
    }

    #[test]
    fn param_name_spans_match_names_in_raw_literal() {
        use tcl_lexer::Span;
        // Outer braces stripped; spans are offset by `base` and point at names.
        let raw = "{arg1 arg2}";
        let spans = param_name_spans(raw, 100);
        assert_eq!(spans, vec![Span::new(101, 105), Span::new(106, 110)]);
        // `{name default}` → the name only.
        let raw2 = "{a {b 1} c}";
        let spans2 = param_name_spans(raw2, 0);
        // a @1..2, b @4..5 (inside inner braces), c @9..10
        assert_eq!(
            spans2,
            vec![Span::new(1, 2), Span::new(4, 5), Span::new(9, 10)]
        );
        // Empty list.
        assert!(param_name_spans("{}", 0).is_empty());
        // Unbraced single word.
        assert_eq!(param_name_spans("args", 5), vec![Span::new(5, 9)]);
    }

    #[test]
    fn braced_single_element_no_default() {
        let params = parse_param_list("{a}");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "a");
        assert!(!params[0].has_default);
        assert!(params[0].default_value.is_none());
    }

    #[test]
    fn mixed_bare_and_braced_default() {
        let params = parse_param_list("a {b 1} c");
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "a");
        assert!(!params[0].has_default);
        assert_eq!(params[1].name, "b");
        assert!(params[1].has_default);
        assert_eq!(params[1].default_value.as_deref(), Some("1"));
        assert_eq!(params[2].name, "c");
        assert!(!params[2].has_default);
    }

    #[test]
    fn whitespace_padding_tolerated() {
        let params = parse_param_list("  a   b  ");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[1].name, "b");
    }

    #[test]
    fn backslash_newline_continuation_splits_params() {
        // Issue #743: a long parameter list wrapped with `\` at end-of-line.
        // Tcl list parsing treats the backslash-newline as element-separating
        // whitespace, so the last name on the wrapped line is `ddrtol`, not
        // `ddrtol\`.
        let params = parse_param_list("a b ddrtol\\\n            ddatol c");
        let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["a", "b", "ddrtol", "ddatol", "c"]);
        assert!(params.iter().all(|p| !p.name.contains('\\')));

        // `\r\n` line endings behave identically.
        let crlf = parse_param_list("x\\\r\ny");
        let crlf_names: Vec<&str> = crlf.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(crlf_names, vec!["x", "y"]);
    }

    #[test]
    fn backslash_escape_keeps_char_in_element() {
        // A non-newline backslash escape keeps the following byte in the same
        // element (matches Tcl `{a\ b}` → single element `a b`).
        let params = parse_param_list("a\\ b c");
        let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["a\\ b", "c"]);
    }

    #[test]
    fn param_name_spans_skip_continuation_backslash() {
        use tcl_lexer::Span;
        // `{a b\<newline>c}` — the span for `b` must not include the trailing
        // backslash, and `c` is a separate parameter.
        let raw = "{a b\\\nc}";
        let spans = param_name_spans(raw, 0);
        // a @1..2, b @3..4 (backslash at 4 excluded), c @6..7
        assert_eq!(spans, vec![Span::new(1, 2), Span::new(3, 4), Span::new(6, 7)]);
    }

    #[test]
    fn default_value_preserves_internal_whitespace() {
        let params = parse_param_list("{name default with spaces}");
        assert_eq!(params.len(), 1);
        assert_eq!(
            params[0].default_value.as_deref(),
            Some("default with spaces")
        );
    }
}
