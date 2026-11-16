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

//! Building a `FunctionUnit`: the `build*` constructors and the shared full pipeline behind them (SSA, SCCP, derived artefacts).

#[cfg(any(test, debug_assertions))]
use super::SourceFunctionPhaseTrace;
use super::{
    FunctionBuildInputs, FunctionLatticeInputs, FunctionUnit, MethodBodyFacts, ModuleAnalysisFacts,
    ModuleTraceFacts, ParamSeeds, UnitDialect,
};
use crate::cfg::Function as CfgFunction;
use crate::def_use::{DefUseFacts, build_def_use_chains};
use crate::rendered_properties::{RenderedValueProps, propagate_rendered_props_with_config};
use crate::sccp::SccpResult;
use crate::semantic_analysis::SemanticAnalysisBundle;
use crate::ssa::{SsaFunction, ValueKey, build_ssa_with_context_for_entry_and_metadata};
use crate::taint::{
    TaintGraph, TaintLattice, TaintPropagationInputs, TaintSourceContext,
    instance_classes_for_function, propagate_taints,
};
use crate::type_infer::{TypePropagationMetadata, propagate_types_with_metadata_context};
use crate::types::TypeLattice;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::AnalysisTier;

#[derive(Clone, Copy)]
struct FullSccpInputs<'a> {
    pub(super) cfg: &'a CfgFunction,
    pub(super) inputs: FunctionBuildInputs<'a>,
    pub(super) ssa: &'a SsaFunction,
    pub(super) existence: crate::sccp::ExistenceEntry<'a>,
    pub(super) tier: AnalysisTier,
    fold_policy: crate::tcl_expr_eval::FoldPolicy,
}

struct FullSccpArtifacts {
    fold_policy: crate::tcl_expr_eval::FoldPolicy,
    pub(super) dynamic_names: crate::dynamic_names::DynamicNameBarrier,
    pub(super) sccp: SccpResult,
}

struct FullDerivedArtifacts {
    pub(super) sccp: SccpResult,
    pub(super) dynamic_names: crate::dynamic_names::DynamicNameBarrier,
    pub(super) semantic_value_projection: Arc<crate::sccp::SemanticValueProjection>,
    pub(super) types: HashMap<ValueKey, TypeLattice>,
    pub(super) return_type: TypeLattice,
    pub(super) rendered_props: HashMap<ValueKey, RenderedValueProps>,
    pub(super) taints: HashMap<ValueKey, TaintLattice>,
}

fn build_full_ssa_artifacts(
    cfg: &CfgFunction,
    inputs: FunctionBuildInputs<'_>,
    #[cfg(any(test, debug_assertions))] phase_trace: &mut SourceFunctionPhaseTrace,
) -> (SsaFunction, DefUseFacts, crate::tcl_expr_eval::FoldPolicy) {
    let entry = function_source_entry(cfg, inputs.entry_context);
    let fold_policy = crate::tcl_expr_eval::FoldPolicy::for_retained_entry(
        inputs.registry,
        entry.invocation_dialect,
        &inputs.config,
    )
    .with_expression_parser(entry.expression_parser);
    let metadata = inputs.source_metadata_input.and_then(|input| {
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            inputs.registry,
            input,
            inputs.config,
            Some(input.unit_profile()),
        )
    });
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(0);
    let ssa = build_ssa_with_context_for_entry_and_metadata(
        cfg,
        inputs.registry,
        inputs.config,
        entry,
        Some(inputs.params),
        metadata,
        cfg.metadata_context.is_standalone(),
    );
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(1);
    let def_use = build_def_use_chains(&ssa, Some(cfg), inputs.config);
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(2);
    (ssa, def_use, fold_policy)
}

