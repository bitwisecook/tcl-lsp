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

//! `json::list2json` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "json::list2json list",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::list2json",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Convert a Tcl list to a JSON array.",
            synopsis: &["json::list2json list"],
            snippet: "",
            source: "tcllib json package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("json"),
        required_package: Some("json"),
        ..CommandSpec::DEFAULT
    }
}
