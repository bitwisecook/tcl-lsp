// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Retained compilation owners for per-body analysis and source edits.

use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{Procedure, ProcedureId};
use crate::optimiser::Optimisation;
use tcl_registry::CommandRegistry;

/// One actual body and its complete immutable compilation inputs.
#[derive(Debug, Clone)]
pub struct BodyAnalysisCapsule {
    unit: Arc<CompilationUnit>,
    body: ProcedureId,
    neutral: Option<Arc<ClosedBodyProjection>>,
}

#[derive(Debug, Clone, PartialEq)]
struct ClosedBodyProjection {
    declaration: NeutralDeclaration,
    body: crate::ir::Script,
    function: FunctionUnit,
    summary: Option<crate::interprocedural::ProcSummary>,
    instance_classes: Option<crate::taint::InstanceClassState>,
    tainted_global_writes: Option<std::collections::HashSet<String>>,
    analytical: Option<crate::command_binding::AnalyticalBodyCorrespondence>,
    module_facts: BodyModuleFacts,
    original_semantic_facts: crate::semantic_analysis::SemanticAnalysisBundle,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NeutralDeclaration {
    publication: crate::signature_scan::scope::SignatureSourceCommand,
    namespace: crate::command_binding::SourceNamespaceKey,
    written_name: String,
    display: String,
    parameters: Vec<String>,
    parameters_text: String,
    body_text: String,
    kind: crate::ir::ProcedureBodyKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BodyModuleFacts {
    mutations: crate::command_binding::ModuleCommandMutations,
    redefined: std::collections::HashSet<String>,
    namespace_imports: Vec<(String, String)>,
    namespace_exports: Vec<(String, String)>,
    traced_commands: std::collections::BTreeSet<String>,
    dynamic_trace: bool,
    traced_variables: std::collections::BTreeSet<String>,
    dynamic_variable_trace: bool,
    deferred_writes: crate::ir::DeferredWrites,
    declared_frame_effects: crate::ir::DeclaredFrameEffects,
    oo_evidence: crate::ir::OoDefinitionEvidence,
}

impl BodyModuleFacts {
    fn retain(unit: &CompilationUnit) -> Self {
        let module = &unit.ir_module;
        Self {
            mutations: unit.command_mutations.clone(),
            redefined: module.redefined_procedures.clone(),
            namespace_imports: module.namespace_imports.clone(),
            namespace_exports: module.namespace_exports.clone(),
            traced_commands: module.traced_commands.clone(),
            dynamic_trace: module.has_dynamic_trace,
            traced_variables: module.traced_variables.clone(),
            dynamic_variable_trace: module.has_dynamic_variable_trace,
            deferred_writes: module.deferred_writes.clone(),
            declared_frame_effects: module.declared_frame_effects.clone(),
            oo_evidence: module.oo_evidence,
        }
    }
}

impl BodyAnalysisCapsule {
    /// Retain a source-corresponding body without deriving an identity from its name.
    #[must_use]
    pub fn retain(
        unit: Arc<CompilationUnit>,
        body: ProcedureId,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let module = &unit.ir_module;
        if !module.procedure_ids.owns(body) {
            return None;
        }
        let procedure = module.procedures.get(&body)?;
        let function = unit.procedures.get(&body)?;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.positioned_for_module(module, registry)?;
        if function.body_id != Some(body)
            || unit.cfg_module.procedures.get(&body) != Some(&function.cfg)
            || module.source.try_text().ok()? != unit.source
            || procedure.declaration_site.is_none()
            || !retained.matches_original_procedure(procedure)
        {
            return None;
        }
        let neutral = native_closed_projection(&unit, body, registry)
            .or_else(|| analytical_closed_projection(&unit, body, registry))
            .map(Arc::new);
        Some(Self {
            unit,
            body,
            neutral,
        })
    }

    /// The complete retained compilation unit, including original world and input owners.
    #[must_use]
    pub fn unit(&self) -> &CompilationUnit {
        &self.unit
    }

    /// The Module-issued body coordinate retained by this capsule.
    #[must_use]
    pub const fn body_id(&self) -> ProcedureId {
        self.body
    }

    /// Whether original Native closure and injective source relocation certify reuse.
    /// The certificate supplies analysis data equivalence, never Native execution.
    #[must_use]
    pub fn has_closed_native_correspondence(&self) -> bool {
        match &self.neutral {
            Some(projection) => projection.analytical.is_none(),
            None => false,
        }
    }

    /// Original closed registry-only lookups exclude document-summary dependencies.
    #[must_use]
    pub const fn has_closed_analysis_correspondence(&self) -> bool {
        self.neutral.is_some()
    }

    /// The exact retained source declaration.
    #[must_use]
    pub fn procedure(&self) -> &Procedure {
        self.unit
            .ir_module
            .procedures
            .get(&self.body)
            .expect("retained capsule declaration")
    }

    /// The actual analysis belonging to the retained declaration.
    #[must_use]
    pub fn function(&self) -> &FunctionUnit {
        self.unit
            .procedures
            .get(&self.body)
            .expect("retained capsule function")
    }

    /// Select body-local source edits from the genuine whole-unit analysis.
    /// A group crossing the body's source boundary is retained by neither memo.
    #[must_use]
    pub fn original_body_optimisations(
        &self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Vec<Optimisation> {
        let span = self.procedure().span;
        let raw = crate::optimiser::optimise_unit_raw(self.unit(), registry, dialect);
        let outside_groups: std::collections::HashSet<_> = raw
            .iter()
            .filter(|edit| edit.span.start() < span.start() || span.end() < edit.span.end())
            .filter_map(|edit| edit.group)
            .collect();
        raw.into_iter()
            .filter(|edit| {
                span.start() <= edit.span.start()
                    && edit.span.end() <= span.end()
                    && edit
                        .group
                        .is_none_or(|group| !outside_groups.contains(&group))
            })
            .map(|mut edit| {
                edit.span = tcl_lexer::Span::new(
                    edit.span.start() - span.start(),
                    edit.span.end() - span.start(),
                );
                edit
            })
            .collect()
    }
}

impl PartialEq for BodyAnalysisCapsule {
    fn eq(&self, other: &Self) -> bool {
        match (&self.neutral, &other.neutral) {
            (Some(left), Some(right)) => {
                left.declaration == right.declaration
                    && left.body == right.body
                    && ((self.body == other.body && Arc::ptr_eq(&self.unit, &other.unit))
                        || left == right)
            }
            (None, None) => self.body == other.body && Arc::ptr_eq(&self.unit, &other.unit),
            _ => false,
        }
    }
}

impl Eq for BodyAnalysisCapsule {}

impl Hash for BodyAnalysisCapsule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(projection) = &self.neutral {
            projection.declaration.hash(state);
            projection.body.hash(state);
        } else {
            self.body.hash(state);
            Arc::as_ptr(&self.unit).hash(state);
        }
    }
}

fn native_closed_projection(
    unit: &CompilationUnit,
    body_id: ProcedureId,
    registry: &CommandRegistry,
) -> Option<ClosedBodyProjection> {
    let module = &unit.ir_module;
    let procedure = module.procedures.get(&body_id)?;
    let publication = procedure.publication.as_ref()?;
    if module
        .procedures
        .values()
        .filter(|candidate| candidate.publication.as_ref() == Some(publication))
        .count()
        != 1
    {
        return None;
    }
    let declaration = NeutralDeclaration {
        publication: publication.clone(),
        namespace: procedure.body.namespace_context.as_deref()?.clone(),
        written_name: procedure.name.clone(),
        display: procedure.qualified_name.clone(),
        parameters: procedure.params.clone(),
        parameters_text: procedure.params_raw.clone(),
        body_text: procedure.body_source.clone()?,
        kind: procedure.body_kind,
    };
    let mut original = procedure.body.clone();
    crate::lattice_rebase::rebase_script(&mut original, -i64::from(procedure.span.start()));
    let bindings = crate::command_binding::ModuleCommandBindings::analyse(module, registry);
    let function = unit.procedures.get(&body_id)?;
    let scope = crate::command_binding::BodyProofScope {
        source: &unit.source,
        body_source: &declaration.body_text,
        original_body_offset: procedure.span.start(),
        executable_body_offset: procedure.body_offset,
        metadata: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
            function.source_metadata_input(),
        ),
        config: function.source_lexer_config(),
    };
    let scope = if module.source_entry.metadata_context.is_standalone() {
        crate::command_binding::BodyProofScope {
            metadata: crate::registry_invocation::InvocationMetadataInput::Standalone,
            ..scope
        }
    } else {
        scope
    };
    let template = bindings.prepare_native_body_template(&original, scope, registry)?;
    let mut normalized = function.clone();
    crate::lattice_rebase::rebase_function_unit(
        &mut normalized,
        -i64::from(procedure.span.start()),
    );
    let mut normalized = normalized
        .relocated_variable_proofs(&template.variable_relocation)?
        .restored_source_proofs(&crate::command_binding::BodySourceProofs::from_body(
            &template.body,
        ))?;
    // The checked original declaration and injective body relocation own this
    // comparison. The private projection is never used as an executable unit.
    normalized.body_id = None;
    let mut summary = unit
        .interproc
        .as_ref()
        .and_then(|analysis| analysis.procedures.get(&body_id))
        .cloned();
    if let Some(summary) = &mut summary {
        if !summary.calls.is_empty()
            || !summary.direct_calls.is_empty()
            || !summary.defined_callees.is_empty()
        {
            return None;
        }
        if let Some(defined_at) = summary.defined_at {
            summary.defined_at =
                Some(defined_at.checked_sub(procedure.declaration_site.as_ref()?.offset)?);
        }
    }
    Some(ClosedBodyProjection {
        module_facts: BodyModuleFacts::retain(unit),
        original_semantic_facts: function.semantic_facts.clone(),
        analytical: None,
        declaration,
        body: template.body,
        function: normalized,
        summary,
        instance_classes: unit
            .interproc
            .as_ref()
            .and_then(|analysis| analysis.global_instance_classes.get(&body_id))
            .cloned(),
        tainted_global_writes: unit
            .interproc
            .as_ref()
            .and_then(|analysis| analysis.tainted_global_writes.get(&body_id))
            .cloned(),
    })
}

