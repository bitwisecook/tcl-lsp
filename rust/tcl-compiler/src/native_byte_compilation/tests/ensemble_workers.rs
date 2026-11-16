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

//! Plans for a selected ensemble worker: its installed named compiler and the named fallbacks that keep the original operands.

use super::*;

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
