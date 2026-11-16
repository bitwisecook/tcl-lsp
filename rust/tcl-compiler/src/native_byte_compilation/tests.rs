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

use super::*;
use tcl_core_types::{ByteNamespacePath, NameBytes, NativeByteCommandSlot};
use tcl_lexer::{Lexer, LexerConfig, group_commands_bytes};
use tcl_runtime_api::native_compilation::{
    NativeCommandImplementation, NativeCompilationBinding, NativeCompilationFrame,
    NativeCompilationNamespace, NativeCompilerHookPresence, NativeInterpreterIdentity,
    NativeVariableObserverPresence,
};

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
fn original_monolithic_match_retains_real_hook_and_registration_guards() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let source = b"string match * $subject";
    let parsed = native_script_words_in(
        SourceImage::native(source.as_slice()),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    let plan =
        native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context).unwrap();
    let NativeByteCommandPlan::Registered(selected) = plan else {
        panic!("original monolithic StringMatch compiler");
    };
    assert!(matches!(
        selected.spec.grammar,
        NativeCompilationGrammar::StringMatch(
            tcl_registry::native_string_compilation::NativeStringMatchScope::PublicMember
        )
    ));
    assert_eq!(
        selected.binding.compiler_hook,
        NativeCompilerHookPresence::Present
    );
    let original = entry
        .lookup_command_bytes(entry.current_namespace, b"string")
        .unwrap()
        .unwrap();
    assert_eq!(selected.binding.token, original.token);
    assert_eq!(selected.binding.compiler, original.compiler);
    for presence in [
        NativeCompilerHookPresence::Absent,
        NativeCompilerHookPresence::Unknown,
    ] {
        let mut changed = entry.clone();
        changed
            .commands
            .iter_mut()
            .find(|binding| binding.token == original.token)
            .unwrap()
            .compiler_hook = presence;
        let plan =
            native_byte_command_plan(&parsed.commands[0].words, &changed, &registry, context);
        if presence == NativeCompilerHookPresence::Absent {
            assert_eq!(plan.unwrap(), NativeByteCommandPlan::Generic);
        } else {
            assert!(plan.is_err());
        }
    }
}

#[test]
fn registered_worker_recipe_preserves_purpose_and_rejects_changed_original_basis() {
    use tcl_registry::native_compilation::NativeCompilationFrame;
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let entry: crate::environment_ingress::CapturedNativeEntry =
            crate::environment_ingress::captured_native_entry(profile);
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (source, frame) in [
            (
                b"array set event_handlers {}".as_slice(),
                NativeCompilationFrame::ScriptCode,
            ),
            (
                b"array set event_handlers {name handler}".as_slice(),
                NativeCompilationFrame::ProcedureCode,
            ),
            (
                b"array exists event_handlers".as_slice(),
                NativeCompilationFrame::ScriptCode,
            ),
        ] {
            let parsed = native_script_words_in(
                SourceImage::native(source),
                Span::new(0, u32::try_from(source.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            let words = &parsed.commands[0].words;
            let context = NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame,
                ..Default::default()
            };
            let NativeByteCommandPlan::Registered(plan) =
                native_byte_command_plan(words, &entry, &registry, context).unwrap()
            else {
                panic!("{version:?}: actual Array worker compiler");
            };
            assert!(
                plan.prerequisite
                    .as_ref()
                    .unwrap()
                    .selected_worker
                    .is_some()
            );
            assert!(matches!(
                plan.original_instruction(words),
                Some(NativeInstructionPlan::Array(_))
            ));
            assert!(registered_operand_available(words, &entry, &plan));
            let changed_source = source.iter().copied().chain(*b" ").collect::<Vec<_>>();
            let changed = native_script_words_in(
                SourceImage::native(changed_source.as_slice()),
                Span::new(0, u32::try_from(changed_source.len()).unwrap()),
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            )
            .unwrap();
            assert!(
                plan.original_instruction(&changed.commands[0].words)
                    .is_none()
            );
            assert!(!registered_operand_available(
                &changed.commands[0].words,
                &entry,
                &plan
            ));
            let mut changed_basis = (*plan).clone();
            changed_basis.operand_from += 1;
            assert!(changed_basis.original_instruction(words).is_none());
            changed_basis = (*plan).clone();
            changed_basis.context.frame = NativeCompilationFrame::Unknown;
            assert!(changed_basis.original_instruction(words).is_none());
        }
    }
}

#[test]
fn registered_concat_uses_shared_recipe_and_original_worker_authority() {
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in tcl_dialect::TclVersion::ALL {
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
        let mut entry: crate::environment_ingress::CapturedNativeEntry =
            crate::environment_ingress::captured_native_entry(profile);
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for source in [
            b"concat { A } { B }".as_slice(),
            b"concat $a $b",
            b"concat {*}{A B} $c",
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
            if version == tcl_dialect::TclVersion::V8_4 && source == b"concat {*}{A B} $c" {
                // The original C8.4 parser rejects the adjacent brace
                // groups before a compiler receives a command vector.
                assert!(parsed.fatal_tail.is_some());
                assert_eq!(
                    parsed.commands,
                    [] as [tcl_lexer::NativeScriptCommandWords; 0]
                );
                continue;
            }
            let plan =
                native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context)
                    .unwrap();
            if version < tcl_dialect::TclVersion::V8_6 {
                assert_eq!(plan, NativeByteCommandPlan::Generic);
            } else {
                assert!(
                    matches!(plan, NativeByteCommandPlan::Registered(_)),
                    "{version:?}: {source:?}"
                );
            }
        }
        let source = b"concat $a $b";
        let parsed = native_script_words_in(
            SourceImage::native(source.as_slice()),
            tcl_lexer::Span::new(
                0,
                u32::try_from(source.len()).expect("fixture source fits Span"),
            ),
            LexerConfig::from_grammar(profile.grammar),
        )
        .unwrap();
        let target = entry
            .commands
            .iter_mut()
            .find(|binding| binding.slot.simple.as_bytes() == b"concat")
            .unwrap();
        target.implementation_generation += 1;
        target.implementation = NativeCommandImplementation::Opaque;
        target.compiler_hook = NativeCompilerHookPresence::Absent;
        target.compiler = None;
        assert_eq!(
            native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context),
            Ok(NativeByteCommandPlan::Generic)
        );
        target_unknown(&mut entry, b"concat");
        assert_eq!(
            native_byte_command_plan(&parsed.commands[0].words, &entry, &registry, context),
            Err(NativeByteCommandUnavailable::Registration)
        );
    }
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
                tcl_registry::InvocationDialect::of_profile(profile),
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
                    tcl_registry::InvocationDialect::of_profile(profile)
                )
                .is_none()
        );
    }
}

fn target_unknown(entry: &mut NativeCompilationEntry, name: &[u8]) {
    entry
        .commands
        .iter_mut()
        .find(|binding| binding.slot.simple.as_bytes() == name)
        .unwrap()
        .compiler_hook = NativeCompilerHookPresence::Unknown;
}

