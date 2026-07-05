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

//! Context-sensitive definition-body helpers, shared by the recursive script
//! walkers ([`crate::folding`] and [`crate::semantic_tokens`]).
//!
//! ## The problem
//!
//! A class/type *definition body* —
//!
//! ```tcl
//! oo::class create Point {
//!     superclass Shape
//!     variable x y
//!     constructor {ax ay} { set x $ax; set y $ay }
//!     method move {dx dy} { incr x $dx; incr y $dy }
//! }
//! ```
//!
//! (and its `snit::type Name { … }` cousin) — is a *definition script*: its
//! top-level words (`superclass`, `constructor`, `method`, `variable`, …) are
//! **not** ordinary commands.  They have no [`tcl_registry::CommandSpec`], so a
//! plain registry lookup can't tell a walker that `method move {dx dy} { … }`'s
//! final word is a script body to recurse into.  Worse, the sub-keywords are
//! context-sensitive: a top-level user proc named `method` outside any class
//! body must **not** be treated as a member definition.
//!
//! ## The model
//!
//! The member grammar lives in the registry: a definer command carries a
//! [`DefinitionBodyGrammar`] on [`tcl_registry::CommandSpec::definition_body`]
//! (see [`tcl_registry::definer`]).  This module is the *generic consumer* — it
//! contains **no** command names.  A new definer (snit, a custom class system)
//! is added by writing a grammar in the registry, never by editing here.
//!
//! A recursive walker threads the *enclosing grammar* (`None` outside any
//! definition body):
//!
//! * Recursing into the body of an *outer* definer
//!   ([`outer_definition_grammar`]) switches to that definer's grammar.
//! * While inside one, a *member* command ([`DefinitionBodyGrammar::is_member`])
//!   contributes body / parameter / variable arguments; recursing into a
//!   member's body leaves definition context (member bodies hold ordinary Tcl).
//! * Every other command inherits the current grammar, so control-flow nesting
//!   (`if` / `foreach` / …) around a `method` keeps the class body's context.
//!
//! Two TclOO members are structurally irregular and handled directly rather
//! than by fixed argument roles: `self …` (a nested member) and
//! `property … -get/-set …` (flag-keyed bodies).

use tcl_registry::definer::DefinitionBodyGrammar;
use tcl_registry::{ArgRole, CommandRegistry};

/// The definition-body grammar for `command`'s body when it is an *outer*
/// definer — a command carrying a [`DefinitionBodyGrammar`] (a TclOO metaclass
/// `create`/`new` body, a snit `type`/`widget` body).  For
/// `oo::define`/`oo::objdefine`, only the bare *script* form
/// (`oo::define Target { script }`) is a definition body; the member forms
/// (`oo::define Target method m {} { body }`) carry ordinary method code and
/// return `None`.  `args` excludes the command head.
#[must_use]
pub fn outer_definition_grammar(
    command: &str,
    args: &[&str],
    registry: &CommandRegistry,
) -> Option<&'static DefinitionBodyGrammar> {
    let grammar = registry.get(command)?.definition_body?;
    if matches!(command, "oo::define" | "oo::objdefine") {
        // The script form resolves its definition body at argument index 1;
        // every member form resolves a (method) body at index ≥ 2.
        return registry
            .arg_indices_for_role(command, args, ArgRole::Body)
            .contains(&1)
            .then_some(grammar);
    }
    Some(grammar)
}

/// The grammar the recursion into `command`'s body arguments should carry,
/// given the enclosing grammar `cur` (`None` = not in a definition body).
/// `args` excludes the command head.
///
/// * An outer definer body switches to that definer's grammar.
/// * A member body (while inside a definition body) is plain Tcl → `None`.
/// * Everything else — including the *member* forms of `oo::define` — inherits
///   `cur`, so control-flow nesting inside a class body stays in context while
///   a member body drops out of it.
#[must_use]
pub fn next_definition_grammar(
    command: &str,
    args: &[&str],
    cur: Option<&'static DefinitionBodyGrammar>,
    registry: &CommandRegistry,
) -> Option<&'static DefinitionBodyGrammar> {
    if let Some(g) = outer_definition_grammar(command, args, registry) {
        Some(g)
    } else if cur.is_some_and(|g| is_member(g, command)) {
        None
    } else {
        cur
    }
}

/// Whether `command` is a member sub-keyword of `grammar`, including the two
/// irregular TclOO members (`self`, `property`) the grammar does not list.
#[must_use]
pub fn is_member(grammar: &DefinitionBodyGrammar, command: &str) -> bool {
    grammar.is_member(command) || matches!(command, "self" | "property")
}

