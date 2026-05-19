//! `clock` — time and date operations.
use crate::prelude::*;

/// Options for `clock scan` / `clock format` / `clock add`.
/// `-validate` is Tcl 9.0+ (TIP 532) and dialect-gated; the
/// others exist since Tcl 8.5.
static SCAN_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-base",
        takes_value: true,
        value_hint: "seconds",
        detail: "Base date/time used for partial input.",
        dialects: None,
    },
    OptionSpec {
        name: "-format",
        takes_value: true,
        value_hint: "format",
        detail: "Explicit format string (defaults to free-form parser).",
        dialects: None,
    },
    OptionSpec {
        name: "-gmt",
        takes_value: true,
        value_hint: "boolean",
        detail: "Use UTC instead of local time.",
        dialects: None,
    },
    OptionSpec {
        name: "-locale",
        takes_value: true,
        value_hint: "locale",
        detail: "Locale for month / day-of-week names.",
        dialects: None,
    },
    OptionSpec {
        name: "-timezone",
        takes_value: true,
        value_hint: "tz",
        detail: "Time zone for interpretation.",
        dialects: None,
    },
    // `-validate` is Tcl 9.0+ (TIP 532).
    OptionSpec {
        name: "-validate",
        takes_value: true,
        value_hint: "boolean",
        detail: "Validate the input date/time strictly (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90),
    },
];

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
        options: SCAN_OPTIONS,
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