#[test]
fn registered_recipe_without_shared_instruction_remains_unavailable() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let source = b"llength $items";
    let parsed = native_script_words_in(
        SourceImage::native(source.as_slice()),
        tcl_lexer::Span::new(
            0,
            u32::try_from(source.len()).expect("fixture source fits Span"),
        ),
        LexerConfig::from_grammar(profile.grammar),
    )
    .unwrap();
    assert_eq!(
        native_byte_command_plan(
            &parsed.commands[0].words,
            &entry,
            &registry,
            NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                ..Default::default()
            }
        ),
        Err(NativeByteCommandUnavailable::Recipe)
    );
}

fn entry(version: tcl_dialect::TclVersion) -> NativeCompilationEntry {
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
    let point = tcl_dialect::model::DialectPoint::for_tcl_version(version);
    NativeCompilationEntry {
        interpreter: NativeInterpreterIdentity {
            owner: 7,
            interpreter: 0,
        },
        epoch: 3,
        profile: profile.cache_key(),
        invocation_policy: Some(profile.cache_key()),
        expression_policy:
            tcl_registry::native_expression_program::native_expression_evaluation_policy(
                profile, point,
            ),
        execution_point: Some(point),
        execution_name_policy: None,
        name_protocol: tcl_syntax::naming::NamePolicyProtocol::for_native_point(point),
        compiled_variable_protocol:
            tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(point),
        compiled_local_layout: None,
        oo_classes: None,
        ensemble_target_objects: None,
        source_string_protocol: tcl_registry::InvocationDialect::of_point(point)
            .native_source_string_protocol(),
        lexer_grammar: Some(profile.grammar),
        inline_compilation_disabled: false,
        direct_evaluation: false,
        authored_tmm_static: None,
        namespace_variable_tables: None,
        empty_literal_world: None,
        compiler_pass_environment: None,
        command_resolvers: None,
        variable_observers: NativeVariableObserverPresence::Unknown,
        math_functions: None,
        closed: true,
        commands: vec![].into(),
        namespaces: vec![namespace(1, ByteNamespacePath::root(), true)],
        current_namespace: 1,
        frame: NativeCompilationFrame::Global,
    }
}

fn namespace(token: u64, path: ByteNamespacePath, visible: bool) -> NativeCompilationNamespace {
    NativeCompilationNamespace {
        path,
        token,
        visible,
        jim_namespace_object: None,
        exports: vec![],
        command_path: vec![],
        unknown_handler: None,
    }
}

fn binding(
    namespace: &NativeCompilationNamespace,
    name: &[u8],
    hook: NativeCompilerHookPresence,
) -> NativeCompilationBinding {
    NativeCompilationBinding {
        slot: NativeByteCommandSlot::new(namespace.path.clone(), NameBytes::from(name)),
        namespace_token: namespace.token,
        token: 11,
        implementation_generation: 4,
        implementation: NativeCommandImplementation::Opaque,
        compiler_hook: hook,
        compiler: None,
        procedure_header: None,
        has_execution_trace: false,
    }
}

fn install_original_target_fixture(entry: &mut NativeCompilationEntry) {
    let mut identity = 1;
    entry.ensemble_target_objects = Some(entry.commands.iter().flat_map(|command| {
            command.compiler.as_ref().and_then(|compiler| compiler.ensemble.as_ref()).into_iter().flat_map(move |configuration| {
                configuration.map.iter().flat_map(move |(member, prefix)| prefix.iter().enumerate().map(move |(position, name)| (command.token, member.clone(), position, name.clone())))
            })
        }).map(|(token, member, position, name)| {
            let observation = tcl_runtime_api::native_compilation::NativeEnsembleTargetObservation {
                ensemble_token: token, member, prefix_index: position, object_identity: identity,
                resident_name: name.clone(), primary: if name.is_some() { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::StockString } else { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::Unavailable },
            };
            identity += 1; observation
        }).collect());
}

fn source_module(source: &[u8], entry: &NativeCompilationEntry) -> crate::ir::Module {
    let image = SourceImage::native(source);
    let config = LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
    let tokens = Lexer::with_source_image(&image, config)
        .tokenise_all()
        .unwrap();
    let statements = group_commands_bytes(&tokens, image.bytes(), config)
        .into_iter()
        .map(|group| crate::ir::Statement::NativeCall {
            span: group.span,
            words: group
                .words
                .iter()
                .map(|word| NativeWord::from_group(image.clone(), config, &tokens, word).unwrap())
                .collect(),
        })
        .collect();
    let mut module = crate::ir::Module {
        source: image,
        native_namespace: Some(
            entry
                .namespaces
                .iter()
                .find(|row| row.token == entry.current_namespace)
                .unwrap()
                .path
                .clone(),
        ),
        ..crate::ir::Module::default()
    };
    module.top_level.statements = statements;
    module
}

#[test]
fn original_generic_selection_precedes_inline_emitter_availability() {
    use tcl_runtime_api::native_compilation::NativeCommandCompiler;
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in tcl_dialect::TclVersion::ALL {
        let mut entry = entry(version);
        let mut target = binding(
            &entry.namespaces[0],
            b"original_worker",
            if version < tcl_dialect::TclVersion::V9_1 {
                NativeCompilerHookPresence::Absent
            } else {
                NativeCompilerHookPresence::Present
            },
        );
        target.compiler = Some(NativeCommandCompiler {
            registry_identity: "uplevel".into(),
            ensemble: None,
        });
        entry.commands.push(target);
        let module = source_module(b"original_worker 1 {error BODY}", &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("original words");
        };
        let selected = native_byte_command_plan(words, &entry, &registry, context);
        if version < tcl_dialect::TclVersion::V9_1 {
            assert!(
                matches!(selected, Ok(NativeByteCommandPlan::Generic)),
                "{version:?}"
            );
        } else {
            let NativeByteCommandPlan::Registered(plan) = selected.unwrap() else {
                panic!("C9.1 original Uplevel compiler recipe");
            };
            assert_eq!(plan.binding, entry.commands[0]);
            assert_eq!(plan.context, context);
            assert_eq!(plan.operand_from, 1);
            assert_eq!(
                plan.dependencies,
                [] as [tcl_runtime_api::native_compilation::NativeCompilationBinding; 0]
            );
            assert!(
                plan.prerequisite.is_none(),
                "ordinary compiler has no delegated worker"
            );
            let original =
                NativeCompilerWords::capture(words, entry.source_string_protocol.unwrap()).unwrap();
            assert_eq!(
                tcl_registry::native_instruction_plan::native_instruction_plan(
                    plan.spec,
                    plan.selection,
                    &original,
                    plan.operand_from,
                    InvocationDialect::for_version(version),
                    context,
                )
                .unwrap(),
                tcl_registry::native_instruction_plan::NativeInstructionPlan::Uplevel(
                    tcl_registry::native_instruction_plan::NativeUplevelInstruction {
                        level_word: Some(1),
                        script_words: 2..3,
                    },
                ),
            );
        }
        if version == tcl_dialect::TclVersion::V9_1 {
            let script_context = NativeCompilationContext {
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..context
            };
            assert!(matches!(
                native_byte_command_plan(words, &entry, &registry, script_context),
                Ok(NativeByteCommandPlan::Generic)
            ));
            entry.commands[0].compiler = None;
            assert!(matches!(
                native_byte_command_plan(words, &entry, &registry, context),
                Err(NativeByteCommandUnavailable::Registration)
            ));
        }
    }
}

