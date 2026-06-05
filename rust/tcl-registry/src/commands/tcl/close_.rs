//! `close` — close an open channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "close channelId ?r(ead)|w(rite)?",
}];

/// Command spec for `close`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Close an open channel.",
            &["close channelId ?r(ead)|w(rite)?"],
            "Tcl close(1)",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