fn build_full_sccp_artifacts(
    full: &FullSccpInputs<'_>,
    #[cfg(any(test, debug_assertions))] phase_trace: &mut SourceFunctionPhaseTrace,
) -> FullSccpArtifacts {
    let FullSccpInputs {
        cfg,
        inputs,
        ssa,
        existence,
        tier,
        fold_policy,
    } = *full;
    let dynamic_names =
        crate::dynamic_names::dynamic_name_barrier(cfg, inputs.registry, inputs.config);
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(3);
    let sccp = crate::sccp::sccp_in_module(&crate::sccp::SolveInputs {
        cfg,
        ssa,
        param_constants: inputs.param_constants,
        policy: fold_policy,
        extra_escaping: inputs.extra_global_escaping,
        trace: crate::sccp::TraceInputs {
            source_metadata_input: inputs.source_metadata_input,
            registry: inputs.registry,
            traced_variables: inputs.trace_facts.traced_variables,
            // SCCP adds the computed-name barrier of the blocks it finds
            // executable itself (`sccp::RunHazards`).
            has_dynamic_variable_trace: inputs.trace_facts.has_dynamic_variable_trace,
            deferred_writes: inputs.trace_facts.deferred_writes,
            analysis_context: inputs.analysis_context,
            existence: (tier == AnalysisTier::Deep).then_some(existence),
        },
        folds: Some(crate::sccp::BuiltinFoldInputs {
            source_metadata_input: inputs.source_metadata_input,
            registry: inputs.registry,
            mutations: inputs.command_trust,
            dialect: None,
            defining_class: None,
            registry_engine: false,
            trust: crate::sccp::FoldTrust::ObservedBindings,
            proven_pure_parameters: false,
        }),
        module: crate::sccp::ModuleRun {
            analysis_body: inputs.analysis_body,
            ..crate::sccp::ModuleRun::reading(inputs.procedures)
        },
    });
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(4);
    // SCCP solved under the whole-function barrier, so a block it leaves
    // unexecutable is unreachable under any narrower one too.
    let dynamic_names = if sccp.executable_blocks.len() == cfg.blocks.len() {
        dynamic_names
    } else {
        crate::dynamic_names::dynamic_name_barrier_in(cfg, inputs.registry, inputs.config, |id| {
            sccp.executable_blocks.contains(id)
        })
    };
    FullSccpArtifacts {
        fold_policy,
        dynamic_names,
        sccp,
    }
}

fn propagate_full_type_artifacts(
    cfg: &CfgFunction,
    inputs: FunctionBuildInputs<'_>,
    ssa: &SsaFunction,
    sccp: &SccpResult,
) -> (HashMap<ValueKey, TypeLattice>, TypeLattice) {
    let type_metadata = TypePropagationMetadata {
        registry: inputs.registry,
        context: inputs.source_metadata_input.and_then(|input| {
            crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                inputs.registry,
                input,
            )
        }),
        config: inputs.config,
        numbers: inputs
            .source_metadata_input
            .map_or(inputs.registry.numbers(), |input| {
                input.unit_profile().grammar.numbers
            }),
    };
    let types = propagate_types_with_metadata_context(
        cfg,
        ssa,
        sccp,
        type_metadata,
        inputs.known_classes,
        inputs.extra_global_escaping,
        inputs.trace_facts,
    );
    let return_type = crate::type_infer::infer_function_return_type(
        cfg,
        sccp,
        &types,
        type_metadata,
        inputs.known_classes,
        ssa,
    );
    (types, return_type)
}

fn build_full_derived_artifacts(
    cfg: &CfgFunction,
    inputs: FunctionBuildInputs<'_>,
    ssa: &SsaFunction,
    mut core: FullSccpArtifacts,
    #[cfg(any(test, debug_assertions))] phase_trace: &mut SourceFunctionPhaseTrace,
) -> FullDerivedArtifacts {
    let value_inputs = inputs.value_fact_inputs(core.fold_policy);
    let semantic_value_projection =
        Arc::new(crate::sccp::SemanticValueProjection::new(value_inputs));
    if inputs.initial_state == crate::sccp::ExistenceInitialState::DeclaredFrame
        || ssa.point_contexts.is_some()
    {
        extend_existence_folds(
            &mut core.sccp,
            cfg,
            ssa,
            crate::sccp::ExistenceFrame {
                params: inputs.params,
                object_state: inputs.object_state,
                initial_global: inputs.initial_global,
            },
            crate::sccp::ExistenceMetadataInputs {
                registry: inputs.registry,
                context: inputs.source_metadata_input.and_then(|input| {
                    crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
                        inputs.registry,
                        input,
                    )
                }),
            },
            inputs.config,
        );
    }
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(5);
    let (types, return_type) = propagate_full_type_artifacts(cfg, inputs, ssa, &core.sccp);
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(6);
    let rendered_props = propagate_rendered_props_with_config(
        cfg,
        ssa,
        &core.sccp,
        inputs.registry,
        Some(inputs.config),
    );
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(7);
    let instance_classes = instance_classes_for_function(
        cfg,
        inputs.registry,
        None,
        false,
        TaintSourceContext::for_input(inputs.registry, inputs.source_metadata_input, inputs.config),
    );
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(8);
    let taints = propagate_taints(
        &TaintGraph::new(cfg, ssa, &core.sccp),
        TaintPropagationInputs {
            registry: inputs.registry,
            rendered_props: Some(&rendered_props),
            interproc: None,
            source: TaintSourceContext::for_input(
                inputs.registry,
                inputs.source_metadata_input,
                inputs.config,
            ),
            param_taints: None,
            taint_summaries: None,
            instance_classes: &instance_classes,
        },
    );
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase(9);
    FullDerivedArtifacts {
        sccp: core.sccp,
        dynamic_names: core.dynamic_names,
        semantic_value_projection,
        types,
        return_type,
        rendered_props,
        taints,
    }
}

