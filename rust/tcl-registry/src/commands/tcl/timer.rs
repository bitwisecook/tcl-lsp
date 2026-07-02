//! `timer` — schedule scripts on the wall-clock or monotonic clock (Tcl 9.1).
//!
//! The monotonic clock (`timer in`) is the right base for timeouts / periodic
//! work; the wall clock (`timer at`) tracks calendar time.  A supplied script
//! yields a timer id that `timer cancel` removes.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "timer subcommand ?arg ...?",
}];

/// Unit spellings accepted by the `timer` subcommands.
static UNITS: [ArgValue; 6] = [
    ArgValue {
        value: "us",
        detail: "Microseconds.",
    },
    ArgValue {
        value: "ms",
        detail: "Milliseconds.",
    },
    ArgValue {
        value: "s",
        detail: "Seconds.",
    },
    ArgValue {
        value: "microseconds",
        detail: "Microseconds.",
    },
    ArgValue {
        value: "milliseconds",
        detail: "Milliseconds.",
    },
    ArgValue {
        value: "seconds",
        detail: "Seconds.",
    },
];

static SLEEP_MODE: [ArgValue; 2] = [
    ArgValue {
        value: "for",
        detail: "Sleep for a monotonic duration.",
    },
    ArgValue {
        value: "until",
        detail: "Sleep until a wall-clock point.",
    },
];

// `timer in delay unit script` / `timer at timepoint unit script`: unit at
// index 1 (after the subcommand word), script at index 2 (a structural body).
static UNIT_AT_1: [(u8, &[ArgValue]); 1] = [(1, &UNITS)];
static SCRIPT_AT_2: [(u8, ArgRole); 1] = [(2, ArgRole::Body)];
// `timer idle script`: script at index 0.
static SCRIPT_AT_0: [(u8, ArgRole); 1] = [(0, ArgRole::Body)];
// `timer sleep for|until time ?unit?`: mode at index 0, unit at index 2.
static SLEEP_VALUES: [(u8, &[ArgValue]); 2] = [(0, &SLEEP_MODE), (2, &UNITS)];

static SUBCOMMANDS: [SubCommand; 6] = [
    SubCommand {
        name: "in",
        arity: Arity::new(3, 3),
        detail: "Execute the script after a monotonic delay of delay time units; returns a timer id.",
        synopsis: "timer in delay unit script",
        arg_values: &UNIT_AT_1,
        arg_roles: &SCRIPT_AT_2,
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "at",
        arity: Arity::new(3, 3),
        detail: "Execute the script at a wall-clock timepoint expressed in time units; returns a timer id.",
        synopsis: "timer at timepoint unit script",
        arg_values: &UNIT_AT_1,
        arg_roles: &SCRIPT_AT_2,
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(0, 1),
        detail: "Return a list describing the given timer event, or the ids of all events when no id is given.",
        synopsis: "timer info ?id?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cancel",
        arity: Arity::new(1, 1),
        detail: "Cancel the timer event with the given id; a no-op when unknown.",
        synopsis: "timer cancel id",
        return_type: Some(TclType::String),
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "idle",
        arity: Arity::new(1, 1),
        detail: "Register a script to be evaluated at idle time; returns a timer id.",
        synopsis: "timer idle script",
        arg_roles: &SCRIPT_AT_0,
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sleep",
        arity: Arity::new(2, 3),
        detail: "Block for a monotonic duration (`timer sleep for time ?unit?`) or until a wall-clock point (`timer sleep until time ?unit?`).",
        synopsis: "timer sleep for|until time ?unit?",
        arg_values: &SLEEP_VALUES,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

static SIDE_EFFECTS: [SideEffect; 1] = [SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

/// Command spec for `timer` (Tcl 9.1).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "timer",
        // The subcommand bodies (`timer in … script`) are event callbacks, not
        // inline-able code.
        traits: Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL91),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        side_effects: &SIDE_EFFECTS,
        hover: Some(HoverSnippet::brief(
            "Execute a script at a wall-clock point or after a monotonic delay (Tcl 9.1).",
            &["timer subcommand ?arg ...?"],
            "Tcl man page timer.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
