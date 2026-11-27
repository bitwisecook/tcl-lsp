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

//! Selection for one compiler target: ensemble, inline, named and no-op operations.

use super::super::{
    Arc, BindingKind, CommandAllocationSite, ModuleCommandBindings, SourceCommandTarget,
};
use super::registered_compiler::registered_compiler_selection;
use super::{
    CompiledExecutionCertainty, CompiledInvocationSelection, CompilerInvocation,
    SourceCompiledInvocationProof, SourceNativeCompilerAdmission,
    SourceNativeStructuredPreparation,
};
use tcl_registry::native_compilation::{NativeCompilationGuard, NativeCompilationSelection};

pub(super) fn select_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    if let Some(prerequisite) = state.runtime_noop_header(
        state
            .compiler_identity_at_lookup(selected.head, &context.namespace_identity())
            .or(target.identity.as_ref()),
        target.implementation_generation,
        target.implementation_allocation.as_ref(),
    ) {
        select_noop_target(target, selected, prerequisite, state, context, result);
        return;
    }
    let registered = registered_compiler_selection(target, selected, state, context);
    if let Some(snapshot) = context.compilation_snapshot
        && let Some(plan) = snapshot.table.state.actual_ensemble_plan(
            target,
            selected.head,
            &context.namespace_identity(),
            state.current_source_origin.as_ref().map(|origin| (
                crate::registry_invocation::OriginalNativeCompilerInvocation {
                    image: origin.source_image(), words: selected.words, offset: selected.offset,
                    config: context.config,
                    source_protocol: super::super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(state),
                    compiler_dialect: state.baseline.compilation_dialect(), context: context.compilation(), operand_from: 2,
                }, context.registry,
            )),
        )
    {
        // A stock public ensemble can delegate to a replaced opaque worker.
        // Its actual configuration then selects a named invocation independently
        // of the registry implementation prerequisites for the original opcode.
        if !target.registry_backed
            || !matches!(
                plan,
                super::super::ensemble_compilation::ActualEnsemblePlan::Unknown
            )
        {
            result.compiler_name_preservation = super::super::SourceCompilerNameClosure::Unknown;
            select_actual_ensemble_target(target, selected, plan, state, context, result);
            return;
        }
    }
    let Some(registered) = registered else {
        // `TclCompileScript` invokes a command without a compile procedure;
        // the runtime certifies that this registration has none.
        if closed_surface_catalogue_dispatch(target, state)
            || state.runtime_compiler_hook(target.identity.as_ref()) == Some(false)
            || basic_member_invoked_by_its_private_name(target, state, context)
        {
            result.live = true;
            result.admit(NativeCompilationSelection::Generic);
            return;
        }
        result.compiler_name_preservation = super::super::SourceCompilerNameClosure::Unknown;
        result.unknown = true;
        result.admit(NativeCompilationSelection::Unknown);
        return;
    };
    result
        .compiler_name_preservation
        .join(registered.compiler_preserves_names.into());
    result.operand_layout = registered.operand_layout;
    result.namespace_bindings = registered.namespace_bindings;
    result.switch = registered.switch;
    result.structured = registered.structured;
    result.admit(registered.admission);
    let spec = registered.spec;
    let admission = registered.admission;
    match admission {
        NativeCompilationSelection::NamedInvocation {
            lookup,
            arguments_from,
            protocol,
        } => {
            select_named_target(
                target,
                selected,
                RegisteredNamedInvocation {
                    preparations: result.structured.as_ref().and_then(|prepared|match prepared.recipe() {tcl_registry::native_instruction_plan::NativeInstructionPlan::NamedInvocation(recipe)=>Some(recipe.preparations.clone()),_=>None}).unwrap_or_default(),
                    lookup,
                    arguments_from,
                    protocol,
                    dependencies: registered.dependencies,
                },
                state,
                context,
                result,
            );
        }
        NativeCompilationSelection::Generic => result.live = true,
        NativeCompilationSelection::Unknown => result.unknown = true,
        NativeCompilationSelection::CompileError => result.compile_error = true,
        NativeCompilationSelection::Inline { operation, guard } => {
            select_inline_target(
                target,
                selected,
                (spec, operation, guard),
                state,
                context,
                result,
            );
        }
    }
}

