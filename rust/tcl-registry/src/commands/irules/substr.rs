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

//! `substr` iRules command.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "substr",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a substring from a string.",
            synopsis: &["substr STRING SKIP_COUNT (TERMINATOR)?"],
            snippet: "A custom iRule function which returns a substring named <string>,\nbased on the values of the <skip_count> and <terminator> arguments.\nNote the following:\n  * The <skip_count> and <terminator> arguments are used in the same\n    way as they are for the findstr command.\n  * The <skip_count> argument is the index into <string> of the first\n    character to be returned, where 0 indicates the first character of\n    <string>.\n  * The <terminator> argument can be either the subtring length or the\n    substring terminating string.",
            source: "https://clouddocs.f5.com/api/irules/substr.html",
            examples: "when HTTP_REQUEST {\n  set uri [substr $uri 1 \"?\"]\n  log local0. \"Uri Part = $uri\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "substr STRING SKIP_COUNT (TERMINATOR)?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
