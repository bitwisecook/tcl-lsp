//! `ACCESS::session` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "create",
        arity: Arity::at_least(0),
        detail: "Create a new session.",
        synopsis: "ACCESS::session create ?-flow? ?-timeout secs? ?-lifetime secs?",
        mutator: true,
        options: &[
            OptionSpec {
                name: "-flow",
                takes_value: false,
                value_hint: "",
                detail: "Create a flow-scoped session.",
                dialects: None,
            },
            OptionSpec {
                name: "-timeout",
                takes_value: true,
                value_hint: "SECONDS",
                detail: "Session timeout in seconds.",
                dialects: None,
            },
            OptionSpec {
                name: "-lifetime",
                takes_value: true,
                value_hint: "SECONDS",
                detail: "Session lifetime in seconds.",
                dialects: None,
            },
        ],
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
        options: &[
            OptionSpec {
                name: "-sid",
                takes_value: true,
                value_hint: "SESSION_ID",
                detail: "Session ID.",
                dialects: None,
            },
            OptionSpec {
                name: "-timeout",
                takes_value: true,
                value_hint: "SECONDS",
                detail: "Session timeout in seconds.",
                dialects: None,
            },
            OptionSpec {
                name: "-lifetime",
                takes_value: true,
                value_hint: "SECONDS",
                detail: "Session lifetime in seconds.",
                dialects: None,
            },
            OptionSpec {
                name: "-remaining",
                takes_value: true,
                value_hint: "",
                detail: "Remaining time.",
                dialects: None,
            },
        ],
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
        options: &[
            OptionSpec {
                name: "-sid",
                takes_value: true,
                value_hint: "SESSION_ID",
                detail: "Session ID.",
                dialects: None,
            },
            OptionSpec {
                name: "-state_allow",
                takes_value: false,
                value_hint: "",
                detail: "Check for allow state.",
                dialects: None,
            },
            OptionSpec {
                name: "-state_deny",
                takes_value: false,
                value_hint: "",
                detail: "Check for deny state.",
                dialects: None,
            },
            OptionSpec {
                name: "-state_redirect",
                takes_value: false,
                value_hint: "",
                detail: "Check for redirect state.",
                dialects: None,
            },
            OptionSpec {
                name: "-state_inprogress",
                takes_value: false,
                value_hint: "",
                detail: "Check for in-progress state.",
                dialects: None,
            },
        ],
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
        options: &[
            OptionSpec {
                name: "-sid",
                takes_value: true,
                value_hint: "SESSION_ID",
                detail: "Session ID.",
                dialects: None,
            },
            OptionSpec {
                name: "-secure",
                takes_value: false,
                value_hint: "",
                detail: "Access secure session data.",
                dialects: None,
            },
            OptionSpec {
                name: "-config",
                takes_value: false,
                value_hint: "",
                detail: "Access config session data.",
                dialects: None,
            },
            OptionSpec {
                name: "-ssid",
                takes_value: true,
                value_hint: "SESSION_ID",
                detail: "Sub-session ID.",
                dialects: None,
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
        ],
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
        options: &[OptionSpec {
            name: "-sid",
            takes_value: true,
            value_hint: "SESSION_ID",
            detail: "Session ID.",
            dialects: None,
        }],
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

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Access or manipulate session information.",
            synopsis: &["ACCESS::session create (('-flow')? ('-timeout' TIMEOUT)? ('-lifetime' LIFETIME)?)#", "ACCESS::session modify ('-sid' SESSION_ID)? (('-timeout' TIMEOUT)? (('-lifetime' LIFETIME)? | ('-remaining' REMAINING)?))#", "ACCESS::session exists ('-state_allow' | '-state_deny' | '-state_redirect' | '-state_inprogress')? (-sid)? (SESSION_ID)?", "ACCESS::session data get ('-sid' SESSION_ID)? ('-secure' | '-config')? KEY (-ssid SESSION_ID)?"],
            snippet: "The different permutations of the ACCESS::session command allow you to\naccess or manipulate different portions of session information when\ndealing with APM requests.\n\nACCESS::session data get\n\n     * Returns the value of session variable.\n\nACCESS::session data set [ ]\n\n     * Sets the value of session variable to be the given.\n\nACCESS::session exists\n\n     * This commands returns TRUE when the session with provided sid\n       exists, and returns FALSE otherwise. This command is allowed to be\n       executed in different events other then ACCESS events. This command\n       added in version 10.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__session.html",
            examples: "when ACCESS_ACL_ALLOWED {\nset user [ACCESS::session data get \"session.logon.last.username\"]\nHTTP::header insert \"X-USERNAME\" $user\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ACCESS::session <subcommand> ?options? ?args?" },
        ],
        options: &[
            OptionSpec { name: "-flow", takes_value: false, value_hint: "", detail: "Create a flow-scoped session.", dialects: None },
            OptionSpec { name: "-timeout", takes_value: true, value_hint: "SECONDS", detail: "Session timeout in seconds.", dialects: None },
            OptionSpec { name: "-lifetime", takes_value: true, value_hint: "SECONDS", detail: "Session lifetime in seconds.", dialects: None },
            OptionSpec { name: "-sid", takes_value: true, value_hint: "SESSION_ID", detail: "Session ID.", dialects: None },
            OptionSpec { name: "-remaining", takes_value: true, value_hint: "", detail: "Remaining time.", dialects: None },
            OptionSpec { name: "-secure", takes_value: false, value_hint: "", detail: "Access secure session data.", dialects: None },
            OptionSpec { name: "-config", takes_value: false, value_hint: "", detail: "Access config session data.", dialects: None },
            OptionSpec { name: "-ssid", takes_value: true, value_hint: "SESSION_ID", detail: "Sub-session ID.", dialects: None },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ApmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
