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

//! Variable-scoping command detection.
//!
//! Shared helpers for identifying which tokens in a `global` /
//! `variable` / `upvar` (or `namespace upvar`) command are
//! declarations. Both the LSP `textDocument/declaration` provider
//! and the compiler's memory-SSA alias detector parse these
//! commands; putting the logic here keeps the semantics in one
//! place.
//!
//! Each helper takes the raw argument texts and returns *indices
//! into the argument list* for the positions that name a declared
//! variable. Callers map those indices back to whatever
//! representation they care about — `String` names for memory-SSA,
//! source tokens for the LSP declaration provider.

use std::sync::OnceLock;

fn default_registry() -> &'static tcl_registry::CommandRegistry {
    static REGISTRY: OnceLock<tcl_registry::CommandRegistry> = OnceLock::new();
    REGISTRY.get_or_init(tcl_registry::CommandRegistry::build_default)
}

fn registry_role_indices(
    registry: &tcl_registry::CommandRegistry,
    command: &str,
    args: &[String],
) -> Vec<usize> {
    if command == "namespace upvar" {
        let mut lowered = Vec::with_capacity(args.len() + 1);
        lowered.push("upvar".to_owned());
        lowered.extend_from_slice(args);
        let refs: Vec<&str> = lowered.iter().map(String::as_str).collect();
        return registry
            .arg_indices_for_role("namespace", &refs, tcl_registry::prelude::ArgRole::VarWrite)
            .into_iter()
            .filter_map(|i| i.checked_sub(1))
            .collect();
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    registry.arg_indices_for_role(command, &refs, tcl_registry::prelude::ArgRole::VarWrite)
}

fn default_registry_role_indices(command: &str, args: &[String]) -> Vec<usize> {
    registry_role_indices(default_registry(), command, args)
}

/// Registry-backed compatibility helpers for declaration-index consumers.
#[must_use]
pub fn global_declaration_indices(args: &[String]) -> Vec<usize> {
    default_registry_role_indices("global", args)
        .into_iter()
        .filter(|&i| {
            args.get(i)
                .is_some_and(|a| !a.is_empty() && !a.starts_with('$'))
        })
        .collect()
}

#[must_use]
/// Return registry-declared `variable` name positions, excluding substitutions.
pub fn variable_declaration_indices(args: &[String]) -> Vec<usize> {
    default_registry_role_indices("variable", args)
        .into_iter()
        .filter(|&i| {
            args.get(i)
                .is_some_and(|a| !a.is_empty() && !a.starts_with('$'))
        })
        .collect()
}

#[must_use]
/// Return registry-declared `my variable` name positions, excluding substitutions.
pub fn my_variable_declaration_indices(args: &[String]) -> Vec<usize> {
    default_registry_role_indices("my", args)
        .into_iter()
        .filter(|&i| {
            args.get(i)
                .is_some_and(|a| !a.is_empty() && !a.starts_with('$'))
        })
        .collect()
}

#[must_use]
/// Return registry-declared `upvar` local positions whose source and local are static.
pub fn upvar_local_declaration_indices(command: &str, args: &[String]) -> Vec<usize> {
    default_registry_role_indices(command, args)
        .into_iter()
        .filter(|&i| {
            args.get(i).is_some_and(|local| !local.starts_with('$'))
                && i > 0
                && args
                    .get(i - 1)
                    .is_some_and(|source| !source.starts_with('$'))
        })
        .collect()
}

#[must_use]
/// Return registry-declared local alias positions, retaining dynamic sources.
pub fn upvar_local_alias_indices(command: &str, args: &[String]) -> Vec<usize> {
    default_registry_role_indices(command, args)
        .into_iter()
        .filter(|&i| args.get(i).is_some_and(|local| !local.starts_with('$')))
        .collect()
}

/// True when `command` (with `args`) creates a scope alias — `global`,
/// `variable`, `upvar`, the `namespace upvar` compound, or `TclOO`'s
/// `my variable`.
///
/// Derived from the registry's [`Traits::CREATES_SCOPE_ALIAS`] trait
/// (top-level commands) and the per-subcommand `creates_scope_alias`
/// flag (`namespace upvar`, `my variable`) — never a hardcoded name
/// list, so a new alias-creating command is registry data only.  A
/// `rename`d or `interp alias`ed spelling is out of static reach by
/// design; the command-binding lattice handles those separately.
#[must_use]
pub fn is_scope_alias_call(
    registry: &tcl_registry::CommandRegistry,
    command: &str,
    args: &[String],
) -> bool {
    let Some(spec) = registry.get(command) else {
        return false;
    };
    if spec
        .traits
        .contains(tcl_registry::prelude::Traits::CREATES_SCOPE_ALIAS)
    {
        return true;
    }
    args.first()
        .and_then(|sub| spec.resolve_subcommand(sub))
        .is_some_and(|sub| sub.creates_scope_alias)
}

/// Indices (into `args`) of the variable names a scope-alias command binds
/// **in the current scope**, or empty for any other command.
///
/// Recognition and layout are registry-driven ([`is_scope_alias_call`] and
/// `ArgRole::VarWrite`); this function only applies the static-name filter.
#[must_use]
pub fn scope_alias_local_indices(
    registry: &tcl_registry::CommandRegistry,
    command: &str,
    args: &[String],
) -> Vec<usize> {
    if !is_scope_alias_call(registry, command, args) {
        return Vec::new();
    }
    registry_role_indices(registry, command.trim_start_matches(':'), args)
        .into_iter()
        .filter(|&i| args.get(i).is_some_and(|a| !a.starts_with('$')))
        .collect()
}

/// Indices (into `args`) of the variable names a scope-alias command
/// *declares* for **navigation** consumers (the LSP go-to-declaration
/// provider), or empty for any other command. Layout and parity come from the
/// registry's repeated argument descriptors; this function only applies the
/// navigation-specific source-side filter.
///
/// The navigation twin of [`scope_alias_local_indices`]: recognition is the
/// same registry-driven [`is_scope_alias_call`], but the `upvar` /
/// `namespace upvar` forms use the stricter
/// [`upvar_local_declaration_indices`] pair filter — a `$`-substituted
/// *source* side means there is nothing to navigate to, while the
/// observability flavour still counts the local as a live alias.
#[must_use]
pub fn scope_alias_declaration_indices(
    registry: &tcl_registry::CommandRegistry,
    command: &str,
    args: &[String],
) -> Vec<usize> {
    if !is_scope_alias_call(registry, command, args) {
        return Vec::new();
    }
    let canonical = command.trim_start_matches(':');
    registry_role_indices(registry, canonical, args)
        .into_iter()
        .filter(|&i| {
            if args.get(i).is_none_or(|a| a.starts_with('$')) {
                return false;
            }
            match canonical {
                "upvar" => i > 0 && args.get(i - 1).is_some_and(|a| !a.starts_with('$')),
                "namespace" | "namespace upvar" => {
                    i > 0 && args.get(i - 1).is_some_and(|a| !a.starts_with('$'))
                }
                _ => true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    // -- global --

    #[test]
    fn global_decls_bare_names_only() {
        let args = v(&["foo", "bar", "$skip"]);
        assert_eq!(global_declaration_indices(&args), vec![0, 1]);
    }

    #[test]
    fn global_decls_empty_input() {
        assert!(global_declaration_indices(&[]).is_empty());
    }

    // -- variable --

    #[test]
    fn variable_decls_skip_values() {
        // `variable foo 42 bar 99 baz` → names at 0, 2, 4.
        let args = v(&["foo", "42", "bar", "99", "baz"]);
        assert_eq!(variable_declaration_indices(&args), vec![0, 2, 4]);
    }

    #[test]
    fn variable_decls_reject_substituted_names() {
        let args = v(&["$x", "42", "bar", "99"]);
        assert_eq!(variable_declaration_indices(&args), vec![2]);
    }

    // -- my variable --

    #[test]
    fn my_variable_decls_all_names_no_values() {
        // Unlike the top-level `variable` command, the object `variable`
        // method takes only names: `my variable x y z` declares x, y, z.
        let args = v(&["variable", "x", "y", "z"]);
        assert_eq!(my_variable_declaration_indices(&args), vec![1, 2, 3]);
    }

    #[test]
    fn my_variable_decls_skip_substituted_and_non_variable_subcommand() {
        let args = v(&["variable", "x", "$dyn", "y"]);
        assert_eq!(my_variable_declaration_indices(&args), vec![1, 3]);
        // A different `my` method (e.g. `my varname x`) is not a declaration.
        assert!(my_variable_declaration_indices(&v(&["varname", "x"])).is_empty());
        assert!(my_variable_declaration_indices(&[]).is_empty());
    }

    // -- upvar --

    #[test]
    fn upvar_without_level() {
        // `upvar caller local`
        let args = v(&["caller", "local"]);
        assert_eq!(upvar_local_declaration_indices("upvar", &args), vec![1]);
    }

    #[test]
    fn upvar_with_integer_level() {
        // `upvar 1 caller local`
        let args = v(&["1", "caller", "local"]);
        assert_eq!(upvar_local_declaration_indices("upvar", &args), vec![2]);
    }

    #[test]
    fn upvar_with_negative_integer_level() {
        // `upvar -1 caller local` — negative levels count from
        // the top of the stack.
        let args = v(&["-1", "caller", "local"]);
        assert_eq!(upvar_local_declaration_indices("upvar", &args), vec![2]);
    }

    #[test]
    fn upvar_with_hash_level() {
        // `upvar #0 caller local`
        let args = v(&["#0", "caller", "local"]);
        assert_eq!(upvar_local_declaration_indices("upvar", &args), vec![2]);
    }

    #[test]
    fn upvar_multi_pair() {
        // `upvar 1 a la b lb`
        let args = v(&["1", "a", "la", "b", "lb"]);
        assert_eq!(upvar_local_declaration_indices("upvar", &args), vec![2, 4]);
    }

    #[test]
    fn upvar_rejects_substituted_pair() {
        // `upvar 1 $cached local` — skip because caller looks
        // like a substitution.
        let args = v(&["1", "$cached", "local"]);
        assert!(upvar_local_declaration_indices("upvar", &args).is_empty());
    }

    #[test]
    fn namespace_upvar_lowered() {
        // Lowered form: segmenter produces
        // `namespace upvar ns src dst src2 dst2` as one command.
        let args = v(&["upvar", "::ns", "src", "dst", "src2", "dst2"]);
        assert_eq!(
            upvar_local_declaration_indices("namespace", &args),
            vec![3, 5]
        );
    }

    #[test]
    fn namespace_upvar_space_joined() {
        // Pre-composed command word "namespace upvar".
        let args = v(&["::ns", "src", "dst"]);
        assert_eq!(
            upvar_local_declaration_indices("namespace upvar", &args),
            vec![2]
        );
    }

    #[test]
    fn upvar_unrelated_command_empty() {
        let args = v(&["a", "b"]);
        assert!(upvar_local_declaration_indices("set", &args).is_empty());
    }

    #[test]
    fn upvar_insufficient_args_empty() {
        assert!(upvar_local_declaration_indices("upvar", &[]).is_empty());
        // Only level, no pairs.
        let args = v(&["1"]);
        assert!(upvar_local_declaration_indices("upvar", &args).is_empty());
    }

    // -- upvar alias flavour (observability consumers) --

    #[test]
    fn upvar_alias_keeps_local_with_dynamic_source_side() {
        // `upvar 0 $src local` — the declaration parser skips the pair
        // (nothing to navigate to), but `local` is a real alias in this
        // scope, so the observability flavour keeps it.
        let args = v(&["0", "$src", "local"]);
        assert!(upvar_local_declaration_indices("upvar", &args).is_empty());
        assert_eq!(upvar_local_alias_indices("upvar", &args), vec![2]);
    }

    #[test]
    fn upvar_alias_skips_substituted_local() {
        // A `$`-substituted *local* names no static variable — skipped by
        // both flavours.
        let args = v(&["0", "src", "$local"]);
        assert!(upvar_local_alias_indices("upvar", &args).is_empty());
    }

    #[test]
    fn upvar_alias_handles_namespace_forms_and_levels() {
        let ns = v(&["upvar", "::ns", "src", "dst", "src2", "dst2"]);
        assert_eq!(upvar_local_alias_indices("namespace", &ns), vec![3, 5]);
        let lv = v(&["#0", "caller", "local"]);
        assert_eq!(upvar_local_alias_indices("upvar", &lv), vec![2]);
    }

    // -- registry-driven scope-alias recognition --

    fn registry() -> tcl_registry::CommandRegistry {
        tcl_registry::CommandRegistry::build_default()
    }

    #[test]
    fn is_scope_alias_call_recognises_alias_creators() {
        let reg = registry();
        assert!(is_scope_alias_call(&reg, "global", &v(&["g"])));
        assert!(is_scope_alias_call(&reg, "variable", &v(&["n", "1"])));
        assert!(is_scope_alias_call(&reg, "upvar", &v(&["src", "dst"])));
        assert!(is_scope_alias_call(
            &reg,
            "namespace",
            &v(&["upvar", "::ns", "a", "b"])
        ));
        // TclOO's per-object `variable` analogue — the per-subcommand flag.
        assert!(is_scope_alias_call(&reg, "my", &v(&["variable", "x"])));
    }

    #[test]
    fn is_scope_alias_call_rejects_non_alias_commands() {
        let reg = registry();
        assert!(!is_scope_alias_call(&reg, "set", &v(&["x", "1"])));
        assert!(!is_scope_alias_call(
            &reg,
            "namespace",
            &v(&["eval", "::n", "body"])
        ));
        // `my` dispatching an ordinary method is not an alias.
        assert!(!is_scope_alias_call(&reg, "my", &v(&["speak"])));
        assert!(!is_scope_alias_call(&reg, "nosuchcmd", &v(&["x"])));
    }

    #[test]
    fn scope_alias_local_indices_covers_every_form() {
        let reg = registry();
        assert_eq!(
            scope_alias_local_indices(&reg, "global", &v(&["a", "b", "$dyn"])),
            vec![0, 1]
        );
        assert_eq!(
            scope_alias_local_indices(&reg, "variable", &v(&["n", "1", "m"])),
            vec![0, 2]
        );
        assert_eq!(
            scope_alias_local_indices(&reg, "upvar", &v(&["1", "src", "dst"])),
            vec![2]
        );
        assert_eq!(
            scope_alias_local_indices(&reg, "namespace", &v(&["upvar", "::ns", "s", "d"])),
            vec![3]
        );
        // `my variable x y` — resolved via the spec's own role resolver.
        assert_eq!(
            scope_alias_local_indices(&reg, "my", &v(&["variable", "x", "y"])),
            vec![1, 2]
        );
        assert!(scope_alias_local_indices(&reg, "puts", &v(&["x"])).is_empty());
    }

    #[test]
    fn upvar_uses_argument_count_not_level_text() {
        assert_eq!(
            upvar_local_declaration_indices("upvar", &v(&["$lvl", "a", "b"])),
            vec![2]
        );
        assert_eq!(
            upvar_local_declaration_indices("upvar", &v(&["1", "b"])),
            vec![1]
        );
    }
}
