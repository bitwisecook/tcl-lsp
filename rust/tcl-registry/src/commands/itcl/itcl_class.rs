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

//! `itcl::class` command ([incr Tcl] class definition).
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "itcl::class name { definition }",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "itcl::class",
        traits: Traits::CREATES_BARRIER
            | Traits::NEVER_INLINE_BODY
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Define a new [incr Tcl] class.",
            synopsis: &["itcl::class name { definition }"],
            snippet: "Defines an [incr Tcl] class. The definition body contains inherit, constructor, destructor, method, proc, variable, and common declarations, optionally prefixed by the public / protected / private access modifiers.",
            source: "[incr Tcl]",
            examples: "itcl::class Stack {\n    variable contents {}\n    method push {value} { lappend contents $value }\n    method pop {} { ... }\n}",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_roles: &[(1, ArgRole::Body)],
        // A separate definition scope, not enclosing-scope data flow —
        // matches `oo::class`'s own `Structural` classification.
        body_kind: BodyKind::Structural,
        required_package: Some("Itcl"),
        definition_body: Some(&crate::definer::ITCL_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}
