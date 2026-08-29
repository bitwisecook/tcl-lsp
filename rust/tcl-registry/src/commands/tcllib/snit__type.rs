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

//! `snit::type` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "snit::type name definition",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::type",
        traits: Traits::CREATES_BARRIER
            | Traits::NEVER_INLINE_BODY
            | Traits::CREATES_DYNAMIC_BARRIER,
        surface: None,
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Define a new snit type (class).",
            synopsis: &["snit::type name definition"],
            snippet: "Defines a new object type. The definition body contains option, variable, constructor, destructor, method, and typemethod declarations.",
            source: "tcllib snit package",
            examples: "snit::type Dog {\n    option -name Fido\n    method bark {} { return \"Woof!\" }\n}",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_roles: &[(1, ArgRole::Body)],
        // A separate definition scope, not enclosing-scope data flow —
        // matches `oo::class`'s own `Structural` classification.
        body_kind: BodyKind::Structural,
        tcllib_package: Some("snit"),
        required_package: Some("snit"),
        definition_body: Some(&crate::definer::SNIT_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}
