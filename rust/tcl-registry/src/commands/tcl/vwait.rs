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

//! `vwait` — wait for a variable to be modified.
//!
//! Every fetched manpage (Tcl 8.4, 8.5, 8.6, 9.0, 9.1) carries the same NAME
//! line, "Process events until a variable is written", and the same core
//! DESCRIPTION: a blocking event-loop wait, varName must be global-scoped
//! (a bare name resolves against the global namespace; a fully-qualified
//! name reaches another namespace), and nested vwait calls "nest and will
//! not happen in parallel" — that nesting caveat, and the whole NESTED
//! VWAITS BY EXAMPLE section, first appear in the Tcl 8.6 manpage, but
//! nothing suggests the underlying blocking behaviour is actually
//! different in 8.4/8.5, only that the prose explaining it was added
//! later.
//!
//! What genuinely changed is the SYNOPSIS. Tcl 8.4, 8.5, and 8.6 recognise
//! only the single-line `vwait varName` form — confirmed empirically
//! against a real `tclsh` 8.6.14: `vwait -all` there just waits on a
//! variable literally named `-all` (blocks forever, since nothing sets
//! it), and both `vwait` (0 args) and `vwait a b` (2 args) raise
//! "wrong # args: should be "vwait name"". Tcl 9.0 added a second form,
//! `vwait ?options? ?varName ...?`, with eleven new switches — `--`,
//! `-all`, `-extended`, `-nofileevents`, `-noidleevents`,
//! `-notimerevents`, `-nowindowevents`, `-readable`, `-timeout`,
//! `-variable`, `-writable` — absent from every earlier release's
//! manpage and (per `generic/tclBasic.c`'s `builtInCmds[]`) command
//! table alike. `generic/tclEvent.c`'s `Tcl_VwaitObjCmd` shows that a
//! 9.0+ call with zero total arguments after options are stripped still
//! parses; it only fails at *run time* ("can't wait: would block
//! forever") when no option supplies a wait condition and no varName is
//! given either. Tcl 9.1's vwait.html is identical to 9.0's except
//! `-timeout` gains one added sentence, "A monotonic clock is used if
//! available" — folded into that `OptionSpec`'s `detail` below as plain
//! prose rather than modelled as a new dialect gate, since the option
//! itself, not just this detail of it, is already present in 9.0.
//!
//! iRules bans `vwait` outright: it is one of the K36322151 event-loop
//! bans, encoded directly in the spec's `dialects` group — `vwait`
//! carries `ALL_TCL` with no `IRULES` bit (see `spec()` below), so it
//! never intersects the bare `IRULES` availability mask and falls out of
//! iRules by plain intersection, with no subtractive disable list
//! (contract-tested by `irules_banned_commands_lack_the_irules_bit`,
//! `tcl-registry/tests/dialect_profile.rs`). No other dialect profile
//! (iApps, tmsh, the EDA vendor shells, Expect, Tk) bans `vwait` or
//! registers a competing spec of its own — the `irules/`, `iapps/`,
//! `expect/`, `tk/`, and `eda_*/` command directories were all grepped
//! for `vwait` with no hits (`tmsh` has no directory of its own; its
//! specs live alongside iApps', tagged with the `TMSH` bit, so the
//! `iapps/` grep already covers it) — so it stays reachable there,
//! gated only by each profile's own `version_ceiling`
//! (all of iApps/tmsh/Quartus/Mentor/Xilinx at Tcl 8.5, Cadence/Synopsys/
//! Expect at Tcl 8.6) intersecting each new option's own `TCL90_PLUS`
//! gate below — none of them reach it, matching their real embedded
//! cores. `bpf` is the one profile whose mask does reach Tcl 9.0
//! (`surface![SpecSurface::core_in(Family::Tcl, &[("9.0", Some("9.1"))]), SpecSurface::package("bpf")]`, a genuine embedded Tcl
//! 9.0 core, and it does not ban `vwait`), so it resolves both the command
//! and the new options today, the same way `tcl::process`'s identical
//! `TCL90_PLUS` gate does (`commands/tcl/tcl_process.rs`).
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "Mark the end of options; every remaining argument is treated as a variable name.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-all",
        value: OptionValue::flag(),
        detail: "Require every wait condition to be met to complete the wait operation. Without -all (the default), the first condition that fires completes the wait.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-extended",
        value: OptionValue::flag(),
        detail: "Return an extended result: an even-sized list whose odd elements are readable, timeleft, variable, or writable and whose following element is the matching channel/variable name or the remaining milliseconds, one pair per condition that fired, ordered by when each fired except timeleft, which always comes last.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-nofileevents",
        value: OptionValue::flag(),
        detail: "Do not service file events while waiting.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-noidleevents",
        value: OptionValue::flag(),
        detail: "Do not invoke idle handlers while waiting.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-notimerevents",
        value: OptionValue::flag(),
        detail: "Do not service timer events (such as after) while waiting.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-nowindowevents",
        value: OptionValue::flag(),
        detail: "Do not handle windowing-system events while waiting.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-readable",
        value: OptionValue::channel("channel"),
        detail: "Complete the wait once channel is, or becomes, readable. channel must name a Tcl channel open for reading.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-timeout",
        value: OptionValue::Takes(OptionArg {
            integer: Some(IntegerDomain::Any),
            hint: "milliseconds",
            ..OptionArg::DEFAULT
        }),
        detail: "Limit the wait to milliseconds. (From Tcl 9.1, a monotonic clock is used for this when available.)",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-variable",
        value: OptionValue::var_name(),
        detail: "varName must be the name of a global variable. Writing or unsetting it completes the wait operation.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-writable",
        value: OptionValue::channel("channel"),
        detail: "Complete the wait once channel is, or becomes, writable. channel must name a Tcl channel open for writing.",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..OptionSpec::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[
    // The only form Tcl 8.4/8.5/8.6 (and the vendor dialects whose
    // embedded core caps below 9.0) recognise — see the module doc
    // comment for the empirical 8.6.14 confirmation. iRules is
    // deliberately absent from this union: `vwait` is fully disabled
    // there (see the module doc comment), so no form of it is legal
    // syntax under `f5-irules` at all.
    FormSpec {
        synopsis: "vwait varName",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.7"))]), SpecSurface::package("iapps"), SpecSurface::package("tmsh"), SpecSurface::package("expect")]),
        ..FormSpec::DEFAULT
    },
    // Tcl 9.0+ (and `bpf`, whose mask reaches Tcl 9.0 automatically
    // through the `TCL90_PLUS` overlap — see the module doc comment).
    FormSpec {
        synopsis: "vwait ?options? ?varName ...?",
        surface: Some(SpecSurface::TCL90_PLUS),
        ..FormSpec::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vwait",
        // Present everywhere except iRules: `ALL_TCL` carries no `IRULES`
        // bit, so this spec never intersects the bare `IRULES` mask and is
        // banned there by plain intersection, with no disable list — see
        // the module doc comment.
        surface: Some(SpecSurface::ALL_TCL),
        // BYTE_COMPILED only in the "recognised core builtin" sense this
        // codebase's convention uses (traits.rs's doc comment) — real
        // `Tcl_VwaitObjCmd` has no `CompileProc`. FRAME_HASH_BUILTIN is
        // named directly in that trait's own doc comment alongside
        // upvar/global/variable/lassign/lset/regexp/regsub/scan/binary/
        // tkwait. CREATES_DYNAMIC_BARRIER: from Tcl 9.0 a single call can
        // name *several* variables at once (`vwait a b c`, or
        // `vwait -all a b c`) — this trait makes the minifier's
        // `find_barrier_scopes` (`tcl-lsp-core/src/minify.rs`) treat any
        // scope that calls `vwait` as unsafe to rename in, which covers
        // every name `vwait` might touch even though the static
        // `arg_roles` below only tags argument 0 (see the comment there).
        // Not SAFE_INTERP_HIDDEN: Tcl 9.0.4's `builtInCmds[]`
        // (`generic/tclBasic.c`) marks vwait's row `CMD_IS_SAFE`
        // outright, so a safe interpreter keeps it, unlike cd/exec/exit/
        // fconfigure/file/glob/load/open/pwd/socket/source/unload. Not
        // FRAMELESS_RUNTIME: `vwait` pumps `Tcl_DoOneEvent` in a loop,
        // which can invoke arbitrary file/timer/idle/window event-handler
        // scripts reentrantly — a genuine eval fallback, the same
        // reasoning documented on `update`'s and `after`'s identical
        // BYTE_COMPILED-only baseline (`update.rs`, `after_.rs`).
        traits: Traits::BYTE_COMPILED
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN,
        // Tcl 8.4/8.5/8.6 accept exactly one argument (`vwait varName`);
        // Tcl 9.0+ accepts any number of leading options followed by any
        // number of varNames — even zero total words parses fine there
        // (it only ever fails at *run time*, "can't wait: would block
        // forever" — see the module doc comment). `Arity` has no
        // per-dialect variant, so this keeps the loosest (9.0+) bound
        // here rather than false-positiving E002/E003 on a legal 9.0+
        // call such as `vwait -readable $chan` (zero bare varNames) or
        // `vwait -all a b c` (three) — the same trade-off `global` makes
        // for its own cross-version arity split (`global_.rs`). See
        // FORMS above for the version-split synopsis text, and each
        // OptionSpec's own `dialects` for exactly which switches exist
        // in which release.
        arity: Arity::any(),
        // `VarWrite`, not `VarRead`: `Tcl_VwaitObjCmd` (tclEvent.c) never
        // reads the variable's value — it installs a
        // `TCL_TRACE_WRITES|TCL_TRACE_UNSETS` trace (creating the entry when
        // absent) and returns only once the variable has been *written* by
        // an event handler.  `vwait forever` on a never-set variable is the
        // canonical infinite-wait idiom, so modelling the operand as a read
        // produced a false W210 (read-before-set); as a write the post-state
        // is a defined variable, which is what the analyser records.
        //
        // This static table only covers argument 0 — the common
        // single-bare-varName call shape that remains legal, and by far
        // the most usual, in every version. Tcl 9.0's further trailing
        // varNames and its `-variable varName` option (itself
        // independently `VarWrite`-tagged above via
        // `OptionValue::var_name()`) are not walked into extra
        // `arg_roles` entries here; `Traits::CREATES_DYNAMIC_BARRIER`
        // (see `traits` above) is the safety net that keeps an untracked
        // second or third varName from ever being silently mis-renamed.
        arg_roles: &[(0, ArgRole::VarWrite)],
        // The value observed after the wait is whatever the event handler
        // stored — unknowable statically — so the written variable is typed
        // overdefined, never from vwait's own (empty-string) return type.
        var_write_typing: VarWriteTyping::Destructured,
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Process events until a variable is written, or — from Tcl 9.0 — until a channel is ready or a timeout elapses.",
            synopsis: &["vwait varName", "vwait ?options? ?varName ...?"],
            snippet: "Enters the Tcl event loop to process events, blocking the application if no events are ready, until the wait condition is met. varName is always interpreted as a variable name with respect to the global namespace, but can refer to any namespace's variable if a fully-qualified name is given. vwait may not return immediately once its condition is met: if the event handler that satisfies it does not itself complete right away — for instance because it calls vwait again, on a different variable — the outer call stays blocked until that inner one finishes; multiple vwait calls nest rather than run in parallel, and the outermost does not return until every inner one does. Some packages (http, in its synchronous modes) use vwait internally, so nesting a vwait inside an event handler risks a deadlock and is best avoided. Tcl 8.4/8.5/8.6 accept only a single varName and no options: waiting is complete once that variable is written by some event handler. Tcl 9.0 adds a second form with options for finer control and multiple event sources: any number of varNames may be given, -variable adds another (a write or an unset both complete it), and -readable/-writable wait on a channel's readiness instead of a variable — by default (without -all) the wait completes as soon as the first of these fires, not once every one of them has; -all instead requires every condition to fire before the wait completes. -timeout separately bounds the whole wait by a millisecond limit, regardless of -all. -nofileevents/-noidleevents/-notimerevents/-nowindowevents each exclude one class of event from being serviced while waiting.",
            source: "Tcl vwait(n)",
            examples: "# The canonical block-forever idiom: nothing besides some other\n# event handler calling \"set forever 1\" (or similar) ever ends this.\nvwait forever\n\n# Wait up to 5 seconds for a connection, then react to which happened\nafter 5000 {set state timeout}\nset server [socket -server accept 12345]\nproc accept {args} {\n    global state connectionInfo\n    set state accepted\n    set connectionInfo $args\n}\nvwait state\nclose $server\nafter cancel {set state timeout}\n\n# Tcl 9.0+: wait for the first of several conditions, with a timeout\nvwait -timeout 5000 -readable $chan -variable done",
            return_value: "An empty string for the plain vwait varName form. From Tcl 9.0, -timeout instead makes the result the number of milliseconds remaining when the wait condition was met, or -1 if the wait timed out; and -extended makes it a list of alternating condition-name/value pairs (readable, timeleft, variable, writable) in firing order, with timeleft always last.",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
