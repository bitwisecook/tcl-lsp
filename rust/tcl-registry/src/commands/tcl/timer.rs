// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `timer` — schedule scripts on the wall-clock or monotonic clock (Tcl 9.1).
//!
//! New in Tcl 9.1: `timer.n` exists only in the 9.1 `TclCmd` manpage tree —
//! the 8.4, 8.5, 8.6, and 9.0 trees were each fetched and individually
//! confirmed to have no such page, rather than assumed absent from the
//! 9.0/9.1 gap alone. `timer` shares `after`'s underlying event queue and
//! id namespace: `after cancel`/`after info` also see `timer`-created
//! handlers, and `timer cancel`/`timer info` likewise see `after`-created
//! ones (see `commands/tcl/after_.rs`, itself updated with this same
//! cross-reference). The monotonic clock (`timer in`, `timer sleep for`)
//! is the right base for timeouts / periodic work; the wall clock
//! (`timer at`, `timer sleep until`) tracks calendar time. A supplied
//! script yields a timer id that `timer cancel` (or `after cancel`)
//! removes.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "timer subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

/// Unit spellings accepted by the `timer` subcommands — `us`/`ms`/`s` or
/// their long forms, "or any unique abbreviation" of one of those six
/// words (doc/timer.n, TIME UNITS section). Modelled as a closed,
/// prefix-accepting set below (`closed_value_args` +
/// `arg_values_accept_prefix` on the subcommands that take a unit), the
/// same shape `string is`'s character-class argument uses.
static UNITS: [ArgValue; 6] = [
    ArgValue {
        value: "us",
        detail: "Microseconds.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "ms",
        detail: "Milliseconds.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "s",
        detail: "Seconds.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "microseconds",
        detail: "Microseconds.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "milliseconds",
        detail: "Milliseconds.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "seconds",
        detail: "Seconds.",
        ..ArgValue::DEFAULT
    },
];

