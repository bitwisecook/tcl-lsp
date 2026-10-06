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

//! Native C Tcl 9 numeric sequences. Pinned `SequenceIdentifyArgument`
//! accepts numbers and unique `..`/`to`/`count`/`by` prefixes; it does not
//! evaluate operand bytes as Tcl expressions. Sequence result provider,
//! argument conversions and native compiler capability are separate contracts.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "lseq start ?(..|to)? end ??by? step?",
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "lseq start count count ??by? step?",
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "lseq count ?by step?",
        ..FormSpec::DEFAULT
    },
];

// Mirrors `expr`'s own `SIDE_EFFECTS` (`commands/tcl/expr_.rs`): the
// expression-valued-argument fallback (see the module comment) can invoke
// an arbitrary nested command, so the conservative, sound declaration is
// "reads some unknown state" rather than no side effects at all.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    ..SideEffect::DEFAULT
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
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::HookFrom(
                tcl_dialect::TclVersion::V9_1,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // Numeric conversion does not execute operand text as a Tcl script.
        // Normal world closure still requires the separate selected operand proof.
        traits: Traits::PURE
            | Traits::BYTE_COMPILED
            | Traits::NOT_PROC_FACTORY
            | Traits::FRAMELESS_RUNTIME,
        surface: Some(SpecSurface::TCL90_PLUS),
        // 1..=5 words after the command name spans every documented form
        // (`lseq count` at the floor; `lseq start to end by step` /
        // `lseq start count count by step` at the ceiling), matching the
        // VM's own "nargs == 0 || nargs > 5" bound
        // (`tcl-cmd-core/src/lseq.rs::decode`) exactly.
        arity: Arity::new(1, 5),
        native_result: Some(crate::native_result::NativeResultContract::ArithmeticSequence),
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::ArithmeticSequenceArguments,
        ),
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
            snippet: "Creates a sequence of wide integers or doubles from start, end, and step, or 0 through count-1 in the bare single-argument form. When start and end are given without an explicit step, the sequence is increasing if start <= end and decreasing otherwise; a step whose sign disagrees with that direction produces an empty list rather than an error. start defines the initial value and end defines a limit, not necessarily the final element. A step of 0 always repeats start: the result has count elements when count is given explicitly (`lseq 1 count 5 by 0` -> five 1s), or a single element when it is not (`lseq 3 to 9 by 0` -> just 3) — end plays no role in sizing the result when step is 0. `..` and `to` are interchangeable range operators; `count` introduces an explicit element count in place of end; `by` introduces the step value and can be dropped whenever a numeric end or count value precedes it (`start end by step` and `start end step` mean the same thing), but not in the bare `count by step` form, where dropping `by` is read as `start end` instead. Numbers must already be numeric when the command is invoked. Argument text is not evaluated as an expression; compute a bound separately with expr when needed. Unique prefixes of the range keywords are accepted.",
            source: "Tcl lseq(n)",
            examples: "lseq 3                 ;# 0 1 2\nlseq 3 0               ;# 3 2 1 0\nlseq 10 .. 1 by -2     ;# 10 8 6 4 2\nlseq 1 count 5 by 0    ;# 1 1 1 1 1\nlseq 3 to 9 by 0        ;# 3 (count defaults to 1 when step is 0)\nlseq 0 0.5 by 0.1      ;# 0.0 0.1 0.2 0.3 0.4 0.5",
            return_value: "A list of wide integers or doubles forming the requested sequence — 0 through count-1 in the one-argument form, or start stepping toward the end limit otherwise. Empty when a given step's sign disagrees with the sequence's direction.",
        }),
        forms: FORMS,
        arg_values: &[(1, OP_KEYWORD_VALUES)],
        ..CommandSpec::DEFAULT
    }
}
