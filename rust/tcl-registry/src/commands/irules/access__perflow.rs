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

//! `ACCESS::perflow` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "get",
        arity: Arity::exact(1),
        detail: "Get a perflow variable value.",
        synopsis: "ACCESS::perflow get <key>",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::exact(2),
        detail: "Set a perflow variable value.",
        synopsis: "ACCESS::perflow set <key> <value>",
        mutator: true,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "perflow.custom",
                    detail: "Custom perflow variable.",
                    min_tcl: None,
                    code: None,
                },
                ArgValue {
                    value: "perflow.scratchpad",
                    detail: "Scratchpad perflow variable.",
                    min_tcl: None,
                    code: None,
                },
                ArgValue {
                    value: "perflow.custom.flow",
                    detail: "Custom flow perflow variable.",
                    min_tcl: None,
                    code: None,
                },
                ArgValue {
                    value: "perflow.scratchpad.flow",
                    detail: "Scratchpad flow perflow variable.",
                    min_tcl: None,
                    code: None,
                },
                ArgValue {
                    value: "perflow.l7_protocol_lookup.result",
                    detail: "L7 protocol lookup result.",
                    min_tcl: None,
                    code: None,
                },
            ],
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::perflow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns perflow variable value",
            synopsis: &[
                "ACCESS::perflow get KEY",
                "ACCESS::perflow set ( 'perflow.custom' | 'perflow.scratchpad' | 'perflow.custom.flow' | 'perflow.scratchpad.flow' | 'perflow.l7_protocol_lookup.result' ) VALUE",
            ],
            snippet: "This command can be used to either set or return the value of a perflow variable that has been set inside the Access Per-Request Policy that is being run.\n\n            ACCESS::perflow get <var> will return the value of any perflow variable that has already been set. A perflow variable with no value set will return an empty string. An invalid perflow variable name will give a connection reset.\n\n            ACCESS::perflow set <var> <val> will set the value of the custom perflow variable. Currently the only perflow variables that can be set are \"perflow.custom\", \"perflow.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__perflow.html",
            examples: "when ACCESS_PER_REQUEST_AGENT_EVENT {\n                set id [ACCESS::perflow get perflow.irule_agent_id]\n\n                if { $id eq \"irule_agent_one\" } {\n                    log local0. \"Made it to iRule agent in perrequest policy.\"\n                    ACCESS::perflow set perflow.custom \"agent_one\"\n                }\n            }",
            return_value: "ACCESS::perflow get will return the string of perflow variable; empty if value isn't set",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ACCESS::perflow <get|set> <key> ?value?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
