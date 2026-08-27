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
//! Two `TclOO` members are structurally irregular and handled directly rather
//! than by fixed argument roles: `self …` (a nested member) and
//! `property … -get/-set …` (flag-keyed bodies).

use tcl_registry::definer::{DefinitionBodyGrammar, MemberKind, MemberRefKind, MemberSpec};
use tcl_registry::{ArgRole, CommandRegistry};

/// The definition-body grammar for `command`'s body when it is an *outer*
/// definer — a command carrying a [`DefinitionBodyGrammar`] (a `TclOO` metaclass
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

pub use tcl_compiler::realm::HeadWords;

/// The grammar the recursion into `head`'s body arguments should carry,
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
    head: HeadWords<'_>,
    args: &[&str],
    cur: Option<&'static DefinitionBodyGrammar>,
    registry: &CommandRegistry,
) -> Option<&'static DefinitionBodyGrammar> {
    let HeadWords { written, resolved } = head;
    let command = written;
    if let Some(g) = outer_definition_grammar(resolved, args, registry) {
        Some(g)
    } else if let Some(g) = cur.filter(|g| is_member(g, command)) {
        // A member command inside a definition body normally drops out of
        // definition context: a `method`/`constructor` body is ordinary Tcl.
        // Exception — the TclOO wrapper-*block* form `private { … }` /
        // `self { … }`, whose single argument is itself a nested definition
        // script; its members (`method`, `variable`, …) must stay resolvable,
        // so the block keeps `cur`. Detected from the registry (the member's
        // `wrapper_block_body` flag + the bare-block arg shape), never by name.
        if is_wrapper_block_form(g, command, args) {
            cur
        } else {
            None
        }
    } else {
        cur
    }
}

/// Whether `command`'s call `args` is the `TclOO` wrapper-*block* form
/// (`private { … }` / `self { … }`): a member declaring
/// [`MemberSpec::wrapper_block_body`] whose first argument is a bare
/// definition script rather than a nested inner member keyword.  In that form
/// the block is a nested definition body (its members stay in the enclosing
/// grammar); the `self method …` / `private method …` wrapper forms are *not*
/// block forms and their inner member's body drops out like any other.  Mirrors
/// the dispatch in [`wrapper_member_indices`] so the two stay in agreement.
#[must_use]
fn is_wrapper_block_form(grammar: &DefinitionBodyGrammar, command: &str, args: &[&str]) -> bool {
    let Some(member) = grammar.member(command) else {
        return false;
    };
    if !member.wrapper_block_body {
        return false;
    }
    match args.split_first() {
        Some((inner, _)) => grammar.member(inner).is_none(),
        None => false,
    }
}

/// Whether `command` is a member sub-keyword of `grammar`.  A thin pass-through
/// kept so consumers depend on `oo_body` rather than the registry type
/// directly; the structurally irregular `self` / `property` members are listed
/// in the grammar like any other and dispatched specially by the index helpers.
#[must_use]
pub fn is_member(grammar: &DefinitionBodyGrammar, command: &str) -> bool {
    grammar.is_member(command)
}

/// Body-argument indices (into `args`, excluding the command head) for a member
/// call under `grammar`.  Covers the flat members via the grammar's argument
/// roles plus the two irregular `TclOO` forms.  Empty when `command` is not a
/// member or has no body.
#[must_use]
pub fn member_body_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Vec<usize> {
    // The registry owns this irregular structural layout. Keeping this thin
    // forwarding function avoids a second Flat/Wrapper/FlagKeyed dispatcher.
    grammar.member_body_indices_in(command, args, tcl_dialect::DialectSet::all())
}

/// Profile-aware [`member_body_indices`].
#[must_use]
pub fn member_body_indices_in(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
    dialect: tcl_dialect::DialectSet,
) -> Vec<usize> {
    grammar.member_body_indices_in(command, args, dialect)
}

/// Parameter-list argument indices for a member call under `grammar` (a
/// `method`/`typemethod`/`constructor`'s `{a b}` word).  A wrapper member
/// (`self method …`, itcl `public method …`) nests.
#[must_use]
pub fn member_param_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Vec<usize> {
    member_role_indices(grammar, command, args, None, ArgRole::ParamList)
}

/// Profile-aware [`member_param_indices`].
#[must_use]
pub fn member_param_indices_in(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
    dialect: tcl_dialect::DialectSet,
) -> Vec<usize> {
    member_role_indices(grammar, command, args, Some(dialect), ArgRole::ParamList)
}

