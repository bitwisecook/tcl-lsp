// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Command-alias detection and resolution.
//!
//! Used by the lowerer to detect
//! `interp alias {} name {} target ?args?` definitions and resolve
//! command names through the alias table.
//!
//! Which commands mutate the command table is registry data
//! ([`tcl_registry::CommandTableEffect`], resolved via
//! `CommandRegistry::command_table_effect`), so the destructuring
//! helpers here take only the argument words: every caller has already
//! dispatched on the effect ([`CommandTableEffect::CreatesAliases`]
//! for the `interp alias` shapes, [`CommandTableEffect::RenamesCommands`]
//! for `rename`) rather than matching the command name itself.
//!
//! [`CommandTableEffect::CreatesAliases`]: tcl_registry::CommandTableEffect::CreatesAliases
//! [`CommandTableEffect::RenamesCommands`]: tcl_registry::CommandTableEffect::RenamesCommands

use std::collections::{HashMap, HashSet};

use crate::naming::{is_dynamic_word, normalise_qualified_name};

/// Alias store: qualified name → (target command, prepended args).
pub type CommandAliasMap = HashMap<String, (String, Vec<String>)>;

/// Detect `interp alias {} name {} target ?args?`.
///
/// `args` are the words after the `interp` head (`args[0]` is the
/// `alias` subcommand word the caller's
/// [`CommandTableEffect::CreatesAliases`] dispatch already resolved).
///
/// Returns `(qualified_alias_name, target_cmd, prepended_args)` or
/// `None` if this is not a current-interpreter alias definition.
/// Only aliases in the current interpreter (empty source and target
/// paths) are tracked.
///
/// [`CommandTableEffect::CreatesAliases`]: tcl_registry::CommandTableEffect::CreatesAliases
#[must_use]
pub fn detect_interp_alias(args: &[String]) -> Option<(String, String, Vec<String>)> {
    debug_assert_eq!(
        args.first().map(String::as_str),
        Some("alias"),
        "caller dispatches on CommandTableEffect::CreatesAliases"
    );
    if args.len() < 5 {
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
    // A dynamic target (`interp alias {} myEval {} $target`) cannot be
    // resolved to a static command name; recording it verbatim would map
    // `myEval` to the literal, never-registered string `"$target"`,
    // silently dropping arg-role/body/sink treatment for `myEval` calls
    // instead of just leaving them unaliased. A dynamic alias name
    // (`interp alias {} $name {} eval`) is safe to skip too — the
    // resulting key can never match a real call's literal command word.
    if is_dynamic_word(alias_name) || is_dynamic_word(target_cmd) {
        return None;
    }

    let qualified = if alias_name.is_empty() {
        alias_name.clone()
    } else {
        normalise_qualified_name(alias_name)
    };

    Some((qualified, target_cmd.clone(), prepended))
}

/// Detect the `interp alias srcPath srcCmd {}` **deletion** form —
/// present but empty `targetCmd` deletes a previously created
/// current-interpreter alias (confirmed against tclsh 9.0.4: a call
/// through the deleted name afterwards fails "invalid command name",
/// exactly like a deleted `rename`). Distinct from the two-argument
/// *query* form (`interp alias srcPath srcCmd`, target path absent
/// entirely), which reads back the current target and deletes nothing —
/// that shape fails this function's exact `args.len() == 4` guard and
/// [`detect_interp_alias`]'s `args.len() >= 5` guard alike, so neither
/// function fires for it.
///
/// `args` are the words after the `interp` head, exactly as for
/// [`detect_interp_alias`].
///
/// Returns the qualified alias name to delete, or `None` when this
/// isn't a current-interpreter deletion.
#[must_use]
pub fn detect_interp_alias_delete(args: &[String]) -> Option<String> {
    debug_assert_eq!(
        args.first().map(String::as_str),
        Some("alias"),
        "caller dispatches on CommandTableEffect::CreatesAliases"
    );
    if args.len() != 4 {
        return None;
    }
    let src_path = &args[1];
    let alias_name = &args[2];
    let target_path = &args[3];
    if !matches!(src_path.as_str(), "" | "{}") || !matches!(target_path.as_str(), "" | "{}") {
        return None;
    }
    if alias_name.is_empty() {
        return None;
    }
    Some(normalise_qualified_name(alias_name))
}

/// Detect a static `rename oldName newName` **move** — both names literal
/// text, `newName` non-empty (an empty `newName` *deletes* `oldName` rather
/// than renaming it, per Tcl semantics, and creates no new alias), and neither
/// dynamically substituted.
///
/// `args` are the words after the `rename` head (the caller has
/// dispatched on [`CommandTableEffect::RenamesCommands`]).
///
/// Returns `(old_name, new_name)` — **both raw**, unlike [`detect_interp_alias`]
/// which pre-qualifies its alias name at the global root. `rename` binds
/// `newName` in the *current* namespace (`rename set myset` inside
/// `namespace eval ::ns` creates `::ns::myset`, not global `::myset`), so the
/// caller qualifies `new` against the namespace the `rename` runs in;
/// qualifying here would wrongly root every renamed name globally. For every
/// purpose the alias map serves (arg-role / body-form resolution, taint sink
/// dispatch, canonical-command typing), a `rename` is then indistinguishable
/// from an `interp alias {} newName {} oldName` — calling through the new name
/// really does invoke the old command.
///
/// [`CommandTableEffect::RenamesCommands`]: tcl_registry::CommandTableEffect::RenamesCommands
#[must_use]
pub fn detect_rename(args: &[String]) -> Option<(String, String)> {
    if args.len() != 2 {
        return None;
    }
    let (old, new) = (&args[0], &args[1]);
    // `rename $old newName` / `rename oldName [x]` — a dynamic component means
    // the true target isn't known statically; recording it verbatim would map
    // `newName` to a literal, never-registered string like `"$old"`, silently
    // dropping arg-role/body/sink treatment for `newName` calls instead of just
    // leaving them unaliased. An empty `new` is a deletion, not a rename.
    if old.is_empty() || new.is_empty() || is_dynamic_word(old) || is_dynamic_word(new) {
        return None;
    }
    Some((old.clone(), new.clone()))
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
///
/// The literal `"expr"` target match stays name-keyed for now: the
/// consumers are the registry-less lowering fast paths
/// (`extract_single_expr_arg` behind `LoweringHookId::Return` / `Set`),
/// whose shared `dispatch_lowering_hook` signature deliberately carries
/// no registry.  Routing this through `EXPR_CONCATENATES_ARGS` means
/// threading a registry through that public dispatch table — tracked
/// migration debt rather than a semantic choice.
#[must_use]
pub fn expr_alias_names(aliases: &CommandAliasMap) -> HashSet<String> {
    let mut result = HashSet::new();
    for (name, (target, prepended)) in aliases {
        if target == "expr" && prepended.is_empty() {
            result.insert(name.clone());
            if let Some(short) = name.rsplit("::").next()
                && !short.is_empty()
                && name.starts_with("::")
            {
                result.insert(short.to_owned());
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
        let result = detect_interp_alias(&args);
        assert!(result.is_some());
        let (name, target, prepended) = result.unwrap();
        assert_eq!(name, "::myalias");
        assert_eq!(target, "puts");
        assert_eq!(prepended, vec!["-nonewline"]);
    }

    #[test]
    fn detect_interp_alias_rejects_dynamic_target() {
        // `interp alias {} myEval {} $target` — the true target isn't
        // known statically; must not map `myEval` to the literal string
        // `"$target"` (which would silently drop arg-role/sink treatment
        // for `myEval` calls instead of just leaving them unaliased).
        let args: Vec<String> = vec![
            "alias".into(),
            "{}".into(),
            "myEval".into(),
            "{}".into(),
            "$target".into(),
        ];
        assert!(detect_interp_alias(&args).is_none());
    }

    #[test]
    fn detect_interp_alias_rejects_dynamic_name() {
        let args: Vec<String> = vec![
            "alias".into(),
            "{}".into(),
            "$name".into(),
            "{}".into(),
            "eval".into(),
        ];
        assert!(detect_interp_alias(&args).is_none());
    }

    #[test]
    fn detect_rename_basic() {
        // Returns `(old, new)` raw — the caller namespace-qualifies `new`.
        let args: Vec<String> = vec!["eval".into(), "myEval".into()];
        let result = detect_rename(&args);
        assert_eq!(result, Some(("eval".to_string(), "myEval".to_string())));
    }

    #[test]
    fn detect_rename_rejects_dynamic_old_name() {
        // `rename $old eval` — the true source isn't known statically;
        // must not map `eval` to the literal string `"$old"` (which would
        // silently drop arg-role/sink treatment for genuine `eval` calls
        // later in the same script instead of just leaving them
        // unaliased).
        let args: Vec<String> = vec!["$old".into(), "eval".into()];
        assert!(detect_rename(&args).is_none());
    }

    #[test]
    fn detect_rename_rejects_dynamic_new_name() {
        let args: Vec<String> = vec!["eval".into(), "myEval[x]".into()];
        assert!(detect_rename(&args).is_none());
    }

    #[test]
    fn detect_rename_deletion_form_is_not_an_alias() {
        // `rename oldName {}` *deletes* oldName — not a rename.
        let args: Vec<String> = vec!["eval".into(), String::new()];
        assert!(detect_rename(&args).is_none());
    }

    #[test]
    fn detect_rename_wrong_arity() {
        assert!(detect_rename(&["eval".to_string()]).is_none());
        assert!(detect_rename(&[]).is_none());
    }

    #[test]
    fn detect_rename_qualified_new_name() {
        // A `::`-qualified NEW is returned verbatim; the caller
        // (`join_namespace`) leaves it rooted globally.
        let args: Vec<String> = vec!["eval".into(), "::ns::myEval".into()];
        let result = detect_rename(&args);
        assert_eq!(
            result,
            Some(("eval".to_string(), "::ns::myEval".to_string()))
        );
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
        assert!(detect_interp_alias(&args).is_none());
    }

    #[test]
    fn detect_delete_basic() {
        let args: Vec<String> = vec!["alias".into(), "{}".into(), "bar".into(), "{}".into()];
        assert_eq!(detect_interp_alias_delete(&args), Some("::bar".to_string()));
    }

    #[test]
    fn detect_delete_rejects_query_form() {
        // Only 2 args after `alias` (no target path at all) — a query,
        // not a deletion.
        let args: Vec<String> = vec!["alias".into(), "{}".into(), "bar".into()];
        assert!(detect_interp_alias_delete(&args).is_none());
    }

    #[test]
    fn detect_delete_rejects_creation_form() {
        // 5 args (a real target command present) — a creation, not a
        // deletion.
        let args: Vec<String> = vec![
            "alias".into(),
            "{}".into(),
            "bar".into(),
            "{}".into(),
            "foo".into(),
        ];
        assert!(detect_interp_alias_delete(&args).is_none());
    }

    #[test]
    fn detect_delete_rejects_foreign_interp() {
        let args: Vec<String> = vec!["alias".into(), "slave".into(), "bar".into(), "{}".into()];
        assert!(detect_interp_alias_delete(&args).is_none());
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
