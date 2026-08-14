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

//! `LB::down` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "node",
        arity: Arity::exact(1),
        detail: "Mark node as down.",
        synopsis: "LB::down node <address>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pool",
        arity: Arity::exact(3),
        detail: "Mark pool member as down.",
        synopsis: "LB::down pool <pool> member <address> <port>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::down",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the status of a node or pool member as being down.",
            synopsis: &[
                "LB::down",
                "LB::down node <address>",
                "LB::down pool <pool> member <address> <port>",
            ],
            snippet: "Sets the status of the specified node or pool member as being down. If you specify no arguments, the status of the currently-selected node is modified.\nNote: Calling LB::down in an iRule triggers an immediate monitor probe regardless of the monitor interval settings.\n\nLB::down\n    Sets the status of the currently-selected node as being down.\n\nLB::down node <address>\n    Sets the status of the specified node as being down.\n    Doesn't work. Use LB::down or LB::down pool <pool> member <address> <port>. Refer to BZ222047 for details.",
            source: "https://clouddocs.f5.com/api/irules/LB__down.html",
            examples: "when HTTP_RESPONSE {\n    if { [HTTP::status] == 500 } {\n        LB::down\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "LB::down ?node <addr> | pool <pool> member <addr> <port>?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NodeSelection,
                writes: true,
                connection_side: ConnectionSide::Server,
                ..SideEffect::DEFAULT
            },
            // Pool selection.
            SideEffect {
                target: SideEffectTarget::PoolSelection,
                reads: true,
                connection_side: ConnectionSide::Server,
                ..SideEffect::DEFAULT
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
