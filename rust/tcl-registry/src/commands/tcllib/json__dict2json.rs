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

//! `json::dict2json` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "json::dict2json dictValue",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::dict2json",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Convert a Tcl dict to a JSON string.",
            synopsis: &["json::dict2json dictValue"],
            snippet: "Converts a Tcl dictionary to a JSON-encoded string.",
            source: "tcllib json package",
            examples: "set json [json::dict2json [dict create name \"test\" value 42]]",
            return_value: "A JSON-encoded string.",
        }),
        forms: FORMS,
        tcllib_package: Some("json"),
        required_package: Some("json"),
        ..CommandSpec::DEFAULT
    }
}
