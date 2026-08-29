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

//! `SSL::forward_proxy` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "policy",
        arity: Arity::new(0, 1),
        detail: "Set or get forward proxy bypass policy.",
        synopsis: "SSL::forward_proxy policy ?bypass | intercept?",
        mutator: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "bypass",
                    detail: "Bypass SSL forward proxy.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "intercept",
                    detail: "Intercept SSL forward proxy.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cert",
        arity: Arity::new(0, 2),
        detail: "Retrieve the forged certificate or set cert options.",
        synopsis: "SSL::forward_proxy cert ?response_control? ?status?",
        pure: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "response_control",
                    detail: "Control response to cert errors.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "status",
                    detail: "Set server certificate status.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "verified_handshake",
        arity: Arity::new(0, 1),
        detail: "Enable, disable, or get verified handshake semantics.",
        synopsis: "SSL::forward_proxy verified_handshake ?enable | disable?",
        mutator: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "enable",
                    detail: "Enable verified handshake.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "disable",
                    detail: "Disable verified handshake.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "extension",
        arity: Arity::exact(2),
        detail: "Insert a certificate extension while forging.",
        synopsis: "SSL::forward_proxy extension <oid> <value>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::forward_proxy",
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves the forged certificate, or enables/disables/gets verified_handshake semantics, or mask/ignore certificate response_control for the SSL handshake or inserts a certificate extension to the certificate, or sets server certificate status.",
            synopsis: &[
                "SSL::forward_proxy ( (policy (bypass | intercept)?) | cert)",
                "SSL::forward_proxy verified_handshake (enable | disable) ?",
                "SSL::forward_proxy cert response_control (ignore | mask) ?",
                "SSL::forward_proxy extension (ARG ARG)",
            ],
            snippet: "This command sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves the forged certificate if the policy or cert subcommands are specified. If verified-handshake subcommand is specified, the command enables, disables or retrieves the verified_handshake behavior for the SSL handshake. If response_control subcommand is specified, the command ignore or mask the server side certificate errors while forging client certificate. If extension subcommand is specified, the command inserts an extension while forging a certificate.",
            source: "https://clouddocs.f5.com/api/irules/SSL__forward_proxy.html",
            examples: "when CLIENTSSL_SERVERHELLO_SEND {\n    log local0. 'bypassing'\n    SSL::forward_proxy policy bypass\n}",
            return_value: "SSL::forward_proxy policy <[bypass] | [intercept]> This command sets the policy of SSL Forward Proxy Bypass feature to \"bypass\" or \"intercept\"",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["CLIENTSSL", "SERVERSSL"],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            flow: false,
        }),
        forms: &[FormSpec {
            synopsis: "SSL::forward_proxy <subcommand> ?args?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
