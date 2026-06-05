//! `raise` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "raise window ?aboveThis?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "raise",
        dialects: Some(DialectSet::TK),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Raise a window's position in the stacking order.",
            synopsis: &["raise window ?aboveThis?"],
            snippet: "",
            source: "Tk man page raise.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
