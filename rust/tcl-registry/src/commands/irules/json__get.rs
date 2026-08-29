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

//! `JSON::get` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::get",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets the value content of a JSON element.",
            synopsis: &["JSON::get JSON_ELEMENT (JSON_TYPE)?"],
            snippet: "A JSON value can be one of many types. This command returns the value (content) of an element according to its type, as described below:\n\nnull : An empty Tcl list.\nboolean : 1 for true or 0 for false.\ninteger : A Tcl number representing an integer in the range -(2^63) through (2^63 - 1).\nliteral: A Tcl string not requiring JSON escape sequences.\nstring : A Tcl string without escape sequences (having been replaced by the characters they represent).\nobject : A JSON object handle.\narray : A JSON array handle.",
            source: "https://clouddocs.f5.com/api/irules/JSON__get.html",
            examples: "when JSON_REQUEST {\n    set rootval [JSON::root]\n    set content [JSON::get $rootval integer]\n    log local0. \"$content\"\n}",
            return_value: "Returns the content held within the JSON element, according to the types listed in the above description.",
        }),
        forms: &[FormSpec {
            synopsis: "JSON::get JSON_ELEMENT (JSON_TYPE)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