/// Name-argument indices for a member call under `grammar` — the declared name
/// of a `method foo {…} {…}` / `typemethod` / `property`.  A wrapper member
/// (`self method …`, itcl `public method …`) nests, so the name is found
/// through the inner member.
///
/// The grammar has always carried [`ArgRole::Name`]; the semantic-token walk
/// simply never consumed it, so a member's declared name fell through to the
/// default literal classification and painted as a plain `string` (#898 §2).
#[must_use]
pub fn member_name_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Vec<usize> {
    member_role_indices(grammar, command, args, None, ArgRole::Name)
}

/// Profile-aware [`member_name_indices`].
#[must_use]
pub fn member_name_indices_in(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
    dialect: tcl_dialect::DialectSet,
) -> Vec<usize> {
    member_role_indices(grammar, command, args, Some(dialect), ArgRole::Name)
}

/// The entity kind a member's arguments *reference*, plus their indices, when
/// the member is an unbounded reference list (`superclass A B` → classes,
/// `export m n` → methods).  `None` for every other member.
///
/// A wrapper member (`self mixin M`) nests, so the lookup follows the inner
/// member — the same way the role lookups do.
#[must_use]
pub fn member_ref_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Option<(MemberRefKind, Vec<usize>)> {
    let member = grammar.member(command)?;
    match member.kind {
        MemberKind::Flat => {
            let kind = member.all_args_ref?;
            Some((kind, (slot_value_start(member, args)..args.len()).collect()))
        }
        // `self mixin M` / itcl `public method …` — resolve through the inner
        // member and shift its indices past the wrapper word.
        MemberKind::Wrapper => {
            let inner = args.first()?;
            let (kind, idx) = member_ref_indices(grammar, inner, args.get(1..)?)?;
            Some((kind, idx.into_iter().map(|i| i + 1).collect()))
        }
        MemberKind::FlagKeyed => None,
    }
}

/// Declared-variable argument indices for a member call under `grammar` — the
/// names bound by `variable a b c` / `typevariable v` / `component c` /
/// `onconfigure -opt valueVar …` / itcl `public variable x`.
#[must_use]
pub fn member_var_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Vec<usize> {
    member_role_indices(grammar, command, args, None, ArgRole::VarWrite)
}

/// Profile-aware [`member_var_indices`].
#[must_use]
pub fn member_var_indices_in(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
    dialect: tcl_dialect::DialectSet,
) -> Vec<usize> {
    member_role_indices(grammar, command, args, Some(dialect), ArgRole::VarWrite)
}

/// Closed-vocabulary option arguments for a definition member. These are
/// supplied by the member grammar (for example Tcl 9's method visibility and
/// definition-namespace facet), never by an LSP-side `-word` parser.
#[must_use]
pub fn member_option_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Vec<usize> {
    member_role_indices(grammar, command, args, None, ArgRole::Option)
}

/// Profile-aware [`member_option_indices`].
#[must_use]
pub fn member_option_indices_in(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
    dialect: tcl_dialect::DialectSet,
) -> Vec<usize> {
    member_role_indices(grammar, command, args, Some(dialect), ArgRole::Option)
}

/// Namespace-name arguments for a definition member. Keeping this separate
/// from a command name means semantic tokens retain Tcl's namespace symbol
/// space for `definitionnamespace` and for any future registry-defined member.
#[must_use]
pub fn member_namespace_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
) -> Vec<usize> {
    member_role_indices(grammar, command, args, None, ArgRole::NamespaceName)
}

/// Profile-aware [`member_namespace_indices`].
#[must_use]
pub fn member_namespace_indices_in(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
    dialect: tcl_dialect::DialectSet,
) -> Vec<usize> {
    member_role_indices(
        grammar,
        command,
        args,
        Some(dialect),
        ArgRole::NamespaceName,
    )
}

