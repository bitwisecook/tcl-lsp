//! `after` — execute a command after a time delay.

use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "cancel",
        arity: Arity::at_least(1),
        detail: "Cancel a previously scheduled delayed command.",
        synopsis: "after cancel id",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "idle",
        arity: Arity::at_least(1),
        detail: "Arrange for a script to be evaluated later as an idle callback.",
        synopsis: "after idle script ?script script ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(0, 1),
        detail: "Returns information about existing event handlers.",
        synopsis: "after info ?id?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `after`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "after",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Execute a command after a time delay.",
            &[
                "after ms",
                "after ms ?script script script ...?",
                "after cancel id",
                "after cancel script script script ...",
            ],
            "Tcl after(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
