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

//! `itcl::configbody` command — define a public variable's config body.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "itcl::configbody class::option body",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "itcl::configbody",
        traits: Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Define the configuration body of a public [incr Tcl] variable.",
            synopsis: &["itcl::configbody class::option body"],
            snippet: "Defines the code run when a public variable is modified via `configure`.",
            source: "[incr Tcl]",
            examples: "itcl::configbody Toplevel::title {\n    wm title $win $title\n}",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_roles: &[(1, ArgRole::Body)],
        required_package: Some("Itcl"),
        ..CommandSpec::DEFAULT
    }
}
