//! `tcl::tm::path` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "list",
        arity: Arity::exact(0),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::tm::path",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Manage the list of paths searched for Tcl modules.",
            synopsis: &[
                "tcl::tm::path add ?path ...?",
                "tcl::tm::path remove ?path ...?",
                "tcl::tm::path list",
            ],
            snippet: "The ``add`` subcommand prepends paths to the module search list, ``remove`` deletes them, and ``list`` returns the current list.",
            source: "Tcl stdlib tm module system",
            examples: "",
            return_value: "",
        }),
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
