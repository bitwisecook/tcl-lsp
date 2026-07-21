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

//! `DIAMETER::avp` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "code",
        arity: Arity::new(1, 3),
        detail: "Get/set AVP code.",
        synopsis: "DIAMETER::avp code <avp_code> ?vendor_id? ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "count",
        arity: Arity::new(1, 2),
        detail: "Count AVPs matching code.",
        synopsis: "DIAMETER::avp count <avp_code> ?vendor_id?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "length",
        arity: Arity::new(1, 3),
        detail: "Get AVP length.",
        synopsis: "DIAMETER::avp length <avp_code> ?vendor_id? ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::new(2, 3),
        detail: "Create a new AVP.",
        synopsis: "DIAMETER::avp create <avp_code> <data> ?vendor_id?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "data",
        arity: Arity::new(1, 3),
        detail: "Get/set AVP data.",
        synopsis: "DIAMETER::avp data <avp_code> ?vendor_id? ?index?",
        pure: true,
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::new(1, 3),
        detail: "Delete an AVP.",
        synopsis: "DIAMETER::avp delete <avp_code> ?vendor_id? ?index?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::new(2, 4),
        detail: "Replace AVP data.",
        synopsis: "DIAMETER::avp replace <avp_code> <data> ?vendor_id? ?index?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::new(3, 4),
        detail: "Insert a new AVP at position.",
        synopsis: "DIAMETER::avp insert <position> <avp_code> <data> ?vendor_id?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "append",
        arity: Arity::new(2, 3),
        detail: "Append an AVP.",
        synopsis: "DIAMETER::avp append <avp_code> <data> ?vendor_id?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "flags",
        arity: Arity::new(2, 5),
        detail: "Get/set AVP flags.",
        synopsis: "DIAMETER::avp flags <get|set> <avp_code> ?value? ?vendor_id? ?index?",
        pure: true,
        mutator: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "get",
                    detail: "Get AVP flags.",
                    min_tcl: None,
                    code: None,
                },
                ArgValue {
                    value: "set",
                    detail: "Set AVP flags.",
                    min_tcl: None,
                    code: None,
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::avp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Provides detailed access to diameter attribute-value pairs.",
            synopsis: &[
                "DIAMETER::avp <subcommand> ?args?",
                "DIAMETER::avp code <avp_code> ?vendor_id? ?index?",
                "DIAMETER::avp data <avp_code> ?vendor_id? ?index?",
            ],
            snippet: "This iRule command gives access to set and get attribute-value pairs.\nSpecifics for each command are below in the syntax section.\n\nThe AVP upon which this command operates is specified in a flexible\nmanner.  An AVP name or code (usually) must be specified, and an\noptional index may be also specified.  Many commands also accept a\nvendor-id.  When an AVP name is specified, it is converted to a code.\nNames are written as listed in RFC 3588, formatted as e.g.,\n\"HOST-IP-ADDRESS\".  AVP codes are 32-bit (4-octet) integer values.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__avp.html",
            examples: "when DIAMETER_EGRESS {\n     # Sets the flags of the AVP Product Name to 0 (for Vendor Specific, Mandatory, Protected and Reserved)\n     DIAMETER::avp flags set 269 0\n     # Checks that the flags are properly set (was a bug in 11.3, solved in 11.4)\n     log local0. \"AVP : [DIAMETER::avp flags get 269] \"\n     # Removes the Supported-Vendor-Id from the request\n     DIAMETER::avp delete 265\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DIAMETER::avp <subcommand> ?args?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
