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

//! Compiled literal bodies: boolean pruning, expansion, exception ranges and body boundaries.

use super::*;

#[test]
fn expanded_no_body_compilation_is_independent_of_runtime_argv_length() {
    use crate::InvocationWord::{Expanded, Literal};
    let registry = crate::model::ingress::static_context_for("tcl9.1").commands();
    let dialect = InvocationDialect::for_version(TclVersion::V9_1);
    let arguments = [Literal("x"), Expanded];
    let shapes = [
        NativeCompilationWordShape::Literal,
        NativeCompilationWordShape::Expanded,
    ];
    for head in ["lappend", "proc", "foreach", "eval", "try"] {
        let words = InvocationWords::structured(Literal(head), &arguments).with_dialect(dialect);
        let resolution = registry.resolve_structured_invocation(words, dialect.authoring_query());
        let facts = resolution
            .resolved()
            .expect("actual stock invocation")
            .facts();
        let spec = facts.native_compilation.expect("authored native compiler");
        let selected = spec.select(
            words,
            &shapes,
            Some(dialect),
            context(NativeCompilationFrame::ProcedureCode),
        );
        assert_eq!(words.arguments().exact_argv_len(), None);
        if head == "lappend" {
            assert!(matches!(
                selected,
                NativeCompilationSelection::Inline { .. }
            ));
            assert!(facts.arg_roles_complete);
            assert_eq!(
                spec.compilation_steps(selected, words, &shapes, &facts, Some(dialect)),
                NativeCompilationSteps::Known(Vec::new())
            );
            let mut incomplete = facts.clone();
            incomplete.arg_roles_complete = false;
            assert_eq!(
                spec.compiled_bodies(selected, words, &shapes, &incomplete, Some(dialect)),
                NativeCompiledBodies::Unknown
            );
        } else {
            assert!(
                !matches!(selected, NativeCompilationSelection::Inline { .. }),
                "{head}: expansion cannot select an original inline body"
            );
        }
        assert_eq!(
            spec.compiled_bodies(
                NativeCompilationSelection::Unknown,
                words,
                &shapes,
                &facts,
                Some(dialect)
            ),
            NativeCompiledBodies::Unknown,
            "{head}: no-body metadata cannot close unknown compiler selection"
        );
    }
}

fn compiled_arguments(head: &str, arguments: &[&str]) -> NativeCompiledBodies {
    let registry =
        crate::cache::registry_for_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
    let facts = registry
        .resolve_invocation(head, arguments, registry.own_surface_query())
        .expect("stock native invocation")
        .facts();
    let spec = registry.get(head).unwrap().native_compilation.unwrap();
    let words = InvocationWords::literals(head, arguments);
    let shapes = vec![NativeCompilationWordShape::BracedLiteral; arguments.len()];
    let dialect = Some(InvocationDialect::for_version(TclVersion::V8_4));
    let selected = spec.select(
        words,
        &shapes,
        dialect,
        context(NativeCompilationFrame::ProcedureCode),
    );
    assert!(matches!(
        selected,
        NativeCompilationSelection::Inline { .. }
    ));
    spec.compiled_bodies(selected, words, &shapes, &facts, dialect)
}

