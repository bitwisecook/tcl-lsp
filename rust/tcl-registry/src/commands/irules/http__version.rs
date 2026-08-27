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

//! `HTTP::version` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::version",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns or sets the HTTP version of the request or response.",
            synopsis: &[
                "HTTP::version ('0.9' | '1.0' | '1.1')?",
                "HTTP::version '-string' (ANY_CHARS)?",
            ],
            snippet: "Returns or sets the HTTP version of the request or response. This\ncommand replaces the BIG-IP 4.X variable http_version.\nIf needed, Connection and Host headers will automatically be added\nappropriately.\nHTTP::version will return the original version of the request or\nresponse, even if it has been changed.  Note that this will return\nthe \"effective\" version used, which may be different than the actual\nversion string in the request or response.  For example, invalid\nversion numbers may be parsed as 1.1 in order to increase\ninter-operability with common HTTP servers.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__version.html",
            examples: "when HTTP_RESPONSE {\n  HTTP::version \"1.1\"\n}",
            return_value: "Returns the HTTP version of the request or response",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "HTTP::version ('0.9' | '1.0' | '1.1')?\nHTTP::version -string ?value?",
            ..FormSpec::DEFAULT
        }],
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "0.9",
                    detail: "HTTP/0.9",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "1.0",
                    detail: "HTTP/1.0",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "1.1",
                    detail: "HTTP/1.1",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        closed_value_args: &[0],
        options: const {
            &[OptionSpec {
                name: "-string",
                value: OptionValue::value("version"),
                detail: "Get/set version as raw string.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpHeader,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
