//! `LB::server` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "name",
        arity: Arity::exact(0),
        detail: "Get server name.",
        synopsis: "LB::server name",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pool",
        arity: Arity::exact(0),
        detail: "Get pool name.",
        synopsis: "LB::server pool",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "route_domain",
        arity: Arity::exact(0),
        detail: "Get route domain.",
        synopsis: "LB::server route_domain",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "addr",
        arity: Arity::exact(0),
        detail: "Get server address.",
        synopsis: "LB::server addr",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "port",
        arity: Arity::exact(0),
        detail: "Get server port.",
        synopsis: "LB::server port",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "priority",
        arity: Arity::exact(0),
        detail: "Get server priority.",
        synopsis: "LB::server priority",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ratio",
        arity: Arity::exact(0),
        detail: "Get server ratio.",
        synopsis: "LB::server ratio",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "weight",
        arity: Arity::exact(0),
        detail: "Get server weight.",
        synopsis: "LB::server weight",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ripeness",
        arity: Arity::exact(0),
        detail: "Get server ripeness.",
        synopsis: "LB::server ripeness",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::server",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns information about the currently selected server.",
            synopsis: &["LB::server ?name | pool | route_domain | addr | port | priority | ratio | weight | ripeness?"],
            snippet: "This command allows you to query for information about the member selected after a load balancing decision has been made.\n\nIf no server was selected (all servers down), this command with either no arguments or the \"name\" argument will return the pool name only--useful for determining the default pool applied to a virtual server. If the node command is called prior to this command a null string is returned as the node command overrides any prior pool selection logic.\n\nLB::server [name | pool | route_domain | addr | port | priority | ratio | weight | ripeness]",
            source: "https://clouddocs.f5.com/api/irules/LB__server.html",
            examples: "when CLIENT_ACCEPTED {\n    # Save the name of the VIP's default pool\n    set default_pool [LB::server pool]\n}",
            return_value: "LB::server returns a Tcl list with pool, pool member address and port. If no server was selected yet or all servers are down, returns default pool name only.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LB::server ?field?" },
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
