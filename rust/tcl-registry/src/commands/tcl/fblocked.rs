//! `fblocked` — test whether the last input operation exhausted all available input.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fblocked channel",
}];

/// Command spec for `fblocked`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fblocked",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Test whether the last input operation exhausted all available input.",
            &["fblocked channel"],
            "Tcl fblocked(1)",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
