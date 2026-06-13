//! Compile-time evaluator for `[subst -nocommands {template}]` (C36a).
//!
//! Used by the C36c lowering hook when we see a `proc $var [subst
//! -nocommands {…}]` shape and want to materialise the body string
//! at compile time instead of deferring to the runtime interpreter.
//!
//! Semantics (matching `tclsh 9.0`'s `subst -nocommands`):
//!
//! * `$var` / `${var}` — variable substitution. The name is looked
//!   up in the supplied const-map; a miss refuses the whole
//!   evaluation by returning `None` (the caller keeps the dynamic
//!   dispatch path in that case).
//! * `\…` — standard backslash processing via
//!   [`tcl_lexer::backslash_subst`]. Handles `\n \t \xNN \uNNNN`
//!   and octal / continuation-line forms.
//! * `[…]` — left as a literal `[…]` string (the `-nocommands`
//!   flag is exactly this: skip command substitution). Unbalanced
//!   `[` / `]` pairs are a user error in real Tcl; we reject them
//!   by returning `None`.
//! * `$a(b)` — array references are refused.
//! * `$::ns::var` — namespace-qualified var refs are refused.
//!
//! Mirrors `core/parsing/subst_nocommands.py` (main commit
//! `8a73e0ac`).

use std::collections::HashMap;
use std::hash::BuildHasher;