/// `timer sleep`'s mode word (position 0, after the subcommand word).
/// Only these two spellings are documented; unlike `UNITS`, doc/timer.n
/// does not say an abbreviation is accepted here, so — absent that
/// confirmation — this stays completion/hover data only (no
/// `closed_value_args` entry for index 0 on the `sleep` `SubCommand`).
static SLEEP_MODE: [ArgValue; 2] = [
    ArgValue {
        value: "for",
        detail: "Sleep for a monotonic duration (delay). Unit defaults to milliseconds.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "until",
        detail: "Sleep until a wall-clock point (timepoint). Unit defaults to seconds.",
        ..ArgValue::DEFAULT
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

/// Two apparent authoring artifacts in the current (9.1b0) `timer.n`
/// manpage, noted here so a future re-fetch against a corrected manpage
/// can drop this comment:
///
/// - Its DESCRIPTION section's own `<dt>` headings misname three of the
///   seven subcommands — `timer wait for`, `timer wait until`, and
///   `after cancel id` — evidently copy-pasted from `after.n` and not
///   fully edited for the new page. The SYNOPSIS section above them, and
///   the explanatory prose *inside* each entry (which talks about "the
///   `timer cancel` command" and "a previous `timer` or `after`
///   command"), agree with each other on `sleep for` / `sleep until` /
///   `cancel` instead — that's what's modelled below, and it also matches
///   `timer_is_91_only_with_scheduler_side_effects`
///   (`tcl-registry/tests/tcl91_dialect.rs`), landed independently on the
///   same six subcommand names.
/// - Its TIME MAXIMUM VALUE section spells the boundary error "time to
///   far" (missing the second "o"); the fact is described in plain prose
///   in the hover snippet below instead of repeating that exact spelling
///   into a user-facing string.
static SUBCOMMANDS: [SubCommand; 6] = [
    SubCommand {
        name: "in",
        arity: Arity::new(3, 3),
        detail: "Schedule script to run once, after a monotonic delay of delay time units, as a global-level event handler; returns a timer id. A delay of 0 or less queues the event immediately, ahead of other pending events.",
        synopsis: "timer in delay unit script",
        arg_values: &UNIT_AT_1,
        // Unit is a closed set; doc/timer.n: "or any unique abbreviation"
        // — `Tcl_GetIndexFromObj`-style prefix matching, the same shape
        // `string is`'s class argument uses.
        closed_value_args: &[1],
        arg_values_accept_prefix: true,
        arg_roles: &SCRIPT_AT_2,
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "at",
        arity: Arity::new(3, 3),
        detail: "Schedule script to run once, at the wall-clock timepoint expressed in time units, as a global-level event handler; returns a timer id. Serviced after any monotonic-clock event due at the same instant.",
        synopsis: "timer at timepoint unit script",
        arg_values: &UNIT_AT_1,
        // Same unit shape as `in` — see its comment.
        closed_value_args: &[1],
        arg_values_accept_prefix: true,
        arg_roles: &SCRIPT_AT_2,
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(0, 1),
        detail: "With no id, return the ids of every pending timer/after handler. With an id, return a 2- or 3-element {script kind ?time?} list describing that handler (kind is idle, monotonic, or wallclock; time, present only for monotonic/wallclock, is the scheduled execution point in microseconds).",
        synopsis: "timer info ?id?",
        pure: true,
        return_type: Some(TclType::List),
        // Overrides the command-level write default: `timer info` only
        // queries scheduler state ("This command returns information
        // about existing event handlers") — the same correction
        // `after info` makes over `after`'s own command-level default
        // (see `commands/tcl/after_.rs`).
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cancel",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::new(1, 1),
        detail: "Cancel the still-pending timer or after handler with the given id; a no-op if it has already fired or the id was never valid.",
        synopsis: "timer cancel id",
        return_type: Some(TclType::String),
        mutator: true,
        // The cancelled handler cannot be re-cancelled, which is why
        // `catch {timer cancel $id}` is the documented fire-and-forget
        // idiom the W302 suppression keys off — same rationale as
        // `after cancel` (`commands/tcl/after_.rs`), which
        // `Traits::FIRE_AND_FORGET_TEARDOWN`'s own doc comment names as a
        // canonical member.
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "idle",
        arity: Arity::new(1, 1),
        detail: "Schedule script to run once, the next time the event loop is entered with no other events pending; returns a timer id.",
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
        detail: "Block the interpreter without servicing events — for a monotonic duration (`timer sleep for delay ?unit?`, unit defaults to milliseconds) or until a wall-clock point (`timer sleep until timepoint ?unit?`, unit defaults to seconds) — then return an empty string.",
        synopsis: "timer sleep for|until time ?unit?",
        arg_values: &SLEEP_VALUES,
        // Only the unit (index 2) is closed — see `UNITS`'s doc comment.
        // The for|until mode word (index 0) stays open; see
        // `SLEEP_MODE`'s doc comment.
        closed_value_args: &[2],
        arg_values_accept_prefix: true,
        return_type: Some(TclType::String),
        // Overrides the command-level write default: unlike `in`/`at`/
        // `idle`, `sleep` registers no handler and touches no
        // interpreter-visible state — it just blocks the calling thread
        // ("While the command is sleeping the application does not
        // respond to events").
        side_effects: &[SideEffect {
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

static SIDE_EFFECTS: [SideEffect; 1] = [SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// Command spec for `timer` (Tcl 9.1 only — confirmed absent from the
/// 8.4, 8.5, 8.6, and 9.0 `TclCmd` manpage trees, each fetched and
/// checked individually rather than assumed from a neighbouring
/// version).
///
/// `BYTE_COMPILED`, not `NEVER_INLINE_BODY`: `timer`'s scheduled-script
/// subcommands (`in`/`at`/`idle`) are a callback-registration shape, not
/// a block/loop/definition body — the same shape `after`'s `ms script`
/// and `idle script` forms use, and `after` itself carries
/// `BYTE_COMPILED` with no `NEVER_INLINE_BODY`
/// (`commands/tcl/after_.rs`). The formatter's `NEVER_INLINE_BODY`
/// (`tcl-lsp-core/src/formatting/engine.rs`) forces a body to always
/// print multi-line, which is right for `if`/`for`/`dict for`/`proc`
/// -style compound statements but wrong for a short deferred callback
/// such as `timer idle {incr x}`, idiomatically written on one line
/// exactly like `after 0 {incr x}` is. The original (pre-audit) spec
/// carried `NEVER_INLINE_BODY` alone, with neither `BYTE_COMPILED` nor
/// this reasoning; both are corrected here for consistency with the
/// closest sibling command.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "timer",
        // `DEFERS_BODY`, exactly as `after` carries it (`tcl/after_.rs`) —
        // this command shares `after`'s event queue and id namespace, as the
        // module header above records. No 9.1 interpreter exists here to
        // oracle, so the evidence is documentary and unambiguous:
        // `doc/timer.n` in the 9.1b0 tree says "The delayed command is given
        // by the argument script" for `timer in`/`timer at`, and for `timer
        // idle`, "Arranges for the script to be evaluated later as an idle
        // callback … the next time the event loop is entered". A scheduled
        // script cannot stop control reaching the caller's next statement
        // (issue #1672 audit).
        traits: Traits::BYTE_COMPILED.union(Traits::DEFERS_BODY),
        surface: Some(SpecSurface::TCL91),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        side_effects: &SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Schedule a script on the monotonic or wall clock, or block for an interval (Tcl 9.1).",
            synopsis: &[
                "timer in delay unit script",
                "timer at timepoint unit script",
                "timer idle script",
                "timer sleep for delay ?unit?",
                "timer sleep until timepoint ?unit?",
                "timer cancel id",
                "timer info ?id?",
            ],
            snippet: "timer is new in Tcl 9.1. It schedules or blocks on either the monotonic clock (in, sleep for) or the wall clock (at, sleep until), sharing after's underlying event queue and id namespace: after cancel and after info also recognise timer-created handlers, and timer cancel/timer info likewise recognise after-created ones — though the two info forms report differently, since after info's second list element is idle or timer while timer info's is idle, monotonic, or wallclock.\n\ntimer in delay unit script and timer at timepoint unit script both return immediately with a timer id and arrange for script to run exactly once, later, as an event handler at global level (outside the context of any Tcl procedure); an error raised while it runs is reported through interp bgerror rather than propagating to the caller. A delay of 0 or less queues a timer in event immediately, ahead of other pending events; a timer at event due at the same instant as a monotonic-clock event is serviced second. timer idle script works the same way but runs once, the next time the event loop is entered with no other events pending.\n\ntimer sleep for delay ?unit? and timer sleep until timepoint ?unit? block the caller instead of scheduling anything — the application does not respond to events while blocked — returning once the delay elapses or the timepoint is reached. unit defaults to milliseconds for sleep for and seconds for sleep until.\n\nTime units are us or microseconds, ms or milliseconds, and s or seconds, or any unique abbreviation of one of those words. delay and timepoint are both wide integers, converted internally to a 63-bit microsecond count; a value large enough to overflow that range (the manpage puts the boundary around the year 294441) is an error.\n\ntimer cancel id removes a still-pending handler — one created by timer or by after — identified by id; canceling a handler that has already fired, or an id that was never valid, is a silent no-op. timer info with no id lists the ids of every pending timer/after handler; given an id it returns a 2- or 3-element list {script kind ?time?}, where kind is idle, monotonic, or wallclock and the optional time (present only for monotonic/wallclock) is the handler's scheduled execution point, in microseconds.\n\nThe scheduled forms — in, at, and idle — only fire once the event loop is entered; a script that is not already event-driven (plain tclsh) must call vwait or update itself to process them.",
            source: "Tcl timer(n)",
            examples: "set id [timer in 5 s {puts \"5 seconds elapsed\"}]\ntimer cancel $id\n\ntimer at [clock add [clock seconds] 1 hours] seconds {puts \"wake up\"}\n\ntimer idle {puts \"runs once the event loop goes idle\"}\n\ntimer sleep for 500 ms\ntimer sleep until [clock add [clock seconds] 2 seconds]\n\ntimer info",
            return_value: "A timer id (an opaque string) for in, at, and idle; an empty string for sleep and cancel; a list of ids, or a 2- or 3-element {script kind ?time?} list, for info.",
        }),
        ..CommandSpec::DEFAULT
    }
}
