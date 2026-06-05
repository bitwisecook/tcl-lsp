//! `seek` — set the access position for a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "seek channelId offset ?origin?",
}];

/// Command spec for `seek`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "seek",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(2, 3),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Set the access position for a channel.",
            synopsis: &["seek channelId offset ?origin?"],
            snippet: "Default origin is `start`. Returns empty string.",
            source: "Tcl man page seek.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
