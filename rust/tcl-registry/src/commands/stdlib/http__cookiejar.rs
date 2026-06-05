//! `http::cookiejar` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::cookiejar",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create or configure an HTTP cookie jar (TclOO class).",
            &["http::cookiejar create name ?filename?"],
            "F5",
        )),
        required_package: Some("cookiejar"),
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
