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

//! `JSON::render` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "JSON::render",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a string containing a textual rendering of the JSON cache content.",
            synopsis: &["JSON::render (JSON_CACHE)?"],
            snippet: "If a JSON cache handle is omitted, renders any JSON cache that preexists in the context in which this is executed. This is the normal case when the command is executed in a the JSON_REQUEST or JSON_RESPONSE event.\nIf a JSON cache handle is provided, renders that JSON cache. This is useful when a JSON profile is not being used.\nNOTE: Rendering consumes the data in the cache, so after a render, no further value retrieval/modification/rendering may be done on this JSON cache instance.",
            source: "https://clouddocs.f5.com/api/irules/JSON__render.html",
            examples: "when MR_INGRESS {\n    set cache [JSON::create]\n    set rootval [JSON::root $cache]\n    JSON::set $rootval string HelloWorld\n    set rendered [JSON::render $cache]\n}",
            return_value: "Returns the string containing the rendered JSON content.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "JSON::render (JSON_CACHE)?",
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
