//! `tk_messageBox` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-default",
        takes_value: true,
        value_hint: "buttonName",
        detail: "Name of the default button.",
        dialects: None,
    },
    OptionSpec {
        name: "-detail",
        takes_value: true,
        value_hint: "string",
        detail: "Supplemental message text.",
        dialects: None,
    },
    OptionSpec {
        name: "-icon",
        takes_value: true,
        value_hint: "iconImage",
        detail: "Icon to display (error, info, question, warning).",
        dialects: None,
    },
    OptionSpec {
        name: "-message",
        takes_value: true,
        value_hint: "string",
        detail: "Main message text to display.",
        dialects: None,
    },
    OptionSpec {
        name: "-parent",
        takes_value: true,
        value_hint: "window",
        detail: "Parent window for the dialogue.",
        dialects: None,
    },
    OptionSpec {
        name: "-title",
        takes_value: true,
        value_hint: "titleString",
        detail: "Title string for the dialogue window.",
        dialects: None,
    },
    OptionSpec {
        name: "-type",
        takes_value: true,
        value_hint: "predefinedType",
        detail: "Arrangement of buttons to display.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tk_messageBox ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_messageBox",
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Pop up a message window and wait for user response.",
            synopsis: &["tk_messageBox ?option value ...?"],
            snippet: "",
            source: "Tk man page tk_messageBox.n",
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
