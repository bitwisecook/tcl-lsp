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

//! `error` — generate an error.

use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "error message ?info? ?code?",
    ..FormSpec::DEFAULT
}];

const COMPLETION_CODES: &[CompletionCode] = &[CompletionCode::Error];

/// Tcl Error accepts a message and optional error information and code.
/// C8.4 presents error globals; C8.5+ also exposes return options. C8.6+
/// registers the Error compiler, with C9.1 Dictionary option construction.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "error",
        // Native TclCompileErrorCmd registration begins in C8.6; C8.4/8.5
        // execute the generic native handler.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::Error,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // Present and unrestricted: its surface explicitly carries an iRules
        // row (unlike the TMM-sandbox-banned commands, whose bare `ALL_TCL`
        // surface never intersects the `IRULES` mask) — `error` is a pure
        // exception-raising primitive with no filesystem/process/network
        // access, so every dialect that hosts a real Tcl core (irules, iapps,
        // tmsh, the EDA shells, expect, tk) carries it unmodified.
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        // `LANGUAGE_KEYWORD`, like its sibling `throw`: both raise an exception
        // and both are `TERMINATES_BLOCK`. Without both, `catch { error boom }`
        // would paint `catch` as a control keyword and `error` as an ordinary
        // library call — the two halves of one construct in two different
        // colours.
        //
        // Every Tcl grammar that *has* a function category agrees `error` is not
        // one: Pygments lists it under `Keyword`, tree-sitter under `@keyword`,
        // Zed under `@operator`, and the TextMate bundle under `keyword.other`.
        // Only grammars with no function bucket at all put it with the builtins.
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::TERMINATES_BLOCK
            | Traits::CATCHABLE_THROW
            | Traits::NEEDS_START_CMD,
        arity: Arity::new(1, 3),
        completion: Some(CompletionDescriptor::exact(COMPLETION_CODES)),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Generate an error",
            synopsis: &["error message ?info? ?code?"],
            snippet: "Returns a TCL_ERROR code that unwinds command interpretation, catchable with catch (or, from Tcl 8.6 on, try). message becomes the error result. The optional info, when given, seeds the -errorinfo return option (the errorInfo global in Tcl 8.4) and suppresses the interpreter's own first increment of stack-trace information — the command containing this error call is then omitted from the trace and info appears in its place, letting a catch handler graft a previously captured trace onto a re-thrown error. The optional code, when given, becomes the -errorcode return option (the errorCode global in Tcl 8.4), a machine-readable error identifier conventionally formatted as a list; when code is omitted, errorCode/-errorcode is reset to \"NONE\". From Tcl 8.5 on, prefer catching with the options-dictionary form and re-throwing via `return -options` over hand-copying errorInfo, since it preserves errorCode and every other return option too.",
            source: "Tcl error(n)",
            examples: "if {$divisor == 0} {\n    error \"cannot divide by zero\" \"\" {ARITH DIVZERO {divide by zero}}\n}\n\n# Re-throw with the original trace and error code intact (Tcl 8.5+)\nif {[catch {doWork} result options]} {\n    return -options $options $result\n}",
            return_value: "Never returns to the caller: unconditionally completes with a TCL_ERROR whose result is message, catchable by an enclosing catch (or, from Tcl 8.6 on, try).",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::Error),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

/// Jim's native error accepts a message and optional stack trace.
pub fn jim_spec() -> CommandSpec {
    let mut command = spec();
    command.surface = Some(tcl_dialect::surface![SpecSurface::core_in(
        tcl_dialect::model::Family::Jim,
        &[("0.84", None)]
    )]);
    command.arity = Arity::new(1, 2);
    command.forms = &[FormSpec {
        synopsis: "error message ?stacktrace?",
        ..FormSpec::DEFAULT
    }];
    command.hover = Some(HoverSnippet {
        summary: "Generate a Jim error",
        synopsis: &["error message ?stacktrace?"],
        snippet: "Raises an error with the selected message and optional raw stack trace. The error code is NONE; a third error-code operand is not accepted.",
        source: "Jim 0.84 Jim_ErrorCoreCommand",
        examples: "error BOOM",
        return_value: "Native command result",
    });
    command.native_compilation = Some(crate::native_compilation::NativeCompilationSpec {
        grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
        operation: crate::SemanticOperationId::Invoke,
        body: crate::native_compilation::NativeBodyCompilation::Inherit,
    });
    command
}