#[test]
fn original_dynamic_expanded_head_keeps_generic_timing_and_literal_cpp_obligations() {
    // Native proof naming.command-head.literal-expansion-and-inline-replacement:
    // docs/design/analysis/name-resolution-proofs/command-head-literal-expansion-and-inline-replacement.md
    // Native proof naming.command-head.dynamic-expansion-after-replacement:
    // docs/design/analysis/name-resolution-proofs/command-head-dynamic-expansion-after-replacement.md
    // Native proof naming.command-head.empty-dynamic-expansion-exposes-written-head:
    // docs/design/analysis/name-resolution-proofs/command-head-empty-dynamic-expansion-exposes-written-head.md
    // Native six-engine control: tests/data/native_expanded_head/observations.tsv.
    // Modern C uses invokeExpanded for a dynamic head and storeScalar for
    // the literal parser-projected head, including command replacement.
    let observations = include_str!("../../tests/data/native_expanded_head/observations.tsv");
    assert_eq!(observations.lines().count(), 6);
    assert!(
        observations
            .lines()
            .any(|row| row.starts_with("8.4|0|0|1|"))
    );
    assert!(
        observations
            .lines()
            .any(|row| row.starts_with("jim|0|0|0|A|1|"))
    );
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        let observation = observations
            .lines()
            .find(|row| row.starts_with(&format!("{}|", version.version_string())))
            .unwrap();
        let fields: Vec<_> = observation.split('|').collect();
        assert_eq!(
            &fields[1..3],
            &["1", "1"],
            "dynamic dispatch and literal inline CPP"
        );
        assert_eq!(
            &fields[9..13],
            &[
                "1",
                "{too many nested evaluations (infinite loop?)}",
                "0",
                "A"
            ]
        );
        let entry = entry(version);
        for source in [b"{*}$command VALUE".as_slice(), b"{*}$command set x A"] {
            let module = source_module(source, &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("original expanded command");
            };
            assert_eq!(
                native_byte_command_plan(words, &entry, &registry, context).unwrap(),
                NativeByteCommandPlan::Generic,
                "{version:?}"
            );
            assert!(closed_original_byte_compilation(
                &module, &entry, &registry, context
            ));
            assert!(closed_generic_byte_compilation(&module, &entry));
            let mut missing_point = entry.clone();
            missing_point.execution_point = None;
            assert_eq!(
                native_byte_command_plan(words, &missing_point, &registry, context),
                Err(NativeByteCommandUnavailable::Entry)
            );
            let mut unsupported = entry.clone();
            unsupported.execution_point = Some(tcl_dialect::model::DialectPoint::for_tcl_version(
                tcl_dialect::TclVersion::V8_4,
            ));
            unsupported.name_protocol = tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                unsupported.execution_point.unwrap(),
            );
            assert_eq!(
                native_byte_command_plan(words, &unsupported, &registry, context),
                Err(NativeByteCommandUnavailable::Recipe)
            );
        }
        for source in [b"{*}{set x} A".as_slice(), b"{*}{} set x A"] {
            let module = source_module(source, &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("original projected command");
            };
            assert_eq!(
                native_byte_command_plan(words, &entry, &registry, context),
                Err(NativeByteCommandUnavailable::Recipe)
            );
            assert!(!closed_generic_byte_compilation(&module, &entry));
        }
    }
}

#[test]
fn byte_generic_certificate_requires_original_absent_hook_or_closed_absence() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let module = source_module(b"opaque\xff ARG", &entry);
    assert!(closed_generic_byte_compilation(&module, &entry));
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"opaque\xff",
        NativeCompilerHookPresence::Absent,
    ));
    assert!(closed_generic_byte_compilation(&module, &entry));
    for hook in [
        NativeCompilerHookPresence::Present,
        NativeCompilerHookPresence::Unknown,
    ] {
        entry.commands[0].compiler_hook = hook;
        assert!(!closed_generic_byte_compilation(&module, &entry));
    }
    entry.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
    entry.closed = false;
    assert!(!closed_generic_byte_compilation(&module, &entry));
    entry.closed = true;
    entry.execution_point = None;
    assert!(!closed_generic_byte_compilation(&module, &entry));
}

#[test]
fn configured_byte_worker_uses_its_installed_named_compiler() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut entry = entry(version);
        let mut worker = binding(
            &entry.namespaces[0],
            b"private",
            NativeCompilerHookPresence::Present,
        );
        worker.token = 12;
        worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "::tcl::info::locals".into(),
            ensemble: None,
        });
        let mut public = binding(
            &entry.namespaces[0],
            b"ensemble",
            NativeCompilerHookPresence::Present,
        );
        public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "custom-ensemble".into(),
            ensemble: Some(
                tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                    namespace_token: entry.current_namespace,
                    map: vec![("member".into(), vec![Some("private".into())])],
                    subcommands: None,
                    prefixes: true,
                    parameters: vec![],
                    unknown_handler: None,
                },
            ),
        });
        entry.commands.extend([public, worker.clone()]);
        install_original_target_fixture(&mut entry);
        for (source, accepted) in [
            (b"ensemble mem $pattern".as_slice(), true),
            (b"ensemble member a b".as_slice(), false),
        ] {
            let module = source_module(source, &entry);
            let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0]
            else {
                panic!("original native words")
            };
            let result = native_byte_command_plan(words, &entry, &registry, context).unwrap();
            let NativeByteCommandPlan::Named(selected) = result else {
                panic!("installed worker compiler")
            };
            assert_eq!(
                selected.recipe.protocol,
                if accepted {
                    tcl_registry::native_compilation::NativeNamedInvocationProtocol::Direct
                } else {
                    tcl_registry::native_compilation::NativeNamedInvocationProtocol::EnsembleRewrite
                }
            );
            assert_eq!(selected.recipe.name, b"::private");
            assert_eq!(
                selected.prerequisite.selected_worker.as_ref(),
                Some(&worker)
            );
            if accepted {
                assert_eq!(selected.recipe.words, [tcl_registry::native_instruction_plan::NativeNamedInvocationWord::Original(tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(2))]);
            } else {
                assert_eq!(selected.recipe.words.len(), 4);
            }
            assert!(closed_original_byte_compilation(
                &module, &entry, &registry, context
            ));
        }
    }
}

