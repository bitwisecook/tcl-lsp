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

//! `SSL::cert` iRules command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "count",
        arity: Arity::exact(0),
        detail: "Get certificate count in chain.",
        synopsis: "SSL::cert count",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "issuer",
        arity: Arity::exact(1),
        detail: "Get issuer info for cert at index.",
        synopsis: "SSL::cert issuer <index>",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mode",
        arity: Arity::new(0, 1),
        detail: "Get/set certificate mode.",
        synopsis: "SSL::cert mode ?ignore|request|require?",
        pure: true,
        mutator: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "ignore",
                    detail: "Do not request client cert.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "request",
                    detail: "Request client cert.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "require",
                    detail: "Require client cert.",
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
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::cert",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        surface: Some(SpecSurface::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns data about an X509 SSL certificate, or sets the certificate mode.",
            synopsis: &[
                "SSL::cert <index>",
                "SSL::cert count",
                "SSL::cert issuer <index>",
                "SSL::cert mode ?ignore|request|require?",
            ],
            snippet: "Returns data about an X509 SSL certificate, or sets the certificate mode.",
            source: "https://clouddocs.f5.com/api/irules/SSL__cert.html",
            examples: "when RULE_INIT {\n    set ::key [AES::key 128]\n}",
            return_value: "SSL::cert <index> returns the X509 SSL certificate at the specified peer-chain index. SSL::cert issuer <index> returns its issuer certificate. SSL::cert count returns the number of peer certificates. SSL::cert mode gets or sets the certificate mode; only require and ignore are valid in a server-side context.",
        }),
        forms: &[FormSpec {
            synopsis: "SSL::cert <subcommand|index> ?args?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
