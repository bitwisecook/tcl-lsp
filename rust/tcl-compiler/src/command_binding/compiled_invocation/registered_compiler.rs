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

//! A registered compiler's selection: its compilation point, preparations and dependencies.

use super::super::{
    Arc, BindingKind, CommandAllocationSite, ModuleCommandBindings, SourceCommandTarget,
};
use super::target_selection::implementation_dependency;
use super::{
    CompilerInvocation, SourceNativeNamespaceBindingPreparation, SourceNativeStructuredPreparation,
    SourceNativeSwitchPreparation,
};
use tcl_registry::native_compilation::{NativeCompilationGuard, NativeCompilationSelection};

pub(super) struct RegisteredCompilerSelection {
    pub(super) spec: tcl_registry::native_compilation::NativeCompilationSpec,
    pub(super) admission: NativeCompilationSelection,
    pub(super) compiler_preserves_names: bool,
    pub(super) dependencies: Vec<super::super::SourceNativeCompilationDependency>,
    pub(super) operand_layout: Option<Arc<super::super::SourceNativeOperandLayoutProof>>,
    pub(super) namespace_bindings: Option<Arc<SourceNativeNamespaceBindingPreparation>>,
    pub(super) switch: Option<Arc<SourceNativeSwitchPreparation>>,
    pub(super) structured: Option<Arc<SourceNativeStructuredPreparation>>,
}

pub(super) fn registered_compiler_selection(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
) -> Option<RegisteredCompilerSelection> {
    let CompilerInvocation {
        arguments, shapes, ..
    } = selected;
    let mut invocation = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(&target.command),
        arguments,
    );
    if let Some(dialect) = state.baseline.compilation_dialect() {
        invocation = invocation.with_dialect(dialect);
    }
    let facts = super::super::resolve_source_invocation_facts(
        context.registry,
        context
            .registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile),
        invocation,
        context.realm(),
    );
    let Some(facts) = facts.as_ref() else {
        trace_missing_compiler_facts(target, state, context);
        return None;
    };
    let descriptor = original_registration_descriptor(
        selected.words,
        selected.head,
        selected.offset,
        facts,
        state,
        context,
    );
    let (spec, argument_offset) = descriptor?;
    let original =
        original_registered_compiler_preparation(spec, selected, state, context, argument_offset);
    let admission = original.as_ref().map_or_else(
        || registered_fallback_admission(spec, selected, facts, state, context, invocation),
        |(selection, _)| *selection,
    );
    let dependencies = compiler_dependencies(spec, target, state, context);
    let dependency_closed = dependencies.is_some();
    let compiler_preserves_names = original_compiler_preserves_names(
        spec,
        selected,
        state,
        context,
        argument_offset,
        dependency_closed,
    );
    let (original_site, original_dependency) =
        original_compilation_point(target, selected, state, context, admission);
    let preparation = dependency_closed
        .then(|| original.as_ref()?.1.as_ref())
        .flatten();
    let OriginalCompilerPreparations {
        namespace_bindings,
        switch,
        structured,
    } = retain_original_preparations(preparation, original_site.as_ref(), &original_dependency);
    let operand_layout = super::super::operand_layout::retain_layout(
        spec,
        super::super::operand_layout::LayoutInvocation {
            invocation,
            shapes,
            target,
            head: selected.head,
            offset: selected.offset,
        },
        dependency_closed,
        state,
        context,
    );
    Some(RegisteredCompilerSelection {
        spec,
        compiler_preserves_names,
        dependencies: dependencies.unwrap_or_default(),
        admission: if dependency_closed {
            admission
        } else {
            NativeCompilationSelection::Unknown
        },
        operand_layout,
        namespace_bindings,
        switch,
        structured,
    })
}

