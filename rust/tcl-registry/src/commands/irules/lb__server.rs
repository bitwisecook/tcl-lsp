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

//! `LB::server` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "name",
        arity: Arity::exact(0),
        detail: "Get server name.",
        synopsis: "LB::server name",
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
        arity: Arity::exact(0),
        detail: "Get pool name.",
        synopsis: "LB::server pool",
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
        name: "route_domain",
        arity: Arity::exact(0),
        detail: "Get route domain.",
        synopsis: "LB::server route_domain",
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
        name: "addr",
        arity: Arity::exact(0),
        detail: "Get server address.",
        synopsis: "LB::server addr",
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
        name: "port",
        arity: Arity::exact(0),
        detail: "Get server port.",
        synopsis: "LB::server port",
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
        name: "priority",
        arity: Arity::exact(0),
        detail: "Get server priority.",
        synopsis: "LB::server priority",
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
        name: "ratio",
        arity: Arity::exact(0),
        detail: "Get server ratio.",
        synopsis: "LB::server ratio",
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
        name: "weight",
        arity: Arity::exact(0),
        detail: "Get server weight.",
        synopsis: "LB::server weight",
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
        name: "ripeness",
        arity: Arity::exact(0),
        detail: "Get server ripeness.",
        synopsis: "LB::server ripeness",
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
        name: "LB::server",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns information about the currently selected server.",
            synopsis: &[
                "LB::server ?name | pool | route_domain | addr | port | priority | ratio | weight | ripeness?",
            ],
            snippet: "This command allows you to query for information about the member selected after a load balancing decision has been made.\n\nIf no server was selected (all servers down), this command with either no arguments or the \"name\" argument will return the pool name only--useful for determining the default pool applied to a virtual server. If the node command is called prior to this command a null string is returned as the node command overrides any prior pool selection logic.\n\nLB::server [name | pool | route_domain | addr | port | priority | ratio | weight | ripeness]",
            source: "https://clouddocs.f5.com/api/irules/LB__server.html",
            examples: "when CLIENT_ACCEPTED {\n    # Save the name of the VIP's default pool\n    set default_pool [LB::server pool]\n}",
            return_value: "LB::server returns a Tcl list with pool, pool member address and port. If no server was selected yet or all servers are down, returns default pool name only.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::server ?field?",
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
