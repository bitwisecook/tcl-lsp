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

//! `snit::typemethod` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "snit::typemethod type name arglist body",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::typemethod",
        dialects: None,
        arity: Arity::exact(4),
        // Snit typemethod bodies run in a dispatch context.
        arg_roles: &[(2, ArgRole::ParamList), (3, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Define a type method outside a type definition body.",
            synopsis: &["snit::typemethod type name arglist body"],
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
