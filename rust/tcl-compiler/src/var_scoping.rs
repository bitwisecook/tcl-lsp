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

// global

/// Return indices of declared variables in `global var1 var2 ...`.
///
/// Every argument whose text does not start with `$` (i.e. is a
/// bare name, not a substituted reference) is a declaration.
#[must_use]
pub fn global_declaration_indices(args: &[String]) -> Vec<usize> {
    args.iter()
        .enumerate()
        .filter(|(_, a)| !a.is_empty() && !a.starts_with('$'))
        .map(|(i, _)| i)
        .collect()
}

// variable

/// Return indices of declared variables in
/// `variable name ?value? name ?value? ...`.
///
/// The `variable` command alternates (name, value?) pairs, so every
/// even-indexed arg is a name. A bare-name filter matches the
/// compiler's `!text.starts_with('$')` guard, skipping substituted
/// references.
#[must_use]
pub fn variable_declaration_indices(args: &[String]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if !args[i].is_empty() && !args[i].starts_with('$') {
            out.push(i);
        }
        i += 2;
    }
    out
}

// my variable (TclOO instance-variable binding)

/// Return indices of declared variables in the `TclOO` `my variable name
/// ?name …?` idiom, given the args of the `my` command (so `args[0]` is the
/// literal `"variable"` subcommand word).
///
/// Unlike the top-level `variable` *command* (which alternates name/value
/// pairs), the object `variable` *method* of `oo::object` takes plain names
/// only — every argument after the subcommand word is a declared instance
/// variable. Substituted (`$`-prefixed) names are skipped, matching the other
/// declaration helpers. Returns an empty vector when `args[0]` is not
/// `"variable"`.
#[must_use]
pub fn my_variable_declaration_indices(args: &[String]) -> Vec<usize> {
    if args.first().map(String::as_str) != Some("variable") {
        return Vec::new();
    }
    args.iter()
        .enumerate()
        .skip(1)
        .filter(|(_, a)| !a.is_empty() && !a.starts_with('$'))
        .map(|(i, _)| i)
        .collect()
}

// upvar / namespace upvar

/// Return indices of the *local-alias* tokens in an `upvar`
/// command.
///
/// Returns indices into the caller's own `args` sequence rather
/// than `(caller, local)` name pairs. Handles both `upvar` and
/// `namespace upvar` forms, including the lowered form where the
/// segmenter has spliced `namespace upvar` into a single command
/// whose first argument is literally `"upvar"`.
///
/// `command` is the command word as the caller saw it; `args` is
/// the command's remaining arguments *excluding* the command word
/// itself. Returns an empty vector for any unrelated command.
///
/// Level detection in the `upvar` form: a bare integer (optionally
/// prefixed with `-`) or `#<digits>` is treated as the level word.
/// Anything else makes the level default to 1 and the first
/// argument becomes the start of the `(otherVar, myVar)` pair list.
///
/// Pairs where either side starts with `$` (a substituted
/// reference) are skipped, matching the compiler's aliasing logic.
#[must_use]
pub fn upvar_local_declaration_indices(command: &str, args: &[String]) -> Vec<usize> {
    if args.is_empty() {
        return Vec::new();
    }

    let offset: usize = match command {
        "namespace" => {
            // Lowered `namespace upvar ns src dst ...` form.
            if args[0] != "upvar" {
                return Vec::new();
            }
            2 // skip 'upvar' subcommand + namespace argument
        }
        "namespace upvar" => 1, // skip namespace argument
        "upvar" => usize::from(looks_like_level(&args[0])),
        _ => return Vec::new(),
    };

    if offset >= args.len() {
        return Vec::new();
    }

    let mut out = Vec::new();
    // Walk pairs of (otherVar, myVar). Report the myVar index
    // (offset + i + 1) when neither side is a substituted reference.
    let mut i = offset;
    while i + 1 < args.len() {
        let caller_text = &args[i];
        let local_text = &args[i + 1];
        if !caller_text.starts_with('$') && !local_text.starts_with('$') {
            out.push(i + 1);
        }
        i += 2;
    }
    out
}

