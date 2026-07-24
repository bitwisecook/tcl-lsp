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

//! `timerate` — measure the calibrated rate of execution of a script.

use crate::prelude::*;

// All three forms are unchanged, synopsis-for-synopsis, across Tcl 8.6,
// 9.0, and 9.1 (see the `spec` doc comment below) — mirroring the real
// manpage's three illustrative lines, per the `return_.rs` convention,
// even though the underlying option grammar is actually a flat,
// freely-combinable set (confirmed empirically; see below). Each entry's
// own `dialects: None` inherits the command-level `TCL86_PLUS` gate.
const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "timerate script ?time? ?max-count?",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "timerate ?-direct? ?-overhead estimate? script ?time? ?max-count?",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "timerate ?-calibrate? ?-direct? script ?time? ?max-count?",
        dialects: None,
    },
];

/// `timerate`'s side effects. The body's effect is unknowable statically
/// (mirrors `time.rs`'s own `SIDE_EFFECTS`, same reasoning: a two-argument
/// shape with a fixed script plus plain integers, not a concatenated
/// variadic tail, so it groups with `if`/`while`/`catch`'s "unknown but
/// present" effect rather than `eval`/`uplevel`'s "whole call is dynamic
/// code"). `-calibrate` additionally has a real, narrower effect beyond
/// the script itself: it measures and stores a fresh overhead estimate in
/// a process-level static (`static double measureOverhead`,
/// `Tcl_TimeRateObjCmd`, `generic/tclCmdMZ.c` — identical in 8.6/9.0/9.1),
/// which every later `timerate` call without an explicit `-overhead` then
/// reads back (`if (overhead == -1) { overhead = measureOverhead; }`) —
/// confirmed directly in the C source, and independently corroborated by
/// the manpage's own "calibration is not thread safe" warning, which only
/// makes sense for state that outlives one call. `InterpState` is the
/// closest existing `SideEffectTarget` for this kind of persistent
/// interpreter/process-level knob (see `return.rs`'s own `InterpState`
/// entry for its `-code`/`-level` completion state); `reads`/`writes` are
/// both `true` since which one happens depends on the call's own options
/// (a bare call reads the stored value, `-calibrate` writes it, `-overhead
/// <val>` touches neither) — the same "declare the union, since the
/// static spec can't see which invocation shape is used" convention
/// `open_.rs`'s two-entry `side_effects` already follows.
const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        target: SideEffectTarget::Unknown,
        reads: true,
        writes: true,
        connection_side: ConnectionSide::None,
        dialects: None,
    },
    SideEffect {
        target: SideEffectTarget::InterpState,
        reads: true,
        writes: true,
        connection_side: ConnectionSide::None,
        dialects: None,
    },
];

