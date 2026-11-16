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

//! Arity gates, guards and control-flow compilers: coroutine, uplevel, return and rejected selections.

use super::*;

#[test]
fn coroutine_compilation_preserves_native_arity_and_expansion_gates() {
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: NativeCompilationFrame::ScriptCode,
        ..NativeCompilationContext::default()
    };
    let registry = crate::model::ingress::static_context_for("tcl9.1").commands();
    let yield_spec = registry.get("yield").unwrap().native_compilation.unwrap();
    let relay = registry.get("yieldto").unwrap().native_compilation.unwrap();
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let dialect = Some(InvocationDialect::for_version(version));
        for count in 0..=2 {
            let values = vec!["value"; count];
            let shapes = vec![NativeCompilationWordShape::Substituted; count];
            let selected = yield_spec.select(
                InvocationWords::literals("yield", &values),
                &shapes,
                dialect,
                context,
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                count <= 1
            );
            let selected = relay.select(
                InvocationWords::literals("yieldto", &values),
                &shapes,
                dialect,
                context,
            );
            assert_eq!(
                matches!(selected, NativeCompilationSelection::Inline { .. }),
                count > 0 || version == TclVersion::V9_1
            );
        }
        let words = [crate::InvocationWord::Expanded];
        let selected = relay.select(
            InvocationWords::structured(crate::InvocationWord::Literal("yieldto"), &words),
            &[NativeCompilationWordShape::Expanded],
            dialect,
            context,
        );
        assert_eq!(
            matches!(selected, NativeCompilationSelection::Inline { .. }),
            version == TclVersion::V9_1
        );
    }
    assert_eq!(
        relay.select(
            InvocationWords::literals("yieldto", &["command"]),
            &[NativeCompilationWordShape::Opaque],
            Some(InvocationDialect::for_version(TclVersion::V9_1)),
            context,
        ),
        NativeCompilationSelection::Unknown
    );
}

#[test]
fn uplevel_compiler_preserves_original_frame_probe_and_release_floor() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::Uplevel,
        operation: SemanticOperationId::StructuredLowering(LoweringHookId::Uplevel),
        body: NativeBodyCompilation::Uplevel,
    };
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        for (first, valid) in [
            ("#0", true),
            ("set", true),
            ("", true),
            ("-0", true),
            ("0d0", true),
            ("1.0", true),
            ("-1", false),
            ("2147483648", false),
            ("#2147483648", false),
        ] {
            let selection = spec.select(
                InvocationWords::literals("uplevel", &[first, "set ::x RIGHT"]),
                &[NativeCompilationWordShape::BracedLiteral; 2],
                Some(dialect),
                context(NativeCompilationFrame::ProcedureCode),
            );
            assert_eq!(
                matches!(selection, NativeCompilationSelection::Inline { .. }),
                version == TclVersion::V9_1 && valid,
                "{version:?} first={first:?}",
            );
        }
        for (frame, first_shape) in [
            (
                NativeCompilationFrame::ScriptCode,
                NativeCompilationWordShape::Literal,
            ),
            (
                NativeCompilationFrame::ProcedureCode,
                NativeCompilationWordShape::Substituted,
            ),
        ] {
            assert_eq!(
                spec.select(
                    InvocationWords::literals("uplevel", &["#0", "set ::x RIGHT"]),
                    &[first_shape, NativeCompilationWordShape::Substituted],
                    Some(dialect),
                    context(frame),
                ),
                NativeCompilationSelection::Generic
            );
        }
    }
}

