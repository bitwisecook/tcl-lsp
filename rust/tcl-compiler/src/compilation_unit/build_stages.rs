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

//! The stages of a unit build and the driver that runs them, from retained lowering to post-analysis and assembly.

#[cfg(any(test, debug_assertions))]
use super::SourcePipelineTrace;
use super::lattice_request::ProcLatticeCache;
use super::module_facts::{RetainedSourceClassFacts, connection_scoped_names};
use super::procedure_units::{build_procedure_units, module_procedures};
use super::source_lowering::{
    SourceSyntaxInputs, lower_and_build_cfg_with_original_caches, module_source_metadata_input,
    prepared_command_trust, resolve_unit_scope,
};
use super::{
    BodyUnitContext, CompilationUnit, DeclaredBodyMemoContext, FunctionUnit, ModuleTraceFacts,
    ModuleWideFacts, OriginalBodyCacheInputs, OriginalLoweringCaches, ProcedureBuildContext,
    UnitBuildOptions, UnitCallerScope, UnitDialect, optimiser_source_owner,
};
use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::cfg_builder::PreparedCfgContext;
use crate::ir::Module as IrModule;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

pub(super) struct RetainedLoweredSource {
    pub(super) ir_module: IrModule,
    pub(super) cfg_module: CfgModule,
    pub(super) tainted_global_writes: HashMap<crate::ir::ProcedureId, HashSet<String>>,
    pub(super) prepared_cfg_context: PreparedCfgContext,
    pub(super) integrity: std::cell::Cell<bool>,
}

pub(super) struct RetainedLoweringInput<'a, 'b> {
    pub(super) source: &'a str,
    pub(super) options: UnitBuildOptions<'b>,
    pub(super) body_caches: OriginalBodyCacheInputs<'b>,
    pub(super) entry: Option<&'b crate::command_binding::SourceAnalysisEntry>,
    pub(super) context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
    pub(super) input: Option<&'b crate::analyser::ResolvedAnalysisInput>,
    pub(super) source_body_cache: Option<&'b crate::lowering::SourceBodySyntaxCache<'b>>,
    pub(super) standalone: Option<crate::lowering::StandaloneSourceInput<'b>>,
    pub(super) source_settings: Option<&'b Arc<crate::analyser::OriginalAnalyserSourceSettings>>,
}

pub(super) fn lower_retained_source(
    request: RetainedLoweringInput<'_, '_>,
    #[cfg(any(test, debug_assertions))] phase_trace: &mut SourcePipelineTrace,
) -> RetainedLoweredSource {
    // An unchanged document under inputs already lowered gets the same module
    // objects, so its procedure identities and source proofs recur.
    if let Some(cache) = request.body_caches.module
        && let Some(inputs) = super::ModuleLoweringInputs::of_request(&request)
        && let Some(retained) = cache(&inputs).and_then(|lowered| lowered.retained_for(&inputs))
    {
        return retained;
    }
    let RetainedLoweringInput {
        source,
        options,
        body_caches,
        entry,
        context,
        input,
        source_body_cache,
        standalone,
        source_settings,
    } = request;
    let integrity = std::cell::Cell::new(true);
    let (ir_module, cfg_module, tainted_global_writes, prepared_cfg_context) =
        lower_and_build_cfg_with_original_caches(
            source,
            options,
            OriginalLoweringCaches {
                legacy: body_caches.legacy,
                source: source_body_cache,
                original: body_caches.original,
                integrity: &integrity,
            },
            entry,
            context,
            input,
            (standalone, source_settings),
        );
    #[cfg(any(test, debug_assertions))]
    {
        phase_trace.procedures = ir_module.procedures.len();
        phase_trace.phase("lower-and-cfg");
    }
    RetainedLoweredSource {
        ir_module,
        cfg_module,
        tainted_global_writes,
        prepared_cfg_context,
        integrity,
    }
}

struct RetainedModuleFacts<'a> {
    pub(super) command_mutations: crate::command_binding::ModuleCommandMutations,
    proc_binding_trust: crate::command_binding::ProcBindingTrustProjection,
    pub(super) analysis_context: crate::value_transfer::AnalysisContextKey,
    pub(super) cfg_context: Option<&'a PreparedCfgContext>,
    call_site_constants: crate::unit_scope::CallSiteEvidence,
    linkage: tcl_registry::Traits,
    extra_callers: Vec<crate::unit_scope::ExtraCallSiteScanContext>,
    has_cross_file_evidence: bool,
    pub(super) tainted_global_writes: HashMap<crate::ir::ProcedureId, HashSet<String>>,
    module_wide: ModuleWideFacts,
    pub(super) trace_facts: ModuleTraceFacts<'a>,
}

