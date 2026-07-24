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

//! `lseq` — build a numeric sequence returned as a list (Tcl 9.0+).
//
// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/lseq.html, `.htm` for the 8.6 tree, each fetched
// directly): the 8.4 and 8.5 URLs 404, and the 8.6 URL 404s on *both* the
// `.html` and `.htm` extensions — a genuine absence, not the broken-
// redirect gotcha, confirmed by also checking the 8.4/8.5/8.6
// TclCmd/contents.htm command indexes, none of which list `lseq` among
// their "l" commands. `lseq` genuinely does not exist before Tcl 9.0. The
// 9.0 and 9.1 pages are byte-identical apart from the version-banner
// breadcrumb ("9.0.4" vs "9.1b0" — confirmed with a byte diff of the raw
// HTML), so nothing about `lseq` changed between the two releases: the
// same three synopsis forms, the same DESCRIPTION prose, and the same
// worked examples.
//
// No dialect directory (irules/, iapps/, tk/, expect/, eda_*/, itcl/)
// references `lseq`, and none of those dialects model a base Tcl core
// version of 9.0 or later (`DialectSet::expr_grammar_base_version` tops
// out at 8.6 for every one of them — iRules is a genuine embedded Tcl
// 8.4.6), so the plain `TCL90_PLUS` gate below already excludes every one
// of them; no extra dialect restriction is needed.
//
// Implementation note, out of this file's scope (`tcl-cmd-core/src/
// lseq.rs`, not touched here): the manpage's own worked example
// (`% lseq 3 to 9 by 0` -> `3`, a one-element list) shows that when step
// is 0 and no explicit count is given, count implicitly defaults to 1
// rather than the usual `int(((end - start) / step) + 1)` formula (which
// cannot apply when step is 0). `tcl_cmd_core::lseq::build_int`'s
// `plan.count.is_none()` branch instead returns the empty list whenever
// the resolved step is 0, before a length is ever computed — a
// discrepancy from the documented example worth an independent look by
// whoever next touches that file.
//
// Security-relevant fact not previously modelled: an argument word that is
// neither a plain number nor one of the `..`/`to`/`count`/`by` keywords is
// evaluated as a Tcl *expression* — the manpage's own worked example
// `foreach i [lseq {[llength $l]-1} 0] { ... }` only works because of this
// fallback, and Tcl's expression grammar accepts `[command]` substitution
// as an operand exactly as `expr` does. This codebase's own VM
// implementation confirms the same edge: `tcl-vm/src/cmd_lseq.rs`'s
// `eval_num` calls `vm.eval_expr`, the very evaluator `expr` itself runs
// on (`Vm::eval_expr`, `tcl-vm/src/interp.rs`), and
// `tcl-cmd-core/src/lseq.rs` documents it as "the expression-valued-
// argument edge", ported straight from C's `Tcl_LseqObjCmd`. So `lseq`
// carries the same `TAINT_SINK` / `PURE_EVALUATION` pairing as `expr`
// (`commands/tcl/expr_.rs`): tainted data reaching *any* argument position
// can execute arbitrary commands via that fallback, and a call whose
// fallback invokes a side-effecting command is not actually side-effect-
// free — so the previous plain `PURE`/`CSE_CANDIDATE` classification was a
// false guarantee. Wired into the VM as a flat native dispatch with no
// script-body/eval fallback of its own (`tcl-vm/src/cmd_lseq.rs::
// cmd_lseq`), so `FRAMELESS_RUNTIME` still holds, matching `expr`'s own
// classification for the identical reason.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "lseq start ?(..|to)? end ??by? step?",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "lseq start count count ??by? step?",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "lseq count ?by step?",
        dialects: None,
    },
];

// Mirrors `expr`'s own `SIDE_EFFECTS` (`commands/tcl/expr_.rs`): the
// expression-valued-argument fallback (see the module comment) can invoke
// an arbitrary nested command, so the conservative, sound declaration is
// "reads some unknown state" rather than no side effects at all.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// Keyword tokens that can appear at argument index 1 — the branch point
/// where each of the three documented forms diverges: the `..`/`to` range
/// operator or a plain numeric `end` value (`lseq start ?(..|to)? end
/// ...`), the `count` keyword (`lseq start count count ...`), or `by`
/// introducing the step directly after a bare count value (`lseq count by
/// step`). Non-exhaustive completion/hover data only: a plain numeric
/// `end` value is equally legal at this index, so it is not in
/// `closed_value_args`.
const OP_KEYWORD_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "..",
        detail: "Range operator: sequence from start through end (an inclusive limit, not necessarily the last element). Interchangeable with `to`.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "to",
        detail: "Range operator: sequence from start through end (an inclusive limit, not necessarily the last element). Interchangeable with `..`.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "count",
        detail: "Keyword introducing an explicit element count in place of end: `lseq start count count`.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "by",
        detail: "Keyword introducing the step value in the bare count form `lseq count by step` — required here (unlike `start end by step`, where `by` can be dropped): `lseq count step` with no `by` is parsed as `lseq start end` instead of count+step.",
        ..ArgValue::DEFAULT
    },
];

