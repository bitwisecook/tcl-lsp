// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual source requests and complete original function-lattice workers.

use super::{FunctionLatticeInputs, FunctionUnit, LatticeRequest, ModuleTraceFacts, UnitDialect};
use crate::body_analysis_capsule::{BodyLatticeCommandBindings, BodyLatticeInputs};
use crate::cfg_builder::global_write_info::GlobalWriteInfo;
use crate::cfg_builder::upvar_info::UpvarInfo;
use crate::registry_invocation::OwnedInvocationMetadataContext;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// Data consumed by the original CFG and function-lattice workers. Constructing
/// this view supplies no provenance; the retained original request checks every
/// field before running or issuing a result.
#[derive(Debug)]
pub struct OriginalFunctionLatticeInputs<'a> {
    /// Actual requested qualified name.
    pub qname: &'a str,
    /// Complete original parameter vector.
    pub params: &'a [String],
    /// Exact prepared caller-frame summaries.
    pub upvar_procs: &'a HashMap<crate::ir::ProcedureId, UpvarInfo>,
    /// Exact prepared procedure parameters.
    pub proc_params: &'a HashMap<crate::ir::ProcedureId, Vec<String>>,
    /// Exact prepared outer-place summaries.
    pub global_write_procs: &'a HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
    /// Actual original or certified neutral command bindings.
    pub command_bindings: &'a BodyLatticeCommandBindings,
    /// Exact original procedure-effect owners.
    pub procedure_summary_owners: &'a crate::cfg_builder::OriginalProcedureSummaryOwners,
    /// Complete offset-zero lexer policy.
    pub lexer_config: tcl_lexer::LexerConfig,
    /// Actual selected optional profile, distinct from the dialect label.
    pub profile: Option<tcl_dialect::DialectProfileKey>,
    /// Actual memo dialect spelling; empty corresponds only to no profile.
    pub dialect: &'a str,
    /// Exact encoded caller-uniform literal seeds.
    pub param_constants: &'a [(String, u32, String)],
    /// Complete original class roster consumed by the worker.
    pub known_classes: &'a [String],
    /// Complete original variable-trace subjects.
    pub traced_variables: &'a [String],
    /// Original dynamic variable-trace obligation.
    pub has_dynamic_variable_trace: bool,
    /// Actual explicit original-declaration body role, if any.
    pub original_declaration_body: Option<crate::ir::ProcedureId>,
    /// Actual runtime-dispatch mode.
    pub plain_command_dispatch: bool,
    /// Exact metadata ingress, including unavailable supplied input.
    pub metadata_context: &'a OwnedInvocationMetadataContext,
    /// Full retained analysis input, independent of dialect labels.
    pub source_metadata_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    /// Original event descriptor; ordinary procedures retain absence.
    pub event_body: Option<&'a Arc<crate::ir::SourceIrulesEventBody>>,
    /// Actual complete command mutation obligations.
    pub command_trust: &'a crate::command_binding::CommandTrustSnapshot,
    /// Actual complete value-transfer context.
    pub analysis_context: &'a crate::value_transfer::AnalysisContextKey,
}

