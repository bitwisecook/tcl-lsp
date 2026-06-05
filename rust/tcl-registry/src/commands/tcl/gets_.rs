//! `gets` — read a line from a channel.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "gets channel ?varName?",
}];

/// Command spec for `gets`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "gets",
        traits: Traits::BYTE_COMPILED | Traits::TAINT_SOURCE,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Channel), (1, ArgRole::VarWrite)],
        assigns_variable_at: Some(1),
        return_type: Some(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
            },
        ],
hover: Some(HoverSnippet {
    summary: "Read a line from a channel",
    synopsis: &["gets channel ?varName?"],
    snippet: "The gets command has been superceded by the chan gets command which supports the same syntax and options.",
    source: "Tcl man page gets.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