#[test]
fn checked_arity_retains_native_entry_error_and_argument_timing() {
    let registry = crate::model::ingress::static_context_for("tcl8.6").commands();
    let facts = registry
        .resolve_invocation("llength", &["items"], registry.own_surface_query())
        .unwrap()
        .facts();
    let spec = facts.native_compilation.unwrap();
    for version in TclVersion::ALL {
        let dialect = InvocationDialect::for_version(version);
        let entry = context(NativeCompilationFrame::ScriptCode);
        assert!(matches!(
            spec.select(
                InvocationWords::literals("renamed", &["items"]),
                &[NativeCompilationWordShape::Substituted],
                Some(dialect),
                entry,
            ),
            NativeCompilationSelection::Inline { .. }
        ));
        let selection = spec.select(
            InvocationWords::literals("renamed", &["items", "extra"]),
            &[NativeCompilationWordShape::Substituted; 2],
            Some(dialect),
            entry,
        );
        assert_eq!(
            selection,
            if version == TclVersion::V8_4 {
                NativeCompilationSelection::CompileError
            } else {
                NativeCompilationSelection::Generic
            }
        );
        assert_eq!(
            spec.failure_for_selection(
                selection,
                InvocationWords::literals("renamed", &["items", "extra"]),
                &facts,
                Some(dialect),
            )
            .is_some(),
            version == TclVersion::V8_4,
        );
    }
}

#[test]
fn known_return_options_have_their_actual_versioned_compiler_protocol() {
    let registry = crate::CommandRegistry::build_default();
    let spec = registry.get("return").unwrap().native_compilation.unwrap();
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let dialect = InvocationDialect::for_version(version);
        let selected = spec.select(
            InvocationWords::literals("return", &["-code", "error", "absent"]),
            &[NativeCompilationWordShape::Literal; 3],
            Some(dialect),
            context(NativeCompilationFrame::ProcedureCode),
        );
        if version == TclVersion::V8_4 {
            assert_eq!(selected, NativeCompilationSelection::Generic);
        } else {
            assert!(matches!(
                selected,
                NativeCompilationSelection::Inline { .. }
            ));
        }
        assert_eq!(
            spec.select(
                InvocationWords::literals("return", &["-level", "-1", "absent"]),
                &[NativeCompilationWordShape::Literal; 3],
                Some(dialect),
                context(NativeCompilationFrame::ProcedureCode),
            ),
            NativeCompilationSelection::Generic
        );
    }
}

#[test]
fn compiler_rejection_keeps_native_usage_after_command_renaming() {
    let registry =
        crate::cache::registry_for_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
    let arguments = ["x", "extra", "bad"];
    let facts = registry
        .resolve_invocation("set", &arguments, registry.own_surface_query())
        .unwrap()
        .facts();
    let spec = registry.get("set").unwrap().native_compilation.unwrap();
    let words = InvocationWords::literals("renamed", &arguments);
    let dialect = Some(InvocationDialect::for_version(TclVersion::V8_4));
    let selected = spec.select(
        words,
        &[NativeCompilationWordShape::Literal; 3],
        dialect,
        context(NativeCompilationFrame::ProcedureCode),
    );
    assert_eq!(selected, NativeCompilationSelection::CompileError);
    assert_eq!(
        spec.failure_for_selection(selected, words, &facts, dialect),
        Some(NativeCompilationFailure {
            message: Some("wrong # args: should be \"set varName ?newValue?\"".into()),
            error_code: Some("NONE".into()),
            error_info: None,
        })
    );
    assert_eq!(
        spec.compiled_bodies(
            NativeCompilationSelection::Unknown,
            words,
            &[],
            &facts,
            dialect
        ),
        NativeCompiledBodies::Unknown
    );
}

#[test]
fn selected_opcode_guard_precedes_substituted_arguments() {
    let spec = NativeCompilationSpec {
        grammar: NativeCompilationGrammar::VariableLoadStore,
        operation: SemanticOperationId::StructuredLowering(LoweringHookId::Set),
        body: NativeBodyCompilation::Inherit,
    };
    assert!(spec.requires_original_word_preparation());
    let arguments = [
        crate::InvocationWord::Literal("x"),
        crate::InvocationWord::Dynamic,
    ];
    let words = InvocationWords::structured(crate::InvocationWord::Literal("set"), &arguments);
    let shapes = [
        NativeCompilationWordShape::Literal,
        NativeCompilationWordShape::Substituted,
    ];
    for version in TclVersion::ALL {
        assert_eq!(
            spec.select(
                words,
                &shapes,
                Some(InvocationDialect::for_version(version)),
                context(NativeCompilationFrame::ScriptCode)
            ),
            NativeCompilationSelection::Inline {
                operation: spec.operation,
                guard: NativeCompilationGuard::for_c_version(version),
            }
        );
    }
}
