//! `update` — process pending events.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "update ?idletasks?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "update",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Process pending events and idle callbacks.",
            &["update ?idletasks?"],
            "Tcl update(1)",
        )),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
