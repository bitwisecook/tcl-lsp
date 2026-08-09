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

//! `HTTP::collect` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::collect",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        data_collection: Some(HTTP_COLLECT),
        hover: Some(HoverSnippet {
            summary: "Collects an amount of HTTP body data that you specify.",
            synopsis: &["HTTP::collect (CONTENT_LENGTH)?"],
            snippet: "Collects an amount of HTTP body data, optionally specified with\nthe <length> argument. When the system collects the specified\namount of data, it calls the Tcl event HTTP_REQUEST_DATA or\nHTTP_RESPONSE_DATA. The collected data can be accessed via the\nHTTP::payload command.\n\nNote that this command cannot be called after any Tcl command that\nsends an HTTP response (e.g. redirect, HTTP::redirect, and\nHTTP::respond). A run-time error will result.\n\nCare must be taken when using HTTP::collect to not stall the\nconnection.\n\n**Caution**: `HTTP::collect` cannot be called twice on the same\nconnection. A second call will fail or cause a TCL error. Use a\nstate variable (e.g. `set http_state collect`) to guard against\ndouble-collect across multiple iRules.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__collect.html",
            examples: "when HTTP_REQUEST_DATA {\n  # do stuff with the payload\n  set payload [HTTP::payload]\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[
                "AUTH_ERROR",
                "AUTH_FAILURE",
                "AUTH_RESULT",
                "AUTH_SUCCESS",
                "AUTH_WANTCREDENTIAL",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP::collect (CONTENT_LENGTH)?",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpBody,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
