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

//! `time` — measure the execution time of a script.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

/// `time`'s script can do literally anything once evaluated — the same
/// "unknowable statically" placeholder `eval`/`apply` declare for their own
/// body arguments. `reads`/`writes` stay `true` rather than `eval`/
/// `apply`'s `false`/`false` to match the sibling body-evaluating
/// constructs that keep an unknown-but-present effect (`if`, `while`,
/// `for`, `catch`, `subst`) instead of a no-effect placeholder — `time`'s
/// two-argument shape (a fixed script plus a plain integer count, never a
/// concatenated variadic tail) groups it with that set rather than with
/// `eval`/`uplevel`'s "the whole call is dynamic code" shape (see the
/// `traits` field on `spec` below).
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// `time script ?count?` is `time`'s only form, unchanged in SYNOPSIS,
/// arity, and argument handling across Tcl 8.4 through 9.1 — see the
/// `spec` doc comment below for the version research this reflects.
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "time script ?count?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `time`.
///
/// `time script ?count?` and its argument handling are unchanged across
/// Tcl 8.4, 8.5, 8.6, 9.0, and 9.1 (all five manpages fetched and
/// compared directly): `Tcl_TimeObjCmd` (`generic/tclCmdMZ.c`) is
/// structurally the same function in all five source trees — `objc == 2`
/// defaults count to 1, `objc == 3` parses count as an integer, any other
/// argument count is a wrong-#-args error, and a script that completes
/// with anything other than `TCL_OK` on any iteration is returned
/// immediately instead of continuing. No option, flag, or second form was
/// ever added, removed, or version-gated. The only cross-version
/// differences are two internal-behaviour changes in Tcl 9.1 (count's
/// accepted integer width, and the clock source used to measure elapsed
/// time) with no SYNOPSIS-visible trace — documented as prose in the
/// hover snippet below rather than as a dialect gate here, since the
/// command's own syntax is unchanged.
///
/// `surface: Some(SpecSurface::ALL_TCL)` is deliberate, not an oversight:
/// `time` is one of the K36322151 commands F5 iRules excludes — unlike
/// `exec`/`open`/`socket`'s filesystem-and-process rationale or
/// `vwait`/`update`'s event-loop one, no equivalent documented reason carries
/// over to a pure CPU-timing command, so only the verified fact of the ban is
/// recorded here, not an invented mechanism. Under the explicit-per-spec model
/// that exclusion is carried by this very surface: `ALL_TCL` spans every core
/// Tcl version but not an iRules row, so the spec never intersects iRules'
/// bare `IRULES` mask and `time` is unreachable there — there is no
/// subtractive disable list. The `irules_banned_commands_never_resolve`
/// contract test (`tcl-registry/tests/dialect_profile.rs`) pins that `time`
/// does not resolve under f5-irules, exactly as `exec_.rs`/`pid.rs` document
/// for their own banned entries. (Its Tcl 8.6+ sibling `timerate` is likewise
/// absent from iRules, but through its `TCL86_PLUS` version gate rather than a
/// ban — see `timerate.rs`.) No other modelled dialect (Expect, f5-iapps,
/// f5-tmsh, the EDA vendor shells, Tk, incr Tcl) disables or alters `time`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "time",
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::BYTE_COMPILED
            // script is evaluated by a runtime call back into the
            // interpreter (`Tcl_EvalObjEx`, every version — see `FORMS`
            // above) directly in the caller's own frame: confirmed
            // empirically that `time {set x 99}` mutates the calling
            // proc's local `x`, and that `info level` read from inside
            // script is unchanged from outside — unlike `apply`/
            // `uplevel`, which move to a different frame. A local in the
            // enclosing proc must therefore stay reachable by name across
            // a `time` call exactly as across `eval`, since a non-literal
            // script (`time $s`) can reference or set any of them.
            // Deliberately *not* EVALUATES_CODE/TAINT_SINK/
            // CREATES_BARRIER (see `eval_.rs`/`apply.rs`): those drive
            // the T100/W101/W309 code-execution sink checks over *every*
            // argument — correct for `eval`'s concatenate-the-whole-tail
            // shape, but wrong here, since `time`'s second argument is
            // strictly a plain integer count, never re-parsed as code,
            // and its script argument is — like `if`/`while`/`catch`'s
            // body — idiomatically always a literal, not untrusted
            // input.
            | Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::new(1, 2),
        arg_roles: &[(0, ArgRole::Body)],
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Time the execution of a script.",
            synopsis: &["time script ?count?"],
            snippet: "Evaluates script through the Tcl interpreter count times, or once if count is omitted, and returns the average elapsed time per iteration as a string of the form \"N microseconds per iteration\" — actually a well-formed four-element Tcl list, so `lindex [time script] 0` is the idiomatic way to pull out just the number. N is a plain integer for a count of 1 (the default) or less — 0 specifically when count is 0 or negative, in which case script is not evaluated at all — and a floating-point average (total elapsed time divided by count), always shown with a decimal point, for a count of 2 or more. count itself must be a literal Tcl integer; a value like 2.0 is rejected. Time is measured as elapsed (wall-clock) time, not CPU time, so the result includes time spent blocked or pre-empted.\n\nscript runs in the caller's own variable scope, exactly like an if or while body, not a fresh call frame the way apply or uplevel use — so it can read and set the caller's local variables directly. If script does not complete normally on any iteration — an error, or a break, continue, or return it triggers — time stops immediately and that exceptional completion becomes time's own result instead of a timing string. break and continue are not scoped to time's own repeat count: unlike a real for or while loop, they are not absorbed by time at all, and propagate straight out of it to whatever loop or proc actually encloses the call.\n\nTcl 9.1 makes two internal changes with no effect on the syntax above: count's accepted range widens from a native 32-bit integer to a full 64-bit integer, and, on most platforms, elapsed time is measured with a monotonic clock instead of the wall clock, so a mid-run system clock adjustment can no longer skew the result.",
            source: "Tcl time(n)",
            examples: "time {expr {2 + 2}}\n\ntime {expr {2 + 2}} 100000\n\n# The result is really a 4-element list; lindex pulls out just the number\nset avgUs [lindex [time {someExpensiveCall} 100] 0]\n\n# A count of 0 (or less) never evaluates script at all\ntime {error \"never runs\"} 0",
            return_value: "A string of the form \"N microseconds per iteration\" — really a well-formed four-element Tcl list. N is a plain integer (0 when count is 0 or negative, since script is then never evaluated) for a count of 1 or less, and a floating-point average — total elapsed time divided by count — for a count of 2 or more. If script does not complete normally on some iteration, time returns that exceptional completion instead: its own error, or whatever break/continue/return/custom code script raised.",
        }),
        forms: FORMS,
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..CommandSpec::DEFAULT
    }
}
