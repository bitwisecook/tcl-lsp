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

//! `ACCESS::policy` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "agent_id",
        arity: Arity::exact(0),
        detail: "Get the agent identifier.",
        synopsis: "ACCESS::policy agent_id",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "evaluate",
        arity: Arity::at_least(0),
        detail: "Evaluate an access policy.",
        synopsis: "ACCESS::policy evaluate ?-sid id?",
        options: const {
            &[OptionSpec {
                name: "-sid",
                value: OptionValue::value("SESSION_ID"),
                detail: "Session ID.",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "result",
        arity: Arity::at_least(0),
        detail: "Get the policy result (allow/deny/redirect).",
        synopsis: "ACCESS::policy result ?-sid id?",
        pure: true,
        options: const {
            &[OptionSpec {
                name: "-sid",
                value: OptionValue::value("SESSION_ID"),
                detail: "Session ID.",
                dialects: None,
                aliases: &[],
                min_version: None,
            }]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "uri",
        arity: Arity::exact(0),
        detail: "Check if URI is internal to ACCESS.",
        synopsis: "ACCESS::policy uri",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
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
        name: "ACCESS::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Return information about access policies.",
            synopsis: &[
                "ACCESS::policy agent_id",
                "ACCESS::policy evaluate ('-sid' SESSION_ID)",
                "ACCESS::policy result (-sid SESSION_ID)?",
                "ACCESS::policy uri",
            ],
            snippet: "The ACCESS::policy commands allow you to retrieve information about the\naccess policies in place for a given connection.\n\nACCESS::policy agent_id\n\n     * Returns the identifier for the agent raising the\n       ACCESS_POLICY_AGENT_EVENT.\n\nACCESS::policy result\n\n     * Returns back the result of an access policy. The result will be one\n       of following:\n     * - allow\n     * - deny\n     * - redirect\n\nACCESS::policy uri\n\n     * Returns TRUE if current request URI is internal to ACCESS (v11+\n       only).",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__policy.html",
            examples: "when CLIENT_CLOSED {\n    # To avoid clutter, remove the access session for the flow.\n    ACCESS::session remove -sid $flow_sid\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ACCESS::policy <subcommand> ?args?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