fn analytical_closed_projection(
    unit: &CompilationUnit,
    body_id: ProcedureId,
    registry: &CommandRegistry,
) -> Option<ClosedBodyProjection> {
    let module = &unit.ir_module;
    let procedure = module.procedures.get(&body_id)?;
    if procedure.body_kind != crate::ir::ProcedureBodyKind::Named
        || !unit.methods.is_empty()
        || !unit.body_units.is_empty()
        || unit.connection_scope.is_some()
        || !module.class_relations.is_empty()
        || !module.oo_unanalysed_classes.is_empty()
        || !module.redefined_methods.is_empty()
        || module.reference_bodies != crate::ir::ReferenceBodies::default()
    {
        return None;
    }
    let publication = procedure.publication.as_ref()?;
    if module
        .procedures
        .values()
        .filter(|candidate| candidate.publication.as_ref() == Some(publication))
        .count()
        != 1
    {
        return None;
    }
    let declaration = NeutralDeclaration {
        publication: publication.clone(),
        namespace: procedure.body.namespace_context.as_deref()?.clone(),
        written_name: procedure.name.clone(),
        display: procedure.qualified_name.clone(),
        parameters: procedure.params.clone(),
        parameters_text: procedure.params_raw.clone(),
        body_text: procedure.body_source.clone()?,
        kind: procedure.body_kind,
    };
    let function = unit.procedures.get(&body_id)?;
    let mut body = procedure.body.clone();
    crate::lattice_rebase::rebase_script(&mut body, -i64::from(procedure.span.start()));
    let bindings = crate::command_binding::ModuleCommandBindings::analyse(module, registry);
    let correspondence = bindings.prepare_analytical_body_correspondence(
        &body,
        crate::command_binding::BodyProofScope {
            source: &unit.source,
            body_source: &declaration.body_text,
            original_body_offset: procedure.span.start(),
            executable_body_offset: procedure.body_offset,
            metadata: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                function.source_metadata_input(),
            ),
            config: function.source_lexer_config(),
        },
        registry,
    )?;
    let mut normalized = function.clone();
    crate::lattice_rebase::rebase_function_unit(
        &mut normalized,
        -i64::from(procedure.span.start()),
    );
    let mut normalized = normalized
        .relocated_variable_proofs(&correspondence.relocation)?
        .restored_source_proofs(&crate::command_binding::BodySourceProofs::from_body(
            &correspondence.body,
        ))?;
    // This comparison-only ID is protected by the original declaration and
    // every authentic closed registry lookup; no returned unit is rewritten.
    normalized.body_id = None;
    let mut summary = unit
        .interproc
        .as_ref()
        .and_then(|analysis| analysis.procedures.get(&body_id))
        .cloned();
    if let Some(summary) = &mut summary {
        if !summary.calls.is_empty()
            || !summary.direct_calls.is_empty()
            || !summary.defined_callees.is_empty()
        {
            return None;
        }
        if let Some(offset) = summary.defined_at {
            summary.defined_at =
                Some(offset.checked_sub(procedure.declaration_site.as_ref()?.offset)?);
        }
    }
    Some(ClosedBodyProjection {
        body: correspondence.body.clone(),
        analytical: Some(correspondence),
        declaration,
        function: normalized,
        summary,
        module_facts: BodyModuleFacts::retain(unit),
        original_semantic_facts: function.semantic_facts.clone(),
        instance_classes: unit
            .interproc
            .as_ref()
            .and_then(|analysis| analysis.global_instance_classes.get(&body_id))
            .cloned(),
        tainted_global_writes: unit
            .interproc
            .as_ref()
            .and_then(|analysis| analysis.tainted_global_writes.get(&body_id))
            .cloned(),
    })
}

