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

//! `LB::connlimit` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "virtual",
        arity: Arity::at_least(0),
        detail: "Set/get virtual connection limit.",
        synopsis: "LB::connlimit virtual ?limit <value>? ?key <value>?",
        pure: true,
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
        name: "node",
        arity: Arity::at_least(0),
        detail: "Set/get node connection limit.",
        synopsis: "LB::connlimit node ?limit <value>? ?key <value>?",
        pure: true,
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
        name: "poolmember",
        arity: Arity::at_least(0),
        detail: "Set/get poolmember connection limit.",
        synopsis: "LB::connlimit poolmember ?limit <value>? ?key <value>?",
        pure: true,
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
        name: "LB::connlimit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set the connection limit for virtual/node/poolmember.",
            synopsis: &[
                "LB::connlimit ('virtual' | 'node' | 'poolmember') ?limit <value>? ?key <value>?",
            ],
            snippet: "Set the connection limit for virtual/node/poolmember",
            source: "https://clouddocs.f5.com/api/irules/LB__connlimit.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "LB::connlimit <target> ?args?",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                writes: true,
                connection_side: ConnectionSide::Both,
                ..SideEffect::DEFAULT
            },
            // Pool selection.
            SideEffect {
                target: SideEffectTarget::PoolSelection,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Server,
                ..SideEffect::DEFAULT
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
