//! `open` — open a file-based or command pipeline channel.

use crate::prelude::*;

/// Command spec for `open`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open",
        traits: Traits::BYTE_COMPILED | Traits::OPENS_CHANNEL,
        arity: Arity::new(1, 3),
        return_type: Some(TclType::Channel),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Open a file-based or command pipeline channel.",
            &[
                "open fileName",
                "open fileName access",
                "open fileName access permissions",
            ],
            "Tcl open(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