/// Genuine source body inputs; the private certificate changes no executable IR.
#[derive(Debug, Clone)]
pub struct BodyLatticeInputs {
    body: crate::ir::Script,
    bindings: Option<crate::command_binding::ModuleCommandBindings>,
    certificate: Option<Arc<BodyLatticeCertificate>>,
}

#[derive(Debug, Clone)]
struct BodyLatticeCertificate {
    issuer: tcl_core_types::ProcedureIdIssuer,
    body: ProcedureId,
    retained_source: Arc<crate::command_binding::RetainedSourceModuleBindings>,
    procedure: Procedure,
    declaration: NeutralDeclaration,
    correspondence: crate::command_binding::AnalyticalBodyCorrespondence,
}

impl PartialEq for BodyLatticeCertificate {
    fn eq(&self, other: &Self) -> bool {
        self.issuer.owns(self.body)
            && other.issuer.owns(other.body)
            && self.declaration == other.declaration
            && self.correspondence == other.correspondence
    }
}
impl Eq for BodyLatticeCertificate {}

impl Hash for BodyLatticeCertificate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.declaration.hash(state);
        self.correspondence.body.hash(state);
    }
}

impl From<crate::ir::Script> for BodyLatticeInputs {
    fn from(body: crate::ir::Script) -> Self {
        Self {
            body,
            bindings: None,
            certificate: None,
        }
    }
}