impl FunctionUnit {
    /// Build per-function analyses from a CFG + its source
    /// parameters. Does *not* populate `memory_ssa`; call
    /// [`FunctionUnit::with_memory_ssa`] when the caller needs
    /// it.
    ///
    /// Runs in order: SSA → def-use → SCCP → type-propagation →
    /// rendered-properties → taint-propagation.
    #[must_use]
    pub fn build(
        name: impl Into<String>,
        cfg: CfgFunction,
        params: &[String],
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        Self::build_with_param_constants(name, cfg, params, registry, None, config)
    }

    /// Like [`Self::build`] but seeds SCCP with interprocedurally-collected
    /// caller-side parameter constants (`param_constants`), so a callee that
    /// reads a param every caller passes the same literal for folds it.
    #[must_use]
    pub fn build_with_param_constants(
        name: impl Into<String>,
        cfg: CfgFunction,
        params: &[String],
        registry: &CommandRegistry,
        param_constants: Option<
            &std::collections::HashMap<
                (String, crate::ssa::Version),
                crate::analyses::LatticeValue,
            >,
        >,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        Self::build_with_param_constants_and_classes(
            name,
            cfg,
            params,
            UnitDialect {
                registry,
                config,
                source_metadata_input: None,
            },
            param_constants,
            &HashSet::new(),
            ModuleTraceFacts::none(),
        )
    }

    /// Like [`Self::build_with_param_constants`] but additionally threads the
    /// compilation unit's `known_classes` set into type propagation and return
    /// inference, so a `TclOO` / itcl constructor call (`Foo new` / `Foo create
    /// x`) whose head names a known class is typed `OBJECT(::ns::Foo)` (the
    /// signal the analyser's W307 / W308 method-dispatch checks consume).  The
    /// per-function [`Self::build`] / [`Self::build_with_param_constants`]
    /// entry points default to an empty set (no object typing); the
    /// compilation-unit builders ([`Self::build_for`] and friends) source the
    /// real set from [`crate::signature_scan`].
    ///
    /// `trace_facts` ([`ModuleTraceFacts`]) is the module-wide variable-trace
    /// fact, so a name traced by a *different* proc still forces SCCP to
    /// `Overdefined` here. [`ModuleTraceFacts::none()`] for the
    /// module-context-free entry points ([`Self::build`],
    /// [`Self::build_with_param_constants`]) and for isolated single-function
    /// rebuilds that have no module to scan.
    #[must_use]
    pub fn build_with_param_constants_and_classes(
        name: impl Into<String>,
        cfg: CfgFunction,
        params: &[String],
        dialect: UnitDialect<'_>,
        param_constants: Option<
            &std::collections::HashMap<
                (String, crate::ssa::Version),
                crate::analyses::LatticeValue,
            >,
        >,
        known_classes: &HashSet<String>,
        trace_facts: ModuleTraceFacts<'_>,
    ) -> Self {
        Self::build_for_lattice(
            name,
            cfg,
            params,
            FunctionLatticeInputs {
                dialect,
                param_constants,
                known_classes,
                trace_facts,
                analysis_context: &crate::value_transfer::AnalysisContextKey::detached(),
                command_trust: &crate::command_binding::ModuleCommandMutations::default(),
                event_body: None,
            },
        )
    }

