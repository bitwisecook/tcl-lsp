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

//! `HTTP::proxy` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "enable",
        arity: Arity::exact(0),
        detail: "Enable HTTP proxy.",
        synopsis: "HTTP::proxy enable",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "disable",
        arity: Arity::exact(0),
        detail: "Disable HTTP proxy.",
        synopsis: "HTTP::proxy disable",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "uri-rewrite",
        arity: Arity::exact(1),
        detail: "Control URI rewriting.",
        synopsis: "HTTP::proxy uri-rewrite ?enable|disable?",
        mutator: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "enable",
                    detail: "Enable URI rewriting.",
                    min_tcl: None,
                    code: None,
                },
                ArgValue {
                    value: "disable",
                    detail: "Disable URI rewriting.",
                    min_tcl: None,
                    code: None,
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "addr",
        arity: Arity::exact(0),
        detail: "Get proxy destination address.",
        synopsis: "HTTP::proxy addr",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "port",
        arity: Arity::exact(0),
        detail: "Get proxy destination port.",
        synopsis: "HTTP::proxy port",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rtdom",
        arity: Arity::exact(0),
        detail: "Get proxy route domain.",
        synopsis: "HTTP::proxy rtdom",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(0),
        detail: "Check if proxy is active.",
        synopsis: "HTTP::proxy exists",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iptuple",
        arity: Arity::exact(0),
        detail: "Get proxy IP tuple.",
        synopsis: "HTTP::proxy iptuple",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "chain",
        arity: Arity::at_least(0),
        detail: "Control proxy chaining.",
        synopsis: "HTTP::proxy chain ?args?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::proxy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Controls the application of HTTP proxy when using an Explicit HTTP profile.",
            synopsis: &[
                "HTTP::proxy",
                "HTTP::proxy ('enable' | 'disable')",
                "HTTP::proxy 'uri-rewrite' ('enable' | 'disable')",
                "HTTP::proxy ('addr' | 'port' | 'rtdom' | 'exists' | 'iptuple')",
                "HTTP::proxy chain ?args?",
            ],
            snippet: "When an Explicit HTTP profile is applied to a virtual server, HTTP::proxy allows control of whether the BIG-IP will handle the proxy of the connection locally or send it to a downstream pool for processing instead.\n\nThis functionality was introduced in v11.6, and is available for v11.5.1 via an Engineering Hotfix.\n\nHTTP::proxy allows inspection of the results of the DNS lookup used in the Explicit HTTP Proxy.\n\nWhen a HTTP Proxy Chaining profile is applied to a virtual server, HTTP::proxy chain may be used to control the CONNECT request used to connect to the next proxy in the chain.",
            source: "https://clouddocs.f5.com/api/irules/HTTP__proxy.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"[HTTP::method] [HTTP::uri]\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTTP::proxy ?subcommand? ?args?",
            dialects: None,
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::HttpUri,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
