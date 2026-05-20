//! `continue` — skip to the next iteration of a loop.

use crate::prelude::*;

/// Command spec for `continue`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "continue",
        traits: Traits::BYTE_COMPILED | Traits::LANGUAGE_KEYWORD | Traits::NEEDS_START_CMD,
        arity: Arity::exact(0),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Skip to the next iteration of a loop.",
            &["continue"],
            "Tcl continue(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
