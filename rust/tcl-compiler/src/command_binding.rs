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

//! Flow-sensitive command-binding lattice.
//!
//! Owns both command-binding views used by compiler consumers. The
//! flow-sensitive CFG lattice tracks what each command *name* resolves to at
//! every program point — its original builtin, a user proc, an `interp alias`,
//! or an opaque target. [`ModuleCommandBindings`] is the richer whole-module
//! may-state: it retains every live alias target and its prepended arguments so
//! effect summaries can resolve stored bodies independent of definition order.
//! Both consume registry-owned transitions; no effect consumer interprets a
//! command-table mutation itself.
//!
//! Consumers: the W128 diagnostic ("call to a command renamed/deleted
//! earlier in this file") in `analyser`, and — via the flow-insensitive
//! whole-module summary [`scan_module_command_mutations`] — the
//! optimiser's builtin-fold trust gate.
//!
//! Predecessors come from [`CfgFunction::block_successors`], the canonical
//! successor view shared by CFG analyses.  This includes analysis-only `try`
//! exception edges, so a handler conservatively joins command mutations that
//! may have occurred before control transfers to it.

#[cfg(test)]
use std::collections::HashSet;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::cfg::BlockId;
use crate::cfg::Function as CfgFunction;
use crate::ir::{Module, Script, Statement};
use crate::naming::normalise_qualified_name as nqn;
use tcl_registry::{
    CommandBindingTransition, CommandRegistry, EffectiveRegistrySemantics, InvocationFacts,
    StateTransition,
};
#[cfg(test)]
use tcl_registry::{NamespaceTransition, TransitionSubject};

mod called_static_layouts;
mod closed_class_configuration;
mod declared_frame_view;
mod default_manufacture_advice;
mod deferred_body_index;
mod deferred_namespace_index;
mod executed_expression_source;
mod executed_script_source;
mod frozen_arguments;
mod future_bodies;
mod hosted_procedure_body;
mod incomplete_body_metadata;
mod jim_empty_iteration;
mod jim_stock_class;
mod jim_wrapper_body;
pub(crate) use jim_stock_class::OriginalJimStockName;
mod logical_definition;
pub use hosted_procedure_body::OriginalHostedProcedureBody;
mod logical_operation;
mod logical_prefix;
mod logical_word_values;
mod materialized_footprint;
mod native_list_assignment;
mod original_callback_lookup;
mod original_compiler_effects;
mod original_missing_command;
mod original_source_projection;
pub(crate) use original_missing_command::OriginalMissingCommandContext;
mod point_extents;
mod pre_handler_failure;
mod root_layout_extents;
mod root_owner_work;
mod source_arguments;
mod source_name_lookup;
mod source_unavailability_advice;
pub use default_manufacture_advice::OriginalDefaultManufactureShape;
pub use original_callback_lookup::OriginalCallbackPrefix;
mod insertion_template;
mod literal_word_script;
mod original_command_mutation;
mod original_command_table;
mod original_delegated_script;
mod original_namespace_binding;
mod original_namespace_body;
pub(crate) mod original_namespace_deletion;
pub(crate) mod original_namespace_ensure;
mod original_namespace_path;
mod own_object_configuration;
pub use insertion_template::OriginalScalarAssignmentCommand;
mod original_class_configuration;
pub use original_class_configuration::OriginalSourceClassConfigurationTarget;
pub use original_class_configuration::OriginalSourceObjectConfigurationTarget;
pub use source_name_lookup::OriginalCommandLookup;
mod source_command_world;
#[cfg(debug_assertions)]
pub(crate) mod source_world_observer;
pub use future_bodies::{SourceFutureBodyInventory, SourceFutureCallSite};
pub use source_command_world::{
    OriginalClassInstanceMethodRoster, OriginalCommandPublication, OriginalCommandPublicationKind,
    OriginalCompletedCommandWorld,
};
mod operand_layout;
mod retained_source;
pub use retained_source::RetainedSourceModuleBindings;
pub(crate) use retained_source::source_analysis_frame;
pub(crate) use retained_source::{FreshNativeSourceAnalysis, SuppliedSourceAnalysisIssuance};
#[cfg(test)]
mod semantic_equality;
mod shared_inventory;
pub use operand_layout::SourceNativeOperandLayoutProof;
use shared_inventory::SharedSourceInventory;
mod conditional_body;
mod declaration_flow;
mod source_declared_command;
mod source_declared_method_template;
pub(crate) use declaration_flow::DeclarationFlowReport;
pub(crate) use declaration_flow::representation::SourceConversionObservation;
pub(crate) use source_declared_command::{DeclaredSourceSelection, OriginalLogicalLambdaBody};
pub use source_declared_command::{
    OriginalDeclaredCommandWords, OriginalDeclaredLogicalBodyContext,
    OriginalDeclaredSourceBodyFrame, OriginalDeclaredSourceObligation,
};
mod declaration_body_ownership;
mod declaration_layout;
mod logical_variable_scope;
pub(crate) use declaration_layout::OriginalBodyParameterSourceScope;
pub use declaration_layout::SourceOriginalVariableFrame;
pub(crate) use logical_variable_scope::OriginalLogicalVariableScope;
mod declaration_preview;
mod source_read_scope;
pub(crate) use declaration_preview::{
    OriginalAuthoredProcedureSourceGeometry, OriginalProcedureReadDeclaration,
    OriginalProcedureStaticReadAlternatives,
};
pub(crate) use source_read_scope::{SourceReadScope, source_targets_match_advice};
mod scalar_body_alpha;
pub use scalar_body_alpha::{OriginalLocalStoreAlphaRename, OriginalScalarBodyAlphaRename};
mod vendor_catalogue_advice;
pub use vendor_catalogue_advice::{
    VendorCatalogueSourceCandidate, VendorCatalogueSourceObligation,
};
mod source_catalogue_advice;
mod source_transition_advice;
pub use source_catalogue_advice::{
    OriginalCatalogueSourceCandidate, OriginalCatalogueSourceObligation,
};
pub(crate) use source_transition_advice::LogicalSourceClassRelationProvider;
pub(crate) use source_transition_advice::OriginalSourceProcedureDeclarationValues;
pub use source_transition_advice::{
    LogicalExternalClassMemberSelector, OriginalLogicalAbsoluteCommandCandidate,
    OriginalLogicalCallbackCandidates, OriginalLogicalCommandCandidates,
    OriginalLogicalMathFunctionOccurrence, OriginalLogicalNameReference,
    OriginalLogicalProcedureBody, OriginalLogicalProcedureCall, OriginalLogicalSourceNameInput,
    OriginalSourceAuthoredCommandPrefix, OriginalSourceCallbackProcedureLookup,
    OriginalSourceCallbackProcedureRefusal, OriginalSourceCallbackProcedureTarget,
    OriginalSourceCallbackProcedureTargetKind, OriginalSourceCallbackRegistration,
    OriginalSourceClassCandidate, OriginalSourceClassDeclaration, OriginalSourceClassHandleBinding,
    OriginalSourceClassInstance, OriginalSourceClassInstanceWords, OriginalSourceClassProcCall,
    OriginalSourceClassPublication, OriginalSourceClassPublications, OriginalSourceClassReference,
    OriginalSourceCommandTransition, OriginalSourceCommandTransitionAdvice,
    OriginalSourceConfiguredClassReference, OriginalSourceConstructorBodyAttempt,
    OriginalSourceConstructorBodyCoverage, OriginalSourceConstructorBodyDisposition,
    OriginalSourceConstructorCall, OriginalSourceConstructorShape,
    OriginalSourceInterpreterHandleBody, OriginalSourceInterpreterPathBinding,
    OriginalSourceProcedureCandidate, OriginalSourceProcedurePublication,
    OriginalSourceProcedurePublications, OriginalSourceProducedCommandPrefix,
    OriginalSourceRegisteredHandleBinding, OriginalSourceRegisteredInstance,
    OriginalSourceRegisteredInstanceWords, SourceAdviceNameInput,
    SourceCommandTransitionObligation,
};
pub use source_transition_advice::{
    LogicalSourceClassRelationCandidates, LogicalSourceClassRelationTarget,
    OriginalSourceClassRelationProvider, OriginalSourcePublicationProposal,
};
pub use source_transition_advice::{
    OriginalAuthoredJimClass, OriginalConditionalLambdaFormattingBody,
};
pub use source_transition_advice::{
    OriginalAuthoredJimClassCall, OriginalAuthoredJimProcedureMember,
};
pub use source_transition_advice::{
    OriginalNativeDeferredAutoloadAdvice, OriginalNativeDeferredAutoloadObligation,
    OriginalNativeProcedureBodyCommand, OriginalSourceProcedureCall,
    source_native_deferred_autoload_advice_at,
};
pub(crate) use source_transition_advice::{
    OriginalNativeSourceBodyChildren, OriginalSourceCataloguePurpose,
};
pub(crate) use source_transition_advice::{
    OriginalSourceTransitionAdviceLookup, OriginalSourceTransitionAdviceTape,
};
pub(crate) use source_transition_advice::{
    original_single_command_substitution_words, original_single_command_substitution_words_in_plan,
};
mod original_literal_argv_rewrite;
pub use original_literal_argv_rewrite::{
    OriginalExpressionGapCompaction, OriginalLiteralExpressionBracing,
};
pub(crate) use original_literal_argv_rewrite::{
    configured_expression_syntax_gaps, hosted_operator_rewrite_holds,
};
mod expression_operand_advice;
mod formal_call_advice;
mod lifecycle_advice;
mod trusted_package_transfer;
pub(crate) use expression_preparation::SourceConditionalExpressionEvaluation;
mod original_math_function;
pub use original_math_function::OriginalMathFunctionOccurrence;
pub(crate) use original_math_function::OriginalMathFunctionWordMemo;
mod math_occurrence;
pub use math_occurrence::{SourceConditionalMathInvocation, SourceMathInvocation};
#[doc(hidden)]
pub mod formal_topology;
mod native_compiler_formals;
pub(crate) use native_compiler_formals::{
    OriginalNativeCompilerFormalScope, OriginalNativeCompilerFormals,
};
mod formal_value;
mod origin_inventory;
#[doc(hidden)]
pub use origin_inventory::OriginalInvocationSourcePurpose;
mod retained_body_inventory;
#[doc(hidden)]
pub use retained_body_inventory::RetainedSourceBodyInventory;
mod original_body_namespace;
pub(crate) use original_body_namespace::original_logical_frame_body_namespace_for_module;
#[cfg(test)]
mod original_restoration_obligation_tests;
#[cfg(test)]
mod original_restored_nested_declaration_tests;
#[cfg(test)]
mod original_script_parse_entry_tests;
pub(crate) use conditional_body::SourceConditionalBodyEntry;
pub use conditional_body::{SourceCallerFrameInvocationTemplate, SourceDeclaredReceiverBodyEntry};
pub use source_declared_method_template::OriginalDeclaredSelfMethodTemplate;
mod source_file;
mod substitution_template;
pub use executed_expression_source::ExecutedExpressionSource;
pub use executed_script_source::{ExecutedScriptMapping, ExecutedScriptSource};
mod analysis_body_correspondence;
pub(crate) use analysis_body_correspondence::AnalyticalBodyCorrespondence;
mod allocation_relocation;
pub use allocation_relocation::{for_each_script_allocation_site_mut, relocate_script};
mod body_template;
mod re_execution;
pub use body_template::{BodyProofScope, BodySourceProofs, NativeBodyTemplate};
pub use re_execution::{ReExecutionDecline, re_executable_body};
mod command_publication;
mod compiled_invocation;
mod ensemble_compilation;
mod ensemble_dispatch;
use ensemble_dispatch::EnsembleDispatches;
mod named_arguments;
mod named_invocation;
pub use named_invocation::SourceNamedInvocationProof;
mod argument_reads;
mod read_store_schedule;
pub use read_store_schedule::SourceReadStoreObservation;
mod method_prefix;
pub use method_prefix::SourceCapturedMethodPrefix;
mod array_destruction;
mod array_iteration;
mod command_mutations;
mod command_observers;
#[cfg(test)]
pub(crate) use command_observers::{max_observer_depth, reset_observer_depth};
mod command_reference;
mod declaration_lookup;
mod linked_definition;
pub use command_reference::{
    SourceCommandDefinition, SourceCommandDefinitionKind, SourceCommandReference,
    SourceCommandReferenceBinding,
};
mod command_presence;
mod definition_method_references;
pub use definition_method_references::SourceDefinitionMethodReference;
mod definition_reference_inventory;
mod generated_procedures;
pub use command_presence::SourceCommandSlotPresence;
pub use generated_procedures::{
    OriginalNativeProcedureBody, SourceInstalledProcedureBody, SourceProcedureImplementationBody,
};
#[doc(hidden)]
pub mod compiled_preflight;
#[doc(hidden)]
pub mod compiler_inventory;
pub(crate) use compiler_inventory::OriginalCompilerArgumentSyntax;
mod variable_access_owners;
mod variable_observers;
mod variable_outputs;
pub use compiled_preflight::{
    SourceNativeCompilationContext, SourceNativeCompilationDependency,
    SourceNativeCompilationFailure,
};
pub use compiler_inventory::SourceRuntimeReachability;
mod case_bodies;
#[cfg(test)]
mod conditional_expression_tests;
mod constructor_execution;
mod deferred_forward;
mod deferred_method;
mod dictionary_scope;
mod evaluated_word;
mod expression_preparation;
mod interpreter_manufacture;
mod keyed_update_store;
#[cfg(test)]
mod logical_dispatch_purpose_tests;
pub(crate) mod named_manufacture;
mod native_class_factory_roles;
mod original_alias_creation;
mod original_class_factory_transfer;
pub(crate) mod original_receiver_body_context;
mod receiver_self;
pub(crate) mod receiver_variable_inventory;
mod registered_class_factory;
pub use receiver_self::SourceReceiverBuiltinCandidates;
mod rule_declaration;
pub use rule_declaration::SourceRuleDeclarationCandidate;
mod try_bodies;
pub use expression_preparation::{SourceExpressionPreparation, SourceExpressionScriptCompilation};
mod invocation_reads;
mod literal_object_pool;
pub use literal_object_pool::SourceStockLiteralObject;
mod normal_result;
mod normal_variable_continuation;
mod original_variable_inventory;
pub use normal_result::{SourceNormalResult, SourceScopedProcedureEvaluation};
pub(crate) mod object_callbacks;
pub use invocation_reads::{
    SourceInvocationVariableRead, SourceInvocationVariableReads, SourceVariableReadResidual,
};
mod container_coercion;
mod native_result;
mod native_string_access;
pub use native_string_access::SourceNativeStringAccess;
mod numeric_operand_cache;
mod object_instance;
pub(crate) mod original_name_value;
pub(crate) use conditional_source::OriginalSourceCutBodyEntryProjection;
pub(crate) use declaration_preview::OriginalSourceCutBodyRows;
pub(crate) use original_name_value::OriginalBodyWrittenWordOwner;
pub(crate) mod original_variable_compilation;
mod package_target_survey;
pub(crate) use package_target_survey::OriginalPackageIndexSourceModel;
mod conditional_source;
pub(crate) use conditional_source::OriginalSourceFiniteCut;
mod availability_report;
mod source_analysis_cache;
mod source_analysis_entry;
mod source_execution_control;
mod source_point_entry;
pub use conditional_source::{
    OriginalClosedSourceCutPublication, OriginalSingleSourceCutSurvey,
    OriginalSourceConditionalContinuation, OriginalSourceConditionalOutcome,
    OriginalSourceConditionalRegion, OriginalSourceConditionalRegionSet,
};
mod source_argument_syntax;
pub use source_analysis_entry::{
    SourceAnalysisEntry, SourceAnalysisOptions, native_entry_config, native_entry_options,
};
pub(crate) use source_analysis_entry::{SourceDeclaredCommandContracts, source_input_dialect};
pub use source_point_entry::{
    OriginalSourcePointEntry, OriginalSourcePointObligation, OriginalSourcePointOrigin,
};
mod source_loop;
mod source_representation;
pub use deferred_method::{
    SourceMethodReceiver, SourceReceiverMethodEntries, SourceReceiverMethodEntry,
};
pub use object_instance::{SourceObjectAllocation, SourceObjectInstanceProof};
mod cfg_lookup_context;
mod namespace_context;
use cfg_lookup_context::CfgLookupContexts;
mod namespace_slots;
pub(crate) use namespace_context::NamespaceKeyQuery;
#[doc(hidden)]
pub use namespace_context::SourceCommandKey;
pub use namespace_context::SourceNamespaceKey;
use namespace_context::{
    CommandKeyQuery, SourceCommandTable, SourceNamespaceMap, SourceNamespaceSet,
};
mod native_variable_tables;
#[doc(hidden)]
pub mod runtime_entry;
mod snit_definition;
pub mod trace_registrations;
pub use compiled_invocation::{
    CompiledExecutionCertainty, CompiledExecutionResidual, NativeCompilationSnapshot,
    NativeCompilerTargets, NativeInlineRejection, SourceCompiledInvocationProof,
    SourceNativeNamespaceBindingPreparation, SourceNativeStructuredPreparation,
};

