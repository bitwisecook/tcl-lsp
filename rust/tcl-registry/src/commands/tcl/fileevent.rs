//! `fileevent` — execute a script when a channel becomes readable or writable.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileevent channel readable ?script?",
}];

/// Command spec for `fileevent`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileevent",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::new(2, 3),
        arg_roles: &[(0, ArgRole::Channel), (2, ArgRole::Body)],
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Execute a script when a channel becomes readable or writable",
            synopsis: &[
                "fileevent channel readable ?script?",
                "fileevent channel writable ?script?",
            ],
            snippet: "The fileevent command has been superseded by the chan event command which supports the same syntax and options.",
            source: "Tcl man page fileevent.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