impl BodyLatticeInputs {
    /// Authenticate original Module allocation, declaration, body, grammar and lookup dependencies.
    #[must_use]
    pub fn retain(
        request: &crate::compilation_unit::LatticeRequest<'_>,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let module = request.module;
        if !module.procedure_ids.owns(request.body_id)
            || module.source.try_text().ok()? != request.source
            || request.irules_event_body.is_some()
        {
            return None;
        }
        let procedure = module.procedures.get(&request.body_id)?;
        let retained = module.retained_source_bindings.as_ref()?;
        retained.positioned_for_module(module, registry)?;
        let mut original = procedure.body.clone();
        crate::lattice_rebase::rebase_script(&mut original, -i64::from(procedure.span.start()));
        if !retained.matches_original_procedure(procedure)
            || procedure.body_kind != crate::ir::ProcedureBodyKind::Named
            || procedure.qualified_name != request.qname
            || procedure.params != request.params
            || procedure.span.start() != request.original_body_offset
            || procedure.body_offset != request.executable_body_offset
            || procedure.body_source.as_deref() != request.body_source
            || &original != request.body
        {
            return None;
        }
        let declaration = NeutralDeclaration {
            publication: procedure.publication.clone()?,
            namespace: procedure.body.namespace_context.as_deref()?.clone(),
            written_name: procedure.name.clone(),
            display: procedure.qualified_name.clone(),
            parameters: procedure.params.clone(),
            parameters_text: procedure.params_raw.clone(),
            body_text: procedure.body_source.clone()?,
            kind: procedure.body_kind,
        };
        let correspondence = request
            .command_bindings
            .prepare_analytical_body_correspondence(
                request.body,
                crate::command_binding::BodyProofScope {
                    source: request.source,
                    body_source: &declaration.body_text,
                    original_body_offset: request.original_body_offset,
                    executable_body_offset: request.executable_body_offset,
                    metadata: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                        request.source_metadata_input,
                    ),
                    config: request.lexer_config,
                },
                registry,
            )?;
        Some(Self {
            body: request.body.clone(),
            bindings: Some(request.command_bindings.clone()),
            certificate: Some(Arc::new(BodyLatticeCertificate {
                issuer: module.procedure_ids.clone(),
                body: request.body_id,
                retained_source: Arc::clone(retained),
                procedure: procedure.clone(),
                declaration,
                correspondence,
            })),
        })
    }

    /// Original offset-zero IR retained by the genuine request.
    /// Original source body retained for this lattice input.
    #[must_use]
    pub fn body(&self) -> &crate::ir::Script {
        &self.body
    }

    /// Retain actual command state under the same checked dependency certificate.
    #[must_use]
    pub fn command_bindings(&self) -> Option<BodyLatticeCommandBindings> {
        Some(BodyLatticeCommandBindings {
            bindings: self.bindings.clone()?,
            certificate: self.certificate.clone(),
        })
    }

    /// Restore a cached offset-zero lattice to this request's actual source and cells.
    #[must_use]
    pub fn rebind_cached(&self, original: &Self, cached: &FunctionUnit) -> Option<FunctionUnit> {
        let current = self.certificate.as_ref()?;
        let previous = original.certificate.as_ref()?;
        if current != previous
            || !current.issuer.owns(current.body)
            || !previous.issuer.owns(previous.body)
            || !current
                .retained_source
                .matches_original_procedure(&current.procedure)
            || !previous
                .retained_source
                .matches_original_procedure(&previous.procedure)
            || cached.name != previous.procedure.qualified_name
        {
            return None;
        }
        let normalized = cached
            .relocated_variable_proofs(&previous.correspondence.relocation)?
            .restored_source_proofs(&crate::command_binding::BodySourceProofs::from_body(
                &previous.correspondence.body,
            ))?;
        normalized
            .relocated_variable_proofs(&current.correspondence.relocation.inverse()?)?
            .restored_source_proofs(&crate::command_binding::BodySourceProofs::from_body(
                &self.body,
            ))
    }
}

