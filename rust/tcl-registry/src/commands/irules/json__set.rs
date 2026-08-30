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

//! `JSON::set` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::set",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets a JSON element (aka.",
            synopsis: &["JSON::set JSON_ELEMENT JSON_TYPE (JSON_VALUE)?"],
            snippet: "Sets the value (content) of a JSON element, replacing any existing value. The given value should be according to the given type, as described below:\n\nnull: Omit. JSON type null has no value.\nboolean: 0 (false) or 1 (true).\ninteger: A Tcl number representing an integer in the range -(2^63) through (2^63 - 1). Otherwise, use the literal type.\nliteral: A Tcl string not requiring JSON escape sequences.\nstring: A Tcl string without escape sequences (certain characters will be replaced by JSON escape sequences).\nobject: Omit. An empty object is created.\narray: Omit. An empty array is created.",
            source: "https://clouddocs.f5.com/api/irules/JSON__set.html",
            examples: "when JSON_REQUEST {\n    set rootval [JSON::root]\n    JSON::set $rootval string HelloWorld\n}",
            return_value: "Returns the JSON element whose value was set (same element as the first argument passed to the command).",
        }),
        forms: &[FormSpec {
            synopsis: "JSON::set JSON_ELEMENT JSON_TYPE (JSON_VALUE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