struct RetainedModuleFactsInput<'a, 'b> {
    pub(super) source: &'a str,
    pub(super) options: UnitBuildOptions<'b>,
    pub(super) ir_module: &'a IrModule,
    pub(super) cfg_module: &'a CfgModule,
    pub(super) tainted_global_writes: HashMap<crate::ir::ProcedureId, HashSet<String>>,
    prepared_cfg_context: &'a PreparedCfgContext,
    cache_available: bool,
}

fn retained_module_facts<'a>(
    input: RetainedModuleFactsInput<'a, '_>,
    #[cfg(any(test, debug_assertions))] phase_trace: &mut SourcePipelineTrace,
) -> RetainedModuleFacts<'a> {
    let RetainedModuleFactsInput {
        source,
        options,
        ir_module,
        cfg_module,
        tainted_global_writes,
        prepared_cfg_context,
        cache_available,
    } = input;
    let registry = options.registry;
    let (command_mutations, proc_binding_trust) =
        prepared_command_trust(ir_module, registry, prepared_cfg_context);
    let analysis_context =
        crate::value_transfer::AnalysisContextKey::for_module(&command_mutations, registry)
            .with_connection_scoped(connection_scoped_names(
                ir_module, cfg_module, registry, options,
            ))
            .with_deferred_writes(ir_module.deferred_writes.clone());
    let cfg_context = (cache_available
        || crate::unit_scope::needs_extra_call_site_scan_contexts(ir_module))
    .then_some(prepared_cfg_context);
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase("module-context");
    let (call_site_constants, linkage, extra_callers) =
        resolve_unit_scope(ir_module, cfg_module, cfg_context, options);
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase("unit-scope");
    let has_cross_file_evidence = options.external_call_sites.is_some();
    let module_wide = ModuleWideFacts::collect(source, ir_module, registry);
    let trace_facts = ModuleTraceFacts::of(ir_module);
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase("module-facts");
    RetainedModuleFacts {
        command_mutations,
        proc_binding_trust,
        analysis_context,
        cfg_context,
        call_site_constants,
        linkage,
        extra_callers,
        has_cross_file_evidence,
        tainted_global_writes,
        module_wide,
        trace_facts,
    }
}

struct RetainedBodyUnits {
    pub(super) methods: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    pub(super) source_body_cfgs: HashMap<crate::ir::ProcedureId, CfgFunction>,
    pub(super) body_units: HashMap<crate::ir::ProcedureId, FunctionUnit>,
}

fn build_retained_body_units(
    ir_module: &IrModule,
    facts: &RetainedModuleFacts<'_>,
    module_procedures: &crate::interprocedural::ModuleProcedures<'_>,
    options: UnitBuildOptions<'_>,
    cache: Option<&mut ProcLatticeCache<'_>>,
) -> RetainedBodyUnits {
    let extra_callers = &facts.extra_callers;
    let context = BodyUnitContext {
        registry: options.registry,
        known_class_set: &facts.module_wide.known_class_set,
        trace_facts: facts.trace_facts,
        command_trust: &facts.command_mutations,
        config: options.config,
        native_source: module_procedures
            .has_original_native_source()
            .then_some(module_procedures),
    };
    let memo_context = DeclaredBodyMemoContext {
        prepared: facts.cfg_context.map(|prepared| &prepared.context),
        dialect: options.dialect,
        known_classes: &facts.module_wide.known_classes,
        traced_variables: &facts.module_wide.traced_variable_names,
        analysis_context: &facts.analysis_context,
    };
    let methods = CompilationUnit::build_method_units(ir_module, extra_callers, context);
    let source_body_cfgs = ir_module
        .body_units
        .iter()
        .filter_map(|(id, _)| {
            extra_callers
                .iter()
                .find_map(|caller| caller.body_analysis_cfg(*id))
                .map(|cfg| (*id, cfg.clone()))
        })
        .collect();
    let body_units =
        CompilationUnit::build_body_units(ir_module, extra_callers, context, cache, memo_context);
    RetainedBodyUnits {
        methods,
        source_body_cfgs,
        body_units,
    }
}