fn registered_fallback_admission(
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    selected: CompilerInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    invocation: tcl_registry::InvocationWords<'_>,
) -> NativeCompilationSelection {
    let shapes = selected.shapes;
    if original_named_selection_is_closed(selected.words, selected.offset, facts, state, context) {
        spec.select_for_original_facts(
            invocation,
            shapes,
            facts,
            state.baseline.compilation_dialect(),
            context.compilation(),
        )
    } else {
        spec.select_for_facts(
            invocation,
            shapes,
            facts,
            state.baseline.compilation_dialect(),
            context.compilation(),
        )
    }
}

/// Whether the retained original command may select a named ensemble
/// compiler from its resolved facts. The handler must declare its transitions:
/// none at all, or only the namespace pattern statements whose closed transfer
/// and premises the namespace-binding seal owns. An undeclared or other
/// transition keeps the compiler selection unknown, and so does an earlier
/// ensemble creation, which may have replaced a stock ensemble command.
pub(in crate::command_binding) fn original_named_selection_is_closed(
    words: &[tcl_source::words::WordExpr],
    offset: u32,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
) -> bool {
    if state.namespace_resolution.ensembles.created {
        return false;
    }
    let Some(origin) = state.current_source_origin.as_ref() else {
        return false;
    };
    if origin
        .original_lexical_command(words, offset, context.config)
        .is_none()
    {
        return false;
    }
    // The compileProc selects its named invocation from the original vector
    // alone (`namespace import` is `TclCompileBasicMin0ArgCmd` in 8.6–9.1,
    // like `export` and `forget`). Whether the invocation's own transitions
    // then close (an `auto_import` prelude, an occupied destination) is the
    // handler's obligation after dispatch, as in a directly evaluated
    // script; it does not make the compiler's selection unknown.
    facts.state_transitions.is_declared()
}

fn original_compilation_point(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    admission: NativeCompilationSelection,
) -> (
    Option<CommandAllocationSite>,
    super::super::SourceNativeCompilationDependency,
) {
    let original_site = state
        .current_source_origin
        .as_ref()
        .map(|source| CommandAllocationSite {
            source: Arc::clone(source),
            offset: selected.offset,
        });
    let preparation_guard = match admission {
        NativeCompilationSelection::Inline { guard, .. } => guard,
        _ => NativeCompilationGuard::ChunkEntry,
    };
    let dependency =
        original_compilation_dependency(target, selected, state, context, preparation_guard);
    (original_site, dependency)
}

fn trace_missing_compiler_facts(
    target: &SourceCommandTarget,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC").is_some() {
        eprintln!(
            "NATIVE_ORIGINAL_COMPILER_FACTS_MISSING command={:?} registry={} compiler_hook={:?}",
            target.command,
            target.registry_backed,
            context
                .compilation_snapshot
                .map_or(state, |snapshot| &snapshot.table.state)
                .runtime_compiler_hook(target.identity.as_ref()),
        );
    }
}

fn original_compilation_dependency(
    target: &SourceCommandTarget,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    preparation_guard: NativeCompilationGuard,
) -> super::super::SourceNativeCompilationDependency {
    super::super::SourceNativeCompilationDependency {
        compiler_prerequisite: state
            .runtime_command_compiler_prerequisite(
                selected.head,
                &context.namespace_identity(),
                match preparation_guard {
                    NativeCompilationGuard::ChunkEntry => {
                        tcl_runtime_api::CommandBindingGuard::ChunkEntry
                    }
                    NativeCompilationGuard::BeforeArguments => {
                        tcl_runtime_api::CommandBindingGuard::BeforeArguments
                    }
                },
            )
            .map(Arc::new),
        target: target.clone(),
        namespace: context.namespace.to_owned(),
        namespace_key: context.namespace_identity(),
        head: selected.head.to_owned(),
        guard: preparation_guard,
    }
}

