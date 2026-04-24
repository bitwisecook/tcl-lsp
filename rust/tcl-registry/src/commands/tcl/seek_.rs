//! `seek` — set the access position for a channel.

use crate::prelude::*;

/// Command spec for `seek`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "seek",
        arity: Arity::new(2, 3),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Set the access position for a channel.",
            &["seek channelId offset ?origin?"],
            "Tcl seek(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
