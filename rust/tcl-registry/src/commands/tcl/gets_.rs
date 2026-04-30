//! `gets` — read a line from a channel.

use crate::prelude::*;

/// Command spec for `gets`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "gets",
        traits: Traits::TAINT_SOURCE,
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
        hover: Some(HoverSnippet::brief(
            "Read a line from a channel.",
            &["gets channel ?varName?"],
            "Tcl gets(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
