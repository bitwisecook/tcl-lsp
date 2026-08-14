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

//! `GTP::header` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "version",
        arity: Arity::exact(0),
        detail: "Get GTP version.",
        synopsis: "GTP::header version ?-message msg?",
        pure: true,
        options: const {
            &[OptionSpec {
                name: "-message",
                value: OptionValue::value("MESSAGE"),
                detail: "Operate on specific message.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(0),
        detail: "Get GTP type.",
        synopsis: "GTP::header type ?-message msg?",
        pure: true,
        options: const {
            &[OptionSpec {
                name: "-message",
                value: OptionValue::value("MESSAGE"),
                detail: "Operate on specific message.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "npdu",
        arity: Arity::at_least(0),
        detail: "Get/set/remove GTP npdu.",
        synopsis: "GTP::header npdu ?set|remove? ?-message msg? ?value?",
        pure: true,
        mutator: true,
        options: const {
            &[OptionSpec {
                name: "-message",
                value: OptionValue::value("MESSAGE"),
                detail: "Operate on specific message.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "teid",
        arity: Arity::at_least(0),
        detail: "Get/set/remove GTP teid.",
        synopsis: "GTP::header teid ?set|remove? ?-message msg? ?value?",
        pure: true,
        mutator: true,
        options: const {
            &[OptionSpec {
                name: "-message",
                value: OptionValue::value("MESSAGE"),
                detail: "Operate on specific message.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sequence",
        arity: Arity::at_least(0),
        detail: "Get/set/remove GTP sequence.",
        synopsis: "GTP::header sequence ?set|remove? ?-message msg? ?value?",
        pure: true,
        mutator: true,
        options: const {
            &[OptionSpec {
                name: "-message",
                value: OptionValue::value("MESSAGE"),
                detail: "Operate on specific message.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "extension",
        arity: Arity::at_least(0),
        detail: "Access GTP extension headers.",
        synopsis: "GTP::header extension ?args?",
        pure: true,
        mutator: true,
        options: const {
            &[OptionSpec {
                name: "-message",
                value: OptionValue::value("MESSAGE"),
                detail: "Operate on specific message.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
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
        name: "GTP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Allows for the parsing of GTP header information.",
            synopsis: &[
                "GTP::header ('version' | 'type') ('-message' MESSAGE)?",
                "GTP::header ('teid' | 'npdu' | 'sequence') ('-message' MESSAGE)?",
                "GTP::header ('teid' | 'npdu' | 'sequence') 'set' ('-message' MESSAGE)? VALUE",
                "GTP::header ('teid' | 'npdu' | 'sequence') 'remove' ('-message' MESSAGE)?",
            ],
            snippet: "Allows for the parsing of GTP header information. UINT -- Unsigned\ninteger value of n bits. For n > 8, appropriate network to host byte\norder conversion happens transparently.",
            source: "https://clouddocs.f5.com/api/irules/GTP__header.html",
            examples: "when GTP_SIGNALLING_INGRESS {\n    log local0. \"GTP version [GTP::header version]\"\n    log local0. \"GTP type [GTP::header type]\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "GTP::header <field> ?set|remove? ?-message msg? ?value?",
            ..FormSpec::DEFAULT
        }],
        options: const {
            &[OptionSpec {
                name: "-message",
                value: OptionValue::value("MESSAGE"),
                detail: "Operate on specific message.",
                dialects: None,
                aliases: &[],
                lifecycle: Lifecycle::UNSPECIFIED,
                min_abbrev: None,
            }]
        },
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
