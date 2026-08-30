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

//! `snit::method` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "snit::method type name arglist body",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::method",
        surface: None,
        arity: Arity::exact(4),
        // Snit method bodies run in a snit dispatch context,
        // not the caller's frame.  Body at index 3.
        arg_roles: &[(2, ArgRole::ParamList), (3, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        // `DEFERS_BODY`: a method *definition* stores the body against the
        // type; it runs on dispatch, not here. tclsh 8.6.16 / 9.0.4,
        // byte-identical: a type whose method body is `error stop` defines
        // cleanly and the next statement runs (issue #1672 audit).
        traits: Traits::DEFERS_BODY,
        hover: Some(HoverSnippet {
            summary: "Define an instance method outside a type definition body.",
            synopsis: &["snit::method type name arglist body"],
            snippet: "",
            source: "tcllib snit package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("snit"),
        required_package: Some("snit"),
        ..CommandSpec::DEFAULT
    }
}
