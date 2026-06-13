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
hover: Some(HoverSnippet {
    summary: "Read from a channel",
    synopsis: &["read ?-nonewline? channel", "read channel numChars"],
    snippet: "The read command has been superseded by the chan read command which supports the same syntax and options.",
    source: "Tcl man page read.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