#[derive(Clone, Copy)]
struct RetainedProcedureBuildInputs<'borrow, 'module, 'registry> {
    pub(super) ir_module: &'module IrModule,
    pub(super) cfg_module: &'module CfgModule,
    facts: &'borrow RetainedModuleFacts<'module>,
    module_procedures: &'borrow crate::interprocedural::ModuleProcedures<'module>,
    pub(super) registry: &'registry CommandRegistry,
    pub(super) dialect: Option<&'static tcl_dialect::DialectProfile>,
    pub(super) config: tcl_lexer::LexerConfig,
}

fn build_top_level_and_procedures(
    input: RetainedProcedureBuildInputs<'_, '_, '_>,
    cache: Option<&mut ProcLatticeCache<'_>>,
    #[cfg(any(test, debug_assertions))] phase_trace: &mut SourcePipelineTrace,
) -> (FunctionUnit, HashMap<crate::ir::ProcedureId, FunctionUnit>) {
    let RetainedProcedureBuildInputs {
        ir_module,
        cfg_module,
        facts,
        module_procedures,
        registry,
        dialect,
        config,
    } = input;
    let ModuleWideFacts {
        known_class_set,
        known_classes,
        top_level_extra_escaping,
        traced_variable_names,
        ..
    } = &facts.module_wide;
    let top_level = FunctionUnit::build_top_level(
        cfg_module.top_level.clone(),
        UnitDialect {
            registry,
            config,
            source_metadata_input: module_source_metadata_input(ir_module, registry),
        },
        known_class_set,
        top_level_extra_escaping,
        facts.trace_facts,
        &facts.command_mutations,
        Some(module_procedures),
    )
    .with_retained_semantic_analysis(
        registry,
        Some(&ir_module.top_level),
        crate::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
    );
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase("top-level-lattice");
    let built = build_procedure_units(
        &ProcedureBuildContext {
            ir_module,
            cfg_module,
            cfg_context: facts.cfg_context.map(|prepared| &prepared.context),
            registry,
            dialect,
            known_class_set,
            known_classes,
            traced_variable_names,
            trace_facts: facts.trace_facts,
            analysis_context: &facts.analysis_context,
            command_trust: &facts.command_mutations,
            tainted_global_writes: &facts.tainted_global_writes,
            procedures: Some(module_procedures),
        },
        cache,
        config,
    );
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase("procedure-lattices");
    (top_level, built)
}

struct RetainedUnitAssembly<'a> {
    pub(super) source: &'a str,
    pub(super) ir_module: IrModule,
    pub(super) cfg_module: CfgModule,
    pub(super) command_mutations: crate::command_binding::ModuleCommandMutations,
    pub(super) source_class_facts: Option<Arc<RetainedSourceClassFacts>>,
    pub(super) top_level: FunctionUnit,
    pub(super) procedures: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    pub(super) methods: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    pub(super) body_units: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    pub(super) source_body_cfgs: HashMap<crate::ir::ProcedureId, CfgFunction>,
    pub(super) connection_scope: Option<crate::connection_scope::ConnectionScope>,
    pub(super) caller_scope: UnitCallerScope,
    pub(super) declared_commands: tcl_registry::model::DeclaredSurface,
    pub(super) transfers: crate::interprocedural::TransferSummaries,
    pub(super) analysis_context: &'a crate::value_transfer::AnalysisContextKey,
    pub(super) options: UnitBuildOptions<'a>,
    original_source_ingress: optimiser_source_owner::OriginalSourceBuildIngress,
    lowering_integrity: std::cell::Cell<bool>,
}

fn assemble_retained_unit(input: RetainedUnitAssembly<'_>) -> CompilationUnit {
    let mut unit = CompilationUnit {
        optimiser_source_owner: None,
        source: input.source.to_owned(),
        ir_module: input.ir_module,
        cfg_module: input.cfg_module,
        command_mutations: input.command_mutations,
        source_class_facts: input.source_class_facts,
        top_level: input.top_level,
        procedures: input.procedures,
        methods: input.methods,
        body_units: input.body_units,
        carried_installed_lowerings: HashMap::new(),
        source_body_cfgs: input.source_body_cfgs,
        interproc: None,
        connection_scope: input.connection_scope,
        caller_scope: input.caller_scope,
        declared_commands: input.declared_commands,
        transfers: input.transfers,
    };
    crate::body_analysis_capsule::carry_readonly_authored_lowerings(
        &mut unit,
        input.options.registry,
    );
    if input.lowering_integrity.get() && unit.original_lattice_callbacks_are_current() {
        unit.retain_original_optimiser_source_origin(
            input.options,
            input.analysis_context,
            input.original_source_ingress,
        );
    }
    unit
}

