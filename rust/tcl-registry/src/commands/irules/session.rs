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

//! `session` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lookup",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "count",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Utilizes the persistence table to store arbitrary information based on the same keys as persistence.",
            synopsis: &[
                "session add SESSION_MODE",
                "session (lookup | delete) SESSION_MODE",
            ],
            snippet: "Utilizes the persistence table to store arbitrary information based on\nthe same keys as persistence. This information does not affect the\npersistence itself.",
            source: "https://clouddocs.f5.com/api/irules/session.html",
            examples: "when HTTP_REQUEST {\nset value [session lookup uie [list $myVar any virtual]]\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["PERSIST_DOWN"],
            init_only: false,
            flow: true,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "session add SESSION_MODE",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Client,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
