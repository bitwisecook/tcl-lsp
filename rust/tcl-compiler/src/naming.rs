//! Variable and command name normalisation.
//!
//! Ports the relevant parts of `core/common/naming.py`. These live in
//! the compiler crate because they are consumed by the expression
//! parser and lowering — not by the lexer itself.

/// Normalise a Tcl variable reference to its base name.
///
/// Strips leading `$`, `${…}` delimiters, and array index `(…)`
/// suffixes:
///
/// ```
/// use tcl_compiler::naming::normalise_var_name;
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

/// Normalise a possibly-qualified Tcl command or procedure name.
///
/// Ensures the name starts with `::` and removes empty parts from
/// consecutive `::` separators.
///
/// ```
/// use tcl_compiler::naming::normalise_qualified_name;
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
    let parts: Vec<&str> = name.split("::").filter(|p| !p.is_empty()).collect();
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
/// use tcl_compiler::naming::split_array_name;
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
    if let Some(after) = name.strip_prefix("${") {
        if let Some(rel) = after.find('}') {
            let inner = &after[..rel];
            if inner.ends_with(')') {
                if let Some(idx) = inner.find('(') {
                    return (&inner[..idx], Some(&inner[idx + 1..inner.len() - 1]));
                }
            }
            return (inner, None);
        }
        // No closing brace — fall through (Python gates on `"}" in base`).
    }
    let base = name.strip_prefix('$').unwrap_or(name);
    if base.ends_with(')') {
        if let Some(idx) = base.find('(') {
            return (&base[..idx], Some(&base[idx + 1..base.len() - 1]));
        }
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
}
