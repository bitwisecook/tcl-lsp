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

//! `IP::stats` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "pkts",
        arity: Arity::new(0, 1),
        detail: "Get packet counts.",
        synopsis: "IP::stats pkts ?in|out?",
        pure: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "in",
                    detail: "Packets received.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "out",
                    detail: "Packets sent.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "bytes",
        arity: Arity::new(0, 1),
        detail: "Get byte counts.",
        synopsis: "IP::stats bytes ?in|out?",
        pure: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "in",
                    detail: "Bytes received.",
                    ..ArgValue::DEFAULT
                },
                ArgValue {
                    value: "out",
                    detail: "Bytes sent.",
                    ..ArgValue::DEFAULT
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "in",
        dialects: None,
        arity: Arity::exact(0),
        detail: "Get all inbound stats.",
        synopsis: "IP::stats in",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "out",
        arity: Arity::exact(0),
        detail: "Get all outbound stats.",
        synopsis: "IP::stats out",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "age",
        arity: Arity::exact(0),
        detail: "Get connection age in ms.",
        synopsis: "IP::stats age",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::stats",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Supplies information about the number of packets or bytes being sent or received in a given connection.",
            synopsis: &[
                "IP::stats ((pkts ('in' | 'out')?) | (bytes ('in' | 'out')?) | in | out | age)?",
            ],
            snippet: "This command supplies information about the number of packets or bytes being sent or received in a given connection.\n\nIP::stats\nReturns a list with Packets In, Packets Out, Bytes In, Bytes Out & Age\n\nIP::stats pkts in\nReturns number of packets received\n\nIP::stats pkts out\nReturns number of packets sent\n\nIP::stats pkts\nReturns a Tcl list of packets in and packets out\n\nIP::stats bytes in\nReturns number of bytes received\n\nIP::stats bytes out\nReturns number of bytes sent\n\nIP::stats bytes\nReturns Tcl list of bytes in and bytes out\n\nIP::stats age\nReturns the age of the connection in milliseconds",
            source: "https://clouddocs.f5.com/api/irules/IP__stats.html",
            examples: "# The following example calculates and logs response time:\nwhen HTTP_REQUEST {\n    set reqAge [IP::stats age]\n    set reqURI [HTTP::uri]\n    set reqClient [IP::remote_addr]:[TCP::remote_port]\n}",
            return_value: "number of packets or bytes being sent or received in a given connection",
        }),
        forms: &[FormSpec {
            synopsis: "IP::stats ?pkts|bytes|in|out|age? ?in|out?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