/// The lattice element a command name resolves to.
///
/// Height-3 join lattice: [`BindingKind::Bottom`] (⊥) is the identity,
/// a concrete binding joined with itself is unchanged, and two
/// *different* bindings rise to [`BindingKind::Unknown`] (⊤).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BindingKind {
    /// ⊥ — identity for join (no contribution yet).
    Bottom,
    /// The original core/registry command, unperturbed.
    Builtin,
    /// A user procedure (`target` = its canonical qname).
    Proc,
    /// A concrete registry-described command whose narrower identity is not
    /// relevant to this lattice.
    Command,
    /// A `TclOO`/snit/itcl class or instance command created by a
    /// registry-described definer (`target` = its canonical qname).
    /// Distinct from [`Self::Proc`] so `NAME destroy` — the universal
    /// object method — is only modelled as a deletion for names that
    /// actually denote objects.
    Class,
    /// An instance of a registry object class (a Tk widget's path), created
    /// by the constructor command `factory`. A call through it dispatches
    /// that class's declared instance methods; it is never a `TclOO` class,
    /// and its methods include no `destroy` deletion.
    Instance {
        /// The constructor's canonical registry command.
        factory: &'static str,
    },
    /// An `interp alias` (`target` = the alias target name).
    Alias,
    /// Renamed/deleted-away or never-defined → dispatches to `unknown`.
    Opaque,
    /// ⊤ — conflicting bindings at a merge, or dynamic mutation.
    Unknown,
}

impl BindingKind {
    /// The binding a registry definition installs before any instance
    /// refinement: an instance is first published as an object command.
    #[must_use]
    pub const fn of_definition(kind: tcl_registry::CommandBindingDefinitionKind) -> Self {
        match kind {
            tcl_registry::CommandBindingDefinitionKind::Procedure => Self::Proc,
            tcl_registry::CommandBindingDefinitionKind::Object
            | tcl_registry::CommandBindingDefinitionKind::Instance => Self::Class,
            tcl_registry::CommandBindingDefinitionKind::Command => Self::Command,
        }
    }
}

/// A command-name binding: its [`BindingKind`] plus an optional target
/// (the proc qname for [`BindingKind::Proc`], the alias target for
/// [`BindingKind::Alias`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The kind of binding.
    pub kind: BindingKind,
    /// Target name for `Proc` / `Alias` bindings; `None` otherwise.
    pub target: Option<String>,
}

/// One terminal command target selected by the module-wide may-binding
/// resolver. `prepended` contains every literal argument contributed by an
/// `interp alias` chain, in invocation order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(hidden)]
pub struct ResolvedCommandTarget {
    pub(crate) command: String,
    pub(crate) prepended: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    /// Whether `command` denotes the registry descriptor of that spelling.
    /// A retained user procedure is an exact call-graph target but must not
    /// inherit a same-named builtin's registry semantics.
    pub(crate) registry_backed: bool,
    kind: BindingKind,
    implementation_generation: u32,
    implementation_allocation: Option<CommandAllocation>,
    terminal: bool,
    target_lookup: tcl_registry::AliasTargetLookup,
    token: Option<CommandToken>,
}

