//! Command-alias detection and resolution.
//!
//! Ports `core/common/alias.py`. Used by the lowerer to detect
//! `interp alias {} name {} target ?args?` definitions and resolve
//! command names through the alias table.

use std::collections::{HashMap, HashSet};

use crate::naming::normalise_qualified_name;

/// Alias store: qualified name → (target command, prepended args).
pub type CommandAliasMap = HashMap<String, (String, Vec<String>)>;

/// Detect `interp alias {} name {} target ?args?`.
///
/// Returns `(qualified_alias_name, target_cmd, prepended_args)` or
/// `None` if this is not a current-interpreter alias definition.
/// Only aliases in the current interpreter (empty source and target
/// paths) are tracked.
#[must_use]
pub fn detect_interp_alias(
    cmd_name: &str,
    args: &[String],
) -> Option<(String, String, Vec<String>)> {
    if cmd_name != "interp" || args.len() < 5 {
        return None;
    }
    if args[0] != "alias" {
        return None;
    }
    let src_path = &args[1];
    let alias_name = &args[2];
    let target_path = &args[3];
    let target_cmd = &args[4];
    let prepended: Vec<String> = args[5..].to_vec();

    if !matches!(src_path.as_str(), "" | "{}") || !matches!(target_path.as_str(), "" | "{}") {
        return None;
    }

    let qualified = if alias_name.is_empty() {
        alias_name.clone()
    } else {
        normalise_qualified_name(alias_name)
    };

    Some((qualified, target_cmd.clone(), prepended))
}

/// Look up a command alias, namespace-aware.
///
/// Returns `(target_cmd, prepended_args)` or `None`.
/// If `cmd_name` starts with `::`, looks up directly.
/// Otherwise tries the current `namespace` first, then global.
#[must_use]
pub fn resolve_alias(
    cmd_name: &str,
    aliases: &CommandAliasMap,
    namespace: &str,
) -> Option<(String, Vec<String>)> {
    if cmd_name.starts_with("::") {
        return aliases.get(&normalise_qualified_name(cmd_name)).cloned();
    }

    if namespace != "::" {
        let candidate = normalise_qualified_name(&format!("{namespace}::{cmd_name}"));
        if let Some(entry) = aliases.get(&candidate) {
            return Some(entry.clone());
        }
    }

    aliases
        .get(&normalise_qualified_name(&format!("::{cmd_name}")))
        .cloned()
}

/// Return names that are aliases for `expr` (no prepended args).
///
/// Returns both the qualified keys (`::=`) and stripped short names
/// (`=`) so callers matching against bare command words get a hit.
#[must_use]
pub fn expr_alias_names(aliases: &CommandAliasMap) -> HashSet<String> {
    let mut result = HashSet::new();
    for (name, (target, prepended)) in aliases {
        if target == "expr" && prepended.is_empty() {
            result.insert(name.clone());
            if let Some(short) = name.rsplit("::").next() {
                if !short.is_empty() && name.starts_with("::") {
                    result.insert(short.to_owned());
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_basic_alias() {
        let args: Vec<String> = vec![
            "alias".into(),
            "{}".into(),
            "myalias".into(),
            "{}".into(),
            "puts".into(),
            "-nonewline".into(),
        ];
        let result = detect_interp_alias("interp", &args);
        assert!(result.is_some());
        let (name, target, prepended) = result.unwrap();
        assert_eq!(name, "::myalias");
        assert_eq!(target, "puts");
        assert_eq!(prepended, vec!["-nonewline"]);
    }

    #[test]
    fn detect_non_alias() {
        let args: Vec<String> = vec!["eval".into(), "{}".into(), "puts hello".into()];
        assert!(detect_interp_alias("interp", &args).is_none());
    }

    #[test]
    fn detect_wrong_command() {
        let args: Vec<String> = vec![
            "alias".into(),
            "{}".into(),
            "x".into(),
            "{}".into(),
            "y".into(),
        ];
        assert!(detect_interp_alias("puts", &args).is_none());
    }

    #[test]
    fn detect_foreign_interp() {
        let args: Vec<String> = vec![
            "alias".into(),
            "slave".into(),
            "x".into(),
            "{}".into(),
            "y".into(),
        ];
        assert!(detect_interp_alias("interp", &args).is_none());
    }

    #[test]
    fn resolve_global() {
        let mut aliases = CommandAliasMap::new();
        aliases.insert("::myalias".into(), ("puts".into(), vec![]));
        let result = resolve_alias("myalias", &aliases, "::");
        assert_eq!(result, Some(("puts".into(), vec![])));
    }

    #[test]
    fn resolve_qualified() {
        let mut aliases = CommandAliasMap::new();
        aliases.insert("::ns::cmd".into(), ("target".into(), vec!["a".into()]));
        let result = resolve_alias("::ns::cmd", &aliases, "::");
        assert_eq!(result, Some(("target".into(), vec!["a".into()])));
    }

    #[test]
    fn resolve_namespace_local() {
        let mut aliases = CommandAliasMap::new();
        aliases.insert("::ns::cmd".into(), ("target".into(), vec![]));
        // When in ::ns namespace, unqualified "cmd" should resolve.
        let result = resolve_alias("cmd", &aliases, "::ns");
        assert_eq!(result, Some(("target".into(), vec![])));
    }

    #[test]
    fn resolve_not_found() {
        let aliases = CommandAliasMap::new();
        assert!(resolve_alias("nope", &aliases, "::").is_none());
    }

    #[test]
    fn expr_aliases() {
        let mut aliases = CommandAliasMap::new();
        aliases.insert("::=".into(), ("expr".into(), vec![]));
        aliases.insert("::notexpr".into(), ("puts".into(), vec![]));
        aliases.insert("::exprwithargs".into(), ("expr".into(), vec!["1".into()]));
        let names = expr_alias_names(&aliases);
        assert!(names.contains("::="));
        assert!(names.contains("="));
        assert!(!names.contains("::notexpr"));
        assert!(!names.contains("::exprwithargs"));
    }
}
