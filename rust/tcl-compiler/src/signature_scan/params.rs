//! Parameter-list parser for Tcl proc declarations.
//!
//! Port of `parse_param_list` in
//! `core/analysis/_analyser/_utils.py`. Splits a parameter-list
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
        while i < bytes.len() && is_whitespace_byte(bytes[i]) {
            i += 1;
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
            while i < bytes.len() && !is_whitespace_byte(bytes[i]) {
                i += 1;
            }
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

#[inline]
fn is_whitespace_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// Split on the first run of whitespace, mirroring Python's
/// `str.split(None, 1)`. Returns `None` when the input contains no
/// whitespace.
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
    fn default_value_preserves_internal_whitespace() {
        let params = parse_param_list("{name default with spaces}");
        assert_eq!(params.len(), 1);
        assert_eq!(
            params[0].default_value.as_deref(),
            Some("default with spaces")
        );
    }
}
