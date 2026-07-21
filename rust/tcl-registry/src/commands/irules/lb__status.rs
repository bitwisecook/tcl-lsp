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

//! `LB::status` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "node",
        arity: Arity::at_least(1),
        detail: "Query/set node status.",
        synopsis: "LB::status node <addr> ?status?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pool",
        arity: Arity::at_least(3),
        detail: "Query/set pool member status.",
        synopsis: "LB::status pool <pool> member <addr> <port> ?status?",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the status of a node address or pool member.",
            synopsis: &[
                "LB::status (LB_STATUS)?",
                "LB::status node IP_ADDR (LB_STATUS)?",
                "LB::status pool POOL_OBJ member IP_ADDR PORT (LB_STATUS)?",
            ],
            snippet: "Returns the status of a node address or pool member. Possible status values are up, down, session_enabled, and session_disabled. If you supply no arguments, returns the status of the currently-selected pool member.\nSyntax:\n    LB::status\n    LB::status node <address>\n    LB::status pool <pool name> member <IP address> <port>\n    LB::status <up | down | session_enabled | session_disabled>\n    LB::status node <address> <up | down | session_enabled | session_disabled>\n    LB::status pool <pool name> member <address> <port> <up | down | session_enabled | session_disabled>",
            source: "https://clouddocs.f5.com/api/irules/LB__status.html",
            examples: "when LB_FAILED {\n    if { [LB::status pool $poolname member $ip $port] eq \"down\" } {\n        log \"Server $ip $port down!\"\n    }\n}",
            return_value: "LB::status Returns the status of the currently-selected node (after LB_SELECTED event only). Possible values are: up | down | session_enabled | session_disabled",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::status ?node <addr> | pool <pool> member <addr> <port>? ?status?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
