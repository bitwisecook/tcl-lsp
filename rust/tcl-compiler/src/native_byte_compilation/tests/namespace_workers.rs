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

//! Plans for `namespace` workers: the legacy `upvar` form and the captured 8.5 root with and without hooks.

use super::*;

#[test]
fn original_namespace_legacy_declines_non_upvar_before_emitter_admission() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.5").unwrap();
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for source in [
        b"namespace eval N {}".as_slice(),
        b"namespace {upvar} N x y",
        b"namespace upvar N x",
    ] {
        let parsed = native_script_words_in(
            SourceImage::native(source),
            tcl_lexer::Span::new(
                0,
                u32::try_from(source.len()).expect("fixture source fits Span"),
            ),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        assert_eq!(
            native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context),
            Ok(NativeByteCommandPlan::Generic)
        );
    }
    let source = b"namespace upvar N x y";
    let parsed = native_script_words_in(
        SourceImage::native(source.as_slice()),
        tcl_lexer::Span::new(
            0,
            u32::try_from(source.len()).expect("fixture source fits Span"),
        ),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap();
    let Ok(NativeByteCommandPlan::Registered(plan)) =
        native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context)
    else {
        panic!("native C8.5 accepted namespace upvar compiler");
    };
    assert!(matches!(
        plan.selection,
        NativeCompilationSelection::Inline { .. }
    ));
}

#[test]
fn captured_c85_namespace_root_retains_accepted_and_partial_upvar_preparation() {
    use tcl_registry::native_namespace_binding_compilation::{
        NativeNamespaceBindingKind, NativeNamespaceBindingOutcome,
    };
    let profile = tcl_dialect::DialectProfile::find("tcl8.5").unwrap();
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    let original_binding = entry
        .lookup_command_bytes(entry.current_namespace, b"namespace")
        .unwrap()
        .unwrap();
    assert!(
        original_binding
            .compiler
            .as_ref()
            .unwrap()
            .ensemble
            .is_none()
    );
    for (source, inline) in [
        (b"namespace upvar N x y".as_slice(), true),
        (b"namespace upvar N x y z bad(k)", false),
    ] {
        let parsed = native_script_words_in(
            SourceImage::native(source),
            Span::new(
                0,
                u32::try_from(source.len()).expect("fixture source fits Span"),
            ),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let words = &parsed.commands[0].words;
        let NativeByteCommandPlan::Registered(plan) =
            native_byte_command_plan(words, &entry, &registry, context).unwrap()
        else {
            panic!("original namespace compiler preparation must remain registered");
        };
        assert_eq!(&plan.binding, original_binding);
        assert_eq!(plan.spec.grammar, NativeCompilationGrammar::NamespaceLegacy);
        let captured =
            NativeCompilerWords::capture(words, entry.source_string_protocol.unwrap()).unwrap();
        let tcl_registry::native_instruction_plan::NativeInstructionPlan::NamespaceBindings(recipe) =
            tcl_registry::native_instruction_plan::native_instruction_plan(
                plan.spec,
                plan.selection,
                &captured,
                plan.operand_from,
                tcl_native_model::InvocationDialect::of_profile(profile),
                plan.context,
            )
            .unwrap()
        else {
            panic!("actual monolithic compiler must select original namespace binding visits");
        };
        assert_eq!(recipe.kind, NativeNamespaceBindingKind::Upvar);
        assert_eq!(
            recipe.outcome == NativeNamespaceBindingOutcome::Inline,
            inline
        );
        assert_ne!(
            recipe.visits,
            [] as [tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingVisit;
                0]
        );
    }
}

#[test]
fn captured_namespace_no_hook_workers_retain_original_named_rewrites() {
    use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let entry: crate::environment_ingress::CapturedNativeEntry =
            crate::environment_ingress::captured_native_entry(profile);
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        for source in [
            b"namespace eval N {}".as_slice(),
            b"namespace inscope N {}",
            b"namespace ensemble exists N",
        ] {
            let parsed = native_script_words_in(
                SourceImage::native(source),
                Span::new(
                    0,
                    u32::try_from(source.len()).expect("fixture source fits Span"),
                ),
                LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let NativeByteCommandPlan::Named(plan) =
                native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context)
                    .unwrap()
            else {
                panic!("original no-hook worker must retain the native named rewrite");
            };
            let worker = plan.prerequisite.selected_worker.as_ref().unwrap();
            assert_eq!(worker.compiler_hook, NativeCompilerHookPresence::Absent);
            assert_eq!(plan.recipe.arguments_from, 1);
            assert_eq!(
                plan.prerequisite.lookup_namespace_token,
                entry.current_namespace
            );
            assert_eq!(plan.prerequisite.interpreter, entry.interpreter);
        }
        assert!(
            registry
                .native_compilation_for_registration(
                    "tcl::namespace::unregistered",
                    tcl_native_model::InvocationDialect::of_profile(profile)
                )
                .is_none()
        );
    }
}
