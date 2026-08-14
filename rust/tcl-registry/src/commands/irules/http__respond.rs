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

//! `HTTP::respond` iRules command.
use crate::prelude::*;

/// Bareword option tokens that follow the `<status>` positional
/// argument (`HTTP::respond 302 content|noserver|reset|version`).
const RESPOND_OPTION_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "content",
        detail: "Inline response body.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "noserver",
        detail: "Suppress Server header.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "reset",
        detail: "Reset server-side connection.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "version",
        detail: "Response HTTP version.",
        ..ArgValue::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::respond",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        // Option set: `-version`/`-content`/`-ifile`/`-noserver`/`-reset`.
        // (`-status` is the positional status arg, not an option.)
        options: const {
            &[
                OptionSpec {
                    name: "-version",
                    value: OptionValue::value("1.0 | 1.1"),
                    detail: "Protocol version on the synthesised response.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-content",
                    value: OptionValue::value("CONTENT"),
                    detail: "Response body content.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-ifile",
                    value: OptionValue::value("IFILE_OBJ"),
                    detail: "Serve the response body from an iFile object.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-noserver",
                    value: OptionValue::flag(),
                    detail: "Suppress the auto-injected `Server` response header.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-reset",
                    value: OptionValue::flag(),
                    detail: "Reset the connection after sending the response.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        hover: Some(HoverSnippet {
            summary: "Send an immediate HTTP response from an iRule.",
            synopsis: &["HTTP::respond <status> ?option value ...?"],
            snippet: "Common options include `content`, `noserver`, `reset`, and `version`.\n\nThe response is sent when the current event completes. You cannot alter it in later HTTP events or after another response has already been sent.\n\n**Security**: When the response body contains user-supplied data\n(HTTP headers, URI, payload), HTML-encode it to prevent XSS.\nFor blocking/maintenance pages, include `Connection close` and\n`Cache-Control no-store` headers:\n```tcl\nHTTP::respond 403 content $html Connection close Cache-Control no-store\n```",
            source: "https://clouddocs.f5.com/api/irules/HTTP__respond.html",
            examples: "",
            return_value: "",
        }),
        // Tainted data in the response body → XSS/content
        // injection (IRULE3001).
        taint_output_sink: Some("IRULE3001"),
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
                "LB_FAILED",
                "MR_EGRESS",
                "MR_FAILED",
                "NAME_RESOLVED",
            ],
            init_only: false,
            flow: false,
            capability: None,
        }),
        // Command-level arg-value completion: the bareword option
        // tokens that follow the `<status>` positional argument
        // (`HTTP::respond 302 content|noserver|reset|version`).
        arg_values: &[(1, RESPOND_OPTION_VALUES)],
        forms: &[FormSpec {
            synopsis: "HTTP::respond <status> ?option value ...?",
            ..FormSpec::DEFAULT
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ResponseCommit,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Client,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