    /// Build a memo lattice under its exact sealed source entry.
    /// A foreign event body, configuration or Registry declines deep analysis.
    #[must_use]
    pub fn build_for_lattice(
        name: impl Into<String>,
        cfg: CfgFunction,
        params: &[String],
        inputs: FunctionLatticeInputs<'_>,
    ) -> Self {
        let FunctionLatticeInputs {
            dialect:
                UnitDialect {
                    registry,
                    config,
                    source_metadata_input,
                },
            param_constants,
            known_classes,
            trace_facts,
            analysis_context,
            command_trust,
            event_body,
        } = inputs;
        let entry = match event_body {
            Some((event, body)) => {
                let Some(entry) = event.conditional_lattice_entry(body, config, registry) else {
                    return Self::trivial_guarded(name, cfg)
                        .with_source_metadata(source_metadata_input, config);
                };
                Some(entry)
            }
            None => None,
        };
        let no_extra_escaping = HashSet::new();
        let mut unit = Self::build_full(
            name,
            cfg,
            FunctionBuildInputs {
                is_event_body: false,
                analysis_body: None,
                initial_state: crate::sccp::ExistenceInitialState::DeclaredFrame,
                source_metadata_input,
                entry_context: entry.as_ref(),
                config,
                params,
                registry,
                param_constants,
                known_classes,
                extra_global_escaping: &no_extra_escaping,
                trace_facts,
                command_trust,
                analysis_context: Some(analysis_context),
                object_state: None,
                initial_global: false,
                procedures: None,
            },
        );
        unit.irules_event_body = event_body.map(|(event, _)| Arc::clone(event));
        unit
    }

    /// Source declaration analysis with its independently retained namespace.
    /// The original script supplies no entered activation or Native frame grant.
    #[must_use]
    pub fn build_for_original_declaration_lattice(
        body_id: crate::ir::ProcedureId,
        name: &str,
        cfg: CfgFunction,
        params: &[String],
        inputs: FunctionLatticeInputs<'_>,
    ) -> Self {
        let FunctionLatticeInputs {
            dialect:
                UnitDialect {
                    registry,
                    config,
                    source_metadata_input,
                },
            param_constants,
            known_classes,
            trace_facts,
            analysis_context: _,
            command_trust,
            event_body: _,
        } = inputs;
        let entry = conditional_function_source_entry(&cfg, source_metadata_input);
        Self::build_full(
            name,
            cfg,
            FunctionBuildInputs {
                is_event_body: false,
                analysis_body: Some(crate::sccp::AnalysisBody::Procedure(body_id)),
                initial_state: crate::sccp::ExistenceInitialState::DeclaredFrame,
                source_metadata_input,
                entry_context: Some(&entry),
                config,
                params,
                registry,
                param_constants,
                known_classes,
                extra_global_escaping: &HashSet::new(),
                trace_facts,
                analysis_context: None,
                command_trust,
                object_state: None,
                initial_global: false,
                procedures: None,
            },
        )
        .with_body_id(body_id)
    }

    /// Conditional inherited-frame facts for an admitted original child CFG.
    /// The actual source namespace is independent of the synthetic report name.
    pub(super) fn build_conditional_original_body(
        cfg: CfgFunction,
        inputs: FunctionLatticeInputs<'_>,
    ) -> Self {
        let FunctionLatticeInputs {
            dialect:
                UnitDialect {
                    registry,
                    config,
                    source_metadata_input,
                },
            param_constants: _,
            known_classes,
            trace_facts,
            analysis_context,
            command_trust,
            event_body: _,
        } = inputs;
        let entry = conditional_function_source_entry(&cfg, source_metadata_input);
        let no_extra_escaping = HashSet::new();
        Self::build_full(
            "::conditional_original_body",
            cfg,
            FunctionBuildInputs {
                is_event_body: false,
                analysis_body: None,
                initial_state: crate::sccp::ExistenceInitialState::InheritedFrame,
                source_metadata_input,
                entry_context: Some(&entry),
                config,
                params: &[],
                registry,
                param_constants: None,
                known_classes,
                extra_global_escaping: &no_extra_escaping,
                trace_facts,
                analysis_context: Some(analysis_context),
                command_trust,
                object_state: None,
                initial_global: false,
                procedures: None,
            },
        )
    }

