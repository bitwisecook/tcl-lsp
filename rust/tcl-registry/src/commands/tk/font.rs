//! `font` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "actual",
        arity: Arity::at_least(1),
        detail: "Return the actual attributes of a font on the display.",
        synopsis: "font actual font ?-displayof window? ?option?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Query or modify the desired attributes of a named font.",
        synopsis: "font configure fontname ?option? ?value option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::at_least(0),
        detail: "Create a new named font with the given options.",
        synopsis: "font create ?fontname? ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(1),
        detail: "Delete one or more named fonts.",
        synopsis: "font delete fontname ?fontname ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "families",
        arity: Arity::new(0, 2),
        detail: "Return a list of all font families available on the display.",
        synopsis: "font families ?-displayof window?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "measure",
        arity: Arity::at_least(2),
        detail: "Measure the width of the text string when rendered in the given font.",
        synopsis: "font measure font ?-displayof window? text",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "metrics",
        arity: Arity::at_least(1),
        detail: "Return metric information for the given font.",
        synopsis: "font metrics font ?-displayof window? ?option?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Return a list of all named fonts currently defined.",
        synopsis: "font names",
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
        name: "-family",
        takes_value: true,
        value_hint: "name",
        detail: "Font family name (e.g. Courier, Times, Helvetica).",
        dialects: None,
    },
    OptionSpec {
        name: "-size",
        takes_value: true,
        value_hint: "size",
        detail: "Desired size of the font in points (positive) or pixels (negative).",
        dialects: None,
    },
    OptionSpec {
        name: "-weight",
        takes_value: true,
        value_hint: "normal|bold",
        detail: "Weight of the font: normal or bold.",
        dialects: None,
    },
    OptionSpec {
        name: "-slant",
        takes_value: true,
        value_hint: "roman|italic",
        detail: "Slant of the font: roman or italic.",
        dialects: None,
    },
    OptionSpec {
        name: "-underline",
        takes_value: true,
        value_hint: "boolean",
        detail: "Whether to draw an underline beneath the text.",
        dialects: None,
    },
    OptionSpec {
        name: "-overstrike",
        takes_value: true,
        value_hint: "boolean",
        detail: "Whether to draw a horizontal line through the text.",
        dialects: None,
    },
    OptionSpec {
        name: "-displayof",
        takes_value: true,
        value_hint: "window",
        detail: "Specifies the display for the font query.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "font option ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "font",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and inspect fonts.",
            synopsis: &[
                "font actual font ?-displayof window? ?option?",
                "font configure fontname ?option? ?value option value ...?",
                "font create ?fontname? ?option value ...?",
                "font delete fontname ?fontname ...?",
                "font families ?-displayof window?",
                "font measure font ?-displayof window? text",
                "font metrics font ?-displayof window? ?option?",
                "font names",
            ],
            snippet: "",
            source: "Tk man page font.n",
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
