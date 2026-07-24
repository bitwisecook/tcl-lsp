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

//! `base64::decode` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "base64::decode encodedData",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "base64::decode",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Decode a base64-encoded string back to binary data.",
            synopsis: &["base64::decode encodedData"],
            snippet: "",
            source: "tcllib base64 package",
            examples: "set binary [base64::decode $encodedString]",
            return_value: "The decoded binary data.",
        }),
        forms: FORMS,
        tcllib_package: Some("base64"),
        required_package: Some("base64"),
        ..CommandSpec::DEFAULT
    }
}
