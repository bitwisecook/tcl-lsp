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
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
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
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
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
                    min_tcl: None,
                    code: None,
                },
                ArgValue {
                    value: "request",
                    detail: "Request client cert.",
                    min_tcl: None,
                    code: None,
                },
                ArgValue {
                    value: "require",
                    detail: "Require client cert.",
                    min_tcl: None,
                    code: None,
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
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
        name: "SSL::cert",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::IRULES),
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
            return_value: "SSL::cert <index> Returns the X509 SSL certificate at the specified index in the peer certificate chain, where index is a value greater than or equal to zero. A value of zero denotes the first certificate in the chain, a value of one denotes the next, and so on.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::cert <subcommand|index> ?args?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
