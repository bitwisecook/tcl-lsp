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

//! Definition-body grammars for class/type *definer* commands.
//!
//! A *definer* command's script argument (`oo::class create Name { … }`,
//! `snit::type Name { … }`, the bare `oo::define Target { … }` script form) is
//! a **definition body**: its top-level words are member sub-keywords
//! (`method`, `typemethod`, `constructor`, `variable`, …) rather than ordinary
//! commands.  Those keywords are context-sensitive — they only mean anything
//! inside a definition body, and have no standalone [`CommandSpec`] — so their
//! argument layout is described here, as registry *data*, and consumed
//! generically by the shared definition-body walker
//! ([`tcl_lsp_core::oo_body`], used by folding + semantic tokens).
//!
//! The point is that adding (or completing) a definer — snit, xotcl, a custom
//! class system — is a matter of writing a [`DefinitionBodyGrammar`] and hanging
//! it off the definer's [`CommandSpec::definition_body`], **never** editing the
//! compiler / analyser / LSP with command-specific `match cmd_name` logic.
//!
//! [`CommandSpec`]: crate::CommandSpec
//! [`CommandSpec::definition_body`]: crate::CommandSpec::definition_body

use crate::arg_role::ArgRole;

/// How a member's argument layout is determined — most members are `Flat`
/// (their `arg_roles` give the layout directly), but two irregular shapes recur
/// across class systems and are described structurally so the walker never
/// hardcodes a member name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    /// Ordinary member — `arg_roles` give the argument layout directly
    /// (`method NAME PARAMS BODY`, `variable v`, …).
    Flat,
    /// A prefix wrapper around an inner member keyword at argument 0: `TclOO`'s
    /// `self method …` and itcl's access modifiers `public`/`protected`/
    /// `private method …`.  The inner member's own roles apply shifted one
    /// place right (past the wrapper word).
    Wrapper,
    /// Flag-keyed bodies rather than positional ones: `TclOO`'s
    /// `property NAME ?-get BODY? ?-set BODY?`.
    FlagKeyed,
}

/// One member sub-keyword of a definition body, with the argument roles a
/// walker should apply to its call.  `arg_roles` indices are 0-based *after*
/// the member keyword itself (`method NAME PARAMS BODY` →
/// `[(0, Name), (1, ParamList), (2, Body)]`).
#[derive(Debug, Clone, Copy)]
pub struct MemberSpec {
    /// The member keyword (`method`, `typemethod`, `constructor`, …).
    pub keyword: &'static str,
    /// Argument roles within the member call, 0-based after the keyword.
    pub arg_roles: &'static [(u8, ArgRole)],
    /// When set, *every* argument is a declared variable name (the unbounded
    /// `variable a b c` form).  Overrides `arg_roles` for name collection.
    pub all_args_var: bool,
    /// The structural shape of the member's arguments (see [`MemberKind`]).
    pub kind: MemberKind,
}

impl MemberSpec {
    /// An ordinary [`MemberKind::Flat`] member.
    #[must_use]
    const fn flat(keyword: &'static str, arg_roles: &'static [(u8, ArgRole)]) -> Self {
        Self { keyword, arg_roles, all_args_var: false, kind: MemberKind::Flat }
    }

    /// A `variable a b c`-style member: every argument is a declared name.
    #[must_use]
    const fn all_vars(keyword: &'static str) -> Self {
        Self { keyword, arg_roles: NO_ROLES, all_args_var: true, kind: MemberKind::Flat }
    }

    /// A name-reference / keyword-only member carrying nothing to recurse or
    /// declare (`superclass A B`, `inherit Base`, `option …`).
    #[must_use]
    const fn keyword_only(keyword: &'static str) -> Self {
        Self { keyword, arg_roles: NO_ROLES, all_args_var: false, kind: MemberKind::Flat }
    }

    /// A [`MemberKind::Wrapper`] member (`self`, `public`, `protected`,
    /// `private`) — an inner member keyword follows at argument 0.
    #[must_use]
    const fn wrapper(keyword: &'static str) -> Self {
        Self { keyword, arg_roles: NO_ROLES, all_args_var: false, kind: MemberKind::Wrapper }
    }

    /// A [`MemberKind::FlagKeyed`] member (`property`).
    #[must_use]
    const fn flag_keyed(keyword: &'static str) -> Self {
        Self { keyword, arg_roles: NO_ROLES, all_args_var: false, kind: MemberKind::FlagKeyed }
    }