fn info_errorstack_entry() -> (NativeCompilationEntry, NativeCompilationBinding) {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut worker = binding(
        &entry.namespaces[0],
        b"private",
        NativeCompilerHookPresence::Present,
    );
    worker.token = 12;
    worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "::tcl::info::errorstack".into(),
        ensemble: None,
    });
    let mut public = binding(
        &entry.namespaces[0],
        b"info",
        NativeCompilerHookPresence::Present,
    );
    public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "info".into(),
        ensemble: Some(
            tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                namespace_token: entry.current_namespace,
                map: vec![("errorstack".into(), vec![Some("private".into())])],
                subcommands: None,
                prefixes: true,
                parameters: vec![],
                unknown_handler: None,
            },
        ),
    });
    entry.commands.extend([public, worker.clone()]);
    install_original_target_fixture(&mut entry);
    (entry, worker)
}

#[test]
fn info_errorstack_keeps_original_worker_and_wrong_arity_fallback() {
    use tcl_registry::native_compilation::NativeNamedInvocationProtocol as Protocol;
    use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand as Word;
    use tcl_registry::native_instruction_plan::NativeNamedInvocationWord;
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    let (mut entry, worker) = info_errorstack_entry();
    for (source, protocol, operands) in [
        (b"info errorstack".as_slice(), Protocol::Direct, 0),
        (b"info errorstack $interp".as_slice(), Protocol::Direct, 1),
        (
            b"info errorstack [lappend ::seen first] [lappend ::seen second]".as_slice(),
            Protocol::EnsembleRewrite,
            4,
        ),
    ] {
        let module = source_module(source, &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("original native words")
        };
        let NativeByteCommandPlan::Named(selected) =
            native_byte_command_plan(words, &entry, &registry, context).unwrap()
        else {
            panic!("selected worker")
        };
        assert_eq!(selected.recipe.protocol, protocol);
        assert_eq!(selected.recipe.name, b"::private");
        assert_eq!(selected.recipe.words.len(), operands);
        assert_eq!(
            selected.prerequisite.selected_worker.as_ref(),
            Some(&worker)
        );
        if protocol == Protocol::EnsembleRewrite {
            assert_eq!(
                selected.recipe.words,
                [
                    NativeNamedInvocationWord::Original(Word::Original(0)),
                    NativeNamedInvocationWord::Replacement(b"errorstack".to_vec()),
                    NativeNamedInvocationWord::Original(Word::Original(2)),
                    NativeNamedInvocationWord::Original(Word::Original(3)),
                ]
            );
        }
        if source == b"info errorstack $interp" {
            assert_eq!(
                selected.recipe.words,
                [NativeNamedInvocationWord::Original(Word::Original(2))]
            );
        }
    }
    let module = source_module(b"info errorstack $interp", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
        panic!("original native words")
    };
    let NativeByteCommandPlan::Named(original) =
        native_byte_command_plan(words, &entry, &registry, context).unwrap()
    else {
        panic!("original worker")
    };
    entry.commands[1].token += 1;
    let NativeByteCommandPlan::Named(rebound) =
        native_byte_command_plan(words, &entry, &registry, context).unwrap()
    else {
        panic!("rebound worker")
    };
    assert_ne!(
        original.prerequisite.selected_worker,
        rebound.prerequisite.selected_worker
    );
    entry.commands[1].has_execution_trace = true;
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context).unwrap(),
        NativeByteCommandPlan::Generic
    );
    entry.commands[1].has_execution_trace = false;
    entry.commands[1]
        .compiler
        .as_mut()
        .unwrap()
        .registry_identity = "::foreign::errorstack".into();
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Err(NativeByteCommandUnavailable::Recipe)
    );
}

#[test]
fn dictionary_named_fallback_retains_worker_and_ordered_original_operands() {
    use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand as Word;
    use tcl_registry::native_instruction_plan::NativeNamedInvocationWord;
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut entry = entry(version);
        let mut worker = binding(
            &entry.namespaces[0],
            b"private",
            NativeCompilerHookPresence::Present,
        );
        worker.token = 12;
        worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "::tcl::dict::incr".into(),
            ensemble: None,
        });
        let mut public = binding(
            &entry.namespaces[0],
            b"ensemble",
            NativeCompilerHookPresence::Present,
        );
        public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "custom-ensemble".into(),
            ensemble: Some(
                tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                    namespace_token: entry.current_namespace,
                    map: vec![("member".into(), vec![Some("private".into())])],
                    subcommands: None,
                    prefixes: true,
                    parameters: vec![],
                    unknown_handler: None,
                },
            ),
        });
        entry.commands.extend([public, worker.clone()]);
        install_original_target_fixture(&mut entry);
        let module = source_module(b"ensemble member $variable [side] 5", &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("original source")
        };
        let NativeByteCommandPlan::Named(original) =
            native_byte_command_plan(words, &entry, &registry, context).unwrap()
        else {
            panic!("selected named fallback {version:?}")
        };
        assert_eq!(
            original.recipe.words,
            [2, 3, 4].map(|i| NativeNamedInvocationWord::Original(Word::Original(i)))
        );
        assert_eq!(
            original.prerequisite.selected_worker.as_ref(),
            Some(&worker)
        );
        entry.commands[1].token += 1;
        let NativeByteCommandPlan::Named(rebound) =
            native_byte_command_plan(words, &entry, &registry, context).unwrap()
        else {
            panic!("rebound worker")
        };
        assert_ne!(
            original.prerequisite.selected_worker,
            rebound.prerequisite.selected_worker
        );
        entry.commands[1].has_execution_trace = true;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context).unwrap(),
            NativeByteCommandPlan::Generic
        );
        entry.commands[1].has_execution_trace = false;
        entry.commands[1]
            .compiler
            .as_mut()
            .unwrap()
            .registry_identity = "::foreign::incr".into();
        assert!(native_byte_command_plan(words, &entry, &registry, context).is_err());
    }
}

#[test]
fn empty_string_map_identity_requires_original_worker_and_source() {
    use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand as Word;
    use tcl_registry::native_instruction_plan::NativeInstructionPlan;
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut worker = binding(
        &entry.namespaces[0],
        b"private",
        NativeCompilerHookPresence::Present,
    );
    worker.token = 12;
    worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "::tcl::string::map".into(),
        ensemble: None,
    });
    let mut public = binding(
        &entry.namespaces[0],
        b"ensemble",
        NativeCompilerHookPresence::Present,
    );
    public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "custom-ensemble".into(),
        ensemble: Some(
            tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                namespace_token: entry.current_namespace,
                map: vec![("member".into(), vec![Some("private".into())])],
                subcommands: None,
                prefixes: true,
                parameters: vec![],
                unknown_handler: None,
            },
        ),
    });
    entry.commands.extend([public, worker.clone()]);
    install_original_target_fixture(&mut entry);
    let module = source_module(b"ensemble member {{} x} $subject", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
        panic!("original source")
    };
    let NativeByteCommandPlan::Registered(original) =
        native_byte_command_plan(words, &entry, &registry, context).unwrap()
    else {
        panic!("selected identity")
    };
    assert_eq!(
        original.original_instruction(words),
        Some(&NativeInstructionPlan::StringMap(
            tcl_registry::native_string_map_compilation::NativeStringMapInstruction::Identity(
                Word::Original(3)
            )
        ))
    );
    assert_eq!(
        original
            .prerequisite
            .as_ref()
            .unwrap()
            .selected_worker
            .as_ref(),
        Some(&worker)
    );
    let foreign = source_module(b"ensemble member {{} x} $other", &entry);
    let crate::ir::Statement::NativeCall {
        words: foreign_words,
        ..
    } = &foreign.top_level.statements[0]
    else {
        panic!("foreign source")
    };
    assert!(original.original_instruction(foreign_words).is_none());
    entry.commands[1].has_execution_trace = true;
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context).unwrap(),
        NativeByteCommandPlan::Generic
    );
    entry.commands[1].has_execution_trace = false;
    entry.commands[1]
        .compiler
        .as_mut()
        .unwrap()
        .registry_identity = "::foreign::map".into();
    assert!(native_byte_command_plan(words, &entry, &registry, context).is_err());
}

