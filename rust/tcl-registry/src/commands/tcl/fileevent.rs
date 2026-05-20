//! `fileevent` — execute a script when a channel becomes readable or writable.

use crate::prelude::*;

/// Command spec for `fileevent`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileevent",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(2, 3),
        arg_roles: &[(0, ArgRole::Channel), (2, ArgRole::Body)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Execute a script when a channel becomes readable or writable.",
            &[
                "fileevent channel readable ?script?",
                "fileevent channel writable ?script?",
            ],
            "Tcl fileevent(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