/// Command spec for `timerate`.
///
/// **Version gate.** Added in Tcl 8.6 (TIP 527, "New measurement
/// facilities in Tcl: New command `timerate`"). Absent from 8.4 and 8.5:
/// `tcl-lang.org/man/tcl8.{4,5}/TclCmd/timerate.html` both serve a "URL
/// Not Found" body (HTTP 200 — the status alone doesn't show it), and
/// `generic/tclBasic.c` at the `core-8-4-20` / `core-8-5-19` release tags
/// has no `timerate` entry at all (`time` is present in both, so this
/// isn't a fetch fluke). `dialects: Some(DialectSet::TCL86_PLUS)` below
/// is therefore correct — narrower than the sibling `time`'s genuine
/// `dialects: Some(DialectSet::ALL_TCL)`.
///
/// In every stock 8.6 build the real command is
/// `::tcl::unsupported::timerate` (`core-8-6-16`'s `tclBasic.c`); a
/// direct `builtInCmds` row exists too but only under `#ifdef
/// TCL_TIMERATE`, which no standard build defines. The bare `timerate`
/// spelling still works, because `library/tclIndex` auto-imports it into
/// the calling namespace on first unqualified use (confirmed live: a
/// fresh `tclsh` 8.6.14 has no `::timerate` until the name is first
/// referenced) — transparent to callers, so it changes nothing about the
/// arity, options, or return shape modelled here. From Tcl 9.0
/// (`core-9-0-4`/`core-9-1-b0`) `timerate` is instead a direct,
/// unconditional `builtInCmds` row, matching TIP 527's own stated plan
/// ("makes the TIP formally for 8.7" — the stream that became 9.0).
/// `CMD_IS_SAFE` holds either way, hence no `Traits::SAFE_INTERP_HIDDEN`
/// here — `timerate` is not on that trait's own enumerated unsafe-core
/// list.
///
/// Tcl 8.6.18's, 9.0.4's, and 9.1b0's manpages (fetched and diffed
/// directly; 9.0.4 vs 9.1b0 differ only in the version banner) are the
/// same three-form command throughout. The only real 8.6→9.0 delta: the
/// `-overhead` placeholder renames `double` → `estimate`, and its
/// parenthetical gains "which may be a floating point number" — the
/// value always accepted a float (`Tcl_GetDoubleFromObj`, every version),
/// so this is a documentation clarification, not new behaviour. Two
/// claims previously attached to this spec do not survive a direct fetch
/// and are corrected below: that 8.6 described `-overhead` as a value
/// "subtracted from the measured time" (that phrase is the original 2018
/// TIP 527 *proposal* text, `core.tcl-lang.org/tips/doc/trunk/tip/527.md`
/// — a design draft; the shipped 8.6 manpage already reads "overrides …
/// the overhead estimated by a previous calibration", word-for-word what
/// 9.0/9.1 say), and that 9.0 newly added the `-overhead 0` worked
/// example (it is already present, verbatim, in 8.6.18).
///
/// **Options.** `-direct`/`-calibrate`/`-overhead` are unchanged across
/// 8.6/9.0/9.1 — confirmed live and by diffing `tclCmdMZ.c`'s
/// option-scanning loop across all three source trees (only
/// compiler-hygiene renames and equivalent arithmetic rewrites, no
/// behavioural change). The three flags combine freely in any order; the
/// live `wrong # args` message is one collapsed `?-direct? ?-calibrate?
/// ?-overhead double? command ?time ?max-count??` form in every version
/// (it still says `double`/`command` even under 9.0/9.1, where the
/// manpage itself already reads `estimate`/`script` — the internal usage
/// string was never updated to match). Flag matching is `TCL_EXACT`: no
/// abbreviation is accepted, and an unrecognised `-`-looking word is
/// never reported as a bad option either — the scan just stops at the
/// first unmatched word, which is then taken as `script` itself, so
/// `timerate -claibrate {…} 10` silently measures the script
/// `-claibrate` rather than raising a usage error on the typo. A bare
/// `--` is also recognised as an explicit end-of-options marker. See the
/// hover snippet below for the value-parsing and `max-count` clamp
/// details.
///
/// **Frame and loop semantics.** `script` runs in the caller's own
/// variable scope on both the default (compiled) and `-direct` paths —
/// confirmed empirically: a `set` inside `script` mutates the calling
/// proc's own local, and `info level` is unchanged across the call —
/// hence `Traits::DYNAMIC_EVAL_BODY` below, the same reasoning `time.rs`
/// already documents for its own sibling command. Unlike `time`, though,
/// `timerate`'s script is a genuine loop body: `break` inside it stops the
/// measurement early and completes normally (`catch` reports `-code 0`,
/// with whatever partial count had been reached), and `continue` skips to
/// the next iteration — both confirmed empirically, on the default path
/// and under `-direct` alike. `time {break}` by contrast errors with
/// `invoked "break" outside of a loop`, since `time`'s own repetition
/// does not establish a real bytecode loop context the way `timerate`'s
/// does. An error, or a `return`, inside `script` propagates out of
/// `timerate` unchanged (and further out of the enclosing proc, for
/// `return`) exactly as it does for `time`.
///
/// **iRules.** No `irules/`, `expect/`, `tk/`, `eda_*/`, `iapps/`, or
/// `itcl/` pack defines its own override for `timerate` (grepped — no
/// hits beyond this file, its `tcl/mod.rs` registration, and a comment
/// cross-reference in `oo_my.rs`). iRules' `availability_mask` is just
/// the bare `IRULES` bit, with no
/// Tcl-version bits unioned in (`DialectProfile::irules()`,
/// `tcl-dialect/src/profile.rs`) — so
/// `Some(TCL86_PLUS).intersects(IRULES)` is already `false`, and the
/// `TCL86_PLUS` gate alone excludes `timerate` from iRules (whose
/// embedded core is pinned to Tcl 8.4.6), exactly the mechanism
/// `irules_banned_commands_never_resolve`
/// (`tcl-registry/tests/dialect_profile.rs`) already documents for
/// `dict`/`lassign`/`apply`/`lmap`/`coroutine`. Unlike its sibling `time`
/// — a genuine K36322151 command whose iRules exclusion rides on its own
/// `ALL_TCL` `dialects` group (see `time.rs`) — `timerate` was never one
/// of the sandbox bans; it is kept out of iRules purely on the version
/// axis, and there is no disable list for it ever to have been on.
///
/// For the additive vendor dialects, each profile's own
/// `availability_mask` settles it the same way: `expect` (`TCL86 |
/// EXPECT`), `cadence-eda-tcl` (`TCL86 | CADENCE`), and
/// `synopsys-eda-tcl` (`TCL86 | SYNOPSYS`) all carry the `TCL86` bit, so
/// `timerate` resolves there; `f5-iapps`, `f5-tmsh`, `xilinx-eda-tcl`,
/// `intel-quartus-eda-tcl`, and `mentor-eda-tcl` are all pinned to
/// `TCL85` only, so it does not — the same way `lmap` (8.6) is
/// unavailable there. Tk and incr Tcl have no `DialectProfile` of their
/// own (`tk` is a library pin layered on a host Tcl version; `itcl` is
/// not a `DialectSet` bit at all), so both simply inherit whatever host
/// Tcl version they run under, exactly like plain Tcl.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "timerate",
        // Tcl 8.6+ only — see the doc comment above. Narrower than the
        // sibling `time`'s `dialects: Some(DialectSet::ALL_TCL)`.
        dialects: Some(DialectSet::TCL86_PLUS),
        traits: Traits::BYTE_COMPILED | Traits::DYNAMIC_EVAL_BODY,
        // The body's position varies with the leading options, so we
        // don't pin a fixed BODY index beyond arg 0; the option /
        // positional mix makes the upper arity bound unbounded (mirrors
        // `return`'s own `Arity::any()`-style reasoning for the same
        // leading-option shape).
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::Body)],
        // `time` (arg 1) and `max-count` (arg 2) are both plain Tcl
        // integers in the no-leading-option form — confirmed empirically
        // (`expected integer but got "abc"` for either). Indices assume no
        // leading `-direct`/`-calibrate`/`-overhead` words, same caveat as
        // `arg_roles` above.
        arg_types: &[
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Measure the calibrated rate of execution of a script.",
            synopsis: &[
                "timerate script ?time? ?max-count?",
                "timerate ?-direct? ?-overhead estimate? script ?time? ?max-count?",
                "timerate ?-calibrate? ?-direct? script ?time? ?max-count?",
            ],
            snippet: "Added in Tcl 8.6 (absent from 8.4 and 8.5); unchanged in Tcl 9.0 and 9.1. Repeatedly evaluates script until time milliseconds have elapsed (default 1000, i.e. one second) or, if max-count is given, until that many iterations have run — whichever limit is reached first. A max-count of zero or less is accepted but runs script zero times, the same convention time's own count argument uses. By default the compiled form of script is used for the whole measurement, as if script were part of a compiled procedure; -direct instead evaluates script directly on every iteration without compiling it, the same way time does, which is useful for measuring the cost of Tcl_EvalObjEx itself and of uncompiled/canonical-list execution.\n\nEither way script runs in the caller's own variable scope, not a fresh call frame, so it can read and set the caller's locals directly. script is also a genuine loop body: break inside it stops the measurement early and completes normally with whatever partial count had been reached, and continue skips straight to the next iteration. This differs from time, where break and continue are simply illegal — the error is invoked \"break\" outside of a loop. An error, or a return, inside script propagates straight out of timerate unchanged, same as with time.\n\n-calibrate measures script's own overhead and stores it as the default overhead subtracted from every later invocation; if time is omitted it runs for up to 10 seconds, and calibration is not thread-safe. -overhead estimate overrides that stored overhead for one invocation only, taking a plain number of microseconds (a floating-point value is fine). Because an explicit -overhead is an absolute value rather than an increment over the calibrated one, measuring a custom overhead value should itself use -overhead 0, as in the last example below. -direct, -calibrate, and -overhead may be freely combined in any order — the three synopsis lines above are the manpage's documented usage patterns, not a hard restriction on which flags may appear together.\n\nOption names must be spelled out in full: unlike many Tcl commands, timerate accepts no abbreviated prefix, so -c or -over are not recognised as -calibrate or -overhead. An unrecognised flag-looking word is not reported as a bad option either — option scanning simply stops at the first word it cannot match, and that word becomes script instead, so a typo such as timerate -claibrate {...} 10 quietly measures the literal script -claibrate rather than raising a usage error. A bare -- explicitly ends option scanning, the same as in many other Tcl commands.",
            source: "Tcl timerate(n)",
            examples: "# calibrate once; the estimated overhead becomes the default for later calls\ntimerate -calibrate {}\n\n# measure a for loop for up to 5 seconds\ntimerate { for {set i 0} {$i < 10} {incr i} {} } 5000\n\n# measure a custom overhead value with -overhead 0, then reuse it\nset tm 0\nset ovh [lindex [timerate -overhead 0 {\n    incr tm [expr {24 * 60 * 60}]\n}] 0]\nset tm 0\ntimerate -overhead $ovh {\n    clock format $tm -format %H\n    incr tm [expr {24 * 60 * 60}]\n} 5000",
            return_value: "A canonical 8-element Tcl list of the form \"N µs/# COUNT # RATE #/sec NETMS net-ms\": lindex $result 0 is the average time per iteration in microseconds, lindex $result 2 is the iteration count actually run, lindex $result 4 is the estimated rate per second, and lindex $result 6 is the estimated net execution time in milliseconds with measurement overhead removed; the odd indices are the fixed unit labels µs/#, #, #/sec, and net-ms. Calling timerate -calibrate instead returns an 8-element list of its own: the freshly measured per-iteration overhead in microseconds followed by the fixed label µs/#-overhead, then that calibration run's own N µs/# COUNT # RATE #/sec statistics with no trailing net-ms pair.",
        }),
        forms: FORMS,
        options: const {
            &[
                OptionSpec {
                    name: "-calibrate",
                    value: OptionValue::flag(),
                    detail: "Calibrates timerate itself: runs script to compute a fresh per-iteration overhead estimate, which becomes the default overhead subtracted by later invocations. Runs for up to 10 seconds when time is omitted; calibration is not thread-safe.",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-overhead",
                    value: OptionValue::value("estimate"),
                    detail: "Overrides, for this invocation only, the per-iteration measurement overhead (in microseconds, which may be a floating-point number) that a previous -calibrate run estimated. To measure a raw overhead value of your own, invoke timerate with -overhead 0.",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "-direct",
                    value: OptionValue::flag(),
                    detail: "Evaluates script directly (without compiling it) on every iteration, the same way time does, instead of using the compiled form for the whole measurement. Useful for measuring the cost of Tcl_EvalObjEx, canonical-list invocation, and uncompiled bytecoded commands.",
                    ..OptionSpec::DEFAULT
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Ends option scanning explicitly. The next word is always taken as script, even if it looks like a flag.",
                    ..OptionSpec::DEFAULT
                },
            ]
        },
        ..CommandSpec::DEFAULT
    }
}