#[test]
fn string_map_dynamic_fallback_retains_worker_and_ordered_original_operands() {
    use tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand as Word;
    use tcl_registry::native_instruction_plan::NativeNamedInvocationWord;
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let version = tcl_dialect::TclVersion::V9_0;
    let mut entry = entry(version);
    let mut worker = binding(
        &entry.namespaces[0],
        b"private",
        NativeCompilerHookPresence::Present,
    );
    worker.token = 12;
    worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "::tcl::string::map".into(),
        ensemble: None,
    });
    let mut public = binding(
        &entry.namespaces[0],
        b"ensemble",
        NativeCompilerHookPresence::Present,
    );
    public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "custom-ensemble".into(),
        ensemble: Some(
            tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                namespace_token: entry.current_namespace,
                map: vec![("member".into(), vec![Some("private".into())])],
                subcommands: None,
                prefixes: true,
                parameters: vec![],
                unknown_handler: None,
            },
        ),
    });
    entry.commands.extend([public, worker.clone()]);
    install_original_target_fixture(&mut entry);
    let module = source_module(b"ensemble member $mapping [side]", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
        panic!("original source")
    };
    let NativeByteCommandPlan::Named(original) =
        native_byte_command_plan(words, &entry, &registry, context).unwrap()
    else {
        panic!("selected named fallback {version:?}")
    };
    assert_eq!(
        original.recipe.words,
        [2, 3].map(|i| NativeNamedInvocationWord::Original(Word::Original(i)))
    );
    assert_eq!(
        original.prerequisite.selected_worker.as_ref(),
        Some(&worker)
    );
    entry.commands[1].token += 1;
    let NativeByteCommandPlan::Named(rebound) =
        native_byte_command_plan(words, &entry, &registry, context).unwrap()
    else {
        panic!("rebound worker")
    };
    assert_ne!(
        original.prerequisite.selected_worker,
        rebound.prerequisite.selected_worker
    );
    entry.commands[1].has_execution_trace = true;
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context).unwrap(),
        NativeByteCommandPlan::Generic
    );
    entry.commands[1].has_execution_trace = false;
    entry.commands[1]
        .compiler
        .as_mut()
        .unwrap()
        .registry_identity = "::foreign::map".into();
    assert!(native_byte_command_plan(words, &entry, &registry, context).is_err());
}

#[test]
fn dynamic_string_is_decline_is_private_and_does_not_admit_ensemble_workers() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut worker = binding(
        &entry.namespaces[0],
        b"private",
        NativeCompilerHookPresence::Present,
    );
    worker.token = 12;
    worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "::tcl::string::is".into(),
        ensemble: None,
    });
    let mut public = binding(
        &entry.namespaces[0],
        b"ensemble",
        NativeCompilerHookPresence::Present,
    );
    public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "custom-ensemble".into(),
        ensemble: Some(
            tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                namespace_token: entry.current_namespace,
                map: vec![("member".into(), vec![Some("private".into())])],
                subcommands: None,
                prefixes: true,
                parameters: vec![],
                unknown_handler: None,
            },
        ),
    });
    entry.commands.extend([public, worker]);
    install_original_target_fixture(&mut entry);
    let direct = source_module(b"private $class -strict $subject", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &direct.top_level.statements[0] else {
        panic!("original private call")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context).unwrap(),
        NativeByteCommandPlan::Generic
    );
    let public = source_module(b"ensemble member $class -strict $subject", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &public.top_level.statements[0] else {
        panic!("original public call")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Err(NativeByteCommandUnavailable::Recipe)
    );
    entry.commands[0]
        .compiler
        .as_mut()
        .unwrap()
        .ensemble
        .as_mut()
        .unwrap()
        .map[0]
        .1
        .push(Some("prefix".into()));
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
}

#[test]
fn dynamic_format_word_decline_keeps_original_dispatch_owner() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut format = binding(
        &entry.namespaces[0],
        b"format",
        NativeCompilerHookPresence::Present,
    );
    format.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "format".into(),
        ensemble: None,
    });
    entry.commands.push(format);
    let module = source_module(b"format [side format] [side value]", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
        panic!("original format invocation")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
    entry.commands[0].token += 1;
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
    entry.commands[0].has_execution_trace = true;
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
    entry.commands[0].has_execution_trace = false;
    entry.commands[0]
        .compiler
        .as_mut()
        .unwrap()
        .registry_identity = "::foreign::format".into();
    assert!(native_byte_command_plan(words, &entry, &registry, context).is_err());
}

#[test]
fn dynamic_format_word_zero_two_values_keep_original_owner() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut format = binding(
        &entry.namespaces[0],
        b"format",
        NativeCompilerHookPresence::Present,
    );
    format.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "format".into(),
        ensemble: None,
    });
    entry.commands.push(format);
    for source in [
        b"format [side format]".as_slice(),
        b"format [side format] [side a] [side b]".as_slice(),
    ] {
        let mut entry = entry.clone();
        let module = source_module(source, &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("original format invocation")
        };
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Ok(NativeByteCommandPlan::Generic)
        );
        entry.commands[0].token += 1;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Ok(NativeByteCommandPlan::Generic)
        );
        entry.commands[0].has_execution_trace = true;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Ok(NativeByteCommandPlan::Generic)
        );
        entry.commands[0].has_execution_trace = false;
        entry.commands[0]
            .compiler
            .as_mut()
            .unwrap()
            .registry_identity = "::foreign::format".into();
        assert!(native_byte_command_plan(words, &entry, &registry, context).is_err());
    }
}