/// A closed load-time surface ([`Family::closed_load_time_resolution`])
/// resolves a literal catalogue head when the rule loads. Its vendor compiler
/// owns no tracked command or namespace name and dispatches to the catalogued
/// handler, as the surface's direct evaluation does, so an undescribed hook
/// selects that handler generically after argv.
///
/// [`Family::closed_load_time_resolution`]: tcl_dialect::model::Family::closed_load_time_resolution
/// A registered member whose compiler is `TclCompileBasicNArgCmd`, reached by
/// its own private name where the registry resolves no invocation facts (a
/// nested worker such as `::tcl::binary::encode::hex`). The basic compiler and
/// its fallback both push the written words and emit `invokeStk`; tclsh
/// 8.6.18 and 9.0.4 disassemble `::tcl::binary::encode::hex X` to
/// `push ::tcl::binary::encode::hex; push X; invokeStk1 2`.
fn basic_member_invoked_by_its_private_name(
    target: &SourceCommandTarget,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
) -> bool {
    if !target.registry_backed || !target.prepended.is_empty() {
        return false;
    }
    let Some(dialect) = state.baseline.compilation_dialect() else {
        return false;
    };
    context
        .registry
        .native_compilation_for_registration(&target.command, dialect)
        .and_then(tcl_registry::native_compilation::NativeCompilationSpec::named_member_slot)
        .is_some_and(|slot| {
            tcl_syntax::naming::unroot_rooted_key(slot).unwrap_or(slot)
                == tcl_syntax::naming::unroot_rooted_key(&target.command).unwrap_or(&target.command)
        })
}

fn closed_surface_catalogue_dispatch(
    target: &SourceCommandTarget,
    state: &ModuleCommandBindings,
) -> bool {
    target.registry_backed
        && target.kind == BindingKind::Builtin
        && target.prepended.is_empty()
        && state
            .baseline
            .dialect
            .and_then(tcl_native_model::InvocationDialect::family)
            .is_some_and(tcl_dialect::model::Family::closed_load_time_resolution)
}

fn select_actual_ensemble_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    plan: super::super::ensemble_compilation::ActualEnsemblePlan,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    use super::super::ensemble_compilation::ActualEnsemblePlan;
    match plan {
        ActualEnsemblePlan::Operation {
            selection,
            recipe,
            prerequisite,
        } => {
            result.admit(selection);
            let Some((site, possible)) =
                original_ensemble_selection_site(target, selected, state, result)
            else {
                return;
            };
            let prerequisite = Arc::from(prerequisite);
            result.structured = Some(Arc::new(SourceNativeStructuredPreparation {
                compilation_site: site.clone(),
                dependency: super::super::SourceNativeCompilationDependency {
                    compiler_prerequisite: Some(Arc::clone(&prerequisite)),
                    target: target.clone(),
                    namespace: context.namespace.to_owned(),
                    namespace_key: context.namespace_identity(),
                    head: selected.head.to_owned(),
                    guard: NativeCompilationGuard::BeforeArguments,
                },
                recipe: *recipe,
            }));
            if let NativeCompilationSelection::Inline { operation, guard } = selection {
                let proof = SourceCompiledInvocationProof {
                    target: target.clone(),
                    operation,
                    guard,
                    compilation_site: site,
                    certainty: CompiledExecutionCertainty::May,
                    lookup_dependencies: Vec::new(),
                    compiler_prerequisite: Some(prerequisite),
                    procedure_header_prerequisite: None,
                };
                result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Inline(Box::new(
                    proof.clone(),
                ))));
                if possible {
                    result.proofs.push(proof);
                }
            }
        }
        ActualEnsemblePlan::Generic => {
            result.admit(NativeCompilationSelection::Generic);
            result.live = true;
        }
        ActualEnsemblePlan::Unknown => {
            result.admit(NativeCompilationSelection::Unknown);
            result.unknown = true;
        }
        ActualEnsemblePlan::Named {
            preparations,
            name,
            replacement_words,
            prerequisite,
            protocol,
            arguments_from,
        } => {
            let Some((site, possible)) =
                original_ensemble_selection_site(target, selected, state, result)
            else {
                return;
            };
            let proof = super::super::SourceNamedInvocationProof {
                preparations,
                lookup: None,
                captured_name: name,
                compiler_prerequisite: Some(Arc::from(prerequisite)),
                arguments_from,
                protocol,
                replacement_words,
                compilation_site: site,
                certainty: CompiledExecutionCertainty::May,
                dependencies: Vec::new(),
                targets: Vec::new(),
                unknown: false,
                may_be_absent: false,
            };
            result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Named(
                proof.clone(),
            )));
            if possible {
                result.named.push(proof);
            }
        }
    }
}

