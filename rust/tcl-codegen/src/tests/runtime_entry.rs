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

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tcl_compiler::command_binding::{
    BindingKind, CommandIdentity, MayBinding, ModuleCommandBindings, ResolvedCommandTarget,
    RuntimeCommandTokenIdentity, SourceCommandKey, SourceNamespaceKey,
};
use tcl_runtime_api::native_compilation::{NativeCommandImplementation, NativeCompilationBinding};

use tcl_compiler::command_binding::runtime_entry::*;
use tcl_compiler::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
};
use tcl_runtime_api::native_compilation::{
    NativeCompilationEntry, NativeCompilationNamespace, NativeInterpreterIdentity,
};

fn select_fixture_native_point(
    snapshot: &mut NativeCompilationEntry,
    version: tcl_dialect::TclVersion,
) {
    let point = tcl_registry::InvocationDialect::for_version(version)
        .execution_point()
        .expect("measured C fixture point");
    snapshot.execution_point = Some(point);
    snapshot.name_protocol = tcl_syntax::naming::NamePolicyProtocol::for_native_point(point);
    snapshot.source_string_protocol =
        Some(tcl_syntax::native_string::NativeStringProtocol::C(version));
    snapshot.lexer_grammar = Some(
        tcl_dialect::DialectProfile::find(version.dialect_profile_name())
            .unwrap()
            .grammar,
    );
    let mut identity = 1;
    snapshot.ensemble_target_objects = Some(snapshot.commands.iter().flat_map(|command| {
            command.compiler.as_ref().and_then(|compiler| compiler.ensemble.as_ref()).into_iter().flat_map(move |configuration| {
                configuration.map.iter().flat_map(move |(member, prefix)| prefix.iter().enumerate().map(move |(position, name)| (command.token, member.clone(), position, name.clone())))
            })
        }).map(|(token, member, position, name)| {
            let row = tcl_runtime_api::native_compilation::NativeEnsembleTargetObservation {
                ensemble_token: token, member, prefix_index: position, object_identity: identity,
                resident_name: name.clone(), primary: if name.is_some() { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::StockString } else { tcl_runtime_api::native_compilation::NativeEnsembleTargetPrimary::Unavailable },
            };
            identity += 1; row
        }).collect());
}

fn mapped_ensemble_entry(worker_bytes: Option<&str>) -> NativeCompilationEntry {
    use tcl_runtime_api::native_compilation::{
        NativeCommandCompiler, NativeCompilerHookPresence, NativeEnsembleCompiler,
    };
    let mut snapshot = entry(NativeCommandImplementation::Opaque);
    let public = &mut snapshot.commands[0];
    public.slot = tcl_core_types::NativeByteCommandSlot::new(
        tcl_core_types::ByteNamespacePath::root(),
        "info".into(),
    );
    public.compiler_hook = NativeCompilerHookPresence::Present;
    public.compiler = Some(NativeCommandCompiler {
        registry_identity: "info".to_owned(),
        ensemble: Some(NativeEnsembleCompiler {
            namespace_token: 0,
            map: vec![("exists".into(), vec![worker_bytes.map(Into::into)])],
            subcommands: None,
            prefixes: true,
            parameters: Vec::new(),
            unknown_handler: None,
        }),
    });
    let mut worker = public.clone();
    worker.slot = tcl_core_types::NativeByteCommandSlot::new(
        tcl_core_types::ByteNamespacePath::root(),
        "helper".into(),
    );
    worker.token = 5;
    worker.compiler_hook = NativeCompilerHookPresence::Absent;
    worker.compiler = None;
    snapshot.commands.push(worker);
    snapshot
}

