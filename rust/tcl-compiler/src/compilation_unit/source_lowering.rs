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

//! Lowering the source to IR and CFGs, with the original-body caches, and resolving the unit's caller scope.

use super::{OriginalLoweringCaches, SourcePipelineTrace, UnitBuildOptions};
use crate::cfg::CfgModule;
use crate::cfg_builder::{
    PreparedCfgContext, build_cfg_with_registry_and_context, prepare_cfg_context_bundle,
};
#[cfg(test)]
use crate::compilation_unit::BodyLoweringCache;
use crate::ir::Module as IrModule;
use crate::unit_scope::{build_extra_call_site_scan_contexts, collect_call_site_constants};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// Lower `source` to IR, run the module-level pre-CFG passes, and build the
/// module CFG — the front half of a [`CompilationUnit`] build, shared by every
/// entry point.
#[cfg(test)]
pub(super) fn lower_and_build_cfg(
    source: &str,
    options: UnitBuildOptions<'_>,
    caches: (
        Option<&BodyLoweringCache<'_>>,
        Option<&crate::lowering::SourceBodySyntaxCache<'_>>,
    ),
    source_entry: Option<&crate::command_binding::SourceAnalysisEntry>,
    context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
    input: Option<&crate::analyser::ResolvedAnalysisInput>,
    standalone: Option<crate::lowering::StandaloneSourceInput<'_>>,
) -> (
    IrModule,
    CfgModule,
    HashMap<crate::ir::ProcedureId, HashSet<String>>,
    PreparedCfgContext,
) {
    let (body_cache, source_body_cache) = caches;
    let integrity = std::cell::Cell::new(true);
    lower_and_build_cfg_with_original_caches(
        source,
        options,
        OriginalLoweringCaches {
            legacy: body_cache,
            source: source_body_cache,
            original: None,
            integrity: &integrity,
        },
        source_entry,
        context,
        input,
        (standalone, None),
    )
}

pub(super) type SourceSyntaxInputs<'a, 'b, 'c, 'd, 'e> = (
    Option<&'a crate::analyser::ResolvedAnalysisInput>,
    Option<&'b crate::lowering::SourceBodySyntaxCache<'c>>,
    Option<crate::lowering::StandaloneSourceInput<'d>>,
    Option<&'e Arc<crate::analyser::OriginalAnalyserSourceSettings>>,
);

pub(super) fn lower_and_build_cfg_with_original_caches(
    source: &str,
    options: UnitBuildOptions<'_>,
    caches: OriginalLoweringCaches<'_>,
    source_entry: Option<&crate::command_binding::SourceAnalysisEntry>,
    context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
    input: Option<&crate::analyser::ResolvedAnalysisInput>,
    (standalone, source_settings): (
        Option<crate::lowering::StandaloneSourceInput<'_>>,
        Option<&Arc<crate::analyser::OriginalAnalyserSourceSettings>>,
    ),
) -> (
    IrModule,
    CfgModule,
    HashMap<crate::ir::ProcedureId, HashSet<String>>,
    PreparedCfgContext,
) {
    let registry = options.registry;
    // One lowerer shape for both paths, so the document's own declarations
    // (`UnitBuildOptions::declared_commands`) reach the memoised body-cache
    // build and the plain one identically.
    #[cfg(any(test, debug_assertions))]
    let mut lower_trace = SourcePipelineTrace::new("lower-cfg", source.len());
    #[cfg(any(test, debug_assertions))]
    lower_trace.phase("initial");
    let mut lowerer = if let Some(owner) = standalone {
        crate::lowering::Lowerer::for_standalone_source_with_input(owner)
    } else if source_entry.is_none() && context.is_none() && input.is_none() {
        // The default build API selects authoring. Explicit supplied or driver
        // ingress keeps its own purpose even when source metadata is missing.
        crate::lowering::Lowerer::for_standalone_source(registry, options.config, options.dialect)
    } else {
        crate::lowering::Lowerer::with_config(registry, options.config)
            .with_dialect(options.dialect)
    }
    .with_declared_commands(options.declared_commands);
    let context_input = context
        .as_ref()
        .zip(options.dialect)
        .map(|(context, profile)| {
            crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                Arc::clone(context),
                options.config,
            )
        });
    let input = input.or(context_input.as_ref());
    if let Some(input) = input.filter(|_| standalone.is_none()) {
        lowerer = lowerer.with_resolved_analysis_input(input.clone());
    } else if let Some(context) = context {
        lowerer = lowerer.with_context_registry(context);
    }
    let configured_entry = source_entry.zip(source_settings).map(|(entry, settings)| {
        let mut entry = entry.clone();
        entry.analyser_source_settings = Some(Arc::clone(settings));
        entry
    });
    if let Some(entry) = configured_entry.as_ref().or(source_entry) {
        lowerer.set_source_analysis_options(entry.options());
    } else if let Some(input) = input.filter(|_| standalone.is_none()) {
        lowerer.set_supplied_source_analysis_input(input, source_settings);
    }
    lowerer = lowerer.with_original_optimiser_lowering(caches.original, caches.integrity);
    if let Some(cache) = caches.source {
        lowerer = lowerer.with_source_body_syntax_cache(cache);
    }
    let mut ir_module = match caches.legacy {
        Some(bc) => crate::lowering::lower_to_ir_with(lowerer.with_body_cache(bc), source),
        None => crate::lowering::lower_to_ir_with(lowerer, source),
    };
    // Specialise Option-shape factories before any other
    // module-level passes so the synthesised child procs
    // appear in module.procedures for the inline_uplevel pass
    // and CFG construction.
    #[cfg(any(test, debug_assertions))]
    {
        lower_trace.procedures = ir_module.procedures.len();
        lower_trace.phase("lowered");
    }
    crate::specialise_factories::specialise_factories(&mut ir_module, registry);
    // Run the inline_uplevel pass before CFG construction so
    // every passthrough callsite is replaced with a Statement::Block
    // that splices the body inline.
    #[cfg(any(test, debug_assertions))]
    lower_trace.phase("factories");
    crate::inline_uplevel::inline_uplevel_passthrough(&mut ir_module, registry);
    #[cfg(any(test, debug_assertions))]
    lower_trace.phase("inline-uplevel");
    let cfg_context = prepare_cfg_context_bundle(&ir_module, registry);
    #[cfg(any(test, debug_assertions))]
    lower_trace.phase("prepared");
    let mut cfg_module = build_cfg_with_registry_and_context(
        &ir_module,
        options.defer_top_level,
        registry,
        &cfg_context,
        options.config,
    );
    #[cfg(any(test, debug_assertions))]
    lower_trace.phase("cfg-built");
    let tainted_global_writes =
        crate::interprocedural::enrich_instance_taint_cfg(&ir_module, &mut cfg_module, registry);
    #[cfg(any(test, debug_assertions))]
    lower_trace.phase("enriched");
    (ir_module, cfg_module, tainted_global_writes, cfg_context)
}

