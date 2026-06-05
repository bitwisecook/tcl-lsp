//! `lower` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lower window ?belowThis?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lower",
        dialects: Some(DialectSet::TK),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Lower a window's position in the stacking order.",
            &["lower window ?belowThis?"],
            "F5",
        )),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