fn entry(implementation: NativeCommandImplementation) -> NativeCompilationEntry {
    NativeCompilationEntry {
        interpreter: NativeInterpreterIdentity {
            owner: 1,
            interpreter: 2,
        },
        epoch: 3,
        profile: tcl_dialect::DialectProfile::plain_tcl().cache_key(),
        invocation_policy: Some(tcl_dialect::DialectProfile::plain_tcl().cache_key()),
        expression_policy: None,
        execution_point: tcl_registry::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V9_0,
        )
        .execution_point(),
        execution_name_policy: None,
        name_protocol: Some(
            tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::TCL_9_0),
            )
            .unwrap(),
        ),
        compiled_variable_protocol: None,
        compiled_local_layout: None,
        oo_classes: None,
        ensemble_target_objects: None,
        source_string_protocol: None,
        lexer_grammar: None,
        inline_compilation_disabled: false,
        authored_tmm_static: None,
        namespace_variable_tables: None,
        empty_literal_world: None,
        compiler_pass_environment: None,
        command_resolvers: None,
        variable_observers:
            tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Unknown,
        math_functions: None,
        closed: true,
        current_namespace: 0,
        frame: tcl_runtime_api::native_compilation::NativeCompilationFrame::Global,
        commands: vec![NativeCompilationBinding {
            slot: tcl_core_types::NativeByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::root(),
                "set".into(),
            ),
            namespace_token: 0,
            token: 4,
            implementation_generation: u64::MAX,
            compiler_hook: match &implementation {
                NativeCommandImplementation::Registry {
                    compiler_hook: true,
                    ..
                } => tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Present,
                NativeCommandImplementation::Registry {
                    compiler_hook: false,
                    ..
                } => tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
                _ => tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Unknown,
            },
            implementation,
            compiler: None,
            procedure_header: None,
            has_execution_trace: false,
        }],
        namespaces: vec![NativeCompilationNamespace {
            path: tcl_core_types::ByteNamespacePath::root(),
            jim_namespace_object: None,
            token: 0,
            visible: true,
            exports: Vec::new(),
            command_path: Vec::new(),
            unknown_handler: None,
        }],
    }
}

#[test]
fn compile_service_emits_original_mutable_named_invocation_without_static_tag() {
    use tcl_runtime_api::CompileService;
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(name).analyser_profile();
        let version = tcl_registry::InvocationDialect::of_profile(profile)
            .tcl_version
            .unwrap();
        let mut snapshot = mapped_ensemble_entry(Some("::helper"));
        select_fixture_native_point(&mut snapshot, version);
        snapshot.profile = profile.cache_key();
        snapshot.invocation_policy = Some(profile.cache_key());
        let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let compiled = service
            .compile_script_with_entry(
                tcl_runtime_api::ScriptCompileTarget {
                    source: "info exists missing",
                    namespace: "",
                    evaluation: tcl_runtime_api::CommandEvaluation::Compiled,
                },
                profile,
                &snapshot,
            )
            .expect("authentic captured Named compiler admission");
        assert_eq!(
            compiled.top_level.native_compilation_preflight,
            tcl_runtime_api::NativeCompilationPreflight::NotRequired
        );
        assert_ne!(
            compiled.top_level.native_compiler_prerequisites,
            [] as [tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite; 0]
        );
        let selections: Vec<_> = compiled
            .top_level
            .instructions
            .iter()
            .filter_map(|instruction| instruction.native_compiler_selection.as_ref())
            .collect();
        assert_ne!(
            selections,
            [] as [&tcl_bytecode::NativeCompilerSelectionSite; 0]
        );
        assert!(
            selections
                .iter()
                .all(|selection| selection.prerequisite.guard()
                    == tcl_runtime_api::CommandBindingGuard::BeforeArguments)
        );
        assert!(
                compiled
                    .top_level
                    .native_compiler_prerequisites
                    .iter()
                    .all(|required| required.guard()
                        == tcl_runtime_api::CommandBindingGuard::ChunkEntry
                        && selections.iter().any(|selection| {
                            use tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite;
                            let (NativeCompilerSelectionPrerequisite::Ensemble(original)
                                | NativeCompilerSelectionPrerequisite::Command(original)) = &selection.prerequisite
                            else {
                                return false;
                            };
                            let mut entry_required = original.as_ref().clone();
                            entry_required.guard = tcl_runtime_api::CommandBindingGuard::ChunkEntry;
                            NativeCompilerSelectionPrerequisite::from_command_registration(
                                std::sync::Arc::new(entry_required),
                            ) == *required
                        }))
            );
    }
}