struct RetainedAnalysisStage<'a> {
    facts: RetainedModuleFacts<'a>,
    pub(super) top_level: FunctionUnit,
    pub(super) procedures: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    pub(super) transfers: crate::interprocedural::TransferSummaries,
    pub(super) body_units: RetainedBodyUnits,
}

struct RetainedPostAnalysisInput<'a, 'b> {
    pub(super) ir_module: &'a IrModule,
    facts: RetainedModuleFacts<'a>,
    pub(super) top_level: FunctionUnit,
    pub(super) procedures: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    pub(super) transfers: crate::interprocedural::TransferSummaries,
    pub(super) options: UnitBuildOptions<'b>,
    pub(super) body_units: RetainedBodyUnits,
}

struct RetainedPostAnalysis {
    pub(super) command_mutations: crate::command_binding::ModuleCommandMutations,
    proc_binding_trust: crate::command_binding::ProcBindingTrustProjection,
    pub(super) analysis_context: crate::value_transfer::AnalysisContextKey,
    call_site_constants: crate::unit_scope::CallSiteEvidence,
    linkage: tcl_registry::Traits,
    has_cross_file_evidence: bool,
    _extra_callers: Vec<crate::unit_scope::ExtraCallSiteScanContext>,
    module_wide: ModuleWideFacts,
    _tainted_global_writes: HashMap<crate::ir::ProcedureId, HashSet<String>>,
    pub(super) top_level: FunctionUnit,
    pub(super) procedures: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    pub(super) methods: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    pub(super) body_units: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    pub(super) source_body_cfgs: HashMap<crate::ir::ProcedureId, CfgFunction>,
    pub(super) connection_scope: Option<crate::connection_scope::ConnectionScope>,
    pub(super) transfers: crate::interprocedural::TransferSummaries,
}

fn build_retained_post_analysis(
    input: RetainedPostAnalysisInput<'_, '_>,
    #[cfg(any(test, debug_assertions))] phase_trace: &mut SourcePipelineTrace,
) -> RetainedPostAnalysis {
    let RetainedPostAnalysisInput {
        ir_module,
        facts,
        top_level,
        procedures,
        transfers,
        options,
        body_units,
    } = input;
    let mut procedures = procedures;
    let RetainedModuleFacts {
        command_mutations,
        proc_binding_trust,
        analysis_context,
        call_site_constants,
        linkage,
        extra_callers,
        has_cross_file_evidence,
        module_wide,
        tainted_global_writes,
        ..
    } = facts;
    let RetainedBodyUnits {
        methods,
        source_body_cfgs,
        body_units,
    } = body_units;
    let connection_scope =
        CompilationUnit::build_connection_scope(&procedures, options.registry, ir_module);
    CompilationUnit::drop_cross_event_existence_folds(
        &mut procedures,
        connection_scope.as_ref(),
        options.registry,
    );
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase("connection-scope");
    RetainedPostAnalysis {
        command_mutations,
        proc_binding_trust,
        analysis_context,
        call_site_constants,
        linkage,
        has_cross_file_evidence,
        _extra_callers: extra_callers,
        module_wide,
        _tainted_global_writes: tainted_global_writes,
        top_level,
        procedures,
        methods,
        body_units,
        source_body_cfgs,
        connection_scope,
        transfers,
    }
}

struct RetainedAnalysisStageInput<'a, 'b, 'borrow, 'callback> {
    pub(super) source: &'a str,
    pub(super) options: UnitBuildOptions<'b>,
    pub(super) ir_module: &'a IrModule,
    pub(super) cfg_module: &'a CfgModule,
    pub(super) tainted_global_writes: HashMap<crate::ir::ProcedureId, HashSet<String>>,
    prepared_cfg_context: &'a PreparedCfgContext,
    cache: Option<&'borrow mut ProcLatticeCache<'callback>>,
}

