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

mod readonly_authored_coverage;

/// Carry each Installed lowering that readonly authored coverage pairs with
/// a readonly authored body unit in that unit. C Tcl creates one command per
/// evaluated `proc`, so the pair is one analysis unit; the Installed lowering
/// keeps its own body ID, function, prepared CFG and Source SSA summary.
pub(crate) fn carry_readonly_authored_lowerings(
    unit: &mut CompilationUnit,
    registry: &CommandRegistry,
) {
    if let Some(coverage) =
        readonly_authored_coverage::ReadonlyAuthoredCoverage::pairs(unit, registry)
    {
        coverage.carry(unit);
    }
}

/// One actual body and its complete immutable compilation inputs.
#[derive(Debug, Clone)]
pub struct BodyAnalysisCapsule {
    unit: Arc<CompilationUnit>,
    body: ProcedureId,
    registry: tcl_registry::RegistrySnapshot,
    neutral: Option<Arc<ClosedBodyProjection>>,
}

#[derive(Debug, Clone)]
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

impl PartialEq for ClosedBodyProjection {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            declaration,
            body,
            function,
            summary,
            instance_classes,
            tainted_global_writes,
            analytical,
            module_facts,
            original_semantic_facts,
        } = self;
        declaration == &other.declaration
            && body == &other.body
            && function == &other.function
            && summary == &other.summary
            && instance_classes == &other.instance_classes
            && tainted_global_writes == &other.tainted_global_writes
            && analytical == &other.analytical
            && module_facts == &other.module_facts
            && (original_semantic_facts == &other.original_semantic_facts
                || analytical
                    .as_ref()
                    .zip(other.analytical.as_ref())
                    .is_some_and(|(source, target)| {
                        original_semantic_facts.matches_rebound_source(
                            &other.original_semantic_facts,
                            source,
                            target,
                        )
                    }))
    }
}

fn validate_body_request_input(
    request: &crate::compilation_unit::LatticeRequest<'_>,
) -> Option<()> {
    let actual_input = request.module.source_metadata_input.as_ref()?;
    let supplied_input = request.source_metadata_input?;
    if (supplied_input != actual_input && supplied_input != &actual_input.for_nested_source())
        || request.lexer_config.nested().normalized()
            != supplied_input.lexer_config().nested().normalized()
    {
        return None;
    }
    Some(())
}

