//! `coroutine` — create a coroutine.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "coroutine name command ?arg...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroutine",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Create a coroutine.",
            &["coroutine name command ?arg ...?"],
            "Tcl coroutine(1)",
        )),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
