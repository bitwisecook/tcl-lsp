//! Variable and command name normalisation.
//!
//! Ports the relevant parts of `core/common/naming.py`. These live in
//! the compiler crate because they are consumed by the expression
//! parser and lowering — not by the lexer itself.
//!
//! The `::`-qualifier split ([`qualifier_segments`] / [`is_qualified`]) is the
//! **one** canonical source for namespace-name parsing, shared by the compiler
//! (`normalise_qualified_name`) and the WASM runtime's command **and** variable
//! resolvers (`runtime/rust/src/namespace.rs`, the var coordinator) — mirroring
//! C Tcl's `TclGetNamespaceForQualName` segmentation (`tmp/tcl9.0.3`). Byte-based
//! so the runtime (which works in UTF-8 bytes) and the compiler (`&str`) share it
//! without one re-deriving the other.

/// Does `name` contain a `::` namespace separator (i.e. is it qualified)?
#[must_use]
pub fn is_qualified(name: &[u8]) -> bool {
    name.windows(2).any(|w| w == b"::")
}

/// Split a (possibly qualified) name on `::`, dropping empty segments — so
/// `::a::b::cmd` → `[a, b, cmd]`, `::cmd` → `[cmd]`, `cmd` → `[cmd]`, `::` → `[]`.
/// Each `::` (or run of colons) is one separator; the trailing component is the
/// simple name. Mirrors `TclGetNamespaceForQualName`'s component walk.
#[must_use]
pub fn qualifier_segments(name: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut seg_start = 0;
    let mut i = 0;
    while i < name.len() {
        if name[i] == b':' && i + 1 < name.len() && name[i + 1] == b':' {
            if i > seg_start {
                out.push(&name[seg_start..i]);
            }
            i += 2;
            seg_start = i;
        } else {
            i += 1;
        }
    }
    if seg_start < name.len() {
        out.push(&name[seg_start..]);
    }
    out
}

/// Strip a variable reference's substitution sigil (`$`, `${…}`) while
/// **keeping** any array-index suffix — the form an evaluator needs to read the
/// actual variable (`$arr(idx)` → `arr(idx)`, `${v}` → `v`, `$x` → `x`). Unlike
/// [`normalise_var_name`], the `(idx)` is preserved.
///
/// ```
/// use tcl_syntax::naming::var_reference;
/// assert_eq!(var_reference("$arr(idx)"), "arr(idx)");
/// assert_eq!(var_reference("${v}"), "v");
/// assert_eq!(var_reference("$x"), "x");
/// ```
#[must_use]
pub fn var_reference(name: &str) -> &str {
    if let Some(inner) = name.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        inner
    } else if let Some(rest) = name.strip_prefix('$') {
        rest
    } else {
        name
    }
}

/// Normalise a Tcl variable reference to its base name.
///
/// Strips leading `$`, `${…}` delimiters, and array index `(…)`
/// suffixes:
///
/// ```
/// use tcl_syntax::naming::normalise_var_name;
/// assert_eq!(normalise_var_name("$foo"), "foo");
/// assert_eq!(normalise_var_name("${bar}"), "bar");
/// assert_eq!(normalise_var_name("$arr(idx)"), "arr");
/// assert_eq!(normalise_var_name("plain"), "plain");
/// ```
#[must_use]
pub fn normalise_var_name(name: &str) -> &str {
    let base = if let Some(inner) = name.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        inner
    } else if let Some(rest) = name.strip_prefix('$') {
        rest
    } else {
        name
    };

    // Strip array index: keep everything before the first `(`.
    match base.find('(') {
        Some(idx) => &base[..idx],
        None => base,
    }
}

/// Return `true` when `$name` would lex as a single bare variable token.
///
/// Mirrors `compiler/parsing/lexer._parse_var`'s bare-form rule: a name is
/// one or more `::`-separated segments, each consisting of Unicode alnum or
/// `_` characters, with an optional leading `::`.  Used to decide between
/// the bare `$name` and brace `${name}` forms in quick fixes.  Mirrors
/// `is_bare_var_name` in `shared/naming.py`.
///
/// ```
/// use tcl_syntax::naming::is_bare_var_name;
/// assert!(is_bare_var_name("foo"));
/// assert!(is_bare_var_name("ns::bar"));
/// assert!(is_bare_var_name("::baz"));
/// assert!(!is_bare_var_name("has-dash"));
/// assert!(!is_bare_var_name(""));
/// ```
#[must_use]
pub fn is_bare_var_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let s = name.strip_prefix("::").unwrap_or(name);
    if s.is_empty() {
        return false;
    }
    for segment in s.split("::") {
        if segment.is_empty() {
            return false;
        }
        if !segment.chars().all(|ch| ch.is_alphanumeric() || ch == '_') {
            return false;
        }
    }
    true
}