    /// [`Self::build_with_param_constants_and_classes`] under the module's
    /// analysis context: the per-procedure path of a unit build and of the
    /// memoised lattice, whose key carries the same context. The command
    /// trust the driver folds under is the context's own
    /// [`crate::command_binding::CommandTrustSnapshot`], so a memoised unit
    /// and a fresh one answer alike by construction, and a module whose
    /// bindings differ is a different key rather than a stale hit (#2164).
    #[must_use]
    pub fn build_with_param_constants_and_classes_under(
        name: impl Into<String>,
        cfg: CfgFunction,
        params: &[String],
        dialect: UnitDialect<'_>,
        param_constants: Option<
            &std::collections::HashMap<
                (String, crate::ssa::Version),
                crate::analyses::LatticeValue,
            >,
        >,
        known_classes: &HashSet<String>,
        facts: ModuleAnalysisFacts<'_>,
    ) -> Self {
        let no_extra_escaping = HashSet::new();
        let UnitDialect {
            registry,
            config,
            source_metadata_input,
        } = dialect;
        Self::build_full(
            name.into(),
            cfg,
            FunctionBuildInputs {
                is_event_body: false,
                analysis_body: None,
                initial_state: crate::sccp::ExistenceInitialState::DeclaredFrame,
                source_metadata_input,
                entry_context: None,
                config,
                params,
                registry,
                param_constants,
                known_classes,
                extra_global_escaping: &no_extra_escaping,
                trace_facts: facts.trace,
                analysis_context: Some(facts.analysis_context),
                command_trust: facts.command_trust,
                object_state: None,
                initial_global: false,
                procedures: None,
            },
        )
    }