/// The argument indices carrying `role` for a member call, dispatched on the
/// member's [`MemberKind`] — so the walker never hardcodes a member name:
/// `Flat` reads the member's own arg-roles (or every arg for `variable a b c`);
/// `Wrapper` recurses into the inner member nested at arg 0
/// (`self`/`public`/`protected`/`private`); `FlagKeyed` reads the `-get`/`-set`
/// flag-value bodies (`property`).
fn member_role_indices(
    grammar: &DefinitionBodyGrammar,
    command: &str,
    args: &[&str],
    dialect: Option<tcl_dialect::DialectSet>,
    role: ArgRole,
) -> Vec<usize> {
    let Some(member) = grammar.member(command) else {
        return Vec::new();
    };
    if dialect.is_some_and(|dialect| member.unavailable_option_for(args, dialect).is_some()) {
        return Vec::new();
    }
    match member.kind {
        MemberKind::Flat => flat_member_indices(member, args, dialect, role),
        MemberKind::Wrapper => wrapper_member_indices(grammar, member, args, dialect, role),
        MemberKind::FlagKeyed => match role {
            ArgRole::Body => collect_property_body_indices(args),
            // `property NAME… ?-get script? ?-set script?` — every leading bare
            // word before the first flag is a declared property name.  Without
            // this the name fell through to the default literal classifier and
            // painted as a plain string, unlike every other member's name.
            ArgRole::Name => args
                .iter()
                .take_while(|a| !a.starts_with('-'))
                .enumerate()
                .map(|(i, _)| i)
                .collect(),
            _ => Vec::new(),
        },
    }
}

/// The `role`-carrying argument indices for a flat member, given `args`
/// (0-based *after* the member keyword).  Handles the unbounded `variable a b
/// c` form (`all_args_var`) as well as the fixed `arg_roles` layout.
fn flat_member_indices(
    member: &MemberSpec,
    args: &[&str],
    dialect: Option<tcl_dialect::DialectSet>,
    role: ArgRole,
) -> Vec<usize> {
    if dialect.is_some_and(|dialect| member.unavailable_option_for(args, dialect).is_some()) {
        return Vec::new();
    }
    if role == ArgRole::VarWrite && member.all_args_var {
        (slot_value_start(member, args)..args.len()).collect()
    } else {
        let indices: Vec<usize> = dialect.map_or_else(
            || member.indices_for_call(args, role).collect(),
            |dialect| member.indices_for_call_in(args, dialect, role).collect(),
        );
        indices.into_iter().filter(|&i| i < args.len()).collect()
    }
}

/// The index of the first *value* argument of a slot member call — `1` when
/// the member is a slot ([`MemberSpec::slot`], issue #1169) and `args[0]` is
/// an explicit slot-operation word (`variable -set c`, `filter -append f`),
/// else `0`.  The operation word names no variable / method / class, so the
/// walker must not paint it as one; a `-word` that is *not* a recognised
/// operation stays classified as data, exactly as real Tcl treats it past
/// argument 0.
fn slot_value_start(member: &MemberSpec, args: &[&str]) -> usize {
    usize::from(
        member.slot.is_some()
            && args
                .first()
                .is_some_and(|a| tcl_registry::definer::SlotOp::parse(a).is_some()),
    )
}

