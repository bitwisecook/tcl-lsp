//! `fcopy` — copy data from one channel to another.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fcopy inputChan outputChan ?-size size? ?-command callback?",
}];

/// Command spec for `fcopy`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fcopy",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::Channel)],
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Copy data from one channel to another",
            synopsis: &["fcopy inputChan outputChan ?-size size? ?-command callback?"],
            snippet: "The fcopy command copies data from one I/O channel, inchan, to another I/O channel, outchan.",
            source: "Tcl man page fcopy.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
