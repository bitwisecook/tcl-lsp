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

//! `if` — conditional execution with optional elseif/else clauses.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

/// The slots of every condition clause: `expr ?then? body`.
const CONDITION_CLAUSE: &[ClauseSlot] = &[
    ClauseSlot::of(ArgRole::Expr),
    ClauseSlot::noise("then"),
    ClauseSlot::of(ArgRole::Body),
];

/// The final clause's one script word.
const FINAL_CLAUSE: &[ClauseSlot] = &[ClauseSlot::of(ArgRole::Body)];

/// `expr ?then? body (elseif expr ?then? body)* (?else? body)?`, the grammar
/// C Tcl's `Tcl_IfObjCmd` accepts — verified word for word against
/// `TclNRIfObjCmd` / `IfConditionCallback` in Tcl 9.0.4's
/// `generic/tclCmdIL.c` and cross-checked against tclsh 8.6 (the same
/// algorithm since at least Tcl 8.4).
///
/// Two properties fall out of declaring the real grammar rather than
/// keyword-matching every position:
///
/// - A condition slot (the head's, or the word right after `elseif`) is
///   *never* keyword-matched — `if else {a}` and `if elseif {a}` are
///   structurally well-formed `if`s whose single condition is the bareword
///   `else` / `elseif`; Tcl evaluates it as a boolean expression and fails
///   there (an invalid-bareword error), not as a malformed `if`. Only `then`
///   (right after a condition) and `elseif` / `else` (once a clause is
///   complete) are ever compared, matching `IfConditionCallback` exactly.
/// - Once a clause completes, at most one more bare word is accepted as the
///   implicit-else body (`?else?` is the tail's optional keyword); anything
///   past it is `ExtraWords`, Tcl's `wrong # args: extra words after "else"
///   clause`, whether or not an explicit `else` introduced that body.
pub const GRAMMAR: ClauseGrammarSpec = ClauseGrammarSpec {
    head: ClauseRow::head(CONDITION_CLAUSE, ClauseTiming::Selected),
    rows: &[ClauseRow::repeated(
        "elseif",
        CONDITION_CLAUSE,
        ClauseTiming::Selected,
    )],
    tail: Some(
        ClauseRow::once(Some("else"), FINAL_CLAUSE, ClauseTiming::Selected).optional_keyword(),
    ),
    fallthrough_body: None,
    // The final body runs when no condition held, and nothing may follow it.
    default_clause: Some(DefaultClause {
        row: None,
        final_only: true,
    }),
    selection: ClauseSelection::FirstMatch,
    surface: None,
};

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",
    ..FormSpec::DEFAULT
}];

/// Command spec for `if`.
///
/// Synopsis, grammar, and semantics are identical across Tcl 8.4, 8.5, 8.6,
/// 9.0, and 9.1: the if(n) manpage's SYNOPSIS, DESCRIPTION, and EXAMPLES
/// sections are word-for-word identical on all five version trees (fetched
/// and line-diffed directly, not paraphrased) — no wording, grammar,
/// option, or semantics change anywhere in the range. The five pages
/// differ only in cosmetic typesetting, split at the same 8.5/8.6
/// boundary: 8.4/8.5 use an ASCII troff-quoted "noise words" phrase and a
/// hyphen in the NAME line, where 8.6+ use a Unicode-quoted "noise words"
/// phrase and an em dash; and 8.4/8.5 render the EXAMPLES code blocks at 3-space indent
/// with the final multi-line-expression example kept on one line, where
/// 8.6+ use 4-space indent and wrap that same expression's `||` operators
/// one per line. `if` has never taken an option, never gained or lost a
/// form, and has no version-gated behaviour to model here.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "if",
        // Present and unrestricted everywhere. `if` is a pure control-flow
        // keyword with no filesystem/process/network access, so its surface
        // carries an iRules row explicitly (`ALL_TCL.union(IRULES)`) and it
        // resolves under the bare `IRULES` mask, the same as under every other
        // dialect that hosts a real Tcl core (iapps, tmsh, the EDA shells,
        // expect, tk). iRules availability is fully explicit per spec now —
        // there is no `disabled_commands` list for a command to be absent
        // from.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_BOOLEAN_COND
            | Traits::NEVER_INLINE_BODY
            | Traits::BRANCH_SELECTED_BODY
            | Traits::STRUCTURALLY_CHECKED_ARITY,
        // The floor here is purely descriptive (hover / hint text): the real
        // minimum is enforced by the clause grammar's walk, which also covers
        // the `elseif`/`else` chain shape a plain range can't express.
        arity: Arity::at_least(2),
        // The closed set of roles the grammar's walk emits.
        arg_role_resolver_roles: &[ArgRole::Expr, ArgRole::Body, ArgRole::Keyword],
        clause_grammar: Some(&GRAMMAR),
        lowering_hook: Some(crate::hooks::LoweringHookId::If),
        native_lowering: Some(NativeLowering::Structured(crate::hooks::LoweringHookId::If)),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Conditional execution with optional elseif/else branches.",
            synopsis: &[
                "if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",
                "if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else bodyN?",
            ],
            snippet: "Each expr is evaluated left to right, the same way expr evaluates its argument, until one is true; that clause's body runs and no later expr or body is touched. `then` and `else` are optional noise words kept only for readability — `if {$x} then {body}` and `if {$x} {body}` are equivalent. A boolean value is either numeric (0 is false, anything else is true) or one of the strings true/yes/false/no. Any number of elseif clauses may appear, including none, and the final body may be introduced with `else` or left bare with no keyword at all; an `else` with no body is an error, but a bare trailing body needs no `else` to be recognised. With no true expr and no final body, `if` returns an empty string.",
            source: "Tcl if(n)",
            examples: "if {$vbl == 1} {\n    puts \"vbl is one\"\n} elseif {$vbl == 2} {\n    puts \"vbl is two\"\n} else {\n    puts \"vbl is not one or two\"\n}",
            return_value: "The result of whichever body script ran, or an empty string if no expr was true and no final body was given.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
