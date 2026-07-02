//! `ttk::scale` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-from",
        takes_value: true,
        value_hint: "value",
        detail: "Starting value of the scale range.",
        dialects: None,
    },
    OptionSpec {
        name: "-to",
        takes_value: true,
        value_hint: "value",
        detail: "Ending value of the scale range.",
        dialects: None,
    },
    OptionSpec {
        name: "-value",
        takes_value: true,
        value_hint: "value",
        detail: "Current value of the scale.",
        dialects: None,
    },
    OptionSpec {
        name: "-variable",
        takes_value: true,
        value_hint: "varName",
        detail: "Variable linked to the scale value.",
        dialects: None,
    },
    OptionSpec {
        name: "-orient",
        takes_value: true,
        value_hint: "orientation",
        detail: "Orientation of the scale (horizontal or vertical).",
        dialects: None,
    },
    OptionSpec {
        name: "-length",
        takes_value: true,
        value_hint: "length",
        detail: "Length of the long axis of the scale widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-command",
        takes_value: true,
        value_hint: "script",
        detail: "Script to evaluate when the scale value changes.",
        dialects: None,
    },
    OptionSpec {
        name: "-state",
        takes_value: true,
        value_hint: "stateSpec",
        detail: "Widget state (normal or disabled).",
        dialects: None,
    },
    OptionSpec {
        name: "-style",
        takes_value: true,
        value_hint: "style",
        detail: "Style to use for the widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-class",
        takes_value: true,
        value_hint: "className",
        detail: "Widget class name for option-database lookups.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "cursor",
        detail: "Cursor to display when the pointer is over the widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "focusSpec",
        detail: "Whether the widget accepts focus during keyboard traversal.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ttk::scale pathName ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::scale",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed scale (slider) widget.",
            synopsis: &["ttk::scale pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_scale.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
