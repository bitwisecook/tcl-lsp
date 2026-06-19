//! `tell` — return current access position for a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tell channel",
}];

/// Command spec for `tell`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tell",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
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
        hover: Some(HoverSnippet {
            summary: "Return current access position for an open channel",
            synopsis: &["tell channel"],
            snippet: "The tell command has been superceded by the chan tell command which supports the same syntax and options.",
            source: "Tcl man page tell.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