/// Memo sidecars never participate in source Eq/Hash or allocation identity.
/// Entries retain only a weak actual owner; dead owners are retired on access.
struct AllocationMemoTable<O, M> {
    entries: HashMap<usize, (Weak<O>, Arc<M>)>,
}
impl<O, M> Default for AllocationMemoTable<O, M> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}
impl<O, M: Default> AllocationMemoTable<O, M> {
    fn get_or_init(&mut self, owner: &Arc<O>) -> Arc<M> {
        self.entries
            .retain(|_, (owner, _)| owner.strong_count() != 0);
        let key = Arc::as_ptr(owner) as usize;
        if let Some((retained, memo)) = self.entries.get(&key)
            && retained
                .upgrade()
                .is_some_and(|retained| Arc::ptr_eq(&retained, owner))
        {
            return Arc::clone(memo);
        }
        let memo = Arc::new(M::default());
        self.entries
            .insert(key, (Arc::downgrade(owner), Arc::clone(&memo)));
        memo
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(hidden)]
pub enum MayBinding {
    Target(ResolvedCommandTarget),
    Imported(ImportedCommandBinding),
    /// The binding is absent on this path, so Tcl falls back from a local name
    /// to the global spelling (or reports `unknown` at the root).
    Missing,
    /// A command exists, but its implementation cannot be named statically.
    Unknown,
}

/// Callable origin and independently retained compiler token of an import.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(hidden)]
pub struct ImportedCommandBinding {
    origin: CommandToken,
    compiler: Option<CommandToken>,
}

impl From<CommandToken> for ImportedCommandBinding {
    fn from(origin: CommandToken) -> Self {
        Self {
            origin,
            compiler: None,
        }
    }
}

fn is_repeated_fresh_binding(binding: &MayBinding) -> bool {
    matches!(binding, MayBinding::Target(target) if target.implementation_allocation.as_ref().is_some_and(|allocation| allocation.incarnation == AllocationIncarnation::RepeatedFresh))
}

/// Immutable registry facts shared by every may-state in one analysis.
///
/// A command-binding walk forks and joins at every structured statement. The
/// fresh-interpreter command set and unresolved-command handlers never change
/// during that walk, so carrying them in each [`ModuleCommandBindings`] clone
/// made every branch and join copy the whole registry universe. Keep that
/// baseline behind one [`Arc`] and let the lattice state stay sparse.
#[derive(Debug, Clone, Default)]
struct BindingBaseline {
    source_point_entry: Option<OriginalSourcePointEntry>,
    metadata_context: crate::registry_invocation::OwnedInvocationMetadataContext,
    /// Explicit module-declared frame effects, independent of native cells.
    declared: Arc<crate::ir::DeclaredFrameEffects>,
    logical_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    source_read_config: crate::var_resolve::SourceReadConfiguration,
    vendor_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    hosted_execution_context: Option<tcl_registry::f5::BigIpExecutionContext>,
    execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    compiled_variable_provider:
        Option<tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider>,
    expression_parser: Option<tcl_registry::invocation_words::LogicalExpressionParseProvider>,
    invocation_realm: tcl_dialect::model::InvocationRealm,
    unknown_entry: bool,
    host_extensible_command_table: bool,
    native_entry: Option<Arc<tcl_runtime_api::NativeCompilationEntry>>,
    native_compilation: tcl_registry::native_compilation::NativeCompilationContext,
    fingerprint: u64,
    registry_snapshot: Option<tcl_registry::RegistrySemanticKey>,
    registry_owner: Option<tcl_registry::RegistrySnapshot>,
    declared_commands: Arc<BTreeMap<String, String>>,
    declared_surface: Option<Arc<source_analysis_entry::SourceDeclaredCommandContracts>>,
    catalogue_commands: Arc<BTreeMap<String, String>>,
    /// Registry-owned, generation-specific facts shared with CFG construction.
    semantics: Arc<EffectiveRegistrySemantics>,
    /// Derived fixed descriptor projection, excluded from equality and hashing.
    metadata_slots: OnceLock<Option<original_command_table::RegistryMetadataSlotIndex>>,
    /// Derived descriptor geometry keyed by the actual immutable map and policy.
    advice_slots: Arc<original_command_table::RegistryAdviceSlotMemo>,
    import_binding: Option<tcl_dialect::NamespaceImportBinding>,
    trusted_loaders: BTreeMap<String, TrustedPackageLoader>,
    trusted_source_modules: Vec<TrustedSourceModuleLoader>,
    release: Option<tcl_dialect::TclVersion>,
    dialect: Option<tcl_registry::InvocationDialect>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PackageProvision {
    present: bool,
    version: Option<String>,
    implementation: Option<Arc<InstalledPackageProvider>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct InstalledPackageProvider {
    id: String,
    /// Rooted descriptor identities attested by this particular loader.
    commands: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ClassDefinitionReceipt {
    factory: String,
    original_factory: Option<registered_class_factory::OriginalClassFactoryState>,
    implementation_generation: u32,
    dispatcher: Option<tcl_registry::definer::DefinitionDispatcher>,
    dispatcher_methods: Arc<BTreeSet<String>>,
    instance_methods: Option<Arc<BTreeSet<tcl_core_types::NameBytes>>>,
    instance_variables: Option<Arc<receiver_variable_inventory::OriginalReceiverVariableInventory>>,
    constructor_entry: Option<Arc<SourceConstructorEntry>>,
    constructor_provider: Option<SourceCommandTarget>,
    destructor_entry: Option<Arc<SourceConstructorEntry>>,
    destructor_provider: Option<SourceCommandTarget>,
    lifecycle_entries_closed: bool,
    receiver_method_entries: Arc<SourceReceiverMethodEntries>,
    forward_method_entries: Arc<deferred_forward::OriginalForwardEntries>,
    inherited_classes: Arc<Vec<SourceCommandTarget>>,
    single_superclass: bool,
    /// The user metaclass whose inherited stock creation made this class
    /// (`Meta create NAME SCRIPT`); `None` for a class a registry factory made.
    custom_metaclass: Option<SourceCommandTarget>,
}

/// Original constructor entry retained at a class incarnation. This authorizes
/// argument-to-formal advice only; it proves no successful manufacture result.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceConstructorEntry {
    declaration: CommandAllocationSite,
    body: ExecutedScriptSource,
    formals: Vec<(String, Option<String>)>,
    frame: crate::var_resolve::VariableExecutionFrame,
}

/// What a runtime receiver's private namespace may hold beyond the commands
/// Tcl creates there.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum ReceiverNamespaceContents {
    /// Only the commands Tcl creates in a fresh object namespace.
    #[default]
    Created,
    /// A member body may also have published commands there.
    Unproven,
}

/// A resolved source invocation without discarding absence or an unknown
/// residual. Catalogue identity alone does not establish loader provenance.
#[derive(Clone, PartialEq, Eq)]
pub struct SourceInvocationBinding {
    /// Native opcodes selected before the later runtime command lookup.
    pub compiled_candidates: Vec<SourceCompiledInvocationProof>,
    /// Private names selected at compilation, with their independent late lookup.
    pub compiled_named_candidates: Vec<SourceNamedInvocationProof>,
    /// At least one execution uses ordinary lookup after argument evaluation.
    pub may_use_live_dispatch: bool,
    /// The native compiler may have selected an operation with no closed descriptor.
    pub compiled_execution_residual: CompiledExecutionResidual,
    /// Closed compiler-table selection, independent of later runtime lookup.
    pub(crate) native_compilation_admission:
        Option<tcl_registry::native_compilation::NativeCompilationSelection>,
    pub(crate) native_compiler_admission:
        Option<Arc<compiled_invocation::SourceNativeCompilerAdmission>>,
    /// Independent name-effect closure of the original compiler traversal.
    original_compiler_preserves_names: SourceCompilerNameClosure,
    /// Original ordered compiler preparation, including a declined prefix.
    native_structured_preparation: Option<Arc<SourceNativeStructuredPreparation>>,
    native_switch_preparation: Option<Arc<compiled_invocation::SourceNativeSwitchPreparation>>,
    runtime_reachability: SourceRuntimeReachability,
    entered_execution_observer: SourceObserverExecution,
    /// Compiler-inline withdrawal selected by the shared chunk preflight.
    pub native_inline_rejection: NativeInlineRejection,
    /// Same immutable native handler before and after argv, independent of opcode selection.
    pub native_handler_envelope: Option<SourceCommandTarget>,
    /// Reads reached after this invocation's argv, with explicit unenumerated effects.
    pub invocation_variable_reads: Option<Arc<SourceInvocationVariableReads>>,
    pub(crate) lookup_state: Option<Arc<SourceLookupSnapshot>>,
    pub(crate) compiler_lookup_state: Option<Arc<SourceLookupSnapshot>>,
    original_logical_source_advice: Option<Arc<OriginalSourceCommandTransitionAdvice>>,
    original_analysis_owner: Option<Arc<OriginalSourceAnalysisOwner>>,
    original_source_operand_attachment:
        Option<declaration_preview::OriginalDeclarationSourceOperandAttachment>,
    original_authored_source_body_scope:
        Option<Arc<crate::registry_invocation::OriginalAuthoredSourceBodyScope>>,
    original_native_source_command:
        Option<Arc<source_transition_advice::OriginalNativeSourceCommand>>,
    declaration_layout_observations:
        Option<Arc<[declaration_layout::DeclarationLayoutObservation]>>,
    declaration_flow_inventory: Option<Arc<declaration_flow::DeclarationFlowInventory>>,
    /// Original declaration availability when this source has no dispatch point.
    /// The private source receipt never supplies runtime targets or lookup state.
    original_logical_declaration_targets:
        Option<Arc<declaration_layout::OriginalLogicalDeclarationTargets>>,
    compiler_policy: Option<Arc<compiler_inventory::SourceNativeCompilerPolicy>>,
    original_compiler_words: Option<Arc<compiled_invocation::SourceOriginalCompilerWords>>,
    original_written_words: Option<Arc<original_name_value::OriginalWrittenSourceWords>>,
    original_argument_syntax: Option<Arc<source_argument_syntax::OriginalSourceArgumentSyntax>>,
    conditional_expression_evaluations:
        Option<Arc<[expression_preparation::SourceConditionalExpressionEvaluation]>>,
    /// Whether a member body of the document may have published commands
    /// into a runtime receiver's private namespace, ahead of every lookup.
    receiver_namespace: ReceiverNamespaceContents,
    #[doc(hidden)]
    pub lookup_namespace: String,
    pub(crate) lookup_namespace_key: SourceNamespaceKey,
    pub(crate) lookup_word: Option<String>,
    dispatch_site: Option<CommandAllocationSite>,
    /// Providers whose private callback state may have been altered.
    pub provider_state_taint: Vec<String>,
    /// Applicable declaration assistance; never executable dispatch proof.
    pub declared_command: Option<DeclaredCommandCandidate>,
    /// Applicable nominal catalogue slot, including an unloaded provider.
    /// This carries assistance only and never establishes an implementation.
    pub catalogue_command: Option<DeclaredCommandCandidate>,
    /// Authoritative variable bindings, traces, lifetimes and allocation facts.
    pub variable_context: Arc<crate::var_resolve::ResolveContext>,
    original_argument_completion: OriginalArgumentCompletion,
    normal_variable_continuation: Option<Arc<crate::var_resolve::ResolveContext>>,
    original_variable_operands: Option<Arc<crate::variable_bindings::OriginalVariableInvocation>>,
    normal_result_observations: Option<Arc<[normal_result::InvocationResultObservation]>>,
    rhs_read_store_observations: Option<Arc<[SourceReadStoreObservation]>>,
    object_callback_effects: Option<object_callbacks::ObjectCallbackEffects>,
    /// Actual variable activation selected for this invocation.
    pub variable_frame: crate::var_resolve::VariableExecutionFrame,
    /// Namespace cells allocated on every reaching execution path.
    pub existing_namespace_cells: Vec<crate::var_resolve::VariableCellKey>,
    /// Complete allocation knowledge, retaining possible cells and closure.
    pub namespace_cell_presence: crate::var_resolve::NamespaceCellPresence,
    /// Namespace objects proved to exist on every path to this dispatch.
    pub known_namespaces: Arc<[String]>,
    /// Written argv values frozen left-to-right, excluding the command word.
    /// Unknown results and expansion retain `None`; aliases add their own
    /// separate prefixes and never change these original word indices.
    pub evaluated_argument_values: Arc<[Option<String>]>,
    /// Argument facts frozen at their own evaluation points, including unknown indices.
    pub evaluated_argument_words: Arc<[crate::registry_invocation::EffectiveInvocationWord]>,
    frozen_written_words: Option<Arc<[crate::registry_invocation::EffectiveInvocationWord]>>,
    frozen_written_names:
        Option<Arc<[Option<Arc<original_name_value::OriginalProducedNameValue>>]>>,
    frozen_head_object: Option<Arc<receiver_self::FrozenSourceObjectHead>>,
    method_prefix_arguments: Vec<(usize, Arc<SourceCapturedMethodPrefix>)>,
    native_operand_layout: Option<Arc<SourceNativeOperandLayoutProof>>,
    /// Terminal implementations and effective alias argument prefixes.
    pub targets: Vec<SourceCommandTarget>,
    /// Lookup may fail on an execution path.
    pub may_be_absent: bool,
    /// An execution path has no bounded implementation identity.
    pub unknown: bool,
}

/// One terminal implementation selected by the shared command-table owner.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceCommandTarget {
    /// Initial runtime implementation incarnation, independent of source offsets.
    pub runtime_implementation_generation: Option<u64>,
    /// Captured registry implementation identity or retained document target.
    /// For registry-backed targets this preserves the issuer's identifier,
    /// which need not be rooted or match the current command-table slot.
    /// It is not a source lookup spelling or namespace presentation.
    pub command: String,
    /// Arguments inserted before the written argument sequence.
    pub prepended: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    /// Original producers of captured prefix values, joined at every actual
    /// alias allocation. They supply no written operand or compiler geometry.
    original_prepended: Option<Vec<crate::signature_scan::scope::SignatureSourceNameInput>>,
    logical_original_prefix: Option<logical_prefix::LogicalCapturedCommandPrefix>,
    /// Only registry-backed implementations may select registry semantics.
    pub registry_backed: bool,
    /// Registry, procedure, object, or generic implementation category.
    pub kind: BindingKind,
    /// Stable object identity, independent of its present command slot.
    pub identity: Option<CommandIdentity>,
    /// Source implementation site; changes when a live token is redefined.
    pub implementation_generation: u32,
    /// Source-instance allocation of the retained implementation.
    pub implementation_allocation: Option<CommandAllocation>,
}

/// Conditional candidates selected from an original retained lookup world.
/// Compiler, declaration and Logical source callers validate separate purposes.
/// This layout cannot be used as an execution binding or a Normal result.
#[derive(Debug)]
pub(crate) struct OriginalCompilationLookupAdvice {
    snapshot: Arc<SourceLookupSnapshot>,
    site: CommandAllocationSite,
    words: Arc<[crate::ir::WordExpr]>,
    config: tcl_lexer::LexerConfig,
    targets: Vec<SourceCommandTarget>,
    dialect: tcl_registry::InvocationDialect,
    namespace: SourceNamespaceKey,
    unknown: bool,
    may_be_absent: bool,
}

/// Immutable semantic point. Ordinary lookup retains the original full world
/// without hashing it. Hashing initializes a derived fingerprint once;
/// equality compares the complete retained state and realm directly, allowing
/// immutable shared fields to use their existing structural equality.
/// A reached state join invalidates the memo on its detached snapshot only.
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct SourceLookupSnapshot {
    realm: tcl_dialect::model::InvocationRealm,
    #[doc(hidden)]
    pub state: ModuleCommandBindings,
    fingerprint: OnceLock<u64>,
    compiler_name_effects: original_compiler_effects::SourceCompilerNameEffectsCache,
    layout_projections: layout_projection_memo::LayoutProjectionMemo,
}

#[cfg(debug_assertions)]
std::thread_local! {
    static RBS_OPERAND_TRACE_ACTIVE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

mod binding_baseline;
mod binding_projection;
mod binding_restoration;
#[cfg(test)]
mod binding_value_fixpoint_probe;
mod cfg_lattice;
mod command_identity;
mod command_world_tables;
mod compilation_lookup_advice;
mod deferred_entries;
mod invocation_binding;
mod layout_projection_memo;
mod module_bindings;
mod mutation_summary;
mod site_queries;
mod source_analysis_driver;
mod source_bindings_equality;
mod source_origin;
mod source_walk;
mod trusted_packages;
mod variable_access;

use binding_projection::{
    CommandTargetLookup, PointWalkEntry, source_binding, source_binding_from_original_input,
    source_binding_projection, source_binding_projection_in,
};
pub use cfg_lattice::analyse_command_binding;
#[cfg(test)]
use cfg_lattice::cfg_entry_state;
#[cfg(test)]
use cfg_lattice::unavailable_cfg_entry;
use command_identity::CommandToken;
pub use command_identity::{
    AllocationIncarnation, CommandAllocation, CommandAllocationSite, CommandIdentity,
    CommandRefInterner, RuntimeCommandTokenIdentity,
};
#[cfg(test)]
use compilation_lookup_advice::original_static_layout_projection;
use compilation_lookup_advice::{
    original_declaration_call_layout_advice, original_logical_declaration_call_layout_advice,
    original_operand_layout_advice, original_site_operand_layout_advice, trace_rbs_operand_option,
};
use deferred_entries::seed_deferred_inputs;
pub use invocation_binding::DeclaredCommandCandidate;
use invocation_binding::{
    OriginalArgumentCompletion, SourceCompilerNameClosure, SourceObserverExecution,
};
#[cfg(test)]
use module_bindings::constructed_script_words_with_optional_metadata;
use module_bindings::{
    BindingInvocationMetadata, BoundedClassConstruction, RetainedBindingRoots,
    apply_may_binding_transition, apply_namespace_transition, definition_body_command_tokens,
    imported_binding_links, qualify_execution_name,
};
#[cfg(test)]
use module_bindings::{
    ProcedureDefinitionReplay, collect_namespace_resolution, discarded_procedure_history,
    recover_procedure_definition,
};
pub(crate) use module_bindings::{
    ResolvedBindingInvocation, ResolvedFrameBody, ResolvedFrameBodySelection,
    SourceBindingTimeline, constructed_script_words_with_metadata_context,
};
use mutation_summary::PrecisionWrites;
pub use mutation_summary::{
    CommandTrustSnapshot, RebindingSubjects, scan_module_command_mutations,
    scan_module_command_mutations_with_bindings,
};
use source_analysis_driver::root_source_execution_context;
#[cfg(test)]
use source_origin::SourceOriginInterner;
pub use source_origin::{MaterialisedSourceKind, SourceOriginId, SourceOriginKind};
use source_origin::{source_activation_name, source_called_body_activation_name};
use source_walk::{
    PreparedSourceNativeInvocation, SourceBodyEntryPurpose, SourceConditionalHandlerNormal,
    SourceNativeNormalCompletion, boxed_source_branch, called_body_namespace,
    closed_procedure_definition, evaluated_script_realm, join_evaluated_words,
    native_procedure_parameters, opaque_source_invocation, original_procedure_publication_key,
    parse_source_expression, prepare_source_native_invocation, publish_source_branch,
    requires_reentry_guard, resolve_source_invocation_facts, retained_script_operand,
    select_contents_write_source, source_argument_context_boxed, source_body_context_boxed,
    source_command_tokens_boxed, source_context_clone_boxed, source_context_conditional_boxed,
    source_effective_words, source_invocation_argument_owner, source_operand_truth,
};
pub use source_walk::{
    SourceBodyPhase, SourceBodyPhaseReachability, SourceImplicitMathInvocation,
    undeclared_transitions_leave_command_world_closed,
};
#[cfg(test)]
use source_walk::{original_variable_arena, original_variable_source};
pub use trusted_packages::{TrustedCommandLookup, TrustedPackageLoader, TrustedSourceModuleLoader};
use trusted_packages::{
    apply_package_transition, trusted_loader_is_selected, trusted_package_loader,
};
pub use variable_access::SourceVariableEvaluationOwner;

#[cfg(debug_assertions)]
struct RbsOperandTraceScope(bool);

/// The point's original dispatch projection, a pure function of the point.
/// Every consumer of a dispatch site asks for it, so it is built once. A
/// clone starts empty: copies are edited in place without a reset.
#[derive(Debug, Default)]
struct PointBindingMemo(OnceLock<SourceInvocationBinding>);

#[derive(Debug, Clone)]
struct SourceBindingPoint {
    declaration_preview: bool,
    walk_entry: PointWalkEntry,
    original_arguments_complete_normally: bool,
    observer_execution: SourceObserverExecution,
    realm: tcl_dialect::model::InvocationRealm,
    compilation: tcl_registry::native_compilation::NativeCompilationContext,
    compiled_execution: compiled_invocation::CompiledInvocationSelection,
    head: Option<String>,
    original_head_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    original_written_words: Option<Arc<original_name_value::OriginalWrittenSourceWords>>,
    evaluated_argument_values: Arc<[Option<String>]>,
    evaluated_argument_words: Arc<[crate::registry_invocation::EffectiveInvocationWord]>,
    frozen_written_words: Option<Arc<[crate::registry_invocation::EffectiveInvocationWord]>>,
    frozen_written_names:
        Option<Arc<[Option<Arc<original_name_value::OriginalProducedNameValue>>]>>,
    frozen_head_object: Option<Arc<receiver_self::FrozenSourceObjectHead>>,
    method_prefix_arguments: Vec<(usize, Arc<SourceCapturedMethodPrefix>)>,
    offset: u32,
    end: u32,
    namespace: String,
    namespace_key: SourceNamespaceKey,
    state: ModuleCommandBindings,
    lookup_snapshot: OnceLock<Arc<SourceLookupSnapshot>>,
    original_binding: PointBindingMemo,
    dispatch: bool,
    /// The actual source invocation whose modeled continuation produced this state.
    continuation_of: Option<CommandAllocationSite>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeferredSourceBody {
    realm: tcl_dialect::model::InvocationRealm,
    identity: String,
    implementation_generation: u32,
    implementation_allocation: Option<CommandAllocation>,
    source_origin: Option<Arc<SourceOriginId>>,
    executed_script: Option<(CommandAllocationSite, usize, ExecutedScriptSource)>,
    executed_word: Option<(Arc<SourceOriginId>, tcl_lexer::Span)>,
    source: String,
    offset: u32,
    namespace: String,
    namespace_key: SourceNamespaceKey,
    event: Option<String>,
    receiver_method: bool,
    receiver_namespace: Option<conditional_body::SourceReceiverNamespace>,
    future_frame: Option<crate::var_resolve::VariableExecutionFrame>,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    original_parameters: Option<formal_topology::OriginalFormalTopology>,
    statics: Option<crate::raw_binding::CapturedStaticBindings>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct DeferredImplementationId {
    command: String,
    generation: u32,
    allocation: Option<CommandAllocation>,
}

/// Future callable outcomes retained independently of actual document continuation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceDeferredOutcome {
    implementation: DeferredImplementationId,
    namespace_key: SourceNamespaceKey,
    frame: crate::var_resolve::VariableExecutionFrame,
    realm: tcl_dialect::model::InvocationRealm,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
    state: Arc<ModuleCommandBindings>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourcePhasePoint {
    site: u32,
    phase: SourceBodyPhase,
    point: SourceBindingPoint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceLifecycleCoverage {
    declaration_preview: bool,
    frame: crate::var_resolve::VariableExecutionFrame,
    namespace: SourceNamespaceKey,
    entered: Option<[bool; 3]>,
}

/// Variable lookup immediately before one reached source substitution reads.
/// Earlier word parts and array-index evaluation have already run; the read's
/// own observer callback has not. Missing records do not license another point.
#[derive(Debug, Clone)]
pub struct SourceVariableAccess {
    /// Exact source reference, including its variable wrapper and index.
    pub source: crate::ir::SourceSite,
    /// Actual evaluation that reached this read; lexical containment alone is insufficient.
    pub owner: SourceVariableEvaluationOwner,
    /// Original undecoded source spelling used by the variable grammar owner.
    pub original_spelling: String,
    /// Shared place-resolution context at the actual read.
    pub variable_context: Arc<crate::var_resolve::ResolveContext>,
    context_alternatives: Vec<Arc<crate::var_resolve::ResolveContext>>,
    context_fingerprint: u64,
    object_instance: Option<Arc<SourceObjectInstanceProof>>,
    selected_places: Vec<SelectedSourceVariablePlace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectedSourceVariablePlace {
    context: Arc<crate::var_resolve::ResolveContext>,
    place: Option<Arc<crate::place::Place>>,
}

/// Positioned source projection of the executable command-binding kernel.
/// Source interpretation and IR interpretation use the same lookup and
/// registry-transition transfer functions.
/// Cache derived from an immutable point inventory. A cloned builder starts
/// empty, so later mutation cannot publish a projection into its sibling's world.
#[derive(Debug, Default)]
struct SourceHeadProjectionCache(Mutex<HashMap<String, SourceInvocationBinding>>);

impl Clone for SourceHeadProjectionCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

type EnteredWordKey = (Arc<SourceOriginId>, (u32, u32), Option<usize>);
type ImplicitMathSite = (Arc<SourceOriginId>, u32, String);
type ImplicitMathInvocations = BTreeMap<ImplicitMathSite, Vec<SourceInvocationBinding>>;
type SourceVariableAccesses = BTreeMap<u32, Vec<SourceVariableAccess>>;
type OriginVariableAccesses = BTreeMap<Arc<SourceOriginId>, SourceVariableAccesses>;
#[derive(Debug, Clone, PartialEq, Eq)]
struct FixedMathObservation {
    declaration_preview: bool,
    frame: crate::var_resolve::VariableExecutionFrame,
    namespace: SourceNamespaceKey,
    prerequisite: Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
}

type ImplicitFixedMathInvocations = BTreeMap<ImplicitMathSite, Vec<FixedMathObservation>>;

#[derive(Debug, Clone)]
pub(crate) struct OriginalSourceAnalysisOwner {
    // Positive original entry option, not inferred from empty command tables.
    source_arguments_empty: bool,
    source: Arc<SourceOriginId>,
    metadata: crate::registry_invocation::OwnedInvocationMetadataContext,
    config: tcl_lexer::LexerConfig,
}
/// Derived products are owned outside every immutable identity used as a key.
#[derive(Debug, Default)]
struct OriginalSourceAnalysisMemo {
    // Derived readonly source metadata cannot change this owner's identity.
    registered_instance_sources: std::sync::OnceLock<
        std::sync::Weak<original_name_value::OriginalRegisteredInstanceSourceCache>,
    >,
    source_operand_layouts: std::sync::OnceLock<
        std::sync::Weak<declaration_preview::OriginalDeclarationSourceOperandCache>,
    >,
    conditional_procedure_declarations:
        std::sync::OnceLock<source_transition_advice::OriginalConditionalProcedureDeclarationMemo>,
}

/// Literal-list loop iterations one source walk unrolls exactly. Nested
/// unrolling multiplies (62 nested two-element `foreach` loops are 2^62 body
/// walks), so past this budget a literal-list loop takes the general
/// join-until-stable walk every other loop uses: less precise, equally sound.
/// Each admitted iteration still walks its whole body, so the cost is this
/// budget times the body: 256 keeps a deep nest to seconds in a debug build
/// where 4,096 took minutes.
const MAX_FINITE_SOURCE_UNROLLING: usize = 256;

/// Retained source-point command and substitution proofs for one analysed script.
#[derive(Debug, Clone, Default)]
pub struct SourceCommandBindings {
    /// Loop iterations this walk has unrolled exactly from literal lists; see
    /// [`MAX_FINITE_SOURCE_UNROLLING`]. Walk-time budget, outside equality.
    finite_unrolled_iterations: usize,
    /// Source loops whose repetition the walk is inside. Walk-time state,
    /// outside equality.
    active_source_loops: usize,
    /// Site of the rule's own statement the loader is reading, with whether
    /// its head was unknown before its words ran, and site of the statement it
    /// has just rejected by its head. Walk-time state, outside equality.
    rule_loader_statement: Option<(u32, bool)>,
    rule_loader_rejection: Option<u32>,
    // Derived point-memo lifetime, outside semantic equality and hash.
    source_point_cache_lease: Option<Arc<source_analysis_cache::SourcePointCacheLease>>,
    conditional_source: conditional_source::SourceConditionalKernel,
    analyser_source_settings:
        Option<Arc<crate::analyser::source_settings::OriginalAnalyserSourceSettings>>,
    original_analysis_owner: Option<Arc<OriginalSourceAnalysisOwner>>,
    // The original analysis retains derived receipts through Module lifetime.
    // Equality and execution identity remain independent of cache warmth.
    original_registered_instance_sources:
        Arc<original_name_value::OriginalRegisteredInstanceSourceCache>,
    original_source_operand_layouts:
        Arc<declaration_preview::OriginalDeclarationSourceOperandCache>,
    declaration_preview_depth: usize,
    /// Depth of walks over bodies the source walk proves never entered.
    unreached_walk_depth: usize,
    /// Survey-scoped Logical conditional source control. Only
    /// `analyse_package_index_survey` sets it; see
    /// `package_target_survey` for its exact admission.
    logical_conditional_source_control: bool,
    compilation_scope: tcl_runtime_api::SourceCompilationScope,
    /// The interpretation entered no command: only the source identity and
    /// entry state are retained.
    unentered: bool,
    lexer_config: Option<tcl_lexer::LexerConfig>,
    declaration_layouts: SharedSourceInventory<declaration_layout::DeclarationLayouts>,
    original_script_activations: Vec<Arc<declaration_layout::OriginalDiagnosticFrameEntry>>,
    pre_handler_failures: SharedSourceInventory<pre_handler_failure::PreHandlerFailures>,
    unrepresented_entries: SharedSourceInventory<pre_handler_failure::UnrepresentedEntries>,
    unpositioned_projections: SourceHeadProjectionCache,
    declaration_flow_cache: declaration_flow::SourceDeclarationFlowCache,
    derived_indexes: shared_inventory::DerivedIndexes,
    module_projection_memo: retained_source::ModuleProjectionMemo,
    deferred_namespace_index: deferred_namespace_index::DeferredNamespaceIndexCache,
    deferred_body_index: deferred_body_index::DeferredBodyIndexCache,
    root_origin: Option<Arc<SourceOriginId>>,
    original_argument_syntax_commands:
        SharedSourceInventory<BTreeMap<CommandAllocationSite, Option<tcl_lexer::LexerConfig>>>,
    origin_points: SharedSourceInventory<BTreeMap<Arc<SourceOriginId>, Vec<SourceBindingPoint>>>,
    implicit_math_invocations: SharedSourceInventory<ImplicitMathInvocations>,
    invocation_reads:
        SharedSourceInventory<BTreeMap<CommandAllocationSite, Arc<SourceInvocationVariableReads>>>,
    normal_variable_continuations:
        SharedSourceInventory<normal_variable_continuation::NormalVariableContinuations>,
    original_variable_invocations:
        SharedSourceInventory<original_variable_inventory::OriginalVariableInvocations>,
    invocation_normal_results: SharedSourceInventory<normal_result::InvocationNormalResults>,
    conditional_procedure_results:
        SharedSourceInventory<normal_result::ConditionalProcedureResults>,
    object_callback_effects: SharedSourceInventory<object_callbacks::ObjectCallbackInventory>,
    rhs_read_store_observations: SharedSourceInventory<read_store_schedule::ReadStoreObservations>,
    definition_method_references:
        SharedSourceInventory<definition_reference_inventory::DefinitionMethodReferences>,
    expression_preparations: SharedSourceInventory<
        BTreeMap<CommandAllocationSite, Option<Vec<SourceExpressionPreparation>>>,
    >,
    conditional_expression_evaluations: SharedSourceInventory<
        BTreeMap<
            CommandAllocationSite,
            Vec<expression_preparation::SourceConditionalExpressionEvaluation>,
        >,
    >,
    current_expression_reads: expression_preparation::CurrentExpressionReadCaptures,
    origin_variable_accesses: SharedSourceInventory<OriginVariableAccesses>,
    implicit_fixed_math_invocations: SharedSourceInventory<ImplicitFixedMathInvocations>,
    fixed_math_diagnostic_presence:
        SharedSourceInventory<BTreeMap<ImplicitMathSite, SourceCommandSlotPresence>>,
    entered_scripts: SharedSourceInventory<
        BTreeMap<
            CommandAllocationSite,
            BTreeMap<usize, Vec<origin_inventory::EnteredScriptObservation>>,
        >,
    >,
    entered_words: SharedSourceInventory<BTreeMap<EnteredWordKey, Vec<ExecutedScriptSource>>>,
    points: SharedSourceInventory<Vec<SourceBindingPoint>>,
    dispatch_points: SharedSourceInventory<BTreeMap<u32, Vec<usize>>>,
    /// Inside one procedure body, only the points of its own declared frame.
    declared_frame_view: Option<Arc<declared_frame_view::DeclaredFrameView>>,
    /// This interpretation with every called procedure's body also walked
    /// in its own declared frame, where a [`Self::declared_frame_view`] reads.
    declared_frame_walk: Option<Arc<SourceCommandBindings>>,
    final_state: Arc<ModuleCommandBindings>,
    /// Closed normal root world, independently of the partial fallback state.
    /// This source-model fact grants no physical interpreter execution.
    original_completed_root_state: Option<Arc<ModuleCommandBindings>>,
    deferred: SharedSourceInventory<BTreeMap<DeferredImplementationId, DeferredSourceBody>>,
    rule_declarations:
        SharedSourceInventory<BTreeMap<CommandAllocationSite, Vec<SourceRuleDeclarationCandidate>>>,
    deferred_outcomes: SharedSourceInventory<Vec<SourceDeferredOutcome>>,
    future_bodies: SharedSourceInventory<Vec<SourceFutureBodyInventory>>,
    active_calls: BTreeSet<DeferredImplementationId>,
    /// Activations whose recursive back-edge assumed the call leaves the
    /// shared command world unchanged; verified when the activation ends.
    recursive_assumptions: BTreeSet<DeferredImplementationId>,
    /// Activations whose assumption failed: their back-edge widens.
    recursive_assumption_refused: BTreeSet<DeferredImplementationId>,
    /// Per procedure identity, whether its declaration-time body walks left
    /// the command world unchanged.
    world_preserving_bodies:
        BTreeMap<String, (bool, Vec<tcl_syntax::formal_params::FormalParameter>)>,
    /// A declaration-time walk of a runtime receiver's member body may have
    /// changed the command world: relative names it publishes land in the
    /// receiver's private namespace, ahead of every lookup there.
    receiver_body_changes_command_world: bool,
    active_variable_observers: Vec<variable_observers::ActiveVariableObserver>,
    called_implementations: BTreeSet<DeferredImplementationId>,
    phases: SharedSourceInventory<Vec<SourcePhasePoint>>,
    lifecycle_coverage:
        SharedSourceInventory<BTreeMap<CommandAllocationSite, Vec<SourceLifecycleCoverage>>>,
    variable_accesses: SharedSourceInventory<BTreeMap<u32, Vec<SourceVariableAccess>>>,
    compilation_failures: SharedSourceInventory<Vec<SourceNativeCompilationFailure>>,
    compilation_boundaries: SharedSourceInventory<
        BTreeMap<CommandAllocationSite, Option<SourceNativeCompilationFailure>>,
    >,
    compilation_provider_required: SharedSourceInventory<BTreeSet<CommandAllocationSite>>,
    compilation_sources:
        SharedSourceInventory<BTreeMap<CommandAllocationSite, Option<ExecutedScriptSource>>>,
    compiled_children: SharedSourceInventory<
        BTreeMap<CommandAllocationSite, Vec<compiled_preflight::SourceCompiledChild>>,
    >,
    expression_syntax_children: SharedSourceInventory<
        BTreeMap<
            CommandAllocationSite,
            Vec<compiled_preflight::expression_syntax::SourceExpressionSyntaxChild>,
        >,
    >,
    #[doc(hidden)]
    pub compiler_invocations: SharedSourceInventory<
        BTreeMap<CommandAllocationSite, Vec<compiler_inventory::SourceCompilerInvocation>>,
    >,
    runtime_coverage: SharedSourceInventory<BTreeMap<CommandAllocationSite, (usize, bool)>>,
}

#[derive(Clone, Copy)]
struct SourceIndexView<'a> {
    arena: &'a tcl_lexer::word_parts::ExecutablePartArena,
    offset: u32,
}

#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct SourceExecutionContext<'a> {
    control: source_execution_control::SourceExecutionControl,
    original_logical_body:
        Option<&'a declaration_preview::logical_body_entry::OriginalLogicalBodyEntry>,
    compilation_snapshot: Option<&'a NativeCompilationSnapshot>,
    selected_compilation: Option<&'a tcl_registry::native_compilation::NativeCompilationSelection>,
    original_variable_compilation:
        Option<&'a original_name_value::OriginalSourceVariableCompilation<'a>>,
    namespace: &'a str,
    namespace_key: Option<&'a SourceNamespaceKey>,
    config: tcl_lexer::LexerConfig,
    registry: &'a CommandRegistry,
    depth: u32,
    frame: &'a crate::var_resolve::VariableExecutionFrame,
    invocation_offset: u32,
    variable_read_owner: Option<&'a SourceVariableEvaluationOwner>,
    original_written_projection: Option<&'a named_arguments::OriginalNamedWords>,
    written_arguments: Option<&'a [crate::registry_invocation::EffectiveInvocationWord]>,
    written_values: Option<&'a [Option<Arc<native_result::EvaluatedSourceValue>>]>,
    written_name_values: Option<&'a [Option<Arc<original_name_value::OriginalProducedNameValue>>]>,
    written_representations:
        Option<&'a [Option<source_representation::FrozenSourceRepresentation>]>,
    written_objects: Option<&'a [Option<Arc<SourceObjectInstanceProof>>]>,
    written_method_prefixes: Option<&'a [Option<Arc<SourceCapturedMethodPrefix>>]>,
    written_variable_reads: Option<&'a [Option<Arc<argument_reads::FrozenSourceArgumentRead>>]>,
    expression_source: Option<&'a ExecutedExpressionSource>,
}

/// Borrow the original source operands and their prepared runtime values together.
#[derive(Clone, Copy)]
struct SourceCommandInput<'a> {
    image: &'a tcl_lexer::SourceImage,
    segment: &'a crate::segmenter::SegmentedCommand,
    words: &'a [crate::ir::WordExpr],
    effective: &'a [crate::registry_invocation::EffectiveInvocationWord],
}

#[derive(Clone, Copy)]
struct SourceNativeInvocation<'a> {
    compilation_spec: Option<&'a tcl_registry::native_compilation::NativeCompilationSpec>,
    compilation_selection: &'a tcl_registry::native_compilation::NativeCompilationSelection,
    segment: &'a crate::segmenter::SegmentedCommand,
    words: &'a [crate::ir::WordExpr],
    written_arguments: Option<&'a [crate::registry_invocation::EffectiveInvocationWord]>,
    target: &'a SourceCommandTarget,
    invocation: &'a tcl_registry::InvocationWords<'a>,
    original_variable_operands: &'a crate::variable_bindings::OriginalVariableInvocation,
}

#[derive(Clone, Copy)]
struct SourceArgumentReceipts<'a> {
    arguments: &'a [crate::registry_invocation::EffectiveInvocationWord],
    values: &'a [Option<Arc<native_result::EvaluatedSourceValue>>],
    name_values: &'a [Option<Arc<original_name_value::OriginalProducedNameValue>>],
    representations: &'a [Option<source_representation::FrozenSourceRepresentation>],
    objects: &'a [Option<Arc<SourceObjectInstanceProof>>],
    prefixes: &'a [Option<Arc<SourceCapturedMethodPrefix>>],
    reads: &'a [Option<Arc<argument_reads::FrozenSourceArgumentRead>>],
}

#[derive(Clone, Copy)]
struct SourceScriptOperands<'a> {
    compilation_spec: Option<&'a tcl_registry::native_compilation::NativeCompilationSpec>,
    compilation_selection: &'a tcl_registry::native_compilation::NativeCompilationSelection,
    words: &'a [crate::ir::WordExpr],
    written_arguments: Option<&'a [crate::registry_invocation::EffectiveInvocationWord]>,
    target: &'a SourceCommandTarget,
    arguments: &'a tcl_registry::InvocationArguments<'a>,
}

