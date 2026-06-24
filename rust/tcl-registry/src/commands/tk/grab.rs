//! `grab` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "current",
        arity: Arity::new(0, 1),
        detail: "Return the path name of the current grab window, if any.",
        synopsis: "grab current ?window?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "release",
        arity: Arity::exact(1),
        detail: "Release the grab on the window.",
        synopsis: "grab release window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::new(1, 2),
        detail: "Set a grab on the window, optionally global.",
        synopsis: "grab set ?-global? window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "status",
        arity: Arity::exact(1),
        detail: "Return the grab status of the window (none, local, or global).",
        synopsis: "grab status window",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-global",
    takes_value: false,
    value_hint: "",
    detail: "Make the grab global (applies to all displays).",
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "grab option ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "grab",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Confine pointer and keyboard events to a window sub-tree.",
            synopsis: &[
                "grab ?-global? window",
                "grab current ?window?",
                "grab release window",
                "grab set ?-global? window",
                "grab status window",
            ],
            snippet: "",
            source: "Tk man page grab.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
