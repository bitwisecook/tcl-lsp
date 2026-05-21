//! `error` — generate an error.

use crate::prelude::*;

/// Command spec for `error`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "error",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::TERMINATES_BLOCK
            | Traits::NEEDS_START_CMD,
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Generate an error.",
            &["error message ?info? ?code?"],
            "Tcl error(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
