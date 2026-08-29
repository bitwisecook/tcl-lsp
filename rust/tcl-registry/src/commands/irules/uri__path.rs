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

//! `URI::path` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::path",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the path portion of the given URI.",
            synopsis: &["URI::path URI_STRING (depth | START | (START END))?"],
            snippet: "Returns the path portion of the given URI.",
            source: "https://clouddocs.f5.com/api/irules/URI__path.html",
            examples: "when RULE_INIT {\n\n    # You can use URI::query against a static string and not in a client-triggered event!\n    log local0. \"\\[URI::query \\\"?param1=val1&param2=val2\\\" param1\\]: [URI::query \"?param1=val1&param2=val2\" param1]\"\n\n    # This doesn't work, as URI::query expects a query string to start with a question mark\n    log local0. \"\\[URI::query \\\"param1=val1&param2=val2\\\" param1\\]: [URI::query \"param1=val1&param2=val2\" param1]\"\n}",
            return_value: "Returns the path portion of the given URI.",
        }),
        forms: &[FormSpec {
            synopsis: "URI::path URI_STRING (depth | START | (START END))?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            connection_side: ConnectionSide::Global,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
