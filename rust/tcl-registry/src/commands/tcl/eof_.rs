//! `eof` — check for end-of-file condition on a channel.

use crate::prelude::*;

/// Command spec for `eof`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "eof",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Check for end-of-file condition on a channel.",
            &["eof channel"],
            "Tcl eof(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