pub(super) fn original_compiler_preserves_names(
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    argument_offset: usize,
    dependency_closed: bool,
) -> bool {
    let Some(origin) = state.current_source_origin.as_ref() else {
        return false;
    };
    let Some(projection) =
        origin.original_lexical_command(selected.words, selected.offset, context.config)
    else {
        return false;
    };
    let Some(protocol) =
        super::super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(state)
    else {
        return false;
    };
    let Ok(captured) = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        projection.native_words(),
        protocol,
    ) else {
        return false;
    };
    let dialect = state.baseline.compilation_dialect();
    let compiler_state = context
        .compilation_snapshot
        .map_or(state, |snapshot| &snapshot.table.state);
    match spec.original_argument_name_effects_in_context(
        &captured,
        argument_offset + 1,
        dialect,
        context.compilation(),
    ) {
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::Preserved => {
            // A selected null compileProc cannot enter a private compiler worker.
            dialect.is_some_and(|dialect| spec.compiler_hook_presence(dialect) == Some(false))
                || dependency_closed
        }
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::FixedLookup(
            lookup,
        ) => {
            dependency_closed
                && original_fixed_compiler_lookup_preserves_names(
                    compiler_state,
                    lookup,
                    compiler_state
                        .baseline
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe),
                )
        }
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::FixedLookupThenVisits { lookup, visits } => {
            dependency_closed
                && original_fixed_compiler_lookup_preserves_names(
                    compiler_state,
                    lookup,
                    compiler_state.baseline.execution_name_policy.and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe),
                )
                && super::super::original_compiler_effects::preserves_visits(&captured, visits, compiler_state, context, origin)
        }
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::Visits(visits) => {
            dependency_closed
                && super::super::original_compiler_effects::preserves_visits(
                    &captured,
                    visits,
                    compiler_state,
                    context,
                    origin,
                )
        }
        tcl_registry::native_compilation::NativeOriginalCompilerNameEffects::Unknown => false,
    }
}

pub(in super::super) fn original_fixed_compiler_lookup_preserves_names(
    state: &ModuleCommandBindings,
    lookup: &tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    policy: Option<tcl_syntax::naming::NamePolicyProtocol>,
) -> bool {
    if state.baseline.unknown_entry {
        return false;
    }
    // Actual runtime lookup absence has its own same-entry issuer. Closed
    // tables and the authored initial world cannot supply this callback axis.
    if let Some(entry) = &state.baseline.native_entry
        && !entry
            .command_resolvers
            .is_some_and(|inventory| inventory.permits_no_callbacks(entry))
    {
        return false;
    }
    let Some(policy) = policy else {
        return false;
    };
    let Some(root) = state.source_root_namespace_key() else {
        return false;
    };
    let Some(target) = state.original_registry_metadata_target(&root, lookup.slot, policy) else {
        return false;
    };
    target.registry_backed
        && target.terminal
        && target.kind == BindingKind::Builtin
        && target.implementation_generation == 0
        && target.prepended.is_empty()
        && tcl_registry::command_descriptor_labels_match(&target.command, lookup.slot)
}

/// A monolithic compileProc owns the complete original command vector. The
/// runtime member's body and effect descriptor does not replace that compiler.
pub(in super::super) fn original_registration_descriptor(
    words: &[tcl_source::words::WordExpr],
    head: &str,
    offset: u32,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
) -> Option<(
    tcl_registry::native_compilation::NativeCompilationSpec,
    usize,
)> {
    let compiler_state = context
        .compilation_snapshot
        .map_or(state, |snapshot| &snapshot.table.state);
    let registered = compiler_state.runtime_command_compiler_prerequisite(
        head,
        &context.namespace_identity(),
        tcl_runtime_api::CommandBindingGuard::ChunkEntry,
    );
    if let Some(registered) = registered
        && registered.compiler.ensemble.is_none()
    {
        let origin = state.current_source_origin.as_ref()?;
        let original = origin.original_lexical_command(words, offset, context.config)?;
        let protocol =
            super::super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(
                state,
            )?;
        let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            original.native_words(),
            protocol,
        )
        .ok()?;
        let spec = context
            .registry
            .native_compilation_for_original_registration(
                &registered.compiler.registry_identity,
                &captured,
                1,
                state.baseline.compilation_dialect()?,
            )?;
        return Some((spec, 0));
    }
    let spec = facts.native_compilation?;
    let argument_offset = spec
        .original_operand_from_for_facts(facts)?
        .checked_sub(1)?;
    Some((spec, argument_offset))
}

