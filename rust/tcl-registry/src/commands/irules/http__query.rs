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

//! `HTTP::query` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::query",
        traits: Traits::PURE
            .union(Traits::CSE_CANDIDATE)
            .union(Traits::UNNORMALISED_HTTP_GETTER),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::new(0, 1),
        options: const {
            &[OptionSpec {
                name: "-normalized",
                value: OptionValue::flag(),
                detail: "Return the canonicalised query (URL evasion patterns rejected).",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Returns or sets the query part of the HTTP request.",
            synopsis: &["HTTP::query (QUERY_STRING)?"],
            snippet: "Returns or sets the query part of the HTTP request. The query is defined as the\npart of the request past a ? character, if any.\nFor the following URL:\nhttp://www.example.com:8080/main/index.jsp?user=test&login=check\nThe query is:\nuser=test&login=check",
            source: "https://clouddocs.f5.com/api/irules/HTTP__query.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"http_path [HTTP::path]\"\n  log local0. \"http_query [HTTP::query]\"\n  HTTP::query user=test_user&login=test_login\n}",
            return_value: "Returns the query part of the HTTP request.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec {
                kind: FormKind::Getter,
                synopsis: "HTTP::query ?-normalized?",
                dialects: None,
            },
            FormSpec {
                kind: FormKind::Setter,
                synopsis: "HTTP::query <QUERY_STRING>",
                dialects: None,
            },
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
