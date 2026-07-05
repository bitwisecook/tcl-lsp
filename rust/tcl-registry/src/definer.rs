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
}

impl MemberSpec {
    /// The argument indices (0-based after the keyword) carrying `role`.
    #[must_use]
    pub fn indices_for(&self, role: ArgRole) -> impl Iterator<Item = usize> + '_ {
        self.arg_roles
            .iter()
            .filter(move |(_, r)| *r == role)
            .map(|(i, _)| *i as usize)
    }
}

/// The grammar of a definer command's definition body: its recognised member
/// sub-keywords plus the variables implicitly in scope inside every member
/// body.
#[derive(Debug, Clone, Copy)]
pub struct DefinitionBodyGrammar {
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

/// `method NAME PARAMS BODY` — shared by TclOO and snit.
const METHOD_ROLES: &[(u8, ArgRole)] =
    &[(0, ArgRole::Name), (1, ArgRole::ParamList), (2, ArgRole::Body)];
/// `constructor PARAMS BODY`.
const CTOR_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::ParamList), (1, ArgRole::Body)];
/// A single trailing body (`destructor BODY`, `typeconstructor BODY`, …).
const BODY0_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Body)];
/// A single declared variable name (`typevariable v`, `component c`).
const VAR0_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite)];

const TCLOO_MEMBERS: &[MemberSpec] = &[
    MemberSpec { keyword: "method", arg_roles: METHOD_ROLES, all_args_var: false },
    MemberSpec { keyword: "classmethod", arg_roles: METHOD_ROLES, all_args_var: false },
    MemberSpec { keyword: "constructor", arg_roles: CTOR_ROLES, all_args_var: false },
    MemberSpec { keyword: "destructor", arg_roles: BODY0_ROLES, all_args_var: false },
    MemberSpec { keyword: "initialise", arg_roles: BODY0_ROLES, all_args_var: false },
    MemberSpec { keyword: "initialize", arg_roles: BODY0_ROLES, all_args_var: false },
    MemberSpec { keyword: "private", arg_roles: BODY0_ROLES, all_args_var: false },
    // `variable a b c` inside a class body declares every name.
    MemberSpec { keyword: "variable", arg_roles: &[], all_args_var: true },
];

/// The definition-body grammar for every TclOO metaclass and the bare
/// `oo::define` / `oo::objdefine` script form.
pub const TCLOO_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
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
    MemberSpec { keyword: "method", arg_roles: METHOD_ROLES, all_args_var: false },
    MemberSpec { keyword: "typemethod", arg_roles: METHOD_ROLES, all_args_var: false },
    MemberSpec { keyword: "constructor", arg_roles: CTOR_ROLES, all_args_var: false },
    MemberSpec { keyword: "destructor", arg_roles: BODY0_ROLES, all_args_var: false },
    MemberSpec { keyword: "typeconstructor", arg_roles: BODY0_ROLES, all_args_var: false },
    MemberSpec { keyword: "onconfigure", arg_roles: ONCONFIGURE_ROLES, all_args_var: false },
    MemberSpec { keyword: "oncget", arg_roles: ONCGET_ROLES, all_args_var: false },
    MemberSpec { keyword: "variable", arg_roles: VAR0_ROLES, all_args_var: false },
    MemberSpec { keyword: "typevariable", arg_roles: VAR0_ROLES, all_args_var: false },
    MemberSpec { keyword: "component", arg_roles: VAR0_ROLES, all_args_var: false },
    MemberSpec { keyword: "typecomponent", arg_roles: VAR0_ROLES, all_args_var: false },
];

/// The definition-body grammar for snit `type` / `widget` / `widgetadaptor`.
/// The implicit variables mirror what snit injects into member bodies; a
/// widget's extra `win` / `hull` are included and simply unreferenced by a
/// plain type.
pub const SNIT_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    members: SNIT_MEMBERS,
    implicit_vars: &["self", "selfns", "type", "options", "win", "hull"],
};
