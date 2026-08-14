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

//! `itcl::delete` command ([incr Tcl] runtime).
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "itcl::delete object|class|namespace name ?name ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "itcl::delete",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet {
            summary: "Delete [incr Tcl] objects, classes, or namespaces.",
            synopsis: &["itcl::delete object|class|namespace name ?name ...?"],
            snippet: "",
            source: "[incr Tcl]",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        required_package: Some("Itcl"),
        ..CommandSpec::DEFAULT
    }
}
