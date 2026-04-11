//! `throw` — generate a machine-readable error.

use crate::prelude::*;

/// Command spec for `throw`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "throw",
        traits: Traits::LANGUAGE_KEYWORD | Traits::TERMINATES_BLOCK,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::exact(2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Generate a machine-readable error.",
            &["throw type message"],
            "Tcl throw(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
