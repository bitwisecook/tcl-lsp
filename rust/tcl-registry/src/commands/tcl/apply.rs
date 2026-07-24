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

//! `apply` — apply an anonymous procedure.
use crate::prelude::*;

/// `apply` never binds a name in the interpreter's command table — the
/// temporary `Proc` structure TIP 194 builds for the lambda is discarded
/// as soon as the call returns — so this is not a [`SideEffectTarget::ProcDefinition`]
/// write. Like `eval`, the lambda body can do literally anything once
/// invoked, so this uses the same `Unknown`/no-reads/no-writes
/// "unknowable statically" placeholder `eval`'s spec declares, rather
/// than asserting a specific effect the command doesn't actually have.
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: false,
    writes: false,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "apply func ?arg1 arg2 ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "apply",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::DYNAMIC_EVAL_BODY
            // func's list-form body is executed verbatim; tainted data
            // reaching it is arbitrary code execution, exactly like
            // `eval`/`uplevel` (Tcl apply(n)). Deliberately *not*
            // `EVALUATES_CODE`: that trait pairs with `TAINT_SINK` to mean
            // "the whole tail is one concatenated script" (`eval`'s shape,
            // consumed by W101/W301/W309 in tcl-compiler's security.rs) —
            // `apply`'s trailing arg1 arg2 ... are ordinary call arguments
            // bound to the lambda's formal parameters, never re-parsed as
            // script text, so that pairing would misclassify it.
            | Traits::TAINT_SINK,
        // Added in Tcl 8.5 (TIP 194).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        // The lambda literal is a `{argList body ?ns?}` list, not a plain
        // script — `ArgRole::LambdaLiteral` (not `Body`) says so, so generic
        // Body-role walkers (SSA's caller-scope scan, the semantic-token
        // highlighter's default body recursion) never try to re-segment the
        // list itself as script source. The real body (element 1) is walked
        // by the dedicated `LoweringHookId::Apply` / `AnalyserHookId::Apply`
        // hooks below, which already split the list correctly.
        arg_roles: &[(0, ArgRole::LambdaLiteral)],
        lowering_hook: Some(crate::hooks::LoweringHookId::Apply),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Invoke an anonymous function with the given arguments.",
            synopsis: &["apply func ?arg1 arg2 ...?"],
            snippet: "func is a two- or three-element list {args body ?namespace?}: args is a formal parameter list in the same form proc accepts (including a trailing args catch-all and default values), body is the script to run, and the optional namespace names the namespace the body's commands and variables resolve in (the global namespace when omitted). Each call builds a fresh, unnamed procedure, applies it to arg1 arg2 ..., and discards it afterwards — the formal parameters become local variables in a new call frame, so the body cannot see or modify the caller's locals without global, variable, or upvar. Raises an error if func is not a valid 2- or 3-element list.",
            source: "Tcl apply(n)",
            examples: "set square {{x} {expr {$x * $x}}}\napply $square 5\napply {{a b {suffix \"\"}} {return \"$a$b$suffix\"}} foo bar",
            return_value: "The result of evaluating the function body.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::Apply),
        ..CommandSpec::DEFAULT
    }
}
