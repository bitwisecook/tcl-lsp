//! `LB::up` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "node",
        arity: Arity::exact(1),
        detail: "Mark node as up.",
        synopsis: "LB::up node <address>",
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
        detail: "Mark pool member as up.",
        synopsis: "LB::up pool <pool> member <address> <port>",
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

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::up",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the status of a node or pool member as being up.",
            synopsis: &[
                "LB::up",
                "LB::up node <address>",
                "LB::up pool <pool> member <address> <port>",
            ],
            snippet: "Sets the status of the specified node or pool member as being up. If you specify no arguments, the status of the currently-selected node is modified.",
            source: "https://clouddocs.f5.com/api/irules/LB__up.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::up ?node <addr> | pool <pool> member <addr> <port>?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..CommandSpec::DEFAULT
    }
}
