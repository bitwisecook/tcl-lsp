//! `session` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lookup",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "count",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::PersistenceTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Utilizes the persistence table to store arbitrary information based on the same keys as persistence.",
            synopsis: &["session add SESSION_MODE", "session (lookup | delete) SESSION_MODE"],
            snippet: "Utilizes the persistence table to store arbitrary information based on\nthe same keys as persistence. This information does not affect the\npersistence itself.",
            source: "https://clouddocs.f5.com/api/irules/session.html",
            examples: "when HTTP_REQUEST {\nset value [session lookup uie [list $myVar any virtual]]\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: true,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["PERSIST_DOWN"],
            init_only: false,
            flow: true,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "session add SESSION_MODE" },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::PersistenceTable,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