#[test]
fn ascii_failindex_decline_is_private_and_does_not_admit_ensemble_workers() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut worker = binding(
        &entry.namespaces[0],
        b"private",
        NativeCompilerHookPresence::Present,
    );
    worker.token = 12;
    worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "::tcl::string::is".into(),
        ensemble: None,
    });
    let mut public = binding(
        &entry.namespaces[0],
        b"ensemble",
        NativeCompilerHookPresence::Present,
    );
    public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "custom-ensemble".into(),
        ensemble: Some(
            tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                namespace_token: entry.current_namespace,
                map: vec![("member".into(), vec![Some("private".into())])],
                subcommands: None,
                prefixes: true,
                parameters: vec![],
                unknown_handler: None,
            },
        ),
    });
    entry.commands.extend([public, worker]);
    install_original_target_fixture(&mut entry);
    let direct = source_module(b"private ascii -failindex index $subject", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &direct.top_level.statements[0] else {
        panic!("original private call")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context).unwrap(),
        NativeByteCommandPlan::Generic
    );
    let public = source_module(b"ensemble member ascii -failindex index $subject", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &public.top_level.statements[0] else {
        panic!("original public call")
    };
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Err(NativeByteCommandUnavailable::Recipe)
    );
    entry.commands[0]
        .compiler
        .as_mut()
        .unwrap()
        .ensemble
        .as_mut()
        .unwrap()
        .map[0]
        .1
        .push(Some("prefix".into()));
    assert_eq!(
        native_byte_command_plan(words, &entry, &registry, context),
        Ok(NativeByteCommandPlan::Generic)
    );
}

#[test]
fn original_selected_worker_trace_veto_and_dictionary_operation_are_distinct() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut entry = entry(version);
        let mut worker = binding(
            &entry.namespaces[0],
            b"private",
            NativeCompilerHookPresence::Present,
        );
        worker.token = 12;
        worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "::tcl::dict::get".into(),
            ensemble: None,
        });
        let mut public = binding(
            &entry.namespaces[0],
            b"ensemble",
            NativeCompilerHookPresence::Present,
        );
        public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "custom-ensemble".into(),
            ensemble: Some(
                tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                    namespace_token: entry.current_namespace,
                    map: vec![("member".into(), vec![Some("private".into())])],
                    subcommands: None,
                    prefixes: true,
                    parameters: vec![],
                    unknown_handler: None,
                },
            ),
        });
        entry.commands.extend([public, worker.clone()]);
        install_original_target_fixture(&mut entry);
        let module = source_module(b"ensemble member $dictionary $key", &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("retained original source");
        };
        let NativeByteCommandPlan::Registered(selected) =
            native_byte_command_plan(words, &entry, &registry, context).unwrap()
        else {
            panic!("actual dictionary worker instruction");
        };
        assert_eq!(selected.operand_from, 2);
        assert_eq!(selected.binding, worker);
        assert_eq!(
            selected
                .prerequisite
                .as_ref()
                .unwrap()
                .selected_worker
                .as_ref(),
            Some(&worker)
        );
        let tcl_registry::native_instruction_plan::NativeInstructionPlan::DictionaryLookup(recipe) =
            selected.original_instruction(words).unwrap().clone()
        else {
            panic!("shared dictionary original geometry");
        };
        assert_eq!(
            recipe.operands,
            [
                tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(
                    2
                ),
                tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::Original(
                    3
                )
            ]
        );
        entry.commands[1].has_execution_trace = true;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context).unwrap(),
            NativeByteCommandPlan::Generic
        );
    }
}

fn configured_collision_fixture(
    version: tcl_dialect::TclVersion,
    target: &str,
) -> (
    NativeCompilationEntry,
    NativeCompilationBinding,
    NativeCompilationBinding,
) {
    let mut entry = entry(version);
    entry.namespaces.push(namespace(
        2,
        ByteNamespacePath::from_segments([b"a:".as_slice()]),
        true,
    ));
    entry.namespaces.push(namespace(
        3,
        ByteNamespacePath::from_segments([b"a".as_slice()]),
        true,
    ));
    let mut worker = binding(
        &entry.namespaces[1],
        b"w",
        NativeCompilerHookPresence::Absent,
    );
    worker.token = 12;
    let mut collision = binding(
        &entry.namespaces[2],
        b"w",
        NativeCompilerHookPresence::Absent,
    );
    collision.token = 13;
    let mut public = binding(
        &entry.namespaces[0],
        b"ensemble",
        NativeCompilerHookPresence::Present,
    );
    public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "custom-ensemble".into(),
        ensemble: Some(
            tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                namespace_token: 2,
                map: vec![("member".into(), vec![Some(target.into())])],
                subcommands: None,
                prefixes: true,
                parameters: vec![],
                unknown_handler: None,
            },
        ),
    });
    entry
        .commands
        .extend([public, worker.clone(), collision.clone()]);
    install_original_target_fixture(&mut entry);
    (entry, worker, collision)
}

fn cached_worker_target(
    entry: &NativeCompilationEntry,
    version: tcl_dialect::TclVersion,
    worker: &NativeCompilationBinding,
) -> tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary {
    use tcl_runtime_api::native_command_name::{
        NativeCommandNameCache, NativeCommandNameLookupState, NativeCommandNameReference,
        NativeCommandNameTarget,
    };
    let cache = NativeCommandNameCache {
        interpreter: entry.interpreter,
        version,
        slot: worker.slot.clone(),
        namespace_token: 2,
        token: worker.token,
        implementation_generation: worker.implementation_generation,
        command_epoch: 4,
        reference: None,
    };
    let lookup = NativeCommandNameLookupState {
        interpreter: entry.interpreter,
        reference: NativeCommandNameReference {
            namespace_token: 2,
            command_reference_epoch: 1,
        },
        target: Some(NativeCommandNameTarget {
            token: worker.token,
            implementation_generation: worker.implementation_generation,
            command_epoch: 4,
            namespace_token: 2,
            namespace_dying: false,
        }),
    };
    tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::CommandName {
        origin: version,
        cache: Some(cache),
        lookup: Some(lookup),
    }
}

#[test]
fn original_map_cache_selects_its_node_and_missing_evidence_stays_unknown() {
    use tcl_registry::native_ensemble::{NativeEnsembleWorkerSelection, select_worker_in_entry};
    use tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary;
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let (mut entry, worker, collision) = configured_collision_fixture(version, "::a:::w");
        let before = entry.clone();
        entry.ensemble_target_objects.as_mut().unwrap()[0].primary =
            cached_worker_target(&entry, version, &worker);
        assert_ne!(entry, before);
        assert!(entry.same_compilation_world(&before));
        let configuration = entry.commands[0]
            .compiler
            .as_ref()
            .unwrap()
            .ensemble
            .as_ref()
            .unwrap()
            .clone();
        let select = |entry: &NativeCompilationEntry| {
            select_worker_in_entry(
                entry,
                11,
                &configuration,
                Some(b"member"),
                Some(tcl_registry::native_compilation::NativeCompilationWordShape::Literal),
                Some(InvocationDialect::for_version(version)),
            )
        };
        assert_eq!(
            select(&entry),
            NativeEnsembleWorkerSelection::Worker {
                member: "member".into(),
                binding: Box::new(worker.clone())
            }
        );
        let mut stale = entry.clone();
        let NativeEnsembleTargetPrimary::CommandName { lookup, .. } =
            &mut stale.ensemble_target_objects.as_mut().unwrap()[0].primary
        else {
            unreachable!()
        };
        lookup
            .as_mut()
            .unwrap()
            .target
            .as_mut()
            .unwrap()
            .command_epoch += 1;
        assert_eq!(
            select(&stale),
            NativeEnsembleWorkerSelection::Worker {
                member: "member".into(),
                binding: Box::new(collision)
            }
        );
        let mut missing = entry.clone();
        missing.ensemble_target_objects = None;
        assert_eq!(select(&missing), NativeEnsembleWorkerSelection::Unknown);
        let mut unavailable = entry.clone();
        unavailable.ensemble_target_objects.as_mut().unwrap()[0].primary =
            NativeEnsembleTargetPrimary::Unavailable;
        assert_eq!(select(&unavailable), NativeEnsembleWorkerSelection::Unknown);
        let mut mutation = entry.clone();
        mutation.epoch += 1;
        assert!(!entry.same_compilation_world(&mutation));
    }
}