fn original_ensemble_selection_site(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    result: &mut CompiledInvocationSelection,
) -> Option<(CommandAllocationSite, bool)> {
    let proved = selected.before.proved_target() == Some(target)
        || (state
            .compiler_identity_at_lookup(selected.head, &selected.before.lookup_namespace_key)
            == target.identity.as_ref()
            && !state.source_execution_observed(target.identity.as_ref()));
    result.live |= !proved;
    let possible = proved || selected.before.unknown || selected.before.targets.contains(target);
    let Some(source) = &state.current_source_origin else {
        result.unknown = true;
        return None;
    };
    Some((
        CommandAllocationSite {
            source: Arc::clone(source),
            offset: selected.offset,
        },
        possible,
    ))
}

fn select_inline_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    selection: (
        tcl_registry::native_compilation::NativeCompilationSpec,
        tcl_registry::SemanticOperationId,
        NativeCompilationGuard,
    ),
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    let (spec, operation, guard) = selection;
    let CompilerInvocation { before, offset, .. } = selected;
    let mut lookup_dependencies = Vec::new();
    if let Some(lookup) = state
        .baseline
        .compilation_dialect()
        .and_then(|dialect| spec.implementation_lookup(dialect))
    {
        let Some(snapshot) = context.compilation_snapshot else {
            result.unknown = true;
            return;
        };
        let Some(dependency) = implementation_dependency(&lookup, &snapshot.table.state, guard)
        else {
            result.live = true;
            return;
        };
        lookup_dependencies.push(dependency);
    }
    let compiler_prerequisite = context
        .compilation_snapshot
        .and_then(|snapshot| {
            snapshot.table.state.runtime_command_compiler_prerequisite(
                selected.head,
                &context.namespace_identity(),
                crate::registry_invocation::native_command_binding_guard(guard),
            )
        })
        .map(Arc::new);
    let raw_guard = compiler_prerequisite.is_some()
        && state.compiler_identity_at_lookup(selected.head, &context.namespace_identity())
            == target.identity.as_ref()
        && !state.source_execution_observed(target.identity.as_ref());
    let proved_guard = guard == NativeCompilationGuard::ChunkEntry
        || raw_guard
        || before.proved_target() == Some(target);
    let possible_guard = proved_guard || before.unknown || before.targets.contains(target);
    result.live |= !proved_guard;
    let Some(source) = &state.current_source_origin else {
        result.unknown = true;
        return;
    };
    let proof = SourceCompiledInvocationProof {
        target: target.clone(),
        operation,
        guard,
        compilation_site: CommandAllocationSite {
            source: Arc::clone(source),
            offset,
        },
        certainty: CompiledExecutionCertainty::May,
        lookup_dependencies,
        compiler_prerequisite,
        procedure_header_prerequisite: None,
    };
    result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Inline(Box::new(
        proof.clone(),
    ))));
    let private_guard = proof.lookup_dependencies.iter().all(|dependency| {
        guard == NativeCompilationGuard::ChunkEntry
            || super::super::source_binding(state, &dependency.head, &dependency.namespace_key)
                .proved_target()
                == Some(&dependency.target)
    });
    if possible_guard && private_guard {
        result.proofs.push(proof);
    } else {
        result.live = true;
    }
}

struct RegisteredNamedInvocation {
    preparations: Vec<tcl_registry::native_control_compilation::NativeControlPreparationStep>,
    pub(super) lookup:
        &'static tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    pub(super) arguments_from: usize,
    pub(super) protocol: tcl_registry::native_compilation::NativeNamedInvocationProtocol,
    pub(super) dependencies: Vec<super::super::SourceNativeCompilationDependency>,
}

fn select_named_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    plan: RegisteredNamedInvocation,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    let compiler_prerequisite = context.compilation_snapshot.and_then(|snapshot| {
        snapshot.table.state.original_named_worker_prerequisite(
            target,
            selected.arguments,
            selected.shapes,
            selected.head,
            context.namespace,
        )
    });
    let CompilerInvocation {
        before,
        head,
        offset,
        ..
    } = selected;
    let RegisteredNamedInvocation {
        preparations,
        lookup,
        arguments_from,
        protocol,
        mut dependencies,
    } = plan;
    let proved = before.proved_target() == Some(target);
    result.live |= !proved;
    let possible = proved || before.unknown || before.targets.contains(target);
    let Some(source) = &state.current_source_origin else {
        result.unknown = true;
        return;
    };
    let captured_name = compiler_prerequisite
        .as_ref()
        .and_then(|prerequisite| prerequisite.selected_worker.as_ref())
        .and_then(|worker| {
            String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(
                &worker.slot,
            ))
            .ok()
        })
        .unwrap_or_else(|| lookup.slot.to_owned());
    let proof = super::super::SourceNamedInvocationProof {
        preparations,
        lookup: Some(lookup),
        captured_name,
        compiler_prerequisite: compiler_prerequisite.map(Arc::new),
        arguments_from,
        protocol,
        replacement_words: lookup
            .prepended
            .iter()
            .map(|word| (*word).to_owned())
            .collect(),
        compilation_site: CommandAllocationSite {
            source: Arc::clone(source),
            offset,
        },
        certainty: CompiledExecutionCertainty::May,
        dependencies: {
            dependencies.insert(
                0,
                super::super::SourceNativeCompilationDependency {
                    compiler_prerequisite: None,
                    target: target.clone(),
                    head: head.to_owned(),
                    namespace: context.namespace.to_owned(),
                    namespace_key: context.namespace_identity(),
                    guard: NativeCompilationGuard::BeforeArguments,
                },
            );
            dependencies
        },
        targets: Vec::new(),
        unknown: false,
        may_be_absent: false,
    };
    result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Named(
        proof.clone(),
    )));
    if possible {
        result.named.push(proof);
    }
}

