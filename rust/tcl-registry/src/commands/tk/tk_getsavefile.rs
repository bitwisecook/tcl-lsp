//! `tk_getSaveFile` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-confirmoverwrite",
        takes_value: true,
        value_hint: "boolean",
        detail: "Prompt for confirmation if the file already exists.",
        dialects: None,
    },
    OptionSpec {
        name: "-defaultextension",
        takes_value: true,
        value_hint: "extension",
        detail: "Default extension to append if the user does not type one.",
        dialects: None,
    },
    OptionSpec {
        name: "-filetypes",
        takes_value: true,
        value_hint: "filePatternList",
        detail: "List of file type patterns to display in the filter.",
        dialects: None,
    },
    OptionSpec {
        name: "-initialdir",
        takes_value: true,
        value_hint: "dirName",
        detail: "Initial directory to display.",
        dialects: None,
    },
    OptionSpec {
        name: "-initialfile",
        takes_value: true,
        value_hint: "fileName",
        detail: "Initial file name to populate in the dialogue.",
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
        name: "-typevariable",
        takes_value: true,
        value_hint: "varName",
        detail: "Variable to store the selected file type.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tk_getSaveFile ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_getSaveFile",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Pop up a dialogue for the user to select a file to save.",
            &["tk_getSaveFile ?option value ...?"],
            "F5",
        )),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