pub(super) fn module_source_metadata_input<'a>(
    module: &'a IrModule,
    registry: &CommandRegistry,
) -> Option<&'a crate::analyser::ResolvedAnalysisInput> {
    crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?
        .source_analysis_input()
}

/// Resolve everything the interprocedural constant seed needs to know about
/// *who can call this unit's procedures* (see [`crate::unit_scope`]):
/// the merged call-site evidence, the registry-declared unit boundaries the
/// file crosses, and the extra caller CFGs reused by later method/body builds.
pub(super) fn resolve_unit_scope(
    ir_module: &IrModule,
    cfg_module: &CfgModule,
    cfg_context: Option<&PreparedCfgContext>,
    options: UnitBuildOptions<'_>,
) -> (
    crate::unit_scope::CallSiteEvidence,
    tcl_registry::Traits,
    Vec<crate::unit_scope::ExtraCallSiteScanContext>,
) {
    let registry = options.registry;
    // Bare CFGs for every TclOO method and synthetic body unit (`apply`
    // lambda, `namespace eval` body) — the call-site scan must walk these as
    // *callers* too, even though neither is itself seeded with
    // `param_constants` (`build_method_units`/`build_body_units` always pass
    // `None`): a call from inside one of these bodies to an ordinary user proc
    // is a real call site whose argument can vary between call sites, exactly
    // like a bare top-level/proc-body call.
    let extra_callers =
        build_extra_call_site_scan_contexts(ir_module, cfg_context, registry, options.config);
    // Collect call-site literal arg values per user proc so each callee's SCCP
    // can fold a param every caller passes the same literal for
    // (interprocedural constant propagation).
    let mut call_sites = if let Some(identities) =
        crate::realm::document_realm_bindings_for_module(ir_module, registry)
    {
        collect_call_site_constants(
            cfg_module,
            &extra_callers,
            &ir_module.procedures,
            &ir_module.future_call_sites,
            crate::unit_scope::CallSiteSourceContext {
                registry,
                module: ir_module,
                declared: options.declared_commands,
                input: module_source_metadata_input(ir_module, registry),
                identities: &identities,
            },
        )
    } else {
        // A missing retained interpretation cannot become an unchanged-name
        // realm or fresh source replay. Every possible local caller is opaque.
        let mut evidence = crate::unit_scope::CallSiteEvidence::default();
        let reachable = ir_module
            .procedures
            .values()
            .map(|procedure| procedure.qualified_name.clone())
            .collect::<Vec<_>>();
        evidence.record_unenumerable_caller(&reachable);
        evidence
    };
    // Fold in the call sites a host with a cross-file view supplied — callers
    // in *other* files, which this single-source unit can never see for itself
    // Merging is monotone: extra evidence can retract a fold,
    // never manufacture one.
    if let Some(external) = options.external_call_sites {
        call_sites.merge_from(external);
    }
    // Which registry-declared unit boundaries this file crosses, derived from
    // the registry rather than a hardcoded `package provide` check.
    let linkage = crate::unit_scope::scan_unit_linkage(ir_module, registry, options.dialect);
    (call_sites, linkage, extra_callers)
}

/// Derive both whole-module command-trust views from the command lattice that
/// CFG preparation already closed. This is the single compilation-unit seam;
/// consumers never rebuild or reinterpret the binding analysis independently.
pub(super) fn prepared_command_trust(
    ir_module: &IrModule,
    registry: &CommandRegistry,
    prepared: &PreparedCfgContext,
) -> (
    crate::command_binding::ModuleCommandMutations,
    crate::command_binding::ProcBindingTrustProjection,
) {
    let bindings = &prepared.context.3;
    (
        crate::command_binding::scan_module_command_mutations_with_bindings(
            ir_module, registry, bindings,
        ),
        bindings.proc_binding_trust_projection(),
    )
}
