// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original source-build inputs and exact prepared analysis states.

use super::{CompilationUnit, UnitBuildOptions};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// Original complete source-build request, issued by the actual source pipeline.
/// Equality compares independently retained input values, never report names or
/// allocation IDs; it supplies no runtime, absence or edit permission by itself.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalOptimiserSourceOrigin(Arc<SourceBuildInputs>);

#[derive(Debug, Clone, PartialEq)]
struct SourceBuildInputs {
    ingress: OriginalSourceBuildIngress,
    image: tcl_lexer::SourceImage,
    input: crate::analyser::ResolvedAnalysisInput,
    entry: crate::command_binding::SourceAnalysisEntry,
    registry: tcl_registry::RegistrySnapshot,
    analysis_context: crate::value_transfer::AnalysisContextKey,
    defer_top_level: bool,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    external_call_sites: Option<crate::unit_scope::CallSiteEvidence>,
    declared_commands: Option<tcl_registry::model::DeclaredSurface>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum OriginalSourceBuildIngress {
    DefaultAuthoring,
    ExplicitStandalone,
    RetainedEntry,
}

#[derive(Debug, Clone)]
pub(crate) struct OriginalOptimiserPreparedUnit(Arc<PreparedState>);
#[derive(Debug)]
struct PreparedState {
    // This owned clone has no producer receipt, preventing recursive state.
    unit: CompilationUnit,
    origin: OriginalOptimiserSourceOrigin,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    parent: Option<OriginalOptimiserPreparedUnit>,
    top_level_projection: bool,
}

impl OriginalOptimiserPreparedUnit {
    pub(super) fn for_source_unit(
        unit: &CompilationUnit,
        options: UnitBuildOptions<'_>,
        analysis_context: &crate::value_transfer::AnalysisContextKey,
        ingress: OriginalSourceBuildIngress,
    ) -> Option<Self> {
        let input = unit.ir_module.source_metadata_input.as_ref()?;
        if unit.source != unit.ir_module.source.try_text().ok()?
            || unit.ir_module.source != tcl_lexer::SourceImage::document(&unit.source)
            || options
                .dialect
                .is_some_and(|profile| profile != input.unit_profile())
            || options.config != input.lexer_config()
            || unit.ir_module.lexer_config != options.config
            || unit.ir_module.registry_snapshot.as_ref()?.semantic_key()
                != options.registry.snapshot().semantic_key()
            || input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
                != options.registry.snapshot().semantic_key()
        {
            return None;
        }
        let origin = OriginalOptimiserSourceOrigin(Arc::new(SourceBuildInputs {
            ingress,
            image: unit.ir_module.source.clone(),
            input: input.clone(),
            entry: unit.ir_module.source_entry.clone(),
            registry: options.registry.snapshot(),
            analysis_context: analysis_context.clone(),
            defer_top_level: options.defer_top_level,
            dialect: options.dialect,
            external_call_sites: options.external_call_sites.cloned(),
            declared_commands: options.declared_commands.cloned(),
        }));
        Some(Self::snapshot(unit, origin, options.dialect, None, false))
    }
    fn snapshot(
        unit: &CompilationUnit,
        origin: OriginalOptimiserSourceOrigin,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
        parent: Option<Self>,
        top_level_projection: bool,
    ) -> Self {
        let mut current = unit.clone();
        current.optimiser_source_owner = None;
        Self(Arc::new(PreparedState {
            unit: current,
            origin,
            dialect,
            parent,
            top_level_projection,
        }))
    }
    pub(super) fn matches_unit(&self, unit: &CompilationUnit) -> bool {
        self.0.unit == *unit
            && self.0.unit.top_level.has_original_semantic_value_owner()
                == unit.top_level.has_original_semantic_value_owner()
            && self.0.unit.all_body_function_units().all(|original| {
                unit.function_for_body(original.body_id)
                    .is_some_and(|current| {
                        original.has_original_semantic_value_owner()
                            == current.has_original_semantic_value_owner()
                    })
            })
    }
    pub(super) fn matches_dialect(
        &self,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> bool {
        self.0.dialect == dialect
    }
    pub(super) fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.0.origin.0.registry.semantic_key() == registry.snapshot().semantic_key()
    }
    pub(crate) fn matches_worker(
        &self,
        unit: &CompilationUnit,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> bool {
        self.matches_unit(unit)
            && self.matches_dialect(dialect)
            && self.0.origin.0.registry.semantic_key() == registry.snapshot().semantic_key()
            && self.0.origin.is_current_analysis_context()
    }
    pub(super) fn after_internal_enrichment(&self, unit: &CompilationUnit) -> Self {
        Self::snapshot(
            unit,
            self.0.origin.clone(),
            self.0.dialect,
            self.0.parent.clone(),
            self.0.top_level_projection,
        )
    }
    pub(crate) fn origin(&self) -> &OriginalOptimiserSourceOrigin {
        &self.0.origin
    }
    pub(crate) fn unit(&self) -> &CompilationUnit {
        &self.0.unit
    }
    pub(crate) fn same_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn is_whole_source_unit(&self) -> bool {
        self.0.parent.is_none() && !self.0.top_level_projection
    }
    pub(crate) fn is_top_level_projection_of(&self, original: &Self) -> bool {
        self.0.top_level_projection
            && self
                .0
                .parent
                .as_ref()
                .is_some_and(|parent| parent.same_allocation(original))
    }
    pub(crate) fn top_level_view(&self) -> CompilationUnit {
        let mut selected = self.0.unit.clone();
        selected.procedures.clear();
        selected.methods.clear();
        selected.body_units.clear();
        selected.carried_installed_lowerings.clear();
        selected.cfg_module.procedures.clear();
        selected.optimiser_source_owner = Some(Self::snapshot(
            &selected,
            self.0.origin.clone(),
            self.0.dialect,
            Some(self.clone()),
            true,
        ));
        selected
    }
}

impl CompilationUnit {
    pub(super) fn retain_original_optimiser_source_origin(
        &mut self,
        options: UnitBuildOptions<'_>,
        analysis_context: &crate::value_transfer::AnalysisContextKey,
        ingress: OriginalSourceBuildIngress,
    ) {
        self.optimiser_source_owner = OriginalOptimiserPreparedUnit::for_source_unit(
            self,
            options,
            analysis_context,
            ingress,
        );
    }
    pub(super) fn retain_enriched_original_optimiser_owner(
        &mut self,
        original: &OriginalOptimiserPreparedUnit,
    ) {
        self.optimiser_source_owner = Some(original.after_internal_enrichment(self));
    }
    pub(crate) fn original_optimiser_prepared_unit(
        &self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Option<OriginalOptimiserPreparedUnit> {
        self.optimiser_source_owner
            .as_ref()
            .filter(|owner| owner.matches_worker(self, registry, dialect))
            .cloned()
    }
    /// Retain this exact whole production snapshot for report-only owners.
    /// This neither rebuilds a unit nor changes runtime or host eligibility.
    pub(crate) fn original_full_report_prepared_unit(
        &self,
        registry: &CommandRegistry,
    ) -> Option<OriginalOptimiserPreparedUnit> {
        self.optimiser_source_owner
            .as_ref()
            .filter(|owner| {
                owner.is_whole_source_unit()
                    && owner.matches_unit(self)
                    && owner.0.origin.0.registry == registry.snapshot()
                    && owner.origin().is_current_analysis_context()
            })
            .cloned()
    }

    pub(crate) fn original_optimiser_source_origin(
        &self,
    ) -> Option<&OriginalOptimiserSourceOrigin> {
        self.optimiser_source_owner
            .as_ref()
            .filter(|owner| owner.matches_unit(self))
            .map(OriginalOptimiserPreparedUnit::origin)
    }
    /// Genuine top-level-only analysis view for the original optimiser assembler.
    /// Its parent receipt retains the whole current unit; public clone mutation
    /// cannot issue this view, and no new source interpretation is built.
    #[must_use]
    pub fn original_optimiser_top_level_view(
        &self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Option<Self> {
        Some(
            self.original_optimiser_prepared_unit(registry, dialect)?
                .top_level_view(),
        )
    }
}

/// Actual private interprocedural summary request. The retained unit was derived
/// by the existing summary worker before per-body taint callbacks; no caller map
/// or manually assigned `interproc` field can create this request.
#[derive(Debug, Clone)]
pub struct OriginalInterproceduralTaintRequest {
    unit: Arc<CompilationUnit>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
}
impl OriginalInterproceduralTaintRequest {
    pub(super) fn from_computed_summary(
        current: &CompilationUnit,
        summary: &crate::interprocedural::InterproceduralAnalysis,
        owner: Option<&OriginalOptimiserPreparedUnit>,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        let mut unit = current.clone();
        unit.interproc = Some(summary.clone());
        unit.optimiser_source_owner = owner.map(|owner| owner.after_internal_enrichment(&unit));
        Self {
            unit: Arc::new(unit),
            dialect,
        }
    }
    /// Reborrow only an actual source body of this computed summary request.
    #[must_use]
    pub fn body_capsule(
        &self,
        body: crate::ir::ProcedureId,
        registry: &CommandRegistry,
    ) -> Option<crate::body_analysis_capsule::BodyAnalysisCapsule> {
        crate::body_analysis_capsule::BodyAnalysisCapsule::retain(
            Arc::clone(&self.unit),
            body,
            registry,
        )
    }
    pub(super) fn accepts(
        &self,
        body: crate::ir::ProcedureId,
        registry: &CommandRegistry,
        report: &crate::body_analysis_capsule::OriginalInterproceduralTaintReport,
    ) -> bool {
        self.body_capsule(body, registry)
            .is_some_and(|capsule| report.matches_current(&capsule, registry, self.dialect))
    }
}

impl OriginalOptimiserSourceOrigin {
    /// Same retained source-build issuer, separate from value equality of the
    /// immutable request and from the rebuilt CU's own token attachment.
    pub(crate) fn same_request_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn analysis_context(&self) -> &crate::value_transfer::AnalysisContextKey {
        &self.0.analysis_context
    }
    pub(crate) fn is_current_analysis_context(&self) -> bool {
        self.0.analysis_context.evaluator_revision
            == u64::from(tcl_registry::pack_hooks::evaluator_generation().0)
    }
    pub(crate) fn source_analysis_entry_for_current_analysis(
        &self,
        source: &str,
        registry: &CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        external: Option<&crate::unit_scope::CallSiteEvidence>,
    ) -> Option<Arc<crate::command_binding::SourceAnalysisEntry>> {
        self.matches_source_request(source, registry, input, external)
            .then(|| Arc::new(self.0.entry.clone()))
    }
    fn matches_source_request(
        &self,
        source: &str,
        registry: &CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        external: Option<&crate::unit_scope::CallSiteEvidence>,
    ) -> bool {
        self.0.image == tcl_lexer::SourceImage::document(source)
            && &self.0.input == input
            && self.0.registry == registry.snapshot()
            && self.0.external_call_sites.as_ref() == external
            && self.is_current_analysis_context()
    }
    pub(crate) fn rebuild_for_current_analysis(
        &self,
        source: &str,
        registry: &CommandRegistry,
        input: &crate::analyser::ResolvedAnalysisInput,
        external: Option<&crate::unit_scope::CallSiteEvidence>,
    ) -> Option<CompilationUnit> {
        if !self.matches_source_request(source, registry, input, external) {
            return None;
        }
        // Reuse the same private source pipeline once, with the exact original
        // retained declaration/sidecar and entry inputs. No pathless surface or
        // fresh evidence is substituted for the current analysis's producer.
        let options = UnitBuildOptions {
            registry,
            defer_top_level: self.0.defer_top_level,
            config: self.0.input.lexer_config(),
            dialect: self.0.dialect,
            external_call_sites: self.0.external_call_sites.as_ref(),
            declared_commands: self.0.declared_commands.as_ref(),
        };
        let rebuilt = match self.0.ingress {
            OriginalSourceBuildIngress::ExplicitStandalone => {
                CompilationUnit::build_standalone_with_analysis_input(
                    source,
                    options,
                    &self.0.input,
                )?
            }
            OriginalSourceBuildIngress::DefaultAuthoring => {
                CompilationUnit::build_with_retained_entry_and_source_syntax(
                    source,
                    options,
                    None,
                    super::OriginalBodyCacheInputs {
                        legacy: None,
                        original: None,
                    },
                    None,
                    None,
                    None,
                    None,
                    None,
                )
            }
            OriginalSourceBuildIngress::RetainedEntry => {
                CompilationUnit::build_with_analysis_input(
                    source,
                    options,
                    Some(&self.0.entry),
                    &self.0.input,
                )
            }
        };
        (rebuilt.original_optimiser_source_origin() == Some(self)).then_some(rebuilt)
    }
}