struct PreparedSourceArguments {
    effective: Vec<crate::registry_invocation::EffectiveInvocationWord>,
    written_arguments: Arc<[crate::registry_invocation::EffectiveInvocationWord]>,
    written_values: Vec<Option<Arc<native_result::EvaluatedSourceValue>>>,
    written_name_values: Vec<Option<Arc<original_name_value::OriginalProducedNameValue>>>,
    written_representations: Vec<Option<source_representation::FrozenSourceRepresentation>>,
    written_objects: Vec<Option<Arc<SourceObjectInstanceProof>>>,
    written_method_prefixes: Vec<Option<Arc<SourceCapturedMethodPrefix>>>,
    written_variable_reads: Vec<Option<Arc<argument_reads::FrozenSourceArgumentRead>>>,
    outcomes: SourceOutcomes,
    complete_normally: bool,
    ready: bool,
}

struct SourceNativeFrameSelection {
    namespace: String,
    namespace_key: SourceNamespaceKey,
    frame: crate::var_resolve::VariableExecutionFrame,
    variables: Option<Arc<crate::var_resolve::ResolveContext>>,
    namespace_activation: bool,
}

struct PreparedSourceNativeBody {
    selection: SourceNativeFrameSelection,
    parent_variables: Arc<crate::var_resolve::ResolveContext>,
    parent_frame: crate::var_resolve::VariableExecutionFrame,
    flow: tcl_registry::script_body_flow::ScriptBodyFlow,
}

