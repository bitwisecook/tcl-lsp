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

//! `JSON::type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the type of the given JSON element (aka.",
            synopsis: &["JSON::type JSON_ELEMENT"],
            snippet: "A JSON value can be one of many types. This command allows you to determine the type of value in the element.",
            source: "https://clouddocs.f5.com/api/irules/JSON__type.html",
            examples: "when JSON_REQUEST {\n    set rootval [JSON::root]\n    JSON::set $rootval string HelloWorld\n    set type [JSON::type $rootval]\n}",
            return_value: "Returns a string representing the JSON type ('null' | 'boolean' | 'integer' | 'literal' | 'string' | 'object' | 'array' | 'invalid').",
        }),
        forms: &[FormSpec {
            synopsis: "JSON::type JSON_ELEMENT",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
