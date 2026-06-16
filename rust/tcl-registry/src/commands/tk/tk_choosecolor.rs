//! `tk_chooseColor` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-initialcolor",
        takes_value: true,
        value_hint: "colour",
        detail: "Initial colour to display in the chooser.",
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
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tk_chooseColor ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_chooseColor",
        dialects: None,
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Pop up a dialogue for the user to select a colour.",
            synopsis: &["tk_chooseColor ?option value ...?"],
            snippet: "",
            source: "Tk man page tk_chooseColor.n",
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