/// Return indices of the *local-alias* tokens in an `upvar` /
/// `namespace upvar` command for **observability** consumers (dead-store /
/// unused-variable suppression, shimmer alias abstention).
///
/// Unlike [`upvar_local_declaration_indices`] — whose pair filter serves
/// declaration *navigation*, where a `$`-substituted source side means
/// there is nothing to navigate to — the local name is a real alias in
/// this scope even when the *other* side is a substituted reference
/// (`upvar 0 $src local` links `local` all the same), so only
/// `$`-substituted locals are skipped here.
#[must_use]
pub fn upvar_local_alias_indices(command: &str, args: &[String]) -> Vec<usize> {
    if args.is_empty() {
        return Vec::new();
    }

    let offset: usize = match command {
        "namespace" => {
            if args[0] != "upvar" {
                return Vec::new();
            }
            2
        }
        "namespace upvar" => 1,
        "upvar" => usize::from(looks_like_level(&args[0])),
        _ => return Vec::new(),
    };

    if offset >= args.len() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut i = offset;
    while i + 1 < args.len() {
        if !args[i + 1].starts_with('$') {
            out.push(i + 1);
        }
        i += 2;
    }
    out
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
/// Recognition is registry-driven ([`is_scope_alias_call`]); the per-form
/// grammar comes from this module's shared parsers — the single home for
/// `global` / `variable` / `upvar` / `namespace upvar` argument layouts.
/// A subcommand-shaped alias whose spec carries its own role resolver
/// (`my variable`) resolves through the registry's `ArgRole::VarWrite`
/// query instead, so its layout stays pure registry data.
#[must_use]
pub fn scope_alias_local_indices(
    registry: &tcl_registry::CommandRegistry,
    command: &str,
    args: &[String],
) -> Vec<usize> {
    if !is_scope_alias_call(registry, command, args) {
        return Vec::new();
    }
    let canonical = command.trim_start_matches(':');
    match canonical {
        "global" => global_declaration_indices(args),
        "variable" => variable_declaration_indices(args),
        "upvar" | "namespace" | "namespace upvar" => upvar_local_alias_indices(canonical, args),
        _ => {
            // Subcommand-shaped alias (`my variable`): the spec's own
            // role resolver locates the bound names.
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            registry
                .arg_indices_for_role(command, &arg_strs, tcl_registry::prelude::ArgRole::VarWrite)
                .into_iter()
                .filter(|&i| args.get(i).is_some_and(|a| !a.starts_with('$')))
                .collect()
        }
    }
}

/// Indices (into `args`) of the variable names a scope-alias command
/// *declares* for **navigation** consumers (the LSP go-to-declaration
/// provider), or empty for any other command.
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
    match canonical {
        "global" => global_declaration_indices(args),
        "variable" => variable_declaration_indices(args),
        "upvar" | "namespace" | "namespace upvar" => {
            upvar_local_declaration_indices(canonical, args)
        }
        _ => {
            // Subcommand-shaped alias (`my variable`): the spec's own
            // role resolver locates the bound names.
            let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
            registry
                .arg_indices_for_role(command, &arg_strs, tcl_registry::prelude::ArgRole::VarWrite)
                .into_iter()
                .filter(|&i| args.get(i).is_some_and(|a| !a.starts_with('$')))
                .collect()
        }
    }
}

/// True when `head` looks like a Tcl upvar-level word:
/// - A decimal integer (optionally prefixed with `-`).
/// - `#<digits>` (absolute frame level).
///
/// The bare `#` form (no digit tail) is rejected. Reused by
/// `var_escape::handlers` so every call site shares one definition.
pub(crate) fn looks_like_level(head: &str) -> bool {
    if head.is_empty() {
        return false;
    }
    if let Some(digits) = head.strip_prefix('#') {
        return !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
    }
    let tail = head.strip_prefix('-').unwrap_or(head);
    !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit())
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
    fn looks_like_level_covers_forms() {
        assert!(looks_like_level("0"));
        assert!(looks_like_level("42"));
        assert!(looks_like_level("-1"));
        assert!(looks_like_level("#0"));
        assert!(looks_like_level("#42"));
        assert!(!looks_like_level(""));
        assert!(!looks_like_level("#"));
        assert!(!looks_like_level("-"));
        assert!(!looks_like_level("abc"));
        assert!(!looks_like_level("1x"));
    }
}
