//! `tailcall` — replace the current procedure with another command.

use crate::prelude::*;

/// Command spec for `tailcall`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tailcall",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Replace the current procedure with another command.",
            &["tailcall command ?arg ...?"],
            "Tcl tailcall(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