/// Body-argument indices (into `args`, excluding the command head) for a member
/// call under `grammar`.  Covers the flat members via the grammar's argument
/// roles plus the two irregular TclOO forms.  Empty when `command` is not a
/// member or has no body.
#[must_use]
pub fn member_body_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Vec<usize> {
    match command {
        "self" => self_member_indices(args, ArgRole::Body),
        "property" => collect_property_body_indices(args),
        _ => grammar
            .member(command)
            .map(|m| {
                m.indices_for(ArgRole::Body)
                    .filter(|&i| i < args.len())
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Parameter-list argument indices for a member call under `grammar` (a
/// `method`/`typemethod`/`constructor`'s `{a b}` word).  `self method …` nests.
#[must_use]
pub fn member_param_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Vec<usize> {
    match command {
        "self" => self_member_indices(args, ArgRole::ParamList),
        "property" => Vec::new(),
        _ => grammar
            .member(command)
            .map(|m| {
                m.indices_for(ArgRole::ParamList)
                    .filter(|&i| i < args.len())
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// Declared-variable argument indices for a member call under `grammar` — the
/// names bound by `variable a b c` / `typevariable v` / `component c` /
/// `onconfigure -opt valueVar …`.
#[must_use]
pub fn member_var_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Vec<usize> {
    let Some(m) = grammar.member(command) else {
        return Vec::new();
    };
    if m.all_args_var {
        (0..args.len()).collect()
    } else {
        m.indices_for(ArgRole::VarWrite)
            .filter(|&i| i < args.len())
            .collect()
    }
}

/// `self constructor ARGS BODY` → BODY 2 / PARAMS 1; `self destructor BODY` →
/// BODY 1; `self method NAME ARGS BODY` → BODY last / PARAMS 2.  `role` selects
/// which index set to return.  `args` is the `self …` call minus the `self`
/// word's own head (so `args[0]` is `constructor`/`method`/…).
fn self_member_indices(args: &[&str], role: ArgRole) -> Vec<usize> {
    if args.is_empty() {
        return Vec::new();
    }
    let n = args.len();
    match (args[0], role) {
        ("constructor", ArgRole::Body) if n >= 3 => vec![2],
        ("constructor", ArgRole::ParamList) if n >= 3 => vec![1],
        ("destructor", ArgRole::Body) if n >= 2 => vec![1],
        ("method" | "classmethod", ArgRole::Body) if n >= 4 => vec![n - 1],
        ("method" | "classmethod", ArgRole::ParamList) if n >= 4 => vec![2],
        _ => Vec::new(),
    }
}

/// Collect the `-set BODY` / `-get BODY` flag-value indices of an inner
/// `property NAME ?-set BODY? ?-get BODY?` invocation.
fn collect_property_body_indices(args: &[&str]) -> Vec<usize> {
    let n = args.len();
    if n == 0 {
        return Vec::new();
    }
    args.iter()
        .enumerate()
        .take(n.saturating_sub(1))
        .filter_map(|(i, &a)| ((a == "-set" || a == "-get") && i + 1 < n).then_some(i + 1))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::definer::{SNIT_GRAMMAR, TCLOO_GRAMMAR};

    fn registry() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        r
    }

    #[test]
    fn metaclass_create_bodies_are_outer() {
        let reg = registry();
        for name in [
            "oo::class",
            "oo::configurable",
            "oo::abstract",
            "oo::singleton",
        ] {
            assert!(
                outer_definition_grammar(name, &["create", "C", "{body}"], &reg).is_some(),
                "{name} create body must be an outer definition body"
            );
        }
    }

    #[test]
    fn snit_definers_are_outer() {
        let reg = registry();
        for name in ["snit::type", "snit::widget", "snit::widgetadaptor"] {
            assert!(
                outer_definition_grammar(name, &["Name", "{body}"], &reg).is_some(),
                "{name} body must be an outer definition body"
            );
        }
    }

    #[test]
    fn oo_define_script_form_is_outer_but_member_form_is_not() {
        let reg = registry();
        for cmd in ["oo::define", "oo::objdefine"] {
            assert!(
                outer_definition_grammar(cmd, &["Target", "{script}"], &reg).is_some(),
                "{cmd} script form must be an outer definition body"
            );
            assert!(
                outer_definition_grammar(cmd, &["Target", "method", "m", "{}", "{body}"], &reg)
                    .is_none(),
                "{cmd} member form must not be an outer definition body"
            );
        }
    }

    #[test]
    fn ordinary_commands_are_not_outer() {
        let reg = registry();
        for name in ["proc", "set", "if", "namespace", "method"] {
            assert!(
                outer_definition_grammar(name, &["a", "b"], &reg).is_none(),
                "{name} must not be an outer definition command"
            );
        }
    }

    #[test]
    fn tcloo_member_body_indices_cover_every_shape() {
        let g = &TCLOO_GRAMMAR;
        assert_eq!(member_body_indices(g, "constructor", &["{}", "body"]), vec![1]);
        assert_eq!(member_body_indices(g, "destructor", &["body"]), vec![0]);
        assert_eq!(member_body_indices(g, "initialise", &["body"]), vec![0]);
        assert_eq!(member_body_indices(g, "private", &["body"]), vec![0]);
        assert_eq!(member_body_indices(g, "method", &["n", "{}", "body"]), vec![2]);
        assert_eq!(
            member_body_indices(g, "classmethod", &["n", "{a}", "body"]),
            vec![2]
        );
        assert_eq!(
            member_body_indices(g, "self", &["constructor", "{}", "body"]),
            vec![2]
        );
        assert_eq!(member_body_indices(g, "self", &["destructor", "body"]), vec![1]);
        assert_eq!(
            member_body_indices(g, "self", &["method", "n", "{}", "body"]),
            vec![3]
        );
        assert_eq!(
            member_body_indices(g, "property", &["name", "-set", "s", "-get", "z"]),
            vec![2, 4]
        );
    }

    #[test]
    fn tcloo_member_param_indices() {
        let g = &TCLOO_GRAMMAR;
        assert_eq!(member_param_indices(g, "method", &["n", "{a}", "b"]), vec![1]);
        assert_eq!(member_param_indices(g, "constructor", &["{a}", "b"]), vec![0]);
        assert_eq!(
            member_param_indices(g, "self", &["method", "n", "{a}", "b"]),
            vec![2]
        );
        assert!(member_param_indices(g, "destructor", &["b"]).is_empty());
    }

    #[test]
    fn snit_member_shapes() {
        let g = &SNIT_GRAMMAR;
        assert_eq!(
            member_body_indices(g, "typemethod", &["n", "{a}", "body"]),
            vec![2]
        );
        assert_eq!(member_param_indices(g, "typemethod", &["n", "{a}", "b"]), vec![1]);
        assert_eq!(member_body_indices(g, "typeconstructor", &["body"]), vec![0]);
        assert_eq!(member_body_indices(g, "onconfigure", &["-o", "vv", "body"]), vec![2]);
        assert_eq!(member_var_indices(g, "onconfigure", &["-o", "vv", "body"]), vec![1]);
        assert_eq!(member_body_indices(g, "oncget", &["-o", "body"]), vec![1]);
        assert_eq!(member_var_indices(g, "typevariable", &["v"]), vec![0]);
        assert_eq!(member_var_indices(g, "component", &["c"]), vec![0]);
    }

    #[test]
    fn tcloo_variable_marks_every_name() {
        let g = &TCLOO_GRAMMAR;
        assert_eq!(member_var_indices(g, "variable", &["a", "b", "c"]), vec![0, 1, 2]);
    }

    #[test]
    fn member_indices_reject_short_forms() {
        let g = &TCLOO_GRAMMAR;
        assert!(member_body_indices(g, "method", &["n"]).is_empty());
        assert!(member_body_indices(g, "destructor", &[]).is_empty());
        assert!(member_body_indices(g, "set", &["x", "1"]).is_empty());
    }

    #[test]
    fn context_transitions() {
        let reg = registry();
        let tcloo = Some(&TCLOO_GRAMMAR);
        // Entering an outer (metaclass create) body switches on.
        assert!(next_definition_grammar("oo::class", &["create", "C", "{b}"], None, &reg).is_some());
        assert!(next_definition_grammar("snit::type", &["C", "{b}"], None, &reg).is_some());
        // `oo::define` script form switches on; member form does not.
        assert!(next_definition_grammar("oo::define", &["C", "{script}"], None, &reg).is_some());
        assert!(
            next_definition_grammar("oo::define", &["C", "method", "m", "{}", "{b}"], None, &reg)
                .is_none()
        );
        // A method body inside a class body switches off.
        assert!(next_definition_grammar("method", &["m", "{}", "{b}"], tcloo, &reg).is_none());
        // Control flow inherits.
        assert!(next_definition_grammar("if", &["{c}", "{b}"], tcloo, &reg).is_some());
        assert!(next_definition_grammar("if", &["{c}", "{b}"], None, &reg).is_none());
        // Inner commands at top level (no enclosing grammar) don't fire.
        assert!(next_definition_grammar("method", &["m", "{}", "{b}"], None, &reg).is_none());
    }
}
