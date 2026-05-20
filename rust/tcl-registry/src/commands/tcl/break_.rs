//! `break` — abort looping command.

use crate::prelude::*;

/// Command spec for `break`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "break",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::NEEDS_START_CMD,
        arity: Arity::exact(0),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Abort looping command.",
            &["break"],
            "Tcl break(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