fn select_noop_target(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    mut prerequisite: tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    result: &mut CompiledInvocationSelection,
) {
    let Some(dialect) = state.baseline.compilation_dialect() else {
        result.unknown = true;
        return;
    };
    let admission = tcl_registry::native_procedure::select_procedure_header(
        prerequisite.header,
        dialect,
        selected.shapes,
        context.compilation(),
    );
    result.admit(admission);
    match admission {
        NativeCompilationSelection::Inline { operation, guard } => {
            let proved = guard == NativeCompilationGuard::ChunkEntry
                || (state
                    .compiler_identity_at_lookup(selected.head, &context.namespace_identity())
                    == target.identity.as_ref()
                    && !state.source_execution_observed(target.identity.as_ref()))
                || selected.before.proved_target() == Some(target);
            result.live |= !proved;
            let possible =
                proved || selected.before.unknown || selected.before.targets.contains(target);
            let Some(source) = &state.current_source_origin else {
                result.unknown = true;
                return;
            };
            let Some(entry) = &state.baseline.native_entry else {
                result.unknown = true;
                return;
            };
            let namespace_key = if tcl_syntax::naming::is_absolute(selected.head) {
                state.native_root_namespace_key()
            } else {
                Some(context.namespace_identity())
            };
            let Some(namespace_context) = namespace_key
                .as_ref()
                .and_then(super::super::SourceNamespaceKey::native_context)
                .filter(|namespace| namespace.interpreter == entry.interpreter)
            else {
                result.unknown = true;
                return;
            };
            let Ok(namespace) = entry.namespace_context(namespace_context.token) else {
                result.unknown = true;
                return;
            };
            if namespace.path != namespace_context.path {
                result.unknown = true;
                return;
            }
            prerequisite.lookup_namespace_token = namespace.token;
            prerequisite.invocation_word = selected.head.into();
            prerequisite.guard = crate::registry_invocation::native_command_binding_guard(guard);
            let proof = SourceCompiledInvocationProof {
                target: target.clone(),
                operation,
                guard,
                compilation_site: CommandAllocationSite {
                    source: Arc::clone(source),
                    offset: selected.offset,
                },
                lookup_dependencies: Vec::new(),
                compiler_prerequisite: None,
                procedure_header_prerequisite: Some(Arc::new(prerequisite)),
                certainty: CompiledExecutionCertainty::May,
            };
            result.admitted = Some(Arc::new(SourceNativeCompilerAdmission::Inline(Box::new(
                proof.clone(),
            ))));
            if possible {
                result.proofs.push(proof);
            }
        }
        NativeCompilationSelection::Generic => result.live = true,
        _ => result.unknown = true,
    }
}

pub(super) fn implementation_dependency(
    lookup: &tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    state: &ModuleCommandBindings,
    guard: NativeCompilationGuard,
) -> Option<super::super::SourceNativeCompilationDependency> {
    let namespace_key = state
        .native_root_namespace_key()
        .unwrap_or_else(|| super::super::SourceNamespaceKey::authored("::"));
    let binding = super::super::source_binding(state, lookup.slot, &namespace_key);
    let target = binding.proved_target()?;
    if !target.registry_backed
        || target.implementation_generation != 0
        || !target.prepended.is_empty()
        || !tcl_registry::command_descriptor_labels_match(&target.command, lookup.slot)
    {
        return None;
    }
    Some(super::super::SourceNativeCompilationDependency {
        compiler_prerequisite: None,
        target: target.clone(),
        namespace: "::".to_owned(),
        namespace_key,
        head: lookup.slot.to_owned(),
        guard,
    })
}