fn original_neutral_declaration(
    procedure: &crate::ir::Procedure,
    native_card: Option<&crate::command_binding::OriginalNativeProcedureBody>,
) -> Option<NeutralDeclaration> {
    Some(NeutralDeclaration {
        publication: if native_card.is_some() {
            None
        } else {
            Some(procedure.publication.clone()?)
        },
        native_source_slot: native_card.map(|card| {
            (
                card.original_name_input().policy(),
                card.source_slot().clone(),
            )
        }),
        authored_source_slot: None,
        namespace: procedure.body.namespace_context.as_deref()?.clone(),
        written_name: procedure.name.clone(),
        display: procedure.qualified_name.clone(),
        parameters: procedure.params.clone(),
        parameters_text: procedure.params_raw.clone(),
        body_text: procedure.body_source.clone()?,
        kind: procedure.body_kind,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NeutralDeclaration {
    publication: Option<crate::signature_scan::scope::SignatureSourceCommand>,
    native_source_slot: Option<(
        tcl_syntax::naming::NamePolicyProtocol,
        tcl_core_types::ByteCommandSlot,
    )>,
    authored_source_slot: Option<tcl_core_types::ByteCommandSlot>,
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
        let procedure = unit.original_procedure_for_body(body)?;
        let function = unit.function_for_body(Some(body))?;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.positioned_for_module(module, registry)?;
        if function.body_id != Some(body)
            || unit.original_procedure_cfg_for_body(body) != Some(&function.cfg)
            || module.source.try_text().ok()? != unit.source
            || procedure.declaration_site.is_none()
            || !retained.matches_original_procedure_summary(body, procedure)
            || !module
                .original_source_procedure_bodies()
                .any(|(id, owned)| *id == body && std::ptr::eq(owned, procedure))
        {
            return None;
        }
        let neutral = native_closed_projection(&unit, body, registry)
            .or_else(|| analytical_closed_projection(&unit, body, registry))
            .map(Arc::new);
        Some(Self {
            unit,
            body,
            registry: registry.snapshot(),
            neutral,
        })
    }

    /// The complete retained compilation unit, including original world and input owners.
    #[must_use]
    pub fn unit(&self) -> &CompilationUnit {
        &self.unit
    }

    /// Exact immutable Registry used to validate this retained source owner.
    #[must_use]
    pub fn registry_snapshot(&self) -> &tcl_registry::RegistrySnapshot {
        &self.registry
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

    /// Independently checked registry-only analysis dependencies exclude document summaries.
    #[must_use]
    pub const fn has_closed_analysis_correspondence(&self) -> bool {
        self.neutral.is_some()
    }

    /// Independently certified readonly source analysis, retaining its complete
    /// captured module mutation and trace dependencies. This supplies neither
    /// runtime quietness nor execution, handler, frame or Native authority.
    #[must_use]
    pub fn has_closed_readonly_source_correspondence(&self) -> bool {
        self.neutral.as_ref().is_some_and(|projection| {
            projection.analytical.is_some()
                && (projection.declaration.native_source_slot.is_some()
                    || projection.declaration.authored_source_slot.is_some())
        })
    }

    /// The exact retained source declaration.
    #[must_use]
    pub fn procedure(&self) -> &Procedure {
        self.unit
            .original_procedure_for_body(self.body)
            .expect("retained capsule declaration")
    }

    /// The actual analysis belonging to the retained declaration.
    #[must_use]
    pub fn function(&self) -> &FunctionUnit {
        self.unit
            .function_for_body(Some(self.body))
            .expect("retained capsule function")
    }

    /// Actual retained worker context for this current source-prepared capsule.
    /// This memo-purpose dependency cannot issue a result or an edit; result
    /// reborrows independently authenticate the same context.
    #[must_use]
    pub fn original_optimiser_analysis_context(
        &self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Option<crate::value_transfer::AnalysisContextKey> {
        if self.registry != registry.snapshot() {
            return None;
        }
        self.unit
            .original_optimiser_prepared_unit(registry, dialect)
            .map(|owner| owner.origin().analysis_context().clone())
    }

    /// Body-local reporting payload from one actual whole-unit worker run.
    /// This compatibility projection carries no collection or edit permission.
    #[must_use]
    pub fn original_body_optimisations(
        &self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Vec<Optimisation> {
        self.original_body_optimiser_report(registry, dialect)
            .members()
            .to_vec()
    }

    /// Complete body projection, including an empty projection, from one actual
    /// raw worker run. Missing issuance never discards or reruns report output.
    #[must_use]
    pub fn original_body_optimiser_report(
        &self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> crate::optimiser::OriginalBodyOptimiserReport {
        let raw = crate::optimiser::optimise_unit_raw_report(self.unit(), registry, dialect);
        crate::optimiser::OriginalBodyOptimiserReport::from_body_worker(
            self, raw, registry, dialect,
        )
    }
}

impl PartialEq for BodyAnalysisCapsule {
    fn eq(&self, other: &Self) -> bool {
        self.registry == other.registry
            && match (&self.neutral, &other.neutral) {
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
        self.registry.hash(state);
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
        publication: Some(publication.clone()),
        native_source_slot: None,
        authored_source_slot: None,
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

fn authored_source_slot(
    module: &crate::ir::Module,
    id: ProcedureId,
) -> Option<tcl_core_types::ByteCommandSlot> {
    let card = module.original_authored_declaration_body_units.get(&id)?;
    let procedure = module.body_units.get(&id)?;
    if !module.procedure_ids.owns(id) || !card.matches_procedure(module, procedure) {
        return None;
    }
    // This card's original producer accepts only the explicit root source
    // declaration grammar. A report label supplies no namespace coordinate.
    let root = tcl_core_types::ByteNamespacePath::root();
    let slot = tcl_syntax::naming::authored_source_command_slot(&root, card.name().as_bytes())?;
    let count = module
        .original_authored_declaration_body_units
        .iter()
        .filter(|(body, candidate)| {
            module
                .body_units
                .get(body)
                .is_some_and(|procedure| candidate.matches_procedure(module, procedure))
                && tcl_syntax::naming::authored_source_command_slot(
                    &root,
                    candidate.name().as_bytes(),
                )
                .as_ref()
                    == Some(&slot)
        })
        .count();
    (count == 1).then_some(slot)
}

fn closed_source_body_inventory(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    authored: bool,
) -> bool {
    let module = &unit.ir_module;
    if authored {
        readonly_authored_coverage::ReadonlyAuthoredCoverage::capture(unit, registry).is_some_and(
            |coverage| {
                unit.body_lowering_functions().all(|(id, _)| {
                    module
                        .original_authored_declaration_body_units
                        .contains_key(id)
                        || coverage.source_for(*id).is_some()
                })
            },
        )
    } else {
        unit.body_lowering_functions().all(|(id, _)| {
            module
                .original_native_declaration_body_units
                .contains_key(id)
                || module
                    .original_authored_declaration_body_units
                    .contains_key(id)
        })
    }
}

fn closed_source_declaration(
    module: &crate::ir::Module,
    procedure: &Procedure,
    body_id: ProcedureId,
    native_card: Option<&crate::command_binding::OriginalNativeProcedureBody>,
    authored: bool,
) -> Option<NeutralDeclaration> {
    let publication = if let Some(card) = native_card {
        if module
            .original_native_declaration_body_units
            .values()
            .filter(|candidate| {
                candidate.original_name_input().policy() == card.original_name_input().policy()
                    && candidate.source_slot() == card.source_slot()
            })
            .count()
            != 1
        {
            return None;
        }
        None
    } else if authored {
        authored_source_slot(module, body_id)?;
        None
    } else {
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
        Some(publication.clone())
    };
    Some(NeutralDeclaration {
        publication,
        native_source_slot: native_card.map(|card| {
            (
                card.original_name_input().policy(),
                card.source_slot().clone(),
            )
        }),
        authored_source_slot: if authored {
            Some(authored_source_slot(module, body_id)?)
        } else {
            None
        },
        namespace: procedure.body.namespace_context.as_deref()?.clone(),
        written_name: procedure.name.clone(),
        display: procedure.qualified_name.clone(),
        parameters: procedure.params.clone(),
        parameters_text: procedure.params_raw.clone(),
        body_text: procedure.body_source.clone()?,
        kind: procedure.body_kind,
    })
}

fn rebase_closed_summary(
    summary: &mut Option<crate::interprocedural::ProcSummary>,
    procedure: &Procedure,
) -> Option<()> {
    if let Some(summary) = summary {
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
    Some(())
}

fn closed_function_metadata(
    function: &FunctionUnit,
    module: &crate::ir::Module,
    registry: &CommandRegistry,
    authored: bool,
) -> Option<()> {
    if authored {
        if function.source_metadata_input().is_some()
            || !function.cfg.metadata_context.is_standalone()
            || !module.source_entry.metadata_context.is_standalone()
            || function.source_lexer_config() != module.native_lexer_config().nested().normalized()
        {
            return None;
        }
    } else {
        function.invocation_metadata_context_for_module(registry, module)?;
    }
    Some(())
}

fn analytical_closed_projection(
    unit: &CompilationUnit,
    body_id: ProcedureId,
    registry: &CommandRegistry,
) -> Option<ClosedBodyProjection> {
    let module = &unit.ir_module;
    let procedure = unit.original_procedure_for_body(body_id)?;
    let native_card = module
        .original_native_declaration_body_units
        .get(&body_id)
        .filter(|card| card.matches_procedure(module, procedure));
    let authored_card = module
        .original_authored_declaration_body_units
        .get(&body_id)
        .filter(|card| card.matches_procedure(module, procedure));
    let readonly_source = native_card.is_some() || authored_card.is_some();
    if (!readonly_source && procedure.body_kind != crate::ir::ProcedureBodyKind::Named)
        || !unit.methods.is_empty()
        || (!readonly_source && !unit.body_units.is_empty())
        || !closed_source_body_inventory(unit, registry, authored_card.is_some())
        || unit.connection_scope.is_some()
        || !module.class_relations.is_empty()
        || !module.oo_unanalysed_classes.is_empty()
        || !module.redefined_methods.is_empty()
        || module.reference_bodies != crate::ir::ReferenceBodies::default()
    {
        return None;
    }
    let declaration = closed_source_declaration(
        module,
        procedure,
        body_id,
        native_card.map(Arc::as_ref),
        authored_card.is_some(),
    )?;
    let function = unit.function_for_body(Some(body_id))?;
    closed_function_metadata(function, module, registry, authored_card.is_some())?;
    let mut body = procedure.body.clone();
    crate::lattice_rebase::rebase_script(&mut body, -i64::from(procedure.span.start()));
    let bindings = crate::command_binding::ModuleCommandBindings::analyse(module, registry);
    let scope = crate::command_binding::BodyProofScope {
        source: &unit.source,
        body_source: &declaration.body_text,
        original_body_offset: procedure.span.start(),
        executable_body_offset: procedure.body_offset,
        metadata: if authored_card.is_some() {
            crate::registry_invocation::InvocationMetadataInput::Retained(
                &module.source_entry.metadata_context,
            )
        } else {
            crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                function.source_metadata_input(),
            )
        },
        config: function.source_lexer_config(),
    };
    let correspondence = if native_card.is_some() {
        bindings
            .prepare_native_source_body_correspondence(&body, scope, module, body_id, registry)?
    } else if authored_card.is_some() {
        bindings
            .prepare_authored_source_body_correspondence(&body, scope, module, body_id, registry)?
    } else {
        bindings.prepare_analytical_body_correspondence(&body, scope, registry)?
    };
    let mut normalized = function.clone();
    crate::lattice_rebase::rebase_function_unit(
        &mut normalized,
        -i64::from(procedure.span.start()),
    );
    let normalized = normalized.relocated_variable_proofs(&correspondence.relocation)?;
    let proofs = crate::command_binding::BodySourceProofs::from_body(&correspondence.body);
    let mut normalized = if readonly_source {
        normalized.restored_readonly_source_proofs(&proofs)?
    } else {
        normalized.restored_source_proofs(&proofs)?
    };
    // This comparison-only ID is protected by the original declaration and
    // every authentic closed registry lookup; no returned unit is rewritten.
    normalized.body_id = None;
    let mut summary = unit
        .interproc
        .as_ref()
        .and_then(|analysis| analysis.procedures.get(&body_id))
        .cloned();
    rebase_closed_summary(&mut summary, procedure)?;
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
    pub(crate) original_worker_recipe:
        Option<Arc<crate::compilation_unit::original_lattice_worker::OriginalLatticeWorkerRecipe>>,
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
            original_worker_recipe: None,
            body,
            bindings: None,
            certificate: None,
        }
    }
}

impl BodyLatticeInputs {
    /// Retain an authentic current body and independently available neutral correspondence.
    #[must_use]
    pub fn retain(
        request: &crate::compilation_unit::LatticeRequest<'_>,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let mut retained = Self::retain_correspondence(request, registry)?;
        let neutral = retained.command_bindings().is_some();
        BodyLatticeInputs::retain_original_worker_recipe(&mut retained, request, registry, neutral);
        Some(retained)
    }

    fn retain_correspondence(
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
        let procedure = request.original_source_procedure()?;
        let native_card = module
            .original_native_declaration_body_units
            .get(&request.body_id)
            .filter(|card| card.matches_procedure(module, procedure));
        let logical_card = module
            .original_logical_declaration_body_units
            .get(&request.body_id)
            .filter(|card| card.matches_procedure(module, procedure));
        if module
            .original_authored_declaration_body_units
            .contains_key(&request.body_id)
        {
            return Self::retain_authored_source(request, registry);
        }
        validate_body_request_input(request)?;
        let retained = module.retained_source_bindings.as_ref()?;
        retained.positioned_for_module(module, registry)?;
        let mut original = procedure.body.clone();
        crate::lattice_rebase::rebase_script(&mut original, -i64::from(procedure.span.start()));
        if !retained.matches_original_procedure_summary(request.body_id, procedure)
            || !module
                .original_source_procedure_bodies()
                .any(|(id, owned)| *id == request.body_id && std::ptr::eq(owned, procedure))
            || (procedure.body_kind != crate::ir::ProcedureBodyKind::Named
                && native_card.is_none()
                && logical_card.is_none())
            || procedure.qualified_name != request.qname
            || procedure.params != request.params
            || procedure.span.start() != request.original_body_offset
            || procedure.body_offset != request.executable_body_offset
            || procedure.body_source.as_deref() != request.body_source
            || &original != request.body
        {
            return None;
        }
        if logical_card.is_some()
            || (!module
                .original_native_declaration_body_units
                .contains_key(&request.body_id)
                && procedure.publication.is_none())
        {
            retained.original_logical_source_slot_for_body(module, request.body_id, registry)?;
            // Authentic conditional source does not imply neutral memo closure.
            return Some(Self::from(request.body.clone()));
        }
        let declaration = original_neutral_declaration(procedure, native_card.map(Arc::as_ref))?;
        let scope = crate::command_binding::BodyProofScope {
            source: request.source,
            body_source: &declaration.body_text,
            original_body_offset: request.original_body_offset,
            executable_body_offset: request.executable_body_offset,
            metadata: crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                request.source_metadata_input,
            ),
            config: request.lexer_config,
        };
        let correspondence = if native_card.is_some() {
            request
                .command_bindings
                .prepare_native_source_body_correspondence(
                    request.body,
                    scope,
                    module,
                    request.body_id,
                    registry,
                )?
        } else if let Some(correspondence) = request
            .command_bindings
            .prepare_analytical_body_correspondence(request.body, scope, registry)
        {
            correspondence
        } else {
            retained.original_logical_source_slot_for_body(module, request.body_id, registry)?;
            return Some(Self::from(request.body.clone()));
        };
        Some(Self {
            original_worker_recipe: None,
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

    fn retain_authored_source(
        request: &crate::compilation_unit::LatticeRequest<'_>,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let module = request.module;
        let procedure = request.original_source_procedure()?;
        let card = module
            .original_authored_declaration_body_units
            .get(&request.body_id)?;
        let retained = module.retained_source_bindings.as_ref()?;
        retained.positioned_for_module(module, registry)?;
        if !card.matches_procedure(module, procedure)
            || !request.original_metadata_context(registry).is_standalone()
            || request.source_metadata_input.is_some()
            || module.source_metadata_input.is_some()
            || module
                .original_native_declaration_body_units
                .contains_key(&request.body_id)
            || module
                .original_logical_declaration_body_units
                .contains_key(&request.body_id)
            || !retained.matches_original_procedure_summary(request.body_id, procedure)
        {
            return None;
        }
        let mut original = procedure.body.clone();
        crate::lattice_rebase::rebase_script(&mut original, -i64::from(procedure.span.start()));
        if &original != request.body {
            return None;
        }
        let declaration = NeutralDeclaration {
            publication: None,
            native_source_slot: None,
            authored_source_slot: Some(authored_source_slot(module, request.body_id)?),
            namespace: procedure.body.namespace_context.as_deref()?.clone(),
            written_name: procedure.name.clone(),
            display: procedure.qualified_name.clone(),
            parameters: procedure.params.clone(),
            parameters_text: procedure.params_raw.clone(),
            body_text: procedure.body_source.clone()?,
            kind: procedure.body_kind,
        };
        let scope = crate::command_binding::BodyProofScope {
            source: request.source,
            body_source: &declaration.body_text,
            original_body_offset: request.original_body_offset,
            executable_body_offset: request.executable_body_offset,
            metadata: crate::registry_invocation::InvocationMetadataInput::Retained(
                &module.source_entry.metadata_context,
            ),
            config: request.lexer_config,
        };
        let correspondence = request
            .command_bindings
            .prepare_authored_source_body_correspondence(
                request.body,
                scope,
                module,
                request.body_id,
                registry,
            )?;
        Some(Self {
            original_worker_recipe: None,
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

    /// Actual issued source body retained by an independent Native source certificate.
    /// This cannot select an executable body or reconstruct an ID from a label.
    #[must_use]
    pub fn certified_native_source_body_id(&self) -> Option<ProcedureId> {
        let certificate = self.certificate.as_ref()?;
        (certificate.declaration.native_source_slot.is_some()
            && certificate.issuer.owns(certificate.body)
            && certificate
                .retained_source
                .matches_original_procedure_summary(certificate.body, &certificate.procedure))
        .then_some(certificate.body)
    }

    /// Actual issued readonly declaration certified for source-analysis restoration.
    /// This includes independent Native-source and explicit Standalone source
    /// purposes; it grants no executable body, entry, publication or frame.
    #[must_use]
    pub fn certified_readonly_source_body_id(&self) -> Option<ProcedureId> {
        let certificate = self.certificate.as_ref()?;
        ((certificate.declaration.native_source_slot.is_some()
            || certificate.declaration.authored_source_slot.is_some())
            && certificate.issuer.owns(certificate.body)
            && certificate
                .retained_source
                .matches_original_procedure_summary(certificate.body, &certificate.procedure))
        .then_some(certificate.body)
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
                .matches_original_procedure_summary(current.body, &current.procedure)
            || !previous
                .retained_source
                .matches_original_procedure_summary(previous.body, &previous.procedure)
            || cached.name != previous.procedure.qualified_name
        {
            return None;
        }
        if (previous.declaration.native_source_slot.is_some()
            || previous.declaration.authored_source_slot.is_some())
            && cached.body_id != Some(previous.body)
        {
            return None;
        }
        let normalized = cached.relocated_variable_proofs(&previous.correspondence.relocation)?;
        let previous_proofs =
            crate::command_binding::BodySourceProofs::from_body(&previous.correspondence.body);
        let normalized = if previous.declaration.native_source_slot.is_some()
            || previous.declaration.authored_source_slot.is_some()
        {
            normalized.restored_readonly_source_proofs(&previous_proofs)?
        } else {
            normalized.restored_source_proofs(&previous_proofs)?
        };
        let rebound =
            normalized.relocated_variable_proofs(&current.correspondence.relocation.inverse()?)?;
        let current_proofs = crate::command_binding::BodySourceProofs::from_body(&self.body);
        let mut rebound = if current.declaration.native_source_slot.is_some()
            || current.declaration.authored_source_slot.is_some()
        {
            rebound.restored_readonly_source_proofs(&current_proofs)?
        } else {
            rebound.restored_source_proofs(&current_proofs)?
        };
        if current.declaration.native_source_slot.is_some()
            || current.declaration.authored_source_slot.is_some()
        {
            rebound.body_id = Some(current.body);
        }
        if !BodyLatticeInputs::rebind_original_worker_result(self, original, cached, &mut rebound) {
            rebound.clear_original_lattice_worker();
        }
        Some(rebound)
    }
}

impl BodyLatticeInputs {
    pub(crate) fn original_worker_body_matches(&self, other: &Self) -> bool {
        match (&self.certificate, &other.certificate) {
            (Some(left), Some(right)) => left == right,
            (None, None) => self.body == other.body,
            _ => false,
        }
    }
}
impl PartialEq for BodyLatticeInputs {
    fn eq(&self, other: &Self) -> bool {
        self.original_worker_recipe.is_some() == other.original_worker_recipe.is_some()
            && self.original_worker_body_matches(other)
    }
}
impl Eq for BodyLatticeInputs {}
impl Hash for BodyLatticeInputs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.original_worker_recipe.is_some() {
            true.hash(state);
        }
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
            .any(|id| capsule.unit().original_procedure_for_body(*id).is_none())
            || summaries.iter().any(|(id, summary)| {
                capsule
                    .unit()
                    .original_procedure_for_body(*id)
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
        let profile = tcl_dialect::DialectProfile::plain_tcl();
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
        assert!(std::ptr::eq(
            capsule.function(),
            std::ptr::from_ref(&unit.procedures[&body])
        ));
        assert_eq!(capsule.body_id(), body);
        assert_eq!(capsule, capsule.clone());
    }

    #[test]
    fn retained_body_registry_keeps_the_actual_source_store() {
        let (context, unit) = unit("proc helper {} {return SAFE}\nproc caller {} {helper}");
        let body = *unit
            .ir_module
            .procedures
            .unique_display("::caller")
            .unwrap()
            .0;
        let capsule =
            BodyAnalysisCapsule::retain(Arc::clone(&unit), body, context.commands()).unwrap();
        assert!(!capsule.has_closed_analysis_correspondence());
        assert_eq!(capsule.registry_snapshot(), &context.commands().snapshot());
        assert!(Arc::ptr_eq(
            &capsule.registry_snapshot().shared_registry(),
            &context.commands().snapshot().shared_registry()
        ));
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
        assert!(BodyAnalysisCapsule::retain(unit, body, foreign.commands()).is_none());
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

#[cfg(test)]
mod native_source_lattice_tests {
    // Unentered native declaration cards exist only under explicitly supplied
    // unknown runtime history; with no entry a fresh interpreter defines each
    // procedure (user decision 2026-10-19T14:20Z).
    use super::*;
    use crate::compilation_unit::{FunctionLatticeInputs, LatticeRequest, UnitDialect};

    fn build_current(req: &LatticeRequest<'_>, registry: &CommandRegistry) -> FunctionUnit {
        let metadata = req.original_metadata_context(registry);
        let prepared = crate::cfg_builder::PreparedCfgContext::from_metadata_input(
            (
                req.upvar_procs.clone(),
                req.proc_params.clone(),
                req.global_write_procs.clone(),
                req.command_bindings.clone(),
                req.procedure_summary_owners.clone(),
            ),
            registry,
            crate::registry_invocation::InvocationMetadataInput::Retained(&metadata),
        );
        let cfg = crate::cfg_builder::build_cfg_function_with_prepared_context(
            req.qname,
            req.body,
            true,
            registry,
            req.plain_command_dispatch,
            &prepared,
            req.lexer_config,
        );
        let known_classes = req.known_classes.iter().cloned().collect();
        let traced_variables = req.traced_variables.iter().cloned().collect();
        let parameters = crate::compilation_unit::decode_param_constants(req.param_constants);
        FunctionUnit::build_for_original_declaration_lattice(
            req.body_id,
            req.qname,
            cfg,
            req.params,
            FunctionLatticeInputs {
                dialect: UnitDialect {
                    registry,
                    config: req.lexer_config,
                    source_metadata_input: req.source_metadata_input,
                },
                param_constants: parameters.as_ref(),
                known_classes: &known_classes,
                trace_facts: crate::compilation_unit::ModuleTraceFacts {
                    traced_variables: &traced_variables,
                    has_dynamic_variable_trace: req.has_dynamic_variable_trace,
                    deferred_writes: &req.analysis_context.deferred_writes,
                },
                analysis_context: req.analysis_context,
                command_trust: req.command_trust,
                event_body: None,
            },
        )
        .with_retained_semantic_analysis(
            registry,
            Some(req.body),
            crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
        )
    }

    #[test]
    fn logical_declared_body_retains_raw_inputs_without_certifying_neutral_reuse() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let foreign_context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&foreign_context),
            config,
        );
        let mut visited = 0;
        let mut memo = |request: &LatticeRequest<'_>| {
            visited += 1;
            assert!(
                request
                    .module
                    .original_logical_declaration_body_units
                    .contains_key(&request.body_id)
            );
            let retained = BodyLatticeInputs::retain(request, context.commands())
                .expect("authentic current Logical declaration body");
            assert_eq!(retained.body(), request.body);
            assert!(retained.certificate.is_none());
            assert!(retained.command_bindings().is_none());
            assert!(retained.certified_native_source_body_id().is_none());
            let missing = LatticeRequest {
                source_metadata_input: None,
                ..*request
            };
            assert!(BodyLatticeInputs::retain(&missing, context.commands()).is_none());
            let foreign = LatticeRequest {
                source_metadata_input: Some(&foreign_input),
                ..*request
            };
            assert!(BodyLatticeInputs::retain(&foreign, context.commands()).is_none());
            let mut changed = request.module.clone();
            changed
                .body_units
                .get_mut(&request.body_id)
                .unwrap()
                .params
                .push("extra".into());
            let changed = LatticeRequest {
                module: &changed,
                ..*request
            };
            assert!(BodyLatticeInputs::retain(&changed, context.commands()).is_none());
            build_current(request, context.commands())
        };
        let no_isolated_body =
            |_: &str, _: &str| -> crate::ir::Script { panic!("source-only body") };
        let unit = CompilationUnit::build_memoized_with_analysis_input(
            "proc absent::logical {x} {return $x}\n",
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                config,
                dialect: Some(profile),
                defer_top_level: false,
                external_call_sites: None,
                declared_commands: None,
            },
            &mut memo,
            &no_isolated_body,
            None,
            &input,
        );
        drop(memo);
        assert_eq!(visited, 1);
        assert_eq!(
            unit.ir_module.original_logical_declaration_body_units.len(),
            1
        );
    }

    #[test]
    fn readonly_source_restoration_preserves_absence_of_execution_and_refuses_missing_carriers() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let source = "proc p {} {set value TEXT; return $value}\n";
        let options = crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        };
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            options,
            Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
            &input,
        );
        let (&id, card) = unit
            .ir_module
            .original_native_declaration_body_units
            .iter()
            .next()
            .unwrap();
        let procedure = unit.original_procedure_for_body(id).unwrap();
        assert!(card.matches_procedure(&unit.ir_module, procedure));
        let function = unit.function_for_body(Some(id)).unwrap();
        let proofs = crate::command_binding::BodySourceProofs::from_body(&procedure.body);
        let restored = function
            .restored_readonly_source_proofs(&proofs)
            .expect("current conditional source carriers without execution");
        assert!(restored.cfg.executed_source.is_none());
        assert!(restored.cfg.native_compilation_admission.is_none());
        assert!(restored.cfg.statement_sources.values().all(Option::is_none));
        assert!(Arc::ptr_eq(&restored.def_use, &function.def_use));
        assert!(function.restored_source_proofs(&proofs).is_none());
        let mut missing = procedure.body.clone();
        missing.command_binding_sites = crate::ir::CommandBindingSites::default();
        for statement in &mut missing.statements {
            if let Some(tokens) = statement.tokens_mut() {
                tokens.source_binding = None;
            }
        }
        let missing = crate::command_binding::BodySourceProofs::from_body(&missing);
        assert!(function.restored_readonly_source_proofs(&missing).is_none());
        let logical_profile = tcl_dialect::DialectProfile::plain_tcl();
        let logical_input = crate::analyser::ResolvedAnalysisInput::new(
            logical_profile,
            logical_profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(logical_profile.grammar),
        );
        let logical = CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                config: logical_input.lexer_config(),
                dialect: Some(logical_profile),
                ..options
            },
            None,
            &logical_input,
        );
        let entered = logical
            .ir_module
            .procedures
            .values()
            .next()
            .unwrap()
            .body
            .executed_source
            .as_ref()
            .unwrap();
        let mut changed = function.clone();
        changed.cfg.executed_source = Some(Arc::clone(entered));
        assert!(changed.restored_readonly_source_proofs(&proofs).is_none());
        let mut changed_body = procedure.body.clone();
        changed_body.executed_source = Some(Arc::clone(entered));
        let mixed = crate::command_binding::BodySourceProofs::from_body(&changed_body);
        assert!(function.restored_readonly_source_proofs(&mixed).is_none());
    }

    #[test]
    fn native_source_capsules_retain_current_owners_under_certified_unrelated_edits() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let build = |other: &str| {
            Arc::new(CompilationUnit::build_with_analysis_input(
                &format!(
                    "proc kept {{x}} {{set value $x; return $value}}\nproc other {{}} {{{other}}}\n"
                ),
                crate::compilation_unit::UnitBuildOptions {
                    registry: context.commands(),
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                Some(
                    &crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input),
                ),
                &input,
            ))
        };
        let left = build("return BEFORE");
        let right = build("return AFTER");
        let select = |unit: &CompilationUnit| {
            *unit
                .ir_module
                .original_native_declaration_body_units
                .iter()
                .find(|(_, card)| card.original_name_input().bytes() == b"kept")
                .unwrap()
                .0
        };
        let left_id = select(&left);
        let right_id = select(&right);
        assert_ne!(left_id, right_id);
        let a =
            BodyAnalysisCapsule::retain(Arc::clone(&left), left_id, context.commands()).unwrap();
        let b =
            BodyAnalysisCapsule::retain(Arc::clone(&right), right_id, context.commands()).unwrap();
        assert!(a.has_closed_analysis_correspondence());
        assert!(a.has_closed_readonly_source_correspondence());
        assert!(!a.has_closed_native_correspondence());
        assert_eq!(a, b);
        assert!(std::ptr::eq(b.unit(), right.as_ref()));
        assert_eq!(b.body_id(), right_id);
        assert!(std::ptr::eq(
            b.function(),
            right.function_for_body(Some(right_id)).unwrap()
        ));
        assert!(
            BodyAnalysisCapsule::retain(Arc::clone(&right), left_id, context.commands()).is_none()
        );
        let mut missing_input = (*right).clone();
        missing_input.ir_module.source_metadata_input = None;
        assert!(
            BodyAnalysisCapsule::retain(Arc::new(missing_input), right_id, context.commands())
                .is_none()
        );
    }

    #[test]
    fn native_source_lattice_certificate_refuses_genuine_dependency_and_owner_changes() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let foreign_context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let foreign_profile = foreign_context.commands().profile().unwrap();
        let foreign_input = crate::analyser::ResolvedAnalysisInput::new(
            foreign_profile,
            foreign_profile,
            Arc::clone(&foreign_context),
            tcl_lexer::LexerConfig::for_file_grammar(foreign_profile.grammar),
        );
        let foreign_store = CompilationUnit::build_with_analysis_input(
            "proc a {x} {return FOREIGN}\n",
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
            &input,
        );
        let foreign_script = foreign_store
            .ir_module
            .body_units
            .values()
            .next()
            .unwrap()
            .body
            .clone();
        let mut visited = 0;
        let mut memo = |req: &LatticeRequest<'_>| {
            visited += 1;
            let source_card = req
                .module
                .original_native_declaration_body_units
                .get(&req.body_id)
                .unwrap();
            assert!(
                source_card.matches_procedure(
                    req.module,
                    req.module.body_units.get(&req.body_id).unwrap()
                )
            );
            if req.body_source == Some("set value $x; return $value") {
                assert!(BodyLatticeInputs::retain(req, context.commands()).is_some());
                let foreign_source_input = LatticeRequest {
                    source_metadata_input: Some(&foreign_input),
                    ..*req
                };
                assert!(
                    BodyLatticeInputs::retain(&foreign_source_input, context.commands()).is_none()
                );
                assert!(BodyLatticeInputs::retain(req, foreign_context.commands()).is_none());
                let mut changed_store = req.module.clone();
                changed_store.body_units.get_mut(&req.body_id).unwrap().body =
                    foreign_script.clone();
                let foreign_store_request = LatticeRequest {
                    module: &changed_store,
                    ..*req
                };
                assert!(
                    BodyLatticeInputs::retain(&foreign_store_request, context.commands()).is_none()
                );
            } else {
                assert!(
                    BodyLatticeInputs::retain(req, context.commands()).is_none(),
                    "{}",
                    req.body_source.unwrap()
                );
            }
            build_current(req, context.commands())
        };
        let unavailable_body_cache =
            |_: &str, _: &str| -> crate::ir::Script { panic!("source-only body cache") };
        let source = "proc a {x} {set value $x; return $value}\nproc store {x} {upvar 0 outside local; return $local}\nproc observer {x} {trace add execution return enter hook; return $x}\nproc tracing {x} {trace add variable x read hook; return $x}\nproc document {x} {helper $x}\n";
        let unit = CompilationUnit::build_memoized_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            &mut memo,
            &unavailable_body_cache,
            None,
            &input,
        );
        drop(memo);
        assert_eq!(visited, 5);
        assert_eq!(
            unit.ir_module.original_native_declaration_body_units.len(),
            5
        );
        let logical_profile = tcl_dialect::DialectProfile::plain_tcl();
        let logical_input = crate::analyser::ResolvedAnalysisInput::new(
            logical_profile,
            logical_profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(logical_profile.grammar),
        );
        let mut logical_seen = 0;
        let mut logical_memo = |req: &LatticeRequest<'_>| {
            logical_seen += 1;
            let input = BodyLatticeInputs::retain(req, context.commands()).unwrap();
            assert!(input.certified_native_source_body_id().is_none());
            assert!(
                req.command_bindings
                    .prepare_native_source_body_correspondence(
                        req.body,
                        crate::command_binding::BodyProofScope {
                            source: req.source,
                            body_source: req.body_source.unwrap(),
                            original_body_offset: req.original_body_offset,
                            executable_body_offset: req.executable_body_offset,
                            metadata:
                                crate::registry_invocation::InvocationMetadataInput::SuppliedSource(
                                    req.source_metadata_input
                                ),
                            config: req.lexer_config,
                        },
                        req.module,
                        req.body_id,
                        context.commands()
                    )
                    .is_none()
            );
            build_current(req, context.commands())
        };
        let _ = CompilationUnit::build_memoized_with_analysis_input(
            "proc logical {x} {return $x}\n",
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: logical_input.lexer_config(),
                dialect: Some(logical_profile),
                external_call_sites: None,
                declared_commands: None,
            },
            &mut logical_memo,
            &unavailable_body_cache,
            None,
            &logical_input,
        );
        drop(logical_memo);
        assert_eq!(logical_seen, 1);
    }

    #[test]
    fn logical_named_source_retains_raw_lattice_without_native_publication() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let mut visited = 0;
        let mut memo = |request: &LatticeRequest<'_>| {
            visited += 1;
            let procedure = request.original_source_procedure().unwrap();
            assert_eq!(procedure.body_kind, crate::ir::ProcedureBodyKind::Named);
            assert!(procedure.publication.is_none());
            assert!(
                !request
                    .module
                    .original_logical_declaration_body_units
                    .contains_key(&request.body_id)
            );
            let raw = BodyLatticeInputs::retain(request, context.commands()).unwrap();
            assert!(raw.command_bindings().is_none());
            assert!(raw.certified_native_source_body_id().is_none());
            assert!(raw.certificate.is_none());
            let missing = LatticeRequest {
                source_metadata_input: None,
                ..*request
            };
            assert!(BodyLatticeInputs::retain(&missing, context.commands()).is_none());
            let mut changed = request.module.clone();
            changed
                .procedures
                .get_mut(&request.body_id)
                .unwrap()
                .params
                .push("foreign".to_owned());
            let altered = LatticeRequest {
                module: &changed,
                ..*request
            };
            assert!(BodyLatticeInputs::retain(&altered, context.commands()).is_none());
            build_current(request, context.commands())
        };
        let unavailable_body_cache =
            |_: &str, _: &str| -> crate::ir::Script { panic!("source-only body cache") };
        let _unit = CompilationUnit::build_memoized_with_analysis_input(
            "proc logical {x} {return $x}\n",
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            &mut memo,
            &unavailable_body_cache,
            None,
            &input,
        );
        drop(memo);
        assert_eq!(visited, 1);
    }

    #[test]
    fn authored_source_capsules_keep_independent_standalone_analysis_purpose() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let options = crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        };
        let build = |other: &str| {
            Arc::new(CompilationUnit::build_with_options(
                &format!(
                    "proc kept {{x}} {{set value $x; return $value}}\nproc other {{}} {{return {other}}}\n"
                ),
                options,
            ))
        };
        let left = build("BEFORE");
        let right = build("AFTER");
        let select = |unit: &CompilationUnit| {
            *unit
                .ir_module
                .original_authored_declaration_body_units
                .iter()
                .find(|(_, card)| card.name() == "kept")
                .unwrap()
                .0
        };
        let a = BodyAnalysisCapsule::retain(Arc::clone(&left), select(&left), context.commands())
            .unwrap();
        let b = BodyAnalysisCapsule::retain(Arc::clone(&right), select(&right), context.commands())
            .unwrap();
        assert!(left.ir_module.source_entry.metadata_context.is_standalone());
        assert!(left.ir_module.source_metadata_input.is_none());
        assert!(left.ir_module.source_entry.native_entry.is_none());
        assert!(a.has_closed_analysis_correspondence());
        assert!(a.has_closed_readonly_source_correspondence());
        assert!(!a.has_closed_native_correspondence());
        assert!(
            a == b,
            "same certified standalone body after unrelated source edit"
        );
        let foreign = CommandRegistry::build_default();
        assert!(BodyAnalysisCapsule::retain(Arc::clone(&left), select(&left), &foreign).is_none());
        let mut missing = (*left).clone();
        missing
            .ir_module
            .original_authored_declaration_body_units
            .clear();
        assert!(
            BodyAnalysisCapsule::retain(Arc::new(missing), select(&left), context.commands())
                .is_none()
        );
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let supplied = Arc::new(CompilationUnit::build_with_analysis_input(
            "proc kept {x} {set value $x; return $value}",
            options,
            Some(&crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)),
            &input,
        ));
        assert!(
            supplied
                .ir_module
                .original_authored_declaration_body_units
                .is_empty()
        );
        assert!(
            !supplied
                .ir_module
                .original_native_declaration_body_units
                .is_empty()
        );
    }

    #[test]
    fn authored_source_lattice_certificate_rebinds_actual_ids_and_refuses_missing_owner() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let options = crate::compilation_unit::UnitBuildOptions {
            registry: context.commands(),
            config,
            dialect: Some(profile),
            defer_top_level: false,
            external_call_sites: None,
            declared_commands: None,
        };
        let mut previous: Option<(BodyLatticeInputs, FunctionUnit)> = None;
        let mut computations = 0;
        let mut ids = Vec::new();
        let mut memo = |request: &LatticeRequest<'_>| {
            assert!(request.module.source_entry.metadata_context.is_standalone());
            let inputs = BodyLatticeInputs::retain(request, context.commands())
                .expect("authentic standalone scalar source dependencies");
            assert_eq!(
                inputs.certified_readonly_source_body_id(),
                Some(request.body_id)
            );
            assert!(inputs.certified_native_source_body_id().is_none());
            let mut missing = request.module.clone();
            missing.original_authored_declaration_body_units.clear();
            assert!(
                BodyLatticeInputs::retain(
                    &LatticeRequest {
                        module: &missing,
                        ..*request
                    },
                    context.commands()
                )
                .is_none()
            );
            let mut wrong_config = LatticeRequest { ..*request };
            wrong_config.lexer_config.strict_quoting = !wrong_config.lexer_config.strict_quoting;
            assert!(BodyLatticeInputs::retain(&wrong_config, context.commands()).is_none());
            if request.qname == "::kept" {
                ids.push(request.body_id);
                if let Some((old, cached)) = &previous {
                    assert!(
                        &inputs == old,
                        "same complete standalone source dependencies"
                    );
                    let rebound = inputs
                        .rebind_cached(old, cached)
                        .expect("same typed source analysis, current issued owner");
                    assert_eq!(rebound.body_id, Some(request.body_id));
                    assert!(Arc::ptr_eq(&rebound.def_use, &cached.def_use));
                    assert!(rebound.cfg.executed_source.is_none());
                    assert!(rebound.cfg.native_compilation_admission.is_none());
                    return rebound;
                }
                computations += 1;
                let current = build_current(request, context.commands());
                previous = Some((inputs, current.clone()));
                current
            } else {
                build_current(request, context.commands())
            }
        };
        for source in [
            "proc kept {x} {set value $x; return $value}\nproc other {} {return BEFORE}\n",
            "\nproc kept {x} {set value $x; return $value}\nproc other {} {return AFTER}\n",
        ] {
            let unit = CompilationUnit::build_for_memoized(source, options, &mut memo);
            assert!(unit.ir_module.source_metadata_input.is_none());
            assert!(unit.ir_module.source_entry.native_entry.is_none());
        }
        drop(memo);
        assert_eq!(computations, 1);
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
    }

    #[test]
    fn native_source_lattice_correspondence_rebinds_actual_ids_after_an_unrelated_edit() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = context.commands().profile().unwrap();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let mut retained: Option<(BodyLatticeInputs, FunctionUnit)> = None;
        let mut a_computations = 0;
        let mut a_ids = Vec::new();
        let mut memo = |req: &LatticeRequest<'_>| {
            assert!(req.module.source_entry.native_entry.is_none());
            assert!(
                req.module
                    .original_native_declaration_body_units
                    .contains_key(&req.body_id)
            );
            let inputs = BodyLatticeInputs::retain(req, context.commands())
                .expect("actual Native source dependency certificate");
            assert_eq!(inputs.certified_native_source_body_id(), Some(req.body_id));
            let mut unavailable = req.module.clone();
            unavailable.source_metadata_input = None;
            let missing = LatticeRequest {
                module: &unavailable,
                ..*req
            };
            assert!(BodyLatticeInputs::retain(&missing, context.commands()).is_none());
            if req.qname == "::a" {
                a_ids.push(req.body_id);
                if let Some((previous, cached)) = &retained {
                    assert_eq!(inputs, *previous);
                    let rebound = inputs
                        .rebind_cached(previous, cached)
                        .expect("current genuine source restoration");
                    assert_eq!(rebound.body_id, Some(req.body_id));
                    assert!(Arc::ptr_eq(&rebound.def_use, &cached.def_use));
                    return rebound;
                }
                a_computations += 1;
                let computed = build_current(req, context.commands());
                retained = Some((inputs, computed.clone()));
                computed
            } else {
                build_current(req, context.commands())
            }
        };
        let unavailable_body_cache = |_: &str, _: &str| -> crate::ir::Script {
            panic!("source-only declarations cannot enter the executable body cache")
        };
        for source in [
            "proc a {x} {set y $x; return $y}\nproc b {} {return TWO}\n",
            "\nproc a {x} {set y $x; return $y}\nproc b {} {return THREE}\n",
        ] {
            let unit = CompilationUnit::build_memoized_with_analysis_input(
                source,
                crate::compilation_unit::UnitBuildOptions {
                    registry: context.commands(),
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                &mut memo,
                &unavailable_body_cache,
                None,
                &input,
            );
            assert_eq!(
                unit.ir_module.original_native_declaration_body_units.len(),
                2
            );
            assert!(
                unit.ir_module
                    .original_native_declaration_body_units
                    .keys()
                    .all(|id| unit.function_for_body(Some(*id)).is_some())
            );
        }
        drop(memo);
        assert_eq!(a_computations, 1);
        assert_eq!(a_ids.len(), 2);
        assert_ne!(a_ids[0], a_ids[1]);
    }
}

