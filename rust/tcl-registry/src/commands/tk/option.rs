//! `option` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::new(2, 3),
        detail: "Add an option to the database with optional priority.",
        synopsis: "option add pattern value ?priority?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clear",
        arity: Arity::exact(0),
        detail: "Clear all options from the database.",
        synopsis: "option clear",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::exact(3),
        detail: "Retrieve the value of the option for a window.",
        synopsis: "option get window name class",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "readfile",
        arity: Arity::new(1, 2),
        detail: "Read options from a file and add them to the database.",
        synopsis: "option readfile fileName ?priority?",
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
    synopsis: "option option ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "option",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Add or retrieve window options to or from the option database.",
            synopsis: &[
                "option add pattern value ?priority?",
                "option clear",
                "option get window name class",
                "option readfile fileName ?priority?",
            ],
            snippet: "",
            source: "Tk man page option.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