impl PartialEq for BodyLatticeInputs {
    fn eq(&self, other: &Self) -> bool {
        match (&self.certificate, &other.certificate) {
            (Some(left), Some(right)) => left == right,
            (None, None) => self.body == other.body,
            _ => false,
        }
    }
}
impl Eq for BodyLatticeInputs {}
impl Hash for BodyLatticeInputs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.certificate.is_some().hash(state);
        if let Some(certificate) = &self.certificate {
            certificate.hash(state);
        } else {
            self.body.hash(state);
        }
    }
}

/// Complete command state retained separately from its checked source correspondence.
#[derive(Debug, Clone)]
pub struct BodyLatticeCommandBindings {
    bindings: crate::command_binding::ModuleCommandBindings,
    certificate: Option<Arc<BodyLatticeCertificate>>,
}

impl From<crate::command_binding::ModuleCommandBindings> for BodyLatticeCommandBindings {
    fn from(bindings: crate::command_binding::ModuleCommandBindings) -> Self {
        Self {
            bindings,
            certificate: None,
        }
    }
}
impl BodyLatticeCommandBindings {
    /// Genuine command state used to construct the cached lattice.
    /// Original command state retained for this lattice input.
    #[must_use]
    pub fn bindings(&self) -> &crate::command_binding::ModuleCommandBindings {
        &self.bindings
    }
}
impl PartialEq for BodyLatticeCommandBindings {
    fn eq(&self, other: &Self) -> bool {
        match (&self.certificate, &other.certificate) {
            (Some(left), Some(right)) => left == right,
            (None, None) => self.bindings == other.bindings,
            _ => false,
        }
    }
}
impl Eq for BodyLatticeCommandBindings {}
impl Hash for BodyLatticeCommandBindings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.certificate.is_some().hash(state);
        if let Some(certificate) = &self.certificate {
            certificate.hash(state);
        } else {
            self.bindings.hash(state);
        }
    }
}

