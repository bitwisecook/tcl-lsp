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

//! Which ensemble worker a call selects: trace vetoes, the map cache, literal expansion and priming.

use super::*;

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
    use tcl_native_model::native_ensemble::NativeEnsembleWorkerSelection;
    use tcl_registry::native_ensemble::select_worker_in_entry;
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