#[derive(Clone, Copy)]
struct SourceLifecycleInvocation<'a> {
    spec: &'a tcl_registry::body_execution::CapturedLifecycleSpec,
    arguments: tcl_registry::InvocationArguments<'a>,
    site: u32,
    words: &'a [crate::ir::WordExpr],
    written_arguments: Option<&'a [crate::registry_invocation::EffectiveInvocationWord]>,
    target: &'a SourceCommandTarget,
}

#[derive(Clone, Copy)]
struct SourceLifecyclePhase<'a> {
    phase: SourceBodyPhase,
    operand: Option<tcl_registry::body_execution::BodyOperand>,
    hook: Option<&'a str>,
}

#[derive(Clone, Copy)]
struct SourceLoopOperands<'a> {
    entry: tcl_registry::iteration_entry::IterationEntry,
    finite: Option<&'a tcl_registry::iteration_entry::FiniteIteration>,
    variable_lists: &'a [usize],
    initial: &'a [usize],
    conditions: &'a [usize],
    repeated: &'a [usize],
    continued: &'a [usize],
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SourceNormalCompletion;

#[derive(Clone, Default, PartialEq, Eq)]
struct SourceOutcomes {
    normal_completion: Option<SourceNormalCompletion>,
    native_normal_completion: Option<Arc<SourceNativeNormalCompletion>>,
    conditional_normal_completion: Option<Arc<SourceConditionalHandlerNormal>>,
    complete_procedure_return: Option<tcl_registry::completion_route::InvocationCompletionRoute>,
    normal: Option<Box<ModuleCommandBindings>>,
    normal_value: Option<Arc<native_result::EvaluatedSourceValue>>,
    normal_name_value: Option<Arc<original_name_value::OriginalProducedNameValue>>,
    // Immutable proof storage stays off frames retained by recursive bodies.
    normal_representation: Option<Arc<source_representation::FrozenSourceRepresentation>>,
    normal_object: Option<Arc<SourceObjectInstanceProof>>,
    normal_method_prefix: Option<Arc<SourceCapturedMethodPrefix>>,
    normal_rhs_read: Option<Arc<read_store_schedule::CapturedExpressionRead>>,
    abrupt_objects: Vec<(
        tcl_registry::completion_route::InvocationCompletionRoute,
        Arc<SourceObjectInstanceProof>,
    )>,
    abrupt_name_values: Vec<(
        tcl_registry::completion_route::InvocationCompletionRoute,
        Arc<original_name_value::OriginalProducedNameValue>,
    )>,
    abrupt_values: Vec<(
        tcl_registry::completion_route::InvocationCompletionRoute,
        Arc<native_result::EvaluatedSourceValue>,
    )>,
    abrupt: Vec<(
        tcl_registry::completion_route::InvocationCompletionRoute,
        Box<ModuleCommandBindings>,
    )>,
}

