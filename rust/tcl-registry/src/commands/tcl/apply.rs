//! `apply` — apply an anonymous procedure.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::ProcDefinition,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "apply func ?arg1 arg2 ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "apply",
        traits: Traits::NOT_PROC_FACTORY | Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Body)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Apply an anonymous procedure.",
            &["apply func ?arg ...?"],
            "Tcl apply(1)",
        )),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
