//! `flush` — flush buffered output for a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "flush ?channelId?",
}];

/// Command spec for `flush`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "flush",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::BYTE_COMPILED,
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Flush buffered output for a channel",
            synopsis: &["flush channel"],
            snippet: "The flush command has been superceded by the chan flush command which supports the same syntax and options.",
            source: "Tcl man page flush.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
