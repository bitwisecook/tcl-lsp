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

//! `SIP::header` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "value",
        arity: Arity::new(1, 2),
        detail: "Get header value by name and optional index.",
        synopsis: "SIP::header value <name> ?index?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::new(1, 2),
        detail: "Remove a header by name and optional index.",
        synopsis: "SIP::header remove <name> ?index?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::new(2, 3),
        detail: "Insert a new header.",
        synopsis: "SIP::header insert <name> <value> ?index?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "List all header names.",
        synopsis: "SIP::header names",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "at",
        arity: Arity::exact(1),
        detail: "Get header at index.",
        synopsis: "SIP::header at <index>",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Check if header exists.",
        synopsis: "SIP::header exists <name>",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::new(2, 3),
        detail: "Replace header value.",
        synopsis: "SIP::header replace <name> <value> ?index?",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "count",
        arity: Arity::exact(1),
        detail: "Count headers with name.",
        synopsis: "SIP::header count <name>",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "values",
        arity: Arity::exact(1),
        detail: "Get all values for header.",
        synopsis: "SIP::header values <name>",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets SIP header information.",
            synopsis: &[
                "SIP::header SIP_HEADER_NAME (INDEX)?",
                "SIP::header ('value' | 'remove') HEADER_NAME (INDEX)?",
                "SIP::header ('insert') HEADER_NAME HEADER_VALUE (INDEX)?",
                "SIP::header 'names'",
            ],
            snippet: "This set of commands allows you to get or set information in the SIP\nheader.\n\nNote: These commands still work on MBLB (Message Based Load\nBalancing) SIP post 11.6+, but there are new commands that only\nrun on MRF (Message Routing Framework) SIP and were introduced\nin 11.6.",
            source: "https://clouddocs.f5.com/api/irules/SIP__header.html",
            examples: "when SIP_REQUEST_SEND {\n  log local0. [SIP::method]\n  SIP::header insert Via [format \"SIP/2.0/TCP %s:%s\" [IP::local_addr] [TCP::local_port]]\n  SIP::header insert Y-Header \"it is yyy\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &["MR_EGRESS", "MR_FAILED", "MR_INGRESS"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SIP::header <subcommand|name> ?args?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
