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

//! `concat` — concatenate original Lists or trimmed string bytes.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "concat ?arg arg ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        // Reached native value handler has no callbacks or variable-name writes.
        successful_handler: Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf),
        completion: Some(crate::completion::CompletionDescriptor::exact(&[
            crate::completion::CompletionCode::Ok,
            crate::completion::CompletionCode::Error,
        ])),
        name: "concat",
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            // TclCompileConcatCmd accepts all retained argument counts; the
            // enclosing compiler declines expanded words before calling it.
            grammar: crate::native_compilation::NativeCompilationGrammar::ArgumentConcatFrom(
                tcl_dialect::TclVersion::V8_6,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("concat"),
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        byte_array_effect: ByteArrayEffect::Coerces,
        const_fold: Some(|args| Some(crate::const_fold::fold_concat(args))),
        codegen_hook: Some(CodegenHookId::Concat),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::PURE
            | Traits::EXPANSION_ESCAPE_SAFE
            | Traits::BYTE_COMPILED,
        arity: Arity::any(),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Join lists together",
            synopsis: &["concat ?arg arg ...?"],
            snippet: "Joins every argument together with a single space, after trimming leading and trailing white-space from each one. If all of the arguments are lists, this has the same effect as concatenating them into a single list. Arguments that are empty after trimming are dropped entirely rather than leaving a stray separating space (documented from Tcl 8.6 on; the 8.4/8.5 manual pages describe only the trim-and-join rule). Whitespace in the interior of an argument is left untouched, so multiple internal spaces survive the join. Because concat splices raw argument text rather than list structure, unbalanced braces or quotes spanning an argument boundary can produce a malformed result; for guaranteed list-safe concatenation use `list {*}listA {*}listB` instead (the official recommendation as of the Tcl 9.0 manual page).",
            source: "Tcl concat(n)",
            examples: "concat a b {c d e} {f {g h}}\nconcat \" a b {c   \" d \"  e} f\"\nconcat \"a   b   c\" { d e f }\nlist {*}\"a   b   c\" {*}{ d e f }",
            return_value: "The concatenated string, or the empty string when no arguments are given.",
        }),
        forms: FORMS,
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    }
}

/// Jim's native concat can depend on retained list representations.
pub fn jim_spec() -> CommandSpec {
    let mut command = spec();
    command.surface = Some(tcl_dialect::surface![SpecSurface::core_in(
        tcl_dialect::model::Family::Jim,
        &[("0.84", None)]
    )]);
    command.const_fold = None;
    command
}
