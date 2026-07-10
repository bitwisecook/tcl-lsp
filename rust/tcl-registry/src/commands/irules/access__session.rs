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

//! `ACCESS::session` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "create",
        arity: Arity::at_least(0),
        detail: "Create a new session.",
        synopsis: "ACCESS::session create ?-flow? ?-timeout secs? ?-lifetime secs?",
        mutator: true,
        options: OPTIONS_1,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "modify",
        arity: Arity::at_least(0),
        detail: "Modify an existing session.",
        synopsis: "ACCESS::session modify ?-sid id? ?-timeout secs? ?-lifetime secs?",
        mutator: true,
        options: OPTIONS_2,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::new(0, 1),
        detail: "Check if a session exists.",
        synopsis: "ACCESS::session exists ?-sid id?",
        pure: true,
        options: OPTIONS_3,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "data",
        arity: Arity::at_least(1),
        detail: "Get or set session data.",
        synopsis: "ACCESS::session data <get|set> ?-sid id? <key> ?--? ?value?",
        options: OPTIONS_4,
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "get",
                    detail: "Get session variable value.",
                },
                ArgValue {
                    value: "set",
                    detail: "Set session variable value.",
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
    SubCommand {
        name: "remove",
        arity: Arity::at_least(0),
        detail: "Remove a session.",
        synopsis: "ACCESS::session remove ?-sid id?",
        mutator: true,
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
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sid",
        arity: Arity::exact(0),
        detail: "Get the session ID.",
        synopsis: "ACCESS::session sid",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

const OPTIONS_1: &[OptionSpec] = &[
    OptionSpec {
        name: "-flow",
        value: OptionValue::flag(),
        detail: "Create a flow-scoped session.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-timeout",
        value: OptionValue::value("SECONDS"),
        detail: "Session timeout in seconds.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-lifetime",
        value: OptionValue::value("SECONDS"),
        detail: "Session lifetime in seconds.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const OPTIONS_2: &[OptionSpec] = &[
    OptionSpec {
        name: "-sid",
        value: OptionValue::value("SESSION_ID"),
        detail: "Session ID.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-timeout",
        value: OptionValue::value("SECONDS"),
        detail: "Session timeout in seconds.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-lifetime",
        value: OptionValue::value("SECONDS"),
        detail: "Session lifetime in seconds.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-remaining",
        value: OptionValue::value(""),
        detail: "Remaining time.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const OPTIONS_3: &[OptionSpec] = &[
    OptionSpec {
        name: "-sid",
        value: OptionValue::value("SESSION_ID"),
        detail: "Session ID.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state_allow",
        value: OptionValue::flag(),
        detail: "Check for allow state.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state_deny",
        value: OptionValue::flag(),
        detail: "Check for deny state.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state_redirect",
        value: OptionValue::flag(),
        detail: "Check for redirect state.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state_inprogress",
        value: OptionValue::flag(),
        detail: "Check for in-progress state.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const OPTIONS_4: &[OptionSpec] = &[
    OptionSpec {
        name: "-sid",
        value: OptionValue::value("SESSION_ID"),
        detail: "Session ID.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-secure",
        value: OptionValue::flag(),
        detail: "Access secure session data.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-config",
        value: OptionValue::flag(),
        detail: "Access config session data.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ssid",
        value: OptionValue::value("SESSION_ID"),
        detail: "Sub-session ID.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const OPTIONS_5: &[OptionSpec] = &[
    OptionSpec {
        name: "-flow",
        value: OptionValue::flag(),
        detail: "Create a flow-scoped session.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-timeout",
        value: OptionValue::value("SECONDS"),
        detail: "Session timeout in seconds.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-lifetime",
        value: OptionValue::value("SECONDS"),
        detail: "Session lifetime in seconds.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-sid",
        value: OptionValue::value("SESSION_ID"),
        detail: "Session ID.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-remaining",
        value: OptionValue::value(""),
        detail: "Remaining time.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state_allow",
        value: OptionValue::flag(),
        detail: "Check for allow state.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state_deny",
        value: OptionValue::flag(),
        detail: "Check for deny state.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state_redirect",
        value: OptionValue::flag(),
        detail: "Check for redirect state.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state_inprogress",
        value: OptionValue::flag(),
        detail: "Check for in-progress state.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-secure",
        value: OptionValue::flag(),
        detail: "Access secure session data.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-config",
        value: OptionValue::flag(),
        detail: "Access config session data.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-ssid",
        value: OptionValue::value("SESSION_ID"),
        detail: "Sub-session ID.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Access or manipulate session information.",
            synopsis: &[
                "ACCESS::session create (('-flow')? ('-timeout' TIMEOUT)? ('-lifetime' LIFETIME)?)#",
                "ACCESS::session modify ('-sid' SESSION_ID)? (('-timeout' TIMEOUT)? (('-lifetime' LIFETIME)? | ('-remaining' REMAINING)?))#",
                "ACCESS::session exists ('-state_allow' | '-state_deny' | '-state_redirect' | '-state_inprogress')? (-sid)? (SESSION_ID)?",
                "ACCESS::session data get ('-sid' SESSION_ID)? ('-secure' | '-config')? KEY (-ssid SESSION_ID)?",
            ],
            snippet: "The different permutations of the ACCESS::session command allow you to\naccess or manipulate different portions of session information when\ndealing with APM requests.\n\nACCESS::session data get\n\n     * Returns the value of session variable.\n\nACCESS::session data set [ ]\n\n     * Sets the value of session variable to be the given.\n\nACCESS::session exists\n\n     * This commands returns TRUE when the session with provided sid\n       exists, and returns FALSE otherwise. This command is allowed to be\n       executed in different events other then ACCESS events. This command\n       added in version 10.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__session.html",
            examples: "when ACCESS_ACL_ALLOWED {\nset user [ACCESS::session data get \"session.logon.last.username\"]\nHTTP::header insert \"X-USERNAME\" $user\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ACCESS::session <subcommand> ?options? ?args?",
        }],
        options: OPTIONS_5,
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