fn original_registered_compiler_preparation(
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    selected: CompilerInvocation<'_>,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
    argument_offset: usize,
) -> Option<(
    NativeCompilationSelection,
    Option<crate::registry_invocation::OriginalNativeCompilerPreparation>,
)> {
    let origin = state.current_source_origin.as_ref()?;
    crate::registry_invocation::original_native_compilation(
        spec,
        crate::registry_invocation::OriginalNativeCompilerInvocation {
            image: origin.source_image(),
            words: selected.words,
            offset: selected.offset,
            config: context.config,
            source_protocol:
                super::super::compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(
                    state,
                ),
            compiler_dialect: state.baseline.compilation_dialect(),
            context: context.compilation(),
            operand_from: argument_offset + 1,
        },
    )
}

struct OriginalCompilerPreparations {
    pub(super) namespace_bindings: Option<Arc<SourceNativeNamespaceBindingPreparation>>,
    pub(super) switch: Option<Arc<SourceNativeSwitchPreparation>>,
    pub(super) structured: Option<Arc<SourceNativeStructuredPreparation>>,
}

fn retain_original_preparations(
    preparation: Option<&crate::registry_invocation::OriginalNativeCompilerPreparation>,
    site: Option<&CommandAllocationSite>,
    dependency: &super::super::SourceNativeCompilationDependency,
) -> OriginalCompilerPreparations {
    let namespace_bindings = preparation.and_then(|preparation| {
        Some(Arc::new(SourceNativeNamespaceBindingPreparation {
            compilation_site: site?.clone(),
            dependency: dependency.clone(),
            recipe: preparation.namespace_bindings()?.clone(),
        }))
    });
    let switch = preparation.and_then(|preparation| {
        Some(Arc::new(SourceNativeSwitchPreparation {
            compilation_site: site?.clone(),
            dependency: dependency.clone(),
            recipe: preparation.switch()?.clone(),
        }))
    });
    let structured = preparation.and_then(|preparation| {
        Some(Arc::new(SourceNativeStructuredPreparation {
            compilation_site: site?.clone(),
            dependency: dependency.clone(),
            recipe: preparation.structured()?.clone(),
        }))
    });
    OriginalCompilerPreparations {
        namespace_bindings,
        switch,
        structured,
    }
}

pub(in super::super) fn compiler_dependencies(
    spec: tcl_registry::native_compilation::NativeCompilationSpec,
    target: &SourceCommandTarget,
    state: &ModuleCommandBindings,
    context: super::super::SourceExecutionContext<'_>,
) -> Option<Vec<super::super::SourceNativeCompilationDependency>> {
    let lookups = spec.implementation_prerequisites(state.baseline.compilation_dialect()?)?;
    if lookups.is_empty() {
        return Some(Vec::new());
    }
    let snapshot = context.compilation_snapshot?;
    let mut current = target.clone();
    let mut dependencies = Vec::with_capacity(lookups.len());
    for lookup in &lookups {
        let mapping_closed = snapshot
            .table
            .state
            .original_ensemble_mapping_holds(&current, lookup);
        let dependency = mapping_closed
            .then(|| {
                implementation_dependency(
                    lookup,
                    &snapshot.table.state,
                    NativeCompilationGuard::ChunkEntry,
                )
            })
            .flatten();
        let dependency = dependency?;
        current = dependency.target.clone();
        dependencies.push(dependency);
    }
    Some(dependencies)
}
