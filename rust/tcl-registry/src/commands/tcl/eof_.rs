//! `eof` — check for end-of-file condition on a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "eof channel",
}];

/// Command spec for `eof`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "eof",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Check for end of file condition on channel",
            synopsis: &["eof channel"],
            snippet: "The eof command has been superceded by the chan eof command which supports the same syntax and options.",
            source: "Tcl man page eof.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
