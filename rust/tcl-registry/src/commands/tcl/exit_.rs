//! `exit` — end the application.

use crate::prelude::*;

/// Command spec for `exit`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exit",
        traits: Traits::TERMINATES_BLOCK,
        arity: Arity::new(0, 1),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "End the application.",
            &["exit ?returnCode?"],
            "Tcl exit(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