    /// The argument indices (0-based after the keyword) carrying `role`.
    pub fn indices_for(&self, role: ArgRole) -> impl Iterator<Item = usize> + '_ {
        self.arg_roles
            .iter()
            .filter(move |(_, r)| *r == role)
            .map(|(i, _)| *i as usize)
    }
}

/// Which class-system a definer belongs to.  Distinguishes definers that share
/// the `definition_body` marker but need a different analyser body-parser /
/// instance-creation shape (`TclOO`'s `metaclass create Name { … }` vs snit's
/// `snit::type Name { … }`).  Consumers that only walk members (folding,
/// semantic tokens) never read this; the analyser dispatches on it instead of
/// hardcoding definer names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinerFamily {
    /// `TclOO` metaclasses and the `oo::define` / `oo::objdefine` script form.
    TclOo,
    /// snit `type` / `widget` / `widgetadaptor`.
    Snit,
    /// [incr Tcl] `itcl::class` (and the bare `class` alias).
    Itcl,
}

/// The grammar of a definer command's definition body: its recognised member
/// sub-keywords plus the variables implicitly in scope inside every member
/// body.
#[derive(Debug, Clone, Copy)]
pub struct DefinitionBodyGrammar {
    /// The class-system this definer belongs to.
    pub family: DefinerFamily,
    /// Recognised member sub-keywords.
    pub members: &'static [MemberSpec],
    /// Variables implicitly available in every member body (snit's `self` /
    /// `type` / `selfns` / `options`, a widget's `win` / `hull`).  Consumed by
    /// the analyser's read-before-set / stray-dispatch suppression.
    pub implicit_vars: &'static [&'static str],
}

impl DefinitionBodyGrammar {
    /// The member grammar for `keyword`, if it is a recognised member.
    #[must_use]
    pub fn member(&self, keyword: &str) -> Option<&'static MemberSpec> {
        // `members` is `&'static`, so the borrow can be handed back as static.
        let idx = self.members.iter().position(|m| m.keyword == keyword)?;
        Some(&self.members[idx])
    }

    /// Whether `keyword` is a recognised member sub-keyword.
    #[must_use]
    pub fn is_member(&self, keyword: &str) -> bool {
        self.members.iter().any(|m| m.keyword == keyword)
    }
}

// ---------------------------------------------------------------------------
// TclOO — `oo::class` / `oo::configurable` / `oo::abstract` / `oo::singleton`
// `create` bodies and the bare `oo::define` / `oo::objdefine` script form.
//
// The irregular `self …` (nested member) and `property … -get/-set …`
// (flag-keyed bodies) forms are handled by the walker directly; every flat
// member is described here.
// ---------------------------------------------------------------------------

/// `method NAME PARAMS BODY` — shared by `TclOO` and snit.
const METHOD_ROLES: &[(u8, ArgRole)] =
    &[(0, ArgRole::Name), (1, ArgRole::ParamList), (2, ArgRole::Body)];
/// `constructor PARAMS BODY`.
const CTOR_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::ParamList), (1, ArgRole::Body)];
/// A single trailing body (`destructor BODY`, `typeconstructor BODY`, …).
const BODY0_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Body)];
/// A single declared variable name (`typevariable v`, `component c`).
const VAR0_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite)];
/// A member keyword that carries no recursable body / parameter list /
/// variable declaration — only a class/method name reference the walker leaves
/// to the default classifier (`superclass A B`, `mixin M`, `export foo`, …).
const NO_ROLES: &[(u8, ArgRole)] = &[];

const TCLOO_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES),
    MemberSpec::flat("classmethod", METHOD_ROLES),
    MemberSpec::flat("constructor", CTOR_ROLES),
    MemberSpec::flat("destructor", BODY0_ROLES),
    MemberSpec::flat("initialise", BODY0_ROLES),
    MemberSpec::flat("initialize", BODY0_ROLES),
    MemberSpec::flat("private", BODY0_ROLES),
    // `variable a b c` inside a class body declares every name.
    MemberSpec::all_vars("variable"),
    // Name-reference-only members — recognised (so they read as keywords and a
    // same-named proc is not) but carry nothing to recurse or declare.
    MemberSpec::keyword_only("superclass"),
    MemberSpec::keyword_only("mixin"),
    MemberSpec::keyword_only("filter"),
    MemberSpec::keyword_only("export"),
    MemberSpec::keyword_only("unexport"),
    MemberSpec::keyword_only("forward"),
    MemberSpec::keyword_only("renamemethod"),
    MemberSpec::keyword_only("deletemethod"),
    MemberSpec::keyword_only("definitionnamespace"),
    // Structurally irregular — a nested-member wrapper (`self method …`) and a
    // flag-keyed body form (`property … -get/-set …`); their body indices come
    // from the walker's `MemberKind`-driven handling, not a hardcoded name.
    MemberSpec::wrapper("self"),
    MemberSpec::flag_keyed("property"),
];

