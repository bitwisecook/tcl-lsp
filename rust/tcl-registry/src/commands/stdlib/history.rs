//! `history` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::new(1, 2),
        detail: "Add a command to the history list.",
        synopsis: "history add command ?exec?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "change",
        arity: Arity::new(1, 2),
        detail: "Replace a history event.",
        synopsis: "history change newValue ?event?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clear",
        arity: Arity::exact(0),
        detail: "Clear the history list.",
        synopsis: "history clear",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "event",
        arity: Arity::new(0, 1),
        detail: "Return a history event by number or pattern.",
        synopsis: "history event ?event?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(0, 1),
        detail: "Return a formatted history list.",
        synopsis: "history info ?count?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "keep",
        arity: Arity::new(0, 1),
        detail: "Get or set the size of the history list.",
        synopsis: "history keep ?count?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nextid",
        arity: Arity::exact(0),
        detail: "Return the next event number.",
        synopsis: "history nextid",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "redo",
        arity: Arity::new(0, 1),
        detail: "Re-evaluate a history event.",
        synopsis: "history redo ?event?",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "history subcommand ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "history",
        traits: Traits::UNSAFE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Manipulate the history list of previously executed commands.",
            synopsis: &[
                "history",
                "history add command ?exec?",
                "history change newValue ?event?",
                "history clear",
                "history event ?event?",
                "history info ?count?",
                "history keep ?count?",
                "history nextid",
                "history redo ?event?",
            ],
            snippet: "",
            source: "Tcl stdlib history command",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