    /// [`Self::build_with_param_constants_and_classes_under`] with the
    /// module's procedures in hand, which a parameter default `info default`
    /// names is read from.
    #[must_use]
    pub(super) fn build_procedure_in_module(
        (id, is_event_body, name): (crate::ir::ProcedureId, bool, &str),
        cfg: CfgFunction,
        (params, param_constants): (&[String], ParamSeeds<'_>),
        dialect: UnitDialect<'_>,
        known_classes: &HashSet<String>,
        facts: ModuleAnalysisFacts<'_>,
        procedures: Option<&crate::interprocedural::ModuleProcedures<'_>>,
    ) -> Self {
        let no_extra_escaping = HashSet::new();
        let UnitDialect {
            registry,
            config,
            source_metadata_input,
        } = dialect;
        Self::build_full(
            name.to_owned(),
            cfg,
            FunctionBuildInputs {
                is_event_body,
                analysis_body: Some(crate::sccp::AnalysisBody::Procedure(id)),
                initial_state: crate::sccp::ExistenceInitialState::DeclaredFrame,
                source_metadata_input,
                entry_context: None,
                config,
                params,
                registry,
                param_constants,
                known_classes,
                extra_global_escaping: &no_extra_escaping,
                trace_facts: facts.trace,
                analysis_context: Some(facts.analysis_context),
                command_trust: facts.command_trust,
                object_state: None,
                initial_global: false,
                procedures,
            },
        )
    }

    /// Build the compilation unit's **top-level** body unit — no parameters,
    /// no interprocedural seeds, no object state, but a non-empty
    /// `extra_global_escaping` (top-level names already live in the global
    /// frame, so another procedure's `global NAME` can reassign them; see
    /// [`crate::sccp::sccp_with_extra_escaping`]).
    #[must_use]
    pub(crate) fn build_top_level(
        cfg: CfgFunction,
        dialect: UnitDialect<'_>,
        known_classes: &HashSet<String>,
        extra_global_escaping: &HashSet<String>,
        trace_facts: ModuleTraceFacts<'_>,
        command_trust: &crate::command_binding::ModuleCommandMutations,
        procedures: Option<&crate::interprocedural::ModuleProcedures<'_>>,
    ) -> Self {
        let UnitDialect {
            registry,
            config,
            source_metadata_input,
        } = dialect;
        Self::build_full(
            "::top".to_owned(),
            cfg,
            FunctionBuildInputs {
                is_event_body: false,
                analysis_body: Some(crate::sccp::AnalysisBody::TopLevel),
                initial_state: crate::sccp::ExistenceInitialState::DeclaredFrame,
                source_metadata_input,
                entry_context: None,
                config,
                params: &[],
                registry,
                param_constants: None,
                known_classes,
                extra_global_escaping,
                trace_facts,
                analysis_context: None,
                command_trust,
                object_state: None,
                initial_global: true,
                procedures,
            },
        )
    }

    /// Build a **`TclOO` method body**'s unit from its typed
    /// [`crate::ir::MethodDef`], so the analyses see the method's own params
    /// *and* the class's instance variables (the cross-definition-block union
    /// lowering computes — see [`crate::ir::MethodDef::instance_vars`]).
    ///
    /// The instance-variable half feeds the existence fold (a class-level
    /// `variable x` binds `x` in every method frame with no binding command
    /// in the body, so without it `[info exists x]` folds to "always
    /// absent"), and — via [`Self::method_facts`], the single carrier built
    /// here — the analyser's W210/W211/W220 `known_bound` set and the
    /// optimiser's method-constants escaping set.
    #[must_use]
    pub fn build_for_method(
        name: impl Into<String>,
        cfg: CfgFunction,
        method: (crate::ir::ProcedureId, &crate::ir::MethodDef),
        dialect: UnitDialect<'_>,
        known_classes: &HashSet<String>,
        trace_facts: ModuleTraceFacts<'_>,
        command_trust: &crate::command_binding::ModuleCommandMutations,
    ) -> Self {
        let (body_id, method) = method;
        let facts = Arc::new(MethodBodyFacts::from_method(method));
        let mut entry_context = crate::var_resolve::ResolveContext::for_frame(
            crate::var_resolve::VariableFrameKind::Local,
            crate::var_resolve::BindingIdentity::Bound,
            None,
        );
        entry_context.instance_vars = if facts.original_source {
            crate::var_resolve::SharedSet::default()
        } else {
            method.instance_vars.iter().cloned().collect()
        };
        entry_context.instance_owner = if facts.original_source {
            String::new()
        } else {
            method.definition_scope.unwrap_or(body_id).reporting_key()
        };
        match method
            .body
            .execution_namespace(method.execution_namespace.clone())
        {
            crate::ir::ExecutionNamespace::Exact(namespace) => {
                entry_context.namespace.clone_from(&namespace);
                entry_context.known_namespaces.insert(namespace);
            }
            crate::ir::ExecutionNamespace::SourceContext(key) => {
                entry_context = entry_context.with_namespace_identity(key);
            }
            crate::ir::ExecutionNamespace::RuntimeSelected => entry_context.namespace_known = false,
        }

        if let Some(original) = &facts.original_receiver_context {
            entry_context = original.context().clone();
        }
        let no_extra_escaping = HashSet::new();
        let UnitDialect {
            registry,
            config,
            source_metadata_input,
        } = dialect;
        let mut unit = Self::build_full(
            name.into(),
            cfg,
            FunctionBuildInputs {
                is_event_body: false,
                analysis_body: Some(crate::sccp::AnalysisBody::Procedure(body_id)),
                initial_state: crate::sccp::ExistenceInitialState::DeclaredFrame,
                source_metadata_input,
                entry_context: Some(&entry_context),
                config,
                params: if facts.original_source {
                    &[]
                } else {
                    &facts.params
                },
                registry,
                param_constants: None,
                known_classes,
                extra_global_escaping: &no_extra_escaping,
                trace_facts,
                analysis_context: None,
                command_trust,
                object_state: (!facts.original_source).then_some(&facts.instance_vars),
                initial_global: false,
                procedures: None,
            },
        );
        unit.body_id = Some(body_id);
        unit.method_facts = Some(facts);
        unit
    }

    /// Shared body behind every `FunctionUnit` build. Takes its analysis inputs
    /// as one [`FunctionBuildInputs`] bundle beyond `name` / `cfg`.
    ///
    /// The top-level unit build passes a non-empty `extra_global_escaping` —
    /// see [`crate::sccp::sccp_with_extra_escaping`]. It widens SCCP's
    /// escaping-set because top-level names already live in the global frame,
    /// so a name never `global`-declared in the top-level body itself can still
    /// be reassigned by another procedure's own `global NAME` (see
    /// [`crate::var_observability::scan_module_global_names`]). Every
    /// per-procedure build passes an empty set via
    /// [`Self::build_with_param_constants_and_classes`].
    #[must_use]
    pub(super) fn build_full(
        name: impl Into<String>,
        mut cfg: CfgFunction,
        inputs: FunctionBuildInputs<'_>,
    ) -> Self {
        #[cfg(any(test, debug_assertions))]
        let mut phase_trace = SourceFunctionPhaseTrace::new();
        let name = name.into();
        // Complexity guard (block-count half): a pathologically large body
        // would cost seconds of SSA + dataflow for near-zero findings, so skip
        // the deep analysis and flag the unit. The body-byte half is applied by
        // the callers that have the body span (see `build_for_with_config`).
        // Backstop for every path through here — `build`, methods, and the
        // salsa `function_lattice` callbacks.
        if crate::ssa::is_complexity_guarded(&cfg) {
            return Self::trivial_guarded(name, cfg)
                .with_source_metadata(inputs.source_metadata_input, inputs.config);
        }
        let (ssa, def_use, fold_policy) = build_full_ssa_artifacts(
            &cfg,
            inputs,
            #[cfg(any(test, debug_assertions))]
            &mut phase_trace,
        );
        // The request's tier: a context key names it, a detached build is
        // deep. Below the deep tier the existence rung is not run, so its
        // every read answers `Unavailable` rather than a fact.
        let tier = inputs
            .analysis_context
            .map_or(AnalysisTier::Deep, |key| key.tier);
        let core = build_full_sccp_artifacts(
            &FullSccpInputs {
                cfg: &cfg,
                inputs,
                ssa: &ssa,
                existence: inputs.existence_entry(&name),
                tier,
                fold_policy,
            },
            #[cfg(any(test, debug_assertions))]
            &mut phase_trace,
        );
        let derived = build_full_derived_artifacts(
            &cfg,
            inputs,
            &ssa,
            core,
            #[cfg(any(test, debug_assertions))]
            &mut phase_trace,
        );
        #[cfg(any(test, debug_assertions))]
        phase_trace.finish(&cfg);
        cfg.retain_math_invocations(&derived.sccp.required_math_invocations);
        cfg.retain_expression_preparations(&derived.sccp.required_expression_preparations);
        Self {
            original_lattice_worker: None,
            original_lattice_callback_admission: None,
            source_metadata_input: inputs.source_metadata_input.cloned(),
            source_config: inputs.config,
            name,
            body_id: None,
            cfg,
            ssa,
            def_use,
            sccp: derived.sccp,
            retained_semantic_value_owner: Some(Arc::clone(&derived.semantic_value_projection)),
            semantic_value_projection: derived.semantic_value_projection,
            types: Arc::new(derived.types),
            return_type: derived.return_type,
            taints: Arc::new(derived.taints),
            rendered_props: Arc::new(derived.rendered_props),
            memory_ssa: None,
            dynamic_names: derived.dynamic_names,
            complexity_guarded: false,
            tier,
            base_offset: 0,
            method_facts: None,
            irules_event_body: None,
            original_hosted_procedure_body: None,
            semantic_facts: SemanticAnalysisBundle::unavailable(None),
        }
    }

    /// A trivial guarded unit for a body the complexity guard skips: trivial
    /// SSA, empty dataflow lattices, `complexity_guarded = true`. Per-proc
    /// diagnostic and optimiser passes consult the flag and skip it, so the
    /// empty lattices are never read as real facts.
    #[must_use]
    pub fn trivial_guarded(name: impl Into<String>, cfg: CfgFunction) -> Self {
        let ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
        Self {
            original_lattice_worker: None,
            original_lattice_callback_admission: None,
            source_metadata_input: None,
            // dialect-drift-ok: a complexity-guarded unit has no source analysis;
            // every pass skips it, so this config never lexes document text
            source_config: tcl_lexer::LexerConfig::default(),
            name: name.into(),
            body_id: None,
            cfg,
            ssa,
            def_use: DefUseFacts::default(),
            sccp: SccpResult::default(),
            semantic_value_projection: Arc::default(),
            retained_semantic_value_owner: None,
            types: Arc::default(),
            return_type: TypeLattice::unknown(),
            taints: Arc::default(),
            rendered_props: Arc::default(),
            memory_ssa: None,
            // A guarded unit's lattices are all trivial and every per-proc
            // pass skips it, so the barrier stays clear (never consulted).
            dynamic_names: crate::dynamic_names::DynamicNameBarrier::default(),
            complexity_guarded: true,
            tier: AnalysisTier::ComplexityGuarded,
            base_offset: 0,
            method_facts: None,
            irules_event_body: None,
            original_hosted_procedure_body: None,
            semantic_facts: SemanticAnalysisBundle::unavailable(None),
        }
    }
}

fn extend_existence_folds(
    sccp: &mut SccpResult,
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    frame: crate::sccp::ExistenceFrame<'_>,
    metadata: crate::sccp::ExistenceMetadataInputs<'_>,
    config: tcl_lexer::LexerConfig,
) {
    // The solver decides an existence condition itself where its existence
    // rung reaches the branch; this scan adds only the branches it left open,
    // so a block never carries the same decision twice.
    let folds = crate::sccp::existence_constant_branches_with_ssa_with_metadata_context(
        cfg, frame, metadata, config, ssa,
    );
    for fold in folds {
        if !sccp
            .constant_branches
            .iter()
            .any(|decided| decided.block == fold.block)
        {
            sccp.constant_branches.push(fold);
        }
    }
}

/// Explicit unknown-frame fallback for an original conditional child.
/// Genuine child source points still select their own original context.
pub(super) fn conditional_function_source_entry(
    cfg: &CfgFunction,
    input: Option<&crate::analyser::ResolvedAnalysisInput>,
) -> crate::var_resolve::ResolveContext {
    let mut entry = crate::var_resolve::ResolveContext::for_frame(
        crate::var_resolve::VariableFrameKind::Unknown,
        crate::var_resolve::BindingIdentity::Bound,
        input.map(crate::command_binding::source_input_dialect),
    );
    entry.source_read_configuration = input.map_or(
        crate::var_resolve::SourceReadConfiguration::Unavailable,
        |input| crate::var_resolve::SourceReadConfiguration::Selected(input.lexer_config()),
    );
    entry.namespace_known = false;
    if let Some(namespace) = cfg.namespace_context.as_deref() {
        entry = entry.with_namespace_identity(namespace.clone());
    }
    entry
}

/// Preserve the actual retained activation before applying a method overlay.
pub(crate) fn function_source_entry(
    cfg: &CfgFunction,
    entry_context: Option<&crate::var_resolve::ResolveContext>,
) -> crate::var_resolve::ResolveContext {
    let source_entry = cfg
        .command_binding_sites
        .iter()
        .filter_map(|site| {
            site.variable_context
                .as_ref()
                .map(|context| (site.span.start(), context))
        })
        .min_by_key(|(offset, _)| *offset)
        .map(|(_, context)| context.as_ref());
    let entry = if let Some(entered) =
        entry_context.filter(|entry| entry.execution_name_policy.is_some())
    {
        entered.clone()
    } else if let Some(source) = source_entry {
        let mut entry = source.clone();
        if let Some(overlay) = entry_context
            && entry.execution_name_policy.is_none()
        {
            entry
                .instance_vars
                .extend(overlay.instance_vars.iter().cloned());
            if !overlay.instance_owner.is_empty() {
                entry.instance_owner.clone_from(&overlay.instance_owner);
            }
            if entry.execution.is_none() {
                entry.execution = overlay.execution;
            }
            if entry.interpreter.is_none() {
                entry.interpreter.clone_from(&overlay.interpreter);
            }
        }
        entry
    } else if let Some(entry) = entry_context {
        entry.clone()
    } else if !cfg.metadata_context.is_standalone()
        || cfg.namespace_context.is_some()
        || cfg.executed_source.is_some()
        || !cfg.command_binding_sites.is_empty()
        || cfg.blocks.values().any(|block| {
            block.statements.iter().any(|statement| {
                statement
                    .tokens()
                    .is_some_and(|tokens| !tokens.words().is_empty())
            })
        })
    {
        conditional_function_source_entry(cfg, cfg.metadata_context.source_analysis_input())
    } else {
        // Explicit source-absent Standalone model data keeps its own frame API.
        crate::var_resolve::ResolveContext::for_function(&cfg.name)
    };
    cfg.namespace_context.as_deref().map_or_else(
        || entry.clone(),
        |key| entry.clone().with_namespace_identity(key.clone()),
    )
}
