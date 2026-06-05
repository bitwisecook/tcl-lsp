//! `pid` — return process ID(s).
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "pid ?fileId?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pid",
        traits: Traits::BYTE_COMPILED | Traits::PURE,
        arity: Arity::new(0, 1),
        return_type: Some(TclType::Int),
        hover: Some(HoverSnippet::brief(
            "Return process ID(s).",
            &["pid ?fileId?"],
            "Tcl pid(1)",
        )),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
