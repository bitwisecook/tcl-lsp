//! Shared Tcl value-shape helpers used across compiler passes.
//!
//! Ported from `core/compiler/value_shapes.py`.

/// Return `true` when `text` is a pure variable reference — either
/// `$name` or `${name}` — with no surrounding text, whitespace, or
/// extra Tcl syntax.
#[must_use]
pub fn is_pure_var_ref(text: &str) -> bool {
    if let Some(inner) = text.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return !inner.contains('}');
    }
    text.starts_with('$')
        && !text
            .bytes()
            .any(|b| matches!(b, b' ' | b'"' | b'{' | b'}' | b'[' | b']'))
}

/// Extract command name and args from `[cmd arg1 arg2 …]`.
///
/// Returns `None` when `text` is not bracket-wrapped, or when the
/// inside is empty. Arguments are whitespace-split — simple case
/// matching the Python helper; callers that need full Tcl list
/// quoting handle it upstream.
#[must_use]
pub fn parse_command_substitution(text: &str) -> Option<(String, Vec<String>)> {
    let stripped = text.trim();
    let inner = stripped.strip_prefix('[')?.strip_suffix(']')?.trim();
    let mut parts = inner.split_ascii_whitespace();
    let cmd = parts.next()?.to_owned();
    let args: Vec<String> = parts.map(str::to_owned).collect();
    Some((cmd, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_var_bare_and_braced() {
        assert!(is_pure_var_ref("$x"));
        assert!(is_pure_var_ref("${x}"));
        assert!(is_pure_var_ref("$foo::bar"));
    }

    #[test]
    fn pure_var_rejects_extras() {
        assert!(!is_pure_var_ref("$x extra"));
        assert!(!is_pure_var_ref("hello$x"));
        assert!(!is_pure_var_ref("$x[nested]"));
        assert!(!is_pure_var_ref("\"$x\""));
        assert!(!is_pure_var_ref("x"));
    }

    #[test]
    fn pure_var_rejects_unbalanced_braces() {
        assert!(!is_pure_var_ref("${x}y"));
    }

    #[test]
    fn parse_command_substitution_basic() {
        let (cmd, args) = parse_command_substitution("[llength $x]").unwrap();
        assert_eq!(cmd, "llength");
        assert_eq!(args, vec!["$x".to_string()]);
    }

    #[test]
    fn parse_command_substitution_with_whitespace() {
        let (cmd, args) = parse_command_substitution("  [ set x 42 ]  ").unwrap();
        assert_eq!(cmd, "set");
        assert_eq!(args, vec!["x".to_string(), "42".into()]);
    }

    #[test]
    fn parse_command_substitution_rejects_non_bracketed() {
        assert!(parse_command_substitution("llength $x").is_none());
        assert!(parse_command_substitution("[empty ").is_none());
        assert!(parse_command_substitution("[]").is_none());
    }

    #[test]
    fn parse_command_substitution_no_args() {
        let (cmd, args) = parse_command_substitution("[pwd]").unwrap();
        assert_eq!(cmd, "pwd");
        assert!(args.is_empty());
    }
}
