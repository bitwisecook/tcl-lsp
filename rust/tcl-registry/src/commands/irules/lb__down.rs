//! `LB::down` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "node",
        arity: Arity::exact(1),
        detail: "Mark node as down.",
        synopsis: "LB::down node <address>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pool",
        arity: Arity::exact(3),
        detail: "Mark pool member as down.",
        synopsis: "LB::down pool <pool> member <address> <port>",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::down",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the status of a node or pool member as being down.",
            synopsis: &["LB::down", "LB::down node <address>", "LB::down pool <pool> member <address> <port>"],
            snippet: "Sets the status of the specified node or pool member as being down. If you specify no arguments, the status of the currently-selected node is modified.\nNote: Calling LB::down in an iRule triggers an immediate monitor probe regardless of the monitor interval settings.\n\nLB::down\n    Sets the status of the currently-selected node as being down.\n\nLB::down node <address>\n    Sets the status of the specified node as being down.\n    Doesn't work. Use LB::down or LB::down pool <pool> member <address> <port>. Refer to BZ222047 for details.",
            source: "https://clouddocs.f5.com/api/irules/LB__down.html",
            examples: "when HTTP_RESPONSE {\n    if { [HTTP::status] == 500 } {\n        LB::down\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LB::down ?node <addr> | pool <pool> member <addr> <port>?" },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NodeSelection,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Server,
            },
            // Mirrors Python `lb__down.py` (POOL_SELECTION).
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