/// Evaluate a `subst -nocommands` template at compile time.
///
/// Returns the substituted string on success, or `None` if any
/// condition above refuses the evaluation. Refusal is always safe
/// — the caller falls back to runtime dispatch, preserving the
/// original semantics.
///
/// *`const_map`* maps variable names (without the leading `$`) to
/// their literal string values. Names are looked up with their
/// `{…}`-stripped form, so `${foo}` and `$foo` resolve the same
/// entry.
#[must_use]
pub fn subst_nocommands<S: BuildHasher>(
    template: &str,
    const_map: &HashMap<String, String, S>,
) -> Option<String> {
    let bytes = template.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0usize;
    while i < n {
        let c = bytes[i];
        if c == b'\\' {
            // Defer to the shared backslash processor for the
            // single following escape (plus continuation-line and
            // octal / hex / unicode forms).
            let j = backslash_end(bytes, i);
            let decoded = tcl_lexer::backslash_subst(&template[i..j]);
            out.push_str(&decoded);
            i = j;
            continue;
        }
        if c == b'$' {
            // ``$$`` — Tcl treats first ``$`` as start of a name,
            // and if no name follows, leaves it literal.
            if i + 1 >= n {
                out.push('$');
                i += 1;
                continue;
            }
            let nxt = bytes[i + 1];
            if nxt == b'{' {
                let close = template[i + 2..].find('}').map(|p| i + 2 + p)?;
                let name = &template[i + 2..close];
                if is_complex_var_name(name) {
                    return None;
                }
                let value = const_map.get(name)?;
                out.push_str(value);
                i = close + 1;
                continue;
            }
            if is_name_byte(nxt) {
                let mut j = i + 1;
                while j < n && is_name_byte(bytes[j]) {
                    j += 1;
                }
                let name = &template[i + 1..j];
                // Array reference ``$name(index)`` — refuse.
                if j < n && bytes[j] == b'(' {
                    return None;
                }
                // Namespace qualifier ``$a::b`` — refuse.
                if j + 1 < n && bytes[j] == b':' && bytes[j + 1] == b':' {
                    return None;
                }
                let value = const_map.get(name)?;
                out.push_str(value);
                i = j;
                continue;
            }
            // ``$::name`` — refuse.
            if nxt == b':' {
                return None;
            }
            // ``$`` followed by a non-name character — leave the
            // ``$`` literal and continue.
            out.push('$');
            i += 1;
            continue;
        }
        if c == b'[' {
            let close = match_bracket(bytes, i)?;
            out.push_str(&template[i..=close]);
            i = close + 1;
            continue;
        }
        if c == b']' {
            // A stray ``]`` without a matching ``[`` — output it.
            out.push(']');
            i += 1;
            continue;
        }
        // ASCII fast path; for non-ASCII we read as a char.
        if c < 128 {
            out.push(c as char);
            i += 1;
        } else {
            let ch = template[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    Some(out)
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn backslash_end(bytes: &[u8], start: usize) -> usize {
    let n = bytes.len();
    if start + 1 >= n {
        return n;
    }
    let c = bytes[start + 1];
    match c {
        b'x' => {
            let mut j = start + 2;
            while j < n && j < start + 4 && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j > start + 2 {
                j
            } else {
                start + 2
            }
        }
        b'u' => {
            let mut j = start + 2;
            while j < n && j < start + 6 && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j > start + 2 {
                j
            } else {
                start + 2
            }
        }
        b'U' => {
            let mut j = start + 2;
            while j < n && j < start + 10 && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j > start + 2 {
                j
            } else {
                start + 2
            }
        }
        b'0'..=b'7' => {
            let mut j = start + 1;
            while j < n && j < start + 4 && (b'0'..=b'7').contains(&bytes[j]) {
                j += 1;
            }
            j
        }
        b'\n' => {
            let mut j = start + 2;
            while j < n && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            j
        }
        b'\r' => {
            let mut j = start + 2;
            if j < n && bytes[j] == b'\n' {
                j += 1;
            }
            while j < n && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            j
        }
        _ => start + 2,
    }
}

fn match_bracket(bytes: &[u8], start: usize) -> Option<usize> {
    let n = bytes.len();
    let mut depth = 1i32;
    let mut i = start + 1;
    while i < n {
        let c = bytes[i];
        if c == b'\\' && i + 1 < n {
            i += 2;
            continue;
        }
        if c == b'[' {
            depth += 1;
        } else if c == b']' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_complex_var_name(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    if name.contains("::")
        || name.contains('(')
        || name.contains(')')
        || name.contains('$')
        || name.contains('[')
    {
        return true;
    }
    !name.bytes().all(is_name_byte)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn simple_var_substitution() {
        let m = map_of(&[("name", "world")]);
        assert_eq!(
            subst_nocommands("hello $name", &m).as_deref(),
            Some("hello world")
        );
    }

    #[test]
    fn braced_var_substitution() {
        let m = map_of(&[("name", "world")]);
        assert_eq!(
            subst_nocommands("hello ${name}", &m).as_deref(),
            Some("hello world")
        );
    }

    #[test]
    fn missing_var_returns_none() {
        let m: HashMap<String, String> = HashMap::new();
        assert!(subst_nocommands("$missing", &m).is_none());
    }

    #[test]
    fn brackets_kept_literal() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(
            subst_nocommands("[cmd arg]", &m).as_deref(),
            Some("[cmd arg]")
        );
    }

    #[test]
    fn unbalanced_bracket_refused() {
        let m: HashMap<String, String> = HashMap::new();
        assert!(subst_nocommands("[unbalanced", &m).is_none());
    }

    #[test]
    fn array_ref_refused() {
        let m = map_of(&[("a", "x")]);
        assert!(subst_nocommands("$a(idx)", &m).is_none());
    }

    #[test]
    fn namespace_qualified_refused() {
        let m = map_of(&[("a", "x")]);
        assert!(subst_nocommands("$a::b", &m).is_none());
        assert!(subst_nocommands("$::name", &m).is_none());
    }

    #[test]
    fn dollar_dollar_keeps_first_literal() {
        // ``$$`` — first ``$`` followed by ``$`` (not a name char).
        // The first ``$`` stays literal, then the second ``$`` is
        // also followed by end-of-string, so it stays literal too.
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands("$$", &m).as_deref(), Some("$$"));
    }

    #[test]
    fn backslash_n_decoded() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands(r"\n", &m).as_deref(), Some("\n"));
    }

    #[test]
    fn backslash_x_hex() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands(r"\x41", &m).as_deref(), Some("A"));
    }

    #[test]
    fn empty_template() {
        let m: HashMap<String, String> = HashMap::new();
        assert_eq!(subst_nocommands("", &m).as_deref(), Some(""));
    }

    #[test]
    fn mixed_var_and_brackets() {
        let m = map_of(&[("name", "foo")]);
        assert_eq!(
            subst_nocommands("$name [list a b]", &m).as_deref(),
            Some("foo [list a b]")
        );
    }

    #[test]
    fn empty_braced_var_refused() {
        let m: HashMap<String, String> = HashMap::new();
        assert!(subst_nocommands("${}", &m).is_none());
    }
}
