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
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "ms",
        detail: "Milliseconds.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "s",
        detail: "Seconds.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "microseconds",
        detail: "Microseconds.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "milliseconds",
        detail: "Milliseconds.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "seconds",
        detail: "Seconds.",
        min_tcl: None,
        code: None,
    },
];

static SLEEP_MODE: [ArgValue; 2] = [
    ArgValue {
        value: "for",
        detail: "Sleep for a monotonic duration.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "until",
        detail: "Sleep until a wall-clock point.",
        min_tcl: None,
        code: None,
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
