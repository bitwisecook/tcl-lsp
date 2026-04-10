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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_dollar() {
        assert_eq!(normalise_var_name("$foo"), "foo");
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
}
