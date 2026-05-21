//! `tell` — return current access position for a channel.

use crate::prelude::*;

/// Command spec for `tell`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tell",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Return current access position for a channel.",
            &["tell channel"],
            "Tcl tell(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