/// Historical namespace-lookup effects discovered through the closed command
/// lattice. Keeping this beside the binding state means alias-prefixed and
/// recovered invocations publish the same resolution facts as direct calls.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct NamespaceResolutionProjection {
    changed: bool,
    dynamic: bool,
    ensembles: EnsembleManagement,
    rebound_names: BTreeSet<String>,
    opaque_namespaces: BTreeSet<String>,
}

/// Ensemble management observed so far; each axis only rises.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
struct EnsembleManagement {
    /// A configuration change may have remapped any ensemble, including a
    /// stock one, so no stock member-to-worker mapping is proved.
    reconfigured: bool,
    /// A creation may have replaced any command, including a stock ensemble,
    /// so a compiled body selects no stock ensemble worker.
    created: bool,
}

/// Module-wide may-binding resolver for command-effect consumers.
///
/// This is deliberately the sole owner of the richer, alias-prefix-preserving
/// command lattice. Consumers can resolve a source statement, enumerate the
/// source spellings observed by the lattice, or ask whether a source spelling
/// may reach an unknown implementation; they cannot mutate or reinterpret the
/// command table themselves.
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_excessive_bools)] // Independent opacity/trust axes; none implies another.
pub struct ModuleCommandBindings {
    /// Canonical original byte publications and modeled namespace geometry.
    original_command_world: Arc<source_command_world::OriginalSourceCommandWorld>,
    logical_alias_prefixes:
        Arc<BTreeMap<ResolvedCommandTarget, Option<Arc<logical_prefix::LogicalAliasPrefix>>>>,
    current_source_origin: Option<Arc<SourceOriginId>>,
    /// Original fresh pool provenance, permanently withdrawn by unknown host
    /// or object methods. New result objects cannot restore this history.
    ordinary_literal_pool: Option<literal_object_pool::SourceOrdinaryLiteralPool>,
    allocation_counts: Arc<BTreeMap<(CommandAllocationSite, SourceCommandKey), u8>>,
    namespace_allocation_counts:
        Arc<BTreeMap<(CommandAllocationSite, tcl_core_types::ByteNamespacePath), u8>>,
    class_definitions: Arc<BTreeMap<CommandIdentity, ClassDefinitionReceipt>>,
    object_instances: Arc<object_instance::SourceObjectState>,
    bounded_constructions: Arc<BTreeMap<CommandIdentity, BoundedClassConstruction>>,
    tainted_object_dispatch: Arc<BTreeSet<String>>,
    tainted_provider_state: Arc<BTreeSet<String>>,
    command_observers: Arc<command_observers::SourceCommandObservers>,
    #[doc(hidden)]
    pub source_variables: Arc<crate::var_resolve::ResolveContext>,
    variable_frame: crate::var_resolve::VariableExecutionFrame,
    /// Sparse flow-sensitive binding state. Analysis forks this state for
    /// every executable root, while most roots leave the command table
    /// unchanged. Copy-on-write keeps those forks allocation-cheap and
    /// detaches only at the centralised replacement/join seams below.
    bindings: Arc<SourceCommandTable<BTreeSet<MayBinding>>>,
    original_entry_bindings: Arc<SourceCommandTable<BTreeSet<MayBinding>>>,
    /// The package rows and loader-handler state the entry starts from. A
    /// top-level replay restarts from them as it does from the entry's command
    /// rows: a package the module's own walk loaded is not loaded before the
    /// document's first command runs.
    original_entry_packages: Option<Arc<OriginalEntryPackages>>,
    /// Implementations of command objects followed by namespace imports.
    objects: Arc<BTreeMap<CommandToken, BTreeSet<MayBinding>>>,
    /// Exact export/path alternatives are joined alongside command bindings.
    namespace_exports: Arc<SourceNamespaceMap<BTreeSet<Vec<tcl_core_types::NameBytes>>>>,
    namespace_paths: Arc<SourceNamespaceMap<BTreeSet<Vec<SourceNamespaceKey>>>>,
    namespaces: Arc<SourceNamespaceSet>,
    unknown_lookup_namespaces: Arc<SourceNamespaceSet>,
    unknown_namespace_paths: Arc<SourceNamespaceSet>,
    namespace_unknown_handlers: Arc<SourceNamespaceSet>,
    ensemble_owner_namespaces: Arc<SourceNamespaceSet>,
    ensemble_dispatches: Arc<EnsembleDispatches>,
    unknown_export_namespaces: Arc<SourceNamespaceSet>,
    packages: Arc<BTreeMap<String, BTreeSet<PackageProvision>>>,
    revoked_loaders: Arc<BTreeSet<String>>,
    loader_handler_unknown: bool,
    /// Binding state reachable between executable roots. Unlike `bindings`,
    /// this excludes transient pre/post states within a top-level or body
    /// execution and is therefore safe to replay as another root's entry. It
    /// is published once after the fixpoint and then cloned into each
    /// per-script analysis, so allocation sharing is the natural ownership
    /// model.
    root_boundary_bindings: Arc<SourceCommandTable<BTreeSet<MayBinding>>>,
    /// Immutable registry facts. Kept separate from `bindings` so the sparse
    /// may-state only publishes names that a module transition actually
    /// affects.
    baseline: Arc<BindingBaseline>,
    /// A registry-declared mutation whose affected name cannot be bounded.
    opaque_domain: bool,
    /// The opaque domain arose from a command-binding/procedure mutation,
    /// rather than from a namespace/lookup transition. Only this dimension
    /// poisons the optimiser's whole-module command-trust projection.
    opaque_binding_mutation: bool,
    /// A dynamic command-binding transition subject (or a runtime-selected
    /// executable root) invalidates a procedure's declared identity. This is
    /// intentionally narrower than [`Self::opaque_binding_mutation`]: an
    /// unavailable/autoloaded body can affect builtin folding without proving
    /// that every retained procedure name was rebound.
    dynamic_proc_binding: bool,
    /// A `rename` / `interp alias` / command delete transition ran whose
    /// **subject** this lattice could not name. Narrower again than
    /// [`Self::dynamic_proc_binding`], which also rises for a runtime-selected
    /// executable root and for a widened command-binding domain — neither of
    /// which moves a binding. Only this dimension may withdraw the claim that
    /// an untouched name still denotes its builtin (#2168).
    unnameable_rebinding_subject: bool,
    /// A command-world transition (binding, namespace resolution, package or
    /// interpreter effect) completed without a closed normal-return receipt.
    /// The normal path then holds no exact completed command world, even when
    /// every abrupt alternative is an error.
    unclosed_command_world: bool,
    /// While the world is withheld, whether every step that withheld it is one
    /// the diagnostic view models exactly (an import under the release's own
    /// preload command). Only absence advice reads this; a proof consumer
    /// reads `unclosed_command_world` alone.
    diagnostic_command_world_closed: bool,
    /// An unbounded command-world effect was reached in a runtime receiver's
    /// member-body frame, where relative publications land in the receiver's
    /// private namespace.
    receiver_frame_opacity: bool,
    /// Namespace resolution transitions selected after alias-prefix and
    /// command-binding resolution. This is historical rather than final-state
    /// data: restoring a path/import later cannot make an earlier fold sound.
    namespace_resolution: NamespaceResolutionProjection,
    /// Qualified procedure names for which this analysis owns a retained or
    /// source-recovered body. A `Define(Procedure)` transition may publish a
    /// precise user-command target only when the invocation that produced it
    /// has just placed that body in this inventory.
    /// Copy-on-write because this inventory is module-wide and normally
    /// immutable after initial discovery. The flow-sensitive binding walk
    /// forks state for every executable root; sharing the common inventory
    /// keeps a module with N procedures from cloning N names for each of its
    /// N roots. Exact readable `proc` recovery is the only path that detaches
    /// a branch.
    procedure_bodies: Arc<BTreeSet<String>>,
    /// Exact current-interpreter names vacated, replaced, or introduced by a
    /// move/delete/alias transition. Unlike `bindings`, this is historical:
    /// restoring the final binding does not restore trust in the intervening
    /// procedure identity.
    /// Shared between forks of one state; a write detaches it.
    rebound_names: SharedSourceInventory<BTreeSet<SourceCommandKey>>,
    /// Flow-insensitive, namespace-candidate rebound names for procedure
    /// call-site trust: a conservative local-or-global interpretation, kept
    /// separate so it cannot weaken the exact binding resolver above.
    proc_rebound_names: BTreeSet<String>,
    /// `Module` records a duplicate procedure declaration even when each
    /// source body was readable and could be replayed. The optimiser's trust
    /// contract deliberately treats that metadata as a whole-domain mutation.
    has_redefined_procedures: bool,
    /// Closed boundary effects of executable roots, retained while the
    /// source-order view advances. User calls join this proven effect state
    /// instead of inventing whole-domain mutation for every retained body.
    source_order_user_call_effects: Option<Arc<Self>>,
}