fn build_retained_analysis_stage<'a>(
    input: RetainedAnalysisStageInput<'a, '_, '_, '_>,
    #[cfg(any(test, debug_assertions))] phase_trace: &mut SourcePipelineTrace,
) -> RetainedAnalysisStage<'a> {
    let RetainedAnalysisStageInput {
        source,
        options,
        ir_module,
        cfg_module,
        tainted_global_writes,
        prepared_cfg_context,
        mut cache,
    } = input;
    let facts = retained_module_facts(
        RetainedModuleFactsInput {
            source,
            options,
            ir_module,
            cfg_module,
            tainted_global_writes,
            prepared_cfg_context,
            cache_available: cache.is_some(),
        },
        #[cfg(any(test, debug_assertions))]
        phase_trace,
    );
    let module_procedures = module_procedures(
        (ir_module, cfg_module, prepared_cfg_context),
        (&facts.command_mutations, &facts.proc_binding_trust),
        &facts.analysis_context,
        (options.registry, options.config),
    );
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase("procedure-transfers");
    let (top_level, built) = build_top_level_and_procedures(
        RetainedProcedureBuildInputs {
            ir_module,
            cfg_module,
            facts: &facts,
            module_procedures: &module_procedures,
            registry: options.registry,
            dialect: options.dialect,
            config: options.config,
        },
        cache.as_deref_mut(),
        #[cfg(any(test, debug_assertions))]
        phase_trace,
    );
    let procedures = built;
    let body_units =
        build_retained_body_units(ir_module, &facts, &module_procedures, options, cache);
    #[cfg(any(test, debug_assertions))]
    phase_trace.phase("method-and-body-lattices");
    let transfers = module_procedures.into_summaries();
    RetainedAnalysisStage {
        facts,
        top_level,
        procedures,
        transfers,
        body_units,
    }
}

impl CompilationUnit {
    pub(super) fn build_with_retained_entry_and_source_syntax(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: Option<&mut ProcLatticeCache<'_>>,
        body_caches: OriginalBodyCacheInputs<'_>,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
        (input, source_body_cache, standalone, source_settings): SourceSyntaxInputs<
            '_,
            '_,
            '_,
            '_,
            '_,
        >,
    ) -> Self {
        // Optional callback/cache outputs need their own authentic analysis
        // receipts before this source-action producer can reborrow them.
        // Report construction and all existing analysis remain unchanged.
        let original_source_ingress = if standalone.is_some() {
            optimiser_source_owner::OriginalSourceBuildIngress::ExplicitStandalone
        } else if entry.is_none() && context.is_none() && input.is_none() {
            optimiser_source_owner::OriginalSourceBuildIngress::DefaultAuthoring
        } else if entry.is_none() {
            optimiser_source_owner::OriginalSourceBuildIngress::SuppliedDocument
        } else {
            optimiser_source_owner::OriginalSourceBuildIngress::RetainedEntry
        };

        #[cfg(any(test, debug_assertions))]
        let mut phase_trace = SourcePipelineTrace::new("unit", source.len());
        #[cfg(any(test, debug_assertions))]
        phase_trace.phase("initial");
        let lowered = lower_retained_source(
            RetainedLoweringInput {
                source,
                options,
                body_caches,
                entry,
                context,
                input,
                source_body_cache,
                standalone,
                source_settings,
            },
            #[cfg(any(test, debug_assertions))]
            &mut phase_trace,
        );
        let RetainedLoweredSource {
            ir_module,
            cfg_module,
            tainted_global_writes,
            prepared_cfg_context,
            integrity,
        } = lowered;
        let RetainedAnalysisStage {
            facts,
            top_level,
            procedures,
            transfers,
            body_units,
        } = build_retained_analysis_stage(
            RetainedAnalysisStageInput {
                source,
                options,
                ir_module: &ir_module,
                cfg_module: &cfg_module,
                tainted_global_writes,
                prepared_cfg_context: &prepared_cfg_context,
                cache,
            },
            #[cfg(any(test, debug_assertions))]
            &mut phase_trace,
        );
        let post = build_retained_post_analysis(
            RetainedPostAnalysisInput {
                ir_module: &ir_module,
                facts,
                top_level,
                procedures,
                transfers,
                options,
                body_units,
            },
            #[cfg(any(test, debug_assertions))]
            &mut phase_trace,
        );
        assemble_retained_unit(RetainedUnitAssembly {
            source,
            ir_module,
            cfg_module,
            command_mutations: post.command_mutations,
            source_class_facts: post.module_wide.source_class_facts,
            top_level: post.top_level,
            procedures: post.procedures,
            methods: post.methods,
            body_units: post.body_units,
            source_body_cfgs: post.source_body_cfgs,
            connection_scope: post.connection_scope,
            caller_scope: UnitCallerScope {
                linkage: post.linkage,
                has_cross_file_evidence: post.has_cross_file_evidence,
                call_sites: post.call_site_constants,
                proc_binding_trust: post.proc_binding_trust,
            },
            declared_commands: options.declared_commands.cloned().unwrap_or_default(),
            transfers: post.transfers,
            analysis_context: &post.analysis_context,
            options,
            original_source_ingress,
            lowering_integrity: integrity,
        })
    }
}
