//! `ttk::progressbar` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-orient",
        takes_value: true,
        value_hint: "orientation",
        detail: "Orientation of the progress bar (horizontal or vertical).",
        dialects: None,
    },
    OptionSpec {
        name: "-length",
        takes_value: true,
        value_hint: "length",
        detail: "Length of the long axis of the progress bar.",
        dialects: None,
    },
    OptionSpec {
        name: "-mode",
        takes_value: true,
        value_hint: "progressMode",
        detail: "Mode of the progress bar (determinate or indeterminate).",
        dialects: None,
    },
    OptionSpec {
        name: "-maximum",
        takes_value: true,
        value_hint: "maximum",
        detail: "Maximum value of the progress bar.",
        dialects: None,
    },
    OptionSpec {
        name: "-value",
        takes_value: true,
        value_hint: "value",
        detail: "Current value of the progress bar.",
        dialects: None,
    },
    OptionSpec {
        name: "-variable",
        takes_value: true,
        value_hint: "varName",
        detail: "Variable linked to the progress bar value.",
        dialects: None,
    },
    OptionSpec {
        name: "-phase",
        takes_value: true,
        value_hint: "phase",
        detail: "Read-only value used by the theme engine for animation.",
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
    synopsis: "ttk::progressbar pathName ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::progressbar",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed progress indicator widget.",
            synopsis: &["ttk::progressbar pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_progressbar.n",
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