#[derive(Debug, Clone, PartialEq)]
struct WorkerContext {
    upvar_procs: HashMap<crate::ir::ProcedureId, UpvarInfo>,
    proc_params: HashMap<crate::ir::ProcedureId, Vec<String>>,
    global_write_procs: HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
    command_bindings: BodyLatticeCommandBindings,
    procedure_summary_owners: crate::cfg_builder::OriginalProcedureSummaryOwners,
}
#[derive(Debug)]
pub(super) struct OriginalLatticeRequestScope {
    module: Arc<crate::ir::Module>,
    context: Arc<WorkerContext>,
}
impl OriginalLatticeRequestScope {
    pub(super) fn capture(
        module: &crate::ir::Module,
        context: &crate::cfg_builder::CfgContext,
    ) -> Self {
        Self {
            module: Arc::new(module.clone()),
            context: Arc::new(WorkerContext {
                upvar_procs: context.0.clone(),
                proc_params: context.1.clone(),
                global_write_procs: context.2.clone(),
                command_bindings: BodyLatticeCommandBindings::from(context.3.clone()),
                procedure_summary_owners: context.4.clone(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct WorkerInputs {
    qname: String,
    params: Vec<String>,
    context: Arc<WorkerContext>,
    lexer_config: tcl_lexer::LexerConfig,
    profile: Option<tcl_dialect::DialectProfileKey>,
    dialect: String,
    param_constants: Vec<(String, u32, String)>,
    known_classes: Vec<String>,
    traced_variables: Vec<String>,
    has_dynamic_variable_trace: bool,
    original_declaration_body: Option<crate::ir::ProcedureId>,
    plain_command_dispatch: bool,
    metadata_context: OwnedInvocationMetadataContext,
    source_metadata_input: Option<crate::analyser::ResolvedAnalysisInput>,
    event_body: Option<Arc<crate::ir::SourceIrulesEventBody>>,
    command_trust: crate::command_binding::CommandTrustSnapshot,
    analysis_context: crate::value_transfer::AnalysisContextKey,
}
impl WorkerInputs {
    fn matches_request(&self, request: &LatticeRequest<'_>, registry: &CommandRegistry) -> bool {
        self.qname == request.qname
            && self.params == request.params
            && &self.context.upvar_procs == request.upvar_procs
            && &self.context.proc_params == request.proc_params
            && &self.context.global_write_procs == request.global_write_procs
            && self.context.command_bindings.bindings() == request.command_bindings
            && &self.context.procedure_summary_owners == request.procedure_summary_owners
            && self.lexer_config == request.lexer_config
            && self.profile == request.dialect.map(tcl_dialect::DialectProfile::cache_key)
            && self.dialect == request.dialect.map_or("", |profile| profile.name)
            && self.param_constants == request.param_constants
            && self.known_classes == request.known_classes
            && self.traced_variables == request.traced_variables
            && self.has_dynamic_variable_trace == request.has_dynamic_variable_trace
            && self.original_declaration_body
                == request
                    .original_source_procedure()
                    .filter(|procedure| {
                        procedure.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration
                    })
                    .map(|_| request.body_id)
            && self.plain_command_dispatch == request.plain_command_dispatch
            && self.metadata_context == request.original_metadata_context(registry)
            && self.source_metadata_input.as_ref() == request.source_metadata_input
            && self.event_body.as_ref() == request.irules_event_body
            && self.command_trust == request.command_trust.snapshot()
            && &self.analysis_context == request.analysis_context
    }

    fn matches(&self, input: &OriginalFunctionLatticeInputs<'_>) -> bool {
        self.qname == input.qname
            && self.params == input.params
            && &self.context.upvar_procs == input.upvar_procs
            && &self.context.proc_params == input.proc_params
            && &self.context.global_write_procs == input.global_write_procs
            && &self.context.command_bindings == input.command_bindings
            && &self.context.procedure_summary_owners == input.procedure_summary_owners
            && self.lexer_config == input.lexer_config
            && self.profile == input.profile
            && self.dialect == input.dialect
            && self.param_constants == input.param_constants
            && self.known_classes == input.known_classes
            && self.traced_variables == input.traced_variables
            && self.has_dynamic_variable_trace == input.has_dynamic_variable_trace
            && self.original_declaration_body == input.original_declaration_body
            && self.plain_command_dispatch == input.plain_command_dispatch
            && &self.metadata_context == input.metadata_context
            && self.source_metadata_input.as_ref() == input.source_metadata_input
            && self.event_body.as_ref() == input.event_body
            && &self.command_trust == input.command_trust
            && &self.analysis_context == input.analysis_context
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OriginalLatticeRequestSeal {
    module: Arc<crate::ir::Module>,
    body_id: crate::ir::ProcedureId,
    body: crate::ir::Script,
    original_body_offset: u32,
    executable_body_offset: u32,
    body_source: Option<String>,
    inputs: WorkerInputs,
    registry: tcl_registry::RegistrySnapshot,
}
impl OriginalLatticeRequestSeal {
    pub(super) fn capture(
        request: &LatticeRequest<'_>,
        scope: &OriginalLatticeRequestScope,
        registry: &CommandRegistry,
    ) -> Self {
        Self {
            module: Arc::clone(&scope.module),
            body_id: request.body_id,
            body: request.body.clone(),
            original_body_offset: request.original_body_offset,
            executable_body_offset: request.executable_body_offset,
            body_source: request.body_source.map(str::to_owned),
            inputs: request.worker_inputs(registry, Arc::clone(&scope.context)),
            registry: registry.snapshot(),
        }
    }
    fn matches(&self, request: &LatticeRequest<'_>, registry: &CommandRegistry) -> bool {
        self.module.as_ref() == request.module
            && self.body_id == request.body_id
            && self.body == *request.body
            && self.module.source.try_text().ok() == Some(request.source)
            && self.original_body_offset == request.original_body_offset
            && self.executable_body_offset == request.executable_body_offset
            && self.body_source.as_deref() == request.body_source
            && self.inputs.matches_request(request, registry)
            && self.registry == registry.snapshot()
    }
}

impl LatticeRequest<'_> {
    fn worker_inputs(
        &self,
        registry: &CommandRegistry,
        context: Arc<WorkerContext>,
    ) -> WorkerInputs {
        WorkerInputs {
            qname: self.qname.to_owned(),
            params: self.params.to_vec(),
            context,
            lexer_config: self.lexer_config,
            profile: self.dialect.map(tcl_dialect::DialectProfile::cache_key),
            dialect: self.dialect.map_or("", |profile| profile.name).to_owned(),
            param_constants: self.param_constants.to_vec(),
            known_classes: self.known_classes.to_vec(),
            traced_variables: self.traced_variables.to_vec(),
            has_dynamic_variable_trace: self.has_dynamic_variable_trace,
            original_declaration_body: self
                .original_source_procedure()
                .filter(|procedure| {
                    procedure.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration
                })
                .map(|_| self.body_id),
            plain_command_dispatch: self.plain_command_dispatch,
            metadata_context: self.original_metadata_context(registry),
            source_metadata_input: self.source_metadata_input.cloned(),
            event_body: self.irules_event_body.cloned(),
            command_trust: self.command_trust.snapshot(),
            analysis_context: self.analysis_context.clone(),
        }
    }
    fn original_worker_seal(
        &self,
        registry: &CommandRegistry,
    ) -> Option<Arc<OriginalLatticeRequestSeal>> {
        let original = self.original_worker_request?;
        original
            .matches(self, registry)
            .then(|| Arc::new(original.clone()))
    }
    pub(super) fn accepts_original_worker_result(
        &self,
        result: &FunctionUnit,
        registry: &CommandRegistry,
    ) -> bool {
        let Some(original) = result.original_lattice_worker.as_ref() else {
            return false;
        };
        original.matches_result(result)
            && original.request.matches(self, registry)
            && self.original_worker_seal(registry).is_some()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OriginalLatticeWorkerRecipe {
    request: Arc<OriginalLatticeRequestSeal>,
    body: BodyLatticeInputs,
    inputs: WorkerInputs,
}
#[derive(Debug, Clone)]
pub(crate) struct OriginalFunctionLatticeResult {
    recipe: Arc<OriginalLatticeWorkerRecipe>,
    request: Arc<OriginalLatticeRequestSeal>,
    result: FunctionUnit,
    semantic_value_owner: bool,
}
impl OriginalFunctionLatticeResult {
    fn matches_result(&self, result: &FunctionUnit) -> bool {
        self.result == *result
            && self.semantic_value_owner == result.has_original_semantic_value_owner()
    }
    fn retain(
        result: &mut FunctionUnit,
        recipe: Arc<OriginalLatticeWorkerRecipe>,
        request: Arc<OriginalLatticeRequestSeal>,
    ) {
        let mut snapshot = result.clone();
        snapshot.original_lattice_worker = None;
        snapshot.original_lattice_callback_admission = None;
        result.original_lattice_worker = Some(Arc::new(Self {
            recipe,
            request,
            semantic_value_owner: snapshot.has_original_semantic_value_owner(),
            result: snapshot,
        }));
    }
}

impl BodyLatticeInputs {
    pub(crate) fn retain_original_worker_recipe(
        &mut self,
        request: &LatticeRequest<'_>,
        registry: &CommandRegistry,
        neutral: bool,
    ) {
        let Some(original) = request.original_worker_seal(registry) else {
            return;
        };
        if self.body() != request.body {
            return;
        }
        let mut inputs = original.inputs.clone();
        if neutral {
            let Some(bindings) = self.command_bindings() else {
                return;
            };
            inputs.context = Arc::new(WorkerContext {
                upvar_procs: HashMap::new(),
                proc_params: HashMap::new(),
                global_write_procs: HashMap::new(),
                command_bindings: bindings,
                procedure_summary_owners:
                    crate::cfg_builder::OriginalProcedureSummaryOwners::default(),
            });
        }
        if self.certified_readonly_source_body_id().is_some() {
            inputs.original_declaration_body = None;
        }
        let mut body = self.clone();
        body.original_worker_recipe = None;
        self.original_worker_recipe = Some(Arc::new(OriginalLatticeWorkerRecipe {
            request: original,
            body,
            inputs,
        }));
    }
    /// Retain the exact source-issued fallback request without inventing neutral
    /// correspondence or admitting a caller-constructed Script.
    #[must_use]
    pub fn retain_original_worker_request(
        request: &LatticeRequest<'_>,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        request.original_worker_seal(registry)?;
        let mut body = Self::from(request.body.clone());
        body.retain_original_worker_recipe(request, registry, false);
        Some(body)
    }
    /// Run the existing complete CFG/function/semantic workers only after every
    /// supplied key field rejoins this genuine source-issued request recipe.
    #[must_use]
    pub fn original_function_lattice(
        &self,
        input: &OriginalFunctionLatticeInputs<'_>,
        registry: &CommandRegistry,
    ) -> Option<FunctionUnit> {
        let recipe = self.original_worker_recipe.as_ref()?;
        if !recipe.body.original_worker_body_matches(self)
            || !recipe.inputs.matches(input)
            || recipe.request.registry != registry.snapshot()
        {
            return None;
        }
        let prepared = crate::cfg_builder::PreparedCfgContext::from_metadata_input(
            (
                input.upvar_procs.clone(),
                input.proc_params.clone(),
                input.global_write_procs.clone(),
                input.command_bindings.bindings().clone(),
                input.procedure_summary_owners.clone(),
            ),
            registry,
            crate::registry_invocation::InvocationMetadataInput::Retained(input.metadata_context),
        );
        let cfg = crate::cfg_builder::build_cfg_function_with_prepared_context(
            input.qname,
            self.body(),
            true,
            registry,
            input.plain_command_dispatch,
            &prepared,
            input.lexer_config,
        );
        let param_constants = super::decode_param_constants(input.param_constants);
        let known_classes: HashSet<_> = input.known_classes.iter().cloned().collect();
        let traced_variables: BTreeSet<_> = input.traced_variables.iter().cloned().collect();
        let command_trust = input.command_trust.to_mutations();
        let inputs = FunctionLatticeInputs {
            dialect: UnitDialect {
                registry,
                config: input.lexer_config,
                source_metadata_input: input.source_metadata_input,
            },
            param_constants: param_constants.as_ref(),
            known_classes: &known_classes,
            trace_facts: ModuleTraceFacts {
                traced_variables: &traced_variables,
                has_dynamic_variable_trace: input.has_dynamic_variable_trace,
                deferred_writes: &input.analysis_context.deferred_writes,
            },
            analysis_context: input.analysis_context,
            command_trust: &command_trust,
            event_body: input.event_body.map(|event| (event, self.body())),
        };
        let mut result = if let Some(body) = input
            .original_declaration_body
            .or_else(|| self.certified_readonly_source_body_id())
        {
            FunctionUnit::build_for_original_declaration_lattice(
                body,
                input.qname,
                cfg,
                input.params,
                inputs,
            )
        } else {
            FunctionUnit::build_for_lattice(input.qname, cfg, input.params, inputs)
        }
        .with_retained_semantic_analysis(
            registry,
            Some(self.body()),
            crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
        );
        OriginalFunctionLatticeResult::retain(
            &mut result,
            Arc::clone(recipe),
            Arc::clone(&recipe.request),
        );
        Some(result)
    }
    pub(crate) fn rebind_original_worker_result(
        &self,
        original: &Self,
        cached: &FunctionUnit,
        rebound: &mut FunctionUnit,
    ) -> bool {
        let Some(current) = self.original_worker_recipe.as_ref() else {
            return false;
        };
        let Some(previous) = original.original_worker_recipe.as_ref() else {
            return false;
        };
        let Some(result) = cached.original_lattice_worker.as_ref() else {
            return false;
        };
        if !result.matches_result(cached)
            || result.recipe.inputs != previous.inputs
            || result.recipe.body != previous.body
            || previous.inputs != current.inputs
            || previous.body != current.body
        {
            return false;
        }
        OriginalFunctionLatticeResult::retain(
            rebound,
            Arc::clone(current),
            Arc::clone(&current.request),
        );
        true
    }
}

impl FunctionUnit {
    pub(crate) fn clear_original_lattice_worker(&mut self) {
        self.original_lattice_worker = None;
    }
}

/// Actual native body-template projection of one source-issued lattice request.
/// The existing template owner retains its original physical relocation; this
/// projection does not accept caller-constructed Script or template fields.
#[derive(Debug, Clone)]
pub struct OriginalFunctionLatticeTemplate {
    body: BodyLatticeInputs,
    relocation: crate::var_resolve::VariableProofRelocation,
}
impl OriginalFunctionLatticeTemplate {
    /// Exact transformed body retained by the genuine template producer.
    #[must_use]
    pub const fn body(&self) -> &BodyLatticeInputs {
        &self.body
    }
    /// Exact original template command bindings used by the CFG worker.
    #[must_use]
    pub fn command_bindings(&self) -> BodyLatticeCommandBindings {
        self.body
            .original_worker_recipe
            .as_ref()
            .expect("retained template recipe")
            .inputs
            .context
            .command_bindings
            .clone()
    }
    /// Restore the existing cached report through the actual original native
    /// relocation and source proofs. Only an unchanged genuine worker result
    /// retains an original result receipt; unsupported reports stay unissued.
    #[must_use]
    pub fn rebind_cached(&self, cached: &FunctionUnit) -> Option<FunctionUnit> {
        let recipe = self.body.original_worker_recipe.as_ref()?;
        let genuine = cached
            .original_lattice_worker
            .as_ref()
            .is_some_and(|original| {
                original.matches_result(cached)
                    && original.recipe.body == recipe.body
                    && original.recipe.inputs == recipe.inputs
            });
        let inverse = self.relocation.inverse()?;
        let proofs = crate::command_binding::BodySourceProofs::from_body(&recipe.request.body);
        let mut current = cached
            .relocated_variable_proofs(&inverse)?
            .restored_source_proofs(&proofs)?;
        if genuine {
            OriginalFunctionLatticeResult::retain(
                &mut current,
                Arc::clone(recipe),
                Arc::clone(&recipe.request),
            );
        } else {
            current.clear_original_lattice_worker();
        }
        Some(current)
    }
}
impl BodyLatticeInputs {
    /// Prepare the existing actual native body template once, retaining the
    /// source-issued request and original relocation for typed memo reborrow.
    #[must_use]
    pub fn retain_original_worker_template(
        request: &LatticeRequest<'_>,
        registry: &CommandRegistry,
    ) -> Option<OriginalFunctionLatticeTemplate> {
        let original = request.original_worker_seal(registry)?;
        let metadata = request.original_metadata_context(registry);
        let template = request.command_bindings.prepare_native_body_template(
            request.body,
            crate::command_binding::BodyProofScope {
                source: request.source,
                body_source: request.body_source?,
                original_body_offset: request.original_body_offset,
                executable_body_offset: request.executable_body_offset,
                metadata: crate::registry_invocation::InvocationMetadataInput::Retained(&metadata),
                config: request.lexer_config,
            },
            registry,
        )?;
        let mut inputs = original.inputs.clone();
        inputs.context = Arc::new(WorkerContext {
            upvar_procs: HashMap::new(),
            proc_params: HashMap::new(),
            global_write_procs: HashMap::new(),
            command_bindings: BodyLatticeCommandBindings::from(template.command_bindings),
            procedure_summary_owners: crate::cfg_builder::OriginalProcedureSummaryOwners::default(),
        });
        let mut body = Self::from(template.body);
        body.original_worker_recipe = Some(Arc::new(OriginalLatticeWorkerRecipe {
            request: original,
            body: body.clone(),
            inputs,
        }));
        Some(OriginalFunctionLatticeTemplate {
            body,
            relocation: template.variable_relocation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

    fn descriptor<'a>(
        request: &'a LatticeRequest<'_>,
        bindings: &'a BodyLatticeCommandBindings,
        metadata: &'a OwnedInvocationMetadataContext,
        trust: &'a crate::command_binding::CommandTrustSnapshot,
    ) -> OriginalFunctionLatticeInputs<'a> {
        OriginalFunctionLatticeInputs {
            qname: request.qname,
            params: request.params,
            upvar_procs: request.upvar_procs,
            proc_params: request.proc_params,
            global_write_procs: request.global_write_procs,
            command_bindings: bindings,
            procedure_summary_owners: request.procedure_summary_owners,
            lexer_config: request.lexer_config,
            profile: request.dialect.map(tcl_dialect::DialectProfile::cache_key),
            dialect: request.dialect.map_or("", |profile| profile.name),
            param_constants: request.param_constants,
            known_classes: request.known_classes,
            traced_variables: request.traced_variables,
            has_dynamic_variable_trace: request.has_dynamic_variable_trace,
            original_declaration_body: request
                .original_source_procedure()
                .filter(|procedure| {
                    procedure.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration
                })
                .map(|_| request.body_id),
            plain_command_dispatch: request.plain_command_dispatch,
            metadata_context: metadata,
            source_metadata_input: request.source_metadata_input,
            event_body: request.irules_event_body,
            command_trust: trust,
            analysis_context: request.analysis_context,
        }
    }

    // Keep the actual set alive in the caller across all later lattice queries.
    fn assert_distinct_body_purpose_hashing<T: Clone + Eq + std::hash::Hash>(
        legacy: T,
        body: &T,
    ) -> impl Sized {
        let mut purposes = HashSet::new();
        assert!(purposes.insert(legacy.clone()));
        assert!(purposes.insert(body.clone()));
        assert!(!purposes.insert(body.clone()));
        assert!(!purposes.insert(legacy));
        purposes
    }

    #[test]
    fn original_lattice_worker_keeps_actual_recipe_and_refuses_changed_key_or_request() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = context.commands();
        let source = "proc p {value} {set copy $value; return $copy}";
        let mut visits = 0;
        let mut callback = |request: &LatticeRequest<'_>| {
            visits += 1;
            let body = BodyLatticeInputs::retain_original_worker_request(request, registry)
                .expect("actual original source request");
            let legacy = BodyLatticeInputs::from(request.body.clone());
            assert_ne!(legacy, body);
            let _purposes = assert_distinct_body_purpose_hashing(legacy, &body);
            let bindings = BodyLatticeCommandBindings::from(request.command_bindings.clone());
            let metadata = request.original_metadata_context(registry);
            let trust = request.command_trust.snapshot();
            let mut input = descriptor(request, &bindings, &metadata, &trust);
            let actual = body
                .original_function_lattice(&input, registry)
                .expect("actual complete original lattice worker");
            assert!(request.accepts_original_worker_result(&actual, registry));
            let original_profile = input.profile;
            input.profile = Some(
                tcl_dialect::DialectProfile::find("tcl9.1")
                    .unwrap()
                    .cache_key(),
            );
            assert!(body.original_function_lattice(&input, registry).is_none());
            input.profile = original_profile;
            let seeds = vec![("value".to_owned(), 0, "foreign".to_owned())];
            input.param_constants = &seeds;
            assert!(body.original_function_lattice(&input, registry).is_none());
            let changed_request = LatticeRequest {
                param_constants: &seeds,
                ..*request
            };
            assert!(
                BodyLatticeInputs::retain_original_worker_request(&changed_request, registry)
                    .is_none()
            );
            let mut changed_module = request.module.clone();
            changed_module.has_dynamic_variable_trace = !changed_module.has_dynamic_variable_trace;
            let changed_request = LatticeRequest {
                module: &changed_module,
                ..*request
            };
            assert!(
                BodyLatticeInputs::retain_original_worker_request(&changed_request, registry)
                    .is_none()
            );
            assert!(
                BodyLatticeInputs::from(request.body.clone())
                    .original_function_lattice(
                        &descriptor(request, &bindings, &metadata, &trust),
                        registry
                    )
                    .is_none()
            );
            actual
        };
        let unit = CompilationUnit::build_for_memoized(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
                dialect: registry.profile(),
                external_call_sites: None,
                declared_commands: None,
            },
            &mut callback,
        );
        assert!(visits > 0);
        assert!(unit.original_lattice_callbacks_are_current());
    }

    #[test]
    fn original_lattice_worker_refuses_public_result_changes_without_changing_report_acceptance() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = context.commands();
        let mut visits = Vec::new();
        let mut callback = |request: &LatticeRequest<'_>| {
            visits.push(request.body_id);
            let body =
                BodyLatticeInputs::retain_original_worker_request(request, registry).unwrap();
            let bindings = BodyLatticeCommandBindings::from(request.command_bindings.clone());
            let metadata = request.original_metadata_context(registry);
            let trust = request.command_trust.snapshot();
            let mut actual = body
                .original_function_lattice(
                    &descriptor(request, &bindings, &metadata, &trust),
                    registry,
                )
                .unwrap();
            assert!(request.accepts_original_worker_result(&actual, registry));
            let cfg = actual.cfg.clone();
            actual.complexity_guarded = !actual.complexity_guarded;
            assert_eq!(actual.cfg, cfg);
            assert!(!request.accepts_original_worker_result(&actual, registry));
            actual
        };
        let unit = CompilationUnit::build_for_memoized(
            "proc p {value} {return $value}",
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
                dialect: registry.profile(),
                external_call_sites: None,
                declared_commands: None,
            },
            &mut callback,
        );
        assert!(!visits.is_empty());
        assert!(!unit.original_lattice_callbacks_are_current());
        assert!(visits.iter().all(|body| {
            unit.function_for_body(Some(*body))
                .expect("the same actual analysed body")
                .complexity_guarded
        }));
    }
}
