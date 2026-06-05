//! `LB::status` iRules command.
use crate::prelude::*;

/// iRules subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "node",
        arity: Arity::at_least(1),
        detail: "Query/set node status.",
        synopsis: "LB::status node <addr> ?status?",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pool",
        arity: Arity::at_least(3),
        detail: "Query/set pool member status.",
        synopsis: "LB::status pool <pool> member <addr> <port> ?status?",
        pure: true,
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the status of a node address or pool member.",
            synopsis: &["LB::status (LB_STATUS)?", "LB::status node IP_ADDR (LB_STATUS)?", "LB::status pool POOL_OBJ member IP_ADDR PORT (LB_STATUS)?"],
            snippet: "Returns the status of a node address or pool member. Possible status values are up, down, session_enabled, and session_disabled. If you supply no arguments, returns the status of the currently-selected pool member.\nSyntax:\n    LB::status\n    LB::status node <address>\n    LB::status pool <pool name> member <IP address> <port>\n    LB::status <up | down | session_enabled | session_disabled>\n    LB::status node <address> <up | down | session_enabled | session_disabled>\n    LB::status pool <pool name> member <address> <port> <up | down | session_enabled | session_disabled>",
            source: "https://clouddocs.f5.com/api/irules/LB__status.html",
            examples: "when LB_FAILED {\n    if { [LB::status pool $poolname member $ip $port] eq \"down\" } {\n        log \"Server $ip $port down!\"\n    }\n}",
            return_value: "LB::status Returns the status of the currently-selected node (after LB_SELECTED event only). Possible values are: up | down | session_enabled | session_disabled",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LB::status ?node <addr> | pool <pool> member <addr> <port>? ?status?" },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::PoolSelection,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Server,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
