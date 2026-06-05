//! `read` — read from a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "read ?-nonewline? channel",
}];

/// Command spec for `read`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read",
        traits: Traits::BYTE_COMPILED | Traits::TAINT_SOURCE,
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
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
