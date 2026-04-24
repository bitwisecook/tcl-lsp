//! `clock` — time and date operations.
use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(1),
        detail: "Add duration to a time.",
        synopsis: "clock add timeVal ?count unit ...?",
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clicks",
        arity: Arity::new(0, 1),
        detail: "Return hi-res clock value.",
        synopsis: "clock clicks ?-option?",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "format",
        arity: Arity::at_least(1),
        detail: "Format a time value.",
        synopsis: "clock format timeVal ?-option value ...?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "microseconds",
        arity: Arity::exact(0),
        detail: "Return current time in microseconds.",
        synopsis: "clock microseconds",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "milliseconds",
        arity: Arity::exact(0),
        detail: "Return current time in milliseconds.",
        synopsis: "clock milliseconds",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(1),
        detail: "Parse a date/time string.",
        synopsis: "clock scan inputString ?-option value ...?",
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "seconds",
        arity: Arity::exact(0),
        detail: "Return current time in seconds.",
        synopsis: "clock seconds",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clock",
        traits: Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet::brief(
            "Time and date operations.",
            &["clock subcommand ?arg ...?"],
            "Tcl clock(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
