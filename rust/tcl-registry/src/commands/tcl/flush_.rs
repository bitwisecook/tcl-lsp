//! `flush` — flush buffered output for a channel.

use crate::prelude::*;

/// Command spec for `flush`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "flush",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Flush buffered output for a channel.",
            &["flush channel"],
            "Tcl flush(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
