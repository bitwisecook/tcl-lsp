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

//! `itcl::body` command — define a method/proc body outside the class.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "itcl::body class::method args body",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "itcl::body",
        traits: Traits::NEVER_INLINE_BODY,
        surface: Some(SpecSurface::ALL_TCL),
        arity: Arity::exact(3),
        hover: Some(HoverSnippet {
            summary: "Define the body of a previously-declared [incr Tcl] method or proc.",
            synopsis: &["itcl::body class::method args body"],
            snippet: "Defines (or redefines) the body of a method or proc declared in a class definition. `args` must match the original argument list.",
            source: "[incr Tcl]",
            examples: "itcl::body Stack::pop {} {\n    ...\n}",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_roles: &[(1, ArgRole::ParamList), (2, ArgRole::Body)],
        required_package: Some("Itcl"),
        ..CommandSpec::DEFAULT
    }
}
