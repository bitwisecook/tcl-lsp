//! `LB::connlimit` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "virtual",
        arity: Arity::at_least(0),
        detail: "Set/get virtual connection limit.",
        synopsis: "LB::connlimit virtual ?limit <value>? ?key <value>?",
        pure: true,
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
        name: "node",
        arity: Arity::at_least(0),
        detail: "Set/get node connection limit.",
        synopsis: "LB::connlimit node ?limit <value>? ?key <value>?",
        pure: true,
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
        name: "poolmember",
        arity: Arity::at_least(0),
        detail: "Set/get poolmember connection limit.",
        synopsis: "LB::connlimit poolmember ?limit <value>? ?key <value>?",
        pure: true,
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
        name: "LB::connlimit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set the connection limit for virtual/node/poolmember.",
            synopsis: &[
                "LB::connlimit ('virtual' | 'node' | 'poolmember') ?limit <value>? ?key <value>?",
            ],
            snippet: "Set the connection limit for virtual/node/poolmember",
            source: "https://clouddocs.f5.com/api/irules/LB__connlimit.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::connlimit <target> ?args?",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
            // Mirrors Python `lb__connlimit.py` (POOL_SELECTION).
            SideEffect {
                target: SideEffectTarget::PoolSelection,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Server,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
