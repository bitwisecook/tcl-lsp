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

//! `http_method` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "http_method",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the action of the HTTP request.",
            synopsis: &["http_method"],
            snippet: "Returns the action of the HTTP request. Common values are GET and\nPOST. This command is a BIG-IP version 4.X variable, provided for\nbackward-compatibility. You can use the equivalent 9.Xcommand\nHTTP::method instead.",
            source: "https://clouddocs.f5.com/api/irules/http_method.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "http_method",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpMethod,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        deprecated_replacement: Some("HTTP::method"),
        deprecated_replacement_drop_in: true,
        ..CommandSpec::DEFAULT
    }
}
