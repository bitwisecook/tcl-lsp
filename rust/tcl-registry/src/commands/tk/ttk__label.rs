//! `ttk::label` command.
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
        detail: "Text to display in the label.",
        dialects: None,
    },
    OptionSpec {
        name: "-textvariable",
        takes_value: true,
        value_hint: "varName",
        detail: "Variable whose value is used as the label text.",
        dialects: None,
    },
    OptionSpec {
        name: "-image",
        takes_value: true,
        value_hint: "imageName",
        detail: "Image to display in the label.",
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
        detail: "Desired width of the label.",
        dialects: None,
    },
    OptionSpec {
        name: "-anchor",
        takes_value: true,
        value_hint: "anchorPos",
        detail: "How the text or image is positioned within the widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-justify",
        takes_value: true,
        value_hint: "justification",
        detail: "How to justify multiple lines of text.",
        dialects: None,
    },
    OptionSpec {
        name: "-wraplength",
        takes_value: true,
        value_hint: "length",
        detail: "Maximum line length for word wrapping.",
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
        name: "-relief",
        takes_value: true,
        value_hint: "relief",
        detail: "Border relief style for the label.",
        dialects: None,
    },
    OptionSpec {
        name: "-font",
        takes_value: true,
        value_hint: "font",
        detail: "Font to use for the label text.",
        dialects: None,
    },
    OptionSpec {
        name: "-foreground",
        takes_value: true,
        value_hint: "colour",
        detail: "Foreground colour for the label text.",
        dialects: None,
    },
    OptionSpec {
        name: "-background",
        takes_value: true,
        value_hint: "colour",
        detail: "Background colour for the label.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ttk::label pathName ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::label",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed label widget.",
            synopsis: &["ttk::label pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_label.n",
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
