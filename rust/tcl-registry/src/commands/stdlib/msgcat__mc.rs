//! `msgcat::mc` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "msgcat::mc",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Translate a source string according to the current locale.",
            &["msgcat::mc src-string ?arg arg ...?"],
            "F5",
        )),
        required_package: Some("msgcat"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
