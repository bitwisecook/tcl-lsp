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

//! `JSON::parse` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::parse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Parses JSON content into a JSON cache that can be manipulated using further JSON:: commands.",
            synopsis: &["JSON::parse (JSON_STRING (JSON_MAX_ENTRIES)? )?"],
            snippet: "If a string is omitted, returns any JSON cache that preexists in the context in which this is executed. This is the normal case when the command is executed in the JSON_REQUEST or JSON_RESPONSE event.\nIf a string is provided, it is assumed to contain JSON and is parsed into a new JSON cache. This will be deleted when it is no longer referenced by a Tcl variable. This is useful when a JSON profile is not being used.",
            source: "https://clouddocs.f5.com/api/irules/JSON__parse.html",
            examples: "when JSON_REQUEST {\n    JSON::render\n}",
            return_value: "Returns a JSON cache instance handle to use for retrieving and overwriting content, and rendering.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "JSON::parse (JSON_STRING (JSON_MAX_ENTRIES)? )?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
