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

//! `lassign` — assign list elements to variables.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    writes: true,
    ..SideEffect::DEFAULT
}];

// Tcl 8.5's SYNOPSIS is `lassign list varName ?varName ...?` — at least one
// varName is required there (`lassign $lst` alone is a "wrong # args"
// error). Tcl 8.6 relaxed this to `lassign list ?varName ...?`, making
// every varName optional; Tcl 9.0 and 9.1 keep the 8.6 shape unchanged.
// The two dialect-scoped forms below capture the exact legal shape per
// version — the broader 8.6+ shape is listed first since
// `CommandSpec::primary_synopsis` (the arity-diagnostic "usage: …" suffix)
// picks the first non-empty form regardless of dialect, and 8.6+ covers
// three of the four dialects `lassign` is available in. The command-level
// `arity` floor below stays at 1 (the 8.6+ minimum) since `Arity` has no
// per-dialect axis of its own — a real but narrow gap for arity
// diagnostics against code specifically targeting Tcl 8.5 and calling
// `lassign` with a list but no varName at all.
const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "lassign list ?varName ...?",
        surface: Some(SpecSurface::TCL86_PLUS),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "lassign list varName ?varName ...?",
        surface: Some(SpecSurface::TCL85),
        ..FormSpec::DEFAULT
    },
];

// Variable names follow the list value at fixed positions, even when that
// value is dynamic. The shared role owner can resolve this without evaluating it.
const REPEATED: &[RepeatedArgLayout] = &[RepeatedArgLayout::every(ArgRole::VarWrite, 1)];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lassign",
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::ListAssignment,
            operation: crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::ListAssign),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::FRAME_HASH_BUILTIN
            | Traits::BYTE_COMPILED
            | Traits::UNCONDITIONAL_VARIABLE_WRITE,
        surface: Some(SpecSurface::TCL85_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        // `lassign` writes list *elements* to its targets — of any intrep —
        // while returning the *leftover* list.  The elements are not the
        // return value, so they must not be typed `List`.
        var_write_typing: VarWriteTyping::ElementsOf { container_arg: 0 },
        // The returned leftover elements are a contiguous tail sub-list of
        // `list` (arg 0) — the same element-type-inference relationship as
        // `lrange list first last`.
        return_elements: Some(ReturnElements::SubListOf { container_arg: 0 }),
        hover: Some(HoverSnippet {
            summary: "Assign list elements to variables",
            synopsis: &["lassign list ?varName ...?"],
            snippet: "Treats list as a Tcl list and assigns its successive elements, in order, to the variables named by the varName arguments. If there are more varName arguments than list elements, the surplus variables are set to the empty string. If list has more elements than there are varName arguments, the leftover elements are returned as a list; otherwise the return value is the empty string. Before Tcl 8.6, at least one varName was required; from Tcl 8.6 onward `lassign list` alone is also legal and simply returns every element of list, since none of them get assigned. A common idiom uses `lassign` to \"shift\" the first element off a list: `set ::argv [lassign $::argv argumentToReadOff]` reassigns argv to everything after the first element while binding the first element to argumentToReadOff.",
            source: "Tcl lassign(n)",
            examples: "lassign {a b c} x y z       ;# assigns a to x, b to y, c to z; returns {}\nlassign {d e} x y z         ;# assigns d to x, e to y; z becomes {}; returns {}\nlassign {f g h i} x y       ;# assigns f to x, g to y; returns {h i}\nset ::argv [lassign $::argv argumentToReadOff]  ;# \"shift\" idiom: pops the first element off argv",
            return_value: "The empty string when every list element was assigned to a variable; otherwise a list of the elements left over after the last variable was assigned.",
        }),
        codegen_hook: Some(CodegenHookId::Lassign),
        forms: FORMS,
        world_effects: Some(crate::WorldEffectDescriptor::VARIABLE_WRITE),
        side_effects: SIDE_EFFECTS,
        repeated_args: REPEATED,
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..CommandSpec::DEFAULT
    }
}
