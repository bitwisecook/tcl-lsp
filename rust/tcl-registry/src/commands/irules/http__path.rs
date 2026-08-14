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

//! `HTTP::path` iRules command.
use crate::prelude::*;
use crate::taint::SetterConstraint;

/// The setter form of `HTTP::path` requires its value to start
/// with `/` (IRULE3101). Registry-driven replacement for the hardcoded
/// `SETTER_CONSTRAINTS` table in `tcl_compiler::taint`.
const SETTER_CONSTRAINTS: &[SetterConstraint] = &[SetterConstraint {
    arg_index: 0,
    required_prefix: "/",
    code: tcl_core_types::DiagCode::Irule3101,
    message: "HTTP::path value must start with '/'",
}];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::path",
        traits: Traits::PURE
            .union(Traits::CSE_CANDIDATE)
            .union(Traits::DIAGRAM_ACTION)
            .union(Traits::UNNORMALISED_HTTP_GETTER),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        options: const {
            &[OptionSpec {
                name: "-normalized",
                value: OptionValue::flag(),
                detail: "Return the canonicalised path (URL evasion patterns rejected).",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Returns or sets the path part of the HTTP request.",
            synopsis: &["HTTP::path (PATH_VALUE)?"],
            snippet: "Returns or sets the path part of the HTTP request. The path is defined\nas the path and filename in a request. It does not include the query string.\nFor the following URL: http://www.example.com:8080/main/index.jsp?user=test&login=check\n\nThe path is: /main/index.jsp\n\nNote that only ? is used as the separator for the query string.\nSo, for the following URL: http://www.example.com:8080/main/index.jsp;session_id=abcdefgh?user=test&login=check\n\nThe path is: /main/index.jsp;session_id=abcdefgh\n\n**Best practice**: Prefer `HTTP::path` over `HTTP::uri` when you\nonly need the path without the query string. Use `-normalized`\nfor consistent matching (prevents URL-encoding evasion).",
            source: "https://clouddocs.f5.com/api/irules/HTTP__path.html",
            examples: "when HTTP_REQUEST {\n  log local0. \"\\[HTTP::path\\] original: [HTTP::path]\"\n  HTTP::path [string tolower [HTTP::path]]\n}",
            return_value: "Returns the path part of the HTTP request.",
        }),
        setter_constraints: SETTER_CONSTRAINTS,
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
                synopsis: "HTTP::path ?-normalized?",
                ..FormSpec::DEFAULT
            },
            FormSpec {
                kind: FormKind::Setter,
                synopsis: "HTTP::path <PATH_VALUE>",
                ..FormSpec::DEFAULT
            },
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::PATH_PREFIXED)),
        ..CommandSpec::DEFAULT
    }
}