/// Genuine fixpoint inputs with a certified registry-only independence case.
#[derive(Debug, Clone)]
pub struct BodySummaryInputs {
    capsule: BodyAnalysisCapsule,
    known: Vec<ProcedureId>,
    summaries: Vec<(ProcedureId, crate::taint_interproc::ProcTaintSummary)>,
}

impl BodySummaryInputs {
    #[must_use]
    /// Retain actual body dependencies, declining a mismatched module inventory.
    pub fn retain(
        capsule: BodyAnalysisCapsule,
        known: &std::collections::HashSet<ProcedureId>,
        summaries: &std::collections::HashMap<
            ProcedureId,
            crate::taint_interproc::ProcTaintSummary,
        >,
    ) -> Option<Self> {
        if known
            .iter()
            .any(|id| !capsule.unit().ir_module.procedures.contains_key(id))
            || summaries.iter().any(|(id, summary)| {
                capsule
                    .unit()
                    .ir_module
                    .procedures
                    .get(id)
                    .is_none_or(|procedure| procedure.params != summary.params)
            })
        {
            return None;
        }
        let mut known: Vec<_> = known.iter().copied().collect();
        known.sort_unstable();
        let mut summaries: Vec<_> = summaries
            .iter()
            .map(|(id, summary)| (*id, summary.clone()))
            .collect();
        summaries.sort_by_key(|(id, _)| *id);
        Some(Self {
            capsule,
            known,
            summaries,
        })
    }

    #[must_use]
    /// The genuine compilation and declaration owner.
    pub fn capsule(&self) -> &BodyAnalysisCapsule {
        &self.capsule
    }

    #[must_use]
    /// Actual retained body IDs used by the current inference.
    pub fn known(&self) -> &[ProcedureId] {
        &self.known
    }

    #[must_use]
    /// Current taint summaries indexed by their actual body IDs.
    pub fn summaries(&self) -> &[(ProcedureId, crate::taint_interproc::ProcTaintSummary)] {
        &self.summaries
    }
}

impl PartialEq for BodySummaryInputs {
    fn eq(&self, other: &Self) -> bool {
        self.capsule == other.capsule
            && (self.capsule.has_closed_analysis_correspondence()
                || (self.known == other.known && self.summaries == other.summaries))
    }
}
impl Eq for BodySummaryInputs {}