/// Command spec for `lseq`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lseq",
        // Deterministic in the common (plain-number-argument) case, but
        // `expr` pairs `PURE_EVALUATION`/`TAINT_SINK` rather than plain
        // `PURE` for exactly the reason that applies here too — see the
        // module comment: an argument word that falls back to expression
        // evaluation can invoke an arbitrary, possibly side-effecting,
        // command via `[...]` substitution, so a blanket `PURE` claim
        // (and `CSE_CANDIDATE`, which would license eliding a "duplicate"
        // call) would be unsound. Wired into `tcl-vm` as a flat native
        // dispatch with no script-body eval fallback of its own, so
        // `FRAMELESS_RUNTIME` holds. It has its own dedicated manual page
        // (not a Tcl-coded library proc), making it a real core builtin,
        // so it carries `BYTE_COMPILED` like its Tcl-9.0-native siblings
        // `lpop`/`ledit`/`lremove`. A 3-argument call whose first word has
        // no `$`/`[` and whose last word is braced (e.g. `lseq 5 to
        // {10}`, or brace-protecting an expression per the documented
        // `lseq {[llength $l]-1} 0` idiom) is a `HEAD NAME BRACED BRACED`
        // four-token invocation shape, so it needs `NOT_PROC_FACTORY` for
        // the same reason `list`/`lindex`/`lrepeat`/`lremove` do.
        traits: Traits::PURE_EVALUATION
            | Traits::TAINT_SINK
            | Traits::BYTE_COMPILED
            | Traits::NOT_PROC_FACTORY
            | Traits::FRAMELESS_RUNTIME,
        // Added in Tcl 9.0; absent from 8.4/8.5/8.6 (404 on every fetch,
        // including both extensions for 8.6, and absent from every 8.x
        // command index) and unchanged in 9.1 (byte-identical manpage) —
        // see the module comment.
        dialects: Some(DialectSet::TCL90_PLUS),
        // 1..=5 words after the command name spans every documented form
        // (`lseq count` at the floor; `lseq start to end by step` /
        // `lseq start count count by step` at the ceiling), matching the
        // VM's own "nargs == 0 || nargs > 5" bound
        // (`tcl-cmd-core/src/lseq.rs::decode`) exactly.
        arity: Arity::new(1, 5),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Build a numeric sequence returned as a list.",
            synopsis: &[
                "lseq start ?(..|to)? end ??by? step?",
                "lseq start count count ??by? step?",
                "lseq count ?by step?",
            ],
            snippet: "Creates a sequence of wide integers or doubles from start, end, and step, or 0 through count-1 in the bare single-argument form. When start and end are given without an explicit step, the sequence is increasing if start <= end and decreasing otherwise; a step whose sign disagrees with that direction produces an empty list rather than an error. start defines the initial value and end defines a limit, not necessarily the final element. A step of 0 always repeats start: the result has count elements when count is given explicitly (`lseq 1 count 5 by 0` -> five 1s), or a single element when it is not (`lseq 3 to 9 by 0` -> just 3) — end plays no role in sizing the result when step is 0. `..` and `to` are interchangeable range operators; `count` introduces an explicit element count in place of end; `by` introduces the step value and can be dropped whenever a numeric end or count value precedes it (`start end by step` and `start end step` mean the same thing), but not in the bare `count by step` form, where dropping `by` is read as `start end` instead. Any argument word that is not a plain number and not one of these four keywords is evaluated as a Tcl expression exactly as expr would — including [command] substitution — which is how the documented `lseq {[llength $l]-1} 0` idiom works, but it also means tainted or attacker-influenced text reaching any argument can execute arbitrary commands the same way an unbraced `expr $x` can.",
            source: "Tcl lseq(n)",
            examples: "lseq 3                 ;# 0 1 2\nlseq 3 0               ;# 3 2 1 0\nlseq 10 .. 1 by -2     ;# 10 8 6 4 2\nlseq 1 count 5 by 0    ;# 1 1 1 1 1\nlseq 3 to 9 by 0        ;# 3 (count defaults to 1 when step is 0)\nlseq 0 0.5 by 0.1      ;# 0.0 0.1 0.2 0.3 0.4 0.5",
            return_value: "A list of wide integers or doubles forming the requested sequence — 0 through count-1 in the one-argument form, or start stepping toward the end limit otherwise. Empty when a given step's sign disagrees with the sequence's direction.",
        }),
        forms: FORMS,
        arg_values: &[(1, OP_KEYWORD_VALUES)],
        ..CommandSpec::DEFAULT
    }
}
