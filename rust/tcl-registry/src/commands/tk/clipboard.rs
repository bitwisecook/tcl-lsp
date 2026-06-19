//! `clipboard` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "append",
        arity: Arity::at_least(1),
        detail: "Append data to the clipboard on the specified display.",
        synopsis: "clipboard append ?-displayof window? ?-format format? ?-type type? data",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clear",
        arity: Arity::at_least(0),
        detail: "Claim ownership of the clipboard and clear its contents.",
        synopsis: "clipboard clear ?-displayof window?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::at_least(0),
        detail: "Retrieve data from the clipboard on the specified display.",
        synopsis: "clipboard get ?-displayof window? ?-type type?",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        takes_value: true,
        value_hint: "window",
        detail: "Specifies the display for the clipboard operation.",
        dialects: None,
    },
    OptionSpec {
        name: "-format",
        takes_value: true,
        value_hint: "format",
        detail: "Specifies the representation format for the data (append).",
        dialects: None,
    },
    OptionSpec {
        name: "-type",
        takes_value: true,
        value_hint: "type",
        detail: "Specifies the form in which the selection is to be returned.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "clipboard option ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clipboard",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Manipulate the Tk clipboard.",
            synopsis: &[
                "clipboard append ?-displayof window? ?-format format? ?-type type? data",
                "clipboard clear ?-displayof window?",
                "clipboard get ?-displayof window? ?-type type?",
            ],
            snippet: "",
            source: "Tk man page clipboard.n",
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