#[test]
fn literal_expansion_selects_original_ensemble_member_and_dictionary_operands() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let mut entry = entry(version);
        let mut worker = binding(
            &entry.namespaces[0],
            b"private",
            NativeCompilerHookPresence::Present,
        );
        worker.token = 12;
        worker.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "::tcl::dict::get".into(),
            ensemble: None,
        });
        let mut public = binding(
            &entry.namespaces[0],
            b"dict",
            NativeCompilerHookPresence::Present,
        );
        public.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: "dict".into(),
            ensemble: Some(
                tcl_runtime_api::native_compilation::NativeEnsembleCompiler {
                    namespace_token: 1,
                    map: vec![("get".into(), vec![Some("private".into())])],
                    subcommands: None,
                    prefixes: true,
                    parameters: vec![],
                    unknown_handler: None,
                },
            ),
        });
        entry.commands.extend([public, worker]);
        install_original_target_fixture(&mut entry);
        let module = source_module(b"dict {*}{get {k v} k}", &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("original expansion geometry");
        };
        let NativeByteCommandPlan::Registered(selected) =
            native_byte_command_plan(words, &entry, &registry, context).unwrap()
        else {
            panic!("actual expanded member compiler");
        };
        let tcl_registry::native_instruction_plan::NativeInstructionPlan::DictionaryLookup(recipe) =
            selected.original_instruction(words).unwrap().clone()
        else {
            panic!("dictionary compiler recipe");
        };
        assert_eq!(recipe.key_count, 1);
        assert_eq!(recipe.operands.len(), 2);
        for operand in recipe.operands {
            assert!(matches!(operand, tcl_registry::native_compiler_word_projection::NativeCompilerWordOperand::LiteralExpansion { original_word: 1, .. }));
        }
    }
}

#[test]
fn configured_byte_worker_priming_keeps_original_token_despite_reported_colons() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for version in [
        tcl_dialect::TclVersion::V8_5,
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        let (entry, worker, collision) = configured_collision_fixture(version, "w");
        let module = source_module(b"ensemble mem VALUE", &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("original native command");
        };
        let selected = native_byte_command_plan(words, &entry, &registry, context).unwrap();
        if version == tcl_dialect::TclVersion::V8_5 {
            assert_eq!(selected, NativeByteCommandPlan::Generic);
            continue;
        }
        let NativeByteCommandPlan::Named(selected) = selected else {
            panic!("actual no-hook worker selection");
        };
        assert_eq!(
            selected.prerequisite.selected_worker.as_ref(),
            Some(&worker)
        );
        assert_eq!(selected.recipe.name, b"::a:::w");
        assert_eq!(
            entry
                .lookup_command_bytes(entry.current_namespace, &selected.recipe.name)
                .unwrap(),
            Some(&collision)
        );
        let literal = tcl_registry::native_command_literal::native_compiled_selected_command_name_literal_from_lookup(
                entry.execution_point.unwrap(), entry.name_protocol.unwrap(),
                tcl_runtime_api::native_command_name::NativeLiteralContext { interpreter: entry.interpreter, namespace_token: entry.current_namespace, entry_epoch: entry.epoch, namespace_path: ByteNamespacePath::root() },
                &selected.recipe.name, selected.prerequisite.clone(),
            ).unwrap();
        let priming = literal.priming.unwrap();
        assert_eq!(priming.binding, worker);
        assert!(priming.matches_entry(&entry));
        let mut changed = entry.clone();
        changed.commands[0]
            .compiler
            .as_mut()
            .unwrap()
            .ensemble
            .as_mut()
            .unwrap()
            .prefixes = false;
        assert!(!priming.matches_entry(&changed));
        let mut changed = entry.clone();
        changed.commands[1].implementation_generation += 1;
        assert!(!priming.matches_entry(&changed));
        changed = entry.clone();
        changed.closed = false;
        assert!(!priming.matches_entry(&changed));
    }
}

#[test]
fn byte_compiler_plan_retains_original_registration_for_opaque_ordinary_operands() {
    let registry = CommandRegistry::build_default();
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
        ..Default::default()
    };
    for (identity, source) in [
        ("set", b"renamed\xff x\xff\0tail VALUE".as_slice()),
        ("incr", b"renamed\xff x\xff\0tail 1".as_slice()),
        ("list", b"renamed\xff A\xff\0tail B".as_slice()),
        ("append", b"renamed\xff x\xff\0tail $value".as_slice()),
        ("lappend", b"renamed\xff x\xff\0tail $value".as_slice()),
        ("return", b"renamed\xff $value".as_slice()),
    ] {
        let mut entry = entry(tcl_dialect::TclVersion::V9_0);
        let mut registration = binding(
            &entry.namespaces[0],
            b"renamed\xff",
            NativeCompilerHookPresence::Present,
        );
        registration.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
            registry_identity: identity.into(),
            ensemble: None,
        });
        entry.commands.push(registration.clone());
        let module = source_module(source, &entry);
        let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
            panic!("retained original word vector");
        };
        let plan = native_byte_command_plan(words, &entry, &registry, context).unwrap();
        let NativeByteCommandPlan::Registered(plan) = plan else {
            panic!("authenticated original compiler");
        };
        assert_eq!(plan.binding, registration);
        assert_eq!(plan.operand_from, 1);
        assert_eq!(
            plan.dependencies,
            [] as [tcl_runtime_api::native_compilation::NativeCompilationBinding; 0]
        );
        assert!(matches!(
            plan.selection,
            NativeCompilationSelection::Inline { .. }
        ));
        assert!(!closed_generic_byte_compilation(&module, &entry));
        assert!(closed_original_byte_compilation(
            &module, &entry, &registry, context
        ));
        entry.commands[0].compiler = None;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Err(NativeByteCommandUnavailable::Registration)
        );
        assert!(!closed_original_byte_compilation(
            &module, &entry, &registry, context
        ));
        entry.commands[0].compiler_hook = NativeCompilerHookPresence::Unknown;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Err(NativeByteCommandUnavailable::Registration)
        );
        entry.source_string_protocol = None;
        assert_eq!(
            native_byte_command_plan(words, &entry, &registry, context),
            Err(NativeByteCommandUnavailable::Entry)
        );
    }
}

