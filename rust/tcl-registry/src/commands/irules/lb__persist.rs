//! `LB::persist` iRules command.
use crate::prelude::*;

/// iRules subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "key",
        arity: Arity::exact(0),
        detail: "Get persistence key.",
        synopsis: "LB::persist key",
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cookie",
        arity: Arity::exact(0),
        detail: "Get persistence cookie.",
        synopsis: "LB::persist cookie",
        pure: true,
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Forces the system to make a persistence decision.",
            synopsis: &["LB::persist", "LB::persist key", "LB::persist cookie"],
            snippet: "This command forces the system to make a persistence decision, and returns a string that can be evaluated to activate that selection, or with the use of the parameter, returns a persistence key that may be used in conjunction with the persist command to manipulate the persistence table.\n\nThis enables an iRule to evaluate the pending load balancing/persistence decision early, and use that information to manage the connection.",
            source: "https://clouddocs.f5.com/api/irules/LB__persist.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LB::persist ?key | cookie?" },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::PersistenceTable,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