/// Return `true` when *word* is the braced indirect-array-element idiom
/// `${name}(index)`.
///
/// Tcl parses `${name}(index)` as the brace-form substitution `${name}`
/// (which the lexer ends at the `}`) concatenated with the *literal* text
/// `(index)`.  In a variable-name position (the target of `set` / `incr` /
/// `append` / `lappend` / `unset` / `info exists` / `vwait`) the resulting
/// string `<value-of-name>(index)` names element `index` of the array whose
/// name is held in the scalar `name` — the standard "array name kept in a
/// variable" idiom (e.g. `set ${token}(status) eof`).  The braces are
/// essential: the bare `$name(index)` is a *direct* array reference, a
/// different construct, so this returns `false` for it.
///
/// This is the discriminator that keeps the W216 (brace-then-paren) and W212
/// (substitution-where-name-expected) checks from false-positiving on the
/// indirect idiom.  Mirrors `is_braced_indirect_array_ref` in
/// `shared/naming.py`.
///
/// ```
/// use tcl_syntax::naming::is_braced_indirect_array_ref;
/// assert!(is_braced_indirect_array_ref("${token}(status)"));
/// assert!(!is_braced_indirect_array_ref("$arr(idx)"));
/// assert!(!is_braced_indirect_array_ref("${x}"));
/// assert!(!is_braced_indirect_array_ref("${}(x)"));
/// ```
#[must_use]
pub fn is_braced_indirect_array_ref(word: &str) -> bool {
    if !word.starts_with("${") {
        return false;
    }
    let bytes = word.as_bytes();
    let n = bytes.len();
    // Walk to the depth-0 `}` that closes the brace-form variable name.
    let mut i = 2usize;
    let mut depth = 0i32;
    let mut close: Option<usize> = None;
    while i < n {
        match bytes[i] {
            b'\\' if i + 1 < n => {
                i += 2;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    close = Some(i);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    // No closing brace, or an empty name (`${}(…)`) — not the idiom.  The
    // closing `}` must sit past the `${` and the (non-empty) name.
    let Some(close) = close.filter(|&c| c > 2) else {
        return false;
    };
    // The closing `}` must be immediately followed by `(…)` running to the
    // end of the word.
    let rest = &bytes[close + 1..];
    rest.len() >= 2 && rest[0] == b'(' && *rest.last().unwrap() == b')'
}

/// Normalise a possibly-qualified Tcl command or procedure name.
///
/// Ensures the name starts with `::` and removes empty parts from
/// consecutive `::` separators.
///
/// ```
/// use tcl_syntax::naming::normalise_qualified_name;
/// assert_eq!(normalise_qualified_name("foo"), "::foo");
/// assert_eq!(normalise_qualified_name("ns::bar"), "::ns::bar");
/// assert_eq!(normalise_qualified_name("::baz"), "::baz");
/// assert_eq!(normalise_qualified_name(""), "");
/// assert_eq!(normalise_qualified_name("::::x"), "::x");
/// ```
#[must_use]
pub fn normalise_qualified_name(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    // Share the one canonical `::` segmentation. Each segment is a subslice of a
    // `&str` split on ASCII `::`, so it is valid UTF-8.
    let parts: Vec<&str> = qualifier_segments(name.as_bytes())
        .into_iter()
        .map(|s| core::str::from_utf8(s).expect("subslice of valid UTF-8"))
        .collect();
    if parts.is_empty() {
        return "::".to_owned();
    }
    format!("::{}", parts.join("::"))
}

/// Split a Tcl variable reference into `(base, element)`.
///
/// Strips `$` / `${…}` substitution sigils first, then separates the
/// optional `(element)` array-index suffix from the base name.  Returns
/// `(base, None)` for scalar references.  Mirrors Python's
/// `shared/naming.py::split_array_name`, including the brace-form rule that
/// `${arr}(foo)` is the scalar `arr` followed by literal `(foo)`, whereas
/// `${arr(foo)}` *is* the array element `arr(foo)`.
///
/// ```
/// use tcl_syntax::naming::split_array_name;
/// assert_eq!(split_array_name("arr"), ("arr", None));
/// assert_eq!(split_array_name("arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("$arr(foo)"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr(foo)}"), ("arr", Some("foo")));
/// assert_eq!(split_array_name("${arr}(foo)"), ("arr", None));
/// assert_eq!(split_array_name("${arr}"), ("arr", None));
/// ```
#[must_use]
pub fn split_array_name(name: &str) -> (&str, Option<&str>) {
    // `${…}` brace form: only the chars inside the braces are the reference;
    // an `(idx)` *inside* the braces is an element, one *after* `}` is not.
    if let Some(after) = name.strip_prefix("${")
        && let Some(rel) = after.find('}')
    {
        let inner = &after[..rel];
        if inner.ends_with(')')
            && let Some(idx) = inner.find('(')
        {
            return (&inner[..idx], Some(&inner[idx + 1..inner.len() - 1]));
        }
        return (inner, None);
    }
    // No closing brace — fall through (Python gates on `"}" in base`).
    let base = name.strip_prefix('$').unwrap_or(name);
    if base.ends_with(')')
        && let Some(idx) = base.find('(')
    {
        return (&base[..idx], Some(&base[idx + 1..base.len() - 1]));
    }
    (base, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_dollar() {
        assert_eq!(normalise_var_name("$foo"), "foo");
    }

    #[test]
    fn split_array_name_forms() {
        assert_eq!(split_array_name("arr"), ("arr", None));
        assert_eq!(split_array_name("arr(foo)"), ("arr", Some("foo")));
        assert_eq!(split_array_name("$arr(foo)"), ("arr", Some("foo")));
        assert_eq!(split_array_name("${arr(foo)}"), ("arr", Some("foo")));
        // `${arr}(foo)` is scalar `arr` then literal `(foo)` — not an element.
        assert_eq!(split_array_name("${arr}(foo)"), ("arr", None));
        assert_eq!(split_array_name("${arr}"), ("arr", None));
        // dynamic index text is preserved verbatim for later classification.
        assert_eq!(split_array_name("a($i)"), ("a", Some("$i")));
        // a `)` with no `(` is not an element.
        assert_eq!(split_array_name("weird)"), ("weird)", None));
    }

    #[test]
    fn braced_dollar() {
        assert_eq!(normalise_var_name("${bar}"), "bar");
    }

    #[test]
    fn array_stripped() {
        assert_eq!(normalise_var_name("$arr(idx)"), "arr");
    }

    #[test]
    fn braced_array_stripped() {
        assert_eq!(normalise_var_name("${arr(idx)}"), "arr");
    }

    #[test]
    fn no_prefix() {
        assert_eq!(normalise_var_name("plain"), "plain");
    }

    #[test]
    fn namespace_qualified() {
        assert_eq!(normalise_var_name("$ns::var"), "ns::var");
    }

    #[test]
    fn empty_string() {
        assert_eq!(normalise_var_name(""), "");
    }

    #[test]
    fn bare_dollar() {
        assert_eq!(normalise_var_name("$"), "");
    }

    // normalise_qualified_name tests

    #[test]
    fn qualified_bare() {
        assert_eq!(normalise_qualified_name("foo"), "::foo");
    }

    #[test]
    fn qualified_already() {
        assert_eq!(normalise_qualified_name("::bar"), "::bar");
    }

    #[test]
    fn qualified_nested() {
        assert_eq!(normalise_qualified_name("ns::sub"), "::ns::sub");
    }

    #[test]
    fn qualified_empty() {
        assert_eq!(normalise_qualified_name(""), "");
    }

    #[test]
    fn qualified_just_colons() {
        assert_eq!(normalise_qualified_name("::"), "::");
    }

    #[test]
    fn qualified_extra_colons() {
        assert_eq!(normalise_qualified_name("::::x"), "::x");
    }

    // qualifier_segments / is_qualified

    #[test]
    fn qualifier_segments_cases() {
        assert_eq!(
            qualifier_segments(b"::a::b::cmd"),
            vec![&b"a"[..], b"b", b"cmd"]
        );
        assert_eq!(qualifier_segments(b"::cmd"), vec![&b"cmd"[..]]);
        assert_eq!(qualifier_segments(b"cmd"), vec![&b"cmd"[..]]);
        assert_eq!(qualifier_segments(b"a::b"), vec![&b"a"[..], b"b"]);
        assert!(qualifier_segments(b"::").is_empty());
        // a trailing separator drops the empty tail; a lone interior colon stays.
        assert_eq!(qualifier_segments(b"a::b::"), vec![&b"a"[..], b"b"]);
        assert_eq!(qualifier_segments(b"a:::b"), vec![&b"a"[..], b":b"]);
    }

    #[test]
    fn is_qualified_cases() {
        assert!(is_qualified(b"::a"));
        assert!(is_qualified(b"a::b"));
        assert!(is_qualified(b"::"));
        assert!(!is_qualified(b"plain"));
        assert!(!is_qualified(b"a:b"));
        assert!(!is_qualified(b""));
    }
}
