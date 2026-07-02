//! `ttk::combobox` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-textvariable",
        takes_value: true,
        value_hint: "varName",
        detail: "Variable linked to the current combobox value.",
        dialects: None,
    },
    OptionSpec {
        name: "-values",
        takes_value: true,
        value_hint: "valueList",
        detail: "List of values to display in the drop-down list.",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "width",
        detail: "Desired width of the combobox in characters.",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "height",
        detail: "Maximum number of rows in the drop-down listbox.",
        dialects: None,
    },
    OptionSpec {
        name: "-state",
        takes_value: true,
        value_hint: "stateSpec",
        detail: "Widget state (normal, readonly, or disabled).",
        dialects: None,
    },
    OptionSpec {
        name: "-postcommand",
        takes_value: true,
        value_hint: "script",
        detail: "Script to evaluate just before displaying the drop-down list.",
        dialects: None,
    },
    OptionSpec {
        name: "-validate",
        takes_value: true,
        value_hint: "validateMode",
        detail: "When to run validation (none, focus, focusin, focusout, key, all).",
        dialects: None,
    },
    OptionSpec {
        name: "-validatecommand",
        takes_value: true,
        value_hint: "script",
        detail: "Script to evaluate for input validation.",
        dialects: None,
    },
    OptionSpec {
        name: "-invalidcommand",
        takes_value: true,
        value_hint: "script",
        detail: "Script to evaluate when validation fails.",
        dialects: None,
    },
    OptionSpec {
        name: "-xscrollcommand",
        takes_value: true,
        value_hint: "script",
        detail: "Command prefix for horizontal scroll communication.",
        dialects: None,
    },
    OptionSpec {
        name: "-exportselection",
        takes_value: true,
        value_hint: "boolean",
        detail: "Whether the selection is exported to the X selection.",
        dialects: None,
    },
    OptionSpec {
        name: "-justify",
        takes_value: true,
        value_hint: "justification",
        detail: "How to justify the text within the combobox.",
        dialects: None,
    },
    OptionSpec {
        name: "-font",
        takes_value: true,
        value_hint: "font",
        detail: "Font to use for the combobox text.",
        dialects: None,
    },
    OptionSpec {
        name: "-foreground",
        takes_value: true,
        value_hint: "colour",
        detail: "Foreground colour for the combobox text.",
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
    synopsis: "ttk::combobox pathName ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::combobox",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed combobox widget.",
            synopsis: &["ttk::combobox pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_combobox.n",
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
