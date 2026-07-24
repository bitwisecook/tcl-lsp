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

//! `after` — execute a command after a time delay.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "after ms ?script script script ...?",
    dialects: None,
}];

/// Mark the sole script word of `after ms script` as [`ArgRole::Body`], so
/// a bareword callback (`after 1000 myProc`) is recursed as a real command
/// invocation — same-file arity checking then sees the `myProc` call
/// exactly as it would inside any other script, rather than treating it as
/// an opaque, unchecked value (matching `fileevent` / `chan event`'s static
/// `(2, ArgRole::Body)` marking). Only reached for the *default*
/// numeric-delay form: [`CommandRegistry::arg_indices_for_role`] tries
/// subcommand resolution first, so `after cancel …` / `after idle …` /
/// `after info …` never call this resolver at all.
///
/// `args[0]` is the delay (`after`'s own arity requires it). Marks a script
/// word only when it is the **sole** trailing word (`args.len() == 2`):
/// `after ms script script script ...?` concatenates every trailing word
/// together (like `concat`) before evaluating the result as one script, so
/// `after 1000 {cb} 1 2` really runs `cb 1 2`, not `cb` alone — with more
/// than one trailing word, marking just the first as `Body` would recurse
/// into a truncated, wrongly-arity-checked fragment of the real script
/// (confirmed against tclsh 9.0.4: `after info` shows the registered script
/// as the space-joined concatenation). Abstain in that case rather than
/// model the concatenation — a single braced script is by far the
/// idiomatic form.
fn after_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() == 2 {
        vec![(1, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

/// Same concatenation-aware guard as [`after_arg_roles`], for `after idle
/// script ?script script ...?`. `args` here is already the subcommand's
/// own slice (the word after `idle`), so the sole script word is at index
/// 0 — marked as `Body` only when it is the only trailing word.
fn after_idle_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() == 1 {
        vec![(0, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "cancel",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::at_least(1),
        detail: "Cancel a previously scheduled after ms or after idle handler (from Tcl 9.1, also a timer handler), matched either by the id returned when it was scheduled or by the exact concatenated script text used to schedule it. A no-op if the handler has already run or does not exist.",
        synopsis: "after cancel id|script ?script script ...?",
        return_type: Some(TclType::String),
        mutator: true,
        // `Tcl_AfterObjCmd` (tclTimer.c, `AFTER_CANCEL` arm) removes the
        // scheduled timer/idle handler — the destroyed handler cannot be
        // re-cancelled, which is why `catch {after cancel $id}` is the
        // documented fire-and-forget idiom the W302 suppression keys off.
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "idle",
        arity: Arity::at_least(1),
        // Same concatenation shape as the default `after ms script` form
        // (`after_arg_roles`) — mark the sole script word only when it is
        // the only one present, abstaining rather than mis-recursing a
        // truncated fragment when several words concatenate together.
        arg_role_resolver: Some(after_idle_arg_roles),
        // The idle script runs later, at global level outside the context
        // of any Tcl procedure (Tcl man page after.n) — confirmed
        // empirically: a local variable set before `after idle {…; set x
        // 1}` is invisible inside the deferred script, and `info level`
        // there reads 0. SSA must not treat it as sharing the caller's
        // frame.
        body_kind: BodyKind::Structural,
        detail: "Concatenate the script arguments (as concat would) and arrange for the result to run exactly once, the next time the event loop is entered with no other events pending. Runs at global level, outside the context of any Tcl procedure.",
        synopsis: "after idle script ?script script ...?",
        return_type: Some(TclType::String),
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(0, 1),
        detail: "Without an id, return a list of the ids of every pending after/idle handler. With an id, return a two-element list of the handler's script and its type, idle or timer (from Tcl 9.1, both after- and timer-command handlers report as timer; use timer info to distinguish them).",
        synopsis: "after info ?id?",
        return_type: Some(TclType::List),
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `after`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "after",
        dialects: Some(DialectSet::ALL_TCL),
        // The `cancel` subform destroys a scheduled
        // handler (`Tcl_AfterObjCmd`, tclTimer.c) — see the `destructive`
        // flag on the `cancel` subcommand.
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        arg_role_resolver: Some(after_arg_roles),
        // The default form's deferred script runs later, at global level
        // outside the context of any Tcl procedure (Tcl man page after.n:
        // "The command will be executed at global level (outside the
        // context of any Tcl procedure)") — confirmed empirically: a bare
        // variable reference inside the script does not see the
        // registering proc's locals when it fires, any more than a
        // `TclOO` method's per-invocation `my`/`next` survive past that
        // method's own call (same reasoning as `idle`'s `body_kind`
        // comment and `uplevel`'s). Neither SSA dataflow nor `my`/`next`/
        // `$obj method` dispatch scanning must treat it as same-frame.
        body_kind: BodyKind::Structural,
        subcommands: SUBCOMMANDS,
        // `after 200 …` — an integer first word selects the default
        // delayed-execution form rather than dispatching on a subcommand.
        default_form_first_word: Some(DefaultFormFirstWord::Integer),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Delay execution, or schedule a script to run later.",
            synopsis: &[
                "after ms",
                "after ms ?script script script ...?",
                "after cancel id",
                "after cancel script script script ...",
                "after idle ?script script script ...?",
                "after info ?id?",
            ],
            snippet: "With ms alone, blocks the caller for ms milliseconds; the application does not respond to events meanwhile. With one or more script arguments, returns immediately and concatenates them (as concat would) into a command scheduled to run exactly once, ms milliseconds later, as a global-level event handler outside the context of any Tcl procedure — variables referenced inside it resolve against the global namespace, not the caller's local scope. A negative ms is treated as 0 (documented from Tcl 8.6 on; the 8.4/8.5 manpages are silent on the clamp). For the scheduling form specifically, a ms of zero or negative queues the event immediately, ahead of other pending events, unless after itself runs from within an event handler, in which case it waits for the next pass through the event loop (also documented from Tcl 8.6 on). after idle schedules the same way but runs at the next idle point — the next time the event loop is entered with no other events pending — instead of after a delay. Both forms return an identifier usable with after cancel. after cancel removes a still-pending timer or idle handler, matched either by that identifier (from Tcl 9.1, also an id returned by the timer command) or by the exact concatenated script text; canceling a handler that has already fired or does not exist is a silent no-op. after info lists the ids of every handler currently pending, or, given an id, returns a two-element list of that handler's script and its type (idle or timer; from Tcl 9.1, both after- and timer-scheduled events report as timer — use timer info to tell them apart). An error raised while a deferred script runs is reported through interp bgerror (Tcl 8.5+; via the global bgerror proc directly in Tcl 8.4, which predates the interp bgerror subcommand) rather than propagating to the caller. Delayed commands only fire once the event loop is entered — vwait or update is needed to process them in a non-event-driven application such as tclsh.",
            source: "Tcl after(n)",
            examples: "set id [after 5000 {puts \"5 seconds elapsed\"}]\nafter cancel $id\n\nafter idle {puts \"runs once the event loop goes idle\"}\n\nafter info",
            return_value: "An empty string for the blocking after ms and after cancel forms; an identifier for after ms script and after idle; a list of ids, or a two-element {script type} list, for after info.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
