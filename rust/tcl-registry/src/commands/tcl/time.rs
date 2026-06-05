//! `time` — measure script execution time.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "time script ?count?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "time",
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Body)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Time the execution of a script.",
            &["time script ?count?"],
            "Tcl time(1)",
        )),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
