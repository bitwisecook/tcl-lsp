//! `focus` command.
use crate::prelude::*;
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
        detail: "Return the focus window on the display of the given window.",
        dialects: None,
    },
    OptionSpec {
        name: "-force",
        takes_value: true,
        value_hint: "window",
        detail: "Set the focus to the window even if the application does not currently have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-lastfor",
        takes_value: true,
        value_hint: "window",
        detail: "Return the name of the most recent window to have the input focus among the window's top-level.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "focus ?option? ?window?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "focus",
        dialects: Some(DialectSet::TK),
        arity: Arity::new(0, 2),
        hover: Some(HoverSnippet {
            summary: "Manage the input focus.",
            synopsis: &[
                "focus",
                "focus window",
                "focus -displayof window",
                "focus -force window",
                "focus -lastfor window",
            ],
            snippet: "",
            source: "Tk man page focus.n",
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
