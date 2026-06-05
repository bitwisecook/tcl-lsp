//! `platform::identify` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::identify",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Return the platform identifier for the current machine.",
            &["platform::identify"],
            "F5",
        )),
        required_package: Some("platform"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
