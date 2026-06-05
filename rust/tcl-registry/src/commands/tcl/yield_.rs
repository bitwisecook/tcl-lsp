//! `yield` — yield a value from a coroutine.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "yield ?value?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "yield",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::new(0, 1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Yield a value from a coroutine.",
            &["yield ?value?"],
            "Tcl yield(1)",
        )),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
