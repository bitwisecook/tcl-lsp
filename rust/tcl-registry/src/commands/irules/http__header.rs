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

//! `HTTP::header` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "at",
        arity: Arity::exact(1),
        detail: "Get header name by index.",
        synopsis: "HTTP::header at <index>",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "count",
        arity: Arity::new(0, 1),
        detail: "Count headers.",
        synopsis: "HTTP::header count ?<name>?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Check if header exists.",
        synopsis: "HTTP::header exists <name>",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Insert header/value pair.",
        synopsis: "HTTP::header insert <name> <value>",
        mutator: true,
        credential_arg: Some(2),
        sensitive_headers: &[
            "authorization",
            "proxy-authorization",
            "x-api-key",
            "x-auth-token",
            "x-secret",
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert_modssl_fields",
        arity: Arity::new(2, 3),
        detail: "Insert mod_ssl-compatible client IP/port headers.",
        synopsis: "HTTP::header insert_modssl_fields <addr port | addr addr addr | port port port>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "is_keepalive",
        arity: Arity::exact(0),
        detail: "Check if connection is keep-alive.",
        synopsis: "HTTP::header is_keepalive",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "is_redirect",
        arity: Arity::exact(0),
        detail: "Check if response is a redirect.",
        synopsis: "HTTP::header is_redirect",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lws",
        arity: Arity::exact(0),
        detail: "Returns 1 if a header with linear white space was encountered.",
        synopsis: "HTTP::header lws",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "List all header names.",
        synopsis: "HTTP::header names",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::exact(1),
        detail: "Remove named header.",
        synopsis: "HTTP::header remove <name>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::new(1, 2),
        detail: "Replace header value.",
        synopsis: "HTTP::header replace <name> ?<string>?",
        mutator: true,
        credential_arg: Some(2),
        sensitive_headers: &[
            "authorization",
            "proxy-authorization",
            "x-api-key",
            "x-auth-token",
            "x-secret",
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sanitize",
        arity: Arity::at_least(1),
        detail: "Remove headers not in the allow list.",
        synopsis: "HTTP::header sanitize <name-list>",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "value",
        arity: Arity::exact(1),
        detail: "Get first header value.",
        synopsis: "HTTP::header value <name>",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "values",
        arity: Arity::exact(1),
        detail: "Get all values for header.",
        synopsis: "HTTP::header values <name>",
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::header",
        traits: Traits::PURE
            .union(Traits::CSE_CANDIDATE)
            .union(Traits::DIAGRAM_ACTION),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        options: const {
            &[OptionSpec {
                name: "-noupdate",
                value: OptionValue::flag(),
                detail: "Do not propagate the header mutation to subsequent BIG-IP filters.",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        hover: Some(HoverSnippet {
            summary: "Inspect or mutate HTTP headers in an iRule event.",
            synopsis: &["HTTP::header <subcommand> ?arg ...?"],
            snippet: "Use subcommands like `value`, `insert`, `replace`, and `remove`.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__header.html",
            examples: "",
            return_value: "",
        }),
        // `HTTP::header insert|replace` with tainted data →
        // header injection (IRULE3002).
        // (`credential_arg` + `sensitive_headers` now live on the
        // `insert` / `replace` SubCommand specs above.)
        taint_output_sink: Some("IRULE3002"),
        taint_output_sink_subcommands: &["insert", "replace"],
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &["MR_EGRESS", "MR_INGRESS", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP::header <subcommand> ?arg ...?",
            dialects: None,
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpHeader,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
