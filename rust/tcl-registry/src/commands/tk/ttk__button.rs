//! `ttk::button` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-text",
        takes_value: true,
        value_hint: "string",
        detail: "Text to display in the button.",
        dialects: None,
    },
    OptionSpec {
        name: "-textvariable",
        takes_value: true,
        value_hint: "varName",
        detail: "Variable whose value is used as the button text.",
        dialects: None,
    },
    OptionSpec {
        name: "-command",
        takes_value: true,
        value_hint: "script",
        detail: "Script to evaluate when the button is invoked.",
        dialects: None,
    },
    OptionSpec {
        name: "-image",
        takes_value: true,
        value_hint: "imageName",
        detail: "Image to display in the button.",
        dialects: None,
    },
    OptionSpec {
        name: "-compound",
        takes_value: true,
        value_hint: "compoundType",
        detail: "How to display image relative to text.",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "width",
        detail: "Desired width of the button.",
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
    OptionSpec {
        name: "-padding",
        takes_value: true,
        value_hint: "padSpec",
        detail: "Internal padding around the widget content.",
        dialects: None,
    },
    OptionSpec {
        name: "-underline",
        takes_value: true,
        value_hint: "index",
        detail: "Index of the character to underline for mnemonic activation.",
        dialects: None,
    },
    OptionSpec {
        name: "-default",
        takes_value: true,
        value_hint: "defaultState",
        detail: "Default button state (normal, active, or disabled).",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ttk::button pathName ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::button",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed button widget.",
            synopsis: &["ttk::button pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_button.n",
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