impl ModuleCommandBindings {
    /// Build the closed may-binding state for every retained executable root.
    #[must_use]
    pub(crate) fn analyse(module: &Module, registry: &CommandRegistry) -> Self {
        Self::analyse_with_options(module, registry, module.source_entry.options())
    }
}

#[derive(Default)]
struct DiscardedProcedureHistory {
    modules: Vec<Module>,
    rebound_names: BTreeSet<String>,
    opaque: bool,
}

struct BindingWalkOutcome {
    /// State reachable after the script completes.
    post: ModuleCommandBindings,
    /// Union of states observed after executable invocations within the
    /// script. This is queryable history, never a replay entry state.
    observed: ModuleCommandBindings,
}

/// Invocation-local walk state shared by direct and readable-body transfers.
struct InvocationBindingContext<'a> {
    retained_roots: &'a mut RetainedBindingRoots,
    source_order_mode: bool,
    metadata: BindingInvocationMetadata<'a>,
}

struct RetainedEvaluatedBody {
    span: tcl_lexer::Span,
    source: String,
    script: Arc<crate::ir::Script>,
}

/// Project the exact command words returned by one source-proven list
/// constructor invocation.
///
/// Recognition is registry-owned through [`tcl_registry::ReturnElements`],
/// while [`ModuleCommandBindings`] proves that the effective constructor head
/// still reaches only registry-backed implementations. Every source operand
/// must have one literal value under the active lexer grammar: substitution,
/// expansion, recovery, or a rebound constructor fails closed so the caller
/// can widen the affected frame.
#[cfg(test)]
pub(crate) fn constructed_script_words(
    word: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &str,
) -> Option<Vec<String>> {
    constructed_script_words_with_optional_metadata(word, registry, bindings, namespace, None)
}

