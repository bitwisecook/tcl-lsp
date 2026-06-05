//! `safe::interpCreate` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "safe::interpCreate",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create a safe child interpreter with restricted capabilities.",
            &["safe::interpCreate ?child? ?options...?"],
            "F5",
        )),
        required_package: Some("safe"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