#[test]
fn byte_compiler_plan_refuses_unsupported_registered_recipes_and_nested_hooks() {
    let registry = CommandRegistry::build_default();
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut registration = binding(
        &entry.namespaces[0],
        b"worker",
        NativeCompilerHookPresence::Present,
    );
    registration.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "expr".into(),
        ensemble: None,
    });
    entry.commands.push(registration);
    let module = source_module(b"worker $x", &entry);
    let crate::ir::Statement::NativeCall { words, .. } = &module.top_level.statements[0] else {
        panic!("original words");
    };
    assert_eq!(
        native_byte_command_plan(
            words,
            &entry,
            &registry,
            NativeCompilationContext {
                mode: NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                ..Default::default()
            }
        ),
        Err(NativeByteCommandUnavailable::Recipe)
    );
    assert!(!closed_generic_byte_compilation(
        &source_module(b"opaque\xff $a([worker])", &entry),
        &entry
    ));
    entry.source_string_protocol = None;
    assert!(!closed_generic_byte_compilation(
        &source_module(b"opaque\xff", &entry),
        &entry
    ));
}

#[test]
fn registered_certificate_keeps_nested_original_hook_obligations() {
    let registry = CommandRegistry::build_default();
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    let mut registration = binding(
        &entry.namespaces[0],
        b"set",
        NativeCompilerHookPresence::Present,
    );
    registration.compiler = Some(tcl_runtime_api::native_compilation::NativeCommandCompiler {
        registry_identity: "set".into(),
        ensemble: None,
    });
    entry.commands.push(registration);
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"child",
        NativeCompilerHookPresence::Present,
    ));
    let context = NativeCompilationContext {
        mode: NativeCompilationMode::BytecodeObject,
        frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
        ..Default::default()
    };
    let module = source_module(b"set a\xff([child]) VALUE", &entry);
    assert!(!closed_original_byte_compilation(
        &module, &entry, &registry, context
    ));
    entry.commands[1].compiler_hook = NativeCompilerHookPresence::Absent;
    assert!(closed_original_byte_compilation(
        &module, &entry, &registry, context
    ));
    let malformed = source_module(b"set a\xff([child {a}b]) VALUE", &entry);
    assert!(!closed_original_byte_compilation(
        &malformed, &entry, &registry, context
    ));
    entry.source_string_protocol = None;
    assert!(!closed_original_byte_compilation(
        &module, &entry, &registry, context
    ));
}

#[test]
fn byte_generic_certificate_uses_search_path_and_retained_namespace_incarnations() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    entry.namespaces.push(namespace(
        2,
        ByteNamespacePath::from_segments([b"N".as_slice()]),
        false,
    ));
    entry.namespaces.push(namespace(
        3,
        ByteNamespacePath::from_segments([b"P".as_slice()]),
        true,
    ));
    entry.namespaces[1].command_path = vec![3];
    entry.current_namespace = 2;
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"worker\xff",
        NativeCompilerHookPresence::Absent,
    ));
    entry.commands.push(binding(
        &entry.namespaces[2],
        b"worker\xff",
        NativeCompilerHookPresence::Present,
    ));
    let module = source_module(b"worker\xff", &entry);
    assert!(!closed_generic_byte_compilation(&module, &entry));
    entry.commands[1].compiler_hook = NativeCompilerHookPresence::Absent;
    assert!(closed_generic_byte_compilation(&module, &entry));
    let descendant = source_module(b"child::worker\xff", &entry);
    assert!(!closed_generic_byte_compilation(&descendant, &entry));
    entry
        .namespaces
        .push(namespace(4, entry.namespaces[1].path.clone(), true));
    assert!(!closed_generic_byte_compilation(&descendant, &entry));
    entry.namespaces[1].command_path = vec![99];
    assert!(!closed_generic_byte_compilation(&module, &entry));
}

#[test]
fn byte_generic_certificate_checks_every_original_bracket_child() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"compileMe",
        NativeCompilerHookPresence::Present,
    ));
    for source in [
        b"opaque\xff [compileMe]".as_slice(),
        b"$head [absent [compileMe]]",
        b"[compileMe] ARG",
        b"opaque\xff $array([compileMe])",
    ] {
        assert!(
            !closed_generic_byte_compilation(&source_module(source, &entry), &entry),
            "{source:?}"
        );
    }
    entry.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
    assert!(closed_generic_byte_compilation(
        &source_module(b"opaque\xff [absent [compileMe]]", &entry),
        &entry
    ));
    // An unresolved expanded head uses native invokeExpanded; it does not
    // choose a literal head's compiler. Its bracket children still require
    // the independently closed registrations checked above. The retained
    // native_expanded_head capture records the actual dynamic_bytecode.
    assert!(closed_generic_byte_compilation(
        &source_module(b"{*}$head ARG", &entry),
        &entry
    ));
}

#[test]
fn byte_generic_certificate_checks_bracket_hooks_beyond_advisory_index_depth() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    entry.commands.push(binding(
        &entry.namespaces[0],
        b"compileMe",
        NativeCompilerHookPresence::Present,
    ));
    let mut source = b"opaque\xff ".to_vec();
    for _ in 0..2000 {
        source.extend_from_slice(b"$a(");
    }
    source.extend_from_slice(b"[compileMe]");
    source.extend(std::iter::repeat_n(b')', 2000));
    let module = source_module(&source, &entry);
    assert!(!closed_generic_byte_compilation(&module, &entry));
    entry.commands[0].compiler_hook = NativeCompilerHookPresence::Absent;
    assert!(closed_generic_byte_compilation(&module, &entry));
}

#[test]
fn byte_generic_certificate_never_looks_up_a_dynamic_compiler_head() {
    let mut entry = entry(tcl_dialect::TclVersion::V9_0);
    entry.closed = false;
    for source in [
        b"$head ARG".as_slice(),
        b"prefix${head} ARG",
        b"$array($key) ARG",
    ] {
        assert!(
            closed_generic_byte_compilation(&source_module(source, &entry), &entry),
            "{source:?}"
        );
    }
    assert!(!closed_generic_byte_compilation(
        &source_module(b"$head [literalChild]", &entry),
        &entry
    ));
    entry.closed = true;
    assert!(closed_generic_byte_compilation(
        &source_module(b"[absent] ARG", &entry),
        &entry
    ));
    entry.name_protocol = None;
    assert!(!closed_generic_byte_compilation(
        &source_module(b"$head ARG", &entry),
        &entry
    ));
}

#[test]
fn byte_generic_certificate_keeps_native_constant_head_rules_by_release() {
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = version.dialect_profile_name();
        let mut entry = entry(version);
        entry.commands.push(binding(
            &entry.namespaces[0],
            b"worker",
            NativeCompilerHookPresence::Present,
        ));
        for source in [b"{worker}".as_slice(), b"\"worker\""] {
            assert!(
                !closed_generic_byte_compilation(&source_module(source, &entry), &entry),
                "{dialect} {source:?}"
            );
        }
        let escaped = source_module(b"wor\\ker", &entry);
        assert_eq!(
            closed_generic_byte_compilation(&escaped, &entry),
            dialect == "tcl8.4" || dialect == "tcl8.5",
            "{dialect}"
        );
    }
}