#[test]
fn compiled_bodies_use_literal_boolean_pruning_instead_of_runtime_folding() {
    use NativeCompilationFailureScope::EnclosingChunk;
    let known =
        |indices: &[usize], loops: &[usize], annotations: &[NativeCompiledBodyErrorContext]| {
            NativeCompiledBodies::Known(
                indices
                    .iter()
                    .zip(annotations)
                    .map(|(argument, annotation)| NativeCompiledBodyOperand {
                        error_context: *annotation,
                        argument: *argument,
                        context: if loops.contains(argument) {
                            NativeCompiledBodyContext::Loop
                        } else {
                            NativeCompiledBodyContext::Inherit
                        },
                        failure_scope: EnclosingChunk,
                    })
                    .collect(),
            )
        };
    for condition in ["0", "00", "0.0", "off", "false", "0x0"] {
        assert_eq!(
            compiled_arguments("if", &[condition, "bad", "else", "good"]),
            known(&[3], &[], &[NativeCompiledBodyErrorContext::IfElse])
        );
        assert_eq!(
            compiled_arguments("while", &[condition, "bad"]),
            known(&[], &[], &[])
        );
    }
    for condition in ["0+0", "2-2", "$flag"] {
        assert_eq!(
            compiled_arguments("if", &[condition, "bad", "else", "good"]),
            known(
                &[1, 3],
                &[],
                &[
                    NativeCompiledBodyErrorContext::IfThen,
                    NativeCompiledBodyErrorContext::IfElse
                ]
            )
        );
        assert_eq!(
            compiled_arguments("while", &[condition, "bad"]),
            known(&[1], &[1], &[NativeCompiledBodyErrorContext::WhileBody])
        );
    }
    assert_eq!(
        compiled_arguments("if", &["1", "good", "else", "bad"]),
        known(&[1], &[], &[NativeCompiledBodyErrorContext::IfThen])
    );
    assert_eq!(
        compiled_arguments("for", &["init", "0", "next", "body"]),
        known(
            &[0, 3, 2],
            &[3, 2],
            &[
                NativeCompiledBodyErrorContext::ForInitial,
                NativeCompiledBodyErrorContext::ForBody,
                NativeCompiledBodyErrorContext::ForNext
            ]
        )
    );
    assert_eq!(
        compiled_arguments("foreach", &["x", "", "body"]),
        known(&[2], &[2], &[NativeCompiledBodyErrorContext::ForeachBody])
    );
    assert_eq!(
        compiled_arguments("catch", &["body"]),
        NativeCompiledBodies::Known(vec![NativeCompiledBodyOperand {
            argument: 0,
            context: NativeCompiledBodyContext::Inherit,
            error_context: NativeCompiledBodyErrorContext::None,
            failure_scope: NativeCompilationFailureScope::FallbackToGeneric
        }])
    );
}

#[test]
fn inline_exception_ranges_preserve_unknown_depth_and_checked_overflow() {
    let entry = context(NativeCompilationFrame::ProcedureCode);
    assert_eq!(entry.with_inline_exception_range().catch_depth, Some(1));
    let child = NativeCompiledBodyOperand {
        error_context: NativeCompiledBodyErrorContext::None,
        argument: 0,
        context: NativeCompiledBodyContext::Loop,
        failure_scope: NativeCompilationFailureScope::EnclosingChunk,
    };
    assert_eq!(child.entered_context(entry).unwrap().loop_depth, 1);
    assert_eq!(
        child.entered_context(NativeCompilationContext {
            loop_depth: u32::MAX,
            ..entry
        }),
        None
    );
    for depth in [None, Some(u32::MAX)] {
        let context = NativeCompilationContext {
            catch_depth: depth,
            ..entry
        };
        assert_eq!(context.with_inline_exception_range().catch_depth, None);
    }
}

#[test]
fn selected_interpreter_controls_independent_body_boundaries() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::NoHook,
        operation: SemanticOperationId::Invoke,
        body: NativeBodyCompilation::Uplevel,
    };
    let enclosing = context(NativeCompilationFrame::ProcedureCode);
    assert_eq!(
        spec.body_context(
            Some(InvocationDialect::for_version(TclVersion::V8_4)),
            enclosing
        )
        .mode,
        NativeCompilationMode::Direct
    );
    assert_eq!(
        spec.body_context(
            Some(InvocationDialect::for_version(TclVersion::V8_5)),
            enclosing
        ),
        context(NativeCompilationFrame::ScriptCode)
    );
    let profile = crate::model::ingress::resolve_environment("jim").unit_profile();
    assert_eq!(
        spec.body_context(Some(InvocationDialect::of_profile(profile)), enclosing)
            .mode,
        NativeCompilationMode::Direct
    );
    assert_eq!(
        spec.body_context(None, enclosing),
        NativeCompilationContext::default()
    );
}