/// A [`MemberKind::Wrapper`] member (`self method …`, itcl `public method …`)
/// nests an inner member keyword at `args[0]`; the inner member's own roles
/// (including its `variable a b c` unbounded form) apply shifted one place
/// right (past the wrapper word).  `args` is the wrapper call minus the wrapper
/// word itself (so `args[0]` is `method`/`constructor`/`variable`/…).
///
/// When the following word is *not* a recognised inner member and the wrapper
/// declares [`MemberSpec::wrapper_block_body`] (`TclOO`'s `private { … }` /
/// `self { … }`), the wrapper's own roles apply directly — `args[0]` is the
/// definition-script body — rather than resolving nothing.
fn wrapper_member_indices(
    grammar: &DefinitionBodyGrammar,
    member: &MemberSpec,
    args: &[&str],
    dialect: Option<tcl_dialect::DialectSet>,
    role: ArgRole,
) -> Vec<usize> {
    let Some((inner, rest)) = args.split_first() else {
        return Vec::new();
    };
    if let Some(m) = grammar.member(inner) {
        // Shift the inner member's indices one place right, past the wrapper.
        return flat_member_indices(m, rest, dialect, role)
            .into_iter()
            .map(|i| i + 1)
            .collect();
    }
    if member.wrapper_block_body {
        // Bare script-block form: the wrapper's own roles (a single body at
        // arg 0) apply unshifted.
        return flat_member_indices(member, args, dialect, role);
    }
    Vec::new()
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
        r.load_dialect(tcl_dialect::DialectSet::IRULES);
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
        assert_eq!(
            member_body_indices(g, "constructor", &["{}", "body"]),
            vec![1]
        );
        assert_eq!(member_body_indices(g, "destructor", &["body"]), vec![0]);
        assert_eq!(member_body_indices(g, "initialise", &["body"]), vec![0]);
        assert_eq!(member_body_indices(g, "private", &["body"]), vec![0]);
        assert_eq!(
            member_body_indices(g, "method", &["n", "{}", "body"]),
            vec![2]
        );
        assert_eq!(
            member_body_indices(g, "method", &["n", "-private", "{}", "body"]),
            vec![3]
        );
        assert_eq!(
            member_body_indices(g, "classmethod", &["n", "{a}", "body"]),
            vec![2]
        );
        assert_eq!(
            member_body_indices(g, "self", &["constructor", "{}", "body"]),
            vec![2]
        );
        assert_eq!(
            member_body_indices(g, "self", &["destructor", "body"]),
            vec![1]
        );
        assert_eq!(
            member_body_indices(g, "self", &["method", "n", "{}", "body"]),
            vec![3]
        );
        assert_eq!(
            member_body_indices(g, "property", &["name", "-set", "s", "-get", "z"]),
            vec![2, 4]
        );
    }

    /// `TclOO`'s `private` (and `self`) accept both the prefix-wrapper form
    /// (`private method m {} {…}`) and the bare script-block form
    /// (`private { … }`).  The wrapper must recurse into the inner member's
    /// roles for the former and treat arg 0 as the body for the latter
    /// (issue 157).
    #[test]
    fn tcloo_private_wrapper_and_block_forms() {
        let g = &TCLOO_GRAMMAR;
        // Prefix-wrapper form: body/params/name come from the inner `method`,
        // shifted past the `private` word — NOT arg 0 (`method`) as a script.
        assert_eq!(
            member_body_indices(g, "private", &["method", "m", "{a}", "body"]),
            vec![3]
        );
        assert_eq!(
            member_param_indices(g, "private", &["method", "m", "{a}", "body"]),
            vec![2]
        );
        // `private variable a b c` — the inner unbounded `variable` form: every
        // trailing word is a declared name (shifted past `private`).
        assert_eq!(
            member_var_indices(g, "private", &["variable", "a", "b", "c"]),
            vec![1, 2, 3]
        );
        // Bare script-block form: arg 0 is the definition-script body.
        assert_eq!(member_body_indices(g, "private", &["{body}"]), vec![0]);
        assert!(member_param_indices(g, "private", &["{body}"]).is_empty());
        // `self` shares the same dual shape.
        assert_eq!(
            member_body_indices(g, "self", &["method", "m", "{}", "body"]),
            vec![3]
        );
        assert_eq!(member_body_indices(g, "self", &["{body}"]), vec![0]);
    }

    #[test]
    fn tcloo_member_param_indices() {
        let g = &TCLOO_GRAMMAR;
        assert_eq!(
            member_param_indices(g, "method", &["n", "{a}", "b"]),
            vec![1]
        );
        assert_eq!(
            member_param_indices(g, "method", &["n", "-unexport", "{a}", "b"]),
            vec![2]
        );
        assert_eq!(
            member_param_indices(g, "constructor", &["{a}", "b"]),
            vec![0]
        );
        assert_eq!(
            member_param_indices(g, "self", &["method", "n", "{a}", "b"]),
            vec![2]
        );
        assert!(member_param_indices(g, "destructor", &["b"]).is_empty());
    }

    #[test]
    fn tcloo_definitionnamespace_roles_are_exposed_to_lsp_consumers() {
        let g = &TCLOO_GRAMMAR;
        assert_eq!(
            member_option_indices(g, "definitionnamespace", &["-instance", "::defs"]),
            vec![0]
        );
        assert_eq!(
            member_namespace_indices(g, "definitionnamespace", &["-instance", "::defs"]),
            vec![1]
        );
        assert_eq!(
            member_namespace_indices(g, "definitionnamespace", &["::defs"]),
            vec![0]
        );
    }

    #[test]
    fn tcloo_method_option_layout_is_profile_aware() {
        let g = &TCLOO_GRAMMAR;
        let args = ["m", "-private", "{x}", "body"];
        assert!(
            member_body_indices_in(g, "method", &args, tcl_dialect::DialectSet::TCL86,).is_empty()
        );
        assert_eq!(
            member_body_indices_in(g, "method", &args, tcl_dialect::DialectSet::TCL90,),
            vec![3]
        );
    }

    #[test]
    fn snit_member_shapes() {
        let g = &SNIT_GRAMMAR;
        assert_eq!(
            member_body_indices(g, "typemethod", &["n", "{a}", "body"]),
            vec![2]
        );
        assert_eq!(
            member_param_indices(g, "typemethod", &["n", "{a}", "b"]),
            vec![1]
        );
        assert_eq!(
            member_body_indices(g, "typeconstructor", &["body"]),
            vec![0]
        );
        assert_eq!(
            member_body_indices(g, "onconfigure", &["-o", "vv", "body"]),
            vec![2]
        );
        assert_eq!(
            member_var_indices(g, "onconfigure", &["-o", "vv", "body"]),
            vec![1]
        );
        assert_eq!(member_body_indices(g, "oncget", &["-o", "body"]), vec![1]);
        assert_eq!(member_var_indices(g, "typevariable", &["v"]), vec![0]);
        assert_eq!(member_var_indices(g, "component", &["c"]), vec![0]);
    }

    #[test]
    fn tcloo_variable_marks_every_name() {
        let g = &TCLOO_GRAMMAR;
        assert_eq!(
            member_var_indices(g, "variable", &["a", "b", "c"]),
            vec![0, 1, 2]
        );
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
        assert!(
            next_definition_grammar(
                HeadWords::plain("oo::class"),
                &["create", "C", "{b}"],
                None,
                &reg
            )
            .is_some()
        );
        assert!(
            next_definition_grammar(HeadWords::plain("snit::type"), &["C", "{b}"], None, &reg)
                .is_some()
        );
        // `oo::define` script form switches on; member form does not.
        assert!(
            next_definition_grammar(
                HeadWords::plain("oo::define"),
                &["C", "{script}"],
                None,
                &reg
            )
            .is_some()
        );
        assert!(
            next_definition_grammar(
                HeadWords::plain("oo::define"),
                &["C", "method", "m", "{}", "{b}"],
                None,
                &reg
            )
            .is_none()
        );
        // A method body inside a class body switches off.
        assert!(
            next_definition_grammar(HeadWords::plain("method"), &["m", "{}", "{b}"], tcloo, &reg)
                .is_none()
        );
        // Control flow inherits.
        assert!(
            next_definition_grammar(HeadWords::plain("if"), &["{c}", "{b}"], tcloo, &reg).is_some()
        );
        assert!(
            next_definition_grammar(HeadWords::plain("if"), &["{c}", "{b}"], None, &reg).is_none()
        );
        // Inner commands at top level (no enclosing grammar) don't fire.
        assert!(
            next_definition_grammar(HeadWords::plain("method"), &["m", "{}", "{b}"], None, &reg)
                .is_none()
        );
    }

    #[test]
    fn wrapper_block_body_stays_in_definition_grammar() {
        let reg = registry();
        let tcloo = Some(&TCLOO_GRAMMAR);
        // The bare-block wrapper form `private { … }` / `self { … }` is a nested
        // definition script — the block keeps the enclosing grammar so its
        // members (`method`, `variable`, …) are still recognised.
        assert!(
            next_definition_grammar(
                HeadWords::plain("private"),
                &["{ method m {} {} }"],
                tcloo,
                &reg
            )
            .is_some(),
            "private {{ … }} block keeps the class grammar",
        );
        assert!(
            next_definition_grammar(
                HeadWords::plain("self"),
                &["{ method m {} {} }"],
                tcloo,
                &reg
            )
            .is_some(),
            "self {{ … }} block keeps the class grammar",
        );
        // But the wrapper form that nests an inner member (`self method …`,
        // `private method …`) is an ordinary member body and drops out.
        assert!(
            next_definition_grammar(
                HeadWords::plain("self"),
                &["method", "m", "{}", "{b}"],
                tcloo,
                &reg
            )
            .is_none(),
            "self method … body is ordinary Tcl",
        );
        assert!(
            next_definition_grammar(
                HeadWords::plain("private"),
                &["method", "m", "{}", "{b}"],
                tcloo,
                &reg
            )
            .is_none(),
            "private method … body is ordinary Tcl",
        );
        // The block form only preserves grammar for a member that actually
        // declares a wrapper block body — a plain `method` never does.
        assert!(
            !is_wrapper_block_form(&TCLOO_GRAMMAR, "method", &["{b}"]),
            "method is not a wrapper-block member",
        );
    }
}
