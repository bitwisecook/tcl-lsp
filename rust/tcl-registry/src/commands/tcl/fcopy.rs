//! `fcopy` — copy data from one channel to another.

use crate::prelude::*;

/// Command spec for `fcopy`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fcopy",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::Channel)],
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Copy data from one channel to another.",
            &["fcopy inputChan outputChan ?-size size? ?-command callback?"],
            "Tcl fcopy(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
