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

//! `uri::join` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "uri::join ?key value ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::join",
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Construct a URI from component parts.",
            synopsis: &["uri::join ?key value ...?"],
            snippet: "",
            source: "tcllib uri package",
            examples: "set url [uri::join scheme https host example.com path /index.html]",
            return_value: "A URI string.",
        }),
        forms: FORMS,
        tcllib_package: Some("uri"),
        required_package: Some("uri"),
        ..CommandSpec::DEFAULT
    }
}
