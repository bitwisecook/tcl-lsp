//! `bell` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        takes_value: true,
        value_hint: "window",
        detail: "Specifies the display on which to ring the bell.",
        dialects: None,
    },
    OptionSpec {
        name: "-nice",
        takes_value: false,
        value_hint: "",
        detail: "Do not reset the screen saver when ringing the bell.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "bell ?-displayof window? ?-nice?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bell",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Ring the display's bell.",
            synopsis: &["bell ?-displayof window? ?-nice?"],
            snippet: "",
            source: "Tk man page bell.n",
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
