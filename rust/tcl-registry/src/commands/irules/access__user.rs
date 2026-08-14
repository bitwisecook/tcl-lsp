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

//! `ACCESS::user` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "getkey",
        arity: Arity::exact(1),
        detail: "Get original SID from hash.",
        synopsis: "ACCESS::user getkey <sid_hash>",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "getsid",
        arity: Arity::exact(1),
        detail: "Get external SIDs for key.",
        synopsis: "ACCESS::user getsid <key>",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::user",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns user ID information.",
            synopsis: &[
                "ACCESS::user getkey SID_HASH",
                "ACCESS::user getsid KEY",
                "ACCESS::user ACCESS_USER_COMMAND (ACCESS_USER_INFO)?",
            ],
            snippet: "The ACCESS::user commands return user ID information.\n\nACCESS::user getsid <key>\n\n     * Returns the list of created external SIDs which is associated wit\n       the specified key\n\nACCESS::user getkey <sid_hash>\n\n     * Returns the original SID for specified hash of SID\n     * This command works for clientless mode only\n\n * Requires APM module",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__user.html",
            examples: "when ACCESS_SESSION_STARTED {\n    # Associate the user_key with the session by assigning the value.\n    if { [ info exists user_key ] } {\n        ACCESS::session data set \"session.user.uuid\" $user_key\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            synopsis: "ACCESS::user <subcommand> <arg>",
            ..FormSpec::DEFAULT
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            connection_side: ConnectionSide::Both,
            ..SideEffect::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
