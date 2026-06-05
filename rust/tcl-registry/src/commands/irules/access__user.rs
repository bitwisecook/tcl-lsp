//! `ACCESS::user` iRules command.
use crate::prelude::*;

/// iRules subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "getkey",
        arity: Arity::exact(1),
        detail: "Get original SID from hash.",
        synopsis: "ACCESS::user getkey <sid_hash>",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "getsid",
        arity: Arity::exact(1),
        detail: "Get external SIDs for key.",
        synopsis: "ACCESS::user getsid <key>",
        pure: true,
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::user",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns user ID information.",
            synopsis: &["ACCESS::user getkey SID_HASH", "ACCESS::user getsid KEY", "ACCESS::user ACCESS_USER_COMMAND (ACCESS_USER_INFO)?"],
            snippet: "The ACCESS::user commands return user ID information.\n\nACCESS::user getsid <key>\n\n     * Returns the list of created external SIDs which is associated wit\n       the specified key\n\nACCESS::user getkey <sid_hash>\n\n     * Returns the original SID for specified hash of SID\n     * This command works for clientless mode only\n\n * Requires APM module",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__user.html",
            examples: "when ACCESS_SESSION_STARTED {\n    # Associate the user_key with the session by assigning the value.\n    if { [ info exists user_key ] } {\n        ACCESS::session data set \"session.user.uuid\" $user_key\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ACCESS::user <subcommand> <arg>" },
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
