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

//! `LB::persist` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "key",
        arity: Arity::exact(0),
        detail: "Get persistence key.",
        synopsis: "LB::persist key",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cookie",
        arity: Arity::exact(0),
        detail: "Get persistence cookie.",
        synopsis: "LB::persist cookie",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
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
        name: "LB::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Forces the system to make a persistence decision.",
            synopsis: &["LB::persist", "LB::persist key", "LB::persist cookie"],
            snippet: "This command forces the system to make a persistence decision, and returns a string that can be evaluated to activate that selection, or with the use of the parameter, returns a persistence key that may be used in conjunction with the persist command to manipulate the persistence table.\n\nThis enables an iRule to evaluate the pending load balancing/persistence decision early, and use that information to manage the connection.",
            source: "https://clouddocs.f5.com/api/irules/LB__persist.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::persist ?key | cookie?",
            dialects: None,
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