/// The definition-body grammar for every `TclOO` metaclass and the bare
/// `oo::define` / `oo::objdefine` script form.
pub const TCLOO_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::TclOo,
    members: TCLOO_MEMBERS,
    implicit_vars: &[],
};

// ---------------------------------------------------------------------------
// snit — `snit::type` / `snit::widget` / `snit::widgetadaptor` bodies.
// ---------------------------------------------------------------------------

/// `onconfigure -option valueVar BODY` (snit 1.x) — the value var + body.
const ONCONFIGURE_ROLES: &[(u8, ArgRole)] = &[(1, ArgRole::VarWrite), (2, ArgRole::Body)];
/// `oncget -option BODY` (snit 1.x) — the body.
const ONCGET_ROLES: &[(u8, ArgRole)] = &[(1, ArgRole::Body)];

const SNIT_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES),
    MemberSpec::flat("typemethod", METHOD_ROLES),
    // A type-private `proc NAME ARGS BODY` — same shape as a method.
    MemberSpec::flat("proc", METHOD_ROLES),
    MemberSpec::flat("constructor", CTOR_ROLES),
    MemberSpec::flat("destructor", BODY0_ROLES),
    MemberSpec::flat("typeconstructor", BODY0_ROLES),
    MemberSpec::flat("onconfigure", ONCONFIGURE_ROLES),
    MemberSpec::flat("oncget", ONCGET_ROLES),
    MemberSpec::flat("variable", VAR0_ROLES),
    MemberSpec::flat("typevariable", VAR0_ROLES),
    MemberSpec::flat("component", VAR0_ROLES),
    MemberSpec::flat("typecomponent", VAR0_ROLES),
    // Name-reference / option-declaration members — recognised keywords with
    // nothing to recurse or declare.
    MemberSpec::keyword_only("option"),
    MemberSpec::keyword_only("delegate"),
    MemberSpec::keyword_only("expose"),
];

/// The definition-body grammar for snit `type` / `widget` / `widgetadaptor`.
/// `implicit_vars` is the set snit injects into *every* member body; a
/// widget's extra `win` / `hull` are added by the analyser only for the widget
/// definers (they are not implicit in a plain `snit::type`), so they are not
/// listed in this shared grammar.
pub const SNIT_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Snit,
    members: SNIT_MEMBERS,
    implicit_vars: &["self", "selfns", "type", "options"],
};

// ---------------------------------------------------------------------------
// [incr Tcl] — `itcl::class Name { … }` (and the bare `class` alias) bodies.
//
// The access modifiers `public` / `protected` / `private` are prefix wrappers
// (`public method foo {args} {body}`, `private variable x`) — `MemberKind::
// Wrapper`, handled generically like TclOO's `self`.  `inherit` lists base
// classes (multiple inheritance).  `variable` declares an instance variable
// (optionally with an init value + config body); `common` a class/static one.
// ---------------------------------------------------------------------------

/// itcl `variable NAME ?init? ?configbody?` — the declared name plus the
/// optional trailing config body (the script run when the public variable is
/// modified via `configure`); the init value between them is left to the
/// default classifier.
const ITCL_VAR_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite), (2, ArgRole::Body)];
/// itcl `common NAME ?init?` — a class/static variable; the declared name only
/// (no config body).
const ITCL_COMMON_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite)];

const ITCL_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES),
    // A class-scoped `proc NAME ARGS BODY` — same shape as a method.
    MemberSpec::flat("proc", METHOD_ROLES),
    MemberSpec::flat("constructor", CTOR_ROLES),
    MemberSpec::flat("destructor", BODY0_ROLES),
    MemberSpec::flat("variable", ITCL_VAR_ROLES),
    MemberSpec::flat("common", ITCL_COMMON_ROLES),
    // Base-class list (multiple inheritance) — name references only.
    MemberSpec::keyword_only("inherit"),
    // Access modifiers: prefix wrappers around an inner member keyword.
    MemberSpec::wrapper("public"),
    MemberSpec::wrapper("protected"),
    MemberSpec::wrapper("private"),
];

/// The definition-body grammar for [incr Tcl] `itcl::class` / bare `class`.
/// Member bodies run in the object's context with the instance/common
/// variables and `this` in scope.
pub const ITCL_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Itcl,
    members: ITCL_MEMBERS,
    implicit_vars: &["this"],
};
