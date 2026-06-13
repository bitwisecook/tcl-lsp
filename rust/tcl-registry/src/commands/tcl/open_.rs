//! `open` — open a file-based or command pipeline channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "open fileName ?access? ?permissions?",
}];

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
        hover: Some(HoverSnippet {
            summary: "Open a file-based or command pipeline channel.",
            synopsis: &[
                "open fileName",
                "open fileName access",
                "open fileName access permissions",
            ],
            snippet: "Returns a channel identifier for use with `read`, `puts`, `close`, etc.",
            source: "Tcl man page open.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
