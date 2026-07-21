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

//! `URI::query` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::query",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the query string portion of the given URI or the value of a query string parameter.",
            synopsis: &["URI::query URI_STRING (PARAMETER_NAME)?"],
            snippet: "Returns the query string portion of the given URI or the value of a\nquery string parameter.",
            source: "https://clouddocs.f5.com/api/irules/URI__query.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"Query string of URI [HTTP::uri] is [URI::query [HTTP::uri]]\"\n}",
            return_value: "Returns the query string portion of the given URI or the value of a query string parameter.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "URI::query URI_STRING (PARAMETER_NAME)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
