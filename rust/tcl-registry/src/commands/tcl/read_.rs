//! `read` — read from a channel.

use crate::prelude::*;

/// Command spec for `read`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read",
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Read from a channel.",
            &["read ?-nonewline? channel", "read channel numChars"],
            "Tcl read(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