/// One actual body taint-worker result and its complete capsule dependency.
/// Reporting output remains available independently of source issuance.
#[derive(Debug, Clone)]
pub struct OriginalInterproceduralTaintReport {
    capsule: BodyAnalysisCapsule,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    taints: Arc<std::collections::HashMap<crate::ssa::ValueKey, crate::taint::TaintLattice>>,
    issued: bool,
    analysis_context: Option<crate::value_transfer::AnalysisContextKey>,
}
impl PartialEq for OriginalInterproceduralTaintReport {
    fn eq(&self, other: &Self) -> bool {
        self.capsule == other.capsule
            && self.dialect == other.dialect
            && self.taints == other.taints
            && self.issued == other.issued
            && self.analysis_context == other.analysis_context
    }
}
impl OriginalInterproceduralTaintReport {
    /// Actual immutable worker output; this projection supplies no edit grant.
    #[must_use]
    pub fn taints(
        &self,
    ) -> &Arc<std::collections::HashMap<crate::ssa::ValueKey, crate::taint::TaintLattice>> {
        &self.taints
    }
    pub(crate) fn matches_current(
        &self,
        capsule: &BodyAnalysisCapsule,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> bool {
        self.issued
            && self.dialect == dialect
            && self.capsule == *capsule
            && self.analysis_context.is_some()
            && self.analysis_context
                == capsule.original_optimiser_analysis_context(registry, dialect)
    }
}
impl BodyAnalysisCapsule {
    /// Run the existing body taint worker once with its actual retained summary.
    /// Missing summary/source keeps the legacy baseline reporting projection
    /// unavailable for source issuance; it is never repaired by a current map.
    #[must_use]
    pub fn original_interproc_taint_report(
        &self,
        registry: &CommandRegistry,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> OriginalInterproceduralTaintReport {
        let computed = self.unit.interproc.as_ref().and_then(|analysis| {
            self.unit
                .original_interproc_taints_for_body(Some(self.body), registry, analysis)
        });
        let analysis_context = self.original_optimiser_analysis_context(registry, dialect);
        let issued = analysis_context.is_some()
            && computed.is_some()
            && self.registry == registry.snapshot()
            && self
                .unit
                .original_optimiser_prepared_unit(registry, dialect)
                .is_some();
        let taints = Arc::new(computed.unwrap_or_else(|| (*self.function().taints).clone()));
        OriginalInterproceduralTaintReport {
            capsule: self.clone(),
            dialect,
            taints,
            issued,
            analysis_context,
        }
    }
}