impl Hash for BodySummaryInputs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.capsule.hash(state);
        if !self.capsule.has_closed_analysis_correspondence() {
            self.known.hash(state);
            self.summaries.hash(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_source_unit(
        source: &str,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Arc<CompilationUnit> {
        let registry = input.borrowed_context_registry().commands();
        Arc::new(
            CompilationUnit::build_with_analysis_input(
                source,
                crate::compilation_unit::UnitBuildOptions {
                    registry,
                    defer_top_level: false,
                    config: input.lexer_config(),
                    dialect: Some(input.unit_profile()),
                    external_call_sites: None,
                    declared_commands: None,
                },
                None,
                input,
            )
            .with_interprocedural(registry, Some(input.unit_profile())),
        )
    }

    #[test]
    fn logical_source_correspondence_reuses_an_unchanged_body_after_an_unrelated_edit() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        );
        for body in ["set x 11111", "puts 11111"] {
            let left = current_source_unit(
                &format!("proc a {{}} {{{body}}}\nproc b {{}} {{set y 22}}"),
                &input,
            );
            let right = current_source_unit(
                &format!("\nproc a {{}} {{{body}}}\nproc b {{}} {{set y 999999}}"),
                &input,
            );
            let left_id = *left.ir_module.procedures.unique_display("::a").unwrap().0;
            let right_id = *right.ir_module.procedures.unique_display("::a").unwrap().0;
            let a = BodyAnalysisCapsule::retain(Arc::clone(&left), left_id, context.commands())
                .unwrap();
            let b = BodyAnalysisCapsule::retain(Arc::clone(&right), right_id, context.commands())
                .unwrap();
            assert_ne!(left_id, right_id);
            assert!(a.has_closed_analysis_correspondence(), "source: {body}");
            assert!(!a.has_closed_native_correspondence());
            assert_eq!(a, b, "unchanged exact Logical body: {body}");
            assert!(std::ptr::eq(b.unit(), right.as_ref()));
            assert_eq!(b.body_id(), right_id);
        }
    }

    #[test]
    fn original_document_target_keeps_its_actual_summary_dependencies() {
        let (context, actual) = unit("proc helper {} {return SAFE}\nproc caller {} {helper}");
        let caller = *actual
            .ir_module
            .procedures
            .unique_display("::caller")
            .unwrap()
            .0;
        let capsule = BodyAnalysisCapsule::retain(actual, caller, context.commands()).unwrap();
        assert!(!capsule.has_closed_analysis_correspondence());
    }

    fn unit(
        source: &str,
    ) -> (
        Arc<tcl_registry::model::ContextRegistry>,
        Arc<CompilationUnit>,
    ) {
        let (context, unit) = crate::interprocedural::logical_completion_unit(source, "tcl8.6");
        (context, Arc::new(unit))
    }

    #[test]
    fn source_body_capsule_keeps_the_actual_module_and_function() {
        let (context, unit) = unit("proc p {argument} {return $argument}");
        let body = *unit.ir_module.procedures.keys().next().unwrap();
        let capsule = BodyAnalysisCapsule::retain(Arc::clone(&unit), body, context.commands())
            .expect("original declaration, actual module and actual body");
        assert!(std::ptr::eq(capsule.unit(), unit.as_ref()));
        assert!(std::ptr::eq(capsule.function(), &unit.procedures[&body]));
        assert_eq!(capsule.body_id(), body);
        assert_eq!(capsule, capsule.clone());
    }

    #[test]
    fn equal_source_and_display_do_not_replace_the_module_owner() {
        let source = "proc p {} {return VALUE}";
        let (context, left) = unit(source);
        let (_, right) = unit(source);
        let left_id = *left.ir_module.procedures.keys().next().unwrap();
        let right_id = *right.ir_module.procedures.keys().next().unwrap();
        assert_eq!(left_id.ordinal(), right_id.ordinal());
        assert_ne!(left_id, right_id);
        assert!(
            BodyAnalysisCapsule::retain(Arc::clone(&left), right_id, context.commands()).is_none()
        );
        assert_ne!(
            BodyAnalysisCapsule::retain(left, left_id, context.commands()).unwrap(),
            BodyAnalysisCapsule::retain(right, right_id, context.commands()).unwrap(),
        );
    }

    #[test]
    fn changed_source_registry_or_body_owner_withdraws_the_capsule() {
        let source = "proc p {} {return VALUE}";
        let (context, actual) = unit(source);
        let body = *actual.ir_module.procedures.keys().next().unwrap();
        let mut changed_source = (*actual).clone();
        changed_source.source = "proc p {} {return OTHER}".to_owned();
        assert!(
            BodyAnalysisCapsule::retain(Arc::new(changed_source), body, context.commands())
                .is_none()
        );
        let mut changed_function = (*actual).clone();
        changed_function.procedures.get_mut(&body).unwrap().body_id = None;
        assert!(
            BodyAnalysisCapsule::retain(Arc::new(changed_function), body, context.commands())
                .is_none()
        );
        let foreign = CommandRegistry::build_default();
        assert!(BodyAnalysisCapsule::retain(actual, body, &foreign).is_none());
    }
}