impl Binding {
    const fn of(kind: BindingKind) -> Self {
        Self { kind, target: None }
    }

    /// True when the name still denotes its original core builtin.
    #[must_use]
    pub fn is_original_builtin(&self) -> bool {
        self.kind == BindingKind::Builtin
    }

    /// True when the name denotes a concrete, foldable user proc.
    #[must_use]
    pub fn is_foldable_proc(&self) -> bool {
        self.kind == BindingKind::Proc && self.target.is_some()
    }
}

/// Catalogue baseline at the supplied key, using exact-first Registry selection.
/// Rooted method-only names cannot borrow a bare descriptor. This baseline
/// does not establish a current Native command binding or implementation.
fn default_binding(qname: &str, registry: &CommandRegistry) -> Binding {
    if registry.get(qname).is_some() {
        Binding::of(BindingKind::Builtin)
    } else {
        Binding::of(BindingKind::Opaque)
    }
}

/// The packages a fresh interpreter of `release` provides before any script
/// runs, keyed by `policy`'s package keys. Requiring one only checks its
/// version: no loader runs and no command binding changes.
/// Package state captured at a binding walk's entry
/// ([`ModuleCommandBindings::original_entry_packages`]).
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalEntryPackages {
    packages: Arc<BTreeMap<String, BTreeSet<PackageProvision>>>,
    revoked_loaders: Arc<BTreeSet<String>>,
    loader_handler_unknown: bool,
    /// The command world before the first command runs. A top-level replay
    /// takes its availability: a later unknown command withdraws the world
    /// from that point on, not for the commands that ran before it.
    command_world: Arc<source_command_world::OriginalSourceCommandWorld>,
}

/// CFG queries are projections of the shared command-table state.
pub struct CommandBinding<'a> {
    block_entry: HashMap<BlockId, ModuleCommandBindings>,
    ordered_blocks: Vec<BlockId>,
    cfg: &'a CfgFunction,
    registry: &'a CommandRegistry,
    seed: ModuleCommandBindings,
    lookup_contexts: CfgLookupContexts,
}

/// Conservative, flow-insensitive summary of command rebindings across a
/// whole module — the input to the optimiser's builtin-fold trust gate.
///
/// A `rename` / proc redef / `interp alias` buried in a proc body only
/// takes effect when that proc is *called*, and the cross-proc call order
/// is not statically known.  Rather than a full interprocedural
/// call-effect fixpoint, this takes the sound over-approximation: any
/// core builtin some body may rebind is treated as untrusted
/// *everywhere*.  Top-level rebindings stay precise via the
/// flow-sensitive [`CommandBinding`] lattice; this whole-module union is
/// the conservative fold gate.
///
/// `Default` trusts everything (no names, not dynamic) — the
/// "no mutations observed" baseline.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleCommandMutations {
    /// Canonical names of core builtins some body may rebind.
    names: std::collections::HashSet<String>,
    /// Canonical names that are the *source* or *target* of a `rename`, or
    /// the alias name of an `interp alias`, anywhere in the module — i.e.
    /// a name that no longer reliably denotes the proc it was declared as
    /// (or never denoted one at all). Unlike `names`, this is NOT
    /// restricted to builtins: a plain `proc NAME { … }` declaration is
    /// deliberately excluded (declaring a name as itself is the expected,
    /// trustworthy binding) — only `rename` / `interp alias` touching the
    /// name is recorded. Feeds [`Self::trusts_proc_binding`].
    rebound: std::collections::HashSet<String>,
    /// A body performs a dynamic `rename`/alias/proc (target not
    /// statically known) → resolution of *any* name is opaque.
    dynamic: bool,
    /// A `rename` / `interp alias` / command delete ran whose **subject** this
    /// scan could not name — `rename $a {}`, not `foo bar`.
    ///
    /// Narrower than [`Self::dynamic`] on purpose. `dynamic` also rises for
    /// reasons that rebind nothing (an unresolved command head, a changed
    /// namespace resolution, the walk depth cap), which is why the value
    /// lattice deliberately does not gate on it — doing so withdraws every
    /// builtin fold from any file naming a command the registry does not know
    /// (#2164). This axis rises *only* when something was definitely rebound
    /// and the name is unknown, so no name can be claimed as still denoting
    /// its builtin (#2168).
    rebinding_subjects: RebindingSubjects,
    /// Some body runs in a receiver- or caller-selected namespace and names a
    /// command relatively, so that name may resolve to an implementation the
    /// static module does not contain. Scoped deliberately: it disqualifies a
    /// fact derived *inside* such a frame ([`Self::has_runtime_selected_frames`]),
    /// and says nothing about the top level or a procedure, which resolve in
    /// namespaces this scan can name. A body that also *changes* a binding
    /// from such a frame sets [`Self::dynamic`] instead, because the subject
    /// it moved is one this scan cannot spell.
    runtime_selected_frames: bool,
    /// A body changes how a namespace *resolves* command names, without
    /// renaming anything: `namespace import`/`forget`, `namespace path`,
    /// `namespace unknown`, `namespace ensemble`, or a namespace delete.
    /// These never touch [`Self::rebound`], yet
    /// `namespace import -force ::evil::abs` into `::tcl::mathfunc` replaces
    /// what `abs(…)` resolves to just as a `rename` would. Recorded whatever
    /// the target namespace, because the pattern list is a Tcl value this
    /// scan does not evaluate.
    resolution_changed: bool,
    /// Namespaces whose command resolution was changed in a way this scan
    /// cannot enumerate — a globbed or non-literal `namespace import`, a
    /// `namespace path`/`unknown`/`ensemble`, or a namespace delete. A name
    /// *in* one of these is no longer trustworthy; a name outside them is
    /// unaffected, which is what keeps `namespace import ::lib::*` into
    /// `::app` from distrusting `::lib::helper` itself.
    opaque_namespaces: std::collections::HashSet<String>,
    /// The module may set the variable through which the target's releases
    /// change how a computed double is spelled
    /// ([`tcl_dialect::NumberSyntax::precision_variable`], `tcl_precision`
    /// through 8.6) — see [`module_may_set_precision`]. Not a command
    /// binding, but the same kind of whole-module fact every fold reads: a
    /// double folded to text under the default spelling would print
    /// differently from the one the program computes.
    precision_written: PrecisionWrites,
}

/// Prepared, narrow procedure-binding trust projection for call-site
/// parameter seeding. Unlike [`ModuleCommandMutations`], it intentionally
/// excludes unresolved commands, external units, and unavailable bodies when
/// those facts do not name a dynamic command-binding transition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProcBindingTrustProjection {
    rebound: std::collections::HashSet<String>,
    dynamic: bool,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod source_variable_access_tests;

#[cfg(test)]
mod source_callable_static_tests;

#[cfg(test)]
mod source_alias_lookup_tests;

#[cfg(test)]
mod compiled_variable_policy_tests;

#[cfg(test)]
mod hosted_storage_entry_tests;

#[cfg(test)]
mod selected_replay_formal_tests;

#[cfg(test)]
mod original_definition_body_purpose_tests;

#[cfg(test)]
mod shared_noop_source_join_tests;

#[cfg(test)]
mod original_explicit_script_purpose_tests;

#[cfg(test)]
mod original_logical_source_layout_tests;

#[cfg(test)]
mod logical_source_advice_join_tests;

#[cfg(test)]
mod retained_source_read_configuration_tests;

#[cfg(test)]
mod unavailable_source_read_configuration_tests;

#[cfg(test)]
mod immutable_source_memo_lifetime_tests;

#[cfg(test)]
mod original_compiled_list_owner_join_tests;

// Temporary bounded original-g4 attribution. Existing assertions remain exact.
#[cfg(test)]
fn trace_original_g4_owner(
    label: &str,
    bindings: &SourceCommandBindings,
    tokens: &crate::ir::CommandTokens,
) {
    let Some(binding) = tokens.source_binding.as_ref() else {
        return;
    };
    let site = binding.invocation_site();
    let current_owner = binding
        .original_analysis_owner
        .as_ref()
        .zip(bindings.original_analysis_owner())
        .is_some_and(|(actual, expected)| Arc::ptr_eq(actual, expected));
    let visits = site.and_then(|site| bindings.compiler_invocations.get(site));
    eprintln!(
        "G4_OWNER label={label} offset={:?} reached={:?} unknown={} target={:?} owner={} words={} native_dialect={} source_protocol={} table={} structured={} admission={:?} visits={}",
        site.map(|site| site.offset),
        binding.runtime_reachability(),
        binding.execution_is_unknown(),
        binding
            .proved_execution_target()
            .map(|target| target.command.as_str()),
        current_owner,
        binding.original_compiler_words.is_some(),
        binding.native_compiler_dialect().is_some(),
        binding.compiler_source_protocol().is_some(),
        binding.compiler_lookup_state.is_some(),
        binding.native_structured_preparation.is_some(),
        binding.native_compilation_admission_selection(),
        visits.map_or(0, Vec::len),
    );
    if let Some(visits) = visits {
        for (ordinal, visit) in visits.iter().take(8).enumerate() {
            eprintln!(
                "G4_VISIT label={label} ordinal={ordinal} selection={:?} admitted={} structured={} words={} policy={} native_entry={} opaque={} unclosed={}",
                visit.selection,
                visit.admitted.is_some(),
                visit.structured.is_some(),
                visit.original_words.is_some(),
                visit.policy.is_some(),
                visit.table.state.baseline.native_entry.is_some(),
                visit.table.state.has_opaque_domain(),
                visit.table.state.unclosed_command_world,
            );
        }
    }
}

#[cfg(test)]
mod standalone_document_layout_tests;

#[cfg(test)]
mod compiled_range_selection_tests;
