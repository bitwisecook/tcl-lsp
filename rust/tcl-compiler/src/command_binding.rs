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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use crate::alias::is_current_interpreter;
use crate::cfg::BlockId;
use crate::cfg::Function as CfgFunction;
use crate::ir::{Module, Script, Statement};
use crate::ir_helpers::{evaluated_command_substitutions_with_replay, nested_bodies};
use crate::naming::normalise_qualified_name as nqn;
use crate::var_escape::helpers::invocation_facts;
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, CommandRegistry,
    EffectiveRegistrySemantics, InvocationFacts, NamespaceTransition, StateTransition,
    StateTransitionDomain, TransitionSubject,
};

mod default_manufacture_advice;
mod executed_expression_source;
mod executed_script_source;
mod frozen_arguments;
mod future_bodies;
mod incomplete_body_metadata;
mod logical_definition;
mod logical_operation;
mod materialized_footprint;
mod native_list_assignment;
mod original_callback_lookup;
mod original_compiler_effects;
mod pre_handler_failure;
mod source_arguments;
mod source_name_lookup;
mod source_unavailability_advice;
pub use default_manufacture_advice::OriginalDefaultManufactureShape;
pub use original_callback_lookup::OriginalCallbackPrefix;
mod insertion_template;
mod original_command_mutation;
mod original_command_table;
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
pub use future_bodies::{SourceFutureBodyInventory, SourceFutureCallSite};
pub use source_command_world::{
    OriginalClassInstanceMethodRoster, OriginalCommandPublication, OriginalCommandPublicationKind,
    OriginalCompletedCommandWorld,
};
mod operand_layout;
mod retained_source;
pub use retained_source::RetainedSourceModuleBindings;
pub(crate) use retained_source::source_analysis_frame;
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
pub(crate) use source_declared_command::DeclaredSourceSelection;
pub use source_declared_command::{
    OriginalDeclaredCommandWords, OriginalDeclaredLogicalBodyContext,
    OriginalDeclaredSourceBodyFrame, OriginalDeclaredSourceObligation,
};
mod declaration_body_ownership;
mod declaration_layout;
pub use declaration_layout::SourceOriginalVariableFrame;
mod declaration_preview;
mod scalar_body_alpha;
pub use scalar_body_alpha::OriginalScalarBodyAlphaRename;
mod vendor_catalogue_advice;
pub use vendor_catalogue_advice::{
    VendorCatalogueSourceCandidate, VendorCatalogueSourceObligation,
};
mod source_catalogue_advice;
mod source_transition_advice;
pub use source_catalogue_advice::{
    OriginalCatalogueSourceCandidate, OriginalCatalogueSourceObligation,
};
pub(crate) use source_transition_advice::original_single_command_substitution_words;
pub use source_transition_advice::{
    OriginalLogicalSourceNameInput, OriginalSourceCallbackProcedureLookup,
    OriginalSourceCallbackProcedureRefusal, OriginalSourceCallbackProcedureTarget,
    OriginalSourceCallbackProcedureTargetKind, OriginalSourceCallbackRegistration,
    OriginalSourceClassCandidate, OriginalSourceClassDeclaration, OriginalSourceClassHandleBinding,
    OriginalSourceClassInstance, OriginalSourceClassInstanceWords, OriginalSourceClassPublication,
    OriginalSourceClassPublications, OriginalSourceClassReference, OriginalSourceCommandTransition,
    OriginalSourceCommandTransitionAdvice, OriginalSourceConfiguredClassReference,
    OriginalSourceConstructorCall, OriginalSourceConstructorShape,
    OriginalSourceInterpreterHandleBody, OriginalSourceInterpreterPathBinding,
    OriginalSourceProcedureCandidate, OriginalSourceProcedurePublication,
    OriginalSourceProcedurePublications, OriginalSourceProducedCommandPrefix,
    OriginalSourceRegisteredHandleBinding, OriginalSourceRegisteredInstance,
    OriginalSourceRegisteredInstanceWords, SourceAdviceNameInput,
    SourceCommandTransitionObligation,
};
pub(crate) use source_transition_advice::{
    OriginalSourceTransitionAdviceLookup, OriginalSourceTransitionAdviceTape,
};
mod original_literal_argv_rewrite;
pub use original_literal_argv_rewrite::OriginalLiteralExpressionBracing;
mod expression_operand_advice;
mod formal_call_advice;
mod lifecycle_advice;
mod trusted_package_transfer;
pub(crate) use expression_preparation::SourceConditionalExpressionEvaluation;
mod original_math_function;
pub use original_math_function::OriginalMathFunctionOccurrence;
mod math_occurrence;
pub use math_occurrence::{SourceConditionalMathInvocation, SourceMathInvocation};
pub(crate) mod formal_topology;
mod formal_value;
mod origin_inventory;
pub(crate) use conditional_body::SourceConditionalBodyEntry;
pub use conditional_body::{SourceCallerFrameInvocationTemplate, SourceDeclaredReceiverBodyEntry};
pub use source_declared_method_template::OriginalDeclaredSelfMethodTemplate;
mod source_file;
mod substitution_template;
pub use executed_expression_source::ExecutedExpressionSource;
pub use executed_script_source::{ExecutedScriptMapping, ExecutedScriptSource};
mod body_template;
pub use body_template::{BodyProofScope, BodySourceProofs, NativeBodyTemplate};
mod command_publication;
mod compiled_invocation;
mod ensemble_compilation;
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
pub use generated_procedures::{SourceInstalledProcedureBody, SourceProcedureImplementationBody};
mod compiled_preflight;
mod compiler_inventory;
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
mod dictionary_store;
mod evaluated_word;
mod expression_preparation;
mod interpreter_manufacture;
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
pub use normal_result::SourceNormalResult;
pub(crate) mod object_callbacks;
pub use invocation_reads::{
    SourceInvocationVariableRead, SourceInvocationVariableReads, SourceVariableReadResidual,
};
mod container_coercion;
mod native_result;
mod numeric_operand_cache;
mod object_instance;
pub(crate) mod original_name_value;
pub(crate) mod original_variable_compilation;
mod source_analysis_cache;
mod source_analysis_entry;
pub(crate) use source_analysis_entry::SourceDeclaredCommandContracts;
use source_analysis_entry::native_compilation_dialect;
pub use source_analysis_entry::{SourceAnalysisEntry, SourceAnalysisOptions};
mod source_loop;
mod source_representation;
pub use deferred_method::{
    SourceMethodReceiver, SourceReceiverMethodEntries, SourceReceiverMethodEntry,
};
pub use object_instance::{SourceObjectAllocation, SourceObjectInstanceProof};
mod cfg_lookup_context;
mod namespace_context;
use cfg_lookup_context::{CfgLookupContexts, CfgPointContext};
mod namespace_slots;
pub(crate) use namespace_context::NamespaceKeyQuery;
pub(crate) use namespace_context::SourceCommandKey;
pub use namespace_context::SourceNamespaceKey;
use namespace_context::{
    CommandKeyQuery, SourceCommandTable, SourceNamespaceMap, SourceNamespaceSet,
};
mod native_variable_tables;
mod runtime_entry;
mod snit_definition;
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
    /// An `interp alias` (`target` = the alias target name).
    Alias,
    /// Renamed/deleted-away or never-defined → dispatches to `unknown`.
    Opaque,
    /// ⊤ — conflicting bindings at a merge, or dynamic mutation.
    Unknown,
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
pub(crate) struct ResolvedCommandTarget {
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

/// Object identity survives movement of its command-table slot. Imports keep
/// this identity; interpreter aliases perform target-name lookup instead.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandIdentity {
    /// Actual interpreter token supplied by a live runtime entry snapshot.
    pub runtime: Option<RuntimeCommandTokenIdentity>,
    /// Slot in which this token was originally created.
    pub origin: String,
    /// Source creation site; zero denotes an explicit entry implementation.
    pub declaration: u32,
    /// Allocation identity; source offsets alone do not identify derived code
    /// or a command recreated by another execution of the same source site.
    pub allocation: Option<CommandAllocation>,
}

/// Stable runtime command identity; generations belong to implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeCommandTokenIdentity {
    /// Interpreter owner and non-reused interpreter arena identity.
    pub interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    /// Non-reused command token within the interpreter.
    pub token: u64,
}

type CommandToken = CommandIdentity;

/// Source instance in which an executable command was allocated.
#[derive(Debug, Clone)]
pub struct SourceOriginId {
    kind: Arc<SourceOriginKind>,
    fingerprint: u64,
    activation_key: u64,
}

#[derive(Default)]
struct SourceOriginInterner {
    buckets: HashMap<u64, Vec<(Weak<SourceOriginKind>, u64)>>,
    next_key: u64,
}

impl SourceOriginInterner {
    fn intern(&mut self, source: &Arc<SourceOriginKind>, fingerprint: u64) -> u64 {
        if self.next_key.is_multiple_of(64) {
            self.buckets.retain(|_, sources| {
                sources.retain(|(source, _)| source.strong_count() != 0);
                !sources.is_empty()
            });
        }
        let bucket = self.buckets.entry(fingerprint).or_default();
        let existing = bucket.iter().find_map(|(retained, key)| {
            retained
                .upgrade()
                .filter(|retained| retained == source)
                .map(|_| *key)
        });
        let key = existing.unwrap_or_else(|| {
            self.next_key = self
                .next_key
                .checked_add(1)
                .expect("source identity exhausted");
            self.next_key
        });
        bucket.push((Arc::downgrade(source), key));
        key
    }
}

/// Semantic source identity retained behind a cached fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceOriginKind {
    /// Authored source bytes supplied by the compilation driver.
    Authored(tcl_lexer::SourceImage),
    /// Driver-attested file bytes, retaining distinct file/provider identity.
    Loaded {
        /// Exact selected path; no filename catalogue proves this source.
        path: Arc<str>,
        /// Stable selected implementation identity supplied by the driver.
        implementation_id: Arc<str>,
        /// Exact decoded script bytes consumed by native source evaluation.
        source: tcl_lexer::SourceImage,
    },
    /// Script materialised from the frozen operands of another invocation.
    Derived {
        /// Parent invocation, including its source instance.
        parent: CommandAllocationSite,
        /// Effective argument positions contributing to the script.
        arguments: Vec<usize>,
        /// Exact concatenated source, retaining semantic equality beyond hashes.
        source: tcl_lexer::SourceImage,
        /// Parsing protocol of the materialised value.
        kind: MaterialisedSourceKind,
    },
}

impl SourceOriginId {
    /// Retain one authored source instance without hashing its bytes at each
    /// command allocation or immutable interpreter snapshot.
    #[must_use]
    pub fn authored(source: &Arc<str>) -> Self {
        Self::authored_image(tcl_lexer::SourceImage::document(source))
    }

    /// Retain the exact original source and its selected input channel.
    #[must_use]
    pub fn authored_image(source: tcl_lexer::SourceImage) -> Self {
        Self::from_kind(SourceOriginKind::Authored(source))
    }

    /// Retain an actual selected file independently of equal bytes in another file.
    #[must_use]
    pub fn loaded(path: Arc<str>, implementation_id: Arc<str>, source: &Arc<str>) -> Self {
        Self::loaded_image(
            path,
            implementation_id,
            tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes())),
        )
    }

    /// Retain selected file identity and the driver's exact source receipt.
    #[must_use]
    pub fn loaded_image(
        path: Arc<str>,
        implementation_id: Arc<str>,
        source: tcl_lexer::SourceImage,
    ) -> Self {
        Self::from_kind(SourceOriginKind::Loaded {
            path,
            implementation_id,
            source,
        })
    }

    /// Full semantic identity; fingerprints never substitute for this data.
    #[must_use]
    pub fn kind(&self) -> &SourceOriginKind {
        self.kind.as_ref()
    }

    /// Original source bytes and channel, without a display conversion.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        match self.kind() {
            SourceOriginKind::Authored(source)
            | SourceOriginKind::Loaded { source, .. }
            | SourceOriginKind::Derived { source, .. } => source,
        }
    }

    /// Whole-source Unicode view for consumers whose contract requires text.
    ///
    /// # Errors
    /// Returns the original UTF-8 failure; opaque bytes retain their identity.
    pub fn try_text(&self) -> Result<&str, std::str::Utf8Error> {
        self.source_image().try_text()
    }

    fn derived(
        parent: CommandAllocationSite,
        arguments: Vec<usize>,
        source: &Arc<str>,
        kind: MaterialisedSourceKind,
    ) -> Self {
        Self::derived_image(
            parent,
            arguments,
            tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes())),
            kind,
        )
    }

    fn derived_image(
        parent: CommandAllocationSite,
        arguments: Vec<usize>,
        source: tcl_lexer::SourceImage,
        kind: MaterialisedSourceKind,
    ) -> Self {
        Self::from_kind(SourceOriginKind::Derived {
            parent,
            arguments,
            source,
            kind,
        })
    }

    fn from_kind(kind: SourceOriginKind) -> Self {
        static INTERNER: OnceLock<Mutex<SourceOriginInterner>> = OnceLock::new();
        let kind = Arc::new(kind);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        kind.hash(&mut hasher);
        let fingerprint = std::hash::Hasher::finish(&hasher);
        let activation_key = INTERNER
            .get_or_init(|| Mutex::new(SourceOriginInterner::default()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .intern(&kind, fingerprint);
        Self {
            kind,
            fingerprint,
            activation_key,
        }
    }

    const fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    fn activation_key(source: &Arc<Self>) -> u64 {
        source.activation_key
    }
}

fn source_activation_name(
    origin: Option<&Arc<SourceOriginId>>,
    label: &str,
    offset: u32,
) -> String {
    origin.map_or_else(
        || format!("{label}/site:{offset}"),
        |origin| {
            format!(
                "{label}/source:{}/site:{offset}",
                SourceOriginId::activation_key(origin)
            )
        },
    )
}

fn source_procedure_activation_name(
    origin: Option<&Arc<SourceOriginId>>,
    target: &SourceCommandTarget,
    site: u32,
) -> String {
    let mut label = format!("{}@{}", target.command, target.implementation_generation);
    if let Some(allocation) = &target.implementation_allocation {
        use std::fmt::Write;
        write!(
            label,
            "/implementation:{}:{}:{:?}",
            SourceOriginId::activation_key(&allocation.site.source),
            allocation.site.offset,
            allocation.incarnation,
        )
        .expect("formatting an activation identity into a String");
    }
    source_activation_name(origin, &label, site)
}

fn source_called_body_activation_name(
    origin: Option<&Arc<SourceOriginId>>,
    target: &SourceCommandTarget,
    body: &DeferredSourceBody,
    site: u32,
) -> String {
    let parent = source_procedure_activation_name(origin, target, site);
    if body.receiver_method
        && let Some((declaration, _, _)) = &body.executed_script
    {
        return format!(
            "{parent}/body:{}:{}",
            SourceOriginId::activation_key(&declaration.source),
            declaration.offset,
        );
    }
    parent
}

impl PartialEq for SourceOriginId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.kind, &other.kind)
            || (self.fingerprint() == other.fingerprint() && self.kind == other.kind)
    }
}

impl Eq for SourceOriginId {}

impl Hash for SourceOriginId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fingerprint().hash(state);
    }
}

impl PartialOrd for SourceOriginId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SourceOriginId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if Arc::ptr_eq(&self.kind, &other.kind) {
            return std::cmp::Ordering::Equal;
        }
        self.fingerprint()
            .cmp(&other.fingerprint())
            .then_with(|| self.kind.cmp(&other.kind))
    }
}

/// Native protocol interpreting a materialised argv value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaterialisedSourceKind {
    /// Tcl script parser.
    Script,
    /// Tcl expression parser.
    Expression,
}

/// An allocation instruction in one authored or materialised source instance.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandAllocationSite {
    /// Source instance; derived offsets are never authored document offsets.
    pub source: Arc<SourceOriginId>,
    /// Offset within that source instance.
    pub offset: u32,
}

impl CommandAllocationSite {
    /// Compact storage projection for this exact source allocation instruction.
    /// Callers retain the typed site as the equality proof and append their
    /// allocation incarnation and implementation generation independently.
    #[must_use]
    pub fn variable_storage_identity(&self) -> String {
        format!(
            "source:{}:{}",
            SourceOriginId::activation_key(&self.source),
            self.offset
        )
    }
}

/// Bounded incarnation of an allocation instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AllocationIncarnation {
    /// First reached execution.
    First,
    /// Second reached execution, distinct from an earlier captured token.
    Second,
    /// Further executions; no unique runtime token is asserted.
    RepeatedFresh,
}

/// Command allocation, distinct from its current name and implementation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandAllocation {
    /// Instruction allocating the token or implementation.
    pub site: CommandAllocationSite,
    /// Repeated executions must not alias a prior captured runtime identity.
    pub incarnation: AllocationIncarnation,
    /// Presentation of the evaluated output slot, without lookup authority.
    pub command: String,
    /// Exact namespace owning the allocation, independent of its presentation.
    pub namespace: SourceNamespaceKey,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum MayBinding {
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
struct ImportedCommandBinding {
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
    metadata_context: crate::registry_invocation::OwnedInvocationMetadataContext,
    /// Explicit module-declared frame effects, independent of native cells.
    declared: Arc<crate::ir::DeclaredFrameEffects>,
    logical_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    vendor_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    hosted_execution_context: Option<tcl_registry::f5::BigIpExecutionContext>,
    execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    compiled_variable_provider:
        Option<tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider>,
    invocation_realm: tcl_dialect::model::InvocationRealm,
    unknown_entry: bool,
    native_entry: Option<Arc<tcl_runtime_api::NativeCompilationEntry>>,
    native_compilation: tcl_registry::native_compilation::NativeCompilationContext,
    fingerprint: u64,
    registry_snapshot: Option<tcl_registry::RegistrySemanticKey>,
    declared_commands: BTreeMap<String, String>,
    declared_surface: Option<Arc<source_analysis_entry::SourceDeclaredCommandContracts>>,
    catalogue_commands: BTreeMap<String, String>,
    /// Registry-owned, generation-specific facts shared with CFG construction.
    semantics: Arc<EffectiveRegistrySemantics>,
    /// Derived fixed descriptor projection, excluded from equality and hashing.
    metadata_slots: OnceLock<Option<original_command_table::RegistryMetadataSlotIndex>>,
    import_binding: Option<tcl_dialect::NamespaceImportBinding>,
    trusted_loaders: BTreeMap<String, TrustedPackageLoader>,
    trusted_source_modules: Vec<TrustedSourceModuleLoader>,
    release: Option<tcl_dialect::TclVersion>,
    dialect: Option<tcl_registry::InvocationDialect>,
}

impl PartialEq for BindingBaseline {
    fn eq(&self, other: &Self) -> bool {
        self.metadata_context == other.metadata_context
            && self.logical_source_input == other.logical_source_input
            && self.vendor_source_input == other.vendor_source_input
            && self.hosted_execution_context == other.hosted_execution_context
            && self.invocation_realm == other.invocation_realm
            && self.execution_name_policy == other.execution_name_policy
            && self.unknown_entry == other.unknown_entry
            && self.registry_snapshot == other.registry_snapshot
            && self.native_entry == other.native_entry
            && self.native_compilation == other.native_compilation
            && self.compiled_variable_provider == other.compiled_variable_provider
            && self.trusted_loaders == other.trusted_loaders
            && self.trusted_source_modules == other.trusted_source_modules
            && self.declared_commands == other.declared_commands
            && self.declared_surface == other.declared_surface
            && self.catalogue_commands == other.catalogue_commands
            && self.release == other.release
            && self.dialect == other.dialect
            && self.import_binding == other.import_binding
            && self.semantics.binding_names() == other.semantics.binding_names()
            && self.semantics.unresolved_command_handlers()
                == other.semantics.unresolved_command_handlers()
            && self.declared == other.declared
    }
}

impl Eq for BindingBaseline {}

impl BindingBaseline {
    /// Original compiler engine only; handler policies remain in `dialect`.
    fn compilation_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        native_compilation_dialect(self.native_entry.as_deref(), self.dialect)
    }

    fn for_registry(registry: &CommandRegistry) -> Self {
        let mut baseline = Self {
            metadata_context:
                crate::registry_invocation::OwnedInvocationMetadataContext::Standalone,
            declared: Arc::new(crate::ir::DeclaredFrameEffects::default()),
            logical_source_input: None,
            vendor_source_input: None,
            hosted_execution_context: None,
            execution_name_policy: registry
                .profile()
                .and_then(|profile| {
                    tcl_registry::InvocationDialect::of_profile(profile).authored_name_policy()
                })
                .map(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe),
            invocation_realm: tcl_dialect::model::InvocationRealm::RuleLoader,
            unknown_entry: false,
            native_entry: None,
            compiled_variable_provider: None,
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext::default(
            ),
            fingerprint: 0,
            registry_snapshot: Some(registry.snapshot().semantic_key()),
            declared_commands: BTreeMap::new(),
            declared_surface: None,
            catalogue_commands: registry
                .command_names()
                .map(|name| (nqn(name), name.to_owned()))
                .collect(),
            semantics: registry.profile().map_or_else(
                || registry.effective_semantics(),
                |profile| {
                    registry.effective_semantics_for_dialect_in_realm(
                        tcl_registry::InvocationDialect::of_profile(profile),
                        tcl_dialect::model::InvocationRealm::InterpreterRuntime,
                    )
                },
            ),
            metadata_slots: OnceLock::new(),
            import_binding: registry
                .profile()
                .and_then(tcl_dialect::DialectProfile::namespace_import_binding),
            trusted_loaders: BTreeMap::new(),
            trusted_source_modules: Vec::new(),
            release: registry.runtime_version(),
            dialect: registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile),
        };
        baseline.refresh_fingerprint();
        baseline
    }

    fn refresh_fingerprint(&mut self) {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        self.registry_snapshot.hash(&mut state);
        self.execution_name_policy.hash(&mut state);
        self.metadata_context.hash(&mut state);
        self.declared.hash(&mut state);
        self.logical_source_input.hash(&mut state);
        self.vendor_source_input.hash(&mut state);
        self.hosted_execution_context.hash(&mut state);
        self.native_entry.hash(&mut state);
        self.native_compilation.hash(&mut state);
        self.compiled_variable_provider.hash(&mut state);
        self.invocation_realm.hash(&mut state);
        self.unknown_entry.hash(&mut state);
        self.declared_commands.hash(&mut state);
        self.declared_surface.hash(&mut state);
        self.catalogue_commands.hash(&mut state);
        self.semantics.binding_fingerprint().hash(&mut state);
        self.import_binding.hash(&mut state);
        self.trusted_loaders.hash(&mut state);
        self.trusted_source_modules.hash(&mut state);
        self.release.hash(&mut state);
        self.dialect.hash(&mut state);
        self.fingerprint = std::hash::Hasher::finish(&state);
    }
}

/// A selected standard-distribution loader contract supplied by the driver.
/// Package-name advertisements alone never create this provenance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrustedPackageLoader {
    /// Native engine family audited by this provider, independent of its name.
    pub required_core_family: Option<tcl_dialect::model::Family>,
    /// Package whose unmodified loader is supplied at entry.
    pub package: String,
    /// Exact selected package version, when established by the provider.
    pub version: Option<String>,
    /// Stable provider/source implementation identity.
    pub implementation_id: String,
    /// Commands installed by this selected loader.
    pub command_surface: Vec<String>,
    /// Actual compiler registrations of installed commands, independently of handler semantics.
    /// Missing rows preserve uncertainty; a package name or factory shape is not a stamp.
    pub compiler_hooks:
        BTreeMap<String, tcl_runtime_api::native_compilation::NativeCompilerHookPresence>,
    /// Driver-attested definition recipes for these exact installed factories.
    /// The command, lookup and namespace-state dependencies must also remain live.
    /// This attestation does not prove constructor closure or an object class.
    pub definition_dispatchers: BTreeSet<(String, tcl_registry::definer::DefinitionDispatcher)>,
    /// Commands present in some audited version alternatives, never assumed absent.
    pub optional_command_surface: Vec<String>,
    /// Export patterns installed by this selected loader.
    pub namespace_exports: BTreeMap<String, Vec<String>>,
    /// Export patterns present in some audited version alternatives.
    pub optional_namespace_exports: BTreeMap<String, Vec<String>>,
    /// Lookup dependencies required before executing the selected loader.
    /// They remain prerequisites of its installed implementation afterwards.
    pub lookup_dependencies: Vec<TrustedCommandLookup>,
    /// Lookup dependencies of the installed implementation, checked after loading.
    /// Commands supplied by the loader itself cannot be preload prerequisites.
    pub installed_lookup_dependencies: Vec<TrustedCommandLookup>,
    /// Namespace storage whose callbacks influence the implementation.
    pub state_dependency_namespaces: Vec<String>,
    /// Cells with effects explicitly represented by the provider contract.
    pub modelled_state_variables: Vec<String>,
}

/// A driver-attested file read consumed only by a reached native source handler.
///
/// The decoded bytes and physical file identity are exact. A workspace filename
/// or known procedure declaration does not certify that this file is loaded.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrustedSourceModuleLoader {
    /// Native engine family audited by the driver.
    pub required_core_family: tcl_dialect::model::Family,
    /// Actual evaluated filename accepted by this file-read contract.
    pub path: String,
    /// Explicit encoding name, or the audited native default when absent.
    pub encoding: Option<String>,
    /// Exact decoded source and its driver-owned file implementation identity.
    pub source: ExecutedScriptSource,
}

impl TrustedSourceModuleLoader {
    /// Retain an actual driver-validated file implementation and decoded bytes.
    /// The driver must apply the selected native encoding and file EOF protocol.
    #[must_use]
    pub fn new(
        required_core_family: tcl_dialect::model::Family,
        path: String,
        encoding: Option<String>,
        implementation_id: String,
        text: &Arc<str>,
    ) -> Self {
        let origin = Arc::new(SourceOriginId::loaded(
            Arc::from(path.as_str()),
            Arc::from(implementation_id),
            text,
        ));
        Self {
            required_core_family,
            path,
            encoding,
            source: ExecutedScriptSource {
                text: origin.source_image().clone(),
                origin,
                mapping: ExecutedScriptMapping::Contiguous { base: 0 },
            },
        }
    }

    /// Validate a selected native file read against this exact retained contract.
    #[must_use]
    pub fn matches_selected_file(
        &self,
        path: &str,
        encoding: Option<&str>,
        dialect: tcl_registry::InvocationDialect,
    ) -> bool {
        dialect.family() == Some(self.required_core_family)
            && self.path == path
            && self.encoding.as_deref() == encoding
            && self.source.base() == 0
            && matches!(self.source.origin.kind(), SourceOriginKind::Loaded {
                path: retained, implementation_id, source,
            } if retained.as_ref() == path && !implementation_id.is_empty() && source == &self.source.text)
    }
}

/// One implementation lookup consumed by an explicitly selected provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrustedCommandLookup {
    /// Command word evaluated by the selected implementation.
    pub head: String,
    /// Original rooted namespace operand declared by the provider. Selection
    /// uses the retained native root and exact namespace rows, never a display.
    pub namespace: String,
    /// Registry implementation identity required by the provider.
    pub registry_identity: String,
    /// Whether this dependency exists in every audited version alternative.
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PackageProvision {
    present: bool,
    version: Option<String>,
    implementation: Option<String>,
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

impl SourceConstructorEntry {
    /// Exact authored constructor declaration and its source instance.
    #[must_use]
    pub fn declaration(&self) -> &CommandAllocationSite {
        &self.declaration
    }
    /// Original evaluated constructor body, before any callback executes.
    #[must_use]
    pub fn body(&self) -> &ExecutedScriptSource {
        &self.body
    }
    /// Parsed native formal names and default values at the declaration.
    #[must_use]
    pub fn formals(&self) -> &[(String, Option<String>)] {
        &self.formals
    }
    /// Actual receiver-method frame selected for this retained body entry.
    #[must_use]
    pub fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.frame
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BoundedClassConstruction {
    factory: String,
    implementation_generation: u32,
}

impl PackageProvision {
    fn absent() -> Self {
        Self {
            present: false,
            version: None,
            implementation: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum SourceObserverExecution {
    #[default]
    Unobserved,
    MayObserved,
}

impl SourceObserverExecution {
    fn from_observed(observed: bool) -> Self {
        if observed {
            Self::MayObserved
        } else {
            Self::Unobserved
        }
    }

    fn observed(self) -> bool {
        self == Self::MayObserved
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OriginalArgumentCompletion {
    Unknown,
    Normal,
}

impl OriginalArgumentCompletion {
    fn from_normal(normal: bool) -> Self {
        if normal { Self::Normal } else { Self::Unknown }
    }
    fn is_normal(self) -> bool {
        self == Self::Normal
    }
    fn join(&mut self, other: Self) {
        if !other.is_normal() {
            *self = Self::Unknown;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SourceCompilerNameClosure {
    Unknown,
    Preserved,
}

impl From<bool> for SourceCompilerNameClosure {
    fn from(preserved: bool) -> Self {
        if preserved {
            Self::Preserved
        } else {
            Self::Unknown
        }
    }
}

impl SourceCompilerNameClosure {
    fn is_preserved(self) -> bool {
        self == Self::Preserved
    }

    fn join(&mut self, other: Self) {
        if !other.is_preserved() {
            *self = Self::Unknown;
        }
    }
}

/// A resolved source invocation without discarding absence or an unknown
/// residual. Catalogue identity alone does not establish loader provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    declaration_layout_observations:
        Option<Arc<[declaration_layout::DeclarationLayoutObservation]>>,
    declaration_flow_inventory: Option<Arc<declaration_flow::DeclarationFlowInventory>>,
    compiler_policy: Option<Arc<compiler_inventory::SourceNativeCompilerPolicy>>,
    original_compiler_words: Option<Arc<compiled_invocation::SourceOriginalCompilerWords>>,
    original_written_words: Option<Arc<original_name_value::OriginalWrittenSourceWords>>,
    conditional_expression_evaluations:
        Option<Arc<[expression_preparation::SourceConditionalExpressionEvaluation]>>,
    pub(crate) lookup_namespace: String,
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
    pub known_namespaces: Vec<String>,
    /// Written argv values frozen left-to-right, excluding the command word.
    /// Unknown results and expansion retain `None`; aliases add their own
    /// separate prefixes and never change these original word indices.
    pub evaluated_argument_values: Vec<Option<String>>,
    /// Argument facts frozen at their own evaluation points, including unknown indices.
    pub evaluated_argument_words: Vec<crate::registry_invocation::EffectiveInvocationWord>,
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

impl Default for SourceInvocationBinding {
    fn default() -> Self {
        Self {
            compiled_candidates: Vec::new(),
            compiled_named_candidates: Vec::new(),
            may_use_live_dispatch: true,
            compiled_execution_residual: CompiledExecutionResidual::Closed,
            native_compilation_admission: None,
            native_compiler_admission: None,
            original_compiler_preserves_names: SourceCompilerNameClosure::Unknown,
            native_structured_preparation: None,
            native_switch_preparation: None,
            runtime_reachability: SourceRuntimeReachability::Unknown,
            entered_execution_observer: SourceObserverExecution::Unobserved,
            native_inline_rejection: NativeInlineRejection::None,
            native_handler_envelope: None,
            invocation_variable_reads: None,
            lookup_state: None,
            compiler_lookup_state: None,
            declaration_layout_observations: None,
            declaration_flow_inventory: None,
            compiler_policy: None,
            original_compiler_words: None,
            original_written_words: None,
            conditional_expression_evaluations: None,
            lookup_namespace: String::new(),
            lookup_namespace_key: SourceNamespaceKey::default(),
            lookup_word: None,
            dispatch_site: None,
            provider_state_taint: Vec::new(),
            declared_command: None,
            catalogue_command: None,
            variable_context: Arc::default(),
            original_argument_completion: OriginalArgumentCompletion::Unknown,
            normal_variable_continuation: None,
            original_variable_operands: None,
            normal_result_observations: None,
            rhs_read_store_observations: None,
            object_callback_effects: None,
            variable_frame: crate::var_resolve::VariableExecutionFrame::default(),
            existing_namespace_cells: Vec::new(),
            namespace_cell_presence: crate::var_resolve::NamespaceCellPresence::default(),
            known_namespaces: Vec::new(),
            evaluated_argument_values: Vec::new(),
            evaluated_argument_words: Vec::new(),
            frozen_written_words: None,
            frozen_written_names: None,
            frozen_head_object: None,
            method_prefix_arguments: Vec::new(),
            native_operand_layout: None,
            targets: Vec::new(),
            may_be_absent: false,
            unknown: false,
        }
    }
}

impl Hash for SourceInvocationBinding {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.compiled_candidates.hash(state);
        self.compiled_named_candidates.hash(state);
        self.may_use_live_dispatch.hash(state);
        self.compiled_execution_residual.hash(state);
        self.native_compilation_admission.hash(state);
        self.native_compiler_admission.hash(state);
        self.original_compiler_preserves_names.hash(state);
        self.native_structured_preparation.hash(state);
        self.native_switch_preparation.hash(state);
        self.native_operand_layout.hash(state);
        self.runtime_reachability.hash(state);
        self.entered_execution_observer.hash(state);
        self.native_inline_rejection.hash(state);
        self.native_handler_envelope.hash(state);
        self.invocation_variable_reads.hash(state);
        self.original_argument_completion.hash(state);
        self.normal_variable_continuation.hash(state);
        self.original_variable_operands.hash(state);
        self.rhs_read_store_observations.hash(state);
        self.object_callback_effects.hash(state);
        self.lookup_state.hash(state);
        self.compiler_lookup_state.hash(state);
        self.declaration_layout_observations.hash(state);
        self.declaration_flow_inventory.hash(state);
        self.compiler_policy.hash(state);
        self.original_compiler_words.hash(state);
        self.original_written_words.hash(state);
        self.conditional_expression_evaluations.hash(state);
        self.lookup_namespace.hash(state);
        self.lookup_namespace_key.hash(state);
        self.lookup_word.hash(state);
        self.dispatch_site.hash(state);
        self.targets.hash(state);
        self.unknown.hash(state);
        self.may_be_absent.hash(state);
        self.declared_command.hash(state);
        self.catalogue_command.hash(state);
        self.provider_state_taint.hash(state);
        self.evaluated_argument_values.hash(state);
        self.evaluated_argument_words.hash(state);
        self.frozen_written_words.hash(state);
        self.frozen_written_names.hash(state);
        self.frozen_head_object.hash(state);
        self.method_prefix_arguments.hash(state);
        if self.lookup_state.is_none() {
            self.variable_context.hash(state);
            self.variable_frame.hash(state);
            self.existing_namespace_cells.hash(state);
            self.namespace_cell_presence.hash(state);
            self.known_namespaces.hash(state);
        }
    }
}

/// A retained declaration whose lookup slot has not been changed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeclaredCommandCandidate {
    /// Name in the retained document/workspace declaration surface.
    pub name: String,
    /// Execution namespace used to select this declaration slot.
    pub lookup_namespace: String,
    /// Canonical lookup slot carrying the assistance contract.
    pub slot: String,
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

impl SourceCommandTarget {
    /// Captured prefix inputs from this exact selected implementation chain.
    /// A missing or conflicting producer never falls back to literal bytes.
    #[must_use]
    pub fn original_prepended_name_inputs(
        &self,
    ) -> Option<&[crate::signature_scan::scope::SignatureSourceNameInput]> {
        let inputs = self.original_prepended.as_deref()?;
        (inputs.len() == self.prepended.len()).then_some(inputs)
    }
    /// Original registry descriptor identity, independent of the current slot.
    /// Native issuers preserve their exact identifier bytes; authored registry
    /// rows may use a rooted descriptor name. This supplies no lookup spelling.
    #[must_use]
    pub fn registry_identity(&self) -> Option<&str> {
        self.registry_backed.then_some(self.command.as_str())
    }
}

/// Lexical candidates selected from a retained original pre-operand table.
/// This diagnostic-purpose receipt cannot be used as an execution binding:
/// argument effects, absent targets and opaque runtime alternatives remain.
#[derive(Debug)]
pub(crate) struct OriginalCompilationLookupAdvice {
    snapshot: Arc<SourceLookupSnapshot>,
    targets: Vec<SourceCommandTarget>,
    dialect: tcl_registry::InvocationDialect,
    namespace: SourceNamespaceKey,
    unknown: bool,
    may_be_absent: bool,
}

impl OriginalCompilationLookupAdvice {
    /// An opaque original command world can contain unrelated handlers. A
    /// retained candidate then supplies diagnostic layout, not its operation.
    pub(crate) fn has_opaque_handler_alternatives(&self) -> bool {
        self.snapshot.state.has_opaque_domain()
    }

    /// Original candidate lookup is complete in its independently known namespace.
    pub(crate) fn closed_lookup(&self) -> bool {
        !self.unknown
            && !self.may_be_absent
            && crate::var_resolve::ResolveContext::default()
                .in_frame(&self.snapshot.state.variable_frame)
                .namespace_known
            && self
                .snapshot
                .state
                .variable_frame
                .namespace_identity()
                .is_none_or(|namespace| namespace == &self.namespace)
    }

    /// Closed command layout in the positively retained Logical source world.
    /// Namespace and original target consensus are independent of a physical
    /// variable frame; this cannot certify native lookup or an entered frame.
    pub(crate) fn closed_logical_source_lookup(&self) -> bool {
        let state = &self.snapshot.state;
        state
            .logical_source_name_advice_input()
            .is_some_and(|input| source_analysis_entry::source_input_dialect(input) == self.dialect)
            && !self.unknown
            && !self.may_be_absent
            && !self.targets.is_empty()
            && !state.has_opaque_domain()
            && matches!(&self.namespace, SourceNamespaceKey::Authored(_))
            && state.namespaces.contains(&self.namespace)
    }

    /// A declaration's conditional receiver-body traits use its authentic
    /// source namespace. The future private runtime namespace stays unknown.
    pub(crate) fn closed_receiver_trait_lookup(
        &self,
        entry: &SourceDeclaredReceiverBodyEntry,
    ) -> bool {
        // naming.tcloo.original-declared-receiver-caller-traits
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
        let state = &self.snapshot.state;
        !self.unknown
            && !self.may_be_absent
            && !self.targets.is_empty()
            && self.frame() == entry.preview_frame()
            && self.namespace() == entry.declaration_namespace_context()
            && !self.has_opaque_handler_alternatives()
            && !state.source_step_observed()
            && state.command_observers.is_quiet()
            && self.targets.iter().all(|target| {
                target.registry_backed
                    && target.kind == BindingKind::Builtin
                    && !state.source_execution_observed(target.identity.as_ref())
                    && !state.runtime_execution_observed(target.identity.as_ref())
            })
    }

    pub(crate) fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        &self.snapshot.state.variable_frame
    }
    pub(crate) fn namespace(&self) -> &SourceNamespaceKey {
        &self.namespace
    }

    pub(crate) fn expression_pool_state(
        &self,
    ) -> tcl_registry::conditional_expression::ConditionalExpressionPoolState {
        use tcl_registry::conditional_expression::ConditionalExpressionPoolState as Pool;
        let state = &self.snapshot.state;
        if !state.opaque_domain
            && !state.source_variables.dynamic_traces
            && state.ordinary_literal_pool.as_ref().is_some_and(
                literal_object_pool::SourceOrdinaryLiteralPool::initial_numeric_representations,
            )
        {
            Pool::InitialAuthoredPool
        } else {
            Pool::Unknown
        }
    }
    pub(crate) fn targets(&self) -> &[SourceCommandTarget] {
        &self.targets
    }

    pub(crate) const fn dialect(&self) -> tcl_registry::InvocationDialect {
        self.dialect
    }

    pub(crate) fn realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.snapshot.realm
    }

    /// Original variable context for declaration-only name projections.
    pub(crate) fn original_variable_context(&self) -> &Arc<crate::var_resolve::ResolveContext> {
        &self.snapshot.state.source_variables
    }

    pub(crate) fn alias_frame(&self) -> tcl_registry::VariableAliasFrame {
        self.snapshot.state.source_variables.alias_frame()
    }
}

/// Immutable semantic point. Ordinary lookup retains the original full world
/// without hashing it. Hashing and equality initialize a derived fingerprint
/// once; equality still checks the full state and realm after that fingerprint.
/// A reached state join invalidates the memo on its detached snapshot only.
#[derive(Debug, Clone)]
pub(crate) struct SourceLookupSnapshot {
    realm: tcl_dialect::model::InvocationRealm,
    state: ModuleCommandBindings,
    fingerprint: OnceLock<u64>,
    compiler_name_effects: original_compiler_effects::SourceCompilerNameEffectsCache,
}

impl SourceLookupSnapshot {
    fn new(state: ModuleCommandBindings) -> Self {
        Self::in_realm(state, tcl_dialect::model::InvocationRealm::RuleLoader)
    }

    fn in_realm(state: ModuleCommandBindings, realm: tcl_dialect::model::InvocationRealm) -> Self {
        Self {
            realm,
            state,
            fingerprint: OnceLock::new(),
            compiler_name_effects:
                original_compiler_effects::SourceCompilerNameEffectsCache::default(),
        }
    }

    fn fingerprint(&self) -> u64 {
        *self.fingerprint.get_or_init(|| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.state.hash(&mut hasher);
            self.realm.hash(&mut hasher);
            std::hash::Hasher::finish(&hasher)
        })
    }
}

impl PartialEq for SourceLookupSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.fingerprint() == other.fingerprint()
            && self.realm == other.realm
            && self.state == other.state
    }
}

impl Eq for SourceLookupSnapshot {}

impl Hash for SourceLookupSnapshot {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fingerprint().hash(state);
    }
}

impl SourceInvocationBinding {
    /// Explicit absence of bounded runtime dispatch and compiler admission.
    #[must_use]
    pub fn unknown() -> Self {
        Self {
            unknown: true,
            ..Self::default()
        }
    }

    /// Explicit availability phase retained at this exact execution point.
    /// Missing entry evidence supplies no phase assumption.
    #[must_use]
    pub fn invocation_realm(&self) -> Option<tcl_dialect::model::InvocationRealm> {
        self.lookup_state.as_ref().map(|snapshot| snapshot.realm)
    }

    /// Command-word value retained at this lookup, before alias composition.
    /// Different reaching values join to unknown rather than borrowing the
    /// terminal implementation's name.
    #[must_use]
    pub fn evaluated_command_word(&self) -> Option<&str> {
        self.lookup_word.as_deref()
    }

    /// Relocate physical cell worlds without changing command allocations or source ownership.
    pub fn relocate_variable_proofs(
        &mut self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) {
        self.relocate_invocation_reads(relocation);
        self.variable_context = Arc::new(self.variable_context.relocated(relocation));
        self.normal_variable_continuation = self
            .normal_variable_continuation
            .as_ref()
            .map(|context| Arc::new(context.relocated(relocation)));
        self.variable_frame = relocation.frame(&self.variable_frame);
        // Captured receiver identities have no physical-frame relocation grant.
        self.method_prefix_arguments.clear();
        self.original_argument_completion = OriginalArgumentCompletion::Unknown;
        self.normal_result_observations = None;
        self.rhs_read_store_observations = None;
        if let Some(snapshot) = &self.lookup_state {
            let mut state = snapshot.state.clone();
            state.source_variables = Arc::new(state.source_variables.relocated(relocation));
            state.variable_frame = relocation.frame(&state.variable_frame);
            self.lookup_state = Some(Arc::new(SourceLookupSnapshot::in_realm(
                state,
                snapshot.realm,
            )));
        }
    }

    /// Contents bytes frozen for one original written argument, before later argv.
    /// This excludes alias prefixes and does not flatten an expanded word into
    /// actual argument positions. It grants no native object representation,
    /// coercion-erasure or command-dispatch licence.
    #[must_use]
    pub fn evaluated_written_argument_value(&self, argument: usize) -> Option<&str> {
        self.evaluated_argument_values.get(argument)?.as_deref()
    }

    /// Resolve another command word in this exact immutable interpreter point.
    /// Uncertainty and possible document implementations remain represented.
    #[must_use]
    pub fn lookup_command_word(&self, head: &str) -> Self {
        self.lookup_state.as_ref().map_or_else(
            || Self {
                unknown: true,
                variable_context: Arc::clone(&self.variable_context),
                variable_frame: self.variable_frame.clone(),
                ..Self::default()
            },
            |snapshot| {
                let mut binding =
                    source_binding_projection(&snapshot.state, head, &self.lookup_namespace_key);
                binding.lookup_state = Some(Arc::clone(snapshot));
                binding
            },
        )
    }

    /// Candidate lexical layout of this original static head, captured before
    /// its arguments ran. Source geometry and logical grammar are retained;
    /// replaced/non-native implementations cannot borrow catalogue contracts.
    /// This query supplies no normal handler, entered body or absence proof.
    pub(crate) fn original_compilation_lookup_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        let snapshot = self.compiler_lookup_state.as_ref()?;
        let site = CommandAllocationSite {
            source: Arc::clone(snapshot.state.current_source_origin.as_ref()?),
            offset: tokens.words().first()?.source().span.start(),
        };
        original_site_operand_layout_advice(
            &site,
            tokens,
            snapshot,
            &self.lookup_namespace_key,
            self.original_lexer_config_for_tokens(tokens)?,
        )
    }

    pub(crate) fn frozen_written_words(
        &self,
    ) -> Option<&[crate::registry_invocation::EffectiveInvocationWord]> {
        self.frozen_written_words.as_deref()
    }

    fn same_dispatch_projection(&self, other: &Self) -> bool {
        let mut left = self.clone();
        let mut right = other.clone();
        left.lookup_state = None;
        right.lookup_state = None;
        left.normal_result_observations = None;
        right.normal_result_observations = None;
        left.declaration_layout_observations = None;
        left.declaration_flow_inventory = None;
        right.declaration_layout_observations = None;
        right.declaration_flow_inventory = None;
        left == right
    }

    fn join_original_operand_receipts(&mut self, other: &Self) {
        if self.original_variable_operands != other.original_variable_operands {
            self.original_variable_operands = None;
        }
        if other.entered_execution_observer.observed() {
            self.entered_execution_observer = SourceObserverExecution::MayObserved;
        }
        if self.runtime_reachability != other.runtime_reachability {
            self.runtime_reachability = SourceRuntimeReachability::Unknown;
        }
        if self.native_operand_layout != other.native_operand_layout {
            self.native_operand_layout = None;
        }
        if self.original_written_words != other.original_written_words {
            self.original_written_words = None;
        }
        if self.original_compiler_words != other.original_compiler_words {
            self.original_compiler_words = None;
        }
        if self.conditional_expression_evaluations != other.conditional_expression_evaluations {
            self.conditional_expression_evaluations = None;
        }
        if self.frozen_written_words != other.frozen_written_words {
            self.frozen_written_words = None;
        }
        if self.frozen_written_names != other.frozen_written_names {
            self.frozen_written_names = None;
        }
        if self.frozen_head_object != other.frozen_head_object {
            self.frozen_head_object = None;
        }
        self.method_prefix_arguments
            .retain(|prefix| other.method_prefix_arguments.contains(prefix));
    }

    fn join(&mut self, other: &Self) {
        self.join_original_operand_receipts(other);
        self.join_compiled_execution(other);
        self.join_invocation_reads(other);
        self.original_argument_completion
            .join(other.original_argument_completion);
        self.join_normal_variable_continuation(other);
        if self.normal_result_observations != other.normal_result_observations {
            self.normal_result_observations = None;
        }
        if self.rhs_read_store_observations != other.rhs_read_store_observations {
            self.rhs_read_store_observations = None;
        }
        self.join_object_callback_effects(other);
        join_evaluated_words(
            &mut self.evaluated_argument_values,
            &other.evaluated_argument_values,
        );
        compiled_invocation::join_argument_words(
            &mut self.evaluated_argument_words,
            &other.evaluated_argument_words,
        );
        if let (Some(state), Some(incoming)) = (&mut self.lookup_state, &other.lookup_state)
            && state.realm == incoming.realm
        {
            let snapshot = Arc::make_mut(state);
            snapshot.state.join(&incoming.state);
            // The joined state owns a new immutable semantic point. A cloned
            // snapshot must not retain its predecessor's derived fingerprint.
            snapshot.fingerprint = OnceLock::new();
            snapshot.compiler_name_effects =
                original_compiler_effects::SourceCompilerNameEffectsCache::default();
        } else {
            self.lookup_state = None;
        }
        if self.lookup_namespace_key != other.lookup_namespace_key {
            self.lookup_state = None;
        }
        if self.compiler_lookup_state != other.compiler_lookup_state {
            self.compiler_lookup_state = None;
            self.native_structured_preparation = None;
            self.native_switch_preparation = None;
        }
        if self.declaration_flow_inventory != other.declaration_flow_inventory {
            self.declaration_flow_inventory = None;
        }
        if self.declaration_layout_observations != other.declaration_layout_observations {
            self.declaration_layout_observations = None;
        }
        if self.compiler_policy != other.compiler_policy {
            self.compiler_policy = None;
        }
        if self.dispatch_site != other.dispatch_site {
            self.dispatch_site = None;
        }
        if self.lookup_word != other.lookup_word {
            self.lookup_word = None;
        }
        self.provider_state_taint
            .extend(other.provider_state_taint.iter().cloned());
        self.provider_state_taint.sort();
        self.provider_state_taint.dedup();
        if self.declared_command != other.declared_command {
            self.declared_command = None;
        }
        if self.catalogue_command != other.catalogue_command {
            self.catalogue_command = None;
        }
        self.unknown |= other.unknown;
        self.may_be_absent |= other.may_be_absent;
        self.known_namespaces
            .retain(|namespace| other.known_namespaces.contains(namespace));
        self.existing_namespace_cells
            .retain(|cell| other.existing_namespace_cells.contains(cell));
        self.namespace_cell_presence
            .join(&other.namespace_cell_presence);
        let mut variables = self.variable_context.as_ref().clone();
        variables.join(&other.variable_context);
        if self.variable_frame != other.variable_frame {
            self.variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
        }
        if self.variable_frame == crate::var_resolve::VariableExecutionFrame::Unknown {
            variables = variables.in_frame(&self.variable_frame);
        }
        self.variable_context = Arc::new(variables);
        for target in &other.targets {
            if !self.targets.contains(target) {
                self.targets.push(target.clone());
            }
        }
    }

    /// A unique target, provided every possible path reaches that target.
    #[must_use]
    pub fn proved_target(&self) -> Option<&SourceCommandTarget> {
        (!self.unknown && !self.may_be_absent && self.targets.len() == 1).then(|| &self.targets[0])
    }

    /// Original class definition manufacturer, without invoking its constructor.
    /// The returned target retains the exact live class identity and allocation.
    #[must_use]
    pub fn proved_class_definition_factory(&self) -> Option<(&str, &SourceCommandTarget)> {
        if self.runtime_reachability == SourceRuntimeReachability::Conditional {
            return None;
        }
        let target = self
            .proved_target()
            .filter(|target| target.kind == BindingKind::Class)?;
        if target.identity.as_ref()?.runtime.is_none()
            && target
                .implementation_allocation
                .as_ref()
                .is_some_and(|allocation| {
                    allocation.incarnation == AllocationIncarnation::RepeatedFresh
                })
        {
            return None;
        }
        let snapshot = &self.lookup_state.as_ref()?.state;
        if snapshot.tainted_object_dispatch.contains("*")
            || snapshot.tainted_object_dispatch.contains(&target.command)
        {
            return None;
        }
        let definition = snapshot.class_definitions.get(target.identity.as_ref()?)?;
        (definition.dispatcher.is_none()
            && target.implementation_generation == definition.implementation_generation
            && snapshot.class_definition_dependencies_hold(definition))
        .then_some((definition.factory.as_str(), target))
    }

    /// A class result whose original manufacturer and construction closure
    /// are proved at this invocation, with the captured effective arguments.
    #[must_use]
    pub fn proved_construction_result(&self, registry: &CommandRegistry) -> Option<String> {
        let target = self
            .proved_execution_target()
            .filter(|target| target.kind == BindingKind::Class)?;
        let snapshot = self.lookup_state.as_ref()?;
        let words = target
            .prepended
            .iter()
            .cloned()
            .chain(self.evaluated_argument_values.iter().map(|value| {
                value.as_ref().map_or(
                    crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                    |value| {
                        crate::registry_invocation::EffectiveInvocationWord::Literal(value.clone())
                    },
                )
            }))
            .collect::<Vec<_>>();
        let namespace = self.lookup_namespace_key.advisory_key()?;
        let mut state = boxed_source_branch(&snapshot.state);
        state
            .apply_bounded_constructor(
                target,
                &words,
                registry,
                &namespace,
                self.invocation_realm()?,
            )
            .then(|| target.command.clone())
    }

    /// Original constructor entry and its actual declaring class provider.
    /// The payload offset is in the composed effective post-head argv. Callback
    /// completion and the result's object class remain independent unknowns.
    #[must_use]
    pub fn constructor_entry(
        &self,
        registry: &CommandRegistry,
    ) -> Option<(&SourceCommandTarget, &SourceConstructorEntry, usize)> {
        let (factory, target) = self.proved_class_definition_factory()?;
        let snapshot = &self.lookup_state.as_ref()?.state;
        let grammar = registry.native_default_construction_grammar(
            factory,
            snapshot.baseline.dialect?,
            self.invocation_realm()?,
        )?;
        if !snapshot.default_construction_dependencies_hold(grammar) {
            return None;
        }
        let first = match target.prepended.first() {
            Some(word) => word.as_registry_word().literal()?,
            None => self.evaluated_argument_values.first()?.as_deref()?,
        };
        let manufacturer = grammar.manufacturer(first)?;
        let definition = snapshot.class_definitions.get(target.identity.as_ref()?)?;
        let entry = definition.constructor_entry.as_deref()?;
        let provider = definition.constructor_provider.as_ref()?;
        if !snapshot.retained_target_is_current(provider)
            || !snapshot.class_definition_dependencies_hold(definition)
        {
            return None;
        }
        let payload_from = usize::from(manufacturer.constructor_args_from);
        let payload_count = target
            .prepended
            .len()
            .checked_add(self.evaluated_argument_values.len())?
            .checked_sub(payload_from)?;
        let parameters = entry
            .formals
            .iter()
            .map(
                |(name, default)| tcl_syntax::formal_params::FormalParameter {
                    name: name.clone(),
                    default: default.clone(),
                },
            )
            .collect::<Vec<_>>();
        tcl_syntax::formal_params::bind_formal_arguments(
            &parameters,
            payload_count,
            snapshot.baseline.dialect?.parameter_grammar()?,
        )
        .ok()?;
        Some((provider, entry, payload_from))
    }
}

/// Validate original geometry and logical word rules before projecting candidate layout.
fn original_operand_layout_advice(
    binding: &SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    original_site_operand_layout_advice(
        binding.invocation_site()?,
        tokens,
        snapshot,
        namespace,
        config,
    )
}

fn original_declaration_call_layout_advice(
    binding: &SourceInvocationBinding,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    original_site_layout_advice(
        binding.invocation_site()?,
        tokens,
        snapshot,
        namespace,
        OriginalOperandLayoutPurpose::DeclarationTargets,
        config,
    )
}

fn original_site_operand_layout_advice(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    original_site_layout_advice(
        site,
        tokens,
        snapshot,
        namespace,
        OriginalOperandLayoutPurpose::RegistryMetadata,
        config,
    )
}

#[derive(Clone, Copy)]
enum OriginalOperandLayoutPurpose {
    RegistryMetadata,
    DeclarationTargets,
}

fn original_replay_words_match(
    original: &crate::ir::CommandTokens,
    retained: &crate::ir::CommandTokens,
    config: tcl_lexer::LexerConfig,
    word_values: tcl_syntax::word_rules::WordValueRules,
) -> bool {
    original.words().len() == retained.words().len()
        && original
            .words()
            .iter()
            .zip(retained.words())
            .all(|(original, retained)| {
                crate::registry_invocation::effective_invocation_word(
                    original,
                    config.escapes,
                    word_values,
                ) == crate::registry_invocation::effective_invocation_word(
                    retained,
                    config.escapes,
                    word_values,
                )
            })
}

fn original_site_layout_advice(
    site: &CommandAllocationSite,
    tokens: &crate::ir::CommandTokens,
    snapshot: &Arc<SourceLookupSnapshot>,
    namespace: &SourceNamespaceKey,
    purpose: OriginalOperandLayoutPurpose,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalCompilationLookupAdvice> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let original = crate::registry_invocation::native_compiler_replay_source(tokens, site)?;
    let dialect = snapshot.state.source_variables.invocation_dialect?;
    let logical_source = snapshot
        .state
        .logical_source_name_advice_input()
        .is_some_and(|input| {
            input.lexer_config() == config
                && source_analysis_entry::source_input_dialect(input) == dialect
        });
    // Conditional Logical command layout uses its retained source namespace
    // and lookup world. It does not require or issue a physical variable frame.
    if snapshot.state.current_source_origin.as_ref() != Some(&site.source)
        || (!logical_source
            && matches!(
                snapshot.state.variable_frame,
                crate::var_resolve::VariableExecutionFrame::Unknown
            ))
    {
        return None;
    }
    let image = tcl_lexer::SourceImage::from_bytes(
        original.as_bytes(),
        site.source.source_image().channel(),
    );
    let segments =
        crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)?;
    let [segment] = segments.as_slice() else {
        return None;
    };
    if segment.is_partial {
        return None;
    }
    let original_tokens =
        crate::ir::CommandTokens::from_segmented(&image.source_map(), config, segment);
    if !original_replay_words_match(&original_tokens, tokens, config, dialect.word_values) {
        return None;
    }
    let native = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    let head = native.first()?;
    let projection = original_static_layout_projection(
        snapshot,
        namespace,
        head,
        config,
        dialect,
        logical_source,
    )?;
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_DECLARATION").is_some() {
        eprintln!(
            "ORIGINAL_DECLARATION offset={} stage=layout logical={} frame={:?} targets={} unknown={} absent={} opaque={}",
            site.offset,
            logical_source,
            snapshot.state.variable_frame,
            projection.targets.len(),
            projection.unknown,
            projection.may_be_absent,
            snapshot.state.opaque_binding_mutation,
        );
    }
    if projection.targets.is_empty()
        || projection.targets.iter().any(|target| {
            !target.registry_backed
                && (!matches!(purpose, OriginalOperandLayoutPurpose::DeclarationTargets)
                    || target.kind != BindingKind::Proc
                    || target.implementation_allocation.is_none())
        })
    {
        return None;
    }
    Some(OriginalCompilationLookupAdvice {
        snapshot: Arc::clone(snapshot),
        targets: projection.targets,
        dialect,
        namespace: namespace.clone(),
        unknown: projection.unknown,
        may_be_absent: projection.may_be_absent,
    })
}

fn original_static_layout_projection(
    snapshot: &SourceLookupSnapshot,
    namespace: &SourceNamespaceKey,
    head: &tcl_lexer::NativeWord,
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
    logical_source: bool,
) -> Option<SourceInvocationBinding> {
    if let Some(policy) = snapshot
        .state
        .source_variables
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
    {
        let key = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            head,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )?;
        source_binding_from_original_input_in(
            &snapshot.state,
            &crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key),
            namespace,
            CommandTargetLookup::NamedSlots,
        )
    } else {
        // Independent authored simulation metadata does not issue a Native slot.
        (logical_source || dialect.authored_name_policy().is_none()).then_some(())?;
        let presentation = if logical_source {
            tcl_syntax::word_rules::original_static_word_source_bytes(head)?
        } else {
            tcl_syntax::word_rules::original_static_word_ascii_presentation(head)?
        };
        let head = std::str::from_utf8(&presentation).ok()?;
        Some(source_binding_projection_in(
            &snapshot.state,
            head,
            namespace,
            CommandTargetLookup::NamedSlots,
        ))
    }
}

#[derive(Debug, Clone)]
struct SourceBindingPoint {
    declaration_preview: bool,
    original_arguments_complete_normally: bool,
    observer_execution: SourceObserverExecution,
    realm: tcl_dialect::model::InvocationRealm,
    compilation: tcl_registry::native_compilation::NativeCompilationContext,
    compiled_execution: compiled_invocation::CompiledInvocationSelection,
    head: Option<String>,
    original_head_input: Option<crate::signature_scan::scope::SignatureSourceNameInput>,
    original_written_words: Option<Arc<original_name_value::OriginalWrittenSourceWords>>,
    evaluated_argument_values: Vec<Option<String>>,
    evaluated_argument_words: Vec<crate::registry_invocation::EffectiveInvocationWord>,
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
    dispatch: bool,
}

// Derived lookup caches do not change the retained source interpretation.
impl PartialEq for SourceBindingPoint {
    fn eq(&self, other: &Self) -> bool {
        self.declaration_preview == other.declaration_preview
            && self.original_arguments_complete_normally
                == other.original_arguments_complete_normally
            && self.observer_execution == other.observer_execution
            && self.realm == other.realm
            && self.compilation == other.compilation
            && self.compiled_execution == other.compiled_execution
            && self.head == other.head
            && self.original_head_input == other.original_head_input
            && self.original_written_words == other.original_written_words
            && self.evaluated_argument_values == other.evaluated_argument_values
            && self.evaluated_argument_words == other.evaluated_argument_words
            && self.frozen_written_words == other.frozen_written_words
            && self.frozen_written_names == other.frozen_written_names
            && self.frozen_head_object == other.frozen_head_object
            && self.method_prefix_arguments == other.method_prefix_arguments
            && self.offset == other.offset
            && self.end == other.end
            && self.namespace_key == other.namespace_key
            && self.state == other.state
            && self.dispatch == other.dispatch
    }
}

impl Eq for SourceBindingPoint {}

impl SourceBindingPoint {
    fn lookup_binding(&self, head: &str) -> SourceInvocationBinding {
        let mut proof = if self.dispatch && self.head.as_deref() == Some(head) {
            self.original_lookup_binding()
        } else {
            source_binding_projection(&self.state, head, &self.namespace_key)
        };
        proof.compiled_execution_residual = CompiledExecutionResidual::Unknown;
        proof.lookup_state = Some(Arc::clone(self.lookup_snapshot.get_or_init(|| {
            Arc::new(SourceLookupSnapshot::in_realm(
                self.state.clone(),
                self.realm,
            ))
        })));
        proof
    }

    fn binding(&self, head: &str) -> SourceInvocationBinding {
        self.binding_projection(head, false)
    }

    fn original_lookup_binding(&self) -> SourceInvocationBinding {
        // naming.consumer.original-workspace-diagnostic-refinement
        // docs/design/analysis/name-resolution-proofs/original-workspace-diagnostic-refinement.md
        // The immutable Logical entry owns this interpreted head. It does
        // not supply a Native name input or permit a source-label fallback.
        if self.state.logical_source_name_advice_input().is_some()
            && let Some(head) = self.head.as_deref()
        {
            return source_binding_projection(&self.state, head, &self.namespace_key);
        }
        self.original_head_input
            .as_ref()
            .and_then(|input| {
                source_binding_from_original_input(&self.state, input, &self.namespace_key)
            })
            .unwrap_or_else(|| SourceInvocationBinding {
                unknown: true,
                lookup_namespace: self.namespace.clone(),
                lookup_namespace_key: self.namespace_key.clone(),
                variable_context: Arc::clone(&self.state.source_variables),
                variable_frame: self.state.variable_frame.clone(),
                ..Default::default()
            })
    }

    fn original_dispatch_binding(&self) -> SourceInvocationBinding {
        self.binding_projection("", true)
    }

    fn binding_projection(&self, head: &str, original: bool) -> SourceInvocationBinding {
        let mut proof = if original {
            self.original_lookup_binding()
        } else {
            self.lookup_binding(head)
        };
        if original {
            proof.lookup_state = Some(Arc::clone(self.lookup_snapshot.get_or_init(|| {
                Arc::new(SourceLookupSnapshot::in_realm(
                    self.state.clone(),
                    self.realm,
                ))
            })));
        }
        if self.dispatch {
            proof.dispatch_site =
                self.state
                    .current_source_origin
                    .as_ref()
                    .map(|source| CommandAllocationSite {
                        source: Arc::clone(source),
                        offset: self.offset,
                    });
        }
        if self.dispatch && (original || self.head.as_deref() == Some(head)) {
            proof.original_argument_completion =
                OriginalArgumentCompletion::from_normal(self.original_arguments_complete_normally);
            proof
                .method_prefix_arguments
                .clone_from(&self.method_prefix_arguments);
            proof.entered_execution_observer = self.observer_execution;
            proof.runtime_reachability = if self.declaration_preview {
                SourceRuntimeReachability::Conditional
            } else {
                SourceRuntimeReachability::Reached
            };
            proof
                .frozen_written_words
                .clone_from(&self.frozen_written_words);
            proof
                .frozen_written_names
                .clone_from(&self.frozen_written_names);
            proof
                .original_written_words
                .clone_from(&self.original_written_words);
            proof
                .compiled_candidates
                .clone_from(&self.compiled_execution.proofs);
            proof.compiled_named_candidates = self
                .compiled_execution
                .named
                .iter()
                .map(|candidate| candidate.with_late_lookup(&self.state))
                .collect();
            proof.may_use_live_dispatch = self.compiled_execution.live;
            proof.compiled_execution_residual = self.compiled_execution.unknown.into();
            proof.native_compilation_admission = self.compiled_execution.admission;
            proof.original_compiler_preserves_names =
                self.compiled_execution.compiler_preserves_names().into();
            proof
                .native_operand_layout
                .clone_from(&self.compiled_execution.operand_layout);
            proof
                .native_compiler_admission
                .clone_from(&self.compiled_execution.admitted);
            proof
                .native_structured_preparation
                .clone_from(&self.compiled_execution.structured);
            proof
                .native_switch_preparation
                .clone_from(&self.compiled_execution.switch);
            proof
                .compiler_policy
                .clone_from(&self.compiled_execution.policy);
            proof
                .original_compiler_words
                .clone_from(&self.compiled_execution.original_words);
            proof.native_inline_rejection = self.compiled_execution.rejection;
            proof.native_handler_envelope = self
                .compiled_execution
                .stable_handler_after_original_arguments(
                    self.original_head_input.as_ref(),
                    &self.state,
                    &self.namespace_key,
                )
                .cloned();
        }
        proof
    }
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

impl DeferredSourceBody {
    fn implementation_id(&self) -> DeferredImplementationId {
        DeferredImplementationId {
            command: self.identity.clone(),
            generation: self.implementation_generation,
            allocation: self.implementation_allocation.clone(),
        }
    }
}

impl CommandAllocation {
    /// Correlate a reached implementation with an analysed declaration's bytes
    /// and offset. Loaded files retain their own identity; this value-purpose
    /// match grants no edit, native compiler, frame or object-erasure licence.
    #[must_use]
    pub fn matches_source_declaration(&self, source: &str, offset: u32) -> bool {
        self.matches_source_declaration_image(&tcl_lexer::SourceImage::document(source), offset)
    }

    /// Match the exact source bytes and input channel of a declaration.
    #[must_use]
    pub fn matches_source_declaration_image(
        &self,
        source: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> bool {
        self.site.offset == offset
            && self.incarnation != AllocationIncarnation::RepeatedFresh
            && match self.site.source.kind() {
                SourceOriginKind::Authored(text) => text == source,
                SourceOriginKind::Loaded {
                    source: text,
                    implementation_id,
                    ..
                } => !implementation_id.is_empty() && text == source,
                SourceOriginKind::Derived { .. } => false,
            }
    }
}

impl SourceCommandTarget {
    /// Whether this retained implementation was allocated by the indicated
    /// authored declaration. Derived sources and repeated fresh summaries do
    /// not become that implementation merely by sharing its numeric offset.
    #[must_use]
    pub fn matches_authored_implementation(&self, source: &str, offset: u32) -> bool {
        self.matches_authored_implementation_image(
            &tcl_lexer::SourceImage::document(source),
            offset,
        )
    }

    /// Match the implementation's original bytes and source input channel.
    #[must_use]
    pub fn matches_authored_implementation_image(
        &self,
        source: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> bool {
        self.implementation_generation == offset
            && self.implementation_allocation.as_ref().is_some_and(|allocation| {
                allocation.site.offset == offset
                    && allocation.incarnation != AllocationIncarnation::RepeatedFresh
                    && matches!(allocation.site.source.kind(), SourceOriginKind::Authored(text) if text == source)
            })
    }

    fn implementation_id(&self) -> DeferredImplementationId {
        DeferredImplementationId {
            command: self
                .identity
                .as_ref()
                .map_or_else(|| self.command.clone(), |identity| identity.origin.clone()),
            generation: self.implementation_generation,
            allocation: self.implementation_allocation.clone(),
        }
    }
}

/// Execution phase selected by a registry-owned caller-frame lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceBodyPhase {
    /// Setup; normal completion permits the main phase.
    Setup,
    /// Main script reached after setup completes normally.
    Body,
    /// Cleanup reached after catchable completions of the earlier phases.
    Cleanup,
}

/// Coverage of a lifecycle phase across every retained selected traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceBodyPhaseReachability {
    /// Every completed traversal skipped this phase.
    NotEntered,
    /// At least one completed traversal entered this phase.
    MayEntered,
    /// A traversal or its lifecycle selection remains unresolved.
    Unknown,
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

/// Temporal owner of a reached read, independently of its lexical source span.
/// A nested invocation retains the evaluation in which it was entered; native
/// body evaluation is distinct from evaluation of that invocation's argv.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceVariableEvaluationOwner {
    /// Original command-word evaluation, before dispatch.
    InvocationArguments {
        /// Exact invocation in its authored or materialised source instance.
        invocation: CommandAllocationSite,
        /// Enclosing evaluation, when this invocation is itself a substitution.
        parent: Option<Arc<Self>>,
    },
    /// Expression evaluation performed by a selected native invocation.
    NativeExpression {
        /// Invocation selecting this native expression evaluation.
        invocation: CommandAllocationSite,
        /// Enclosing evaluation, when native evaluation occurs in a substitution.
        parent: Option<Arc<Self>>,
    },
    /// Post-argv execution of a reached invocation and its entered bodies.
    InvocationBody {
        /// Actual caller site, independent of the callee's lexical body source.
        invocation: CommandAllocationSite,
        /// Active enclosing evaluation when this call was itself entered there.
        parent: Option<Arc<Self>>,
    },
    /// The same lexical read was reached from several temporal evaluations.
    Alternatives(Vec<Self>),
    /// No exact temporal evaluation owner was retained.
    Unspecified,
}

impl SourceVariableEvaluationOwner {
    fn is_argument_evaluation_of(&self, site: &CommandAllocationSite) -> bool {
        match self {
            Self::InvocationArguments { invocation, parent } => {
                invocation == site
                    || parent
                        .as_deref()
                        .is_some_and(|parent| parent.is_argument_evaluation_of(site))
            }
            Self::NativeExpression { parent, .. } | Self::InvocationBody { parent, .. } => parent
                .as_deref()
                .is_some_and(|parent| parent.is_argument_evaluation_of(site)),
            Self::Alternatives(owners) => owners
                .iter()
                .any(|owner| owner.is_argument_evaluation_of(site)),
            Self::Unspecified => false,
        }
    }

    fn is_direct_arguments_of(&self, site: &CommandAllocationSite) -> bool {
        match self {
            Self::InvocationArguments { invocation, .. } => invocation == site,
            Self::Alternatives(owners) => owners
                .iter()
                .any(|owner| owner.is_direct_arguments_of(site)),
            Self::NativeExpression { .. } | Self::InvocationBody { .. } | Self::Unspecified => {
                false
            }
        }
    }

    fn is_expression_evaluation_of(&self, site: &CommandAllocationSite) -> bool {
        match self {
            Self::NativeExpression { invocation, parent } => {
                invocation == site
                    || parent
                        .as_deref()
                        .is_some_and(|parent| parent.is_expression_evaluation_of(site))
            }
            Self::InvocationArguments { parent, .. } | Self::InvocationBody { parent, .. } => {
                parent
                    .as_deref()
                    .is_some_and(|parent| parent.is_expression_evaluation_of(site))
            }
            Self::Alternatives(owners) => owners
                .iter()
                .any(|owner| owner.is_expression_evaluation_of(site)),
            Self::Unspecified => false,
        }
    }

    fn join(&mut self, other: Self) {
        if *self == other {
            return;
        }
        if let Self::Alternatives(owners) = self {
            if !owners.contains(&other) {
                owners.push(other);
            }
        } else {
            *self = Self::Alternatives(vec![self.clone(), other]);
        }
    }
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

impl SourceVariableAccess {
    /// Physical address frozen after index evaluation and before the read's
    /// own observers. Disagreeing reached addresses retain the original
    /// unresolved lookup rather than selecting one evaluated index.
    #[must_use]
    pub(crate) fn place_in_context(
        &self,
        context: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> crate::place::Place {
        self.selected_places
            .iter()
            .find(|selected| selected.context.as_ref() == context)
            .and_then(|selected| selected.place.as_deref())
            .or_else(|| {
                if context != self.variable_context.as_ref() {
                    return None;
                }
                let place = self.selected_places.first()?.place.as_deref()?;
                self.selected_places
                    .iter()
                    .all(|selected| selected.place.as_deref() == Some(place))
                    .then_some(place)
            })
            .map_or_else(
                || {
                    crate::var_resolve::resolve_substitution_access(
                        &self.original_spelling,
                        context,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    )
                },
                Clone::clone,
            )
    }

    fn retain_selected_place(
        &mut self,
        context: &Arc<crate::var_resolve::ResolveContext>,
        place: &crate::place::Place,
    ) {
        if let Some(selected) = self
            .selected_places
            .iter_mut()
            .find(|selected| selected.context == *context)
        {
            if selected.place.as_deref() != Some(place) {
                selected.place = None;
            }
        } else {
            self.selected_places.push(SelectedSourceVariablePlace {
                context: Arc::clone(context),
                place: Some(Arc::new(place.clone())),
            });
        }
    }

    /// Actual instance allocation and current dispatch at this exact read.
    /// Nominal type labels and an unchanged variable version cannot supply it.
    #[must_use]
    pub fn proved_object_instance(&self) -> Option<&SourceObjectInstanceProof> {
        self.object_instance.as_deref()
    }

    /// Actual physical read contexts before their compatibility lattice join.
    /// Distinct activations remain distinct; this supplies no logical-slot alias.
    #[must_use]
    pub fn context_alternatives(&self) -> &[Arc<crate::var_resolve::ResolveContext>] {
        &self.context_alternatives
    }

    /// Whether retained contexts leave additional address uncertainty.
    #[must_use]
    pub fn context_residual(&self) -> SourceVariableReadResidual {
        if self
            .context_alternatives
            .iter()
            .any(|context| context.dynamic_bindings)
        {
            SourceVariableReadResidual::Unknown
        } else {
            SourceVariableReadResidual::Closed
        }
    }

    fn selected_namespace_context(&self, namespace: &SourceNamespaceKey) -> Option<Self> {
        let matches = |context: &crate::var_resolve::ResolveContext| {
            context
                .namespace_identity
                .as_ref()
                .is_some_and(|identity| identity == namespace)
                || (context.namespace_identity.is_none()
                    && matches!(namespace, SourceNamespaceKey::Authored(text) if text == &context.namespace))
        };
        let contexts = self
            .context_alternatives
            .iter()
            .filter(|context| matches(context))
            .cloned()
            .collect::<Vec<_>>();
        let mut context = contexts.first()?.as_ref().clone();
        for other in contexts.iter().skip(1) {
            context.join(other);
        }
        let mut selected = self.clone();
        selected.variable_context = Arc::new(context);
        selected.context_fingerprint = Self::fingerprint(&selected.variable_context);
        selected.context_alternatives = contexts;
        selected
            .selected_places
            .retain(|place| matches(&place.context));
        // Joined source ownership and receiver uncertainty remain withdrawn;
        // selecting a namespace cannot recreate a discarded read/result proof.
        Some(selected)
    }

    /// Relocate physical variable proof while retaining lexical/temporal source ownership.
    #[must_use]
    pub fn relocated_variables(
        &self,
        relocation: &crate::var_resolve::VariableProofRelocation,
    ) -> Self {
        let mut access = self.clone();
        access.object_instance = None;
        access.selected_places = self
            .selected_places
            .iter()
            .map(|selected| SelectedSourceVariablePlace {
                context: Arc::new(selected.context.relocated(relocation)),
                place: selected
                    .place
                    .as_ref()
                    .map(|place| Arc::new(relocation.place(place))),
            })
            .collect();
        access.variable_context = Arc::new(self.variable_context.relocated(relocation));
        access.context_alternatives = self
            .context_alternatives
            .iter()
            .map(|context| Arc::new(context.relocated(relocation)))
            .collect();
        access.context_fingerprint = Self::fingerprint(&access.variable_context);
        access
    }

    /// Select one exact lexical read from a retained access inventory.
    /// An enclosing invocation or another read of the same name cannot supply
    /// its context. Duplicate records at the exact site remain unresolved,
    /// including duplicates with a different spelling.
    #[must_use]
    pub fn find_at_source<'a>(
        accesses: &'a [Self],
        source: &crate::ir::SourceSite,
        original_spelling: &str,
    ) -> Option<&'a Self> {
        Self::find_at_site(accesses, source)
            .filter(|access| access.original_spelling == original_spelling)
    }

    /// Retrieve the original spelling of one unique exact source reference.
    /// Conflicting retained records at that site remain unresolved; a matching
    /// name elsewhere in the containing word is never consulted.
    #[must_use]
    pub fn find_at_site<'a>(
        accesses: &'a [Self],
        source: &crate::ir::SourceSite,
    ) -> Option<&'a Self> {
        let mut candidates = accesses.iter().filter(|access| access.source == *source);
        let candidate = candidates.next()?;
        candidates.next().is_none().then_some(candidate)
    }

    /// Containment in an authored command range uses the original token's
    /// byte extent. Native read sites retain their grammar-specific inner end.
    #[must_use]
    pub fn is_contained_in(&self, span: tcl_lexer::Span) -> bool {
        u32::try_from(self.original_spelling.len())
            .ok()
            .and_then(|length| self.source.span.start().checked_add(length))
            .is_some_and(|end| {
                self.source.span.start() >= span.start() && end <= span.end().saturating_add(1)
            })
    }

    fn fingerprint(context: &crate::var_resolve::ResolveContext) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        context.hash(&mut hasher);
        std::hash::Hasher::finish(&hasher)
    }
}

impl PartialEq for SourceVariableAccess {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.owner == other.owner
            && self.original_spelling == other.original_spelling
            && self.context_fingerprint == other.context_fingerprint
            && self.variable_context == other.variable_context
            && self.context_alternatives == other.context_alternatives
            && self.object_instance == other.object_instance
            && self.selected_places == other.selected_places
    }
}

impl Eq for SourceVariableAccess {}

impl Hash for SourceVariableAccess {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.source.hash(state);
        self.owner.hash(state);
        self.original_spelling.hash(state);
        self.context_fingerprint.hash(state);
        self.context_alternatives.hash(state);
        self.object_instance.hash(state);
        for selected in &self.selected_places {
            Self::fingerprint(&selected.context).hash(state);
            selected.place.hash(state);
        }
    }
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

/// Reached implicit expression dispatch, sampled after its argument evaluations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceImplicitMathInvocation {
    /// Exact source instance owning the expression AST.
    pub origin: Arc<SourceOriginId>,
    /// AST function-call start in that source instance.
    pub site: u32,
    /// Function name retained by the expression parser.
    pub function: String,
    /// Actual lookup evidence also proves execution observers absent.
    pub unobserved: bool,
    pub(crate) object_callback_effects: Option<object_callbacks::ObjectCallbackEffects>,
    /// Actual mutable command-table lookup; absent for a fixed-table engine.
    pub command_binding: Option<SourceInvocationBinding>,
    /// Actual fixed table retained from interpreter entry, independently of names.
    pub fixed_functions: Option<tcl_runtime_api::native_compilation::NativeMathFunctionTable>,
    /// Actual interpreter owner plus fixed registration identities for emitted guards.
    pub fixed_prerequisite:
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
}

impl SourceImplicitMathInvocation {
    /// Actual original operand objects have closed native conversion effects.
    /// Implementation identity and successful value analysis are independent.
    pub(crate) fn object_callback_effects_closed(&self) -> bool {
        self.object_callback_effects == Some(object_callbacks::ObjectCallbackEffects::Closed)
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

/// Retained source-point command and substitution proofs for one analysed script.
#[derive(Debug, Clone, Default)]
pub struct SourceCommandBindings {
    declaration_preview_depth: usize,
    compilation_scope: tcl_runtime_api::SourceCompilationScope,
    lexer_config: Option<tcl_lexer::LexerConfig>,
    declaration_layouts: SharedSourceInventory<declaration_layout::DeclarationLayouts>,
    original_script_activations: Vec<Arc<declaration_layout::OriginalDiagnosticFrameEntry>>,
    pre_handler_failures: SharedSourceInventory<pre_handler_failure::PreHandlerFailures>,
    unrepresented_entries: SharedSourceInventory<pre_handler_failure::UnrepresentedEntries>,
    unpositioned_projections: SourceHeadProjectionCache,
    declaration_flow_cache: declaration_flow::SourceDeclarationFlowCache,
    root_origin: Option<Arc<SourceOriginId>>,
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
    compiler_invocations: SharedSourceInventory<
        BTreeMap<CommandAllocationSite, Vec<compiler_inventory::SourceCompilerInvocation>>,
    >,
    runtime_coverage: SharedSourceInventory<BTreeMap<CommandAllocationSite, (usize, bool)>>,
}

// Derived lookup caches do not change the retained source interpretation.
impl PartialEq for SourceCommandBindings {
    fn eq(&self, other: &Self) -> bool {
        self.declaration_preview_depth == other.declaration_preview_depth
            && self.lexer_config == other.lexer_config
            && self.declaration_layouts == other.declaration_layouts
            && self.pre_handler_failures == other.pre_handler_failures
            && self.unrepresented_entries == other.unrepresented_entries
            && self.root_origin == other.root_origin
            && self.origin_points == other.origin_points
            && self.implicit_math_invocations == other.implicit_math_invocations
            && self.invocation_reads == other.invocation_reads
            && self.normal_variable_continuations == other.normal_variable_continuations
            && self.original_variable_invocations == other.original_variable_invocations
            && self.invocation_normal_results == other.invocation_normal_results
            && self.conditional_procedure_results == other.conditional_procedure_results
            && self.object_callback_effects == other.object_callback_effects
            && self.rhs_read_store_observations == other.rhs_read_store_observations
            && self.definition_method_references == other.definition_method_references
            && self.expression_preparations == other.expression_preparations
            && self.conditional_expression_evaluations == other.conditional_expression_evaluations
            && self.origin_variable_accesses == other.origin_variable_accesses
            && self.implicit_fixed_math_invocations == other.implicit_fixed_math_invocations
            && self.fixed_math_diagnostic_presence == other.fixed_math_diagnostic_presence
            && self.entered_scripts == other.entered_scripts
            && self.entered_words == other.entered_words
            && self.points == other.points
            && self.dispatch_points == other.dispatch_points
            && self.final_state == other.final_state
            && self.deferred == other.deferred
            && self.rule_declarations == other.rule_declarations
            && self.deferred_outcomes == other.deferred_outcomes
            && self.future_bodies == other.future_bodies
            && self.active_calls == other.active_calls
            && self.active_variable_observers == other.active_variable_observers
            && self.called_implementations == other.called_implementations
            && self.phases == other.phases
            && self.lifecycle_coverage == other.lifecycle_coverage
            && self.variable_accesses == other.variable_accesses
            && self.compilation_failures == other.compilation_failures
            && self.compilation_boundaries == other.compilation_boundaries
            && self.compilation_provider_required == other.compilation_provider_required
            && self.compilation_sources == other.compilation_sources
            && self.compiled_children == other.compiled_children
            && self.compiler_invocations == other.compiler_invocations
            && self.runtime_coverage == other.runtime_coverage
    }
}

impl Eq for SourceCommandBindings {}

/// Resolve the potentially large descriptor snapshot outside recursive source
/// frames. Only its owned heap pointer remains live while nested scripts run.
#[inline(never)]
fn resolve_source_invocation_facts(
    registry: &CommandRegistry,
    context: Option<tcl_registry::model::semantic::SemanticContext>,
    invocation: tcl_registry::InvocationWords<'_>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<Box<tcl_registry::InvocationFacts>> {
    tcl_registry::model::semantic::resolve_structured_invocation_in_realm(
        registry, context, invocation, realm,
    )
    .resolved()
    .map(|resolved| Box::new(resolved.facts()))
    .or_else(|| {
        registry
            .native_registration_invocation_facts(invocation)
            .map(Box::new)
    })
}

#[inline(never)]
fn resolve_source_success_facts(
    registry: &CommandRegistry,
    context: Option<tcl_registry::model::semantic::SemanticContext>,
    invocation: tcl_registry::InvocationWords<'_>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<Box<tcl_registry::InvocationFacts>> {
    tcl_registry::model::semantic::resolve_structured_invocation_in_realm(
        registry, context, invocation, realm,
    )
    .resolved()
    .map(|resolved| Box::new(resolved.facts_after_success()))
    .or_else(|| {
        registry
            .native_registration_success_facts(invocation)
            .map(Box::new)
    })
}

fn native_procedure_parameters(
    facts: &InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
) -> Option<Vec<tcl_syntax::formal_params::FormalParameter>> {
    let grammar = arguments
        .dialect()
        .and_then(tcl_registry::InvocationDialect::parameter_grammar)?;
    let source = facts.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::ParamList)
            .then(|| arguments.literal_at(facts.argument_offset + usize::from(*index)))
            .flatten()
    })?;
    tcl_syntax::formal_params::parse_formal_parameters_in(source, grammar).ok()
}

fn seed_deferred_inputs(
    branch: &mut ModuleCommandBindings,
    root: &DeferredSourceBody,
    frame: &crate::var_resolve::VariableExecutionFrame,
    registry: &CommandRegistry,
) {
    if root.event.is_none() {
        branch.source_variables = Arc::new(branch.source_variables.in_frame(frame));
        branch.variable_frame = frame.clone();
        Arc::make_mut(&mut branch.source_variables).widen_deferred_namespace_inputs();
        if let Some(statics) = &root.statics {
            Arc::make_mut(&mut branch.source_variables).install_callable_statics(statics, registry);
        }
        if let Some(topology) = &root.original_parameters {
            topology.seed_unknown(Arc::make_mut(&mut branch.source_variables), registry);
        } else if let Some(grammar) = branch
            .source_variables
            .invocation_dialect
            .and_then(tcl_registry::InvocationDialect::parameter_grammar)
            && let Ok(parameters) = tcl_syntax::formal_params::bind_formal_arguments(
                &root.parameters,
                root.parameters.len(),
                grammar,
            )
        {
            use tcl_syntax::formal_params::FormalArgumentBinding as Binding;
            for parameter in parameters {
                match parameter {
                    Binding::Value { parameter, .. } | Binding::Default { parameter } => {
                        Arc::make_mut(&mut branch.source_variables)
                            .bind_unknown_incoming(&root.parameters[parameter].name, registry);
                    }
                    Binding::Rest { name, .. } => {
                        Arc::make_mut(&mut branch.source_variables)
                            .bind_unknown_incoming(&name, registry);
                    }
                    Binding::CallerLink { name, .. } => {
                        crate::variable_bindings::bind_caller_reference(
                            Arc::make_mut(&mut branch.source_variables),
                            &name,
                            None,
                            registry,
                        );
                    }
                }
            }
        }
    }
}

/// Clone a binding branch outside recursive frames, retaining it on the heap.
#[inline(never)]
fn boxed_source_branch(state: &ModuleCommandBindings) -> Box<ModuleCommandBindings> {
    Box::new(state.clone())
}

/// Move a completed branch without materialising its owned state in a driver
/// frame that remains live through the next recursive command/body descent.
#[inline(never)]
fn publish_source_branch(state: &mut ModuleCommandBindings, branch: Box<ModuleCommandBindings>) {
    *state = *branch;
}

fn original_variable_arena(
    original: Option<&(&str, crate::ir::SourceSite)>,
    state: &ModuleCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_lexer::ExecutablePartArena> {
    original.and_then(|(raw, read_site)| {
        let image = state.current_source_origin.as_ref()?.source_image();
        let end = read_site
            .span
            .start()
            .checked_add(u32::try_from(raw.len()).ok()?)?;
        let span = tcl_lexer::Span::new(read_site.span.start(), end);
        (image.bytes().get(span.as_range())? == raw.as_bytes()).then_some(())?;
        tcl_lexer::ExecutablePartArena::decompose(
            image.clone(),
            span,
            tcl_lexer::word_parts::SubstFlags::default(),
            config,
        )
        .ok()
    })
}

fn original_variable_source<'s>(
    document: &'s str,
    base: u32,
    source: &crate::ir::SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Option<(&'s str, crate::ir::SourceSite)> {
    let start = usize::try_from(source.span.start().checked_sub(base)?).ok()?;
    if document.as_bytes().get(start) != Some(&b'$') {
        return None;
    }
    let reference =
        tcl_lexer::word_parts::scan_var_ref(document.as_bytes(), start, config).ok()??;
    let spelling = document.get(start..reference.next)?;
    let span = reference.source_span(document.as_bytes(), start, base)?;
    Some((
        spelling,
        crate::ir::SourceSite {
            span,
            provenance: source.provenance.clone(),
        },
    ))
}

#[derive(Clone, Copy)]
struct SourceIndexView<'a> {
    arena: &'a tcl_lexer::word_parts::ExecutablePartArena,
    offset: u32,
}

enum SourceIndexFrame {
    List {
        id: tcl_lexer::word_parts::PartListId,
        next: usize,
        outcomes: Box<SourceOutcomes>,
        value: Option<String>,
        native_value: Option<original_name_value::OriginalProducedNameValue>,
        native_complete: bool,
    },
    Variable {
        id: tcl_lexer::word_parts::PartListId,
        position: usize,
    },
}

fn join_index_fragment(
    outcomes: &mut SourceOutcomes,
    value: &mut Option<String>,
    fragment: &SourceOutcomes,
    native_value: &mut Option<original_name_value::OriginalProducedNameValue>,
    native_complete: &mut bool,
) {
    if *native_complete {
        *native_value = match (native_value.take(), fragment.normal_name_value.as_deref()) {
            (None, Some(component)) => Some(component.clone()),
            (Some(value), Some(component)) => value.concatenated(component),
            _ => None,
        };
        *native_complete = native_value.is_some();
    }
    *value = value
        .take()
        .zip(fragment.normal_value.as_ref())
        .map(|(mut text, fragment)| {
            text.push_str(&fragment.text);
            text
        });
    outcomes.join(fragment);
}

fn index_normal_result(
    outcomes: &SourceOutcomes,
    value: Option<String>,
) -> Option<Arc<native_result::EvaluatedSourceValue>> {
    outcomes.normal.as_ref().and(value).map(|text| {
        Arc::new(native_result::EvaluatedSourceValue {
            text,
            representation: tcl_syntax::value::ValueRepresentation::Unknown,
            numeric: None,
        })
    })
}

fn index_text_fragment(
    source: SourceIndexView<'_>,
    component: &tcl_lexer::word_parts::SpannedExecutablePart,
    state: &ModuleCommandBindings,
) -> SourceOutcomes {
    let mut fragment = SourceOutcomes::normal(state);
    fragment.normal_name_value = state.source_variables.execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        .and_then(|policy| {
            let value = crate::signature_scan::scope::SignatureSourceNameValue::from_original_executable_text_fragment(
                source.arena, source.arena.image(), source.arena.config(), component.span, policy)?;
            original_name_value::OriginalProducedNameValue::from_source_input(
                &crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(value), &state.source_variables)
        }).map(Arc::new);
    fragment.retain_complete_normal_evaluation();
    fragment.normal_value = source
        .arena
        .text(component)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(|text| {
            Arc::new(native_result::EvaluatedSourceValue {
                text: text.to_owned(),
                representation: tcl_syntax::value::ValueRepresentation::Unknown,
                numeric: None,
            })
        });
    fragment
}

#[derive(Clone, Copy)]
struct SourceExecutionContext<'a> {
    realm: tcl_dialect::model::InvocationRealm,
    compilation: tcl_registry::native_compilation::NativeCompilationContext,
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

impl SourceExecutionContext<'_> {
    fn withdraw_observed_operand_facts(&mut self, observed: bool) {
        if observed {
            self.written_method_prefixes = None;
            self.written_name_values = None;
            self.written_representations = None;
            self.written_variable_reads = None;
        }
    }

    fn namespace_identity(&self) -> SourceNamespaceKey {
        self.namespace_key
            .cloned()
            .unwrap_or_else(|| SourceNamespaceKey::authored(self.namespace))
    }
}

struct PreparedSourceNativeInvocation<'a> {
    arguments: Vec<tcl_registry::InvocationWord<'a>>,
    dialect: Option<tcl_registry::InvocationDialect>,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
    facts: Box<InvocationFacts>,
    original_variable_operands: OnceLock<crate::variable_bindings::OriginalVariableInvocation>,
}

impl PreparedSourceNativeInvocation<'_> {
    fn completion_route(
        &self,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> tcl_registry::completion_route::InvocationCompletionRoute {
        // Source-inspection proof: naming.list.constructor-intrinsic-return-code
        // docs/design/analysis/name-resolution-proofs/list-constructor-intrinsic-return-code.md
        if self.facts.successful_handler
            == Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::Leaf)
            && let Some(code) = self.facts.native_result.and_then(|contract| {
                contract.list_constructor_completion_code(
                    self.invocation("").arguments(),
                    self.facts.argument_offset,
                )
            })
        {
            return tcl_registry::completion_route::InvocationCompletionRoute::Tcl(code);
        }
        if state.closed_rename_receiver(&self.facts, context)
            || crate::variable_bindings::alias_registration_is_closed(
                &state.source_variables,
                &self.facts,
                self.invocation("").arguments(),
                context.registry,
            )
            || self.original_variable_operands.get().is_some_and(|operands| {
                crate::variable_bindings::variable_trace_registration_is_closed_with_original_operands(
                    &state.source_variables, &self.facts, context.registry, operands,
                )
            })
        {
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Ok,
            )
        } else {
            self.route
        }
    }

    fn invocation<'a>(&'a self, head: &'a str) -> tcl_registry::InvocationWords<'a> {
        let words = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(head),
            &self.arguments,
        );
        self.dialect
            .map_or(words, |dialect| words.with_dialect(dialect))
    }

    /// Keep the dialect-bearing argv view on the heap through body traversal.
    #[inline(never)]
    fn invocation_boxed<'a>(&'a self, head: &'a str) -> Box<tcl_registry::InvocationWords<'a>> {
        Box::new(self.invocation(head))
    }
}

enum SourceNativePreparationError {
    Unresolved(tcl_registry::completion_route::InvocationCompletionRoute),
    InvalidArguments,
}

/// Build source word carriers in a leaf frame rather than retaining their
/// inline construction temporary through recursive command substitution.
#[inline(never)]
fn source_command_tokens_boxed(
    image: &tcl_lexer::SourceImage,
    base: u32,
    config: tcl_lexer::LexerConfig,
    segment: &crate::segmenter::SegmentedCommand,
) -> Box<crate::ir::CommandTokens> {
    Box::new(crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image).with_base(base, 0, 0),
        config,
        segment,
    ))
}

/// Materialise argv and descriptors outside frames retained through descent.
#[inline(never)]
fn prepare_source_native_invocation<'a>(
    target: &'a SourceCommandTarget,
    effective: &'a [crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    registry: &CommandRegistry,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<Box<PreparedSourceNativeInvocation<'a>>, SourceNativePreparationError> {
    let mut arguments = target
        .prepended
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    arguments.extend(
        effective
            .iter()
            .skip(1)
            .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word),
    );
    let dialect = state.baseline.dialect;
    if state.baseline.native_entry.is_some() && dialect.is_none() {
        return Err(SourceNativePreparationError::Unresolved(
            tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
        ));
    }
    let mut invocation = tcl_registry::InvocationWords::structured(
        tcl_registry::InvocationWord::Literal(&target.command),
        &arguments,
    );
    if let Some(dialect) = dialect {
        invocation = invocation.with_dialect(dialect);
    }
    let route = registry
        .invocation_completion_route_in_frame(
            &target.command,
            invocation.arguments(),
            invocation
                .arguments()
                .dialect()
                .and_then(tcl_registry::InvocationDialect::authoring_query)
                .map(|query| query.with_realm(realm)),
            state.source_variables.alias_frame(),
        )
        .or_else(|| {
            registry.native_registration_completion_route(
                invocation,
                state.source_variables.alias_frame(),
            )
        })
        .unwrap_or(tcl_registry::completion_route::InvocationCompletionRoute::Unknown);
    let semantic_context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    let facts = resolve_source_invocation_facts(registry, semantic_context, invocation, realm)
        .ok_or(SourceNativePreparationError::Unresolved(route))?;
    if source_target_arguments_invalid(&facts, invocation.arguments()) {
        return Err(SourceNativePreparationError::InvalidArguments);
    }
    let facts = resolve_source_success_facts(registry, semantic_context, invocation, realm)
        .unwrap_or(facts);
    Ok(Box::new(PreparedSourceNativeInvocation {
        arguments,
        dialect,
        route,
        facts,
        original_variable_operands: OnceLock::new(),
    }))
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

#[cfg(test)]
fn trace_original_selection(
    words: &[crate::ir::WordExpr],
    site: u32,
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
    compiled: &compiled_invocation::CompiledInvocationSelection,
) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        let head = original_name_value::original_static_command_head_input(
            words,
            site,
            state,
            context.config,
        );
        eprintln!(
            "ORIGINAL_SELECTION site={} words={} head={:?} mode={:?} admission={:?} live={} unknown={} error={} namespace={:?} name_policy={:?}",
            site,
            words.len(),
            head.as_ref()
                .map(super::signature_scan::name_value::SignatureSourceNameInput::bytes),
            context.compilation.mode,
            compiled.admission,
            compiled.live,
            compiled.unknown,
            compiled.compile_error,
            context.namespace_identity(),
            state.source_variables.execution_name_policy
        );
    }
}

#[cfg(test)]
fn trace_original_arguments(
    words: &[crate::ir::WordExpr],
    site: u32,
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
    arguments: &PreparedSourceArguments,
) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        let head = original_name_value::original_command_head_input(words, site, state, *context);
        eprintln!(
            "ORIGINAL_ARGV site={} complete={} effective={:?} written={} names={} head={:?}",
            site,
            arguments.complete_normally,
            arguments.effective,
            arguments.written_arguments.len(),
            arguments
                .written_name_values
                .iter()
                .filter(|value| value.is_some())
                .count(),
            head.as_ref()
                .map(super::signature_scan::name_value::SignatureSourceNameInput::bytes)
        );
    }
}

#[cfg(debug_assertions)]
fn trace_original_generic_head(site: u32, state: &ModuleCommandBindings, has_head: bool) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
        eprintln!(
            "ORIGINAL_GENERIC_HEAD site={} input={} dynamic={} opaque={}",
            site, has_head, state.source_variables.dynamic_bindings, state.opaque_binding_mutation
        );
    }
}

#[cfg(debug_assertions)]
fn trace_original_generic_binding(site: u32, binding: Option<&SourceInvocationBinding>) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
        eprintln!(
            "ORIGINAL_GENERIC_BINDING site={} available={} unknown={} absent={} targets={}",
            site,
            binding.is_some(),
            binding.as_ref().is_some_and(|binding| binding.unknown),
            binding
                .as_ref()
                .is_some_and(|binding| binding.may_be_absent),
            binding.as_ref().map_or(0, |binding| binding.targets.len())
        );
    }
}

#[cfg(debug_assertions)]
fn trace_original_generic_result(site: u32, outcomes: &SourceOutcomes) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
        eprintln!(
            "ORIGINAL_GENERIC_RESULT site={} normal={} abrupt={} normal_dynamic={}",
            site,
            outcomes.normal.is_some(),
            outcomes.abrupt.len(),
            outcomes
                .normal
                .as_ref()
                .is_some_and(|normal| normal.source_variables.dynamic_bindings)
        );
    }
}

fn finish_index_list(
    mut outcomes: Box<SourceOutcomes>,
    value: Option<String>,
    native_value: Option<original_name_value::OriginalProducedNameValue>,
    state: &mut ModuleCommandBindings,
) -> SourceOutcomes {
    outcomes.normal_name_value = native_value
        .filter(|value| {
            outcomes
                .normal
                .as_ref()
                .is_some_and(|normal| value.is_current(&normal.source_variables))
        })
        .map(Arc::new);
    outcomes.normal_value = index_normal_result(&outcomes, value);
    outcomes.publish(state);
    *outcomes
}

struct CapturedNativeCompletionReceipts {
    mutation: Option<Box<original_command_mutation::OriginalCommandMove>>,
    alias_creation: Option<Box<original_alias_creation::OriginalAliasCreation>>,
    namespace_binding: Option<Box<original_namespace_binding::OriginalNamespaceBinding>>,
    namespace_deletion: Option<Box<original_namespace_deletion::OriginalNamespaceDeletion>>,
}

impl CapturedNativeCompletionReceipts {
    fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Self {
        let mutation = boxed_source_completion_receipt(|| {
            original_command_mutation::OriginalCommandMove::capture(native, facts, state, *context)
        });
        let alias_creation = boxed_source_completion_receipt(|| {
            original_alias_creation::OriginalAliasCreation::capture(native, facts, state, *context)
        });
        let namespace_binding = boxed_source_completion_receipt(|| {
            original_namespace_binding::OriginalNamespaceBinding::capture(
                native, facts, state, *context,
            )
        });
        let namespace_deletion = boxed_source_completion_receipt(|| {
            original_namespace_deletion::OriginalNamespaceDeletion::capture(
                native, facts, state, *context,
            )
        });
        Self {
            mutation,
            alias_creation,
            namespace_binding,
            namespace_deletion,
        }
    }

    fn captured(&self) -> bool {
        self.mutation.is_some()
            || self.alias_creation.is_some()
            || self.namespace_binding.is_some()
            || self.namespace_deletion.is_some()
    }

    fn completed(
        &self,
        outcomes: &SourceOutcomes,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        self.mutation.as_ref().is_some_and(|receipt| {
            outcomes.abrupt.is_empty()
                && outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| receipt.completed(native, facts, normal, context))
        }) || self.alias_creation.as_ref().is_some_and(|receipt| {
            outcomes.abrupt.is_empty()
                && outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| receipt.completed(native, facts, normal, context))
        }) || self.namespace_binding.as_ref().is_some_and(|receipt| {
            outcomes.abrupt.is_empty()
                && outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| receipt.completed(native, normal, context))
        }) || self.namespace_deletion.as_ref().is_some_and(|receipt| {
            outcomes.abrupt.is_empty()
                && outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| receipt.completed(native, facts, normal, context))
        })
    }
}

fn source_frame_namespace(frame: &crate::var_resolve::VariableExecutionFrame) -> &str {
    use crate::var_resolve::VariableExecutionFrame;
    match frame.layout() {
        VariableExecutionFrame::Namespace(namespace)
        | VariableExecutionFrame::NamespaceActivation { namespace, .. }
        | VariableExecutionFrame::Procedure { namespace, .. }
        | VariableExecutionFrame::Selected {
            namespace: Some(namespace),
            ..
        } => namespace,
        VariableExecutionFrame::Selected {
            namespace: None, ..
        }
        | VariableExecutionFrame::ReceiverMethod { .. }
        | VariableExecutionFrame::Global
        | VariableExecutionFrame::Unknown => "::",
        VariableExecutionFrame::NamespaceIdentity { .. } => {
            unreachable!("layout excludes identity wrappers")
        }
    }
}

/// Keep formal value representation and object identity from the same call operands.
fn retain_called_body_value_formals(
    state: &mut ModuleCommandBindings,
    called: &mut crate::var_resolve::ResolveContext,
    body: &DeferredSourceBody,
    actual_count: usize,
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) {
    source_representation::retain_value_formals(
        called,
        &body.parameters,
        actual_count,
        1,
        target,
        context,
    );
    object_instance::retain_value_formals(
        state,
        called,
        &body.parameters,
        actual_count,
        1,
        target,
        context,
    );
}

enum PreparedSourceLambda {
    Ready {
        lambda: tcl_registry::lambda_invocation::LambdaInvocation,
        parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    },
    Invalid,
    Unknown,
}

/// Selection and formal parsing precede entry into a lambda frame.
fn prepare_source_lambda(
    selection: tcl_registry::lambda_invocation::LambdaInvocationSelection,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> PreparedSourceLambda {
    use tcl_registry::lambda_invocation::LambdaInvocationSelection;
    let lambda = match selection {
        LambdaInvocationSelection::Selected(lambda) => lambda,
        LambdaInvocationSelection::Invalid => return PreparedSourceLambda::Invalid,
        LambdaInvocationSelection::Unknown => return PreparedSourceLambda::Unknown,
    };
    let Some(grammar) = dialect.and_then(tcl_registry::InvocationDialect::parameter_grammar) else {
        return PreparedSourceLambda::Unknown;
    };
    let Ok(parameters) =
        tcl_syntax::formal_params::parse_formal_parameters_in(&lambda.parameters, grammar)
    else {
        return PreparedSourceLambda::Invalid;
    };
    PreparedSourceLambda::Ready { lambda, parameters }
}

fn root_source_execution_context<'a>(
    frame: &'a crate::var_resolve::VariableExecutionFrame,
    namespace: &'a str,
    namespace_key: &'a SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
    registry: &'a CommandRegistry,
    options: SourceAnalysisOptions<'a>,
) -> SourceExecutionContext<'a> {
    SourceExecutionContext {
        realm: options.invocation_realm,
        compilation: options.native_compilation,
        compilation_snapshot: None,
        selected_compilation: None,
        original_variable_compilation: None,
        namespace,
        config,
        registry,
        depth: 0,
        frame,
        invocation_offset: 0,
        variable_read_owner: None,
        original_written_projection: None,
        written_arguments: None,
        written_values: None,
        written_name_values: None,
        written_representations: None,
        written_objects: None,
        written_method_prefixes: None,
        written_variable_reads: None,
        namespace_key: Some(namespace_key),
        expression_source: None,
    }
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

fn is_default_namespace_unknown_handler(
    state: &ModuleCommandBindings,
    handler: &TransitionSubject,
) -> bool {
    handler.literal().is_some_and(|handler| {
        state.baseline.dialect.is_some_and(|dialect| {
            dialect
                .word_values
                .split_list(handler)
                .is_ok_and(|prefix| prefix.is_empty())
        })
    })
}

struct CapturedNativeResultReceipts {
    normal_read: Option<crate::var_resolve::OriginalNormalValueRead>,
    normal_write: Option<crate::var_resolve::OriginalNormalValueWrite>,
    range: Option<Box<source_representation::CapturedRangeResult>>,
    coercion: Option<Box<container_coercion::CapturedContainerCoercion>>,
    numeric_operands: Vec<numeric_operand_cache::CapturedNumericOperandCache>,
}

/// Build physical operand receipts outside recursive body-driver frames.
#[inline(never)]
fn capture_source_native_results_boxed(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
) -> Box<CapturedNativeResultReceipts> {
    let context = *context;
    Box::new(CapturedNativeResultReceipts {
        normal_read: native_result::capture_original_read_completion(
            native,
            facts,
            state,
            context.registry,
        ),
        normal_write: native_result::original_store_receiver(
            facts,
            native.target,
            native.invocation.arguments(),
            context,
            &state.source_variables,
        )
        .and_then(|(receiver, _)| {
            state
                .source_variables
                .original_normal_value_write(&receiver, context.registry)
        }),
        range: source_representation::capture_range_result(native, facts, state, context),
        coercion: container_coercion::capture(native, facts, state, context).map(Box::new),
        numeric_operands: numeric_operand_cache::capture_indices(native, facts, state, context),
    })
}

/// Unpack post-handler receipts only after recursive body traversal has ended.
#[inline(never)]
fn finish_source_native_results(
    outcomes: &mut SourceOutcomes,
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    incoming: &ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
    receipts: &mut CapturedNativeResultReceipts,
) {
    let context = *context;
    let range_result = receipts.range.take();
    let coercion = receipts.coercion.take();
    let numeric_operands = std::mem::take(&mut receipts.numeric_operands);
    numeric_operand_cache::finish_indices(outcomes, numeric_operands, context.registry);
    container_coercion::finish(outcomes, coercion.map(|receipt| *receipt), context);
    dictionary_store::retain_dictionary_store(outcomes, native, facts, incoming, context);
    source_representation::retain_created_result(
        outcomes,
        native,
        facts,
        context,
        range_result
            .as_deref()
            .map(source_representation::CapturedRangeResult::representation),
    );
    source_representation::retain_range_result_bytes(outcomes, range_result);
    native_result::retain_store_results(
        outcomes,
        facts,
        native.target,
        native.invocation.arguments(),
        context,
    );
    method_prefix::retain_created_prefix(outcomes, native, facts, context);
    method_prefix::retain_scoped_prefix(outcomes, native, facts, context);
    method_prefix::retain_prefix_store(
        outcomes,
        facts,
        native.target,
        native.invocation.arguments(),
        context,
    );
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

impl<'a> SourceNativeInvocation<'a> {
    fn script_operands(self) -> SourceScriptOperands<'a> {
        SourceScriptOperands {
            compilation_spec: self.compilation_spec,
            compilation_selection: self.compilation_selection,
            words: self.words,
            written_arguments: self.written_arguments,
            target: self.target,
            arguments: self.invocation.arguments_ref(),
        }
    }
}

fn frozen_script_operands(
    arguments: tcl_registry::InvocationArguments<'_>,
    from: usize,
) -> Option<Vec<&str>> {
    let count = arguments.exact_argv_len()?;
    (from..count)
        .map(|index| arguments.literal_at(index))
        .collect()
}

/// Preserve the enclosing admission realm only for literal source operands.
/// Decoding literal escapes does not turn authored code into a runtime script;
/// substitution and alias-prefix values do, independently of source mapping.
fn evaluated_script_realm(
    arguments: std::ops::Range<usize>,
    operands: SourceScriptOperands<'_>,
    context: SourceExecutionContext<'_>,
) -> tcl_dialect::model::InvocationRealm {
    let rules = tcl_syntax::word_rules::WordValueRules::from_config(&context.config);
    if arguments.into_iter().all(|index| {
        operands.written_word(index).is_some_and(|word| {
            matches!(
                crate::registry_invocation::effective_invocation_word(
                    word,
                    context.config.escapes,
                    rules,
                ),
                crate::registry_invocation::EffectiveInvocationWord::Literal(_)
            )
        })
    }) {
        context.realm
    } else {
        tcl_dialect::model::InvocationRealm::InterpreterRuntime
    }
}

/// Retain the derived body context on the heap while its script descends.
/// Its native realm and compiler inheritance are selected before entry.
#[inline(never)]
fn boxed_body_operand_context<'a>(
    index: usize,
    operands: SourceScriptOperands<'_>,
    state: &ModuleCommandBindings,
    context: &SourceExecutionContext<'a>,
) -> Box<SourceExecutionContext<'a>> {
    let realm = evaluated_script_realm(index..index + 1, operands, *context);
    let context = compiled_invocation::body_context(index, operands, state, *context);
    Box::new(SourceExecutionContext { realm, ..context })
}

fn authored_braced_operand<'a>(
    native: SourceNativeInvocation<'a>,
    argument: usize,
    value: &str,
) -> Option<&'a crate::ir::SourceSite> {
    match native.script_operands().written_word(argument)? {
        crate::ir::WordExpr::BracedLiteral { text, source } if text == value => Some(source),
        _ => None,
    }
}

fn retained_script_operand(
    argument: usize,
    operands: SourceScriptOperands<'_>,
    state: &ModuleCommandBindings,
    invocation: u32,
    config: tcl_lexer::LexerConfig,
) -> Option<ExecutedScriptSource> {
    let parent = CommandAllocationSite {
        source: Arc::clone(state.current_source_origin.as_ref()?),
        offset: invocation,
    };
    if let Some(bytes) = operands.arguments.native_bytes_at(argument) {
        if let Some(crate::registry_invocation::InvocationWordOrigin::Written(written)) =
            operands.written_origin(argument)
            && let Some(protocol) =
                compiler_inventory::SourceNativeCompilerPolicy::source_protocol_of(state)
            && let Some(original) = crate::registry_invocation::original_native_compiler_words(
                parent.source.source_image(),
                operands.words,
                invocation,
                config,
            )
            && let Some(word) = original.get(written)
        {
            return Some(ExecutedScriptSource::from_original_word_value(
                parent, argument, word, bytes, protocol,
            ));
        }
        return Some(ExecutedScriptSource::materialised_image(
            parent,
            vec![argument],
            tcl_lexer::SourceImage::native(Arc::<[u8]>::from(bytes)),
        ));
    }
    let value = operands.arguments.literal_at(argument)?;
    let script = operands.written_word(argument).map_or_else(
        || ExecutedScriptSource::materialised(parent.clone(), vec![argument], value),
        |word| ExecutedScriptSource::from_word(parent.clone(), argument, word, value, config),
    );
    Some(script)
}

/// Carry both value-formal receipts through the same original lambda argv.
/// Preparation ends before recursive body descent.
#[inline(never)]
fn retain_lambda_value_formals(
    state: &mut ModuleCommandBindings,
    variables: &mut crate::var_resolve::ResolveContext,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
    actual_count: usize,
    lambda: &tcl_registry::lambda_invocation::LambdaInvocation,
    native: SourceNativeInvocation<'_>,
    context: SourceExecutionContext<'_>,
) {
    let first_actual_word = lambda.call_arguments_from + 1;
    source_representation::retain_value_formals(
        variables,
        parameters,
        actual_count,
        first_actual_word,
        native.target,
        context,
    );
    object_instance::retain_value_formals(
        state,
        variables,
        parameters,
        actual_count,
        first_actual_word,
        native.target,
        context,
    );
}

fn retained_lambda_body(
    lambda: &tcl_registry::lambda_invocation::LambdaInvocation,
    native: SourceNativeInvocation<'_>,
    state: &ModuleCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<ExecutedScriptSource> {
    let parent = CommandAllocationSite {
        source: Arc::clone(state.current_source_origin.as_ref()?),
        offset: native.segment.span.start(),
    };
    let dialect = native.invocation.arguments().dialect()?;
    let value = native
        .invocation
        .arguments()
        .literal_at(lambda.lambda_argument)?;
    let list = lambda
        .lambda_argument
        .checked_sub(native.target.prepended.len())
        .and_then(|written| native.words.get(written + 1))
        .map_or_else(
            || {
                ExecutedScriptSource::materialised(
                    parent.clone(),
                    vec![lambda.lambda_argument],
                    value,
                )
            },
            |word| {
                ExecutedScriptSource::from_word(
                    parent.clone(),
                    lambda.lambda_argument,
                    word,
                    value,
                    config,
                )
            },
        );
    let body = list.list_element(parent, lambda.lambda_argument, 1, dialect.word_values)?;
    (body.text.bytes() == lambda.body.as_bytes()).then_some(body)
}

fn note_source_command_lookup(
    state: &mut ModuleCommandBindings,
    compiled: &compiled_invocation::CompiledInvocationSelection,
    effective: &[crate::registry_invocation::EffectiveInvocationWord],
) {
    if (compiled.live || compiled.unknown)
        && let (Some(pool), Some(dialect)) =
            (&mut state.ordinary_literal_pool, state.baseline.dialect)
        && let Some(head) = effective
            .first()
            .and_then(|word| word.as_registry_word().literal())
    {
        pool.note_command_lookup(head, dialect);
    }
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

impl PreparedSourceArguments {
    fn finish_normal_completions(&mut self, observed: bool) {
        self.outcomes.normal_completion = self
            .outcomes
            .normal_completion
            .filter(|_| self.complete_normally && !observed);
        self.outcomes.complete_procedure_return = self
            .outcomes
            .complete_procedure_return
            .filter(|_| self.complete_normally && !observed);
    }

    /// Construct the owned preparation only in a nonrecursive leaf frame.
    #[inline(never)]
    fn boxed(word_count: usize) -> Box<Self> {
        Box::new(Self {
            effective: Vec::with_capacity(word_count),
            written_arguments: Arc::from([]),
            written_values: Vec::with_capacity(word_count),
            written_name_values: Vec::with_capacity(word_count),
            written_representations: Vec::with_capacity(word_count),
            written_objects: Vec::with_capacity(word_count),
            written_method_prefixes: Vec::with_capacity(word_count),
            written_variable_reads: Vec::with_capacity(word_count),
            outcomes: SourceOutcomes::default(),
            complete_normally: true,
            ready: false,
        })
    }

    #[inline(never)]
    fn into_outcomes(self: Box<Self>) -> SourceOutcomes {
        self.outcomes
    }
}

/// The evaluator owns the updated original facade; transient Copy views
/// borrow it for no longer than this holder. No view points at a helper local.
struct SourceArgumentExecutionContext<'a> {
    context: SourceExecutionContext<'a>,
    compilation: Option<original_name_value::OriginalSourceVariableCompilation<'a>>,
}

impl SourceArgumentExecutionContext<'_> {
    fn context(&self) -> SourceExecutionContext<'_> {
        SourceExecutionContext {
            original_variable_compilation: self.compilation.as_ref(),
            ..self.context
        }
    }
}

/// Capture the original argument receipts without retaining construction
/// temporaries in the recursive command driver.
#[inline(never)]
fn source_argument_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    receipts: SourceArgumentReceipts<'a>,
) -> Box<SourceArgumentExecutionContext<'a>> {
    let SourceArgumentReceipts {
        arguments,
        values,
        name_values,
        representations,
        objects,
        prefixes,
        reads,
    } = receipts;
    let context = *context;
    let compilation = context
        .original_variable_compilation
        .and_then(|original| (*original).with_evaluated_words(name_values));
    Box::new(SourceArgumentExecutionContext {
        context: SourceExecutionContext {
            original_written_projection: None,
            written_arguments: Some(arguments),
            written_values: Some(values),
            written_name_values: Some(name_values),
            original_variable_compilation: None,
            written_representations: Some(representations),
            written_objects: Some(objects),
            written_method_prefixes: Some(prefixes),
            written_variable_reads: Some(reads),
            ..context
        },
        compilation,
    })
}

#[inline(never)]
fn source_execution_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    owner: Option<&'a SourceVariableEvaluationOwner>,
    offset: u32,
) -> Box<SourceExecutionContext<'a>> {
    let context = *context;
    Box::new(SourceExecutionContext {
        invocation_offset: offset,
        variable_read_owner: owner,
        ..context
    })
}

/// Derived traversal contexts live on the heap at genuine entry changes.
/// Every unmodified field retains its exact original frame and source receipt.
#[inline(never)]
fn source_chunk_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    snapshot: Option<&'a NativeCompilationSnapshot>,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        expression_source: None,
        original_variable_compilation: None,
        compilation_snapshot: context.compilation_snapshot.or(snapshot),
        ..*context
    })
}

#[inline(never)]
fn source_selected_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    selected: &'a tcl_registry::native_compilation::NativeCompilationSelection,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        selected_compilation: Some(selected),
        original_variable_compilation: context
            .original_variable_compilation
            .filter(|original| original.selection() == selected),
        ..*context
    })
}

#[inline(never)]
fn source_body_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
    namespace: &'a str,
    frame: &'a crate::var_resolve::VariableExecutionFrame,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        namespace,
        namespace_key: frame.namespace_identity().or_else(|| {
            (context.namespace == namespace)
                .then_some(context.namespace_key)
                .flatten()
        }),
        frame,
        depth: context.depth + 1,
        ..*context
    })
}

#[inline(never)]
fn source_loop_context_boxed<'a>(
    context: &SourceExecutionContext<'a>,
) -> Box<SourceExecutionContext<'a>> {
    Box::new(SourceExecutionContext {
        compilation: tcl_registry::native_compilation::NativeCompilationContext {
            loop_depth: context.compilation.loop_depth.saturating_add(1),
            ..context.compilation
        },
        ..*context
    })
}

#[inline(never)]
fn retain_source_native_result(
    outcomes: &mut SourceOutcomes,
    incoming: &ModuleCommandBindings,
    target: &SourceCommandTarget,
    arguments: &[crate::registry_invocation::EffectiveInvocationWord],
    context: &SourceExecutionContext<'_>,
) {
    outcomes.retain_native_result(incoming, target, arguments, *context);
}

struct SourceNativeDefinition {
    statics: Option<crate::raw_binding::CapturedStaticBindings>,
}

fn original_procedure_publication_key(
    native: SourceNativeInvocation<'_>,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
) -> Option<SourceCommandKey> {
    original_command_table::OriginalCommandOperands::capture(native, facts, state, context)
        .procedure_key(facts, state, &context.namespace_identity())
}

fn closed_procedure_definition(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    #[cfg(debug_assertions)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some()
        || std::env::var_os("TCL_LSP_TRACE_SCALAR_ALPHA").is_some()
    {
        let key = original_procedure_publication_key(native, state, context, facts);
        eprintln!(
            "ORIGINAL_PROCEDURE_CLOSURE offset={} opaque={} definition_valid={} topology={} key={} holder_exists={} installed_count={}",
            native.segment.span.start(),
            state.has_opaque_domain(),
            matches!(
                facts.procedure_definition,
                Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
            ),
            formal_topology::from_invocation(native, facts, state, context).is_some(),
            key.is_some(),
            key.as_ref()
                .is_some_and(|key| state.namespaces.contains(key.holder().as_ref())),
            key.as_ref().map_or(0, |key| state
                .installed_procedure_targets(key, native.segment.span.start())
                .len()),
        );
    }
    if state.has_opaque_domain()
        || !matches!(
            facts.procedure_definition,
            Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
        )
        || formal_topology::from_invocation(native, facts, state, context).is_none()
    {
        return false;
    }
    let Some(key) = original_procedure_publication_key(native, state, context, facts) else {
        return false;
    };
    let holder = key.holder();
    state.namespaces.contains(holder.as_ref())
        && !state
            .installed_procedure_targets(&key, native.segment.span.start())
            .is_empty()
}

fn source_invocation_argument_owner(
    state: &ModuleCommandBindings,
    offset: u32,
    parent: Option<&SourceVariableEvaluationOwner>,
) -> Option<SourceVariableEvaluationOwner> {
    state.current_source_origin.as_ref().map(|source| {
        SourceVariableEvaluationOwner::InvocationArguments {
            invocation: CommandAllocationSite {
                source: Arc::clone(source),
                offset,
            },
            parent: parent.cloned().map(Arc::new),
        }
    })
}

fn empty_source_command_outcomes(
    state: &ModuleCommandBindings,
    earlier: &SourceOutcomes,
) -> SourceOutcomes {
    let mut result = SourceOutcomes::normal(state);
    result.normal_value = Some(Arc::new(native_result::EvaluatedSourceValue {
        text: String::new(),
        representation: tcl_syntax::value::ValueRepresentation::Unknown,
        numeric: None,
    }));
    result.join(earlier);
    result
}

fn procedure_body_admission_refusal(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    realm: tcl_dialect::model::InvocationRealm,
    state: &mut ModuleCommandBindings,
) -> Option<SourceOutcomes> {
    match source_procedure_body_admission(native, facts, realm) {
        Some(true) => None,
        Some(false) => Some(SourceOutcomes::invocation(
            state,
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Error,
            ),
        )),
        None => Some(opaque_source_invocation(state)),
    }
}

fn source_procedure_body_admission(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    realm: tcl_dialect::model::InvocationRealm,
) -> Option<bool> {
    use tcl_registry::native_procedure::{
        NativeProcedureDefinitionSelection as Definition, procedure_definition_body_policy,
    };
    let Some(Definition::Valid(definition)) = facts.procedure_definition else {
        return Some(true);
    };
    let dialect = native.invocation.arguments().dialect()?;
    let policy = procedure_definition_body_policy(dialect, realm)?;
    if policy == tcl_registry::native_procedure::ProcedureDefinitionBodyPolicy::EvaluatedValue {
        return Some(true);
    }
    let word = native.script_operands().written_word(definition.body_at)?;
    policy.accepts(crate::registry_invocation::native_compilation_word_shape(
        word,
    ))
}

fn transfer_native_definition(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
) -> Result<SourceNativeDefinition, crate::raw_binding::StaticCaptureResult> {
    use tcl_registry::native_procedure::NativeProcedureDefinitionSelection as Definition;
    if !matches!(facts.procedure_definition, Some(Definition::Valid(definition)) if definition.statics_at.is_some())
    {
        transfer_native_invocation(native, state, context, facts);
        return Ok(SourceNativeDefinition { statics: None });
    }
    let mut candidate = boxed_source_branch(state);
    transfer_native_invocation(native, &mut candidate, context, facts);
    let statics = prepare_source_callable_statics(native, &mut candidate, context, facts)?;
    *state = *candidate;
    Ok(SourceNativeDefinition { statics })
}

fn prepare_source_callable_statics(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
) -> Result<
    Option<crate::raw_binding::CapturedStaticBindings>,
    crate::raw_binding::StaticCaptureResult,
> {
    use crate::raw_binding::{CallableStorageIdentity, StaticCaptureResult as Capture};
    use tcl_registry::native_procedure::NativeProcedureDefinitionSelection as Definition;
    let Some(Definition::Valid(definition)) = facts.procedure_definition else {
        return Ok(None);
    };
    let Some(index) = definition.statics_at else {
        return Ok(None);
    };
    let input = native
        .original_variable_operands
        .input(index, &state.source_variables)
        .ok_or(Capture::Unknown)?;
    tcl_registry::native_procedure::parse_static_variables_bytes(
        input.bytes(),
        state
            .source_variables
            .invocation_dialect
            .ok_or(Capture::Unknown)?,
    )
    .ok_or(Capture::Unknown)?
    .map_err(|_| Capture::Invalid)?;
    let key = original_procedure_publication_key(native, state, context, facts)
        .ok_or(Capture::Unknown)?;
    let targets = state.installed_procedure_targets(&key, native.segment.span.start());
    if targets.len() != 1 {
        return Err(Capture::Unknown);
    }
    let target = targets.first().ok_or(Capture::Unknown)?;
    let allocation = target
        .implementation_allocation
        .as_ref()
        .ok_or(Capture::Unknown)?;
    let identity = CallableStorageIdentity {
        site: allocation.site.clone(),
        incarnation: allocation.incarnation,
        implementation_generation: target.implementation_generation,
    };
    match Arc::make_mut(&mut state.source_variables).capture_original_callable_statics(
        &identity,
        input,
        context.registry,
    ) {
        Capture::Prepared {
            bindings,
            may_error: false,
        } => Ok(Some(bindings)),
        Capture::Prepared {
            may_error: true, ..
        } => Err(Capture::Unknown),
        outcome => Err(outcome),
    }
}

fn opaque_source_invocation(state: &mut ModuleCommandBindings) -> SourceOutcomes {
    state.mark_opaque_binding_mutation();
    SourceOutcomes::invocation(
        state,
        tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
    )
}

fn procedure_compilation(
    dialect: Option<tcl_registry::InvocationDialect>,
) -> tcl_registry::native_compilation::NativeCompilationContext {
    use tcl_registry::native_compilation::{
        NativeBodyCompilation, NativeCompilationGrammar, NativeCompilationSpec,
    };
    NativeCompilationSpec {
        grammar: NativeCompilationGrammar::NoHook,
        operation: tcl_registry::SemanticOperationId::Invoke,
        body: NativeBodyCompilation::ProcedureObject,
    }
    .body_context(
        dialect,
        tcl_registry::native_compilation::NativeCompilationContext::default(),
    )
}

fn source_target_arguments_invalid(
    facts: &InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
) -> bool {
    use tcl_registry::native_procedure::NativeProcedureDefinitionSelection as Selection;
    if facts.arity_accepts_frozen_arguments() == Some(false) {
        return true;
    }
    let Some(selection) = facts.procedure_definition else {
        return false;
    };
    let definition = match selection {
        Selection::Invalid => return true,
        Selection::Unknown => return false,
        Selection::Valid(definition) => definition,
    };
    let invalid_parameters = arguments
        .literal_at(definition.parameters_at)
        .zip(
            arguments
                .dialect()
                .and_then(tcl_registry::InvocationDialect::parameter_grammar),
        )
        .is_some_and(|(parameters, grammar)| {
            tcl_syntax::formal_params::parse_formal_parameters_in(parameters, grammar).is_err()
        });
    invalid_parameters
        || definition
            .statics_at
            .and_then(|index| arguments.native_bytes_at(index))
            .zip(arguments.dialect())
            .is_some_and(|(statics, dialect)| {
                tcl_registry::native_procedure::parse_static_variables_bytes(statics, dialect)
                    .is_some_and(|result| result.is_err())
            })
}

fn select_contents_write_source(state: &mut ModuleCommandBindings) {
    if state.source_variables.contents_write_source() != state.current_source_origin.as_ref() {
        let origin = state.current_source_origin.clone();
        Arc::make_mut(&mut state.source_variables).set_contents_write_source(origin);
    }
}

fn transfer_native_invocation(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
) -> Option<String> {
    select_contents_write_source(state);
    if matches!(
        native_output_binding_phase(facts),
        tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation
            | tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
    ) {
        return None;
    }
    let SourceNativeInvocation {
        segment,
        invocation,
        ..
    } = native;
    let SourceExecutionContext { registry, .. } = context;
    let execution =
        crate::ir_helpers::ExecutionNamespace::SourceContext(context.namespace_identity());
    let original_operands =
        original_command_table::OriginalCommandOperands::capture(native, facts, state, context);
    let procedure = original_operands
        .procedure_key(facts, state, &context.namespace_identity())
        .map(|key| ModuleCommandBindings::command_key_label(&key).unwrap_or_default());
    if let Some(procedure) = &procedure {
        state.extend_procedure_bodies([procedure.clone()]);
    }
    let read_projection =
        crate::variable_bindings::source_variable_read_places_with_original_operands(
            facts,
            invocation.arguments(),
            &state.source_variables,
            registry,
            native.original_variable_operands,
        );
    let write_projection =
        crate::variable_bindings::source_variable_write_places_with_original_operands(
            facts,
            invocation.arguments(),
            &state.source_variables,
            registry,
            native.original_variable_operands,
        );
    if read_projection.iter().any(|place| place.observed)
        || write_projection.iter().any(|place| place.observed)
    {
        state.mark_opaque_binding_mutation();
    }
    let publication =
        command_publication::PreservingCommandPublication::capture(native, facts, state, context);
    let original_transfer = source_command_world::capture(native, facts, state, context);
    let namespace_ensures = original_namespace_ensure::OriginalNamespaceEnsureTransfer::capture(
        native, facts, state, context,
    );
    let namespace_deletions =
        original_namespace_deletion::OriginalNamespaceDeletionTransfer::capture(
            native, facts, state, context,
        );
    apply_declared_binding_transitions(
        facts,
        state,
        &execution,
        procedure.as_deref(),
        segment.span.start(),
        publication.as_ref(),
        Some(&original_operands),
    );
    if let Some(transfer) = original_transfer {
        transfer.finish(state);
    } else {
        Arc::make_mut(&mut state.original_command_world).withdraw();
    }
    let namespace_deletions =
        namespace_deletions.and_then(|transfer| transfer.after_command_transfer(state));
    let namespace_ensures =
        namespace_ensures.and_then(|transfer| transfer.after_command_transfer(state));
    transfer_native_namespace_writes(
        native,
        state,
        context,
        facts,
        original_namespace_ensure::OriginalNamespaceCellOperations {
            ensures: namespace_ensures.as_ref(),
            deletions: namespace_deletions.as_ref(),
        },
    );
    procedure
}

fn transfer_native_namespace_writes(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
    namespace_operations: original_namespace_ensure::OriginalNamespaceCellOperations<'_>,
) {
    let registry = context.registry;
    let SourceNativeInvocation {
        segment,
        invocation,
        ..
    } = native;
    let script_interpreted = facts.body_execution.is_some()
        || facts.traits.contains(tcl_registry::Traits::DEFERS_BODY)
        || !matches!(
            tcl_registry::script_body_flow::script_body_flow(facts),
            tcl_registry::script_body_flow::ScriptBodyFlow::None
        );
    state.record_provider_state_writes(
        facts,
        *invocation,
        registry,
        native.original_variable_operands,
    );
    let variables = Arc::make_mut(&mut state.source_variables);
    variables.namespace_identities = state.namespaces.iter().cloned().collect();
    variables.known_namespaces = state
        .namespaces
        .iter()
        .filter_map(SourceNamespaceKey::advisory_key)
        .collect();
    let order = native_output_order(native, facts);
    crate::variable_bindings::transfer_source_namespace_cells_with_input(
        variables,
        facts,
        invocation.arguments(),
        registry,
        crate::variable_bindings::SourceNamespaceTransfer::new(
            script_interpreted,
            segment.span.start(),
            order.as_deref(),
        )
        .with_original_operands(native.original_variable_operands)
        .with_namespace_operations(namespace_operations),
    );
}

#[inline(never)]
fn native_implementation_dependency_holds(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    effective: &[crate::registry_invocation::EffectiveInvocationWord],
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    use tcl_registry::native_compilation::NormalHandlerImplementationLookup;
    if matches!(
        context.selected_compilation,
        Some(tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. })
    ) {
        return true;
    }
    let binding = original_name_value::original_command_head_input(
        native.words,
        native.segment.span.start(),
        state,
        context,
    )
    .and_then(|input| {
        source_binding_from_original_input(state, &input, &context.namespace_identity())
    });
    let normal = facts.normal_handler_implementation_lookup(state.baseline.dialect);
    if match normal {
        NormalHandlerImplementationLookup::NoneRequired => false,
        NormalHandlerImplementationLookup::Required(lookup) => !binding
            .as_ref()
            .is_some_and(|binding| binding.proves_native_implementation_lookup(&lookup)),
        NormalHandlerImplementationLookup::RequiredPath(paths) => {
            let arguments = binding
                .as_ref()
                .and_then(SourceInvocationBinding::proved_handler_target)
                .into_iter()
                .flat_map(|target| target.prepended.iter())
                .chain(effective.iter().skip(1))
                .map(|word| word.as_registry_word())
                .collect::<Vec<_>>();
            let mut invocation = tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(""),
                &arguments,
            );
            if let Some(dialect) = state.baseline.dialect {
                invocation = invocation.with_dialect(dialect);
            }
            !binding.as_ref().is_some_and(|binding| {
                binding.proves_native_handler_path(
                    paths,
                    invocation.arguments(),
                    facts.argument_offset,
                )
            })
        }
        NormalHandlerImplementationLookup::Unknown => true,
    } {
        return false;
    }
    facts
        .native_compilation
        .and_then(|spec| {
            state
                .baseline
                .dialect
                .and_then(|dialect| spec.implementation_lookup(dialect))
        })
        .is_none_or(|lookup| {
            binding
                .as_ref()
                .is_some_and(|binding| binding.proves_native_implementation_lookup(&lookup))
        })
}

#[inline(never)]
fn transfer_native_completion_outputs(
    outcomes: &mut SourceOutcomes,
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
    context: SourceExecutionContext<'_>,
) {
    if native_output_binding_phase(facts)
        == tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation
        && let Some(normal) = &mut outcomes.normal
    {
        let before_outputs = boxed_source_branch(normal);
        transfer_native_outputs(native, facts, normal, context);
        if route.abrupt_possible() {
            let after_outputs = boxed_source_branch(normal);
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
                &before_outputs,
            );
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
                &after_outputs,
            );
        }
    }
}

fn native_output_binding_phase(
    facts: &InvocationFacts,
) -> tcl_registry::native_compilation::VariableOperandBindingPhase {
    facts.successful_handler.map_or(
        tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
        tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
    )
}

fn native_output_order(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
) -> Option<Vec<usize>> {
    use tcl_registry::native_compilation::VariableOutputLookup;
    match facts
        .successful_handler
        .map(tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_output_lookup)
    {
        Some(VariableOutputLookup::CatchSelected) => {
            let dialect = native.invocation.arguments().dialect()?;
            let tcl_registry::catch_invocation::CatchInvocationSelection::Valid(capture) =
                tcl_registry::catch_invocation::select_catch_invocation(
                    native.invocation.arguments(),
                    dialect,
                )
            else {
                return None;
            };
            Some(
                capture
                    .output_order(dialect, *native.compilation_selection)
                    .indices(capture)?
                    .into_iter()
                    .flatten()
                    .collect(),
            )
        }
        Some(VariableOutputLookup::Sequential | VariableOutputLookup::SingleTarget) => Some(
            facts
                .arg_roles
                .iter()
                .filter_map(|&(index, role)| {
                    (role == tcl_registry::ArgRole::VarWrite)
                        .then_some(facts.argument_offset + usize::from(index))
                })
                .collect(),
        ),
        None | Some(VariableOutputLookup::BodyProtocol) => None,
    }
}

fn transfer_native_outputs(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) {
    let registry = context.registry;
    select_contents_write_source(state);
    let order = native_output_order(native, facts);
    let places = crate::variable_bindings::source_variable_write_places_with_output_order_and_original_operands(
        facts,
        native.invocation.arguments(),
        &state.source_variables,
        registry,
        order.as_deref(),
        native.original_variable_operands,
    );
    if places.iter().any(|place| place.observed) {
        state.mark_opaque_binding_mutation();
    }
    state.record_provider_state_writes(
        facts,
        *native.invocation,
        registry,
        native.original_variable_operands,
    );
    let variables = Arc::make_mut(&mut state.source_variables);
    variables.namespace_identities = state.namespaces.iter().cloned().collect();
    variables.known_namespaces = state
        .namespaces
        .iter()
        .filter_map(SourceNamespaceKey::advisory_key)
        .collect();
    crate::variable_bindings::transfer_source_namespace_cells_with_input(
        variables,
        facts,
        native.invocation.arguments(),
        registry,
        crate::variable_bindings::SourceNamespaceTransfer::new(
            true,
            native.segment.span.start(),
            order.as_deref(),
        )
        .with_original_operands(native.original_variable_operands),
    );
}

/// Keep physical variable-frame construction out of recursive script frames.
#[inline(never)]
fn source_variables_in_frame(
    variables: &Arc<crate::var_resolve::ResolveContext>,
    frame: &crate::var_resolve::VariableExecutionFrame,
) -> Arc<crate::var_resolve::ResolveContext> {
    Arc::new(variables.in_frame(frame))
}

#[derive(Default)]
struct PreparedSourceChunk {
    snapshot: Option<NativeCompilationSnapshot>,
    entry_error: Option<Box<ModuleCommandBindings>>,
    failed_outcome: Option<Box<SourceOutcomes>>,
}

struct SourceNativeFrameSelection {
    namespace: String,
    namespace_key: SourceNamespaceKey,
    frame: crate::var_resolve::VariableExecutionFrame,
    variables: Option<Arc<crate::var_resolve::ResolveContext>>,
    namespace_activation: bool,
}

fn entered_receiver_body_context(
    body: &DeferredSourceBody,
    target: &SourceCommandTarget,
    receiver: Option<receiver_self::CalledBodyReceiver>,
    frame: &crate::var_resolve::VariableExecutionFrame,
    called: &crate::var_resolve::ResolveContext,
    config: tcl_lexer::LexerConfig,
) -> Option<Arc<original_receiver_body_context::OriginalReceiverBodyContext>> {
    receiver
        .and_then(|receiver| {
            original_receiver_body_context::OriginalReceiverBodyContext::at_entered_call(
                body, target, receiver, frame, called, config,
            )
        })
        .map(Arc::new)
}

fn source_lambda_frame(
    state: &ModuleCommandBindings,
    namespace: &str,
    namespace_key: &SourceNamespaceKey,
    site: u32,
) -> crate::var_resolve::VariableExecutionFrame {
    crate::var_resolve::VariableExecutionFrame::Procedure {
        namespace: namespace.to_owned(),
        identity: source_activation_name(state.current_source_origin.as_ref(), "lambda", site),
    }
    .with_namespace_identity(namespace_key.clone())
}

fn called_body_frame(
    body: &DeferredSourceBody,
    identity: String,
    namespace: &str,
    namespace_key: &SourceNamespaceKey,
) -> crate::var_resolve::VariableExecutionFrame {
    let frame = if body.receiver_method {
        crate::var_resolve::VariableExecutionFrame::ReceiverMethod { identity }
    } else {
        crate::var_resolve::VariableExecutionFrame::Procedure {
            namespace: namespace.to_owned(),
            identity,
        }
    };
    frame.with_namespace_identity(namespace_key.clone())
}

fn called_body_namespace(
    state: &ModuleCommandBindings,
    target: &SourceCommandTarget,
    body: &DeferredSourceBody,
) -> Option<SourceNamespaceKey> {
    Some(if body.receiver_method {
        body.namespace_key.clone()
    } else {
        let holders = state
            .bindings
            .iter()
            .filter(|&(_, bindings)| {
                bindings.iter().any(|binding| matches!(binding,
                    MayBinding::Target(installed) if installed.terminal
                        && installed.kind == BindingKind::Proc
                        && installed.implementation_generation == target.implementation_generation
                        && installed.implementation_allocation == target.implementation_allocation
                        && installed.token == target.identity
                ))
            })
            .map(|(slot, _)| slot.holder().into_owned())
            .collect::<BTreeSet<_>>();
        let mut holders = holders.into_iter();
        let holder = holders.next()?;
        if holders.next().is_some() {
            return None;
        }
        holder
    })
}

fn retain_source_namespace_world(
    state: &mut ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
) {
    let policy = match state.baseline.native_entry.as_ref() {
        Some(entry) => entry.execution_name_policy(),
        None => state.baseline.execution_name_policy,
    };
    let variables = Arc::make_mut(&mut state.source_variables);
    variables.execution_name_policy = policy;
    variables.retain_namespace_world(
        context.namespace_identity(),
        state.namespaces.iter().cloned(),
        policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
    );
}

fn apply_selected_native_frame_effect(
    selection: &mut SourceNativeFrameSelection,
    invocation: &tcl_registry::InvocationWords<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
) -> Result<(), tcl_registry::completion_route::InvocationCompletionRoute> {
    if let Some(effect) = facts.frame_effect.filter(|effect| {
        effect.layout == tcl_registry::frame_effect::FrameArgLayout::ScriptInSelectedFrame
    }) {
        match effect.successful_layout(invocation.arguments()).layout {
            tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. }
                if level.is_current_frame() => {}
            tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. }
                if level.is_global_frame() =>
            {
                selection.frame = crate::var_resolve::VariableExecutionFrame::Global;
                selection.namespace_key = state
                    .native_root_namespace_key()
                    .unwrap_or_else(|| SourceNamespaceKey::authored("::"));
                "::".clone_into(&mut selection.namespace);
            }
            tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. } => {
                if let Some(selected) = state.source_variables.selected_frame_context(level) {
                    selection.namespace.clone_from(&selected.namespace);
                    selection.namespace_key = selected
                        .namespace_identity
                        .clone()
                        .unwrap_or_else(|| SourceNamespaceKey::authored(&selected.namespace));
                    selection.variables = Some(Arc::new(selected));
                }
                selection.frame = crate::var_resolve::VariableExecutionFrame::Selected {
                    selector: level,
                    namespace: selection
                        .variables
                        .as_ref()
                        .map(|selected| selected.namespace.clone()),
                };
            }
            tcl_registry::frame_effect::FrameArgumentResolution::Invalid => {
                return Err(
                    tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::completion::CompletionCode::Error,
                    ),
                );
            }
            tcl_registry::frame_effect::FrameArgumentResolution::Unknown => {
                selection.frame = crate::var_resolve::VariableExecutionFrame::Unknown;
            }
        }
    }
    Ok(())
}

#[inline(never)]
fn select_native_frame(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
    facts: &InvocationFacts,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
) -> Result<SourceNativeFrameSelection, tcl_registry::completion_route::InvocationCompletionRoute> {
    let SourceNativeInvocation {
        segment,
        invocation,
        ..
    } = native;
    let namespace = context.namespace;
    let mut namespace_key = context.namespace_identity();
    if facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::NamespaceEval,
        )
    {
        let Some(target) = facts.state_transitions.declared().and_then(|transitions| {
            let mut targets =
                transitions
                    .facts()
                    .iter()
                    .filter_map(|fact| match &fact.transition {
                        StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
                            namespace,
                        }) => Some(namespace),
                        _ => None,
                    });
            let selected = targets.next()?;
            targets.all(|target| target == selected).then_some(selected)
        }) else {
            state.mark_opaque_resolution();
            return Err(route);
        };
        let selected = match target {
            tcl_registry::NamespaceTransitionTarget::Named(subject)
                if matches!(
                    state
                        .baseline
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                        .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
                    Some(tcl_syntax::naming::NativeNameProtocol::C(_))
                ) && (subject.literal().is_none()
                    || !matches!(namespace_key, SourceNamespaceKey::Authored(_))) =>
            {
                original_command_table::original_operand(native, subject, state, context).and_then(
                    |input| state.original_namespace_key_for_input(&namespace_key, &input),
                )
            }
            _ => state.ensure_namespace_key_at(target, &namespace_key, segment.span.start()),
        };
        let Some(selected) = selected else {
            state.mark_opaque_resolution();
            return Err(route);
        };
        namespace_key = selected;
    }
    let body_namespace = namespace_key
        .display()
        .unwrap_or_else(|| namespace.to_owned());
    let namespace_activation = facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::NamespaceEval,
        );
    let frame = if namespace_activation {
        crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
            namespace: body_namespace.clone(),
            identity: source_activation_name(
                state.current_source_origin.as_ref(),
                "namespace-eval",
                segment.span.start(),
            ),
        }
    } else {
        context.frame.clone()
    };
    let mut selection = SourceNativeFrameSelection {
        namespace: body_namespace,
        namespace_key,
        frame,
        variables: None,
        namespace_activation,
    };
    apply_selected_native_frame_effect(&mut selection, invocation, facts, state)?;
    if matches!(
        selection.frame.layout(),
        crate::var_resolve::VariableExecutionFrame::Unknown
            | crate::var_resolve::VariableExecutionFrame::Selected {
                namespace: None,
                ..
            }
    ) {
        return Err(tcl_registry::completion_route::InvocationCompletionRoute::Unknown);
    }
    selection.frame = selection
        .frame
        .with_namespace_identity(selection.namespace_key.clone());
    Ok(selection)
}

#[inline(never)]
fn install_selected_native_frame(
    state: &mut ModuleCommandBindings,
    frame: &crate::var_resolve::VariableExecutionFrame,
    namespace_key: &SourceNamespaceKey,
    namespace_activation: bool,
    selected_variables: Option<Arc<crate::var_resolve::ResolveContext>>,
) {
    if namespace_activation {
        let mut variables = state.source_variables.enter_called_frame(frame);
        // This exact selected Ensure/frame owns the source namespace identity.
        // Authored frame presentation deliberately supplies no native token;
        // retain the independent source key without decoding that presentation.
        variables.namespace_identity = Some(namespace_key.clone());
        state.source_variables = Arc::new(variables);
        state.variable_frame = frame.clone();
    } else if let Some(selected) = selected_variables {
        state.source_variables = selected;
        state.variable_frame = frame.clone();
    }
}

struct PreparedSourceNativeBody {
    selection: SourceNativeFrameSelection,
    parent_variables: Arc<crate::var_resolve::ResolveContext>,
    parent_frame: crate::var_resolve::VariableExecutionFrame,
    flow: tcl_registry::script_body_flow::ScriptBodyFlow,
}

/// Construct frame and body descriptors before retaining recursive descent.
#[inline(never)]
fn prepare_selected_native_body(
    native: SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: &SourceExecutionContext<'_>,
    facts: &InvocationFacts,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
) -> Result<Box<PreparedSourceNativeBody>, tcl_registry::completion_route::InvocationCompletionRoute>
{
    let context = *context;
    let flow = tcl_registry::case_bodies::script_body_flow_in_registry(
        context.registry,
        facts,
        native.invocation.arguments(),
    );
    if matches!(flow, tcl_registry::script_body_flow::ScriptBodyFlow::None) {
        return Err(route);
    }
    let parent_variables = Arc::clone(&state.source_variables);
    let parent_frame = state.variable_frame.clone();
    let mut selection = select_native_frame(native, state, context, facts, route)?;
    install_selected_native_frame(
        state,
        &selection.frame,
        &selection.namespace_key,
        selection.namespace_activation,
        selection.variables.take(),
    );
    Ok(Box::new(PreparedSourceNativeBody {
        selection,
        parent_variables,
        parent_frame,
        flow,
    }))
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
struct SourceLoopBodyIndices<'a> {
    initial: &'a [usize],
    conditions: &'a [usize],
    repeated: &'a [usize],
    continued: &'a [usize],
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

fn source_iteration_variable_lists(
    native: SourceNativeInvocation<'_>,
    registry: &CommandRegistry,
    realm: tcl_dialect::model::InvocationRealm,
) -> Vec<usize> {
    resolve_source_invocation_facts(registry, None, *native.invocation, realm)
        .map(|facts| {
            facts
                .arg_roles
                .iter()
                .filter_map(|&(index, role)| {
                    (role == tcl_registry::ArgRole::LoopVarList)
                        .then_some(facts.argument_offset + usize::from(index))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct SourceNormalCompletion;

/// A closed native leaf handler at this exact original invocation. A body's
/// normal child result cannot supply the enclosing handler's certificate.
#[derive(Clone)]
struct SourceNativeNormalCompletion {
    site: CommandAllocationSite,
    config: tcl_lexer::LexerConfig,
    words: Vec<crate::ir::WordExpr>,
    operation: tcl_registry::SemanticOperationId,
}

impl SourceNativeNormalCompletion {
    fn matches(
        &self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        self.config == context.config
            && self.words == native.words
            && self.operation == facts.operation
            && self.site.offset == native.segment.span.start()
            && state.current_source_origin.as_ref() == Some(&self.site.source)
            && !state.source_step_observed()
            && !state.source_execution_observed(None)
    }
}

#[derive(Clone, Default)]
struct SourceOutcomes {
    normal_completion: Option<SourceNormalCompletion>,
    native_normal_completion: Option<SourceNativeNormalCompletion>,
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

/// Allocate the outcome accumulator outside frames retained by live body
/// recursion. The heap owner preserves all normal/abrupt alternatives.
/// Preserve complete completion premises on the heap across body descent.
/// Construction temporaries end in this leaf before recursion begins.
#[inline(never)]
fn boxed_source_completion_receipt<T>(capture: impl FnOnce() -> Option<T>) -> Option<Box<T>> {
    // Implementation contract: naming.source.recursive-driver-state-transport
    // docs/design/analysis/name-resolution-proofs/recursive-driver-state-transport.md
    capture().map(Box::new)
}

#[inline(never)]
fn boxed_source_outcomes() -> Box<SourceOutcomes> {
    Box::new(SourceOutcomes::default())
}

/// Construct the special empty result before returning to native body dispatch.
#[inline(never)]
fn join_source_procedure_noop(outcomes: &mut SourceOutcomes, incoming: &ModuleCommandBindings) {
    let mut result = SourceOutcomes::normal(incoming);
    result.normal_value = Some(Arc::new(native_result::EvaluatedSourceValue {
        text: String::new(),
        representation: tcl_syntax::value::ValueRepresentation::Unknown,
        numeric: None,
    }));
    outcomes.join(&result);
}

impl SourceOutcomes {
    fn retain_closed_native_handler_completion(
        &mut self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        context: SourceExecutionContext<'_>,
    ) {
        let Some(normal) = self.normal.as_ref().filter(|_| self.abrupt.is_empty()) else {
            return;
        };
        let Some(source) = normal.current_source_origin.as_ref() else {
            return;
        };
        if normal.source_step_observed() || normal.source_execution_observed(None) {
            return;
        }
        self.native_normal_completion = Some(SourceNativeNormalCompletion {
            site: CommandAllocationSite {
                source: Arc::clone(source),
                offset: native.segment.span.start(),
            },
            config: context.config,
            words: native.words.to_vec(),
            operation: facts.operation,
        });
        self.retain_complete_normal_evaluation();
    }

    fn discard_normal_handler_entry(&mut self) {
        self.normal = None;
        self.normal_value = None;
        self.normal_name_value = None;
        self.normal_representation = None;
        self.normal_object = None;
        self.normal_method_prefix = None;
        self.normal_rhs_read = None;
    }

    fn retain_complete_normal_evaluation(&mut self) {
        self.normal_completion =
            (self.normal.is_some() && self.abrupt.is_empty()).then_some(SourceNormalCompletion);
    }

    fn normal(state: &ModuleCommandBindings) -> Self {
        Self {
            normal: Some(boxed_source_branch(state)),
            abrupt: Vec::new(),
            ..Self::default()
        }
    }

    fn invocation(
        state: &ModuleCommandBindings,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
    ) -> Self {
        let mut outcomes = Self::default();
        outcomes.add(route, state);
        outcomes
    }

    fn native_invocation(
        before: &ModuleCommandBindings,
        after: &ModuleCommandBindings,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
    ) -> Self {
        let mut outcomes = Self::default();
        if route.normal_possible() {
            outcomes.normal = Some(boxed_source_branch(after));
        }
        if route.abrupt_possible() {
            let mut partial = boxed_source_branch(before);
            partial.join(after);
            outcomes.add_abrupt(route, &partial);
        }
        outcomes
    }

    fn add(
        &mut self,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        state: &ModuleCommandBindings,
    ) {
        if route.normal_possible() {
            self.normal_value = None;
            self.normal_name_value = None;
            self.normal_representation = None;
            self.normal_object = None;
            self.normal_method_prefix = None;
            self.normal_rhs_read = None;
            if let Some(normal) = &mut self.normal {
                normal.join(state);
            } else {
                self.normal = Some(boxed_source_branch(state));
            }
        }
        if route.abrupt_possible() {
            self.add_abrupt(route, state);
        }
    }

    fn add_abrupt(
        &mut self,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        state: &ModuleCommandBindings,
    ) {
        self.normal_completion = None;
        self.complete_procedure_return = None;
        self.abrupt_values.retain(|(known, _)| *known != route);
        self.abrupt_name_values.retain(|(known, _)| *known != route);
        self.abrupt_objects.retain(|(known, _)| *known != route);
        if matches!(
            route,
            tcl_registry::completion_route::InvocationCompletionRoute::TailcallOrError { .. }
        ) {
            for alternative in route.alternatives() {
                self.add_abrupt(alternative, state);
            }
            return;
        }
        if let tcl_registry::completion_route::InvocationCompletionRoute::TclAlternatives(codes) =
            route
        {
            for code in codes {
                if *code != tcl_registry::completion::CompletionCode::Ok {
                    self.add_abrupt(
                        tcl_registry::completion_route::InvocationCompletionRoute::Tcl(*code),
                        state,
                    );
                }
            }
            return;
        }
        if let Some((_, previous)) = self
            .abrupt
            .iter_mut()
            .find(|(previous, _)| *previous == route)
        {
            previous.join(state);
        } else {
            self.abrupt.push((route, boxed_source_branch(state)));
        }
    }

    fn join(&mut self, other: &Self) {
        let complete_return = if self.normal.is_none() && self.abrupt.is_empty() {
            other.complete_procedure_return
        } else {
            self.complete_procedure_return
                .filter(|proof| Some(*proof) == other.complete_procedure_return)
        };
        let complete = if self.normal.is_none() && self.abrupt.is_empty() {
            other.normal_completion
        } else {
            self.normal_completion
                .filter(|proof| Some(*proof) == other.normal_completion)
        };
        if let Some(normal) = &other.normal {
            let representation = if self.normal.is_none()
                || self.normal_representation == other.normal_representation
            {
                other.normal_representation.clone()
            } else {
                None
            };
            let object = if self.normal.is_none() || self.normal_object == other.normal_object {
                other.normal_object.clone()
            } else {
                None
            };
            let rhs_read = if self.normal.is_none() || self.normal_rhs_read == other.normal_rhs_read
            {
                other.normal_rhs_read.clone()
            } else {
                None
            };
            let prefix = if self.normal.is_none()
                || self.normal_method_prefix == other.normal_method_prefix
            {
                other.normal_method_prefix.clone()
            } else {
                None
            };
            let name_value = if self.normal.is_none() {
                other.normal_name_value.clone()
            } else {
                self.normal_name_value
                    .as_ref()
                    .zip(other.normal_name_value.as_ref())
                    .and_then(|(left, right)| left.joined(right))
                    .map(Arc::new)
            };
            self.add_with_result(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Ok,
                ),
                normal,
                other.normal_value.as_ref(),
            );
            self.normal_name_value = name_value;
            self.normal_object = object;
            self.normal_method_prefix = prefix;
            self.normal_rhs_read = rhs_read;
            self.normal_representation = representation;
        }
        for (route, state) in &other.abrupt {
            let present = self.abrupt.iter().any(|(known, _)| known == route);
            let name_value = if present {
                self.abrupt_name_values
                    .iter()
                    .find(|(known, _)| known == route)
                    .map(|(_, value)| value)
                    .zip(
                        other
                            .abrupt_name_values
                            .iter()
                            .find(|(known, _)| known == route)
                            .map(|(_, value)| value),
                    )
                    .and_then(|(left, right)| left.joined(right))
                    .map(Arc::new)
            } else {
                other
                    .abrupt_name_values
                    .iter()
                    .find(|(known, _)| known == route)
                    .map(|(_, value)| Arc::clone(value))
            };
            self.join_abrupt_result(
                *route,
                state,
                other.result_for(*route),
                other.object_for(*route),
            );
            if let Some(value) = name_value {
                self.abrupt_name_values.push((*route, value));
            }
        }
        self.normal_completion = complete.filter(|_| self.abrupt.is_empty());
        self.complete_procedure_return = complete_return.filter(|route| {
            self.normal.is_none() && self.abrupt.len() == 1 && self.abrupt[0].0 == *route
        });
    }

    fn publish(&self, state: &mut ModuleCommandBindings) {
        let mut combined = self.normal.clone();
        for (_, abrupt) in &self.abrupt {
            if let Some(combined) = &mut combined {
                combined.join(abrupt);
            } else {
                combined = Some(abrupt.clone());
            }
        }
        if let Some(combined) = combined {
            *state = *combined;
        }
    }

    fn through_procedure_boundary(&self, dialect: Option<tcl_registry::InvocationDialect>) -> Self {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let mut outcomes = Self::default();
        let mut representation = self.normal.as_ref().and_then(|normal| {
            self.normal_representation
                .as_ref()
                .filter(|receipt| receipt.is_current(&normal.source_variables))
                .cloned()
        });
        if let Some(normal) = &self.normal {
            let name_value = self
                .normal_name_value
                .as_ref()
                .filter(|value| value.is_current(&normal.source_variables))
                .cloned();
            outcomes.add_with_object_result(
                Route::Tcl(tcl_registry::completion::CompletionCode::Ok),
                normal,
                self.normal_value.as_ref(),
                self.normal_object.as_ref(),
            );
            outcomes.normal_name_value = name_value;
        }
        for (route, state) in &self.abrupt {
            let converted = route.through_procedure_boundary_in(dialect);
            if converted.normal_possible() {
                // Abrupt results have no representation receipt. Their
                // conversion cannot inherit the fall-through result's object.
                representation = None;
            }
            let name_value = self
                .abrupt_name_values
                .iter()
                .find(|(known, _)| known == route)
                .map(|(_, value)| value)
                .filter(|value| value.is_current(&state.source_variables));
            let joined_name = if converted.normal_possible() {
                if outcomes.normal.is_some() {
                    outcomes
                        .normal_name_value
                        .as_ref()
                        .zip(name_value)
                        .and_then(|(left, right)| left.joined(right))
                        .map(Arc::new)
                } else {
                    name_value.cloned()
                }
            } else {
                outcomes.normal_name_value.clone()
            };
            outcomes.add_with_object_result(
                converted,
                state,
                self.result_for(*route),
                self.object_for(*route),
            );
            outcomes.normal_name_value = joined_name;
            if converted.abrupt_possible()
                && let Some(value) = name_value
            {
                outcomes
                    .abrupt_name_values
                    .push((converted, Arc::clone(value)));
            }
        }
        outcomes.normal_completion = if self.complete_procedure_return.is_some()
            && self.normal.is_none()
            && self.abrupt.len() == 1
            && outcomes.normal.is_some()
            && outcomes.abrupt.is_empty()
        {
            Some(SourceNormalCompletion)
        } else {
            self.normal_completion
                .filter(|_| self.abrupt.is_empty() && outcomes.abrupt.is_empty())
        };
        outcomes.normal_representation = representation.filter(|receipt| {
            outcomes
                .normal
                .as_ref()
                .is_some_and(|normal| receipt.is_current(&normal.source_variables))
        });
        outcomes
    }

    fn capture_with(&self, selection: tcl_registry::catch_invocation::CatchInvocation) -> Self {
        let mut captured = Self::default();
        if let Some(normal) = &self.normal {
            captured.join(&Self::normal(normal));
        }
        for (route, state) in &self.abrupt {
            let selected = selection.route(*route);
            if selected.captured {
                captured.join(&Self::normal(state));
            }
            for route in selected.propagated {
                captured.add(route, state);
            }
        }
        captured
    }

    fn capture_tcl_completions(&self) -> Self {
        self.capture_with(tcl_registry::catch_invocation::CatchInvocation::CAPTURE_TCL_PHASE)
    }

    fn restore_frame(
        &mut self,
        parent: &crate::var_resolve::ResolveContext,
        frame: &crate::var_resolve::VariableExecutionFrame,
    ) {
        for state in self
            .normal
            .iter_mut()
            .chain(self.abrupt.iter_mut().map(|(_, state)| state))
        {
            state.source_variables = Arc::new(crate::var_resolve::restore_execution_frame(
                parent,
                &state.source_variables,
            ));
            state.variable_frame = frame.clone();
        }
    }

    fn restore_source_origin(&mut self, origin: Option<&Arc<SourceOriginId>>) {
        for state in self
            .normal
            .iter_mut()
            .chain(self.abrupt.iter_mut().map(|(_, state)| state))
        {
            state.current_source_origin = origin.cloned();
        }
    }
}

#[cfg(any(test, debug_assertions))]
fn trace_source_deferred(
    started: Option<std::time::Instant>,
    stage: &str,
    bindings: &SourceCommandBindings,
) {
    if let Some(started) = started {
        eprintln!(
            "SOURCE_DEFERRED_PHASE stage={stage} ms={} roots={} outcomes={} layouts={}",
            started.elapsed().as_millis(),
            bindings.deferred.len(),
            bindings.deferred_outcomes.len(),
            bindings.declaration_layouts.len()
        );
    }
}

#[cfg(any(test, debug_assertions))]
fn trace_source_analysis(
    start: Option<std::time::Instant>,
    stage: &str,
    source_len: usize,
    bindings: &SourceCommandBindings,
) {
    if let Some(start) = start {
        eprintln!(
            "SOURCE_PHASE bytes={} stage={} ms={} points={} layouts={}",
            source_len,
            stage,
            start.elapsed().as_millis(),
            bindings.points.len(),
            bindings.declaration_layouts.len()
        );
    }
}

#[cfg(any(test, debug_assertions))]
fn trace_source_lookup(before: Option<(u64, u128)>, stage: &str, source_len: usize) {
    if let Some((before_calls, before_nanos)) = before {
        let (calls, nanos) = original_command_table::baseline_lookup_totals();
        eprintln!(
            "SOURCE_LOOKUP_PHASE bytes={} stage={} calls={} ms={}",
            source_len,
            stage,
            calls.saturating_sub(before_calls),
            nanos.saturating_sub(before_nanos) / 1_000_000
        );
    }
}

impl SourceCommandBindings {
    /// Borrow the original analysis availability ingress, including terminal
    /// supplied-missing ownership. This creates no source or execution entry.
    pub(crate) fn source_metadata_owner(
        &self,
    ) -> &crate::registry_invocation::OwnedInvocationMetadataContext {
        &self.final_state.baseline.metadata_context
    }

    /// Interpret a fresh authoring entry using the selected registry profile,
    /// or the compiler convenience driver's Tcl 9.0 native target when none
    /// is selected. Use [`Self::analyse_with_options`] for another runtime or
    /// interpreter history; assistance metadata alone is not an entry proof.
    #[must_use]
    pub fn analyse(
        source: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) -> Self {
        Self::analyse_in_namespace(source, "::", config, registry)
    }

    /// Interpret an isolated source instance in its actual command namespace.
    #[must_use]
    pub fn analyse_in_namespace(
        source: &str,
        namespace: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) -> Self {
        Self::analyse_in_namespace_with_options(
            source,
            namespace,
            config,
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(crate::environment_ingress::authoring_invocation_dialect(
                    registry, None, config,
                )),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..SourceAnalysisOptions::default()
            },
        )
    }

    /// Interpret source under explicitly supplied entry and loader contracts.
    #[must_use]
    pub fn analyse_with_options(
        source: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        Self::analyse_in_namespace_with_options(source, "::", config, registry, options)
    }

    /// Interpret a source instance with its namespace and explicit entry facts.
    #[must_use]
    pub fn analyse_in_namespace_with_options(
        source: &str,
        namespace: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        let frame = if namespace == "::" {
            crate::var_resolve::VariableExecutionFrame::Global
        } else {
            crate::var_resolve::VariableExecutionFrame::Namespace(namespace.to_owned())
        };
        Self::analyse_in_frame_with_options(source, &frame, config, registry, options)
    }

    /// Interpret source in a proved activation rather than inferring variable
    /// ownership from a namespace spelling.
    #[must_use]
    pub fn analyse_in_frame_with_options(
        source: &str,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        Self::analyse_image_in_frame_with_options(
            &tcl_lexer::SourceImage::document(source),
            frame,
            config,
            registry,
            options,
        )
        .expect("document source is valid UTF-8")
    }

    /// Interpret a checked text view while retaining the original source channel.
    /// Opaque native source has no Unicode analysis projection.
    #[must_use]
    pub fn analyse_image_in_frame_with_options(
        image: &tcl_lexer::SourceImage,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<Self> {
        let source = image.try_text().ok()?;
        Some(Self::analyse_source_in_frame_with_options(
            source, image, frame, config, registry, options,
        ))
    }

    fn analyse_source_in_frame_with_options(
        source: &str,
        image: &tcl_lexer::SourceImage,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        #[cfg(any(test, debug_assertions))]
        let phase_start = std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(std::time::Instant::now);
        #[cfg(any(test, debug_assertions))]
        let lookup_start = phase_start.map(|_| original_command_table::baseline_lookup_totals());
        let config = options.native_lexer_config(config);
        let memo_key = source_analysis_cache::SourceAnalysisCacheKey::at_entry(
            image, frame, config, registry, options,
        );
        if let Some(bindings) = memo_key.as_ref().and_then(source_analysis_cache::lookup) {
            #[cfg(any(test, debug_assertions))]
            trace_source_analysis(phase_start, "memo", source.len(), &bindings);
            return bindings;
        }
        let namespace = source_frame_namespace(frame);
        let mut bindings = Self {
            compilation_scope: options.compilation_scope,
            lexer_config: Some(config),
            ..Self::default()
        };
        let mut state =
            ModuleCommandBindings::initial_with_options(registry, options, Some(config));
        state.current_source_origin = Some(Arc::new(SourceOriginId::authored_image(image.clone())));
        state.ordinary_literal_pool =
            literal_object_pool::SourceOrdinaryLiteralPool::at_entry(&state, options);
        bindings
            .root_origin
            .clone_from(&state.current_source_origin);
        let namespace_key = frame
            .namespace_identity()
            .cloned()
            .or_else(|| {
                options
                    .native_entry
                    .and_then(|entry| SourceNamespaceKey::from_native_entry(entry).ok())
            })
            .unwrap_or_else(|| SourceNamespaceKey::authored(namespace));
        let frame = frame.clone().with_namespace_identity(namespace_key.clone());
        // This API's supplied frame is the entry activation. It is not a
        // nested call whose final state should restore the initial model frame.
        if state.variable_frame != frame {
            state.source_variables = source_variables_in_frame(&state.source_variables, &frame);
            state.variable_frame = frame.clone();
        }
        for name in options.incoming_formals {
            Arc::make_mut(&mut state.source_variables).bind_unknown_incoming(name, registry);
        }
        #[cfg(any(test, debug_assertions))]
        trace_source_analysis(phase_start, "initial", source.len(), &bindings);
        let root_outcomes = bindings.walk_source(
            source,
            0,
            &mut state,
            &root_source_execution_context(
                &frame,
                namespace,
                &namespace_key,
                config,
                registry,
                options,
            ),
        );
        #[cfg(any(test, debug_assertions))]
        trace_source_analysis(phase_start, "walk", source.len(), &bindings);
        #[cfg(any(test, debug_assertions))]
        trace_source_lookup(lookup_start, "walk", source.len());
        if root_outcomes.normal_completion.is_some() && root_outcomes.abrupt.is_empty() {
            bindings.original_completed_root_state = root_outcomes
                .normal
                .as_ref()
                .map(|normal| Arc::new((**normal).clone()));
        }
        if let Some(normal) = root_outcomes.normal {
            state = *normal;
        }
        if options.compilation_scope == tcl_runtime_api::SourceCompilationScope::WholeModule {
            bindings.analyse_deferred_entries(&state, config, registry);
        }
        #[cfg(any(test, debug_assertions))]
        trace_source_analysis(phase_start, "deferred", source.len(), &bindings);
        #[cfg(any(test, debug_assertions))]
        trace_source_lookup(lookup_start, "deferred", source.len());
        bindings.invalidate_unpositioned_projections();
        bindings.final_state = Arc::new(state);
        if let Some(key) = memo_key {
            source_analysis_cache::retain(key, &bindings);
        }
        #[cfg(any(test, debug_assertions))]
        trace_source_analysis(phase_start, "complete", source.len(), &bindings);
        bindings
    }

    fn analyse_deferred_entries(
        &mut self,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) {
        #[cfg(any(test, debug_assertions))]
        let started = std::env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(std::time::Instant::now);
        #[cfg(any(test, debug_assertions))]
        let mut last_report = started;
        #[cfg(any(test, debug_assertions))]
        let mut iteration = 0_usize;
        loop {
            let root_count = self.deferred.len();
            #[cfg(any(test, debug_assertions))]
            if let Some(started) = started {
                iteration += 1;
                eprintln!(
                    "SOURCE_DEFERRED_PROGRESS stage=select iteration={iteration} ms={} roots={root_count}",
                    started.elapsed().as_millis()
                );
            }
            for (_root_index, (root, entry)) in
                self.deferred_entry_states(incoming).into_iter().enumerate()
            {
                #[cfg(any(test, debug_assertions))]
                if let Some(started) = started
                    && (_root_index == 0
                        || last_report.is_some_and(|last| last.elapsed().as_secs() >= 1))
                {
                    eprintln!(
                        "SOURCE_DEFERRED_PROGRESS stage=entry iteration={iteration} index={_root_index} ms={} roots={root_count} bytes={} offset={} outcomes={} layouts={}",
                        started.elapsed().as_millis(),
                        root.source.len(),
                        root.offset,
                        self.deferred_outcomes.len(),
                        self.declaration_layouts.len()
                    );
                    last_report = Some(std::time::Instant::now());
                }
                if self
                    .called_implementations
                    .contains(&root.implementation_id())
                {
                    continue;
                }
                self.walk_deferred_entry(&root, &entry, config, registry);
            }
            if root_count == self.deferred.len() {
                break;
            }
        }
        #[cfg(any(test, debug_assertions))]
        trace_source_deferred(started, "entries", self);
        self.analyse_called_declaration_results(incoming, config, registry);
        #[cfg(any(test, debug_assertions))]
        trace_source_deferred(started, "called", self);
        self.retain_uninstalled_declaration_body_layouts(registry);
        #[cfg(any(test, debug_assertions))]
        trace_source_deferred(started, "layouts", self);
    }

    fn analyse_called_declaration_results(
        &mut self,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) {
        let entries = self
            .deferred_entry_states(incoming)
            .into_iter()
            .filter(|(root, _)| {
                self.called_implementations
                    .contains(&root.implementation_id())
                    && !root.receiver_method
                    && root.event.is_none()
                    && root.future_frame.is_none()
                    && root.statics.is_none()
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return;
        }
        // The separate collector retains the original declaration and lookup
        // environment. seed_deferred_inputs installs unknown formals and
        // widens namespace contents before the shared body walker runs.
        // Its dispatch/read/compiler observations cannot become actual calls.
        let mut conditional = self.clone();
        conditional.conditional_procedure_results = SharedSourceInventory::default();
        conditional.active_calls.clear();
        conditional.active_variable_observers.clear();
        for (root, entry) in entries {
            conditional.walk_deferred_entry(&root, &entry, config, registry);
        }
        self.merge_original_declaration_layouts(&conditional);
        self.merge_conditional_procedure_results(&conditional);
    }

    fn deferred_entry_states(
        &self,
        incoming: &ModuleCommandBindings,
    ) -> Vec<(DeferredSourceBody, Arc<ModuleCommandBindings>)> {
        let entry = Arc::new(incoming.clone());
        let mut entries = self
            .deferred_entry_bodies(incoming)
            .into_iter()
            .map(|root| (root, Arc::clone(&entry)))
            .collect::<Vec<_>>();
        let initial = entries
            .iter()
            .map(|(root, _)| root.implementation_id())
            .collect::<BTreeSet<_>>();
        if initial.len() == self.deferred.len() {
            return entries;
        }
        // A future invocation can install a new callable even when its final
        // completion is abrupt. Its exact outcome supplies that callable's
        // lookup table; the actual document continuation remains untouched.
        for outcome in &self.deferred_outcomes {
            if outcome.route
                == tcl_registry::completion_route::InvocationCompletionRoute::ProcessExit
            {
                continue;
            }
            for root in self.deferred_entry_bodies(&outcome.state) {
                if !initial.contains(&root.implementation_id())
                    && !entries.iter().any(|(known, state)| {
                        known.implementation_id() == root.implementation_id()
                            && known.namespace_key == root.namespace_key
                            && state.same_state(&outcome.state)
                    })
                {
                    entries.push((root, Arc::clone(&outcome.state)));
                }
            }
        }
        entries
    }

    fn deferred_entry_bodies(&self, incoming: &ModuleCommandBindings) -> Vec<DeferredSourceBody> {
        self.deferred
            .values()
            .flat_map(|root| {
                if root.event.is_some() || root.receiver_method || root.future_frame.is_some() {
                    return vec![root.clone()];
                }
                let namespaces = incoming
                    .bindings
                    .iter()
                    .flat_map(|(slot, bindings)| {
                        bindings.iter().map(move |binding| (slot, binding))
                    })
                    .filter_map(|(slot, binding)| match binding {
                        MayBinding::Target(target)
                            if target.kind == BindingKind::Proc
                                && target.implementation_generation
                                    == root.implementation_generation
                                && target.implementation_allocation
                                    == root.implementation_allocation
                                && target
                                    .token
                                    .as_ref()
                                    .is_some_and(|identity| identity.origin == root.identity) =>
                        {
                            Some(slot.holder().into_owned())
                        }
                        _ => None,
                    })
                    .collect::<BTreeSet<_>>();
                namespaces
                    .into_iter()
                    .map(|namespace_key| DeferredSourceBody {
                        namespace: namespace_key.display().unwrap_or_default(),
                        namespace_key,
                        ..root.clone()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    fn walk_deferred_entry(
        &mut self,
        root: &DeferredSourceBody,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) {
        use crate::var_resolve::VariableExecutionFrame;
        if root.future_frame.is_some() {
            self.walk_future_body(root, incoming, config, registry);
            return;
        }
        let mut branch = boxed_source_branch(incoming);
        branch.current_source_origin.clone_from(&root.source_origin);
        let parent = Arc::clone(&branch.source_variables);
        let frame = if root.receiver_method {
            VariableExecutionFrame::ReceiverMethod {
                identity: source_activation_name(
                    root.source_origin.as_ref(),
                    &root.identity,
                    root.implementation_generation,
                ),
            }
        } else if let Some(event) = &root.event {
            let mut entry = crate::connection_scope::event_resolve_context(event);
            entry.invocation_dialect = branch.baseline.dialect;
            entry.namespace_cells = parent.namespace_cells.clone();
            let frame = if entry.dynamic_bindings {
                VariableExecutionFrame::Unknown
            } else if entry.global_frame() {
                VariableExecutionFrame::Global
            } else {
                VariableExecutionFrame::Procedure {
                    namespace: root.namespace.clone(),
                    identity: source_activation_name(
                        root.source_origin.as_ref(),
                        &root.identity,
                        root.implementation_generation,
                    ),
                }
            };
            branch.source_variables = Arc::new(entry.in_frame(&frame));
            branch.variable_frame = frame.clone();
            frame
        } else {
            VariableExecutionFrame::Procedure {
                namespace: root.namespace.clone(),
                identity: source_activation_name(
                    root.source_origin.as_ref(),
                    &root.identity,
                    root.implementation_generation,
                ),
            }
        };
        let frame = if root.receiver_method {
            frame
        } else {
            frame.with_namespace_identity(root.namespace_key.clone())
        };
        seed_deferred_inputs(&mut branch, root, &frame, registry);
        if let Some((parent, argument, script)) = &root.executed_script {
            self.record_executed_script(parent.clone(), *argument, script.clone(), None, None);
            if let Some((origin, span)) = &root.executed_word {
                self.record_executed_word(origin, *span, None, script.clone());
            }
        }
        let compilation = procedure_compilation(branch.baseline.compilation_dialect());
        // This root is already being interpreted in its selected preview
        // frame. A recursive call must reach the ordinary backedge residual,
        // rather than re-enter this body once before noticing the cycle.
        // Preview ownership does not mark a document call as reached.
        let implementation = root.implementation_id();
        let activated = self.active_calls.insert(implementation.clone());
        self.declaration_preview_depth += 1;
        let outcomes = self.walk_source(
            &root.source,
            root.offset,
            &mut branch,
            &SourceExecutionContext {
                realm: root.realm,
                compilation,
                compilation_snapshot: None,
                selected_compilation: None,
                original_variable_compilation: None,
                namespace: &root.namespace,
                config,
                registry,
                depth: 0,
                frame: &frame,
                invocation_offset: 0,
                variable_read_owner: None,
                original_written_projection: None,
                written_arguments: None,
                written_values: None,
                written_name_values: None,
                written_representations: None,
                written_objects: None,
                written_method_prefixes: None,
                written_variable_reads: None,
                namespace_key: (!root.receiver_method).then_some(&root.namespace_key),
                expression_source: None,
            },
        );
        self.declaration_preview_depth -= 1;
        if activated {
            self.active_calls.remove(&implementation);
        }
        self.record_conditional_procedure_result(root, &outcomes, branch.baseline.dialect);
        self.retain_deferred_outcomes(root, &frame, outcomes);
        // Declaration analysis supplies body points; it is not an execution
        // edge and does not alter the document's actual continuation.
    }

    fn retain_deferred_outcomes(
        &mut self,
        root: &DeferredSourceBody,
        frame: &crate::var_resolve::VariableExecutionFrame,
        outcomes: SourceOutcomes,
    ) {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let normal = outcomes.normal.into_iter().map(|state| {
            (
                Route::Tcl(tcl_registry::completion::CompletionCode::Ok),
                state,
            )
        });
        for (route, state) in normal.chain(outcomes.abrupt) {
            let implementation = root.implementation_id();
            if let Some(known) = self.deferred_outcomes.iter_mut().find(|known| {
                known.implementation == implementation
                    && known.namespace_key == root.namespace_key
                    && known.frame == *frame
                    && known.realm == root.realm
                    && known.route == route
            }) {
                Arc::make_mut(&mut known.state).join(&state);
            } else {
                self.deferred_outcomes.push(SourceDeferredOutcome {
                    implementation,
                    namespace_key: root.namespace_key.clone(),
                    frame: frame.clone(),
                    realm: root.realm,
                    route,
                    state: Arc::from(state),
                });
            }
        }
    }

    /// Reached variable reads wholly contained in an original source span.
    /// Records use this analysis instance's source root and absolute offsets;
    /// absent or skipped reads remain absent instead of inheriting dispatch facts.
    #[must_use]
    pub fn variable_accesses_in_span(&self, span: tcl_lexer::Span) -> Vec<SourceVariableAccess> {
        self.variable_accesses
            .range(span.start()..=span.end())
            .flat_map(|(_, accesses)| accesses)
            .filter(|access| access.is_contained_in(span))
            .cloned()
            .collect()
    }

    /// Reads directly owned by this invocation's original word evaluation.
    /// Nested commands and native body phases have their own temporal owners;
    /// the lexical inventory is deliberately broader than this projection.
    #[must_use]
    pub fn variable_accesses_for_invocation_args(&self, offset: u32) -> Vec<SourceVariableAccess> {
        let Some(source) = &self.root_origin else {
            return Vec::new();
        };
        let site = CommandAllocationSite {
            source: Arc::clone(source),
            offset,
        };
        self.variable_accesses
            .values()
            .flatten()
            .filter(|access| access.owner.is_direct_arguments_of(&site))
            .cloned()
            .collect()
    }

    /// Reads executed during original argv evaluation, including entered nested
    /// substitutions. This transitive evaluation projection is distinct from
    /// the directly owned argv projection and never relies on lexical nesting.
    #[must_use]
    pub fn variable_accesses_during_argument_evaluation(
        &self,
        offset: u32,
    ) -> Vec<SourceVariableAccess> {
        let Some(source) = &self.root_origin else {
            return Vec::new();
        };
        let site = CommandAllocationSite {
            source: Arc::clone(source),
            offset,
        };
        self.variable_accesses
            .values()
            .flatten()
            .filter(|access| access.owner.is_argument_evaluation_of(&site))
            .cloned()
            .collect()
    }

    /// Reached reads while evaluating this invocation's original argv or its
    /// selected native expression, including substitutions entered by either
    /// evaluation. Native script bodies have separate temporal ownership.
    #[must_use]
    pub fn variable_accesses_during_expression_invocation(
        &self,
        offset: u32,
    ) -> Vec<SourceVariableAccess> {
        let Some(source) = &self.root_origin else {
            return Vec::new();
        };
        let site = CommandAllocationSite {
            source: Arc::clone(source),
            offset,
        };
        self.variable_accesses
            .values()
            .flatten()
            .filter(|access| {
                access.owner.is_argument_evaluation_of(&site)
                    || access.owner.is_expression_evaluation_of(&site)
            })
            .cloned()
            .collect()
    }

    /// Query one exact original reference; an unrecorded read is unresolved.
    #[must_use]
    pub fn variable_access_at(
        &self,
        source: &crate::ir::SourceSite,
        original_spelling: &str,
    ) -> Option<&SourceVariableAccess> {
        SourceVariableAccess::find_at_source(
            self.variable_accesses.get(&source.span.start())?,
            source,
            original_spelling,
        )
    }

    fn record_variable_access(
        &mut self,
        source: &crate::ir::SourceSite,
        original_spelling: &str,
        state: &ModuleCommandBindings,
        owner: Option<&SourceVariableEvaluationOwner>,
        registry: &CommandRegistry,
        selected_place: &crate::place::Place,
    ) {
        let Some(origin) = &state.current_source_origin else {
            return;
        };
        let context = &state.source_variables;
        let object_instance = state.object_read_proof(selected_place, registry);
        let owner = owner
            .cloned()
            .unwrap_or(SourceVariableEvaluationOwner::Unspecified);
        let reached_read = SourceVariableAccess {
            source: source.clone(),
            owner: owner.clone(),
            original_spelling: original_spelling.to_owned(),
            variable_context: Arc::clone(context),
            context_alternatives: vec![Arc::clone(context)],
            context_fingerprint: SourceVariableAccess::fingerprint(context),
            object_instance: object_instance.clone(),
            selected_places: vec![SelectedSourceVariablePlace {
                context: Arc::clone(context),
                place: Some(Arc::new(selected_place.clone())),
            }],
        };
        self.capture_current_expression_read(origin, &state.variable_frame, &reached_read);
        self.record_invocation_substitution_read(reached_read);
        let inventory = if self.root_origin.as_ref() == Some(origin) {
            &mut self.variable_accesses
        } else {
            self.origin_variable_accesses
                .entry(Arc::clone(origin))
                .or_default()
        };
        let accesses = inventory.entry(source.span.start()).or_default();
        if let Some(access) = accesses.iter_mut().find(|access| {
            access.source == *source && access.original_spelling == original_spelling
        }) {
            access.owner.join(owner);
            access.retain_selected_place(context, selected_place);
            if access.object_instance != object_instance {
                access.object_instance = None;
            }
            if !access.context_alternatives.contains(context) {
                access.context_alternatives.push(Arc::clone(context));
            }
            if access.variable_context != *context {
                Arc::make_mut(&mut access.variable_context).join(context);
                access.context_fingerprint =
                    SourceVariableAccess::fingerprint(&access.variable_context);
            }
        } else {
            accesses.push(SourceVariableAccess {
                source: source.clone(),
                owner,
                original_spelling: original_spelling.to_owned(),
                variable_context: Arc::clone(context),
                context_alternatives: vec![Arc::clone(context)],
                context_fingerprint: SourceVariableAccess::fingerprint(context),
                object_instance,
                selected_places: vec![SelectedSourceVariablePlace {
                    context: Arc::clone(context),
                    place: Some(Arc::new(selected_place.clone())),
                }],
            });
        }
    }

    /// A source query can only prove dispatch for a represented invocation.
    #[must_use]
    pub fn has_site(&self, offset: u32) -> bool {
        self.dispatch_points.contains_key(&offset)
    }

    /// Namespace objects proved to exist immediately before this invocation.
    #[must_use]
    pub fn known_namespaces_at(&self, offset: u32) -> Vec<String> {
        let mut points = self.dispatch_points_at(offset);
        let Some(first) = points.next() else {
            return Vec::new();
        };
        let mut namespaces = first.state.namespaces.as_ref().clone();
        for point in points {
            namespaces.retain(|namespace| point.state.namespaces.contains(namespace));
        }
        namespaces
            .into_iter()
            .filter_map(|key| key.advisory_key())
            .collect()
    }

    /// Exact source-site namespace proof, shared with place resolution.
    #[must_use]
    pub fn namespaces_at_source(&self, offset: u32) -> Vec<String> {
        self.known_namespaces_at(offset)
    }

    /// Command absence before dispatch, independently of `unknown` fallback.
    #[must_use]
    pub fn command_is_absent_at(&self, command: &str, offset: u32) -> bool {
        let mut points = self.dispatch_points_at(offset);
        let Some(first) = points.next() else {
            return false;
        };
        first.state.definitely_absent(command, &first.namespace_key)
            && points.all(|point| point.state.definitely_absent(command, &point.namespace_key))
    }

    /// Whether every selected lifecycle traversal proves a phase unentered.
    /// Missing or unfinished traversal evidence remains explicit uncertainty.
    #[must_use]
    pub fn lifecycle_phase_reachability_at(
        &self,
        site: &CommandAllocationSite,
        phase: SourceBodyPhase,
    ) -> SourceBodyPhaseReachability {
        let Some(coverage) = self.lifecycle_coverage.get(site) else {
            return SourceBodyPhaseReachability::Unknown;
        };
        let coverage = coverage
            .iter()
            .filter(|entry| !entry.declaration_preview)
            .collect::<Vec<_>>();
        if coverage.is_empty() || coverage.iter().any(|entry| entry.entered.is_none()) {
            return SourceBodyPhaseReachability::Unknown;
        }
        let index = match phase {
            SourceBodyPhase::Setup => 0,
            SourceBodyPhase::Body => 1,
            SourceBodyPhase::Cleanup => 2,
        };
        if coverage
            .iter()
            .filter_map(|entry| entry.entered)
            .any(|entered| entered[index])
        {
            SourceBodyPhaseReachability::MayEntered
        } else {
            SourceBodyPhaseReachability::NotEntered
        }
    }

    /// Exact phase-entry binding used for hook and phase dependency proofs.
    #[must_use]
    pub fn invocation_at_phase(
        &self,
        command: &str,
        site: u32,
        phase: SourceBodyPhase,
    ) -> SourceInvocationBinding {
        Self::query_points(
            command,
            self.phases
                .iter()
                .filter(|point| {
                    point.site == site
                        && point.phase == phase
                        && point.point.state.current_source_origin.as_ref()
                            == self.root_origin.as_ref()
                        && !point.point.declaration_preview
                })
                .map(|point| &point.point),
        )
    }

    /// Command absence at a represented lifecycle phase entry.
    #[must_use]
    pub fn command_is_absent_at_phase(
        &self,
        command: &str,
        site: u32,
        phase: SourceBodyPhase,
    ) -> bool {
        let points = self
            .phases
            .iter()
            .filter(|point| {
                point.site == site
                    && point.phase == phase
                    && point.point.state.current_source_origin.as_ref() == self.root_origin.as_ref()
                    && !point.point.declaration_preview
            })
            .collect::<Vec<_>>();
        !points.is_empty()
            && points.iter().all(|point| {
                point
                    .point
                    .state
                    .definitely_absent(command, &point.point.namespace_key)
            })
    }

    /// Whether text-only body lowering can reuse a fresh isolated proof.
    /// Every invocation is compared through the same binding interpretation;
    /// namespace and provider facts are part of the proof, not lexical keys.
    #[must_use]
    pub fn can_reuse_isolated_body(
        &self,
        body: &str,
        base: u32,
        namespace: &str,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) -> bool {
        let loaders = self
            .final_state
            .baseline
            .trusted_loaders
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let options = SourceAnalysisOptions {
            metadata_context: crate::registry_invocation::InvocationMetadataInput::Retained(
                &self.final_state.baseline.metadata_context,
            ),
            compilation_scope: tcl_runtime_api::SourceCompilationScope::WholeModule,
            invocation_realm: self
                .points
                .iter()
                .find(|point| point.dispatch && point.offset >= base)
                .map_or(self.final_state.baseline.invocation_realm, |point| {
                    point.realm
                }),
            incoming_formals: &[],
            native_entry: self.final_state.baseline.native_entry.as_deref(),
            native_compilation: self
                .points
                .iter()
                .find(|point| point.dispatch && point.offset >= base)
                .map_or(self.final_state.baseline.native_compilation, |point| {
                    point.compilation
                }),
            declared_commands: None,
            trusted_package_loaders: &loaders,
            trusted_source_modules: &self.final_state.baseline.trusted_source_modules,
            unknown_entry: false,
            invocation_dialect: self.final_state.baseline.dialect,
            hosted_execution_context: self.final_state.baseline.hosted_execution_context,
            execution_name_policy: self.final_state.baseline.execution_name_policy,
            logical_source_input: self.final_state.baseline.logical_source_input.as_ref(),
            vendor_source_input: self.final_state.baseline.vendor_source_input.as_ref(),
            compiled_variable_provider: self.final_state.baseline.compiled_variable_provider,
        };
        let fallback = crate::var_resolve::VariableExecutionFrame::Namespace(namespace.to_owned());
        let frame = self
            .points
            .iter()
            .find(|point| {
                point.dispatch
                    && point.offset >= base
                    && point.offset
                        < base.saturating_add(u32::try_from(body.len()).unwrap_or(u32::MAX))
            })
            .map_or(&fallback, |point| &point.state.variable_frame);
        let isolated = Self::analyse_in_frame_with_options(body, frame, config, registry, options);
        // A cached isolated Script does not replay document declaration,
        // import/export or deferred-unit publication into the enclosing
        // lowerer. Admit cache reuse only when the shared transfer proves no
        // such state changes, before comparing every actual dispatch point.
        if isolated.has_transitions() || !isolated.deferred.is_empty() {
            return false;
        }
        isolated
            .points
            .iter()
            .filter(|point| point.dispatch)
            .all(|point| {
                let Some(original_offset) = base.checked_add(point.offset) else {
                    return false;
                };
                let Some(head) = point.head.as_deref() else {
                    return false;
                };
                if isolated
                    .invocation_at_source(head, point.offset)
                    .targets
                    .iter()
                    .any(|target| {
                        registry
                            .get(&target.command)
                            .is_some_and(|spec| spec.body_execution.is_some())
                    })
                {
                    return false;
                }
                if !isolated
                    .invocation_at_source(head, point.offset)
                    .same_dispatch_projection(&self.invocation_at_source(head, original_offset))
                {
                    return false;
                }
                self.same_isolated_body_dependencies(
                    &isolated,
                    head,
                    point.offset,
                    original_offset,
                    registry,
                )
            })
    }

    fn same_isolated_body_dependencies(
        &self,
        isolated: &Self,
        head: &str,
        offset: u32,
        original_offset: u32,
        registry: &CommandRegistry,
    ) -> bool {
        let contracts = isolated.invocation_at_source(head, offset).targets;
        contracts.iter().all(|target| {
            registry
                .get(&target.command)
                .and_then(|spec| spec.body_execution)
                .is_none_or(|contract| {
                    let tcl_registry::body_execution::BodyExecutionSpec::CapturedLifecycle(
                        contract,
                    ) = contract
                    else {
                        return true;
                    };
                    isolated.loaded_implementation_at(contract.provider.package, offset)
                        == self.loaded_implementation_at(contract.provider.package, original_offset)
                })
        })
    }

    /// Restore every rebased cache carrier from its original source instance.
    /// Phase-region dependencies require a separate relocation contract and
    /// are deliberately declined here instead of retaining stale provenance.
    pub fn restore_script_bindings(&self, script: &mut Script) -> bool {
        let mut valid = true;
        crate::ir::for_each_script_mut(script, &mut |script| {
            let Some(source) = self.restored_script_source(script) else {
                valid = false;
                return;
            };
            let Some(namespace) = script
                .namespace_context
                .as_deref()
                .cloned()
                .or_else(|| self.executed_script_namespace_context(&source))
            else {
                valid = false;
                return;
            };
            let Some(bindings) = self.selected_source_in_context(&source, &namespace) else {
                valid = false;
                return;
            };
            script.namespace_context = Some(Box::new(namespace));
            {
                if let Some(admission) = &mut script.native_compilation_admission {
                    let admission = Arc::make_mut(admission);
                    valid &= !admission.requires_native_provider();
                    admission.source = Some(source.clone());
                }
                script.implicit_math_invocations =
                    bindings.implicit_math_invocations_for_script(&source);
                script.expression_preparations =
                    bindings.expression_preparations_for_script(&source);
                script.executed_source = Some(Arc::new(source));
            }
            for site in script.command_binding_sites.iter_mut() {
                let proof = bindings.invocation_at_source(&site.binding.name, site.span.start());
                if !proof.proved_execution_target().is_some_and(|target| {
                    target.registry_backed && nqn(&target.command) == nqn(&site.binding.identity)
                }) {
                    valid = false;
                }
                site.known_namespaces = Some(proof.known_namespaces);
                site.variable_frame = Some(proof.variable_frame);
                site.variable_context = Some(proof.variable_context);
                site.existing_namespace_cells = Some(proof.existing_namespace_cells);
                if let Some(tokens) = &mut site.source_tokens {
                    tokens.source_binding =
                        Some(bindings.invocation_at_source("", site.span.start()));
                    tokens.variable_accesses =
                        bindings.variable_accesses_during_expression_invocation(site.span.start());
                    for (offset, proof) in &mut tokens.nested_bindings {
                        *proof = bindings.invocation_at_source("", *offset);
                    }
                }
            }
            for statement in &mut script.statements {
                let span = statement.span();
                let offset = span.start();
                let Some(tokens) = statement.tokens_mut() else {
                    continue;
                };
                if tokens.evaluated_body.is_some() {
                    valid = false;
                }
                tokens.source_binding = Some(bindings.invocation_at_source("", offset));
                tokens.variable_accesses =
                    bindings.variable_accesses_during_expression_invocation(offset);
                for (offset, proof) in &mut tokens.nested_bindings {
                    *proof = bindings.invocation_at_source("", *offset);
                }
            }
        });
        valid
    }

    /// Attach exact retained source proofs to original command words. Missing
    /// reached sites stay explicitly unknown; this never constructs entry facts.
    pub fn stamp_original_tokens(&self, tokens: &mut crate::ir::CommandTokens) {
        self.stamp_original_tokens_with_bindings(tokens, |offset| {
            self.invocation_at_source("", offset)
        });
    }

    /// Retain the same original offset inventory and attachments while an
    /// immutable owner supplies its complete derived point projection.
    /// Mutable builders use the ordinary query and retain no projection memo.
    pub(crate) fn stamp_original_tokens_with_bindings(
        &self,
        tokens: &mut crate::ir::CommandTokens,
        mut binding_at: impl FnMut(u32) -> SourceInvocationBinding,
    ) {
        // Implementation contract: naming.source.original-invocation-projection-memo
        // docs/design/analysis/name-resolution-proofs/original-invocation-projection-memo.md
        let Some(head) = tokens.words().first() else {
            return;
        };
        let offset = head.source().span.start();
        tokens.variable_accesses = self.variable_accesses_during_expression_invocation(offset);
        tokens.nested_bindings.clear();
        let config = self.lexer_config.unwrap_or_default();
        for lifted in crate::word_subst::lifted_calls(Some(tokens), config) {
            if let Some(nested) = lifted.tokens
                && let Some(head) = nested.words().first()
            {
                let offset = head.source().span.start();
                tokens.nested_bindings.push((offset, binding_at(offset)));
            }
        }
        let mut retained = tokens
            .nested_bindings
            .iter()
            .map(|(offset, _)| *offset)
            .collect::<BTreeSet<_>>();
        let nested_offsets = tokens
            .words()
            .iter()
            .flat_map(|word| {
                let span = &word.source().span;
                self.dispatch_points
                    .range(span.start()..=span.end())
                    .map(|(offset, _)| *offset)
            })
            .filter(|nested| *nested != offset)
            .collect::<BTreeSet<_>>();
        let compiled_offsets = self
            .compiler_invocations
            .keys()
            .filter(|site| {
                self.root_origin.as_ref() == Some(&site.source)
                    && site.offset != offset
                    && tokens.words().iter().any(|word| {
                        word.source().span.start() <= site.offset
                            && site.offset <= word.source().span.end()
                    })
            })
            .map(|site| site.offset)
            .collect::<Vec<_>>();
        for nested in nested_offsets.into_iter().chain(compiled_offsets) {
            if retained.insert(nested) {
                tokens.nested_bindings.push((nested, binding_at(nested)));
            }
        }
        tokens.source_binding = Some(binding_at(offset));
    }

    /// Loader provenance live at an exact invocation site, independently of
    /// its current command-table binding.
    #[must_use]
    pub fn loaded_implementation_at(
        &self,
        package: &str,
        offset: u32,
    ) -> Option<&TrustedPackageLoader> {
        let mut points = self.dispatch_points_at(offset);
        let first = points.next()?;
        let loader = first.state.loaded_provider(package)?;
        points
            .all(|point| point.state.loaded_provider(package) == Some(loader))
            .then_some(loader)
    }

    /// Every possible provider command is original or absent on its audited
    /// version path; document bindings and unknown targets withdraw proof.
    #[must_use]
    pub fn provider_surface_untampered(
        &self,
        package: &str,
        implementation_id: &str,
        offset: u32,
    ) -> bool {
        let mut points = self.dispatch_points_at(offset).peekable();
        points.peek().is_some()
            && points.all(|point| {
                point.state.loaded_provider(package).is_some_and(|loader| {
                    loader.implementation_id == implementation_id
                        && point.state.provider_surface_is_live(loader)
                })
            })
    }

    /// Provider closure proof at the exact lifecycle phase entry.
    #[must_use]
    pub fn provider_surface_untampered_at_phase(
        &self,
        package: &str,
        implementation_id: &str,
        site: u32,
        phase: SourceBodyPhase,
    ) -> bool {
        let points = self
            .phases
            .iter()
            .filter(|point| {
                point.site == site
                    && point.phase == phase
                    && point.point.state.current_source_origin.as_ref() == self.root_origin.as_ref()
                    && !point.point.declaration_preview
            })
            .collect::<Vec<_>>();
        !points.is_empty()
            && points.iter().all(|point| {
                point
                    .point
                    .state
                    .loaded_provider(package)
                    .is_some_and(|loader| {
                        loader.implementation_id == implementation_id
                            && point.point.state.provider_surface_is_live(loader)
                    })
            })
    }

    /// Query the binding immediately before dispatch at a source site.
    #[must_use]
    pub fn invocation_at(
        &self,
        _head: &str,
        namespace: &str,
        offset: u32,
    ) -> SourceInvocationBinding {
        self.attach_invocation_reads(
            Self::query_dispatch_points(
                self.dispatch_points_at(offset)
                    .filter(|point| point.namespace == namespace),
            ),
            self.root_origin.as_ref(),
            offset,
        )
    }

    /// Query with the execution namespace recorded for the source region.
    #[must_use]
    pub fn invocation_at_source(&self, _head: &str, offset: u32) -> SourceInvocationBinding {
        self.attach_invocation_reads(
            Self::query_dispatch_points(self.dispatch_points_at(offset)),
            self.root_origin.as_ref(),
            offset,
        )
    }

    fn dispatch_points_at(&self, offset: u32) -> impl Iterator<Item = &SourceBindingPoint> {
        self.dispatch_points
            .get(&offset)
            .into_iter()
            .flatten()
            .map(|index| &self.points[*index])
    }

    /// Interpret a materialised script using the actual before-call state and
    /// selected activation. Missing dispatch points cannot create entry proof.
    #[must_use]
    pub fn analyse_script_at_site(
        &self,
        script: &str,
        source_base: u32,
        callsite: u32,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        let mut points = self.dispatch_points_at(callsite);
        let first = points.next()?;
        let realm = first.realm;
        let mut state = first.state.clone();
        for point in points {
            if point.realm != realm {
                return None;
            }
            state.join(&point.state);
        }
        let selected = state.source_variables.in_frame(frame);
        let config = selected
            .invocation_dialect
            .map_or(config, |native| config.with_grammar(native.lexer_grammar));
        let namespace = selected.namespace.clone();
        let mut bindings = Self {
            lexer_config: Some(config),
            root_origin: state.current_source_origin.clone(),
            deferred: self.deferred.clone(),
            ..Self::default()
        };
        let compilation = state.baseline.native_compilation;
        let root_outcomes = bindings.walk_source(
            script,
            source_base,
            &mut state,
            &SourceExecutionContext {
                realm,
                compilation,
                compilation_snapshot: None,
                selected_compilation: None,
                original_variable_compilation: None,
                namespace: &namespace,
                frame,
                config,
                registry,
                depth: 0,
                invocation_offset: 0,
                variable_read_owner: None,
                original_written_projection: None,
                written_arguments: None,
                written_values: None,
                written_name_values: None,
                written_representations: None,
                written_objects: None,
                written_method_prefixes: None,
                written_variable_reads: None,
                namespace_key: selected.namespace_identity.as_ref(),
                expression_source: None,
            },
        );
        if root_outcomes.normal_completion.is_some() && root_outcomes.abrupt.is_empty() {
            bindings.original_completed_root_state = root_outcomes
                .normal
                .as_ref()
                .map(|normal| Arc::new((**normal).clone()));
        }
        if let Some(normal) = root_outcomes.normal {
            state = *normal;
        }
        bindings.invalidate_unpositioned_projections();
        bindings.final_state = Arc::new(state);
        Some(bindings)
    }

    fn query_points<'a>(
        head: &str,
        mut points: impl Iterator<Item = &'a SourceBindingPoint>,
    ) -> SourceInvocationBinding {
        let Some(first) = points.next() else {
            return SourceInvocationBinding {
                unknown: true,
                ..SourceInvocationBinding::default()
            };
        };
        let mut result = first.binding(head);
        for point in points {
            result.join(&point.binding(head));
        }
        result
    }

    fn query_dispatch_points<'a>(
        points: impl Iterator<Item = &'a SourceBindingPoint>,
    ) -> SourceInvocationBinding {
        let points = points.collect::<Vec<_>>();
        let actual = points.iter().any(|point| !point.declaration_preview);
        let mut points = points
            .into_iter()
            .filter(|point| !actual || !point.declaration_preview);
        let Some(first) = points.next() else {
            return SourceInvocationBinding {
                unknown: true,
                ..SourceInvocationBinding::default()
            };
        };
        let binding = |point: &SourceBindingPoint| {
            let mut binding = point.original_dispatch_binding();
            binding
                .frozen_head_object
                .clone_from(&point.frozen_head_object);
            if point.frozen_head_object.is_some() {
                binding.entered_execution_observer = point.observer_execution;
                binding
                    .frozen_written_words
                    .clone_from(&point.frozen_written_words);
                binding
                    .frozen_written_names
                    .clone_from(&point.frozen_written_names);
            }
            binding
                .evaluated_argument_values
                .clone_from(&point.evaluated_argument_values);
            binding
                .evaluated_argument_words
                .clone_from(&point.evaluated_argument_words);
            if point.original_head_input.is_none() {
                binding.unknown = true;
                binding.targets.clear();
                binding.declared_command = None;
                binding.catalogue_command = None;
            }
            binding
        };
        let mut result = binding(first);
        for point in points {
            result.join(&binding(point));
        }
        result
    }

    /// Compatibility source-position projection for assistance consumers.
    /// Semantic lowering must query an exact dispatch site instead.
    #[must_use]
    pub fn projection_at_source(&self, head: &str, offset: u32) -> SourceInvocationBinding {
        let point = self
            .points
            .iter()
            .filter(|point| point.offset <= offset && offset <= point.end)
            .max_by_key(|point| point.offset);
        point.map_or_else(
            || {
                let mut proof = source_binding(&self.final_state, head, "::");
                proof.compiled_execution_residual = CompiledExecutionResidual::Unknown;
                proof
            },
            |point| point.lookup_binding(head),
        )
    }

    /// Offset-free consumers must agree across every represented state.
    #[must_use]
    pub fn invocation_unpositioned(&self, head: &str) -> SourceInvocationBinding {
        if let Some(projection) = self
            .unpositioned_projections
            .0
            .lock()
            .expect("source projection cache")
            .get(head)
            .cloned()
        {
            return projection;
        }
        let mut result = source_binding(&self.final_state, head, "::");
        result.compiled_execution_residual = CompiledExecutionResidual::Unknown;
        for point in &self.points {
            result.join(&point.lookup_binding(head));
        }
        self.unpositioned_projections
            .0
            .lock()
            .expect("source projection cache")
            .insert(head.to_owned(), result.clone());
        result
    }

    fn invalidate_unpositioned_projections(&mut self) {
        self.unpositioned_projections
            .0
            .get_mut()
            .expect("source projection cache")
            .clear();
    }

    /// Whether the document changes the registry entry table.
    #[must_use]
    pub fn has_transitions(&self) -> bool {
        !self.final_state.bindings.is_empty()
            || self.final_state.opaque_domain
            || self.final_state.namespace_resolution.changed
    }

    fn walk_source(
        &mut self,
        source: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let channel = state
            .current_source_origin
            .as_ref()
            .map_or(tcl_lexer::SourceChannel::Document, |origin| {
                origin.source_image().channel()
            });
        let image = tcl_lexer::SourceImage::from_bytes(source.as_bytes(), channel);
        self.walk_source_image(&image, base, state, context)
    }

    fn walk_source_image(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Ok(source) = image.try_text() else {
            self.record_unrepresented_source_entry(state, context);
            return opaque_source_invocation(state);
        };
        let entry_context = source_chunk_context_boxed(context, None);
        let context = entry_context.as_ref();
        let config = context.config;
        let depth = context.depth;
        let mut prepared = Box::<PreparedSourceChunk>::default();
        self.prepare_source_chunk(image, base, state, context, &mut prepared);
        if let Some(failed) = prepared.failed_outcome.take() {
            self.record_incomplete_body_header(image, base, state, context);
            self.record_unrepresented_source_entry(state, context);
            self.record_runtime_coverage(
                state.current_source_origin.as_ref(),
                base,
                source.len(),
                &failed,
            );
            return *failed;
        }
        let prepared_context = source_chunk_context_boxed(context, prepared.snapshot.as_ref());
        let context = prepared_context.as_ref();
        if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(depth) {
            self.record_unrepresented_source_entry(state, context);
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            );
        }
        let Some(segments) =
            crate::segmenter::segment_commands_image_with_offset_and_config(image, base, config)
        else {
            self.record_unrepresented_source_entry(state, context);
            return opaque_source_invocation(state);
        };
        let namespace_known = context.namespace_key.is_some()
            || context.frame.namespace_identity().is_some()
            || !matches!(
                context.frame.layout(),
                crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                    | crate::var_resolve::VariableExecutionFrame::Unknown
                    | crate::var_resolve::VariableExecutionFrame::Selected {
                        namespace: None,
                        ..
                    }
            );
        let selected_frame = if namespace_known {
            context
                .frame
                .clone()
                .with_namespace_identity(context.namespace_identity())
        } else {
            context.frame.clone()
        };
        let context = &SourceExecutionContext {
            frame: &selected_frame,
            ..*context
        };
        let previous_frame = state.variable_frame.clone();
        let parent =
            (previous_frame != *context.frame).then(|| Arc::clone(&state.source_variables));
        if parent.is_some() {
            state.source_variables =
                source_variables_in_frame(&state.source_variables, context.frame);
            state.variable_frame = context.frame.clone();
        }
        if namespace_known {
            retain_source_namespace_world(state, context);
        }
        self.retain_original_script_activation(image, base, state, context);
        let mut outcomes = self.walk_source_segments(
            image,
            base,
            &segments,
            state,
            context,
            prepared.entry_error.as_deref(),
        );
        if let Some(parent) = parent {
            outcomes.restore_frame(&parent, &previous_frame);
        }
        outcomes.publish(state);
        self.record_runtime_coverage(
            state.current_source_origin.as_ref(),
            base,
            source.len(),
            &outcomes,
        );
        outcomes
    }

    fn walk_source_segments(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        segments: &[crate::segmenter::SegmentedCommand],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        entry_error: Option<&ModuleCommandBindings>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::normal(state);
        outcomes.retain_complete_normal_evaluation();
        if let Some(abrupt) = entry_error {
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
                abrupt,
            );
        }
        for (index, segment) in segments.iter().enumerate() {
            let Some(mut continuing) = outcomes.normal.take() else {
                if !outcomes.abrupt.is_empty() {
                    let mut declaration_state = boxed_source_branch(state);
                    outcomes.publish(&mut declaration_state);
                    self.record_unentered_declaration_suffix(
                        image,
                        base,
                        &segments[index..],
                        &declaration_state,
                        context,
                    );
                }
                break;
            };
            let prefix_complete = outcomes.normal_completion;
            let command = self.walk_source_command(image, base, &mut continuing, segment, context);
            #[cfg(debug_assertions)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
                eprintln!(
                    "ORIGINAL_COMMAND_COMPLETION site={} prefix_complete={} command_complete={} normal={} abrupt={}",
                    segment.span.start(),
                    prefix_complete.is_some(),
                    command.normal_completion.is_some(),
                    command.normal.is_some(),
                    command.abrupt.len(),
                );
            }
            outcomes.join(&command);
            outcomes.normal_completion = outcomes
                .normal_completion
                .filter(|_| prefix_complete.is_some());
            outcomes.complete_procedure_return = outcomes
                .complete_procedure_return
                .filter(|_| prefix_complete.is_some());
            let Some(continuing) = &outcomes.normal else {
                continue;
            };
            self.record_continuation_source_point(segment, continuing, context);
        }
        outcomes
    }

    /// Resolve the chunk entry outside recursive script frames. Failure
    /// carriers and compilation snapshots remain on the heap during descent.
    #[inline(never)]
    fn prepare_source_chunk(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        prepared: &mut PreparedSourceChunk,
    ) {
        let context = *context;
        prepared.snapshot = (context.compilation_snapshot.is_none()
            && context.compilation.mode
                != tcl_registry::native_compilation::NativeCompilationMode::Direct)
            .then(|| state.native_compilation_snapshot_in_realm(context.realm));
        let Some(snapshot) = &mut prepared.snapshot else {
            return;
        };
        let assessment = compiled_preflight::preflight_image(image, base, state, context, snapshot);
        // A declaration template interprets possible compilation locally. It
        // never publishes an actual compiler visit or chunk-entry obligation.
        if self.declaration_preview_depth == 0 {
            self.record_compilation_source(image, base, state);
            self.retain_compiler_invocations(&assessment.compiler_invocations);
            for (site, children) in &assessment.compiled_children {
                let retained = self.compiled_children.entry(site.clone()).or_default();
                for child in children {
                    if !retained.contains(child) {
                        retained.push(child.clone());
                    }
                }
            }
            self.record_compilation_boundary(base, state, assessment.failure.as_ref());
            if (assessment.possible_error || assessment.native_entry_unavailable)
                && let Some(origin) = &state.current_source_origin
            {
                self.compilation_provider_required
                    .insert(CommandAllocationSite {
                        source: Arc::clone(origin),
                        offset: base,
                    });
            }
        }
        if let Some(failure) = assessment.failure {
            if self.declaration_preview_depth == 0 && !self.compilation_failures.contains(&failure)
            {
                self.compilation_failures.push(failure);
            }
            compiled_preflight::materialise_error_storage(state, context.registry);
            prepared.failed_outcome = Some(Box::new(SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
            )));
        } else if assessment.possible_error && self.declaration_preview_depth == 0 {
            // An unentered declaration preview supplies conditional source
            // advice. Missing compiler evidence is an execution obligation,
            // not a guest error state owned by that declaration.
            let mut abrupt = boxed_source_branch(state);
            compiled_preflight::materialise_error_storage(&mut abrupt, context.registry);
            prepared.entry_error = Some(abrupt);
        }
    }

    /// Materialise a continuation snapshot outside the recursive script frame.
    #[inline(never)]
    fn record_continuation_source_point(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        let context = *context;
        if self.root_origin != state.current_source_origin {
            return;
        }
        self.invalidate_unpositioned_projections();
        self.points.push(SourceBindingPoint {
            declaration_preview: self.declaration_preview_depth != 0,
            original_arguments_complete_normally: false,
            observer_execution: SourceObserverExecution::Unobserved,
            realm: context.realm,
            compilation: context.compilation,
            compiled_execution: compiled_invocation::CompiledInvocationSelection::default(),
            head: None,
            original_head_input: None,
            original_written_words: None,
            evaluated_argument_values: Vec::new(),
            evaluated_argument_words: Vec::new(),
            frozen_written_words: None,
            frozen_written_names: None,
            frozen_head_object: None,
            method_prefix_arguments: Vec::new(),
            offset: segment.span.end().saturating_add(1),
            end: u32::MAX,
            namespace: context.namespace.to_owned(),
            namespace_key: context.namespace_identity(),
            state: state.clone(),
            lookup_snapshot: OnceLock::new(),
            dispatch: false,
        });
    }

    fn walk_source_command(
        &mut self,
        image: &tcl_lexer::SourceImage,
        base: u32,
        state: &mut ModuleCommandBindings,
        segment: &crate::segmenter::SegmentedCommand,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let SourceExecutionContext { config, .. } = *context;
        let Ok(source) = image.try_text() else {
            return opaque_source_invocation(state);
        };
        let words = source_command_tokens_boxed(image, base, config, segment);
        self.record_declaration_operand_layout(state, context, segment.span.start(), &words);
        let compiled = compiled_invocation::select_invocation_boxed(
            words.words(),
            state,
            context,
            segment.span.start(),
        );
        #[cfg(test)]
        trace_original_selection(
            words.words(),
            segment.span.start(),
            state,
            context,
            &compiled,
        );
        let original_variable_compilation =
            original_name_value::OriginalSourceVariableCompilation::from_selected(
                &compiled, state, *context,
            );
        let original_context = Box::new(SourceExecutionContext {
            original_variable_compilation: original_variable_compilation.as_ref(),
            ..*context
        });
        let context = original_context.as_ref();
        if let Some(outcomes) = self.walk_original_list_assignment(
            SourceCommandInput {
                image,
                segment,
                words: words.words(),
                effective: &[],
            },
            base,
            state,
            &compiled,
            context,
        ) {
            return outcomes;
        }
        // Unknown opcode admission alone does not establish name effects.
        // Unclosed compiler effects can precede the original argv, so their
        // possible world cannot borrow argv-first absence.
        if compiled.unknown && !compiled.compile_error && !compiled.compiler_preserves_names() {
            state.mark_opaque_binding_mutation();
        }
        let argument_entry =
            pre_handler_failure::ArgumentEntry::capture(state, context, segment.span.start());
        let arguments =
            self.prepare_source_arguments(source, base, state, segment, words.words(), context);
        if !arguments.ready {
            self.record_pre_handler_failure(argument_entry, words.words(), &arguments.outcomes);
            return arguments.into_outcomes();
        }
        if arguments.effective.is_empty() {
            return empty_source_command_outcomes(state, &arguments.outcomes);
        }
        note_source_command_lookup(state, &compiled, &arguments.effective);
        self.walk_evaluated_source_command(
            SourceCommandInput {
                image,
                segment,
                words: words.words(),
                effective: &[],
            },
            state,
            &compiled,
            context,
            arguments,
        )
    }

    #[inline(never)]
    fn walk_evaluated_source_command(
        &mut self,
        input: SourceCommandInput<'_>,
        state: &mut ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
        mut arguments: Box<PreparedSourceArguments>,
    ) -> SourceOutcomes {
        let SourceCommandInput {
            image,
            segment,
            words,
            ..
        } = input;
        let argument_context = source_argument_context_boxed(
            context,
            SourceArgumentReceipts {
                arguments: &arguments.written_arguments,
                values: &arguments.written_values,
                name_values: &arguments.written_name_values,
                representations: &arguments.written_representations,
                objects: &arguments.written_objects,
                prefixes: &arguments.written_method_prefixes,
                reads: &arguments.written_variable_reads,
            },
        );
        let context = Box::new(argument_context.context());
        #[cfg(test)]
        trace_original_arguments(words, segment.span.start(), state, &context, &arguments);

        let execution_owner = self.begin_invocation_reads(
            state.current_source_origin.as_ref(),
            segment.span.start(),
            context.variable_read_owner,
        );
        let mut execution_context = source_execution_context_boxed(
            &context,
            execution_owner.as_ref(),
            segment.span.start(),
        );
        let mut observer_scope =
            self.prepare_command_observers(&arguments.effective, state, &execution_context);
        arguments.outcomes.join(&observer_scope.entry);
        let Some(continuing) = observer_scope.entry.normal.take() else {
            self.finish_invocation_reads(execution_owner.as_ref(), &arguments.outcomes);
            arguments.outcomes.publish(state);
            return arguments.into_outcomes();
        };
        // Only abrupt callback paths join the eventual handler completion.
        arguments.outcomes.discard_normal_handler_entry();
        publish_source_branch(state, continuing);
        let input = SourceCommandInput {
            image,
            segment,
            words,
            effective: &arguments.effective,
        };
        let normal_result_basis = self.record_evaluated_source_point(
            input,
            state,
            compiled,
            SourceObserverExecution::from_observed(observer_scope.was_observed),
            arguments.complete_normally && !observer_scope.was_observed,
            &context,
        );
        execution_context.withdraw_observed_operand_facts(observer_scope.was_observed);
        let dispatched =
            self.walk_selected_source_command(input, state, compiled, &execution_context);
        let dispatched =
            self.finish_command_observer_scope(dispatched, &observer_scope, &execution_context);
        self.complete_normal_variable_continuation(
            state.current_source_origin.as_ref(),
            segment.span.start(),
            &dispatched,
            observer_scope.was_observed,
        );
        arguments.outcomes.join(&dispatched);
        arguments.finish_normal_completions(observer_scope.was_observed);
        self.finish_invocation_reads(execution_owner.as_ref(), &arguments.outcomes);
        self.record_invocation_normal_result(normal_result_basis, &arguments.outcomes);
        arguments.outcomes.publish(state);
        arguments.into_outcomes()
    }

    /// Freeze argv in its own frame before retaining recursive body dispatch.
    #[inline(never)]
    fn prepare_source_arguments(
        &mut self,
        source: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        context: &SourceExecutionContext<'_>,
    ) -> Box<PreparedSourceArguments> {
        self.prepare_source_argument_range(
            source_arguments::SourceArgumentSlice {
                source,
                base,
                words,
                selected: 0..words.len(),
            },
            state,
            segment,
            context,
        )
    }

    /// Capture large point projections before entering any recursive body.
    #[inline(never)]
    fn record_evaluated_source_point(
        &mut self,
        input: SourceCommandInput<'_>,
        state: &ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        observer_execution: SourceObserverExecution,
        arguments_complete_normally: bool,
        context: &SourceExecutionContext<'_>,
    ) -> Box<normal_result::InvocationResultBasis> {
        let SourceCommandInput {
            segment,
            words,
            effective,
            ..
        } = input;
        let head_word = words.first();
        let context = *context;
        let head = effective
            .first()
            .and_then(|word| word.as_registry_word().literal());
        let point = SourceBindingPoint {
            declaration_preview: self.declaration_preview_depth != 0,
            original_arguments_complete_normally: arguments_complete_normally,
            observer_execution,
            realm: context.realm,
            compilation: context.compilation,
            compiled_execution: compiled.clone(),
            head: head.map(str::to_owned),
            original_head_input: original_name_value::original_command_head_input(
                words,
                segment.span.start(),
                state,
                context,
            ),
            original_written_words: original_name_value::OriginalWrittenSourceWords::at_dispatch(
                words,
                segment.span.start(),
                state,
                context.config,
            )
            .map(Arc::new),
            evaluated_argument_values: context
                .written_arguments
                .unwrap_or(effective)
                .iter()
                .skip(1)
                .map(|word| word.as_registry_word().literal().map(str::to_owned))
                .collect(),
            evaluated_argument_words: context
                .written_arguments
                .unwrap_or(effective)
                .iter()
                .skip(1)
                .cloned()
                .collect(),
            frozen_written_words: context.written_arguments.map(Arc::from),
            frozen_written_names: context.written_name_values.map(Arc::from),
            frozen_head_object: head_word.and_then(|word| {
                context
                    .written_objects
                    .and_then(|words| words.first())
                    .and_then(Option::as_ref)
                    .map(|object| {
                        Arc::new(receiver_self::FrozenSourceObjectHead {
                            word: word.clone(),
                            object: Arc::clone(object),
                        })
                    })
            }),
            method_prefix_arguments: context
                .written_method_prefixes
                .into_iter()
                .flatten()
                .enumerate()
                .skip(1)
                .filter_map(|(written, prefix)| {
                    prefix
                        .as_ref()
                        .filter(|prefix| prefix.is_current(state))
                        .map(|prefix| (written - 1, Arc::clone(prefix)))
                })
                .collect(),
            offset: segment.span.start(),
            end: segment.span.end(),
            namespace: context.namespace.to_owned(),
            namespace_key: context.namespace_identity(),
            state: state.clone(),
            lookup_snapshot: OnceLock::new(),
            dispatch: true,
        };
        let basis = Box::new(normal_result::InvocationResultBasis::of(&point, words));
        self.record_point(point);
        basis
    }

    fn walk_selected_source_command(
        &mut self,
        input: SourceCommandInput<'_>,
        state: &mut ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        let SourceCommandInput {
            image,
            segment,
            words,
            effective,
        } = input;
        let mut outcomes = SourceOutcomes::default();
        self.retain_rule_loader_events(segment, words, effective, state, *context);
        if let Some(outcomes) =
            logical_definition::transfer_selected(words, segment.span.start(), state, *context)
        {
            return outcomes;
        }
        if !compiled.compile_error
            && let Some(result) =
                self.walk_selected_receiver_command(segment, words, effective, state, context)
        {
            return *result;
        }
        let incoming = boxed_source_branch(state);
        if compiled.live {
            self.join_live_source_command(input, &incoming, context, &mut outcomes);
        }
        for proof in &compiled.proofs {
            if proof.operation
                == tcl_registry::SemanticOperationId::Intrinsic(
                    tcl_registry::IntrinsicId::ProcedureNoOp,
                )
            {
                join_source_procedure_noop(&mut outcomes, &incoming);
                continue;
            }
            let mut native = incoming.clone();
            let selection = NativeCompilationSelection::Inline {
                operation: proof.operation,
                guard: proof.guard,
            };
            let selected_context = source_selected_context_boxed(context, &selection);
            let mut result = self.walk_source_target(
                segment,
                words,
                effective,
                &mut native,
                &proof.target,
                &selected_context,
            );
            retain_source_native_result(&mut result, &incoming, &proof.target, effective, context);
            outcomes.join(&result);
        }
        self.join_other_selected_source_commands(
            SourceCommandInput {
                image,
                segment,
                words,
                effective,
            },
            &incoming,
            compiled,
            context,
            &mut outcomes,
        );
        outcomes.publish(state);
        outcomes
    }

    /// Receiver-only temporaries do not remain live through native body descent.
    #[inline(never)]
    fn walk_selected_receiver_command(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Option<Box<SourceOutcomes>> {
        let context = *context;
        if let Some(result) = Self::retained_receiver_result(effective, state, context) {
            return Some(Box::new(result));
        }
        if let Some(result) = Self::walk_receiver_builtin_links(effective, state, context) {
            return Some(Box::new(result));
        }
        self.walk_retained_receiver_method(segment.span.start(), words, effective, state, context)
            .map(Box::new)
    }

    /// Live lookup owns its preparation only on the live dispatch branch.
    #[inline(never)]
    fn join_live_source_command(
        &mut self,
        input: SourceCommandInput<'_>,
        incoming: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) {
        let mut live = boxed_source_branch(incoming);
        let selected_context = source_selected_context_boxed(
            context,
            &tcl_registry::native_compilation::NativeCompilationSelection::Generic,
        );
        let mut live_outcomes = boxed_source_outcomes();
        self.walk_resolved_source_command(input, &mut live, &selected_context, &mut live_outcomes);
        outcomes.join(&live_outcomes);
    }

    /// Named and residual branches follow inline execution without retaining
    /// their construction slots across that execution.
    #[inline(never)]
    fn join_other_selected_source_commands(
        &mut self,
        input: SourceCommandInput<'_>,
        incoming: &ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) {
        let SourceCommandInput {
            segment,
            words,
            effective,
            ..
        } = input;
        for proof in &compiled.named {
            outcomes
                .join(&self.walk_named_source_command(
                    segment, words, effective, incoming, proof, context,
                ));
        }
        if compiled.unknown {
            outcomes.join(&self.walk_unknown_compiler_handler(
                segment, words, effective, incoming, compiled, context,
            ));
        }
        if compiled.compile_error {
            let mut residual = boxed_source_branch(incoming);
            residual.mark_opaque_binding_mutation();
            outcomes.join(&SourceOutcomes::invocation(
                &residual,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            ));
        }
    }

    fn walk_unknown_compiler_handler(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        incoming: &ModuleCommandBindings,
        compiled: &compiled_invocation::CompiledInvocationSelection,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::native_compilation::NativeCompilationSelection;
        let mut outcomes = SourceOutcomes::default();
        let head = original_name_value::original_command_head_input(
            words,
            segment.span.start(),
            incoming,
            *context,
        );
        if let Some(target) = compiled.stable_handler_after_original_arguments(
            head.as_ref(),
            incoming,
            &context.namespace_identity(),
        ) && let Some(declaration) =
            self.walk_rule_loader_procedure(segment, words, effective, incoming, target, *context)
        {
            return declaration;
        }
        if let Some((target, transfer)) = compiled
            .stable_handler_after_original_arguments(
                head.as_ref(),
                incoming,
                &context.namespace_identity(),
            )
            .and_then(|target| {
                compiled_invocation::successful_handler_transfer(
                    target,
                    effective,
                    incoming,
                    context.registry,
                    context.realm,
                )
                .map(|transfer| (target, transfer))
            })
        {
            let mut stable = boxed_source_branch(incoming);
            let selected_context =
                source_selected_context_boxed(context, &NativeCompilationSelection::Unknown);
            let mut result = self.walk_source_target(
                segment,
                words,
                effective,
                &mut stable,
                target,
                &selected_context,
            );
            retain_source_native_result(&mut result, incoming, target, effective, context);
            if transfer == compiled_invocation::HandlerSourceTransfer::PossibleBodies {
                // The inventory is conditional on body entry. Retain the
                // unchanged normal branch so these effects remain May.
                result.join(&SourceOutcomes::normal(incoming));
            }
            outcomes.join(&result);
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
                incoming,
            );
        } else {
            let mut residual = boxed_source_branch(incoming);
            residual.mark_opaque_binding_mutation();
            outcomes.join(&SourceOutcomes::invocation(
                &residual,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            ));
        }
        outcomes
    }

    fn walk_named_source_command(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        incoming: &ModuleCommandBindings,
        selected: &SourceNamedInvocationProof,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let proof = selected.with_late_lookup(incoming);
        let Ok(projected) = named_arguments::ProjectedNamedArguments::capture(
            words,
            effective,
            incoming,
            context,
            proof.arguments_from,
        ) else {
            let mut residual = boxed_source_branch(incoming);
            residual.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(&residual, Route::Unknown);
        };
        let mut outcomes = SourceOutcomes::default();
        if proof.unknown {
            let mut residual = boxed_source_branch(incoming);
            residual.mark_opaque_binding_mutation();
            outcomes.join(&SourceOutcomes::invocation(&residual, Route::Unknown));
        }
        if proof.may_be_absent {
            outcomes.add_abrupt(Route::UnknownAbrupt, incoming);
        }
        for target in &proof.targets {
            let mut branch = boxed_source_branch(incoming);
            let selected_context = projected.context(context);
            let context = selected_context.as_ref();
            let mut result = if target.registry_backed {
                self.walk_source_target(
                    segment,
                    &projected.words,
                    &projected.effective,
                    &mut branch,
                    target,
                    context,
                )
            } else {
                self.walk_document_target(
                    segment.span.start(),
                    Some(&projected.words),
                    &projected.effective,
                    &mut branch,
                    target,
                    *context,
                )
            };
            retain_source_native_result(
                &mut result,
                incoming,
                target,
                &projected.effective,
                context,
            );
            outcomes.join(&result);
        }
        outcomes
    }

    fn walk_resolved_source_command(
        &mut self,
        input: SourceCommandInput<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        outcomes: &mut SourceOutcomes,
    ) {
        let SourceCommandInput {
            segment,
            words,
            effective,
            ..
        } = input;
        let namespace = context.namespace_identity();
        let head = original_name_value::original_command_head_input(
            words,
            segment.span.start(),
            state,
            *context,
        );
        #[cfg(debug_assertions)]
        trace_original_generic_head(segment.span.start(), state, head.is_some());
        let Some(head) = head else {
            state.mark_opaque_binding_mutation();
            outcomes.join(&SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            ));
            return;
        };
        let binding = source_binding_from_original_input_boxed(state, &head, &namespace);
        #[cfg(debug_assertions)]
        trace_original_generic_binding(segment.span.start(), binding.as_deref());
        let Some(binding) = binding else {
            state.mark_opaque_binding_mutation();
            outcomes.join(&SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            ));
            return;
        };
        let incoming = boxed_source_branch(state);
        if binding.unknown {
            let mut residual = incoming.clone();
            residual.mark_opaque_binding_mutation();
            residual.loader_handler_unknown = true;
            outcomes.join(&SourceOutcomes::invocation(
                &residual,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            ));
        }
        if binding.may_be_absent {
            outcomes.add_abrupt(
                tcl_registry::completion_route::InvocationCompletionRoute::UnknownAbrupt,
                &incoming,
            );
        }
        for target in &binding.targets {
            let mut alternative = boxed_source_branch(&incoming);
            let mut result = if target.registry_backed {
                self.walk_source_target(
                    segment,
                    words,
                    effective,
                    &mut alternative,
                    target,
                    context,
                )
            } else {
                self.walk_document_target(
                    segment.span.start(),
                    Some(words),
                    effective,
                    &mut alternative,
                    target,
                    *context,
                )
            };
            retain_source_native_result(&mut result, &incoming, target, effective, context);
            #[cfg(debug_assertions)]
            trace_original_generic_result(segment.span.start(), &result);
            outcomes.join(&result);
        }
        outcomes.publish(state);
    }

    fn walk_document_target(
        &mut self,
        site: u32,
        original_words: Option<&[crate::ir::WordExpr]>,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        if self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource {
            if let Some(body) = self.deferred.get(&target.implementation_id()).cloned() {
                self.record_unrepresented_called_body(site, state, target, &body);
            }
            return opaque_source_invocation(state);
        }
        if state.installed_definition_dispatcher(target).is_some() {
            // No constructor or typemethod execution contract is donated by
            // the definition receipt. Its nominal result query is separate.
            return opaque_source_invocation(state);
        }
        if let Some(result) = original_words.and_then(|words| {
            self.walk_original_constructor(site, words, effective, state, target, context)
        }) {
            return result;
        }
        let body = self.deferred.get(&target.implementation_id()).cloned();
        let Some(body) = body else {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        };
        if self.declaration_preview_depth == 0 {
            self.called_implementations.insert(body.implementation_id());
        }
        let implementation = body.implementation_id();
        if !self.active_calls.insert(implementation.clone()) {
            self.record_unrepresented_called_body(site, state, target, &body);
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let result = self.walk_called_body(
            site,
            effective,
            state,
            receiver_self::CalledBodyTarget {
                target,
                receiver: None,
            },
            &body,
            context,
        );
        self.active_calls.remove(&implementation);
        result
    }

    fn walk_called_body(
        &mut self,
        site: u32,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        selected: receiver_self::CalledBodyTarget<'_>,
        body: &DeferredSourceBody,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let target = selected.target;
        if effective.iter().skip(1).any(|word| {
            matches!(
                word,
                crate::registry_invocation::EffectiveInvocationWord::Expanded
                    | crate::registry_invocation::EffectiveInvocationWord::Opaque
            )
        }) {
            self.record_unrepresented_called_body(site, state, target, body);
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let Some(namespace_key) = called_body_namespace(state, target, body) else {
            self.record_unrepresented_called_body(site, state, target, body);
            return opaque_source_invocation(state);
        };
        let holder = namespace_key.display().unwrap_or_default();
        let identity = source_called_body_activation_name(
            state.current_source_origin.as_ref(),
            target,
            body,
            site,
        );
        let frame = called_body_frame(body, identity, &holder, &namespace_key);
        let parent = Arc::clone(&state.source_variables);
        let previous_frame = state.variable_frame.clone();
        let mut called = parent.enter_called_frame(&frame);
        if let Some(statics) = &body.statics {
            called.install_callable_statics(statics, context.registry);
        }
        let actual = source_call_arguments(target, effective);
        match bind_called_body_parameters(&mut called, body, &actual, target, context) {
            Some(true) => {}
            Some(false) => {
                return SourceOutcomes::invocation(
                    state,
                    Route::Tcl(tcl_registry::completion::CompletionCode::Error),
                );
            }
            None => {
                self.record_unrepresented_called_body(site, state, target, body);
                return opaque_source_invocation(state);
            }
        }
        state.bind_called_receiver_variables(
            &mut called,
            body,
            target,
            selected.receiver,
            context.registry,
        );
        retain_called_body_value_formals(state, &mut called, body, actual.len(), target, context);
        let receiver_context = entered_receiver_body_context(
            body,
            target,
            selected.receiver,
            &frame,
            &called,
            context.config,
        );
        self.record_called_body_source(body, &frame, Some(&namespace_key), receiver_context);
        state.source_variables = Arc::new(called);
        state.variable_frame = frame.clone();
        let previous_origin = state.current_source_origin.clone();
        state.current_source_origin.clone_from(&body.source_origin);
        let mut outcomes = self
            .walk_source(
                &body.source,
                body.offset,
                state,
                &SourceExecutionContext {
                    compilation: procedure_compilation(state.baseline.compilation_dialect()),
                    compilation_snapshot: None,
                    selected_compilation: None,
                    original_variable_compilation: None,
                    namespace: &holder,
                    namespace_key: Some(&namespace_key),
                    depth: context.depth + 1,
                    frame: &frame,
                    ..context
                },
            )
            .through_procedure_boundary(state.baseline.dialect);
        outcomes.restore_frame(&parent, &previous_frame);
        outcomes.restore_source_origin(previous_origin.as_ref());
        outcomes.publish(state);
        outcomes
    }

    fn record_called_body_source(
        &mut self,
        body: &DeferredSourceBody,
        frame: &crate::var_resolve::VariableExecutionFrame,
        namespace: Option<&SourceNamespaceKey>,
        receiver_context: Option<Arc<original_receiver_body_context::OriginalReceiverBodyContext>>,
    ) {
        if let Some((parent, argument, script)) = &body.executed_script {
            self.record_executed_script(
                parent.clone(),
                *argument,
                script.clone(),
                Some(frame),
                namespace,
            );
            self.record_original_receiver_body_context(
                parent,
                *argument,
                script,
                frame,
                receiver_context,
            );
            if let Some((origin, span)) = &body.executed_word {
                self.record_executed_word(origin, *span, None, script.clone());
            }
        }
    }

    fn walk_substitutions(
        &mut self,
        word: &crate::ir::WordExpr,
        document: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use crate::ir::WordExpr;
        match word {
            WordExpr::Variable { spelling, source } => {
                self.observe_variable_substitution(spelling, source, document, base, state, context)
            }
            WordExpr::CommandSubstitution { spelling, source } => {
                let script = spelling
                    .strip_prefix('[')
                    .and_then(|text| text.strip_suffix(']'))
                    .unwrap_or(spelling);
                self.walk_source(script, source.span.start() + 1, state, &context)
            }
            WordExpr::Template { parts, .. } => {
                self.walk_template_substitutions(word, parts, document, base, state, context)
            }
            WordExpr::Expand { word, .. } => {
                let mut outcomes = self.walk_substitutions(word, document, base, state, context);
                // Element expansion has an independent list conversion and arity.
                outcomes.normal_completion = None;
                outcomes
            }
            WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => {
                let mut outcomes = SourceOutcomes::normal(state);
                outcomes.retain_complete_normal_evaluation();
                outcomes
            }
            WordExpr::Opaque { .. } => SourceOutcomes::normal(state),
        }
    }

    fn observe_variable_substitution(
        &mut self,
        spelling: &str,
        source: &crate::ir::SourceSite,
        document: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let original = original_variable_source(document, base, source, context.config);
        let arena = original_variable_arena(original.as_ref(), state, context.config);
        let root = arena.as_ref().zip(original.as_ref()).and_then(|(arena, (_, read_site))| {
            let policy = state.source_variables.execution_name_policy?.native_recipe()?;
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_executable(
                arena, arena.image(), context.config, read_site.span,
                tcl_syntax::word_rules::WordValueRules::from_config(&context.config), policy)
        });
        let lexical = original_variable_compilation::OriginalSourceLexicalCompilation::at_source(
            state, context,
        );
        let mut outcomes = SourceOutcomes::normal(state);
        let mut evaluated_index = root.as_ref().and_then(
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::static_index_input,
        );
        if let Some(arena) = &arena {
            for part in arena.list(arena.root()) {
                if let tcl_lexer::ExecutablePart::Variable {
                    index: Some(index), ..
                } = part.part
                {
                    let Some(continuing) = outcomes.normal.take() else {
                        break;
                    };
                    publish_source_branch(state, continuing);
                    let index_outcomes = self.walk_variable_index_arena(
                        SourceIndexView { arena, offset: 0 },
                        index,
                        state,
                        context,
                    );
                    if evaluated_index.is_none() {
                        evaluated_index = index_outcomes.normal_name_value.as_deref().map(|value|
                            crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(
                                crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(value)));
                    }
                    outcomes.join(&index_outcomes);
                }
            }
        }
        if let Some(continuing) = outcomes.normal.take() {
            *state = *continuing;
            let spelling = original.as_ref().map_or(spelling, |(original, _)| original);
            let logical = original_variable_compilation::logical_original_substitution_access(
                original.as_ref(),
                arena.as_ref(),
                state,
                context,
            );
            let access = logical.unwrap_or_else(|| {
                root.as_ref().zip(lexical.as_ref()).map_or_else(
                    crate::place::unknown_top,
                    |(root, lexical)| {
                        lexical.resolve_root(
                            root,
                            evaluated_index.as_ref(),
                            &state.source_variables,
                            context.registry,
                            tcl_registry::TraceOperation::Read,
                        )
                    },
                )
            });
            if let Some((original, read_site)) = &original
                && read_site.provenance == crate::ir::Provenance::Source
            {
                self.record_variable_access(
                    read_site,
                    original,
                    state,
                    context.variable_read_owner,
                    context.registry,
                    &access,
                );
            }
            if access.observed {
                let observed = self.walk_captured_observed_read(
                    source.span.start(),
                    &access,
                    Some(tcl_syntax::naming::var_reference_for_style(
                        spelling,
                        context.config.braced_var,
                    )),
                    state,
                    context,
                );
                outcomes.join(&observed);
            } else {
                let mut normal =
                    source_representation::unobserved_read_result(state, &access, context.registry);
                normal.normal_object = state.object_read_proof(&access, context.registry);
                outcomes.join(&normal);
            }
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_variable_index_arena(
        &mut self,
        source: SourceIndexView<'_>,
        root: tcl_lexer::word_parts::PartListId,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use SourceIndexFrame as Frame;
        use tcl_lexer::word_parts::ExecutablePart;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;

        let new_list = |id, state: &ModuleCommandBindings| Frame::List {
            id,
            next: 0,
            outcomes: Box::new(SourceOutcomes::normal(state)),
            value: Some(String::new()),
            native_value: None,
            native_complete: true,
        };
        let mut pending = vec![new_list(root, state)];
        let mut completed: Option<SourceOutcomes> = None;
        while let Some(frame) = pending.pop() {
            match frame {
                Frame::Variable { id, position } => {
                    let index = completed.take().expect("completed native index list");
                    completed = Some(self.finish_index_variable(
                        source,
                        &source.arena.list(id)[position],
                        index,
                        state,
                        context,
                    ));
                }
                Frame::List {
                    id,
                    next,
                    mut outcomes,
                    mut value,
                    mut native_value,
                    mut native_complete,
                } => {
                    if let Some(fragment) = completed.take() {
                        join_index_fragment(
                            &mut outcomes,
                            &mut value,
                            &fragment,
                            &mut native_value,
                            &mut native_complete,
                        );
                    }
                    let component = source.arena.list(id).get(next);
                    if component.is_none() || outcomes.normal.is_none() {
                        completed = Some(finish_index_list(outcomes, value, native_value, state));
                        continue;
                    }
                    let component = component.expect("remaining index component");
                    publish_source_branch(
                        state,
                        outcomes.normal.take().expect("continuing index branch"),
                    );
                    pending.push(Frame::List {
                        id,
                        next: next + 1,
                        outcomes,
                        value,
                        native_value,
                        native_complete,
                    });
                    match component.part {
                        ExecutablePart::Variable {
                            index: Some(index), ..
                        } => {
                            pending.push(Frame::Variable { id, position: next });
                            pending.push(new_list(index, state));
                        }
                        ExecutablePart::Variable { index: None, .. } => {
                            completed = Some(self.finish_index_variable(
                                source,
                                component,
                                SourceOutcomes::normal(state),
                                state,
                                context,
                            ));
                        }
                        ExecutablePart::Command { body } => {
                            completed = Some(self.walk_index_command(source, body, state, context));
                        }
                        ExecutablePart::Expression { .. } => {
                            completed = Some(opaque_source_invocation(state));
                        }
                        ExecutablePart::ParseError(_) => {
                            completed = Some(SourceOutcomes::invocation(
                                state,
                                Route::Tcl(tcl_registry::completion::CompletionCode::Error),
                            ));
                        }
                        ExecutablePart::Text(_) => {
                            completed = Some(index_text_fragment(source, component, state));
                        }
                    }
                }
            }
        }
        completed.expect("completed original index evaluation")
    }

    fn walk_index_command(
        &mut self,
        source: SourceIndexView<'_>,
        body: tcl_lexer::Span,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        match source
            .arena
            .bytes(body)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .zip(source.offset.checked_add(body.start()))
        {
            Some((script, offset)) => self.walk_source(
                script,
                offset,
                state,
                &SourceExecutionContext {
                    depth: context.depth + 1,
                    ..context
                },
            ),
            None => opaque_source_invocation(state),
        }
    }

    fn finish_index_variable(
        &mut self,
        source: SourceIndexView<'_>,
        component: &tcl_lexer::word_parts::SpannedExecutablePart,
        mut outcomes: SourceOutcomes,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(continuing) = outcomes.normal.take() else {
            return outcomes;
        };
        publish_source_branch(state, continuing);
        let Some(spelling) = source
            .arena
            .bytes(component.span)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            outcomes.join(&opaque_source_invocation(state));
            return outcomes;
        };
        let tcl_lexer::word_parts::ExecutablePart::Variable { index, .. } = component.part else {
            unreachable!("selected index variable component")
        };
        let root = state.source_variables.execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .and_then(|policy| crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_executable(
                source.arena, source.arena.image(), context.config, component.span,
                tcl_syntax::word_rules::WordValueRules::from_config(&context.config), policy));
        let evaluated_index = root.as_ref().and_then(crate::signature_scan::variable_name::SignatureSourceVariableRoot::static_index_input)
            .or_else(|| index.and(outcomes.normal_name_value.as_deref()).map(|value|
                crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(
                    crate::signature_scan::scope::SignatureSourceNameValue::from_original_produced_value(value))));
        let lexical = original_variable_compilation::OriginalSourceLexicalCompilation::at_source(
            state, context,
        );
        let access = root.as_ref().zip(lexical.as_ref()).map_or_else(
            crate::place::unknown_top,
            |(root, lexical)| {
                lexical.resolve_root(
                    root,
                    evaluated_index.as_ref(),
                    &state.source_variables,
                    context.registry,
                    tcl_registry::TraceOperation::Read,
                )
            },
        );
        if let Some(span) = source.arena.source_span(component)
            && let Some((start, end)) = source
                .offset
                .checked_add(span.start())
                .zip(source.offset.checked_add(span.end()))
        {
            self.record_variable_access(
                &crate::ir::SourceSite::source(tcl_lexer::Span::new(start, end)),
                spelling,
                state,
                context.variable_read_owner,
                context.registry,
                &access,
            );
        } else {
            outcomes.join(&opaque_source_invocation(state));
            return outcomes;
        }
        let read = if access.observed {
            self.walk_captured_observed_read(
                context.invocation_offset,
                &access,
                Some(tcl_syntax::naming::var_reference_for_style(
                    spelling,
                    context.config.braced_var,
                )),
                state,
                context,
            )
        } else {
            source_representation::unobserved_read_result(state, &access, context.registry)
        };
        outcomes.join(&read);
        outcomes
    }

    fn record_point(&mut self, point: SourceBindingPoint) {
        self.invalidate_unpositioned_projections();
        if self.root_origin != point.state.current_source_origin {
            self.record_origin_point(point);
            return;
        }
        let indices = self.dispatch_points.entry(point.offset).or_default();
        if let Some(index) = indices.iter().find(|index| {
            self.points[**index].namespace_key == point.namespace_key
                && self.points[**index].declaration_preview == point.declaration_preview
                && self.points[**index].realm == point.realm
                && self.points[**index].state.variable_frame == point.state.variable_frame
        }) {
            self.points[*index].original_arguments_complete_normally &=
                point.original_arguments_complete_normally;
            if point.observer_execution.observed() {
                self.points[*index].observer_execution = SourceObserverExecution::MayObserved;
            }
            if self.points[*index].head != point.head {
                self.points[*index].head = None;
            }
            if self.points[*index].original_head_input != point.original_head_input {
                self.points[*index].original_head_input = None;
            }
            if self.points[*index].original_written_words != point.original_written_words {
                self.points[*index].original_written_words = None;
            }
            join_evaluated_words(
                &mut self.points[*index].evaluated_argument_values,
                &point.evaluated_argument_values,
            );
            self.points[*index].state.join(&point.state);
            self.points[*index]
                .compiled_execution
                .join(&point.compiled_execution);
            if self.points[*index].frozen_written_words != point.frozen_written_words {
                self.points[*index].frozen_written_words = None;
            }
            if self.points[*index].frozen_written_names != point.frozen_written_names {
                self.points[*index].frozen_written_names = None;
            }
            if self.points[*index].frozen_head_object != point.frozen_head_object {
                self.points[*index].frozen_head_object = None;
            }
            self.points[*index]
                .method_prefix_arguments
                .retain(|prefix| point.method_prefix_arguments.contains(prefix));
            compiled_invocation::join_argument_words(
                &mut self.points[*index].evaluated_argument_words,
                &point.evaluated_argument_words,
            );
            self.points[*index].lookup_snapshot = OnceLock::new();
        } else {
            indices.push(self.points.len());
            self.points.push(point);
        }
    }

    fn walk_source_target_after_observers(
        &mut self,
        segment: &crate::segmenter::SegmentedCommand,
        words: &[crate::ir::WordExpr],
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        select_contents_write_source(state);
        if let Some(outcomes) =
            logical_definition::transfer(words, target, segment.span.start(), state, *context)
        {
            return outcomes;
        }
        let incoming = boxed_source_branch(state);
        let prepared = match prepare_source_native_invocation(
            target,
            effective,
            state,
            context.registry,
            context.realm,
        ) {
            Ok(prepared) => prepared,
            Err(SourceNativePreparationError::Unresolved(route)) => {
                state.mark_opaque_binding_mutation();
                return SourceOutcomes::native_invocation(&incoming, state, route);
            }
            Err(SourceNativePreparationError::InvalidArguments) => {
                return SourceOutcomes::invocation(
                    state,
                    tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::completion::CompletionCode::Error,
                    ),
                );
            }
        };
        let invocation = prepared.invocation_boxed(&target.command);
        let facts = prepared.facts.as_ref();
        let selection = context
            .selected_compilation
            .unwrap_or(&tcl_registry::native_compilation::NativeCompilationSelection::Unknown);
        let operands = SourceScriptOperands {
            compilation_spec: facts.native_compilation.as_ref(),
            compilation_selection: selection,
            words,
            written_arguments: context.written_arguments,
            target,
            arguments: invocation.arguments_ref(),
        };
        let original_variable_operands = prepared.original_variable_operands.get_or_init(|| {
            original_name_value::original_variable_invocation(
                operands,
                segment.span.start(),
                state,
                *context,
            )
        });
        self.retain_original_variable_operands(
            state.current_source_origin.as_ref(),
            segment.span.start(),
            original_variable_operands,
        );
        let native = SourceNativeInvocation {
            compilation_spec: facts.native_compilation.as_ref(),
            compilation_selection: selection,
            segment,
            words,
            written_arguments: context.written_arguments,
            target,
            invocation: &invocation,
            original_variable_operands,
        };
        self.walk_prepared_source_target(native, &prepared, effective, state, context, &incoming)
    }

    fn walk_prepared_source_target(
        &mut self,
        native: SourceNativeInvocation<'_>,
        prepared: &PreparedSourceNativeInvocation<'_>,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        incoming: &ModuleCommandBindings,
    ) -> SourceOutcomes {
        let facts = prepared.facts.as_ref();
        // An attested definition recipe owns its installation prerequisites;
        // scripted providers do not borrow a stock normal-handler envelope.
        if let Some(outcomes) = Self::walk_snit_definition(native, state, *context) {
            return outcomes;
        }
        if let Some(outcomes) =
            source_representation::opaque_result_producer(native, facts, state, *context)
        {
            return outcomes;
        }
        let completions = CapturedNativeCompletionReceipts::capture(native, facts, state, context);
        let mut receipts = capture_source_native_results_boxed(native, facts, state, context);
        let route = match self.prepare_native_handler(prepared, native, effective, state, context) {
            Ok(route) => route,
            Err(outcomes) => return *outcomes,
        };
        let read_complete = receipts
            .normal_read
            .as_ref()
            .is_some_and(|proof| proof.is_current(&state.source_variables, context.registry));
        let write_complete = receipts
            .normal_write
            .as_ref()
            .is_some_and(|proof| proof.is_current(&state.source_variables, context.registry));
        let route = if read_complete || write_complete || completions.captured() {
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Ok,
            )
        } else {
            route
        };
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_HANDLER_CLOSED_RECEIVER site={} read_captured={} read_current={} write_current={} route={route:?}",
                native.segment.span.start(),
                receipts.normal_read.is_some(),
                read_complete,
                write_complete
            );
        }
        let mut outcomes =
            self.walk_prepared_native_target(native, state, context, facts, route, incoming);
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_HANDLER_RETURN site={} normal={} abrupt={} normal_dynamic={}",
                native.segment.span.start(),
                outcomes.normal.is_some(),
                outcomes.abrupt.len(),
                outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| normal.source_variables.dynamic_bindings)
            );
        }
        // Implementation contract: naming.variable.original-set-read-completion
        // docs/design/analysis/name-resolution-proofs/original-set-read-completion.md
        if read_complete
            || write_complete
            || completions.completed(&outcomes, native, facts, *context)
        {
            outcomes.retain_closed_native_handler_completion(native, facts, *context);
        }
        finish_source_native_results(
            &mut outcomes,
            native,
            facts,
            incoming,
            context,
            &mut receipts,
        );
        let outcomes = self.finish_native_source_completion(
            outcomes,
            native,
            facts,
            incoming,
            context,
            receipts.normal_write.as_ref().filter(|_| write_complete),
        );
        outcomes.publish(state);
        outcomes
    }

    fn finish_native_source_completion(
        &mut self,
        mut outcomes: SourceOutcomes,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        incoming: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        write: Option<&crate::var_resolve::OriginalNormalValueWrite>,
    ) -> SourceOutcomes {
        // A selected total arithmetic expression owns this certificate.
        // Other native body handlers have independent condition, iteration,
        // restoration and completion obligations, even with a normal child.
        let handler_complete = outcomes
            .native_normal_completion
            .as_ref()
            .is_some_and(|proof| {
                outcomes
                    .normal
                    .as_ref()
                    .is_some_and(|normal| proof.matches(native, facts, normal, *context))
            });
        if handler_complete && let Some(write) = write {
            self.retain_quiet_normal_variables(
                native,
                facts,
                incoming,
                &outcomes,
                write,
                context.registry,
            );
        }
        if facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Expr,
            )
            && !handler_complete
        {
            outcomes.normal_completion = None;
        }
        outcomes.native_normal_completion = None;
        // The default value-only Return has no code/level conversion. Its
        // exact pending successful route becomes Normal only at the actual
        // procedure boundary; options and configured returns remain separate.
        if facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Return,
            )
            && native
                .invocation
                .arguments()
                .exact_argv_len()
                .is_some_and(|count| count <= 1)
            && outcomes.normal.is_none()
            && outcomes.abrupt.len() == 1
            && matches!(
                outcomes.abrupt[0].0,
                tcl_registry::completion_route::InvocationCompletionRoute::Return(
                    tcl_registry::completion_route::ReturnCompletionRoute {
                        eventual_code: tcl_registry::completion::CompletionCode::Ok,
                        remaining_level: 1,
                    }
                )
            )
        {
            outcomes.complete_procedure_return = Some(outcomes.abrupt[0].0);
        }
        outcomes
    }

    /// Retain only the selected handler contract through recursive execution.
    #[inline(never)]
    fn prepare_native_handler(
        &mut self,
        prepared: &PreparedSourceNativeInvocation<'_>,
        native: SourceNativeInvocation<'_>,
        effective: &[crate::registry_invocation::EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> Result<tcl_registry::completion_route::InvocationCompletionRoute, Box<SourceOutcomes>>
    {
        let context = *context;
        let facts = prepared.facts.as_ref();
        if state.command_observer_registration_invalid(facts, &context.namespace_identity()) {
            return Err(Box::new(SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::completion::CompletionCode::Error,
                ),
            )));
        }
        let route =
            container_coercion::ordinary_list_length_completion(native, facts, state, context)
                .unwrap_or_else(|| prepared.completion_route(state, context));
        let implementation =
            native_implementation_dependency_holds(native, facts, effective, state, context);
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_HANDLER_ENTRY site={} implementation={} route={route:?} dynamic={}",
                native.segment.span.start(),
                implementation,
                state.source_variables.dynamic_bindings
            );
        }
        if !implementation {
            return Err(Box::new(opaque_source_invocation(state)));
        }
        self.record_native_invocation_reads(
            facts,
            native.invocation.arguments(),
            state,
            context,
            native.original_variable_operands,
        );
        let object_protocol = self.prepare_object_callbacks(native, facts, state, context);
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_TRANSFER").is_some() {
            eprintln!(
                "ORIGINAL_HANDLER_OBJECTS site={} preserves={} dynamic={}",
                native.segment.span.start(),
                object_protocol.preserves_representation(),
                state.source_variables.dynamic_bindings
            );
        }
        if !object_protocol.preserves_representation() {
            object_callbacks::coerce_closed_list_representations(
                native,
                facts,
                state,
                context,
                object_protocol,
            );
        }
        self.record_rhs_read_store(native, facts, state, context);
        let writes = crate::variable_bindings::source_variable_write_places_with_original_operands(
            facts,
            native.invocation.arguments(),
            &state.source_variables,
            context.registry,
            native.original_variable_operands,
        );
        if !writes.is_empty() {
            Arc::make_mut(&mut state.object_instances).invalidate_writes(&writes);
        }
        Ok(route)
    }

    fn walk_native_body_protocol(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
    ) -> Option<SourceOutcomes> {
        if let Some(outcomes) = Self::walk_array_entries_store(native, facts, state, context) {
            return Some(outcomes);
        }
        match facts.body_execution {
            Some(tcl_registry::body_execution::BodyExecutionSpec::ArrayIteration) => {
                Some(self.walk_array_iteration(native, facts, state, context))
            }
            Some(tcl_registry::body_execution::BodyExecutionSpec::SourceFile) => {
                Some(self.walk_selected_source_file(native, state, context))
            }
            Some(tcl_registry::body_execution::BodyExecutionSpec::SubstitutionTemplate) => {
                Some(self.walk_native_substitution_template(native, state, context))
            }
            _ => None,
        }
    }

    fn walk_prepared_native_target(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        incoming: &ModuleCommandBindings,
    ) -> SourceOutcomes {
        if let Some(outcomes) = trusted_package_transfer::OriginalTrustedPackageTransfer::capture(
            native, facts, state, *context,
        )
        .map(|transfer| transfer.finish(native, facts, state, *context))
        {
            return outcomes;
        }
        if self.defers_runtime_body(native, facts) {
            return opaque_source_invocation(state);
        }
        let route = match self.prepare_native_target(native, state, context, facts, route, incoming)
        {
            Ok(route) => route,
            Err(outcomes) => return *outcomes,
        };
        if let Some(tcl_registry::body_execution::BodyExecutionSpec::CapturedLifecycle(spec)) =
            facts.body_execution.as_ref()
        {
            return self.walk_prepared_native_lifecycle(spec, native, state, *context);
        }
        let mut outcomes =
            self.walk_selected_native_body(native, state, context, facts, route, incoming);
        transfer_native_completion_outputs(&mut outcomes, native, facts, route, *context);
        outcomes.publish(state);
        outcomes
    }

    /// Runtime body entry cannot prepare a child of a generic invocation.
    /// Native compiler-selected bodies retain their own traversal and checks.
    fn defers_runtime_body(
        &self,
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
    ) -> bool {
        use tcl_registry::{
            body_execution::BodyExecutionSpec,
            native_compilation::NativeCompilationSelection,
            script_body_flow::{ScriptBodyFlow, script_body_flow},
        };
        self.compilation_scope == tcl_runtime_api::SourceCompilationScope::EnteredSource
            && !matches!(
                native.compilation_selection,
                NativeCompilationSelection::Inline { .. }
            )
            && (matches!(
                facts.body_execution,
                Some(
                    BodyExecutionSpec::SourceFile
                        | BodyExecutionSpec::ArrayIteration
                        | BodyExecutionSpec::SubstitutionTemplate
                )
            ) || !matches!(
                script_body_flow(facts),
                ScriptBodyFlow::None
                    | ScriptBodyFlow::Deferred
                    | ScriptBodyFlow::Expressions(_)
                    | ScriptBodyFlow::ConcatenatedExpression { .. }
            ))
    }

    /// Finish cold handler/definition checks before retaining a recursive body.
    #[inline(never)]
    fn prepare_native_target(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        incoming: &ModuleCommandBindings,
    ) -> Result<tcl_registry::completion_route::InvocationCompletionRoute, Box<SourceOutcomes>>
    {
        let context = *context;
        if let Some(outcomes) = self.walk_native_body_protocol(native, state, context, facts) {
            return Err(Box::new(outcomes));
        }
        if let Some(outcomes) = variable_outputs::walk_known_outputs(native, facts, state, context)
        {
            return Err(Box::new(outcomes));
        }
        if let Some(refusal) = procedure_body_admission_refusal(native, facts, context.realm, state)
        {
            return Err(Box::new(refusal));
        }
        if let Some(outcomes) =
            self.walk_observed_native_store(native, facts, state, context, route, incoming)
        {
            self.retain_captured_normal_variables(
                native,
                facts,
                incoming,
                &outcomes,
                context.registry,
            );
            return Err(Box::new(outcomes));
        }
        let route =
            self.prepare_native_definition(native, state, context, facts, route, incoming)?;
        Ok(route)
    }

    /// Only lifecycle entry retains its dialect-bearing argument construction.
    #[inline(never)]
    fn walk_prepared_native_lifecycle(
        &mut self,
        spec: &tcl_registry::body_execution::CapturedLifecycleSpec,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        self.walk_lifecycle(
            SourceLifecycleInvocation {
                spec,
                arguments: native.invocation.arguments(),
                site: native.segment.span.start(),
                words: native.words,
                written_arguments: context.written_arguments,
                target: native.target,
            },
            state,
            context,
        )
    }

    /// End definition and registration temporaries before recursive body execution.
    #[inline(never)]
    fn prepare_native_definition(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        incoming: &ModuleCommandBindings,
    ) -> Result<tcl_registry::completion_route::InvocationCompletionRoute, Box<SourceOutcomes>>
    {
        if let Some(outcomes) =
            self.walk_closed_own_object_method_configuration(native, facts, state, context)
        {
            return Err(Box::new(outcomes));
        }
        let class_transfer = original_class_factory_transfer::OriginalClassFactoryTransfer::capture(
            native, facts, state, context,
        );
        self.retain_standalone_definition_references(native, facts, state, context);
        let definition = match transfer_native_definition(native, state, context, facts) {
            Ok(definition) => definition,
            Err(crate::raw_binding::StaticCaptureResult::Invalid) => {
                return Err(Box::new(SourceOutcomes::invocation(
                    state,
                    tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::completion::CompletionCode::Error,
                    ),
                )));
            }
            Err(_) => return Err(Box::new(opaque_source_invocation(state))),
        };
        let procedure_complete = closed_procedure_definition(native, facts, state, context);
        let route = if procedure_complete {
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                tcl_registry::completion::CompletionCode::Ok,
            )
        } else {
            route
        };
        let deferred_class = self.register_bounded_methods(native, state, context, facts);
        if state.record_bounded_class(facts, native.words, native.target, context) || deferred_class
        {
            let complete = deferred_class
                && class_transfer.as_ref().is_some_and(|transfer| {
                    transfer.retain_completed_definition(native, facts, state, context)
                });
            let route = if complete {
                tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                    tcl_registry::CompletionCode::Ok,
                )
            } else {
                route
            };
            let mut outcomes = SourceOutcomes::native_invocation(incoming, state, route);
            if complete {
                outcomes.retain_closed_native_handler_completion(native, facts, context);
            }
            return Err(Box::new(outcomes));
        }
        if facts
            .traits
            .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
            && !facts
                .state_transitions
                .declared()
                .is_some_and(|transitions| {
                    transitions
                        .facts()
                        .iter()
                        .any(|fact| matches!(fact.transition, StateTransition::Package(_)))
                })
        {
            state.mark_opaque_binding_mutation();
            state.loader_handler_unknown = true;
        }
        if facts.traits.contains(tcl_registry::Traits::DEFERS_BODY) {
            self.register_deferred_native(
                native,
                state,
                context,
                facts,
                definition.statics.as_ref(),
            );
            let mut outcomes = SourceOutcomes::native_invocation(incoming, state, route);
            if procedure_complete {
                outcomes.retain_closed_native_handler_completion(native, facts, context);
            }
            return Err(Box::new(outcomes));
        }
        Ok(route)
    }

    fn register_deferred_native(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        statics: Option<&crate::raw_binding::CapturedStaticBindings>,
    ) {
        if matches!(
            facts.procedure_definition,
            Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
        ) {
            self.register_deferred_procedure(native, state, context, facts, statics);
        } else if facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::When,
            )
        {
            self.register_deferred_event(native, state, context, facts);
        } else {
            self.register_future_body(native, state, context, facts);
        }
    }

    fn register_deferred_procedure(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        statics: Option<&crate::raw_binding::CapturedStaticBindings>,
    ) {
        let SourceNativeInvocation {
            segment,
            invocation,
            ..
        } = native;
        let original_parameters = formal_topology::from_invocation(native, facts, state, context);
        let Some(parameters) = original_parameters
            .as_ref()
            .map(formal_topology::OriginalFormalTopology::advisory_parameters)
            .or_else(|| native_procedure_parameters(facts, invocation.arguments()))
        else {
            return;
        };
        let Some(key) = original_procedure_publication_key(native, state, context, facts) else {
            return;
        };
        let namespace_key = key.holder().into_owned();
        let holder = namespace_key.display().unwrap_or_default();
        for (index, role) in &facts.arg_roles {
            if *role != tcl_registry::ArgRole::Body {
                continue;
            }
            if let Some(script) = retained_script_operand(
                facts.argument_offset + usize::from(*index),
                native.script_operands(),
                state,
                segment.span.start(),
                context.config,
            ) {
                let Ok(source) = script.try_text() else {
                    continue;
                };
                let source = source.to_owned();
                let offset = script.base();
                let source_origin = Arc::clone(&script.origin);
                let entered = state.current_source_origin.as_ref().map(|origin| {
                    (
                        CommandAllocationSite {
                            source: Arc::clone(origin),
                            offset: segment.span.start(),
                        },
                        facts.argument_offset + usize::from(*index),
                        script.clone(),
                    )
                });
                for created in state.installed_procedure_targets(&key, segment.span.start()) {
                    let Some(identity) = created.token.map(|identity| identity.origin) else {
                        continue;
                    };
                    self.deferred.insert(
                        DeferredImplementationId {
                            command: identity.clone(),
                            generation: segment.span.start(),
                            allocation: created.implementation_allocation.clone(),
                        },
                        DeferredSourceBody {
                            realm: evaluated_script_realm(
                                facts.argument_offset + usize::from(*index)
                                    ..facts.argument_offset + usize::from(*index) + 1,
                                native.script_operands(),
                                context,
                            ),
                            identity,
                            implementation_generation: segment.span.start(),
                            source: source.clone(),
                            offset,
                            namespace: holder.clone(),
                            namespace_key: namespace_key.clone(),
                            event: None,
                            receiver_method: false,
                            future_frame: None,
                            parameters: parameters.clone(),
                            original_parameters: original_parameters.clone(),
                            statics: statics.cloned(),
                            implementation_allocation: created.implementation_allocation,
                            source_origin: Some(Arc::clone(&source_origin)),
                            executed_script: entered.clone(),
                            executed_word: facts
                                .argument_offset
                                .checked_add(usize::from(*index))
                                .and_then(|argument| {
                                    argument.checked_sub(native.target.prepended.len())
                                })
                                .and_then(|written| native.words.get(written + 1))
                                .zip(state.current_source_origin.as_ref())
                                .map(|(word, origin)| (Arc::clone(origin), word.source().span)),
                        },
                    );
                }
            }
        }
    }

    fn register_deferred_event(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        facts: &InvocationFacts,
    ) {
        let SourceNativeInvocation {
            segment,
            invocation,
            ..
        } = native;
        let event = invocation.arguments().literal_at(0).map(str::to_owned);
        for (index, role) in &facts.arg_roles {
            if *role != tcl_registry::ArgRole::Body {
                continue;
            }
            if let Some(script) = retained_script_operand(
                facts.argument_offset + usize::from(*index),
                native.script_operands(),
                state,
                segment.span.start(),
                context.config,
            ) {
                let Ok(source) = script.try_text() else {
                    continue;
                };
                let source = source.to_owned();
                let offset = script.base();
                let source_origin = Arc::clone(&script.origin);
                let entered = state.current_source_origin.as_ref().map(|origin| {
                    (
                        CommandAllocationSite {
                            source: Arc::clone(origin),
                            offset: segment.span.start(),
                        },
                        facts.argument_offset + usize::from(*index),
                        script.clone(),
                    )
                });
                let identity = format!("event:{}", segment.span.start());
                self.deferred.insert(
                    DeferredImplementationId {
                        command: identity.clone(),
                        generation: segment.span.start(),
                        allocation: None,
                    },
                    DeferredSourceBody {
                        realm: evaluated_script_realm(
                            facts.argument_offset + usize::from(*index)
                                ..facts.argument_offset + usize::from(*index) + 1,
                            native.script_operands(),
                            context,
                        ),
                        identity,
                        implementation_generation: segment.span.start(),
                        source,
                        offset,
                        namespace: "::".to_owned(),
                        namespace_key: state.native_root_namespace_key().unwrap_or_default(),
                        event: Some(event.clone().unwrap_or_default()),
                        receiver_method: false,
                        future_frame: None,
                        parameters: Vec::new(),
                        original_parameters: None,
                        statics: None,
                        implementation_allocation: None,
                        source_origin: Some(source_origin),
                        executed_script: entered,
                        executed_word: facts
                            .argument_offset
                            .checked_add(usize::from(*index))
                            .and_then(|argument| {
                                argument.checked_sub(native.target.prepended.len())
                            })
                            .and_then(|written| native.words.get(written + 1))
                            .zip(state.current_source_origin.as_ref())
                            .map(|(word, origin)| (Arc::clone(origin), word.source().span)),
                    },
                );
            }
        }
    }

    fn walk_selected_native_body(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        facts: &InvocationFacts,
        route: tcl_registry::completion_route::InvocationCompletionRoute,
        incoming: &ModuleCommandBindings,
    ) -> SourceOutcomes {
        let prepared = match prepare_selected_native_body(native, state, context, facts, route) {
            Ok(prepared) => prepared,
            Err(route) => return SourceOutcomes::native_invocation(incoming, state, route),
        };
        let body_context = source_body_context_boxed(
            context,
            &prepared.selection.namespace,
            &prepared.selection.frame,
        );
        let body_context = Box::new(SourceExecutionContext {
            namespace_key: Some(&prepared.selection.namespace_key),
            ..*body_context
        });
        let namespace_body = boxed_source_completion_receipt(|| {
            original_namespace_body::OriginalNamespaceBody::capture(
                native, facts, state, *context, &prepared,
            )
        });
        let mut outcomes = self.walk_body_flow(&prepared.flow, native, state, &body_context);
        if prepared.selection.namespace_activation
            || prepared.parent_frame != prepared.selection.frame
        {
            outcomes.restore_frame(&prepared.parent_variables, &prepared.parent_frame);
        }
        if namespace_body
            .as_ref()
            .is_some_and(|receipt| receipt.completed(native, &outcomes, *context, &prepared))
        {
            outcomes.retain_closed_native_handler_completion(native, facts, *context);
        }
        outcomes.publish(state);
        outcomes
    }

    fn record_phase(
        &mut self,
        site: u32,
        phase: SourceBodyPhase,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) {
        if let Some(previous) = self.phases.iter_mut().find(|point| {
            point.site == site
                && point.phase == phase
                && point.point.namespace_key == context.namespace_identity()
                && point.point.realm == context.realm
                && point.point.state.current_source_origin == state.current_source_origin
                && point.point.declaration_preview == (self.declaration_preview_depth != 0)
                && point.point.state.variable_frame == state.variable_frame
        }) {
            previous.point.state.join(state);
            previous.point.lookup_snapshot = OnceLock::new();
        } else {
            self.phases.push(SourcePhasePoint {
                site,
                phase,
                point: SourceBindingPoint {
                    declaration_preview: self.declaration_preview_depth != 0,
                    original_arguments_complete_normally: false,
                    observer_execution: SourceObserverExecution::Unobserved,
                    realm: context.realm,
                    compilation: context.compilation,
                    compiled_execution: compiled_invocation::CompiledInvocationSelection::default(),
                    head: None,
                    original_head_input: None,
                    original_written_words: None,
                    evaluated_argument_values: Vec::new(),
                    evaluated_argument_words: Vec::new(),
                    frozen_written_words: None,
                    frozen_written_names: None,
                    frozen_head_object: None,
                    method_prefix_arguments: Vec::new(),
                    offset: site,
                    end: site,
                    namespace: context.namespace.to_owned(),
                    namespace_key: context.namespace_identity(),
                    state: state.clone(),
                    lookup_snapshot: OnceLock::new(),
                    dispatch: false,
                },
            });
        }
    }

    fn walk_lifecycle(
        &mut self,
        invocation: SourceLifecycleInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let repetition = state
            .loaded_provider(invocation.spec.provider.package)
            .map_or(
                tcl_registry::body_execution::BodyRepetition::MayRepeat,
                |loader| invocation.spec.repetition(loader.version.as_deref()),
            );
        let mut outcomes = SourceOutcomes::normal(state);
        let mut head = boxed_source_branch(state);
        loop {
            let before = head.clone();
            // A transfer may publish into its mutable input, including an
            // early unknown-provider path. Keep the loop predecessor intact
            // so differing issuance histories meet at the back edge rather
            // than comparing a newly issued stamp with itself.
            let mut iteration_state = head.clone();
            let iteration =
                self.walk_lifecycle_iteration(invocation, &mut iteration_state, context);
            outcomes.join(&iteration);
            if let Some(normal) = iteration.normal {
                head.join(&normal);
            }
            if repetition == tcl_registry::body_execution::BodyRepetition::Once
                || head.same_state(&before)
            {
                break;
            }
        }
        outcomes.publish(state);
        outcomes
    }

    fn begin_lifecycle_coverage(
        &mut self,
        invocation_site: u32,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<(CommandAllocationSite, usize)> {
        state.current_source_origin.as_ref().map(|origin| {
            let site = CommandAllocationSite {
                source: Arc::clone(origin),
                offset: invocation_site,
            };
            let entries = self.lifecycle_coverage.entry(site.clone()).or_default();
            let index = entries.len();
            entries.push(SourceLifecycleCoverage {
                declaration_preview: self.declaration_preview_depth != 0,
                frame: state.variable_frame.clone(),
                namespace: context.namespace_identity(),
                entered: None,
            });
            (site, index)
        })
    }

    fn walk_lifecycle_iteration(
        &mut self,
        invocation: SourceLifecycleInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::body_execution::BodyExecutionSelection;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let coverage = self.begin_lifecycle_coverage(invocation.site, state, context);
        let mut entered = [false; 3];
        let spec = invocation.spec;
        if spec.has_unmodelled_pre_phase_effects(invocation.arguments) {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let Some(loader) = state.loaded_provider(spec.provider.package).cloned() else {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        };
        if loader.implementation_id != spec.provider.implementation_id
            || !state.provider_surface_is_live(&loader)
        {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let phases = match spec.select(invocation.arguments) {
            BodyExecutionSelection::CapturedLifecycle(phases) => phases,
            BodyExecutionSelection::InvalidArguments => {
                return SourceOutcomes::invocation(
                    state,
                    Route::Tcl(tcl_registry::completion::CompletionCode::Error),
                );
            }
            BodyExecutionSelection::UnknownArguments => {
                state.mark_opaque_binding_mutation();
                return SourceOutcomes::invocation(state, Route::Unknown);
            }
        };
        let incoming = boxed_source_branch(state);
        let mut pending = SourceOutcomes::normal(state);
        for (phase, operand, hook) in [
            (
                SourceBodyPhase::Setup,
                phases.setup,
                spec.hook_commands.first(),
            ),
            (
                SourceBodyPhase::Body,
                phases.body,
                spec.hook_commands.get(1),
            ),
        ] {
            let Some(mut normal) = pending.normal.take() else {
                break;
            };
            entered[match phase {
                SourceBodyPhase::Setup => 0,
                SourceBodyPhase::Body => 1,
                SourceBodyPhase::Cleanup => 2,
            }] = true;
            let executed = self.walk_lifecycle_phase(
                invocation,
                SourceLifecyclePhase {
                    phase,
                    operand,
                    hook: hook.copied(),
                },
                &loader,
                &mut normal,
                context,
            );
            pending.join(&executed);
        }
        // Stock lifecycle catches phase completions. Only normal setup enters
        // the body; every catchable completion proceeds through cleanup.
        let mut escaped = pending.capture_tcl_completions();
        if let Some(mut cleanup_entry) = escaped.normal.take() {
            entered[2] = true;
            let cleanup = self.walk_lifecycle_phase(
                invocation,
                SourceLifecyclePhase {
                    phase: SourceBodyPhase::Cleanup,
                    operand: phases.cleanup,
                    hook: spec.hook_commands.get(2).copied(),
                },
                &loader,
                &mut cleanup_entry,
                context,
            );
            escaped.join(&cleanup.capture_tcl_completions());
        }
        if let Some((site, index)) = coverage {
            self.lifecycle_coverage
                .get_mut(&site)
                .expect("recorded lifecycle traversal")[index]
                .entered = Some(entered);
        }
        // Test-selection skip is an independently possible successful path.
        escaped.join(&SourceOutcomes::normal(&incoming));
        escaped.publish(state);
        escaped
    }

    fn walk_lifecycle_phase(
        &mut self,
        invocation: SourceLifecycleInvocation<'_>,
        selected: SourceLifecyclePhase<'_>,
        loader: &TrustedPackageLoader,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let SourceLifecyclePhase {
            phase,
            operand,
            hook,
        } = selected;
        if !state.provider_surface_is_live(loader) {
            state.mark_opaque_binding_mutation();
        }
        self.record_phase(invocation.site, phase, state, context);
        let hooks = invocation
            .spec
            .required_absent_hooks(loader.version.as_deref());
        if hook.is_some_and(|hook| {
            hooks.contains(&hook) && !state.definitely_absent(hook, context.namespace)
        }) {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let Some(operand) = operand else {
            return SourceOutcomes::normal(state);
        };
        if operand.list_element.is_some() {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        self.walk_body_operand(
            operand.argument,
            SourceScriptOperands {
                compilation_spec: Some(&tcl_registry::native_compilation::NativeCompilationSpec {
                    grammar: tcl_registry::native_compilation::NativeCompilationGrammar::NoHook,
                    operation: tcl_registry::SemanticOperationId::Invoke,
                    body: invocation.spec.phase_compilation,
                }),
                compilation_selection:
                    &tcl_registry::native_compilation::NativeCompilationSelection::Generic,
                words: invocation.words,
                written_arguments: invocation.written_arguments,
                target: invocation.target,
                arguments: &invocation.arguments,
            },
            state,
            &SourceExecutionContext {
                depth: context.depth + 1,
                ..context
            },
        )
    }

    fn walk_expression_sequence(
        &mut self,
        indices: &[usize],
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut evaluated = SourceOutcomes::normal(state);
        for &index in indices {
            let Some(mut normal) = evaluated.normal.take() else {
                break;
            };
            evaluated.join(&self.walk_expression_operand(index, operands, &mut normal, context));
        }
        evaluated
    }

    fn walk_body_flow(
        &mut self,
        flow: &tcl_registry::script_body_flow::ScriptBodyFlow,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::script_body_flow::ScriptBodyFlow;
        let outcomes = match flow {
            ScriptBodyFlow::Conditional(branches) => {
                self.walk_conditional(branches, native.script_operands(), state, *context)
            }
            ScriptBodyFlow::Loop {
                initial,
                conditions,
                repeated,
                continued,
            } => self.walk_loop_body_flow(
                SourceLoopBodyIndices {
                    initial,
                    conditions,
                    repeated,
                    continued,
                },
                native,
                state,
                context,
            ),
            other => self.walk_non_loop_body_flow(other, native, state, context),
        };
        outcomes.publish(state);
        outcomes
    }

    fn walk_loop_body_flow(
        &mut self,
        indices: SourceLoopBodyIndices<'_>,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let variable_lists =
            source_iteration_variable_lists(native, context.registry, context.realm);
        let facts = resolve_source_invocation_facts(
            context.registry,
            None,
            *native.invocation,
            context.realm,
        );
        let entry = facts.as_ref().map_or(
            tcl_registry::iteration_entry::IterationEntry::Unknown,
            |facts| facts.iteration_entry(native.invocation.arguments()),
        );
        let finite = facts
            .as_ref()
            .and_then(|facts| facts.finite_iteration(native.invocation.arguments()));
        let loop_context = source_loop_context_boxed(context);
        self.walk_source_loop(
            SourceLoopOperands {
                entry,
                finite: finite.as_ref(),
                variable_lists: &variable_lists,
                initial: indices.initial,
                conditions: indices.conditions,
                repeated: indices.repeated,
                continued: indices.continued,
            },
            native.script_operands(),
            state,
            &loop_context,
        )
    }

    /// Borrow the selected grammar across recursive bodies. Owning a clone of
    /// the largest selection would charge every nested sequence for its stack.
    fn walk_non_loop_body_flow(
        &mut self,
        flow: &tcl_registry::script_body_flow::ScriptBodyFlow,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::script_body_flow::ScriptBodyFlow;
        match flow {
            ScriptBodyFlow::Deferred | ScriptBodyFlow::None | ScriptBodyFlow::CapturedLifecycle => {
                SourceOutcomes::normal(state)
            }
            ScriptBodyFlow::Sequence(indices) => {
                self.walk_body_sequence(indices, native.script_operands(), state, context)
            }
            ScriptBodyFlow::ConcatenatedScript { argument_offset } => {
                self.walk_concatenated_script(*argument_offset, native, state, *context)
            }
            other => self.walk_other_body_flow(other, native, state, context),
        }
    }

    /// Other grammar temporaries stay outside source-sequence recursion.
    #[inline(never)]
    fn walk_other_body_flow(
        &mut self,
        flow: &tcl_registry::script_body_flow::ScriptBodyFlow,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::script_body_flow::ScriptBodyFlow;
        match flow {
            ScriptBodyFlow::ConcatenatedExpression { argument_offset } => {
                self.walk_concatenated_expression(*argument_offset, native, state, *context)
            }
            ScriptBodyFlow::Lambda(_)
            | ScriptBodyFlow::DictionaryScope(_)
            | ScriptBodyFlow::Capture(_) => {
                self.walk_owned_body_selection(flow, native, state, *context)
            }
            ScriptBodyFlow::Expressions(indices) => {
                self.walk_expression_sequence(indices, native.script_operands(), state, *context)
            }
            ScriptBodyFlow::Try(selection) => {
                self.walk_try_bodies(selection.as_ref(), native, state, *context)
            }
            ScriptBodyFlow::Unknown(indices) => {
                state.mark_opaque_binding_mutation();
                self.walk_possible_body_alternatives(indices, native, state, *context, false)
            }
            ScriptBodyFlow::CaseBodies(selection) => {
                self.walk_case_bodies(selection, native, state, *context)
            }
            ScriptBodyFlow::Alternatives(indices) => {
                self.walk_possible_body_alternatives(indices, native, state, *context, true)
            }
            ScriptBodyFlow::Conditional(_) => {
                unreachable!("conditional flow uses its ordered condition handler")
            }
            ScriptBodyFlow::Loop { .. } => unreachable!("loop flow uses its iteration handler"),
            ScriptBodyFlow::Deferred
            | ScriptBodyFlow::None
            | ScriptBodyFlow::CapturedLifecycle
            | ScriptBodyFlow::Sequence(_)
            | ScriptBodyFlow::ConcatenatedScript { .. } => {
                unreachable!("source sequences use their borrowed context")
            }
        }
    }

    /// Only these handlers consume a selection. Their owned temporaries stay
    /// outside the common sequence/namespace recursion path.
    #[inline(never)]
    fn walk_owned_body_selection(
        &mut self,
        flow: &tcl_registry::script_body_flow::ScriptBodyFlow,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::script_body_flow::ScriptBodyFlow;
        match flow {
            ScriptBodyFlow::Lambda(selection) => {
                self.walk_lambda_invocation(selection.clone(), native, state, context)
            }
            ScriptBodyFlow::DictionaryScope(selection) => {
                self.walk_dictionary_scope(selection.clone(), native, state, context)
            }
            ScriptBodyFlow::Capture(selection) => {
                self.walk_captured_operand(*selection, native.script_operands(), state, context)
            }
            _ => unreachable!("only owned selections enter this handler"),
        }
    }

    fn walk_possible_body_alternatives(
        &mut self,
        indices: &[usize],
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
        retain_unchanged: bool,
    ) -> SourceOutcomes {
        let mut possible = if retain_unchanged {
            SourceOutcomes::normal(state)
        } else {
            SourceOutcomes::invocation(
                state,
                tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
            )
        };
        let incoming = boxed_source_branch(state);
        for &index in indices {
            let mut branch = boxed_source_branch(&incoming);
            possible.join(&self.walk_body_operand(
                index,
                native.script_operands(),
                &mut branch,
                &context,
            ));
        }
        possible
    }

    fn walk_captured_operand(
        &mut self,
        selection: tcl_registry::catch_invocation::CatchInvocationSelection,
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::catch_invocation::CatchInvocationSelection;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        match selection {
            CatchInvocationSelection::Valid(selected) => self
                .walk_body_operand(selected.script_at, operands, state, &context)
                .capture_with(selected),
            CatchInvocationSelection::Invalid => SourceOutcomes::invocation(
                state,
                Route::Tcl(tcl_registry::completion::CompletionCode::Error),
            ),
            CatchInvocationSelection::Unknown => {
                state.mark_opaque_binding_mutation();
                SourceOutcomes::invocation(state, Route::Unknown)
            }
        }
    }

    fn walk_lambda_invocation(
        &mut self,
        selection: tcl_registry::lambda_invocation::LambdaInvocationSelection,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion::CompletionCode as Code;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let (lambda, parameters) = match prepare_source_lambda(selection, state.baseline.dialect) {
            PreparedSourceLambda::Ready { lambda, parameters } => (lambda, parameters),
            PreparedSourceLambda::Invalid => {
                return SourceOutcomes::invocation(state, Route::Tcl(Code::Error));
            }
            PreparedSourceLambda::Unknown => return opaque_source_invocation(state),
        };
        let arguments = native.invocation.arguments();
        let actual = (lambda.call_arguments_from..arguments.exact_argv_len().unwrap())
            .map(|index| arguments.literal_at(index))
            .collect::<Vec<_>>();
        let Some(namespace_key) = native
            .original_variable_operands
            .input(lambda.lambda_argument, &state.source_variables)
            .and_then(|input| state.namespace_for_lambda_operand(input, context.config))
        else {
            return opaque_source_invocation(state);
        };
        let namespace = namespace_key.display().unwrap_or_default();
        let frame = source_lambda_frame(
            state,
            &namespace,
            &namespace_key,
            native.segment.span.start(),
        );
        let parent_variables = Arc::clone(&state.source_variables);
        let parent_frame = state.variable_frame.clone();
        let mut variables = parent_variables.enter_called_frame(&frame);
        match bind_source_parameters(
            &mut variables,
            &parameters,
            &actual,
            lambda.body.is_empty(),
            context.registry,
            None,
        ) {
            Some(true) => {}
            Some(false) => return SourceOutcomes::invocation(state, Route::Tcl(Code::Error)),
            None => return opaque_source_invocation(state),
        }
        retain_lambda_value_formals(
            state,
            &mut variables,
            &parameters,
            actual.len(),
            &lambda,
            native,
            context,
        );
        let Some(script) = retained_lambda_body(&lambda, native, state, context.config) else {
            return opaque_source_invocation(state);
        };
        self.record_lambda_body(
            &lambda,
            native,
            state,
            &script,
            &frame,
            Some(&namespace_key),
        );
        let realm = evaluated_script_realm(
            lambda.lambda_argument..lambda.lambda_argument + 1,
            native.script_operands(),
            context,
        );
        let offset = script.base();
        let parent_origin = state.current_source_origin.replace(script.origin);
        state.source_variables = Arc::new(variables);
        state.variable_frame = frame.clone();
        let mut outcomes = self
            .walk_source(
                &lambda.body,
                offset,
                state,
                &SourceExecutionContext {
                    realm,
                    compilation: procedure_compilation(state.baseline.compilation_dialect()),
                    compilation_snapshot: None,
                    selected_compilation: None,
                    original_variable_compilation: None,
                    namespace: &namespace,
                    namespace_key: Some(&namespace_key),
                    frame: &frame,
                    depth: context.depth + 1,
                    ..context
                },
            )
            .through_procedure_boundary(state.baseline.dialect);
        outcomes.restore_frame(&parent_variables, &parent_frame);
        outcomes.restore_source_origin(parent_origin.as_ref());
        outcomes.publish(state);
        outcomes
    }

    fn record_lambda_body(
        &mut self,
        lambda: &tcl_registry::lambda_invocation::LambdaInvocation,
        native: SourceNativeInvocation<'_>,
        state: &ModuleCommandBindings,
        script: &ExecutedScriptSource,
        frame: &crate::var_resolve::VariableExecutionFrame,
        namespace: Option<&SourceNamespaceKey>,
    ) {
        let parent = CommandAllocationSite {
            source: Arc::clone(state.current_source_origin.as_ref().unwrap()),
            offset: native.segment.span.start(),
        };
        if let Some(word) = lambda
            .lambda_argument
            .checked_sub(native.target.prepended.len())
            .and_then(|written| native.words.get(written + 1))
        {
            self.record_executed_word(&parent.source, word.source().span, Some(1), script.clone());
        }
        self.record_executed_script(
            parent,
            lambda.lambda_argument,
            script.clone(),
            Some(frame),
            namespace,
        );
    }

    fn walk_body_sequence(
        &mut self,
        indices: &[usize],
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::normal(state);
        for index in indices {
            let Some(mut normal) = outcomes.normal.take() else {
                break;
            };
            outcomes.join(&self.walk_body_operand(*index, operands, &mut normal, context));
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_concatenated_script(
        &mut self,
        argument_offset: usize,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        if let Some(count) = native.invocation.arguments().exact_argv_len()
            && argument_offset.checked_add(1) == Some(count)
        {
            return self.walk_body_operand(
                argument_offset,
                native.script_operands(),
                state,
                &context,
            );
        }
        let Some(values) = frozen_script_operands(native.invocation.arguments(), argument_offset)
        else {
            return opaque_source_invocation(state);
        };
        let script = if values.len() == 1 {
            values[0].to_owned()
        } else if native
            .invocation
            .arguments()
            .dialect()
            .is_some_and(|dialect| {
                matches!(
                    dialect.concat_policy(),
                    Some(tcl_dialect::ConcatPolicy::Tcl(_))
                )
            })
        {
            tcl_syntax::list::concat_values(values.iter().copied())
        } else {
            return opaque_source_invocation(state);
        };
        let realm = evaluated_script_realm(
            argument_offset..argument_offset + values.len(),
            native.script_operands(),
            context,
        );
        let context = compiled_invocation::body_context(
            argument_offset,
            native.script_operands(),
            state,
            context,
        );
        self.walk_materialised_source(
            &script,
            argument_offset..argument_offset + values.len(),
            MaterialisedSourceKind::Script,
            state,
            SourceExecutionContext { realm, ..context },
        )
    }

    fn walk_materialised_source(
        &mut self,
        source: &str,
        arguments: std::ops::Range<usize>,
        kind: MaterialisedSourceKind,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(origin) = state.current_source_origin.clone() else {
            return opaque_source_invocation(state);
        };
        let expression_owner = SourceVariableEvaluationOwner::NativeExpression {
            invocation: CommandAllocationSite {
                source: Arc::clone(&origin),
                offset: context.invocation_offset,
            },
            parent: context.variable_read_owner.cloned().map(Arc::new),
        };
        let indices = arguments.collect::<Vec<_>>();
        let parent = CommandAllocationSite {
            source: Arc::clone(&origin),
            offset: context.invocation_offset,
        };
        let derived = Arc::new(SourceOriginId::derived(
            parent.clone(),
            indices.clone(),
            &Arc::from(source),
            kind,
        ));
        if kind == MaterialisedSourceKind::Script {
            let script = ExecutedScriptSource {
                text: tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes())),
                origin: Arc::clone(&derived),
                mapping: ExecutedScriptMapping::Materialised,
            };
            for argument in indices {
                self.record_executed_script(
                    parent.clone(),
                    argument,
                    script.clone(),
                    Some(context.frame),
                    state.source_variables.namespace_identity.as_ref(),
                );
            }
        }
        state.current_source_origin = Some(Arc::clone(&derived));
        let mut outcomes = if kind == MaterialisedSourceKind::Expression {
            self.walk_prepared_expression(
                parent,
                ExecutedScriptSource {
                    text: tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes())),
                    origin: derived,
                    mapping: ExecutedScriptMapping::Materialised,
                },
                state,
                SourceExecutionContext {
                    variable_read_owner: Some(&expression_owner),
                    ..context
                },
            )
        } else {
            self.walk_source(source, 0, state, &context)
        };
        outcomes.restore_source_origin(Some(&origin));
        outcomes.publish(state);
        outcomes
    }

    fn walk_conditional(
        &mut self,
        branches: &[(Option<usize>, usize)],
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::default();
        let mut fallthrough = Some(boxed_source_branch(state));
        for (condition, body) in branches {
            let Some(mut incoming) = fallthrough.take() else {
                break;
            };
            if let Some(condition) = condition {
                let evaluated =
                    self.walk_expression_operand(*condition, operands, &mut incoming, context);
                for (route, state) in &evaluated.abrupt {
                    outcomes.add_abrupt(*route, state);
                }
                let Some(normal) = evaluated.normal else {
                    break;
                };
                incoming = normal;
                let truth = source_operand_truth(*condition, operands, &incoming, context.registry);
                if truth != Some(true) {
                    fallthrough = Some(incoming.clone());
                }
                if truth == Some(false) {
                    continue;
                }
            }
            outcomes.join(&self.walk_body_operand(*body, operands, &mut incoming, &context));
        }
        if let Some(fallthrough) = fallthrough {
            outcomes.join(&SourceOutcomes::normal(&fallthrough));
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_body_operand(
        &mut self,
        index: usize,
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let context = boxed_body_operand_context(index, operands, state, context);
        let Some(script) = retained_script_operand(
            index,
            operands,
            state,
            context.invocation_offset,
            context.config,
        ) else {
            return opaque_source_invocation(state);
        };
        let Some(origin) = &state.current_source_origin else {
            return opaque_source_invocation(state);
        };
        let parent = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: context.invocation_offset,
        };
        if let Some(word) = operands.written_word(index) {
            self.record_executed_word(&parent.source, word.source().span, None, script.clone());
        }
        self.record_executed_script(
            parent,
            index,
            script.clone(),
            Some(context.frame),
            state.source_variables.namespace_identity.as_ref(),
        );
        let previous_origin = state
            .current_source_origin
            .replace(Arc::clone(&script.origin));
        let mut outcomes = self.walk_source_image(&script.text, script.base(), state, &context);
        outcomes.restore_source_origin(previous_origin.as_ref());
        outcomes.publish(state);
        outcomes
    }

    fn walk_concatenated_expression(
        &mut self,
        offset: usize,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(values) = frozen_script_operands(native.invocation.arguments(), offset) else {
            return opaque_source_invocation(state);
        };
        if values.len() == 1 && authored_braced_operand(native, offset, values[0]).is_some() {
            return self.walk_expression_operand(offset, native.script_operands(), state, context);
        }
        self.walk_mapped_expression(offset, &values, native.script_operands(), state, context)
    }

    fn walk_expression_operand(
        &mut self,
        index: usize,
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(value) = operands.arguments.literal_at(index) else {
            return opaque_source_invocation(state);
        };
        let word = operands.written_word(index);
        let Some(word) = word else {
            return self.walk_materialised_source(
                value,
                index..index + 1,
                MaterialisedSourceKind::Expression,
                state,
                context,
            );
        };
        let (text, source, offset) = match word {
            crate::ir::WordExpr::BracedLiteral { text, source } if text == value => {
                (text.as_str(), source, 1)
            }
            crate::ir::WordExpr::Literal { text, source } => {
                let effective = source_effective_words(
                    std::slice::from_ref(word),
                    state.baseline.dialect,
                    Some((&state.source_variables, context.registry)),
                );
                if text != value
                    || effective
                        .first()
                        .and_then(|word| word.as_registry_word().literal())
                        != Some(text.as_str())
                {
                    // Changed decoded text cannot carry original source spans.
                    return self.walk_materialised_source(
                        value,
                        index..index + 1,
                        MaterialisedSourceKind::Expression,
                        state,
                        context,
                    );
                }
                (text.as_str(), source, 0)
            }
            _ => {
                return self.walk_materialised_source(
                    value,
                    index..index + 1,
                    MaterialisedSourceKind::Expression,
                    state,
                    context,
                );
            }
        };
        let owner = state.current_source_origin.as_ref().map(|source| {
            SourceVariableEvaluationOwner::NativeExpression {
                invocation: CommandAllocationSite {
                    source: Arc::clone(source),
                    offset: context.invocation_offset,
                },
                parent: context.variable_read_owner.cloned().map(Arc::new),
            }
        });
        let Some(origin) = state.current_source_origin.clone() else {
            return opaque_source_invocation(state);
        };
        let Some(expression_source) = ExecutedScriptSource::contiguous(
            Arc::clone(&origin),
            text,
            source.span.start() + offset,
        ) else {
            return opaque_source_invocation(state);
        };
        self.walk_prepared_expression(
            CommandAllocationSite {
                source: origin,
                offset: context.invocation_offset,
            },
            expression_source,
            state,
            SourceExecutionContext {
                variable_read_owner: owner.as_ref(),
                ..context
            },
        )
    }

    fn walk_expression(
        &mut self,
        expression: &crate::expr_ast::ExprNode,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use crate::expr_ast::ExprNode;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        if crate::depth_guard::MAX_SOURCE_NEST_DEPTH.exceeded(context.depth) {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let nested = SourceExecutionContext {
            depth: context.depth + 1,
            ..context
        };
        let coercion = matches!(
            expression,
            ExprNode::Unary { .. } | ExprNode::Binary { .. } | ExprNode::Ternary { .. }
        )
        .then(|| {
            expression_preparation::RuntimeExpressionCoercion::capture(expression, state, context)
        })
        .flatten();
        let mut outcomes = match expression {
            ExprNode::Command { text, start, .. } => {
                if let Some(script) = text
                    .strip_prefix('[')
                    .and_then(|text| text.strip_suffix(']'))
                {
                    self.walk_source(
                        script,
                        base.saturating_add(*start).saturating_add(1),
                        state,
                        &nested,
                    )
                } else {
                    state.mark_opaque_binding_mutation();
                    SourceOutcomes::invocation(state, Route::Unknown)
                }
            }
            ExprNode::Binary { op, left, right } => {
                self.walk_binary_expression(*op, left, right, base, state, nested)
            }
            ExprNode::Unary { operand, .. } => self.walk_expression(operand, base, state, nested),
            ExprNode::Var { text, start, .. } => {
                self.walk_expression_variable(expression, text, *start, base, state, nested)
            }

            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => self.walk_ternary_expression(
                condition,
                true_branch,
                false_branch,
                base,
                state,
                nested,
            ),
            ExprNode::Call {
                function,
                args,
                start,
                ..
            } => self.walk_expression_call(
                function,
                args,
                base,
                base.saturating_add(*start),
                state,
                nested,
            ),
            ExprNode::Raw { .. } => {
                state.mark_opaque_binding_mutation();
                SourceOutcomes::invocation(state, Route::Unknown)
            }
            ExprNode::String { text, start, .. } => {
                self.walk_expression_string(text, base.saturating_add(*start), state, nested)
            }
            _ => SourceOutcomes::normal(state),
        };
        if let Some(normal) = &mut outcomes.normal
            && let Some(coercion) = coercion
        {
            coercion.apply(normal);
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_expression_call(
        &mut self,
        function: &str,
        args: &[crate::expr_ast::ExprNode],
        base: u32,
        site: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::normal(state);
        let mut input = source_representation::SourceMathInput {
            arity: args.len(),
            closed: false,
        };
        for argument in args {
            let Some(continuing) = outcomes.normal.take() else {
                break;
            };
            *state = *continuing;
            let evaluated = self.walk_expression(argument, base, state, context);
            if args.len() == 1 {
                input.closed =
                    source_representation::scalar_math_input_is_closed(argument, &evaluated);
            }
            outcomes.join(&evaluated);
        }
        if let Some(continuing) = outcomes.normal.take() {
            *state = *continuing;
            if args.len() != 1 {
                input.closed = args.iter().all(|argument| {
                    literal_object_pool::SourceOrdinaryLiteralObject::capture_expression_literal(
                        argument, state,
                    )
                    .is_some()
                });
            }
            outcomes.join(&self.walk_math_function(function, input, site, state, context));
        }
        outcomes
    }

    fn walk_expression_string(
        &mut self,
        text: &str,
        offset: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let segments =
            crate::segmenter::segment_commands_with_offset_and_config(text, offset, context.config);
        if let Some(segment) = segments.first() {
            let tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(text).with_base(offset, 0, 0),
                context.config,
                segment,
            );
            if let Some(word) = tokens.words().first() {
                self.walk_substitutions(word, text, offset, state, context)
            } else {
                SourceOutcomes::normal(state)
            }
        } else {
            SourceOutcomes::normal(state)
        }
    }

    fn walk_binary_expression(
        &mut self,
        op: crate::expr_ast::BinOp,
        left: &crate::expr_ast::ExprNode,
        right: &crate::expr_ast::ExprNode,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use crate::expr_ast::BinOp;
        let short = match op {
            BinOp::And | BinOp::WordAnd => Some(false),
            BinOp::Or | BinOp::WordOr => Some(true),
            _ => None,
        };
        let condition_coercion =
            short.and_then(|_| expression_preparation::root_result_coercion(left, state, context));
        let mut outcomes = self.walk_expression(left, base, state, context);
        if let Some(continuing) = outcomes.normal.take() {
            *state = *continuing;
            if let Some(coercion) = condition_coercion {
                coercion.apply(state);
            }
            let truth = source_expression_truth(left, state, context.registry);
            if short.is_some() && truth == short {
                outcomes.join(&SourceOutcomes::normal(state));
            } else {
                let incoming = state.clone();
                outcomes.join(&self.walk_expression(right, base, state, context));
                if short.is_some() && truth.is_none() {
                    outcomes.join(&SourceOutcomes::normal(&incoming));
                }
            }
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_ternary_expression(
        &mut self,
        condition: &crate::expr_ast::ExprNode,
        true_branch: &crate::expr_ast::ExprNode,
        false_branch: &crate::expr_ast::ExprNode,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let condition_coercion =
            expression_preparation::root_result_coercion(condition, state, context);
        let mut outcomes = self.walk_expression(condition, base, state, context);
        if let Some(continuing) = outcomes.normal.take() {
            *state = *continuing;
            if let Some(coercion) = condition_coercion {
                coercion.apply(state);
            }
            match source_expression_truth(condition, state, context.registry) {
                Some(true) => {
                    outcomes.join(&self.walk_expression(true_branch, base, state, context));
                }
                Some(false) => {
                    outcomes.join(&self.walk_expression(false_branch, base, state, context));
                }
                None => {
                    let mut alternative = state.clone();
                    outcomes.join(&self.walk_expression(true_branch, base, state, context));
                    outcomes.join(&self.walk_expression(
                        false_branch,
                        base,
                        &mut alternative,
                        context,
                    ));
                }
            }
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_math_function(
        &mut self,
        function: &str,
        input: source_representation::SourceMathInput,
        site: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        use tcl_registry::mathfunc::NativeMathFunctionDispatch;
        let dispatch = state
            .baseline
            .dialect
            .or_else(|| {
                context
                    .registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile)
            })
            .and_then(tcl_registry::mathfunc::native_function_dispatch);
        let Some(dispatch) = dispatch else {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        };
        self.retain_math_object_callback_effects(state, site, input.closed);
        match dispatch {
            NativeMathFunctionDispatch::FixedTable => {
                self.record_fixed_math_diagnostic_presence(function, site, state);
                if let Some(origin) = &state.current_source_origin {
                    let table = (!state.opaque_domain)
                        .then(|| {
                            state
                                .baseline
                                .native_entry
                                .as_ref()
                                .and_then(|entry| entry.math_functions.clone().map(|table| tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite { interpreter: entry.interpreter, table }))
                        })
                        .flatten();
                    let observation = FixedMathObservation {
                        declaration_preview: self.declaration_preview_depth != 0,
                        frame: state.variable_frame.clone(),
                        namespace: context.namespace_identity(),
                        prerequisite: table.clone(),
                    };
                    let observations = self
                        .implicit_fixed_math_invocations
                        .entry((Arc::clone(origin), site, function.to_owned()))
                        .or_default();
                    if !observations.contains(&observation) {
                        observations.push(observation);
                    }
                    if let Some(prerequisite) = table.as_ref()
                        && let tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Present(row) = prerequisite.table.lookup(function)
                        && let Some(identity) = row.registry_identity.as_deref()
                        && let Some(dialect) = state.baseline.dialect
                        && let Some(protocol) = context.registry.fixed_scalar_math_protocol(
                            dialect, identity, row.arity, input.arity,
                        )
                    {
                        return source_representation::scalar_math_result(state, protocol, input.closed);
                    }
                }
                state.mark_opaque_binding_mutation();
                return SourceOutcomes::invocation(state, Route::Unknown);
            }
            NativeMathFunctionDispatch::CommandTable => {}
        }
        self.walk_command_table_math_function(function, input, site, state, context)
    }

    fn walk_command_table_math_function(
        &mut self,
        function: &str,
        input: source_representation::SourceMathInput,
        site: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let absolute = tcl_registry::mathfunc::qualified_name(function);
        let mut effective = vec![
            crate::registry_invocation::EffectiveInvocationWord::Literal(
                absolute.trim_start_matches("::").to_owned(),
            ),
        ];
        effective.extend(std::iter::repeat_n(
            crate::registry_invocation::EffectiveInvocationWord::Dynamic,
            input.arity,
        ));
        let context = SourceExecutionContext {
            invocation_offset: site,
            ..context
        };
        let mut scope = self.prepare_command_observers(&effective, state, &context);
        let Some(continuing) = scope.entry.normal.take() else {
            scope.entry.publish(state);
            return scope.entry;
        };
        *state = *continuing;
        let mut outcomes = std::mem::take(&mut scope.entry);
        let mut input = input;
        if scope.was_observed {
            input.closed = false;
            self.retain_math_object_callback_effects(state, site, false);
        }
        let namespace_key = context.namespace_key.cloned().or_else(|| {
            state
                .baseline
                .native_entry
                .is_none()
                .then(|| SourceNamespaceKey::authored(context.namespace))
        });
        let mut binding = namespace_key
            .as_ref()
            .map_or_else(SourceInvocationBinding::default, |namespace| {
                source_binding(state, absolute.trim_start_matches("::"), namespace)
            });
        binding.entered_execution_observer = if scope.was_observed {
            SourceObserverExecution::MayObserved
        } else {
            SourceObserverExecution::Unobserved
        };
        binding.object_callback_effects = Some(if input.closed {
            object_callbacks::ObjectCallbackEffects::Closed
        } else {
            object_callbacks::ObjectCallbackEffects::Unknown
        });
        if let Some(origin) = &state.current_source_origin {
            let bindings = self
                .implicit_math_invocations
                .entry((Arc::clone(origin), site, function.to_owned()))
                .or_default();
            if !bindings.contains(&binding) {
                bindings.push(binding.clone());
            }
        }
        let Some(target) = binding.proved_target() else {
            state.mark_opaque_binding_mutation();
            let dispatched = SourceOutcomes::invocation(state, Route::Unknown);
            outcomes.join(&self.finish_command_observer_scope(dispatched, &scope, &context));
            outcomes.publish(state);
            return outcomes;
        };
        let dispatched = if target.registry_backed {
            Self::walk_registry_math_function(target, input, state, context)
        } else {
            self.walk_document_target(site, None, &effective, state, target, context)
        };
        outcomes.join(&self.finish_command_observer_scope(dispatched, &scope, &context));
        outcomes.publish(state);
        outcomes
    }

    fn walk_registry_math_function(
        target: &SourceCommandTarget,
        input: source_representation::SourceMathInput,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let mut arguments = target
            .prepended
            .iter()
            .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        arguments.extend(std::iter::repeat_n(
            tcl_registry::InvocationWord::DynamicNonOption,
            input.arity,
        ));
        let mut invocation = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&target.command),
            &arguments,
        );
        if let Some(dialect) = state.baseline.dialect {
            invocation = invocation.with_dialect(dialect);
        }
        let route = context
            .registry
            .invocation_completion_route_in_frame(
                &target.command,
                invocation.arguments(),
                invocation
                    .arguments()
                    .dialect()
                    .and_then(tcl_registry::InvocationDialect::authoring_query)
                    .map(|query| query.with_realm(context.realm)),
                state.source_variables.alias_frame(),
            )
            .unwrap_or(Route::Unknown);
        if let Some(facts) = tcl_registry::model::semantic::resolve_structured_invocation_in_realm(
            context.registry,
            context
                .registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile),
            invocation,
            context.realm,
        )
        .resolved()
        .map(|resolved| resolved.facts())
            && facts.effects.callback().kinds == tcl_registry::CallbackKinds::NONE
            && facts.effects.accesses().iter().all(|access| {
                access.mode == tcl_registry::EffectAccessMode::Read
                    || !matches!(
                        access.domain,
                        tcl_registry::WorldStateDomain::CommandBindings
                            | tcl_registry::WorldStateDomain::NamespaceLookup
                            | tcl_registry::WorldStateDomain::VariableStore
                    )
            })
        {
            if let Some(protocol) = facts.normal_scalar_math_protocol(invocation.arguments()) {
                return source_representation::scalar_math_result(state, protocol, input.closed);
            }
            if facts
                .numeric_object_conversion_arguments(invocation.arguments())
                .is_some()
                && !input.closed
            {
                // The selected implementation remains current for this call,
                // but its original operands can mutate interpreter worlds
                // through updateStringProc/freeIntRepProc during conversion.
                state.mark_opaque_binding_mutation();
            }
            return SourceOutcomes::invocation(state, route);
        }
        state.mark_opaque_binding_mutation();
        SourceOutcomes::invocation(state, route)
    }
}

fn parse_source_expression(
    source: &str,
    state: &ModuleCommandBindings,
    registry: &CommandRegistry,
) -> crate::expr_ast::ExprNode {
    if let Some(dialect) = state.baseline.dialect {
        crate::expr_parser::parse_expr_with_syntax_context(
            source,
            &dialect.expression_parse_context(registry.profile()),
        )
    } else {
        crate::expr_parser::parse_expr_for_profile(source, registry.profile())
    }
}

fn source_operand_truth(
    index: usize,
    operands: SourceScriptOperands<'_>,
    state: &ModuleCommandBindings,
    registry: &CommandRegistry,
) -> Option<bool> {
    let value = operands.arguments.literal_at(index)?;
    let expression = parse_source_expression(value, state, registry);
    source_expression_truth(&expression, state, registry)
}

fn source_expression_truth(
    expression: &crate::expr_ast::ExprNode,
    state: &ModuleCommandBindings,
    registry: &CommandRegistry,
) -> Option<bool> {
    use crate::expr_ast::ExprNode;
    let mut env = crate::tcl_expr_eval::Env::new();
    let mut pending = vec![expression];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } => {}
            ExprNode::Var { text, name, .. } => {
                let value = state
                    .source_variables
                    .substitution_literal_value(text, registry)?;
                env.insert(
                    name.clone(),
                    crate::tcl_expr_eval::EnvValue::Str(value.to_owned()),
                );
            }
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => pending.extend([
                condition.as_ref(),
                true_branch.as_ref(),
                false_branch.as_ref(),
            ]),
            // Calls can name replaced math commands; folding their nominal
            // builtin implementation would lose the source binding proof.
            _ => return None,
        }
    }
    let mut policy = crate::tcl_expr_eval::FoldPolicy::from_registry(registry);
    if let Some(dialect) = state.baseline.dialect {
        policy = policy.with_invocation_dialect(dialect);
    }
    crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
        expression,
        &env,
        policy,
        &|_, _| None,
        None,
    )
    .and_then(|evaluation| match evaluation.value {
        crate::tcl_expr_eval::TclValue::Float(value) if value.is_nan() => None,
        value => Some(value.is_truthy()),
    })
}

fn bind_called_body_parameters(
    called: &mut crate::var_resolve::ResolveContext,
    body: &DeferredSourceBody,
    actual: &[Option<&str>],
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) -> Option<bool> {
    if let Some(topology) = &body.original_parameters {
        if called
            .invocation_dialect?
            .parameter_grammar()?
            .skips_empty_body_activation()
            && body.source.is_empty()
        {
            return Some(true);
        }
        return topology.bind(called, actual, context.registry);
    }
    bind_source_parameters(
        called,
        &body.parameters,
        actual,
        body.source.is_empty(),
        context.registry,
        native_result::numeric_call_arguments(context, target),
    )
}

fn bind_source_parameters(
    context: &mut crate::var_resolve::ResolveContext,
    parameters: &[tcl_syntax::formal_params::FormalParameter],
    actual: &[Option<&str>],
    body_is_empty: bool,
    registry: &CommandRegistry,
    numeric_values: Option<&[Option<Arc<native_result::EvaluatedSourceValue>>]>,
) -> Option<bool> {
    use tcl_syntax::formal_params::FormalArgumentBinding as Binding;
    let grammar = context.invocation_dialect?.parameter_grammar()?;
    let Ok(bindings) =
        tcl_syntax::formal_params::bind_formal_arguments(parameters, actual.len(), grammar)
    else {
        return Some(false);
    };
    if grammar.skips_empty_body_activation() && body_is_empty {
        return Some(true);
    }
    for binding in bindings {
        match binding {
            Binding::Value {
                parameter,
                argument,
            } => {
                let name = &parameters[parameter].name;
                if let Some(value) = actual[argument] {
                    context.bind_literal_incoming(name, value, registry);
                    if let Some(receipt) = numeric_values
                        .and_then(|values| values.get(argument))
                        .and_then(Option::as_deref)
                        .filter(|receipt| receipt.text == value)
                        .and_then(|receipt| receipt.numeric.as_ref())
                    {
                        context.retain_incoming_native_numeric(name, receipt, registry);
                    }
                } else {
                    context.bind_unknown_incoming(name, registry);
                }
            }
            Binding::Default { parameter } => {
                context.bind_literal_incoming(
                    &parameters[parameter].name,
                    parameters[parameter].default.as_deref()?,
                    registry,
                );
            }
            Binding::Rest {
                name, start, len, ..
            } => {
                if let Some(values) = actual
                    .get(start..start + len)
                    .and_then(|values| values.iter().copied().collect::<Option<Vec<_>>>())
                {
                    context.bind_literal_incoming(
                        &name,
                        &tcl_syntax::list::join_list(values),
                        registry,
                    );
                } else {
                    context.bind_unknown_incoming(&name, registry);
                }
            }
            Binding::CallerLink { name, argument, .. } => {
                match crate::variable_bindings::bind_caller_reference(
                    context,
                    &name,
                    actual[argument],
                    registry,
                ) {
                    crate::variable_bindings::CallerReferenceBinding::Bound => {}
                    crate::variable_bindings::CallerReferenceBinding::Missing => {
                        return Some(false);
                    }
                    crate::variable_bindings::CallerReferenceBinding::Unknown => return None,
                }
            }
        }
    }
    Some(true)
}

fn join_evaluated_words(values: &mut Vec<Option<String>>, other: &[Option<String>]) {
    if values.len() != other.len() {
        *values = vec![None; values.len().max(other.len())];
        return;
    }
    for (value, other) in values.iter_mut().zip(other) {
        if value != other {
            *value = None;
        }
    }
}

fn source_effective_words(
    words: &[crate::ir::WordExpr],
    dialect: Option<tcl_registry::InvocationDialect>,
    context: Option<(&crate::var_resolve::ResolveContext, &CommandRegistry)>,
) -> Vec<crate::registry_invocation::EffectiveInvocationWord> {
    use crate::registry_invocation::EffectiveInvocationWord;
    words
        .iter()
        .map(|word| {
            if let (crate::ir::WordExpr::Variable { spelling, .. }, Some((variables, registry))) =
                (word, context)
                && let Some(value) = variables.substitution_literal_value(spelling, registry)
            {
                return EffectiveInvocationWord::Literal(value.to_owned());
            }
            dialect.map_or_else(
                || match crate::registry_invocation::invocation_word(word) {
                    tcl_registry::InvocationWord::Literal(value) => {
                        EffectiveInvocationWord::Literal(value.to_owned())
                    }
                    tcl_registry::InvocationWord::KnownBytes(value) => {
                        EffectiveInvocationWord::from_bytes(value)
                    }
                    tcl_registry::InvocationWord::Dynamic
                    | tcl_registry::InvocationWord::DynamicNonOption => {
                        EffectiveInvocationWord::Dynamic
                    }
                    tcl_registry::InvocationWord::Expanded => EffectiveInvocationWord::Expanded,
                    tcl_registry::InvocationWord::Opaque => EffectiveInvocationWord::Opaque,
                    tcl_registry::InvocationWord::ArrayElementName { root } => {
                        EffectiveInvocationWord::ArrayElementName {
                            root: root.to_owned(),
                        }
                    }
                },
                |dialect| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    )
                },
            )
        })
        .collect()
}

/// Materialise the complete dispatch carrier before a recursive target walk.
#[inline(never)]
/// Lookup the authoritative command table from a current original producer.
/// Unicode metadata is presentation; it never reconstructs the selected slot.
fn source_binding_from_original_input(
    state: &ModuleCommandBindings,
    head: &crate::signature_scan::scope::SignatureSourceNameInput,
    namespace: &SourceNamespaceKey,
) -> Option<SourceInvocationBinding> {
    source_binding_from_original_input_in(state, head, namespace, CommandTargetLookup::WithFallback)
}

/// Keep the complete original binding outside frames retained by nested bodies.
#[inline(never)]
fn source_binding_from_original_input_boxed(
    state: &ModuleCommandBindings,
    head: &crate::signature_scan::scope::SignatureSourceNameInput,
    namespace: &SourceNamespaceKey,
) -> Option<Box<SourceInvocationBinding>> {
    source_binding_from_original_input(state, head, namespace).map(Box::new)
}

fn source_binding_from_original_input_in(
    state: &ModuleCommandBindings,
    head: &crate::signature_scan::scope::SignatureSourceNameInput,
    namespace: &SourceNamespaceKey,
    lookup: CommandTargetLookup,
) -> Option<SourceInvocationBinding> {
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
        eprintln!(
            "ORIGINAL_COMMAND_BINDING input={:?} current={} namespace={:?} origin={} variable_policy={:?} baseline_policy={:?} opaque={}",
            head.bytes(),
            head.is_current(&state.source_variables),
            namespace,
            state.current_source_origin.is_some(),
            state.source_variables.execution_name_policy,
            state.baseline.execution_name_policy,
            state.has_opaque_domain()
        );
    }
    head.is_current(&state.source_variables).then_some(())?;
    let (declared_command, catalogue_command) =
        state.original_advice_candidates_for_input(namespace, head);
    let selected = state
        .original_targets_for_input(head, namespace, lookup)
        .or_else(|| {
            (declared_command.is_some() || catalogue_command.is_some()).then(|| {
                original_command_table::OriginalCommandTargetSelection {
                    targets: BTreeSet::new(),
                    original_prefixes: BTreeMap::new(),
                    may_be_absent: true,
                    unknown: true,
                }
            })
        })?;
    Some(SourceInvocationBinding {
        declared_command,
        catalogue_command,
        lookup_state: Some(Arc::new(SourceLookupSnapshot::new(state.clone()))),
        lookup_namespace: namespace.display().unwrap_or_default(),
        lookup_namespace_key: namespace.clone(),
        lookup_word: std::str::from_utf8(head.bytes()).ok().map(str::to_owned),
        targets: selected
            .targets
            .into_iter()
            .map(|target| {
                let original_prepended = (!target.prepended.is_empty())
                    .then(|| selected.original_prefixes.get(&target).cloned().flatten())
                    .flatten();
                SourceCommandTarget {
                    runtime_implementation_generation: state
                        .runtime_implementation_generation(target.token.as_ref()),
                    command: target.command,
                    prepended: target.prepended,
                    original_prepended,
                    registry_backed: target.registry_backed,
                    kind: target.kind,
                    identity: target.token,
                    implementation_generation: target.implementation_generation,
                    implementation_allocation: target.implementation_allocation,
                }
            })
            .collect(),
        may_be_absent: selected.may_be_absent,
        unknown: selected.unknown,
        provider_state_taint: state.tainted_provider_state.iter().cloned().collect(),
        variable_context: Arc::clone(&state.source_variables),
        variable_frame: state.variable_frame.clone(),
        namespace_cell_presence: state.source_variables.namespace_cells.clone(),
        known_namespaces: state
            .namespaces
            .iter()
            .filter_map(SourceNamespaceKey::advisory_key)
            .collect(),
        ..SourceInvocationBinding::default()
    })
}

fn source_binding(
    state: &ModuleCommandBindings,
    head: &str,
    namespace: &(impl NamespaceKeyQuery + ?Sized),
) -> SourceInvocationBinding {
    let mut projection = source_binding_projection(state, head, namespace);
    projection.lookup_state = Some(Arc::new(SourceLookupSnapshot::new(state.clone())));
    projection
}

#[derive(Clone, Copy)]
enum CommandTargetLookup {
    WithFallback,
    NamedSlots,
}

fn source_binding_projection(
    state: &ModuleCommandBindings,
    head: &str,
    namespace: &(impl NamespaceKeyQuery + ?Sized),
) -> SourceInvocationBinding {
    source_binding_projection_in(state, head, namespace, CommandTargetLookup::WithFallback)
}

fn source_binding_projection_in(
    state: &ModuleCommandBindings,
    head: &str,
    namespace: &(impl NamespaceKeyQuery + ?Sized),
    lookup: CommandTargetLookup,
) -> SourceInvocationBinding {
    let namespace_key = namespace.namespace_key();
    let namespace_display = namespace_key.display().unwrap_or_default();
    let (declared_command, catalogue_command) =
        state.original_advice_candidates_for_metadata(namespace_key.as_ref(), head);
    let keys = state.source_keys(head, namespace);
    let may_be_absent = keys.last().is_none_or(|key| {
        state
            .binding_alternatives(key)
            .contains(&MayBinding::Missing)
    });
    SourceInvocationBinding {
        compiled_candidates: Vec::new(),
        compiled_named_candidates: Vec::new(),
        may_use_live_dispatch: true,
        compiled_execution_residual: CompiledExecutionResidual::Closed,
        native_compilation_admission: None,
        native_compiler_admission: None,
        original_compiler_preserves_names: SourceCompilerNameClosure::Unknown,
        native_structured_preparation: None,
        native_switch_preparation: None,
        native_operand_layout: None,
        runtime_reachability: SourceRuntimeReachability::Unknown,
        entered_execution_observer: SourceObserverExecution::Unobserved,
        native_inline_rejection: NativeInlineRejection::None,
        native_handler_envelope: None,
        invocation_variable_reads: None,
        lookup_state: None,
        compiler_lookup_state: None,
        declaration_layout_observations: None,
        declaration_flow_inventory: None,
        compiler_policy: None,
        original_compiler_words: None,
        original_written_words: None,
        conditional_expression_evaluations: None,
        lookup_namespace: namespace_display.clone(),
        lookup_namespace_key: namespace_key.into_owned(),
        lookup_word: Some(head.to_owned()),
        dispatch_site: None,
        provider_state_taint: state.tainted_provider_state.iter().cloned().collect(),
        declared_command,
        catalogue_command,
        variable_context: Arc::clone(&state.source_variables),
        original_argument_completion: OriginalArgumentCompletion::Unknown,
        normal_variable_continuation: None,
        original_variable_operands: None,
        normal_result_observations: None,
        rhs_read_store_observations: None,
        object_callback_effects: None,
        variable_frame: state.variable_frame.clone(),
        existing_namespace_cells: {
            let mut cells = state
                .source_variables
                .namespace_cells
                .present
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            cells.sort_by_cached_key(|key| format!("{key:?}"));
            cells
        },
        namespace_cell_presence: state.source_variables.namespace_cells.clone(),
        known_namespaces: state
            .namespaces
            .iter()
            .filter_map(SourceNamespaceKey::advisory_key)
            .collect(),
        evaluated_argument_values: Vec::new(),
        evaluated_argument_words: Vec::new(),
        frozen_written_words: None,
        frozen_written_names: None,
        frozen_head_object: None,
        method_prefix_arguments: Vec::new(),
        targets: state
            .resolve_targets_in(head, namespace, &mut BTreeSet::new(), lookup)
            .into_iter()
            .map(|target| SourceCommandTarget {
                runtime_implementation_generation: state
                    .runtime_implementation_generation(target.token.as_ref()),
                command: target.command,
                prepended: target.prepended,
                original_prepended: None,
                registry_backed: target.registry_backed,
                kind: target.kind,
                identity: target.token,
                implementation_generation: target.implementation_generation,
                implementation_allocation: target.implementation_allocation,
            })
            .collect(),
        may_be_absent,
        unknown: state.target_may_be_unknown(head, namespace)
            || (!head.starts_with("::")
                && namespace.namespace_key().native_context().is_none()
                && !state.source_variables.namespace_known
                && state.receiver_command_lookup_path(head).is_none()),
    }
}

fn command_holder(command: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(command);
    if holder.is_empty() {
        "::".to_owned()
    } else {
        holder.to_owned()
    }
}

impl Hash for BindingBaseline {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fingerprint.hash(state);
    }
}

/// Historical namespace-lookup effects discovered through the closed command
/// lattice. Keeping this beside the binding state means alias-prefixed and
/// recovered invocations publish the same resolution facts as direct calls.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct NamespaceResolutionProjection {
    changed: bool,
    dynamic: bool,
    rebound_names: BTreeSet<String>,
    opaque_namespaces: BTreeSet<String>,
}

/// Compact identity for every mutable observation axis other than the live
/// binding map. All set-valued axes below are monotone during the binding
/// walk, so their cardinality changes exactly when their semantic value grows.
/// The immutable baseline and the once-published root-boundary map are omitted:
/// neither can change during one invocation transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // Mirrors independent monotone observation axes.
struct NonBindingObservationStamp {
    original_command_world: Arc<source_command_world::OriginalSourceCommandWorld>,
    ordinary_literal_pool: Option<literal_object_pool::SourceOrdinaryLiteralPool>,
    class_definitions: Arc<BTreeMap<CommandIdentity, ClassDefinitionReceipt>>,
    object_instances: Arc<object_instance::SourceObjectState>,
    bounded_constructions: Arc<BTreeMap<CommandIdentity, BoundedClassConstruction>>,
    tainted_object_dispatch: Arc<BTreeSet<String>>,
    tainted_provider_state: Arc<BTreeSet<String>>,
    command_observers: Arc<command_observers::SourceCommandObservers>,
    source_variables: Arc<crate::var_resolve::ResolveContext>,
    variable_frame: crate::var_resolve::VariableExecutionFrame,
    objects: Arc<BTreeMap<CommandToken, BTreeSet<MayBinding>>>,
    namespace_exports: Arc<SourceNamespaceMap<BTreeSet<Vec<tcl_core_types::NameBytes>>>>,
    namespace_paths: Arc<SourceNamespaceMap<BTreeSet<Vec<SourceNamespaceKey>>>>,
    namespaces: Arc<SourceNamespaceSet>,
    unknown_lookup_namespaces: Arc<SourceNamespaceSet>,
    unknown_namespace_paths: Arc<SourceNamespaceSet>,
    namespace_unknown_handlers: Arc<SourceNamespaceSet>,
    unknown_export_namespaces: Arc<SourceNamespaceSet>,
    packages: Arc<BTreeMap<String, BTreeSet<PackageProvision>>>,
    revoked_loaders: Arc<BTreeSet<String>>,
    loader_handler_unknown: bool,
    opaque_domain: bool,
    opaque_binding_mutation: bool,
    dynamic_proc_binding: bool,
    unnameable_rebinding_subject: bool,
    namespace_changed: bool,
    namespace_dynamic: bool,
    namespace_rebound_count: usize,
    namespace_opaque_count: usize,
    procedure_body_count: usize,
    rebound_count: usize,
    proc_rebound_count: usize,
    has_redefined_procedures: bool,
}

impl NamespaceResolutionProjection {
    fn record(
        &mut self,
        transition: &NamespaceTransition,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) {
        let site = match namespace {
            crate::ir_helpers::ExecutionNamespace::Exact(site) => Some(site.as_str()),
            crate::ir_helpers::ExecutionNamespace::SourceContext(_)
            | crate::ir_helpers::ExecutionNamespace::RuntimeSelected => None,
        };
        let mut rebound = std::collections::HashSet::new();
        let mut opaque = std::collections::HashSet::new();
        let mut changed = false;
        if !collect_namespace_resolution(transition, site, &mut rebound, &mut changed, &mut opaque)
        {
            self.dynamic = true;
        }
        self.changed |= changed;
        self.rebound_names.extend(rebound);
        self.opaque_namespaces.extend(opaque);
    }

    fn join(&mut self, other: &Self) -> bool {
        let mut joined = false;
        if other.changed && !self.changed {
            self.changed = true;
            joined = true;
        }
        if other.dynamic && !self.dynamic {
            self.dynamic = true;
            joined = true;
        }
        let rebound_count = self.rebound_names.len();
        self.rebound_names
            .extend(other.rebound_names.iter().cloned());
        joined |= self.rebound_names.len() != rebound_count;
        let opaque_count = self.opaque_namespaces.len();
        self.opaque_namespaces
            .extend(other.opaque_namespaces.iter().cloned());
        joined |= self.opaque_namespaces.len() != opaque_count;
        joined
    }
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
    source_variables: Arc<crate::var_resolve::ResolveContext>,
    variable_frame: crate::var_resolve::VariableExecutionFrame,
    /// Sparse flow-sensitive binding state. Analysis forks this state for
    /// every executable root, while most roots leave the command table
    /// unchanged. Copy-on-write keeps those forks allocation-cheap and
    /// detaches only at the centralised replacement/join seams below.
    bindings: Arc<SourceCommandTable<BTreeSet<MayBinding>>>,
    original_entry_bindings: Arc<SourceCommandTable<BTreeSet<MayBinding>>>,
    /// Implementations of command objects followed by namespace imports.
    objects: Arc<BTreeMap<CommandToken, BTreeSet<MayBinding>>>,
    /// Exact export/path alternatives are joined alongside command bindings.
    namespace_exports: Arc<SourceNamespaceMap<BTreeSet<Vec<tcl_core_types::NameBytes>>>>,
    namespace_paths: Arc<SourceNamespaceMap<BTreeSet<Vec<SourceNamespaceKey>>>>,
    namespaces: Arc<SourceNamespaceSet>,
    unknown_lookup_namespaces: Arc<SourceNamespaceSet>,
    unknown_namespace_paths: Arc<SourceNamespaceSet>,
    namespace_unknown_handlers: Arc<SourceNamespaceSet>,
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
    rebound_names: BTreeSet<SourceCommandKey>,
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

impl PartialEq for ModuleCommandBindings {
    fn eq(&self, other: &Self) -> bool {
        // This public state participates in semantic interning. Two
        // independent analyses of the same registry must therefore compare
        // equal even though each owns a distinct baseline allocation.
        self.baseline == other.baseline
            && self.original_command_world == other.original_command_world
            && self.current_source_origin == other.current_source_origin
            && self.ordinary_literal_pool == other.ordinary_literal_pool
            && self.allocation_counts == other.allocation_counts
            && self.namespace_allocation_counts == other.namespace_allocation_counts
            && self.class_definitions == other.class_definitions
            && self.object_instances == other.object_instances
            && self.bounded_constructions == other.bounded_constructions
            && self.tainted_object_dispatch == other.tainted_object_dispatch
            && self.tainted_provider_state == other.tainted_provider_state
            && self.command_observers == other.command_observers
            && self.source_variables == other.source_variables
            && self.variable_frame == other.variable_frame
            && self.bindings == other.bindings
            && self.objects == other.objects
            && self.namespace_exports == other.namespace_exports
            && self.namespace_paths == other.namespace_paths
            && self.namespaces == other.namespaces
            && self.unknown_lookup_namespaces == other.unknown_lookup_namespaces
            && self.unknown_namespace_paths == other.unknown_namespace_paths
            && self.namespace_unknown_handlers == other.namespace_unknown_handlers
            && self.unknown_export_namespaces == other.unknown_export_namespaces
            && self.packages == other.packages
            && self.revoked_loaders == other.revoked_loaders
            && self.loader_handler_unknown == other.loader_handler_unknown
            && self.root_boundary_bindings == other.root_boundary_bindings
            && self.original_entry_bindings == other.original_entry_bindings
            && self.opaque_domain == other.opaque_domain
            && self.opaque_binding_mutation == other.opaque_binding_mutation
            && self.dynamic_proc_binding == other.dynamic_proc_binding
            && self.unnameable_rebinding_subject == other.unnameable_rebinding_subject
            && self.namespace_resolution == other.namespace_resolution
            && self.procedure_bodies == other.procedure_bodies
            && self.rebound_names == other.rebound_names
            && self.proc_rebound_names == other.proc_rebound_names
            && self.has_redefined_procedures == other.has_redefined_procedures
            && self.source_order_user_call_effects == other.source_order_user_call_effects
    }
}

impl Eq for ModuleCommandBindings {}

impl ModuleCommandBindings {
    /// Availability phase retained by the source-entry owner. Reading this
    /// phase grants no entered body, native handler or compiler admission.
    pub(crate) fn invocation_realm(&self) -> tcl_dialect::model::InvocationRealm {
        self.baseline.invocation_realm
    }

    /// Registry head and prepended arguments for the shared expression-word
    /// descent. CFG projection and binding replay use the same alias answer.
    pub(crate) fn resolved_embedded_head(
        &self,
        head: &str,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) -> Option<crate::ir_helpers::ResolvedEmbeddedHead> {
        let namespace = namespace.for_head_context(head)?;
        if self.target_resolution_may_be_unknown(head, namespace.as_ref()) {
            return None;
        }
        let mut found = self.targets(head, namespace.as_ref()).into_iter();
        let target = found.next()?;
        if found.next().is_some() || !target.registry_backed {
            return None;
        }
        Some(crate::ir_helpers::ResolvedEmbeddedHead {
            command: target.command,
            prepended: target
                .prepended
                .iter()
                .map(|word| word.as_registry_word().literal().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?,
        })
    }

    /// Closed root-boundary effects do not include pre-definition missing
    /// candidates from historical observations. A completed retained call can
    /// publish those effects without inventing arbitrary binding mutation.
    fn source_order_call_boundary(&self) -> Arc<Self> {
        let mut effects = self.clone();
        effects.bindings.clone_from(&self.root_boundary_bindings);
        effects.source_order_user_call_effects = None;
        Arc::new(effects)
    }

    /// Apply the closed module effects after a retained user call.
    pub(crate) fn mark_source_order_user_procedure_call(&mut self) {
        if let Some(effects) = self.source_order_user_call_effects.clone() {
            self.join(&effects);
        } else {
            self.mark_opaque_binding_mutation();
        }
    }

    /// Advance this source-order state past one recovered command, in Tcl's
    /// evaluation order: the registry binding transitions the command makes
    /// (`rename`, `interp alias`, an opaque readable-eval body), and the closed
    /// module effects of a retained user procedure it calls. A computed head,
    /// or one whose namespace is not known, may change any binding. A command
    /// expression control may skip (`conditional`) joins the state it leaves
    /// with the one before it.
    pub(crate) fn advance_source_order_for_command(
        &mut self,
        words: &[crate::ir_helpers::CommandWord],
        conditional: bool,
        registry: &CommandRegistry,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) {
        let owner = self.baseline.metadata_context.clone();
        self.source_order_registry_barrier_for_command_with_metadata_context(
            words,
            conditional,
            registry,
            namespace,
            tcl_registry::Traits::empty(),
            owner.metadata_context(registry).flatten(),
        );
    }
    pub(crate) fn source_order_registry_barrier_for_command(
        &mut self,
        words: &[crate::ir_helpers::CommandWord],
        conditional: bool,
        registry: &CommandRegistry,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        barrier_traits: tcl_registry::Traits,
    ) -> bool {
        let owner = self.baseline.metadata_context.clone();
        self.source_order_registry_barrier_for_command_with_metadata_context(
            words,
            conditional,
            registry,
            namespace,
            barrier_traits,
            owner.metadata_context(registry).flatten(),
        )
    }

    /// Advance conditional source effects using the independently retained
    /// availability. Missing or foreign metadata widens instead of borrowing
    /// catalogue effects; this supplies no physical command execution.
    pub(crate) fn source_order_registry_barrier_for_command_with_metadata_context(
        &mut self,
        words: &[crate::ir_helpers::CommandWord],
        conditional: bool,
        registry: &CommandRegistry,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        barrier_traits: tcl_registry::Traits,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> bool {
        if self.source_order_user_call_effects.is_none() {
            self.source_order_user_call_effects = Some(self.source_order_call_boundary());
        }
        let Some(head) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            self.mark_opaque_binding_mutation();
            return true;
        };
        let Some(command_namespace) = namespace.for_head_context(head) else {
            self.mark_opaque_binding_mutation();
            return true;
        };
        let skipped = conditional.then(|| self.clone());
        let source_may_be_unknown = self.target_may_be_unknown(head, command_namespace.as_ref());
        let reaches_user_procedure = self
            .targets(head, command_namespace.as_ref())
            .iter()
            .any(|target| !target.registry_backed);
        let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
            self.mark_opaque_binding_mutation();
            return true;
        };
        let facts = self.resolve_command_words_with_metadata_context(
            words,
            registry,
            Some(metadata),
            command_namespace.as_ref(),
        );
        let barrier = source_may_be_unknown
            || facts
                .iter()
                .any(|facts| facts.traits.intersects(barrier_traits));
        apply_resolved_may_transitions(facts, source_may_be_unknown, true, self, namespace);
        if reaches_user_procedure {
            self.mark_source_order_user_procedure_call();
        }
        if let Some(skipped) = skipped {
            self.join(&skipped);
        }
        barrier
    }

    fn default_construction_dependencies_hold(
        &self,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
    ) -> bool {
        let Some(root) = self.source_root_namespace_key() else {
            return false;
        };
        let Some(policy) = self
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        else {
            return false;
        };
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_OO_DEPENDENCIES root_known={} geometry_policy={} taint_all={} opaque={} step={} execution={}",
                self.namespaces.contains(&root),
                (*self.original_command_world)
                    .clone()
                    .select_policy(self)
                    .is_some(),
                self.tainted_object_dispatch.contains("*"),
                self.has_opaque_domain(),
                self.source_step_observed(),
                self.source_execution_observed(None)
            );
        }
        !self.tainted_object_dispatch.contains("*")
            && grammar
                .default_construction_lookup_dependencies()
                .is_some_and(|dependencies| {
                    dependencies.iter().all(|command| {
                        let selected = self.original_registry_metadata_target(&root, command, policy);
                        #[cfg(test)]
                        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
                            eprintln!("ORIGINAL_OO_DEPENDENCY command={command} tainted={} paths={} selected={} generation={:?}",
                                self.tainted_object_dispatch.contains(*command), self.original_registry_command_paths(&root, command, policy).is_some(),
                                selected.is_some(), selected.as_ref().map(|target| target.implementation_generation));
                        }
                        !self.tainted_object_dispatch.contains(*command)
                            && selected.is_some_and(|target| {
                                    target.registry_backed
                                        && target.prepended.is_empty()
                                        && nqn(&target.command) == nqn(command)
                                        && target.implementation_generation == 0
                                })
                    })
                })
    }

    fn record_bounded_class(
        &mut self,
        facts: &tcl_registry::InvocationFacts,
        words: &[crate::ir::WordExpr],
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        let Some(dialect) = self.baseline.dialect else {
            return false;
        };
        let Some(grammar) = context.registry.native_default_construction_grammar(
            &target.command,
            dialect,
            context.realm,
        ) else {
            return false;
        };
        if !self.default_construction_dependencies_hold(grammar) {
            return false;
        }
        let Some(name) = facts.state_transitions.declared().and_then(|transitions| {
            transitions
                .facts()
                .iter()
                .find_map(|fact| match &fact.transition {
                    StateTransition::ObjectDispatch(
                        tcl_registry::ObjectDispatchTransition::Create {
                            target: tcl_registry::ObjectDispatchTarget::Named(subject),
                            kind: tcl_registry::ObjectDispatchKind::Class,
                            ..
                        },
                    ) => subject.literal(),
                    _ => None,
                })
        }) else {
            return false;
        };
        let source = facts.arg_roles.iter().find_map(|(index, role)| {
            (*role == tcl_registry::ArgRole::Body)
                .then_some(facts.argument_offset + usize::from(*index))
        });
        if let Some(index) = source {
            let Some(index) = index.checked_sub(target.prepended.len()) else {
                return false;
            };
            if let Some(word) = words.get(index + 1) {
                let effective = source_effective_words(
                    std::slice::from_ref(word),
                    self.baseline.dialect,
                    Some((&self.source_variables, context.registry)),
                );
                let Some(body) = effective[0].as_registry_word().literal() else {
                    return false;
                };
                if !self.definition_has_only_deferred_members(body, grammar, context) {
                    return false;
                }
            }
        }
        let proof = source_binding(self, name, &context.namespace_identity());
        let Some(created) = proof.proved_target() else {
            return false;
        };
        let Some(identity) = &created.identity else {
            return false;
        };
        Arc::make_mut(&mut self.bounded_constructions).insert(
            identity.clone(),
            BoundedClassConstruction {
                factory: target.command.clone(),
                implementation_generation: created.implementation_generation,
            },
        );
        true
    }

    fn definition_has_only_deferred_members(
        &self,
        body: &str,
        grammar: &tcl_registry::definer::DefinitionBodyGrammar,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        let map = tcl_lexer::SourceMap::new(body);
        crate::segmenter::segment_commands_with_offset_and_config(body, 0, context.config)
            .into_iter()
            .all(|segment| {
                let words =
                    crate::ir::CommandTokens::from_segmented(&map, context.config, &segment);
                let effective = source_effective_words(
                    words.words(),
                    self.baseline.dialect,
                    Some((&self.source_variables, context.registry)),
                );
                let Some(head) = effective
                    .first()
                    .and_then(|word| word.as_registry_word().literal())
                else {
                    return false;
                };
                let Some(member) = grammar.member(head) else {
                    return false;
                };
                if member.visibility_effect.is_some()
                    && grammar.preserves_local_constructor_entry(member)
                    && grammar.manufacture_member_effect(member)
                        == tcl_registry::definer::MemberManufactureDispatchEffect::PreservesNativeFactory
                {
                    return deferred_method::original_definition_member(
                        grammar,
                        member,
                        tcl_registry::definer::DefinitionReceiver::Instance,
                        self,
                        context,
                    ) && effective
                        .iter()
                        .all(|word| word.as_registry_word().literal().is_some());
                }
                let query = self.baseline.dialect
                    .and_then(tcl_registry::InvocationDialect::authoring_query)
                    .map(|query| query.with_realm(context.realm));
                let delegate_preserves_manufacturers = member.arg_roles.iter()
                    .find_map(|(index, role)| (*role == tcl_registry::ArgRole::Name)
                        .then_some(usize::from(*index) + 1))
                    .and_then(|index| effective.get(index))
                    .and_then(|word| word.as_registry_word().literal())
                    .is_some_and(|name| grammar.native_classmethod_preserves_default_manufacturers(member, name, query));
                if grammar.construction_member_effect(member)
                    != tcl_registry::definer::MemberConstructionEffect::DeferredInstanceMethod
                    && !delegate_preserves_manufacturers
                {
                    return self
                        .closed_inherited_member(grammar, member, &effective, context)
                        .is_some();
                }
                let Some(lookup) =
                    grammar.definition_member_lookup(member, query)
                else {
                    return false;
                };
                if !source_binding(self, head, lookup.namespace)
                    .proved_target()
                    .is_some_and(|target| {
                        target.registry_backed
                            && target.prepended.is_empty()
                            && target.command == lookup.implementation
                            && target.implementation_generation == 0
                    })
                {
                    return false;
                }
                let expected = member
                    .arg_roles
                    .iter()
                    .map(|(index, _)| usize::from(*index) + 2)
                    .max()
                    .unwrap_or(1);
                effective.len() == expected
                    && effective
                        .iter()
                        .all(|word| word.as_registry_word().literal().is_some())
                    && member.arg_roles.iter().all(|(index, role)| {
                        if *role != tcl_registry::ArgRole::ParamList {
                            return true;
                        }
                        effective
                            .get(usize::from(*index) + 1)
                            .and_then(|word| word.as_registry_word().literal())
                            .is_some_and(|parameters| {
                                self.baseline.dialect
                                    .and_then(tcl_registry::InvocationDialect::parameter_grammar)
                                    .is_some_and(|grammar| {
                                        tcl_syntax::formal_params::parse_formal_parameters_in(
                                            parameters, grammar,
                                        ).is_ok()
                                    })
                            })
                    })
            })
    }

    fn apply_bounded_constructor(
        &mut self,
        target: &SourceCommandTarget,
        words: &[crate::registry_invocation::EffectiveInvocationWord],
        registry: &CommandRegistry,
        namespace: &str,
        realm: tcl_dialect::model::InvocationRealm,
    ) -> bool {
        let Some(identity) = &target.identity else {
            return false;
        };
        let Some(construction) = self.bounded_constructions.get(identity) else {
            return false;
        };
        if target.implementation_generation != construction.implementation_generation {
            return false;
        }
        if self
            .class_definitions
            .get(identity)
            .is_none_or(|definition| !self.class_definition_dependencies_hold(definition))
        {
            return false;
        }
        let Some(grammar) = self.baseline.dialect.and_then(|dialect| {
            registry.native_default_construction_grammar(&construction.factory, dialect, realm)
        }) else {
            return false;
        };
        if !self.default_construction_dependencies_hold(grammar) {
            return false;
        }
        let Some(method) = words
            .first()
            .and_then(|word| word.as_registry_word().literal())
            .and_then(|method| grammar.manufacturer(method))
        else {
            return false;
        };
        if method.visibility != tcl_registry::definer::MemberVisibility::Exported
            || words.len() != usize::from(method.constructor_args_from)
            || words.iter().any(|word| {
                matches!(
                    word,
                    crate::registry_invocation::EffectiveInvocationWord::Expanded
                        | crate::registry_invocation::EffectiveInvocationWord::Opaque
                )
            })
        {
            return false;
        }
        self.install_manufacture_slot(target, method, words, namespace)
    }

    fn install_manufacture_slot(
        &mut self,
        target: &SourceCommandTarget,
        method: &tcl_registry::definer::ManufacturerMethod,
        words: &[crate::registry_invocation::EffectiveInvocationWord],
        namespace: &str,
    ) -> bool {
        if let Some(index) = method.names_instance_at {
            let Some(name) = words
                .get(usize::from(index))
                .and_then(|word| word.as_registry_word().literal())
                .filter(|name| !name.is_empty())
            else {
                return false;
            };
            if !name.starts_with("::") && !self.source_variables.namespace_known {
                return false;
            }
            let name = tcl_syntax::naming::qualify(namespace, name);
            if !self.definitely_absent(&name, namespace) {
                return false;
            }
            // A proved fresh root slot cannot replace an existing object's
            // private dispatcher. Other namespace routes keep that dependency
            // conservative until their actual namespace object is retained.
            let preserves_dispatcher = tcl_syntax::naming::key_holder_and_tail(&name).0 == "::";
            let dispatcher_generation = self.object_instances.receiver_dispatcher_generation;
            let receivers = preserves_dispatcher.then(|| self.object_instances.receivers.clone());
            self.install(
                name.clone(),
                MayBinding::Target(ResolvedCommandTarget {
                    command: name.clone(),
                    prepended: Vec::new(),
                    registry_backed: false,
                    kind: BindingKind::Command,
                    implementation_generation: 0,
                    terminal: true,
                    target_lookup: tcl_registry::AliasTargetLookup::Global,
                    token: Some(CommandIdentity {
                        runtime: None,
                        origin: name,
                        declaration: target.implementation_generation,
                        allocation: None,
                    }),
                    implementation_allocation: None,
                }),
            );
            if let Some(receivers) = receivers {
                let objects = Arc::make_mut(&mut self.object_instances);
                objects.receiver_dispatcher_generation = dispatcher_generation;
                objects.receivers = receivers;
            }
        }
        true
    }
    /// Runtime policies supplied by the source execution entry contract.
    #[must_use]
    pub(crate) fn invocation_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.baseline.dialect
    }
    fn definitely_absent(&self, name: &str, namespace: &(impl NamespaceKeyQuery + ?Sized)) -> bool {
        self.source_slot_presence(name, namespace) == SourceCommandSlotPresence::Absent
    }

    fn provider_surface_is_live(&self, loader: &TrustedPackageLoader) -> bool {
        let Some(root) = self.source_root_namespace_key() else {
            return false;
        };
        if self.tainted_provider_state.contains(&loader.package) {
            return false;
        }
        let required = loader.command_surface.iter().all(|command| {
            let proof = source_binding(self, command, &root);
            proof.proved_target().is_some_and(|target| {
                target.registry_backed
                    && target.prepended.is_empty()
                    && nqn(&target.command) == nqn(command)
                    && target.implementation_generation == 0
            })
        });
        required
            && self.provider_lookups_are_live(loader)
            && loader.optional_command_surface.iter().all(|command| {
                let slot = nqn(command);
                let keys = self.source_keys(command, &root);
                keys.len() == 1
                    && self.bindings.get(&keys[0]).is_some_and(|alternatives| {
                        alternatives.iter().all(|binding| match binding {
                            MayBinding::Missing => true,
                            MayBinding::Target(target) => {
                                target.registry_backed
                                    && target.terminal
                                    && target.prepended.is_empty()
                                    && target.command == slot
                                    && target.implementation_generation == 0
                            }
                            MayBinding::Imported(_) | MayBinding::Unknown => false,
                        })
                    })
            })
    }

    fn record_provider_state_writes(
        &mut self,
        facts: &tcl_registry::InvocationFacts,
        invocation: tcl_registry::InvocationWords<'_>,
        registry: &CommandRegistry,
        operands: &crate::variable_bindings::OriginalVariableInvocation,
    ) {
        let mut places =
            crate::variable_bindings::source_variable_write_places_with_original_operands(
                facts,
                invocation.arguments(),
                &self.source_variables,
                registry,
                operands,
            );
        let mut observer_places = Vec::new();
        for fact in facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
        {
            if let StateTransition::Trace(tcl_registry::TraceTransition::Add {
                target: tcl_registry::TraceTarget::Variable(subject),
                ..
            }) = &fact.transition
            {
                observer_places.push(
                    subject
                        .argument_index()
                        .map_or_else(crate::place::unknown_top, |index| {
                            operands.trace_subject_access(index, &self.source_variables, registry)
                        }),
                );
            }
        }
        places.extend(observer_places.iter().cloned());
        self.record_provider_write_places(&places, &observer_places, registry);
    }

    fn record_provider_write_places(
        &mut self,
        places: &[crate::place::Place],
        observer_places: &[crate::place::Place],
        registry: &CommandRegistry,
    ) {
        if places.is_empty() {
            return;
        }
        let protocol = self
            .baseline
            .dialect
            .and_then(|dialect| dialect.package_protocol)
            .unwrap_or(tcl_dialect::PackageProtocol::Tcl);
        let dependencies = tcl_registry::model::binding::package_resolver_dependencies(protocol).iter().filter_map(|dependency| {
            let name = match dependency {
                tcl_registry::model::binding::PackageResolverDependency::GlobalVariable(name) => (*name).to_owned(),
                tcl_registry::model::binding::PackageResolverDependency::GlobalArrayElement { array, element } => format!("{array}({element})"),
                tcl_registry::model::binding::PackageResolverDependency::WorkingDirectory
                | tcl_registry::model::binding::PackageResolverDependency::Filesystem => return None,
            };
            Some(crate::var_resolve::resolve_place(&name, &self.source_variables, false, registry))
        }).collect::<Vec<_>>();
        if places.iter().any(|place| {
            dependencies
                .iter()
                .any(|dependency| crate::place::overlap(place, dependency))
        }) {
            Arc::make_mut(&mut self.revoked_loaders)
                .extend(self.baseline.trusted_loaders.keys().cloned());
        }
        for loader in self.baseline.trusted_loaders.values() {
            if loader.state_dependency_namespaces.is_empty() {
                continue;
            }
            let modelled = loader
                .modelled_state_variables
                .iter()
                .map(|name| {
                    crate::var_resolve::resolve_place(name, &self.source_variables, false, registry)
                })
                .collect::<Vec<_>>();
            let affected = places.iter().any(|place| {
                if place.dynamic
                    || place.kind == crate::place::PlaceKind::UpvarAlias
                    || (place.kind == crate::place::PlaceKind::Unknown
                        && place.ns == crate::place::LOCAL_NS)
                {
                    return true;
                }
                let actual = tcl_syntax::naming::key_segments(&place.ns);
                if !loader.state_dependency_namespaces.iter().any(|namespace| {
                    let dependency = tcl_syntax::naming::key_segments(namespace);
                    actual.starts_with(&dependency)
                        || (place.kind == crate::place::PlaceKind::Unknown
                            && dependency.starts_with(&actual))
                }) {
                    return false;
                }
                observer_places.contains(place)
                    || !modelled.iter().any(|allowed| {
                        allowed.kind == place.kind
                            && allowed.ns == place.ns
                            && allowed.name == place.name
                            && allowed.index == place.index
                    })
            });
            if affected {
                Arc::make_mut(&mut self.tainted_provider_state).insert(loader.package.clone());
            }
        }
    }

    fn provider_lookups_are_live(&self, loader: &TrustedPackageLoader) -> bool {
        self.provider_lookup_dependencies_are_live(
            loader
                .lookup_dependencies
                .iter()
                .chain(&loader.installed_lookup_dependencies),
        )
    }

    fn provider_lookup_dependencies_are_live<'a>(
        &self,
        mut dependencies: impl Iterator<Item = &'a TrustedCommandLookup>,
    ) -> bool {
        dependencies.all(|dependency| {
            let Some(namespace_key) = self.namespace_for_rooted_operand(&dependency.namespace)
            else {
                return false;
            };
            let proof = source_binding(self, &dependency.head, &namespace_key);
            if proof.proved_target().is_some_and(|target| {
                target.registry_backed
                    && target.prepended.is_empty()
                    && nqn(&target.command) == nqn(&dependency.registry_identity)
                    && target.implementation_generation == 0
            }) {
                return true;
            }
            !dependency.required
                && self.definitely_absent(&dependency.head, &namespace_key)
                && self
                    .source_keys(&dependency.head, &namespace_key)
                    .iter()
                    .all(|slot| {
                        !self.bindings.contains_key(slot)
                            && !slot.authored_spelling().is_some_and(|slot| {
                                self.baseline.semantics.binding_names().contains(slot)
                            })
                    })
        })
    }

    fn loaded_provider(&self, package: &str) -> Option<&TrustedPackageLoader> {
        if self.opaque_domain {
            return None;
        }
        let provisions = self.packages.get(package)?;
        if provisions.len() != 1 {
            return None;
        }
        let provision = provisions.first()?;
        let loader = self.baseline.trusted_loaders.get(package)?;
        if loader.required_core_family.is_some_and(|family| {
            self.baseline
                .dialect
                .and_then(tcl_registry::InvocationDialect::family)
                != Some(family)
        }) {
            return None;
        }
        (provision.present
            && provision.implementation.as_deref() == Some(loader.implementation_id.as_str()))
        .then_some(loader)
    }

    fn initial(registry: &CommandRegistry) -> Self {
        let baseline = Arc::new(BindingBaseline::for_registry(registry));
        Self::initial_from_baseline(registry, baseline)
    }

    fn initial_from_baseline(registry: &CommandRegistry, baseline: Arc<BindingBaseline>) -> Self {
        let mut exports: BTreeMap<String, Vec<tcl_core_types::NameBytes>> = BTreeMap::new();
        let mut namespaces = BTreeSet::from(["::".to_owned()]);
        if let Some(context) = baseline.hosted_execution_context {
            namespaces.extend(
                tcl_registry::f5::runtime_namespaces(context)
                    .iter()
                    .map(|namespace| (*namespace).to_owned()),
            );
        }
        for command in baseline.semantics.binding_names() {
            let (holder, tail) = tcl_syntax::naming::key_holder_and_tail(command);
            namespaces.insert(if holder.is_empty() {
                "::".to_owned()
            } else {
                holder.to_owned()
            });
            if registry
                .get(command)
                .is_some_and(|spec| spec.is_namespace_exported)
                && command.is_ascii()
                && !command.as_bytes().contains(&0)
            {
                exports
                    .entry(holder.to_owned())
                    .or_default()
                    .push(tcl_core_types::NameBytes::from(tail.as_bytes()));
            }
        }
        let mut state = Self {
            baseline,
            namespace_exports: Arc::new(
                exports
                    .into_iter()
                    .map(|(name, patterns)| (name, BTreeSet::from([patterns])))
                    .collect(),
            ),
            namespaces: Arc::new(namespaces.into_iter().collect()),
            ..Self::default()
        };
        state.original_command_world =
            Arc::new(source_command_world::OriginalSourceCommandWorld::for_baseline(&state));
        if state.baseline.native_entry.is_none()
            && !state.baseline.unknown_entry
            && let Some(dialect) = state.baseline.dialect
            && let Some(support) = registry
                .native_class_factory_recipe(
                    "oo::configurable",
                    dialect,
                    state.baseline.invocation_realm,
                )
                .and_then(tcl_registry::native_tcloo_bootstrap::NativeClassFactoryRecipe::support)
        {
            for receiver in [
                tcl_registry::definer::DefinitionReceiver::Instance,
                tcl_registry::definer::DefinitionReceiver::Class,
            ] {
                let namespace =
                    SourceNamespaceKey::authored(support.definition_namespace(receiver));
                let path = support
                    .definition_path(receiver)
                    .iter()
                    .map(|name| SourceNamespaceKey::authored(*name))
                    .collect::<Vec<_>>();
                if state.namespaces.contains(&namespace)
                    && path.iter().all(|key| state.namespaces.contains(key))
                {
                    Arc::make_mut(&mut state.namespace_paths)
                        .insert(namespace, BTreeSet::from([path]));
                }
            }
        }
        state
    }

    fn initial_with_options(
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
        config: Option<tcl_lexer::LexerConfig>,
    ) -> Self {
        let mut baseline = BindingBaseline::for_registry(registry);
        if options.native_entry.is_some()
            || options.execution_name_policy.is_some()
            || options.invocation_dialect.is_some()
            || options.vendor_source_input.is_some()
        {
            baseline.execution_name_policy = options.execution_name_policy();
        }
        baseline.metadata_context = options.retained_metadata_context();
        baseline.logical_source_input = options.retained_logical_source_input(registry, config);
        baseline.vendor_source_input = options.retained_vendor_source_input(registry, config);
        baseline.hosted_execution_context = options.hosted_execution_context;
        baseline.native_compilation = options.native_compilation;
        baseline.compiled_variable_provider = options.compiled_variable_provider;
        baseline.invocation_realm = options.invocation_realm;
        baseline.unknown_entry = options.unknown_entry;
        baseline.native_entry = options.native_entry.cloned().map(Arc::new);
        baseline.declared_surface = options.retained_declared_command_contracts();
        baseline.declared_commands =
            tcl_registry::model::DocumentCommandSurface::new(registry, options.declared_commands)
                .declared_names()
                .map(|name| (nqn(name), name.to_owned()))
                .collect();
        baseline.trusted_source_modules = options.trusted_source_modules.to_vec();
        baseline.trusted_loaders = options
            .trusted_package_loaders
            .iter()
            .map(|loader| (loader.package.clone(), loader.clone()))
            .collect();
        let logical = config.map_or_else(
            || options.logical_invocation_dialect(),
            |config| options.source_invocation_dialect(config),
        );
        if options.native_entry.is_some() && logical.is_none() {
            baseline.dialect = None;
            baseline.release = None;
            baseline.import_binding = None;
        }
        if let Some(dialect) = logical {
            baseline.semantics = match baseline.logical_source_input.as_ref() {
                Some(input) => registry.authored_source_semantics_in_context(
                    input.context_registry().context(),
                    baseline.invocation_realm,
                ),
                None => registry.effective_semantics_for_dialect_in_realm(
                    dialect,
                    tcl_dialect::model::InvocationRealm::InterpreterRuntime,
                ),
            };
            baseline.import_binding = dialect.namespace_import_binding;
            baseline.release = dialect.tcl_version;
            baseline.dialect = Some(dialect);
        }
        baseline.refresh_fingerprint();
        Self::initial_entry_from_baseline(registry, Arc::new(baseline))
    }

    /// Rebuild entry state from immutable entry facts, never a dispatch world's
    /// already-applied substitutions, declarations or command mutations.
    fn initial_entry_from_baseline(
        registry: &CommandRegistry,
        baseline: Arc<BindingBaseline>,
    ) -> Self {
        let mut state = Self::initial_from_baseline(registry, baseline);
        let mut variables = crate::var_resolve::ResolveContext::for_namespace("::");
        variables.frame_kind = crate::var_resolve::VariableFrameKind::Global;
        variables.invocation_dialect = state.baseline.dialect;
        variables.execution_name_policy = state.baseline.execution_name_policy;
        variables.hosted_execution_context = state.baseline.hosted_execution_context;
        variables.namespace_identities = state.namespaces.iter().cloned().collect();
        variables.known_namespaces = state
            .namespaces
            .iter()
            .filter_map(SourceNamespaceKey::advisory_key)
            .collect();
        if !state.baseline.unknown_entry {
            variables.namespace_cells = crate::variable_bindings::fresh_namespace_cells(registry);
        }
        state.source_variables = Arc::new(variables);
        state.variable_frame = crate::var_resolve::VariableExecutionFrame::Global;
        if let Some(entry) = state.baseline.native_entry.clone() {
            state.install_runtime_entry(&entry);
        }
        if state.baseline.unknown_entry && state.baseline.native_entry.is_none() {
            state.mark_opaque_binding_mutation();
            state.loader_handler_unknown = true;
        }
        state
    }
    #[cfg(test)]
    pub(crate) fn effective_semantics(&self) -> &Arc<EffectiveRegistrySemantics> {
        &self.baseline.semantics
    }

    /// Fast equality for clones and branches inside *one* analysis. Public
    /// equality remains semantic for Salsa/context interning; the fixpoint
    /// owns one baseline [`Arc`], so its allocation identity is sufficient.
    fn same_state(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.baseline, &other.baseline)
            && (Arc::ptr_eq(&self.original_command_world, &other.original_command_world)
                || self.original_command_world == other.original_command_world)
            && self.ordinary_literal_pool == other.ordinary_literal_pool
            && self.class_definitions == other.class_definitions
            && self.object_instances == other.object_instances
            && self.bounded_constructions == other.bounded_constructions
            && self.tainted_object_dispatch == other.tainted_object_dispatch
            && self.tainted_provider_state == other.tainted_provider_state
            && self.command_observers == other.command_observers
            && self.source_variables == other.source_variables
            && self.variable_frame == other.variable_frame
            && (Arc::ptr_eq(&self.bindings, &other.bindings) || self.bindings == other.bindings)
            && self.objects == other.objects
            && self.namespace_exports == other.namespace_exports
            && self.namespace_paths == other.namespace_paths
            && self.namespaces == other.namespaces
            && self.unknown_lookup_namespaces == other.unknown_lookup_namespaces
            && self.unknown_namespace_paths == other.unknown_namespace_paths
            && self.namespace_unknown_handlers == other.namespace_unknown_handlers
            && self.unknown_export_namespaces == other.unknown_export_namespaces
            && self.packages == other.packages
            && self.revoked_loaders == other.revoked_loaders
            && self.loader_handler_unknown == other.loader_handler_unknown
            && (Arc::ptr_eq(&self.root_boundary_bindings, &other.root_boundary_bindings)
                || self.root_boundary_bindings == other.root_boundary_bindings)
            && self.opaque_domain == other.opaque_domain
            && self.opaque_binding_mutation == other.opaque_binding_mutation
            && self.dynamic_proc_binding == other.dynamic_proc_binding
            && self.unnameable_rebinding_subject == other.unnameable_rebinding_subject
            && self.namespace_resolution == other.namespace_resolution
            && (Arc::ptr_eq(&self.procedure_bodies, &other.procedure_bodies)
                || self.procedure_bodies == other.procedure_bodies)
            && self.rebound_names == other.rebound_names
            && self.proc_rebound_names == other.proc_rebound_names
            && self.has_redefined_procedures == other.has_redefined_procedures
            && self.source_order_user_call_effects == other.source_order_user_call_effects
    }

    fn non_binding_observation_stamp(&self) -> NonBindingObservationStamp {
        NonBindingObservationStamp {
            original_command_world: Arc::clone(&self.original_command_world),
            ordinary_literal_pool: self.ordinary_literal_pool.clone(),
            class_definitions: Arc::clone(&self.class_definitions),
            object_instances: Arc::clone(&self.object_instances),
            bounded_constructions: Arc::clone(&self.bounded_constructions),
            tainted_object_dispatch: Arc::clone(&self.tainted_object_dispatch),
            tainted_provider_state: Arc::clone(&self.tainted_provider_state),
            command_observers: Arc::clone(&self.command_observers),
            source_variables: Arc::clone(&self.source_variables),
            variable_frame: self.variable_frame.clone(),
            objects: Arc::clone(&self.objects),
            namespace_exports: Arc::clone(&self.namespace_exports),
            namespace_paths: Arc::clone(&self.namespace_paths),
            namespaces: Arc::clone(&self.namespaces),
            unknown_lookup_namespaces: Arc::clone(&self.unknown_lookup_namespaces),
            unknown_namespace_paths: Arc::clone(&self.unknown_namespace_paths),
            namespace_unknown_handlers: Arc::clone(&self.namespace_unknown_handlers),
            unknown_export_namespaces: Arc::clone(&self.unknown_export_namespaces),
            packages: Arc::clone(&self.packages),
            revoked_loaders: Arc::clone(&self.revoked_loaders),
            loader_handler_unknown: self.loader_handler_unknown,
            opaque_domain: self.opaque_domain,
            opaque_binding_mutation: self.opaque_binding_mutation,
            dynamic_proc_binding: self.dynamic_proc_binding,
            unnameable_rebinding_subject: self.unnameable_rebinding_subject,
            namespace_changed: self.namespace_resolution.changed,
            namespace_dynamic: self.namespace_resolution.dynamic,
            namespace_rebound_count: self.namespace_resolution.rebound_names.len(),
            namespace_opaque_count: self.namespace_resolution.opaque_namespaces.len(),
            procedure_body_count: self.procedure_bodies.len(),
            rebound_count: self.rebound_names.len(),
            proc_rebound_count: self.proc_rebound_names.len(),
            has_redefined_procedures: self.has_redefined_procedures,
        }
    }
}

impl std::hash::Hash for ModuleCommandBindings {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.original_command_world.hash(state);
        self.current_source_origin.hash(state);
        self.ordinary_literal_pool.hash(state);
        self.allocation_counts.hash(state);
        self.namespace_allocation_counts.hash(state);
        self.class_definitions.hash(state);
        self.object_instances.hash(state);
        self.bounded_constructions.hash(state);
        self.tainted_object_dispatch.hash(state);
        self.tainted_provider_state.hash(state);
        self.command_observers.hash(state);
        self.source_variables.hash(state);
        self.variable_frame.hash(state);
        let mut entries: Vec<_> = self.bindings.iter().collect();
        entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        entries.hash(state);
        self.objects.hash(state);
        self.namespace_exports.hash(state);
        self.namespace_paths.hash(state);
        self.namespaces.hash(state);
        self.unknown_lookup_namespaces.hash(state);
        self.unknown_namespace_paths.hash(state);
        self.namespace_unknown_handlers.hash(state);
        self.unknown_export_namespaces.hash(state);
        self.packages.hash(state);
        self.revoked_loaders.hash(state);
        self.loader_handler_unknown.hash(state);
        let mut boundary_entries: Vec<_> = self.root_boundary_bindings.iter().collect();
        boundary_entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        boundary_entries.hash(state);
        self.baseline.hash(state);
        self.opaque_domain.hash(state);
        self.opaque_binding_mutation.hash(state);
        self.dynamic_proc_binding.hash(state);
        self.unnameable_rebinding_subject.hash(state);
        self.namespace_resolution.hash(state);
        self.procedure_bodies.hash(state);
        self.rebound_names.hash(state);
        self.proc_rebound_names.hash(state);
        self.has_redefined_procedures.hash(state);
        self.source_order_user_call_effects.hash(state);
    }
}

/// A registry invocation selected through the module-wide may-binding state.
/// Alias-prefix arguments have already been prepended.
pub(crate) struct ResolvedBindingInvocation {
    command: String,
    source_span: tcl_lexer::Span,
    pub(crate) facts: Box<tcl_registry::InvocationFacts>,
    pub(crate) arguments: Vec<String>,
    /// Post-alias argv values proven literal at their effective positions.
    /// Dynamic, expanded, and opaque positions are `None` even though
    /// [`Self::arguments`] retains their source spelling for diagnostics and
    /// token metadata alignment.
    literal_arguments: Vec<Option<String>>,
    /// Exact effective argv length after alias-prefix insertion. Expansion or
    /// another indeterminate source word leaves this unknown.
    exact_argument_count: Option<usize>,
}

/// Frame selected for a registry-resolved evaluated script body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedFrameBodySelection {
    /// The frame in which the invocation itself executes.
    Current,
    /// A frame selected by the invocation's registry-owned level grammar.
    Selected(tcl_registry::frame_effect::FrameLevel),
}

/// Source-safe projection of one registry-resolved frame-evaluated body.
pub(crate) enum ResolvedFrameBody {
    /// The invocation has no script-in-frame descriptor.
    NotApplicable,
    /// Tcl rejects the invocation before a body can run.
    KnownError,
    /// One exact source script and its selected frame.
    Readable {
        source: String,
        selection: ResolvedFrameBodySelection,
    },
    /// A body may run in the selected frame, but its source is not exact.
    Opaque {
        selection: ResolvedFrameBodySelection,
    },
}

impl ResolvedBindingInvocation {
    /// Frame-body layout under actual supplied availability, without a
    /// Native frame-entry or execution grant. Missing input widens selection.
    pub(crate) fn resolved_frame_body_with_metadata_context(
        &self,
        registry: &CommandRegistry,
        bindings: &ModuleCommandBindings,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> ResolvedFrameBody {
        let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
            return ResolvedFrameBody::Opaque {
                selection: ResolvedFrameBodySelection::Selected(
                    tcl_registry::frame_effect::FrameLevel::Dynamic,
                ),
            };
        };
        self.resolved_frame_body_with_optional_metadata(
            registry,
            bindings,
            namespace,
            Some(metadata),
        )
    }

    fn resolved_frame_body_with_optional_metadata(
        &self,
        registry: &CommandRegistry,
        bindings: &ModuleCommandBindings,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> ResolvedFrameBody {
        use tcl_registry::frame_effect::FrameArgLayout;

        let Some(spec) = self.facts.frame_effect else {
            return ResolvedFrameBody::NotApplicable;
        };
        let refs: Vec<&str> = self.arguments.iter().map(String::as_str).collect();
        let (body_index, body_len, selection) = match spec.layout {
            FrameArgLayout::ScriptInCurrentFrame => {
                (0, refs.len(), ResolvedFrameBodySelection::Current)
            }
            FrameArgLayout::ScriptInSelectedFrame => {
                let words = self
                    .literal_arguments
                    .iter()
                    .map(|word| {
                        word.as_deref().map_or(
                            tcl_registry::InvocationWord::Dynamic,
                            tcl_registry::InvocationWord::Literal,
                        )
                    })
                    .collect::<Vec<_>>();
                let mut invocation = tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&self.command),
                    &words,
                )
                .with_profile(registry.profile());
                if let Some(dialect) = bindings.baseline.dialect {
                    invocation = invocation.with_dialect(dialect);
                }
                let arguments = invocation.arguments();
                let (width, level) = match spec.resolve_arguments(arguments) {
                    tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                        level_word_len,
                        level,
                    } => (level_word_len, level),
                    tcl_registry::frame_effect::FrameArgumentResolution::Invalid => {
                        return ResolvedFrameBody::KnownError;
                    }
                    tcl_registry::frame_effect::FrameArgumentResolution::Unknown => {
                        return ResolvedFrameBody::Opaque {
                            selection: ResolvedFrameBodySelection::Selected(
                                tcl_registry::frame_effect::FrameLevel::Dynamic,
                            ),
                        };
                    }
                };
                let body = &refs[width..];
                (
                    refs.len().saturating_sub(body.len()),
                    body.len(),
                    ResolvedFrameBodySelection::Selected(level),
                )
            }
            FrameArgLayout::AliasPairs | FrameArgLayout::OpaqueCallerVars => {
                return ResolvedFrameBody::NotApplicable;
            }
        };
        if body_len == 0 {
            return ResolvedFrameBody::KnownError;
        }
        if self.exact_argument_count.is_none() || body_len != 1 {
            return ResolvedFrameBody::Opaque { selection };
        }
        readable_script_argument_with_optional_metadata(
            self, body_index, registry, bindings, namespace, metadata,
        )
        .map_or(ResolvedFrameBody::Opaque { selection }, |source| {
            ResolvedFrameBody::Readable { source, selection }
        })
    }
}

impl ModuleCommandBindings {
    /// Extend the retained-body inventory at the one copy-on-write mutation
    /// seam. Ordinary state forks keep sharing the module's original set;
    /// only a branch that recovers a previously-unseen procedure allocates.
    fn extend_procedure_bodies(&mut self, names: impl IntoIterator<Item = String>) -> bool {
        let additions: Vec<String> = names
            .into_iter()
            .filter(|name| !self.procedure_bodies.contains(name))
            .collect();
        if additions.is_empty() {
            return false;
        }
        let bodies = Arc::make_mut(&mut self.procedure_bodies);
        bodies.extend(additions);
        true
    }

    /// Widen command resolution without claiming an unknown command-table
    /// mutation. Namespace lookup changes are relevant to source-safe
    /// resolution but do not by themselves invalidate optimiser trust.
    fn mark_opaque_resolution(&mut self) {
        Arc::make_mut(&mut self.original_command_world).withdraw();
        self.opaque_domain = true;
    }

    /// Widen both command resolution and the optimiser's command-trust
    /// projection because an unbounded command-binding effect may occur.
    fn mark_opaque_binding_mutation(&mut self) {
        Arc::make_mut(&mut self.original_command_world).withdraw();
        self.ordinary_literal_pool = None;
        Arc::make_mut(&mut self.object_instances).invalidate_dispatch(true);
        self.opaque_domain = true;
        self.opaque_binding_mutation = true;
        Arc::make_mut(&mut self.namespaces).retain(SourceNamespaceKey::is_root);
        Arc::make_mut(&mut self.source_variables).widen();
    }

    /// Record the narrow event that invalidates an otherwise-retained
    /// procedure's declared binding at arbitrary call sites.
    fn mark_dynamic_proc_binding(&mut self) {
        self.mark_opaque_binding_mutation();
        self.dynamic_proc_binding = true;
    }

    /// Record that a binding *transition* moved something this lattice cannot
    /// name, so no name in the module can be claimed as untouched (#2168).
    fn mark_unnameable_rebinding_subject(&mut self) {
        self.mark_dynamic_proc_binding();
        self.unnameable_rebinding_subject = true;
    }

    fn record_proc_rebound_candidates(
        &mut self,
        name: &str,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) {
        let Some(context) = namespace.for_head_context(name) else {
            self.mark_dynamic_proc_binding();
            return;
        };
        if let SourceNamespaceKey::Authored(namespace) = context.as_ref() {
            insert_rebound_candidates(name, namespace, &mut self.proc_rebound_names);
            return;
        }
        let Ok(candidates) = self.source_keys_checked(name, context.as_ref()) else {
            self.mark_dynamic_proc_binding();
            return;
        };
        let Some(labels) = candidates
            .iter()
            .map(crate::command_binding::ModuleCommandBindings::command_key_label)
            .collect::<Option<Vec<_>>>()
        else {
            self.mark_dynamic_proc_binding();
            return;
        };
        // Labels only invalidate procedure trust. The original typed slots
        // remain the command-table lookup and mutation identities.
        self.proc_rebound_names.extend(labels);
    }

    /// Build the closed may-binding state for every retained executable root.
    #[must_use]
    pub(crate) fn analyse(module: &Module, registry: &CommandRegistry) -> Self {
        Self::analyse_with_options(module, registry, module.source_entry.options())
    }

    /// Build the executable may-state under the same entry contract as source
    /// interpretation. Catalogue-only callers retain unknown loader effects.
    #[must_use]
    pub(crate) fn analyse_with_options(
        module: &Module,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        if let Some(observed) = Self::analyse_retained_source(module, registry, options) {
            return observed;
        }
        Self::analyse_retained_roots(module, registry, options)
    }

    /// Prefer the source kernel's actual execution history over IR replay.
    fn analyse_retained_source(
        module: &Module,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<Self> {
        if let Some(retained) = &module.retained_source_bindings
            && let Some(observed) = retained.projection(module, registry, options)
        {
            return Some(observed);
        }
        if !module.source.is_empty() {
            let config = module.native_lexer_config();
            let namespace = if module.top_level_namespace.is_empty() {
                "::"
            } else {
                &module.top_level_namespace
            };
            let frame = retained_source::source_analysis_frame(
                options.native_entry,
                namespace,
                module.top_level_kind,
            );
            let interpreted = SourceCommandBindings::analyse_image_in_frame_with_options(
                &module.source,
                &frame,
                config,
                registry,
                options,
            )?;
            return Some(interpreted.module_projection(module));
        }
        None
    }

    /// An IR-only caller supplies executable roots without an original source instance.
    fn analyse_retained_roots(
        module: &Module,
        registry: &CommandRegistry,
        options: SourceAnalysisOptions<'_>,
    ) -> Self {
        let discarded = discarded_procedure_history(module, registry);
        let opaque_binding_mutation = discarded.opaque
            || module.oo_evidence.unretained_executable_roots
            || discarded
                .modules
                .iter()
                .any(|module| module.oo_evidence.unretained_executable_roots);
        let dynamic_proc_binding = module
            .independent_executable_script_roots()
            .into_iter()
            .any(|(script, namespace)| {
                matches!(namespace, crate::ir::ExecutionNamespace::RuntimeSelected)
                    && crate::ir_helpers::requires_runtime_command_namespace(script, registry)
            });
        let mut procedure_bodies: BTreeSet<String> = module.procedures.keys().cloned().collect();
        for discarded_module in &discarded.modules {
            procedure_bodies.extend(discarded_module.procedures.keys().cloned());
        }
        let mut state = Self {
            opaque_domain: opaque_binding_mutation || options.unknown_entry,
            opaque_binding_mutation: opaque_binding_mutation || options.unknown_entry,
            dynamic_proc_binding,
            procedure_bodies: Arc::new(procedure_bodies),
            rebound_names: discarded
                .rebound_names
                .into_iter()
                .map(SourceCommandKey::authored)
                .collect(),
            has_redefined_procedures: !module.redefined_procedures.is_empty(),
            ..Self::initial_with_options(registry, options, Some(module.native_lexer_config()))
        };
        let baseline = Arc::make_mut(&mut state.baseline);
        baseline.declared = Arc::new(module.declared_frame_effects.clone());
        baseline.refresh_fingerprint();
        // Execute the top-level root once in source order. Its intermediate
        // states are observable, but they are not valid entry states for a
        // later procedure invocation: replaying a rename/restore sequence
        // from their union creates command-table states Tcl can never reach.
        let mut roots = module.independent_executable_script_roots().into_iter();
        let (top_level, top_namespace) = roots
            .next()
            .expect("a module always publishes its top-level script root");
        let mut retained_roots = RetainedBindingRoots::default();
        retained_roots.extend_module_roots(module, true);
        for discarded_module in &discarded.modules {
            retained_roots.extend_module_roots(discarded_module, false);
        }
        let top = collect_normal_binding_states(
            top_level,
            registry,
            &state,
            &top_namespace,
            &mut retained_roots,
        );
        let mut live = top.post;
        let mut observed = top.observed;

        // Procedure, method, body-unit, and recovered roots may run in any
        // order and more than once. Iterate their *post* states to a fixpoint;
        // retain intermediate states for consumer queries without feeding
        // those historical states back as executable entry states.
        loop {
            let before = live.clone();
            let root_count_before = retained_roots.len();
            let body_roots = retained_roots.snapshot();
            let mut next = live.clone();
            for root in &body_roots {
                let outcome = collect_normal_binding_states(
                    &root.script,
                    registry,
                    &live,
                    &root.namespace,
                    &mut retained_roots,
                );
                if !next.same_state(&outcome.post) {
                    next.join(&outcome.post);
                }
                // An effect-free root observes only its unchanged entry state.
                // That state is already represented by the top-level history
                // (and by every prior fixpoint round), so comparing/joining it
                // against the larger historical union merely rescans the full
                // binding map once per procedure. A root with any transient or
                // lasting transition produces a distinct observed state and
                // still takes the ordinary lattice join below.
                if !live.same_state(&outcome.observed) && !observed.same_state(&outcome.observed) {
                    observed.join(&outcome.observed);
                }
            }
            for root in retained_roots
                .source_roots
                .values()
                .cloned()
                .collect::<Vec<_>>()
            {
                let frame = crate::var_resolve::VariableExecutionFrame::Procedure {
                    namespace: root.namespace.clone(),
                    identity: root.identity.clone(),
                }
                .with_namespace_identity(root.namespace_key.clone());
                let outcome = interpret_binding_source(
                    &root.source,
                    root.offset,
                    &root.namespace,
                    &frame,
                    registry,
                    &live,
                    &mut retained_roots,
                );
                next.join(&outcome.post);
                observed.join(&outcome.observed);
            }
            if next.same_state(&before) && retained_roots.len() == root_count_before {
                // Publish every state that was genuinely observable during a
                // root execution, including temporary aliases. Only boundary
                // post-states feed the fixpoint above, so this historical
                // union is never replayed into an impossible command cycle.
                observed.root_boundary_bindings = next.bindings;
                return observed;
            }
            live = next;
        }
    }

    /// Replay an independently callable body from completed root boundaries,
    /// excluding historical pre-definition missing candidates.
    pub(crate) fn source_binding_timeline_from_boundary(
        &self,
        script: &Script,
        registry: &CommandRegistry,
        namespace: &crate::ir::ExecutionNamespace,
    ) -> SourceBindingTimeline {
        let effects = self.source_order_call_boundary();
        let mut initial = effects.as_ref().clone();
        initial.source_order_user_call_effects = Some(effects);
        Self::source_binding_timeline_from_initial(script, registry, namespace, &initial)
    }

    /// Replay one root in source order for CFG-local registry projections.
    /// Top level begins with its fresh registry state; independently callable
    /// procedure roots begin from the historical state reachable after module
    /// initialisation and never feed their transient states back into it.
    #[must_use]
    pub(crate) fn source_binding_timeline(
        &self,
        script: &Script,
        registry: &CommandRegistry,
        namespace: &crate::ir::ExecutionNamespace,
        top_level_root: bool,
    ) -> SourceBindingTimeline {
        let mut initial = self.clone();
        if initial.source_order_user_call_effects.is_none() {
            initial.source_order_user_call_effects = Some(self.source_order_call_boundary());
        }
        if top_level_root {
            initial.bindings = Arc::new(SourceCommandTable::default());
            initial.root_boundary_bindings = Arc::new(SourceCommandTable::default());
            initial.opaque_domain = false;
            initial.opaque_binding_mutation = false;
            initial.dynamic_proc_binding = false;
            initial.unnameable_rebinding_subject = false;
            initial.namespace_resolution = NamespaceResolutionProjection::default();
            initial.rebound_names.clear();
            initial.proc_rebound_names.clear();
        }
        Self::source_binding_timeline_from_initial(script, registry, namespace, &initial)
    }

    /// Replay one source root from a caller-provided entry state. Procedure
    /// CFGs use this to start from the states reachable after their own
    /// definition, instead of the module's historical union from before that
    /// definition existed.
    #[must_use]
    pub(crate) fn source_binding_timeline_from_initial(
        script: &Script,
        registry: &CommandRegistry,
        namespace: &crate::ir::ExecutionNamespace,
        initial: &ModuleCommandBindings,
    ) -> SourceBindingTimeline {
        let mut timeline = SourceBindingTimeline::default();
        let mut retained_roots = RetainedBindingRoots::default();
        let mut context = BindingWalkContext {
            registry,
            retained_roots: &mut retained_roots,
            timeline: Some(&mut timeline),
            source_order_mode: true,
        };
        timeline.post = Some(collect_binding_states(script, &mut context, initial, namespace).post);
        timeline
    }

    /// Resolve every live source-safe invocation selected by the exact active
    /// registry. Alias chains are expanded to their terminal target.
    #[must_use]
    pub(crate) fn resolve_statement(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<ResolvedBindingInvocation> {
        let context = registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile);
        let context = context.map(crate::registry_invocation::InvocationMetadataContext::from);
        self.resolve_statement_with_optional_metadata(stmt, registry, context, namespace)
    }

    /// Original may-target metadata under supplied complete availability.
    /// Missing or foreign metadata refuses; targets and composed original
    /// operands do not establish a Native frame, completion or physical effect.
    #[must_use]
    pub(crate) fn resolve_statement_with_metadata_context(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<ResolvedBindingInvocation> {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return Vec::new();
        };
        self.resolve_statement_with_optional_metadata(stmt, registry, Some(context), namespace)
    }

    fn resolve_statement_with_optional_metadata(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<ResolvedBindingInvocation> {
        if !stmt.is_executable_invocation() {
            return Vec::new();
        }
        let source_span = stmt.span();
        let (Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. }) = stmt
        else {
            return Vec::new();
        };
        if tokens.is_none() {
            return Vec::new();
        }

        let realm = tokens
            .as_ref()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(SourceInvocationBinding::invocation_realm)
            .unwrap_or(self.baseline.invocation_realm);
        let mut resolved = Vec::new();
        self.for_each_resolved_invocation(stmt, namespace, |target, words| {
            if !target.registry_backed {
                return;
            }
            let Some(facts) =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    registry,
                    context.map(crate::registry_invocation::InvocationMetadataContext::context),
                    words,
                    realm,
                )
                .resolved()
                .map(|resolved| Box::new(resolved.facts()))
            else {
                return;
            };
            let Some(mut arguments) = target
                .prepended
                .iter()
                .map(|word| word.as_registry_word().literal().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
            else {
                return;
            };
            arguments.extend(args.iter().cloned());
            let invocation_arguments = words.arguments();
            let exact_argument_count = invocation_arguments.exact_argv_len();
            let literal_arguments = (0..arguments.len())
                .map(|index| match invocation_arguments.argv_at(index) {
                    tcl_registry::InvocationArgument::Word(
                        tcl_registry::InvocationWord::Literal(literal),
                    ) => Some(literal.to_owned()),
                    tcl_registry::InvocationArgument::Word(_)
                    | tcl_registry::InvocationArgument::Indeterminate
                    | tcl_registry::InvocationArgument::Missing => None,
                })
                .collect();
            resolved.push(ResolvedBindingInvocation {
                command: target.command.clone(),
                source_span,
                facts,
                arguments,
                literal_arguments,
                exact_argument_count,
            });
        });
        resolved
    }

    /// Visit every effective terminal invocation selected for `stmt`.
    ///
    /// This is the single source of truth for joining source words with the
    /// literal arguments prepended by an `interp alias` chain. Both retained
    /// user procedures and registry-backed targets are reported; consumers
    /// select the semantic domain they own from [`ResolvedCommandTarget`].
    /// A source-aware statement keeps substitution and expansion opaque, while
    /// a hand-built statement without tokens falls back to the all-literal
    /// argument view.
    fn contextual_invocation_words<'w>(
        &self,
        command: &'w str,
        arguments: &'w [tcl_registry::InvocationWord<'w>],
    ) -> tcl_registry::InvocationWords<'w> {
        let words = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(command),
            arguments,
        );
        self.baseline
            .dialect
            .map_or(words, |dialect| words.with_dialect(dialect))
    }

    pub(crate) fn for_each_resolved_invocation<F>(
        &self,
        stmt: &Statement,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        mut visit: F,
    ) where
        F: for<'w> FnMut(&'w ResolvedCommandTarget, tcl_registry::InvocationWords<'w>),
    {
        if !stmt.is_executable_invocation() {
            return;
        }
        let (Statement::Call {
            command,
            args,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            args,
            tokens,
            ..
        }) = stmt
        else {
            return;
        };
        let targets = self.targets(command, namespace);
        if let Some(tokens) = tokens {
            let words = tokens.words();
            if !matches!(
                words
                    .first()
                    .map(crate::registry_invocation::invocation_word),
                Some(tcl_registry::InvocationWord::Literal(_))
            ) {
                return;
            }
            for target in targets {
                let mut arguments: Vec<_> = target
                    .prepended
                    .iter()
                    .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                    .collect();
                arguments.extend(
                    words
                        .get(1..)
                        .unwrap_or_default()
                        .iter()
                        .map(crate::registry_invocation::invocation_word),
                );
                visit(
                    &target,
                    self.contextual_invocation_words(&target.command, &arguments),
                );
            }
            return;
        }

        for target in targets {
            let arguments: Vec<_> = target
                .prepended
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                .chain(
                    args.iter()
                        .map(|word| tcl_registry::InvocationWord::Literal(word.as_str())),
                )
                .collect();
            visit(
                &target,
                self.contextual_invocation_words(&target.command, &arguments),
            );
        }
    }

    /// Visit every effective terminal invocation selected for parsed embedded
    /// command words.
    ///
    /// This is the sole binding owner for joining a tokenised embedded command
    /// with the literal argv prefix supplied by an `interp alias` chain. The
    /// caller supplies the exact execution namespace and words tokenised with
    /// the active dialect; consumers never repeat binding or prefix logic.
    pub(crate) fn for_each_resolved_command_words<F>(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        mut visit: F,
    ) where
        F: for<'w> FnMut(&'w ResolvedCommandTarget, tcl_registry::InvocationWords<'w>),
    {
        let Some(command) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return;
        };
        for target in self.targets(command, namespace) {
            let mut arguments: Vec<_> = target
                .prepended
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                .collect();
            arguments.extend(
                words
                    .get(1..)
                    .unwrap_or_default()
                    .iter()
                    .map(crate::ir_helpers::CommandWord::invocation_word),
            );
            visit(
                &target,
                self.contextual_invocation_words(&target.command, &arguments),
            );
        }
    }

    /// Resolve parsed embedded command words through the same closed binding,
    /// alias-prefix, registry, and dialect context as direct statements.
    #[must_use]
    pub(crate) fn resolve_command_words(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<tcl_registry::InvocationFacts> {
        let context = registry
            .profile()
            .map(tcl_registry::model::semantic::SemanticContext::for_profile);
        let context = context.map(crate::registry_invocation::InvocationMetadataContext::from);
        self.resolve_command_words_with_optional_metadata(words, registry, context, namespace)
    }

    /// Original may-target metadata under supplied complete availability.
    /// Missing or foreign metadata refuses; targets and composed original
    /// operands do not establish a Native frame, completion or physical effect.
    #[must_use]
    pub(crate) fn resolve_command_words_with_metadata_context(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<tcl_registry::InvocationFacts> {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return Vec::new();
        };
        self.resolve_command_words_with_optional_metadata(words, registry, Some(context), namespace)
    }

    fn resolve_command_words_with_optional_metadata(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Vec<tcl_registry::InvocationFacts> {
        let mut resolved = Vec::new();
        self.for_each_resolved_command_words(words, namespace, |target, invocation_words| {
            if !target.registry_backed {
                return;
            }
            if let Some(facts) =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    registry,
                    context.map(crate::registry_invocation::InvocationMetadataContext::context),
                    invocation_words,
                    self.baseline.invocation_realm,
                )
                .resolved()
                .map(|invocation| invocation.facts())
            {
                resolved.push(facts);
            }
        });
        resolved
    }

    /// Project every registry-backed variable value write a direct statement
    /// may perform after command binding and alias-prefix resolution.
    ///
    /// This is the compiler's single bridge from the module command lattice
    /// to the registry-owned write projection.  In particular, an alias such
    /// as `interp alias {} put {} set x` contributes the literal `x` before
    /// the source call's words, while an alias to a retained user procedure
    /// never borrows a same-named registry command's semantics.
    #[must_use]
    pub(crate) fn variable_write_projection(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableWriteProjection {
        self.variable_write_projection_with_metadata_context(
            stmt,
            registry,
            namespace,
            registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                .map(Into::into),
        )
    }

    /// Conditional source footprint using the supplied availability generation.
    /// Missing or foreign metadata widens; original alias prefixes retain their
    /// values without certifying a variable write or Native frame entry.
    pub(crate) fn variable_write_projection_with_metadata_context(
        &self,
        stmt: &Statement,
        registry: &CommandRegistry,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> tcl_registry::VariableWriteProjection {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return tcl_registry::VariableWriteProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableWriteProjection::default()
            };
        };
        let literal_head = match stmt {
            Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. } => {
                tokens.as_ref().is_none_or(|tokens| {
                    tokens.synthetic.is_none()
                        && tokens.words().first().is_none_or(|head| {
                            crate::registry_invocation::invocation_word(head)
                                .literal()
                                .is_some()
                        })
                })
            }
            _ => return tcl_registry::VariableWriteProjection::default(),
        };
        if !literal_head {
            // The ordinary computed-command dispatch path owns this barrier;
            // do not manufacture a second variable-frame effect here.
            return tcl_registry::VariableWriteProjection::default();
        }

        let realm = match stmt
            .tokens()
            .and_then(|tokens| tokens.source_binding.as_ref())
        {
            Some(binding) => {
                let Some(realm) = binding.invocation_realm() else {
                    return tcl_registry::VariableWriteProjection {
                        opaque_variable_frame: true,
                        ..tcl_registry::VariableWriteProjection::default()
                    };
                };
                realm
            }
            None => self.baseline.invocation_realm,
        };
        let mut projection = tcl_registry::VariableWriteProjection::default();
        self.for_each_resolved_invocation(stmt, namespace, |target, words| {
            if !target.registry_backed {
                return;
            }
            let candidate = registry.variable_write_projection_in_resolved_context(
                context.context(),
                words,
                realm,
            );
            projection.opaque_variable_frame |= candidate.opaque_variable_frame;
            for name in candidate.literal_names {
                if !projection.literal_names.contains(&name) {
                    projection.literal_names.push(name);
                }
            }
        });
        projection
    }

    /// Conditional variable writes from one original embedded command vector.
    /// The terminal source target composes retained alias prefixes before the
    /// Registry footprint; unavailable metadata/targets remain opaque.
    pub(crate) fn variable_write_projection_for_command_words_with_metadata_context(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableWriteProjection {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return tcl_registry::VariableWriteProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableWriteProjection::default()
            };
        };
        let Some(head) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return tcl_registry::VariableWriteProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableWriteProjection::default()
            };
        };
        let mut projection = tcl_registry::VariableWriteProjection {
            opaque_variable_frame: self.target_resolution_may_be_unknown(head, namespace),
            ..tcl_registry::VariableWriteProjection::default()
        };
        self.for_each_resolved_command_words(words, namespace, |target, invocation| {
            if !target.registry_backed {
                return;
            }
            let candidate = registry.variable_write_projection_in_resolved_context(
                context.context(),
                invocation,
                self.baseline.invocation_realm,
            );
            projection.opaque_variable_frame |= candidate.opaque_variable_frame;
            for name in candidate.literal_names {
                if !projection.literal_names.contains(&name) {
                    projection.literal_names.push(name);
                }
            }
            for name in candidate.read_before_write_names {
                if !projection.read_before_write_names.contains(&name) {
                    projection.read_before_write_names.push(name);
                }
            }
        });
        projection
    }

    /// Join possible source effects from an alternative of this same retained
    /// world. This grants no execution; the existing baseline invariant applies.
    pub(crate) fn join_possible_source_effects(&mut self, other: &Self) {
        self.join(other);
    }

    /// Selected by-name reads under complete supplied availability and aliases.
    /// Missing metadata or target alternatives retain an opaque residual.
    pub(crate) fn variable_read_projection_for_command_words_with_metadata_context(
        &self,
        words: &[crate::ir_helpers::CommandWord],
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> tcl_registry::VariableReadProjection {
        let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
            return tcl_registry::VariableReadProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableReadProjection::default()
            };
        };
        let Some(head) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return tcl_registry::VariableReadProjection {
                opaque_variable_frame: true,
                ..tcl_registry::VariableReadProjection::default()
            };
        };
        let mut projection = tcl_registry::VariableReadProjection {
            opaque_variable_frame: self.target_resolution_may_be_unknown(head, namespace),
            ..tcl_registry::VariableReadProjection::default()
        };
        self.for_each_resolved_command_words(words, namespace, |target, invocation| {
            if !target.registry_backed {
                return;
            }
            let candidate = registry.variable_read_projection_in_resolved_context(
                context.context(),
                invocation,
                self.baseline.invocation_realm,
            );
            projection.opaque_variable_frame |= candidate.opaque_variable_frame;
            for name in candidate.literal_names {
                if !projection.literal_names.contains(&name) {
                    projection.literal_names.push(name);
                }
            }
        });
        projection
    }

    /// Every source spelling explicitly represented by the closed may-state.
    #[must_use]
    pub(crate) fn source_spellings(&self) -> Vec<String> {
        let mut names: Vec<_> = self
            .bindings
            .keys()
            .filter_map(|key| self.callable_spelling_for_key(key))
            .collect();
        names.sort_unstable();
        names
    }

    /// Current changed cells and retained mutation history exclude the
    /// independently captured initial command table. A restored slot remains
    /// in the history even when its current implementation matches the entry.
    fn changed_command_spellings(&self) -> impl Iterator<Item = String> + '_ {
        self.rebound_names
            .iter()
            .chain(self.bindings.iter().filter_map(|(key, observed)| {
                let original = self
                    .original_entry_bindings
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| {
                        Self::unmodified_bindings(key, self.baseline.semantics.binding_names())
                    });
                (*observed != original).then_some(key)
            }))
            .filter_map(|key| self.callable_spelling_for_key(key))
            .chain(self.namespace_resolution.rebound_names.iter().cloned())
    }

    fn callable_spelling_for_key(&self, key: &SourceCommandKey) -> Option<String> {
        match key {
            SourceCommandKey::Authored(text) => Some(text.clone()),
            SourceCommandKey::Slot { namespace, simple } => {
                let protocol = self
                    .baseline
                    .native_entry
                    .as_ref()?
                    .command_name_policy()?
                    .recipe();
                let slot = tcl_core_types::NativeByteCommandSlot::new(
                    namespace.exact_native_path()?.clone(),
                    simple.clone(),
                );
                tcl_syntax::naming::native_command_source_spelling(protocol, &slot)
            }
        }
    }

    /// Whether any command spelling may have been affected by an unbounded
    /// command-table mutation.
    #[must_use]
    pub(crate) const fn has_opaque_domain(&self) -> bool {
        self.opaque_domain
    }

    /// The commands the analysed document declares as plain calls, which
    /// this state's command table binds beside the registry's names, with the
    /// frame effect each declaration states ([`Module::declared_frame_effects`]).
    #[must_use]
    pub(crate) fn declared_frame_effects(&self) -> Arc<crate::ir::DeclaredFrameEffects> {
        Arc::clone(&self.baseline.declared)
    }

    /// Whether this state holds a command table to resolve against: the
    /// registry's baseline of the analysed module. A state built for no module
    /// (`Default`) names no command, so it can say nothing about one.
    #[must_use]
    pub(crate) fn holds_a_command_table(&self) -> bool {
        !self.baseline.semantics.binding_names().is_empty()
    }

    /// Whether a `rename`, an `interp alias` or a command delete moved a name
    /// this lattice cannot name, so that no spelling can be claimed to denote
    /// what it did before ([`Self::unnameable_rebinding_subject`]).
    #[must_use]
    pub(crate) const fn has_unnameable_rebinding_subject(&self) -> bool {
        self.unnameable_rebinding_subject
    }

    /// Project this already-computed binding summary into the optimiser's
    /// flow-insensitive command-binding trust view.
    ///
    /// This is the allocation boundary for consumers that already retain a
    /// [`ModuleCommandBindings`]: they must not repeat the module walk via
    /// [`scan_module_command_mutations`]. The published state includes every
    /// transient executable binding, so a rename followed by a restore still
    /// distrusts the affected builtin. Registry-declared namespace effects are
    /// recorded by the same resolved invocation walk, including alias prefixes
    /// and recovered executable roots.
    #[must_use]
    pub(crate) fn mutation_projection(&self, registry: &CommandRegistry) -> ModuleCommandMutations {
        let mut names = std::collections::HashSet::new();
        for (key, observed) in self.bindings.iter() {
            let Some(name) = self.callable_spelling_for_key(key) else {
                continue;
            };
            let original = self
                .original_entry_bindings
                .get(key)
                .cloned()
                .unwrap_or_else(|| {
                    Self::unmodified_bindings(key, self.baseline.semantics.binding_names())
                });
            if *observed == original {
                continue;
            }
            if default_binding(&name, registry).kind == BindingKind::Builtin {
                names.insert(name.clone());
            } else if let Some(shadowed) = builtin_shadowed_by_qualified_definition(&name, registry)
            {
                // The same tail projection the source scan applies, so a
                // qualified shadow installed through a recovered binding is
                // distrusted too. An alias carrying the prepended name —
                // `interp alias {} make {} proc ::n::expr` then
                // `make {s} {return "SHADOW:$s"}` — defines `::n::expr`
                // without the source scan ever seeing that spelling, and
                // O110 would still rewrite an unqualified `expr` inside
                // `::n` (#2159).
                names.insert(shadowed);
            }
        }
        ModuleCommandMutations {
            names,
            rebound: self
                .rebound_names
                .iter()
                .filter_map(|key| self.callable_spelling_for_key(key))
                .chain(self.namespace_resolution.rebound_names.iter().cloned())
                .collect(),
            dynamic: self.opaque_binding_mutation
                || self.namespace_resolution.dynamic
                || self.has_redefined_procedures,
            // The narrowest of the three binding-opacity flags, deliberately.
            // `opaque_binding_mutation` also rises for an opaque substitution,
            // a non-literal head, an unresolvable head namespace and the walk
            // depth cap; `dynamic_proc_binding` adds a runtime-selected
            // executable root (every `oo` method body) and a widened binding
            // domain. None of those move a binding, and gating folds on them
            // is the cost #2164 measured and refused. Only a transition that
            // moved a subject this lattice could not name may withdraw the
            // claim that an untouched name still denotes its builtin.
            rebinding_subjects: RebindingSubjects::from_unnameable(
                self.unnameable_rebinding_subject,
            ),
            // This projection is about names the closed lattice observed, not
            // about which frame observed them; the frame fact is the scan's.
            runtime_selected_frames: false,
            resolution_changed: self.namespace_resolution.changed,
            opaque_namespaces: self
                .namespace_resolution
                .opaque_namespaces
                .iter()
                .cloned()
                .collect(),
        }
    }

    /// Project the prepared lattice into the narrower trust fact used when a
    /// call site wants to seed a retained procedure's parameters: only an
    /// explicit rebinding or a dynamic command-binding transition
    /// disqualifies the declared procedure identity; source/lookup/body
    /// opacity alone does not.
    #[must_use]
    pub(crate) fn proc_binding_trust_projection(&self) -> ProcBindingTrustProjection {
        ProcBindingTrustProjection {
            rebound: self.proc_rebound_names.iter().cloned().collect(),
            dynamic: self.dynamic_proc_binding || self.has_redefined_procedures,
        }
    }

    /// Whether executing `script` itself introduces an unbounded command
    /// binding effect, excluding opacity inherited from unrelated module
    /// roots. Effect-summary consumers use this delta query instead of
    /// projecting one root's uncertainty onto every procedure.
    #[must_use]
    pub(crate) fn script_has_opaque_binding_effect(
        &self,
        script: &Script,
        registry: &CommandRegistry,
        namespace: &str,
    ) -> bool {
        let mut baseline = self.clone();
        baseline.opaque_domain = false;
        baseline.opaque_binding_mutation = false;
        baseline.dynamic_proc_binding = false;
        baseline.unnameable_rebinding_subject = false;
        baseline.bindings.clone_from(&self.root_boundary_bindings);
        let mut retained_roots = RetainedBindingRoots::default();
        let execution_namespace = crate::ir::ExecutionNamespace::exact(namespace);
        let outcome = {
            let mut context = BindingWalkContext {
                registry,
                retained_roots: &mut retained_roots,
                timeline: None,
                source_order_mode: false,
            };
            collect_binding_states(script, &mut context, &baseline, &execution_namespace)
        };
        outcome.post.opaque_binding_mutation || outcome.observed.opaque_binding_mutation
    }

    #[cfg(test)]
    fn rebound_names(&self) -> impl Iterator<Item = &str> {
        self.rebound_names
            .iter()
            .filter_map(SourceCommandKey::authored_spelling)
    }

    /// Resolve every terminal target a source spelling may invoke in
    /// `namespace`, including arguments prepended by alias chains.
    #[must_use]
    pub(crate) fn targets(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> BTreeSet<ResolvedCommandTarget> {
        self.resolve_targets(name, namespace, &mut BTreeSet::new())
    }

    /// A just-published declaration key is already constructed; resolving it
    /// again as written syntax could select a different namespace object.
    fn installed_procedure_targets(
        &self,
        key: &(impl CommandKeyQuery + ?Sized),
        declaration: u32,
    ) -> Vec<ResolvedCommandTarget> {
        self.bindings
            .get(key)
            .into_iter()
            .flatten()
            .filter_map(|binding| match binding {
                MayBinding::Target(target)
                    if target.terminal
                        && target.kind == BindingKind::Proc
                        && target.implementation_generation == declaration =>
                {
                    Some(target.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// Whether invoking `name` in `namespace` may reach an implementation that
    /// cannot be named statically.
    #[must_use]
    pub(crate) fn target_may_be_unknown(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> bool {
        self.has_opaque_domain() || self.target_resolution_may_be_unknown(name, namespace)
    }

    /// Whether this exact source spelling's resolved binding may be unknown,
    /// excluding unrelated module-wide opacity. Semantic effect consumers use
    /// this narrower query; runtime provenance still consumes
    /// [`Self::target_may_be_unknown`] and [`Self::has_opaque_domain`].
    #[must_use]
    pub(crate) fn target_resolution_may_be_unknown(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
    ) -> bool {
        self.source_keys_checked(name, namespace).is_err()
            || self.resolve_target_may_be_unknown(name, namespace, &mut BTreeSet::new())
    }

    /// Original-word candidates in the exact retained namespace domain. A
    /// later native fallback is validated only after earlier slots permit it.
    fn original_source_keys_for_paths(
        &self,
        paths: Vec<Vec<SourceCommandKey>>,
    ) -> Result<
        Vec<SourceCommandKey>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        let mut keys = Vec::new();
        for path in paths {
            for (index, key) in path.iter().enumerate() {
                let bindings = self.original_bindings_for_key(key).ok_or(
                    tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
                )?;
                let absent = bindings.contains(&MayBinding::Missing);
                if (!absent || bindings.len() > 1 || index + 1 == path.len()) && !keys.contains(key)
                {
                    keys.push(key.clone());
                }
                if !absent {
                    break;
                }
            }
        }
        Ok(keys)
    }

    fn source_keys_checked<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
    ) -> Result<
        Vec<SourceCommandKey>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        let namespace = namespace.namespace_key();
        if let Some(paths) = self.receiver_native_lookup_paths(name, namespace.as_ref()) {
            return paths.map(|paths| paths.into_iter().flatten().collect());
        }
        if name.is_ascii()
            && !name.as_bytes().contains(&0)
            && let Some(policy) = self
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        {
            let paths = self
                .original_registry_command_paths(namespace.as_ref(), name, policy)
                .ok_or(
                    tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
                )?;
            return self.original_source_keys_for_paths(paths);
        }
        if !name.starts_with("::") && self.unknown_lookup_namespaces.contains(namespace.as_ref()) {
            return Err(
                tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
            );
        }
        if !matches!(namespace.as_ref(), SourceNamespaceKey::Authored(_)) {
            return self
                .native_source_lookup_paths(name, namespace.as_ref())
                .map(|paths| {
                    let mut result = Vec::new();
                    for key in paths.into_iter().flatten() {
                        if !result.contains(&key) {
                            result.push(key);
                        }
                    }
                    result
                });
        }
        let SourceNamespaceKey::Authored(namespace_name) = namespace.as_ref() else {
            return Err(
                tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
            );
        };
        let paths = if let Some(path) = self.receiver_command_lookup_path(name) {
            vec![path]
        } else {
            let default_path = BTreeSet::from([Vec::new()]);
            self.namespace_paths
                .get(namespace.as_ref())
                .unwrap_or(&default_path)
                .iter()
                .map(|path| {
                    let path = path
                        .iter()
                        .map(|key| match key {
                            SourceNamespaceKey::Authored(text) => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()?;
                    Some(
                        tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
                            namespace_name,
                            &path,
                            name,
                        ),
                    )
                })
                .collect::<Option<Vec<_>>>()
                .ok_or(
                    tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable::Namespace,
                )?
        };
        let mut keys = Vec::new();
        for candidates in paths {
            for (index, text) in candidates.iter().enumerate() {
                let key = SourceCommandKey::authored(text.clone());
                let bindings = self.bindings.get(&key).cloned().unwrap_or_else(|| {
                    Self::unmodified_bindings(&key, self.baseline.semantics.binding_names())
                });
                let absent = bindings.contains(&MayBinding::Missing);
                if (!absent || bindings.len() > 1 || index + 1 == candidates.len())
                    && !keys.contains(&key)
                {
                    keys.push(key);
                }
                if !absent {
                    break;
                }
            }
        }
        Ok(keys)
    }

    fn source_lookup_paths<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
    ) -> Vec<Vec<SourceCommandKey>> {
        let key = namespace.namespace_key();
        if let Some(paths) = self.receiver_native_lookup_paths(name, key.as_ref()) {
            return paths.unwrap_or_default();
        }
        if name.is_ascii()
            && !name.as_bytes().contains(&0)
            && let Some(policy) = self
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        {
            return self
                .original_registry_command_paths(key.as_ref(), name, policy)
                .unwrap_or_default();
        }
        if !matches!(key.as_ref(), SourceNamespaceKey::Authored(_)) {
            return self
                .native_source_lookup_paths(name, key.as_ref())
                .unwrap_or_default();
        }
        let SourceNamespaceKey::Authored(namespace) = key.as_ref() else {
            return Vec::new();
        };
        if let Some(path) = self.receiver_command_lookup_path(name) {
            return vec![path.into_iter().map(SourceCommandKey::authored).collect()];
        }
        let default_path = BTreeSet::from([Vec::new()]);
        self.namespace_paths
            .get(key.as_ref())
            .unwrap_or(&default_path)
            .iter()
            .map(|path| {
                let path = path
                    .iter()
                    .filter_map(|context| match context {
                        SourceNamespaceKey::Authored(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
                    namespace, &path, name,
                )
                .into_iter()
                .map(SourceCommandKey::authored)
                .collect()
            })
            .collect()
    }

    fn source_keys<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
    ) -> Vec<SourceCommandKey> {
        self.source_keys_checked(name, namespace)
            .unwrap_or_default()
    }

    /// Conditional unresolved dispatch against this exact original source table.
    /// Historical missing alternatives do not erase a named module target.
    pub(crate) fn may_dispatch_unresolved<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
    ) -> bool {
        self.dispatches_unresolved(name, namespace, &mut BTreeSet::new())
    }
    fn dispatches_unresolved<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        name: &str,
        namespace: &Q,
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> bool {
        let Ok(keys) = self.source_keys_checked(name, namespace) else {
            return true;
        };
        let mut named = false;
        let mut unresolved = false;
        for key in keys {
            if !visiting.insert(key.clone()) {
                continue;
            }
            if !self.bindings.contains_key(&key)
                && key
                    .authored_spelling()
                    .is_some_and(|name| self.baseline.declared.contains_key(name))
            {
                named = true;
            }
            for binding in self.binding_alternatives(&key) {
                match binding {
                    MayBinding::Unknown => unresolved = true,
                    MayBinding::Missing => {}
                    MayBinding::Imported(import) => {
                        named = true;
                        unresolved |= self.objects.get(&import.origin).is_none_or(|alternatives| {
                            alternatives
                                .iter()
                                .any(|binding| matches!(binding, MayBinding::Unknown))
                        });
                    }
                    MayBinding::Target(target) if target.terminal => named = true,
                    MayBinding::Target(target) => {
                        named = true;
                        unresolved |= self.dispatches_unresolved(
                            &target.command,
                            &self.alias_target_namespace_key(&target, namespace),
                            visiting,
                        );
                    }
                }
            }
            visiting.remove(&key);
        }
        unresolved || !named
    }

    fn alias_target_namespace_key<Q: NamespaceKeyQuery + ?Sized>(
        &self,
        target: &ResolvedCommandTarget,
        caller: &Q,
    ) -> SourceNamespaceKey {
        if target.target_lookup == tcl_registry::AliasTargetLookup::CallerNamespace {
            return caller.namespace_key().into_owned();
        }
        self.native_root_namespace_key()
            .unwrap_or_else(|| SourceNamespaceKey::authored("::"))
    }

    fn source_root_namespace_key(&self) -> Option<SourceNamespaceKey> {
        if self.baseline.native_entry.is_some() {
            self.native_root_namespace_key()
        } else {
            Some(SourceNamespaceKey::default())
        }
    }

    fn native_root_namespace_key(&self) -> Option<SourceNamespaceKey> {
        let entry = self.baseline.native_entry.as_ref()?;
        let mut roots = entry
            .namespaces
            .iter()
            .filter(|row| row.visible && row.path.is_root());
        let root = roots.next()?;
        if roots.next().is_some() {
            return None;
        }
        runtime_entry::native_namespace_key(entry, root.token)
    }

    fn resolve_target_may_be_unknown(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> bool {
        if self.source_keys_checked(name, namespace).is_err() {
            return true;
        }
        let keys = self.source_keys(name, namespace);
        let mut unknown = false;
        for (index, key) in keys.iter().enumerate() {
            if !visiting.insert(key.clone()) {
                return true;
            }
            let bindings = self.binding_alternatives(key);
            unknown |= bindings.iter().any(|binding| match binding {
                MayBinding::Unknown => true,
                MayBinding::Imported(token) => {
                    self.runtime_execution_observed(token.compiler.as_ref())
                        || self
                            .objects
                            .get(&token.origin)
                            .is_none_or(|implementations| {
                                implementations
                                    .iter()
                                    .any(|implementation| match implementation {
                                        MayBinding::Target(target) if target.terminal => false,
                                        MayBinding::Target(target) => self
                                            .resolve_target_may_be_unknown(
                                                &target.command,
                                                &self.alias_target_namespace_key(target, namespace),
                                                visiting,
                                            ),
                                        MayBinding::Missing
                                        | MayBinding::Unknown
                                        | MayBinding::Imported(_) => true,
                                    })
                            })
                }
                MayBinding::Missing => {
                    index + 1 == keys.len()
                        && (self.namespace_unknown_handlers.contains(namespace)
                            || self.unresolved_target_may_be_unknown(name, visiting))
                }
                MayBinding::Target(target)
                    if self.runtime_execution_observed(target.token.as_ref()) =>
                {
                    true
                }
                MayBinding::Target(target) if target.terminal => false,
                MayBinding::Target(target) => self.resolve_target_may_be_unknown(
                    &target.command,
                    &self.alias_target_namespace_key(target, namespace),
                    visiting,
                ),
            });
            visiting.remove(key);
        }
        unknown
    }

    /// Whether Tcl's registry-declared unresolved-command fallback can reach
    /// an implementation whose effects are not statically bounded. A missing
    /// handler never dispatches recursively through itself.
    fn unresolved_target_may_be_unknown(
        &self,
        missing_name: &str,
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> bool {
        let missing_key = nqn(missing_name);
        let Some(root) = self.source_root_namespace_key() else {
            return true;
        };
        self.baseline
            .semantics
            .unresolved_command_handlers()
            .iter()
            .filter(|handler| **handler != missing_key)
            .any(|handler| self.resolve_target_may_be_unknown(handler, &root, visiting))
    }

    /// Resolve Tcl's registry-declared unresolved-command fallback, retaining
    /// the missing command name as the argument Tcl appends to the handler's
    /// command prefix. A missing handler is terminal failure, not recursion.
    fn unresolved_targets(
        &self,
        missing_name: &str,
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> BTreeSet<ResolvedCommandTarget> {
        let missing_key = nqn(missing_name);
        let Some(root) = self.source_root_namespace_key() else {
            return BTreeSet::new();
        };
        let mut resolved = BTreeSet::new();
        for handler in self
            .baseline
            .semantics
            .unresolved_command_handlers()
            .iter()
            .filter(|handler| **handler != missing_key)
        {
            for mut terminal in self.resolve_targets(handler, &root, visiting) {
                terminal.prepended.push(
                    crate::registry_invocation::EffectiveInvocationWord::Literal(
                        missing_name.to_owned(),
                    ),
                );
                resolved.insert(terminal);
            }
        }
        resolved
    }

    fn resolve_targets(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        visiting: &mut BTreeSet<SourceCommandKey>,
    ) -> BTreeSet<ResolvedCommandTarget> {
        self.resolve_targets_in(name, namespace, visiting, CommandTargetLookup::WithFallback)
    }

    /// Named-slot lookup and executable fallback share ordering and alias
    /// traversal. A fallback handler is not the written command's operation
    /// and cannot supply original compilation-layout advice.
    fn resolve_targets_in(
        &self,
        name: &str,
        namespace: &(impl NamespaceKeyQuery + ?Sized),
        visiting: &mut BTreeSet<SourceCommandKey>,
        lookup: CommandTargetLookup,
    ) -> BTreeSet<ResolvedCommandTarget> {
        let mut resolved = BTreeSet::new();
        let keys = self.source_keys(name, namespace);
        for (index, key) in keys.iter().enumerate() {
            if !visiting.insert(key.clone()) {
                continue;
            }
            let bindings = self.binding_alternatives(key);
            for binding in &bindings {
                match binding {
                    MayBinding::Imported(token) => {
                        if let Some(implementations) = self.objects.get(&token.origin) {
                            for implementation in implementations {
                                if let MayBinding::Target(target) = implementation {
                                    if target.terminal {
                                        resolved.insert(target.clone());
                                    } else {
                                        for mut terminal in self.resolve_targets_in(
                                            &target.command,
                                            &self.alias_target_namespace_key(target, namespace),
                                            visiting,
                                            lookup,
                                        ) {
                                            terminal
                                                .prepended
                                                .extend(target.prepended.iter().cloned());
                                            resolved.insert(terminal);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    MayBinding::Target(target) if target.terminal => {
                        resolved.insert(target.clone());
                    }
                    MayBinding::Target(target) => {
                        for mut terminal in self.resolve_targets_in(
                            &target.command,
                            &self.alias_target_namespace_key(target, namespace),
                            visiting,
                            lookup,
                        ) {
                            terminal.prepended.extend(target.prepended.iter().cloned());
                            resolved.insert(terminal);
                        }
                    }
                    MayBinding::Missing
                        if index + 1 == keys.len()
                            && matches!(lookup, CommandTargetLookup::WithFallback)
                            && !self.namespace_unknown_handlers.contains(namespace) =>
                    {
                        resolved.extend(self.unresolved_targets(name, visiting));
                    }
                    MayBinding::Missing | MayBinding::Unknown => {}
                }
            }
            visiting.remove(key);
        }
        resolved
    }

    /// Replace sparse binding entries without detaching a shared state when
    /// every requested value is already present.
    fn replace_bindings(
        &mut self,
        bindings: impl IntoIterator<Item = (impl Into<SourceCommandKey>, BTreeSet<MayBinding>)>,
    ) -> bool {
        let replacements: Vec<_> = bindings
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .filter(|(key, value)| self.bindings.get(key) != Some(value))
            .collect();
        if replacements.is_empty() {
            return false;
        }
        // The actual object-private namespace is not a displayed command
        // name. A command-table mutation cannot prove that its injected
        // dispatcher survived merely from the class incarnation.
        Arc::make_mut(&mut self.object_instances).invalidate_receiver_dispatcher();
        Arc::make_mut(&mut self.bindings).extend(replacements);
        true
    }

    fn replace(&mut self, key: impl Into<SourceCommandKey>, bindings: BTreeSet<MayBinding>) {
        self.replace_bindings([(key, bindings)]);
    }

    /// Join one known-changed binding into the historical may-state. The
    /// ordinary lattice join handles arbitrary branches; a deterministic
    /// registry transition already identifies its exact key and should not
    /// rescan every unchanged binding merely to publish that one delta.
    fn join_binding_from(&mut self, other: &Self, key: &(impl CommandKeyQuery + ?Sized)) -> bool {
        let mut joined = self.binding_alternatives(key);
        joined.extend(other.binding_alternatives(key));
        self.replace_bindings([(key.command_key().into_owned(), joined)])
    }

    fn remove(&mut self, key: impl Into<SourceCommandKey>) {
        let key = key.into();
        self.rebound_names.insert(key.clone());
        let tokens = self
            .bindings
            .get(&key)
            .into_iter()
            .flatten()
            .filter_map(|binding| match binding {
                MayBinding::Target(target) => target.token.clone(),
                MayBinding::Imported(_) | MayBinding::Missing | MayBinding::Unknown => None,
            })
            .collect::<BTreeSet<_>>();
        for token in tokens {
            Arc::make_mut(&mut self.objects).remove(&token);
            let imported = self
                .bindings
                .iter()
                .filter_map(|(name, alternatives)| {
                    alternatives
                        .iter().any(|binding| matches!(binding, MayBinding::Imported(import) if import.origin == token))
                        .then_some(name.clone())
                })
                .collect::<Vec<_>>();
            for name in imported {
                let mut alternatives = self.bindings[&name].clone();
                alternatives.retain(|binding| !matches!(binding, MayBinding::Imported(import) if import.origin == token));
                alternatives.insert(MayBinding::Missing);
                self.replace(name, alternatives);
            }
        }
        self.replace(key, BTreeSet::from([MayBinding::Missing]));
    }

    fn allocate_command(
        &mut self,
        command: &SourceCommandKey,
        offset: u32,
    ) -> Option<CommandAllocation> {
        let site = CommandAllocationSite {
            source: Arc::clone(self.current_source_origin.as_ref()?),
            offset,
        };
        let count = Arc::make_mut(&mut self.allocation_counts)
            .entry((site.clone(), command.clone()))
            .or_default();
        *count = count.saturating_add(1).min(3);
        Some(CommandAllocation {
            site,
            incarnation: match *count {
                1 => AllocationIncarnation::First,
                2 => AllocationIncarnation::Second,
                _ => AllocationIncarnation::RepeatedFresh,
            },
            command: crate::command_binding::ModuleCommandBindings::command_key_label(command)
                .unwrap_or_default(),
            namespace: command.holder().into_owned(),
        })
    }

    fn install(&mut self, key: impl Into<SourceCommandKey>, mut implementation: MayBinding) {
        let key = key.into();
        self.rebound_names.insert(key.clone());
        let label = crate::command_binding::ModuleCommandBindings::command_key_label(&key)
            .unwrap_or_default();
        if let MayBinding::Target(target) = &mut implementation
            && (!target.registry_backed || target.kind == BindingKind::Alias)
            && target.implementation_allocation.is_none()
        {
            target.implementation_allocation =
                self.allocate_command(&key, target.implementation_generation);
            target.token = Some(CommandIdentity {
                runtime: None,
                origin: label,
                declaration: target.implementation_generation,
                allocation: target.implementation_allocation.clone(),
            });
        }
        let prior = self.binding_alternatives(&key);
        let tokens = prior
            .iter()
            .filter_map(|binding| {
                if let MayBinding::Target(target) = binding {
                    target.token.clone()
                } else {
                    None
                }
            })
            .filter(|token| !self.command_token_replaced_on_install(token, &key))
            .collect::<BTreeSet<_>>();
        for token in &tokens {
            self.remove_command_observers_for_token(token);
        }
        let mut installed = BTreeSet::new();
        for token in tokens {
            let mut existing = implementation.clone();
            if let MayBinding::Target(target) = &mut existing {
                target.token = Some(token.clone());
            }
            let mut implementations = BTreeSet::from([existing.clone()]);
            if is_repeated_fresh_binding(&existing) {
                implementations.insert(MayBinding::Unknown);
            }
            Arc::make_mut(&mut self.objects).insert(token, implementations);
            installed.insert(existing);
        }
        if installed.is_empty()
            || prior.iter().any(
                |binding| !matches!(binding, MayBinding::Target(target) if target.token.is_some()),
            )
        {
            installed.insert(implementation);
        }
        if installed.iter().any(is_repeated_fresh_binding) {
            installed.insert(MayBinding::Unknown);
        }
        self.replace(key, installed);
    }

    fn unmodified_bindings(
        key: &(impl CommandKeyQuery + ?Sized),
        initial_registry_bindings: &BTreeSet<String>,
    ) -> BTreeSet<MayBinding> {
        let key = key.command_key();
        let Some(key) = key.authored_spelling() else {
            return BTreeSet::from([MayBinding::Missing]);
        };
        if initial_registry_bindings.contains(key) {
            BTreeSet::from([MayBinding::Target(ResolvedCommandTarget {
                command: key.to_owned(),
                prepended: Vec::new(),
                registry_backed: true,
                kind: BindingKind::Builtin,
                implementation_generation: 0,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: Some(CommandIdentity {
                    runtime: None,
                    origin: key.to_owned(),
                    declaration: 0,
                    allocation: None,
                }),
                implementation_allocation: None,
            })])
        } else {
            // A name first introduced by this module did not exist on the
            // pre-transition path. Treating it as an executable self-target
            // invents a user command and makes an exact alias spuriously
            // opaque when the before/after states are joined.
            BTreeSet::from([MayBinding::Missing])
        }
    }

    /// Join two sparse states sharing one immutable registry baseline.
    ///
    /// The boolean lets callers avoid publishing an observational history
    /// entry when a transition's alternatives made no lattice change.
    fn join(&mut self, other: &Self) -> bool {
        debug_assert!(
            Arc::ptr_eq(&self.baseline, &other.baseline),
            "a binding analysis may only join states from one registry baseline"
        );
        let mut changed = self.join_activation_values(other);
        if !Arc::ptr_eq(&self.original_command_world, &other.original_command_world)
            && self.original_command_world != other.original_command_world
        {
            changed |=
                Arc::make_mut(&mut self.original_command_world).join(&other.original_command_world);
        }
        changed |= self.join_lookup_support(other);
        changed |= self.join_binding_history(other);
        if !Arc::ptr_eq(&self.bindings, &other.bindings) && self.bindings != other.bindings {
            let keys: BTreeSet<SourceCommandKey> = self
                .bindings
                .keys()
                .chain(other.bindings.keys())
                .cloned()
                .collect();
            let joined = keys.into_iter().map(|key| {
                let mut bindings = self.bindings.get(&key).cloned().unwrap_or_else(|| {
                    Self::unmodified_bindings(&key, self.baseline.semantics.binding_names())
                });
                bindings.extend(other.bindings.get(&key).cloned().unwrap_or_else(|| {
                    Self::unmodified_bindings(&key, self.baseline.semantics.binding_names())
                }));
                (key, bindings)
            });
            changed |= self.replace_bindings(joined.collect::<Vec<_>>());
        }
        changed
    }

    /// Join the activation's reachable values and construction prerequisites.
    fn join_activation_values(&mut self, other: &Self) -> bool {
        let pool = self
            .ordinary_literal_pool
            .as_ref()
            .zip(other.ordinary_literal_pool.as_ref())
            .and_then(|(left, right)| left.joined(right));
        let pool_changed = self.ordinary_literal_pool != pool;
        self.ordinary_literal_pool = pool;
        let before_objects = Arc::clone(&self.object_instances);
        Arc::make_mut(&mut self.object_instances).join(&other.object_instances);
        let mut changed = pool_changed || self.object_instances != before_objects;
        for (site, incoming) in other.allocation_counts.iter() {
            let current = self.allocation_counts.get(site).copied().unwrap_or(0);
            if *incoming > current {
                Arc::make_mut(&mut self.allocation_counts).insert(site.clone(), *incoming);
                changed = true;
            }
        }
        for (site, incoming) in other.namespace_allocation_counts.iter() {
            let current = self
                .namespace_allocation_counts
                .get(site)
                .copied()
                .unwrap_or(0);
            if *incoming > current {
                Arc::make_mut(&mut self.namespace_allocation_counts)
                    .insert(site.clone(), *incoming);
                changed = true;
            }
        }
        let before = self.class_definitions.len();
        Arc::make_mut(&mut self.class_definitions).retain(|identity, definition| {
            other.class_definitions.get(identity) == Some(definition)
        });
        changed |= before != self.class_definitions.len();
        let before = self.bounded_constructions.len();
        Arc::make_mut(&mut self.bounded_constructions).retain(|identity, construction| {
            other.bounded_constructions.get(identity) == Some(construction)
        });
        changed |= before != self.bounded_constructions.len();
        changed |= Arc::make_mut(&mut self.command_observers).join(&other.command_observers);
        let before = self.tainted_object_dispatch.len();
        Arc::make_mut(&mut self.tainted_object_dispatch)
            .extend(other.tainted_object_dispatch.iter().cloned());
        changed |= before != self.tainted_object_dispatch.len();
        if self.source_variables != other.source_variables {
            let mut joined = self.source_variables.as_ref().clone();
            joined.join(&other.source_variables);
            if *self.source_variables != joined {
                self.source_variables = Arc::new(joined);
                changed = true;
            }
        }
        if self.variable_frame != other.variable_frame
            && self.variable_frame != crate::var_resolve::VariableExecutionFrame::Unknown
        {
            self.variable_frame = crate::var_resolve::VariableExecutionFrame::Unknown;
            changed = true;
        }
        changed
    }

    /// Lookup depends on command objects, namespace alternatives and provider state.
    fn join_lookup_support(&mut self, other: &Self) -> bool {
        let mut changed = false;
        changed |= join_alternatives(&mut self.objects, &other.objects);
        changed |= Arc::make_mut(&mut self.namespace_exports)
            .join_with_default(&other.namespace_exports, &Vec::new());
        changed |= Arc::make_mut(&mut self.namespace_paths)
            .join_with_default(&other.namespace_paths, &Vec::new());
        changed |= join_alternatives_with_default(
            &mut self.packages,
            &other.packages,
            PackageProvision::absent(),
        );
        if other.loader_handler_unknown && !self.loader_handler_unknown {
            self.loader_handler_unknown = true;
            changed = true;
        }
        if !Arc::ptr_eq(&self.namespaces, &other.namespaces) {
            let before = self.namespaces.len();
            Arc::make_mut(&mut self.namespaces)
                .retain(|namespace| other.namespaces.contains(namespace));
            changed |= self.namespaces.len() != before;
        }
        for (destination, incoming) in [
            (
                &mut self.unknown_lookup_namespaces,
                &other.unknown_lookup_namespaces,
            ),
            (
                &mut self.unknown_namespace_paths,
                &other.unknown_namespace_paths,
            ),
            (
                &mut self.namespace_unknown_handlers,
                &other.namespace_unknown_handlers,
            ),
            (
                &mut self.unknown_export_namespaces,
                &other.unknown_export_namespaces,
            ),
        ] {
            let before = destination.len();
            Arc::make_mut(destination).extend(incoming.iter().cloned());
            changed |= destination.len() != before;
        }
        for (destination, incoming) in [
            (&mut self.revoked_loaders, &other.revoked_loaders),
            (
                &mut self.tainted_provider_state,
                &other.tainted_provider_state,
            ),
        ] {
            let before = destination.len();
            Arc::make_mut(destination).extend(incoming.iter().cloned());
            changed |= destination.len() != before;
        }
        changed
    }

    /// Historical opacity survives restoration of the current lookup state.
    fn join_binding_history(&mut self, other: &Self) -> bool {
        let mut changed = false;
        if other.opaque_domain && !self.opaque_domain {
            self.opaque_domain = true;
            changed = true;
        }
        if other.opaque_binding_mutation && !self.opaque_binding_mutation {
            self.opaque_binding_mutation = true;
            changed = true;
        }
        if other.dynamic_proc_binding && !self.dynamic_proc_binding {
            self.dynamic_proc_binding = true;
            changed = true;
        }
        if other.unnameable_rebinding_subject && !self.unnameable_rebinding_subject {
            self.unnameable_rebinding_subject = true;
            changed = true;
        }
        changed |= self.namespace_resolution.join(&other.namespace_resolution);
        if !Arc::ptr_eq(&self.procedure_bodies, &other.procedure_bodies)
            && self.procedure_bodies != other.procedure_bodies
        {
            changed |= self.extend_procedure_bodies(other.procedure_bodies.iter().cloned());
        }
        let rebound_count = self.rebound_names.len();
        self.rebound_names
            .extend(other.rebound_names.iter().cloned());
        changed |= self.rebound_names.len() != rebound_count;
        let proc_rebound_count = self.proc_rebound_names.len();
        self.proc_rebound_names
            .extend(other.proc_rebound_names.iter().cloned());
        changed |= self.proc_rebound_names.len() != proc_rebound_count;
        if other.has_redefined_procedures && !self.has_redefined_procedures {
            self.has_redefined_procedures = true;
            changed = true;
        }
        changed
    }
}

fn join_alternatives<K: Clone + Ord, V: Clone + Ord>(
    destination: &mut Arc<BTreeMap<K, BTreeSet<V>>>,
    incoming: &Arc<BTreeMap<K, BTreeSet<V>>>,
) -> bool {
    if destination == incoming {
        return false;
    }
    let mut changed = false;
    for (key, values) in incoming.iter() {
        let entry = Arc::make_mut(destination).entry(key.clone()).or_default();
        let previous = entry.len();
        entry.extend(values.iter().cloned());
        changed |= entry.len() != previous;
    }
    changed
}

fn join_alternatives_with_default<K: Clone + Ord, V: Clone + Ord>(
    destination: &mut Arc<BTreeMap<K, BTreeSet<V>>>,
    incoming: &Arc<BTreeMap<K, BTreeSet<V>>>,
    default: V,
) -> bool {
    if destination == incoming {
        return false;
    }
    let keys = destination
        .keys()
        .chain(incoming.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let fallback = BTreeSet::from([default]);
    let joined = keys
        .into_iter()
        .map(|key| {
            let mut values = destination
                .get(&key)
                .cloned()
                .unwrap_or_else(|| fallback.clone());
            values.extend(
                incoming
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| fallback.clone()),
            );
            (key, values)
        })
        .collect::<BTreeMap<_, _>>();
    if **destination == joined {
        return false;
    }
    *destination = Arc::new(joined);
    true
}

#[derive(Default)]
struct DiscardedProcedureHistory {
    modules: Vec<Module>,
    rebound_names: BTreeSet<String>,
    opaque: bool,
}

/// Recover the bodies which [`Module::procedures`] cannot retain when one
/// statically named procedure is defined more than once. The declaration
/// statements remain in executable IR, so typed procedure-definition
/// provenance can distinguish a readable discarded body from a genuinely
/// unavailable one. Readable bodies are lowered and walked like every retained
/// procedure body; only an unreadable history widens the whole command domain.
fn discarded_procedure_history(
    module: &Module,
    registry: &CommandRegistry,
) -> DiscardedProcedureHistory {
    fn walk(
        script: &Script,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        retained_procedures: &HashSet<(String, tcl_lexer::Span)>,
        registry: &CommandRegistry,
        occurrences: &mut HashMap<String, usize>,
        history: &mut DiscardedProcedureHistory,
        depth: u32,
    ) {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            history.opaque = true;
            return;
        }
        for stmt in &script.statements {
            if let Some((name, body)) = procedure_declaration(stmt, registry) {
                let crate::ir_helpers::ExecutionNamespace::Exact(declaration_namespace) = namespace
                else {
                    // A caller-selected frame makes an otherwise literal
                    // procedure name runtime-dependent.
                    history.opaque = true;
                    continue;
                };
                let qname = tcl_syntax::naming::qualify(declaration_namespace, &name);
                let count = occurrences.entry(qname.clone()).or_default();
                *count += 1;
                if *count > 1 {
                    history.rebound_names.insert(qname.clone());
                }
                let represented_exactly =
                    retained_procedures.contains(&(qname.clone(), stmt.span()));
                if represented_exactly {
                    // The retained body is already one of `module`'s
                    // executable roots. An unreadable source body, however,
                    // may have lowered to an empty placeholder and cannot be
                    // treated as an exact no-op.
                    if body.is_none() {
                        history.opaque = true;
                    }
                } else if let Some(body) = body {
                    let recovered = recovered_procedure_source(&body, &qname);
                    // A second generation of discarded bodies is not
                    // available from the parent module's declaration
                    // history. Preserve soundness rather than recursively
                    // guessing which nested definition survived.
                    history.opaque |= !recovered.redefined_procedures.is_empty();
                    history.modules.push(recovered);
                } else {
                    history.opaque = true;
                }
            }

            for (body, body_namespace) in
                crate::ir_helpers::nested_execution_bodies(stmt, namespace)
            {
                walk(
                    body,
                    &body_namespace,
                    retained_procedures,
                    registry,
                    occurrences,
                    history,
                    depth + 1,
                );
            }
        }
    }

    let mut history = DiscardedProcedureHistory::default();
    let retained_procedures = module
        .procedures
        .iter()
        .map(|(qname, procedure)| (qname.clone(), procedure.span))
        .collect();
    let mut occurrences = HashMap::new();
    // Keep body units here: namespace/apply barriers do not retain their body
    // as nested statement IR, so each separately lowered body unit is the one
    // place this declaration-history scan can see definitions inside it.
    for (script, namespace) in module.executable_script_roots() {
        walk(
            script,
            &namespace,
            &retained_procedures,
            registry,
            &mut occurrences,
            &mut history,
            0,
        );
    }
    history
}

fn procedure_declaration(
    stmt: &Statement,
    registry: &CommandRegistry,
) -> Option<(String, Option<String>)> {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::hooks::LoweringHookId;

    let (Statement::Call {
        command,
        args,
        tokens,
        ..
    }
    | Statement::Barrier {
        command,
        args,
        tokens,
        ..
    }) = stmt
    else {
        return None;
    };
    let facts = invocation_facts(stmt, registry)?;
    if facts.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Proc) {
        return None;
    }
    let name = facts
        .state_transitions
        .declared()?
        .command_bindings()
        .find_map(|transition| match transition {
            CommandBindingTransition::Define { name, kind }
                if *kind == CommandBindingDefinitionKind::Procedure =>
            {
                name.literal()
            }
            _ => None,
        })?;
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let body_index = registry
        .arg_indices_for_role(command, &arg_refs, tcl_registry::ArgRole::Body)
        .into_iter()
        .next();
    let body = body_index.and_then(|index| {
        tokens
            .as_ref()?
            .words()
            .get(index + 1)
            .and_then(|word| crate::registry_invocation::invocation_word(word).literal())
            .map(str::to_owned)
    });
    Some((name.to_owned(), body))
}

fn procedure_namespace(qname: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
    if holder.is_empty() {
        "::".to_owned()
    } else {
        holder.to_owned()
    }
}

fn recovered_procedure_source(body: &str, qname: &str) -> Module {
    Module {
        source: tcl_lexer::SourceImage::document(body),
        top_level_namespace: procedure_namespace(qname),
        ..Module::default()
    }
}

struct BindingWalkOutcome {
    /// State reachable after the script completes.
    post: ModuleCommandBindings,
    /// Union of states observed after executable invocations within the
    /// script. This is queryable history, never a replay entry state.
    observed: ModuleCommandBindings,
}

/// Source-order command states for one executable script root. The two
/// snapshots distinguish substitutions, which run before the host command,
/// from the direct invocation itself.
#[derive(Debug, Clone, Default)]
pub(crate) struct SourceBindingTimeline {
    states: HashMap<tcl_lexer::Span, StatementBindingStates>,
    /// State after this source root completes. This is not historical
    /// observation: a procedure defined late in the root can begin from it,
    /// while a procedure defined earlier joins it with the suffix it could
    /// have observed after becoming callable.
    post: Option<ModuleCommandBindings>,
    /// Cache suffix joins once for all procedure definitions in a source root.
    /// Replaying every suffix separately makes large multi-proc files cubic.
    suffix_states: std::sync::OnceLock<Vec<(u32, ModuleCommandBindings)>>,
}

#[derive(Debug, Clone)]
pub(crate) struct StatementBindingStates {
    before_substitutions: ModuleCommandBindings,
    before_direct_call: Option<ModuleCommandBindings>,
}

impl SourceBindingTimeline {
    /// States a procedure may enter after its defining command at
    /// `definition_span` has run.
    ///
    /// Source snapshots before that definition are deliberately excluded: a
    /// later `proc eval` has replaced the builtin before a later `proc p`
    /// first becomes callable. The source suffix and final post-state retain
    /// temporary and terminal transitions that occur after `p` is available.
    /// The closed root-boundary state adds independently executable roots
    /// without replaying the module's earlier historical observations.
    pub(crate) fn entry_after(
        &self,
        definition_span: tcl_lexer::Span,
        module: &ModuleCommandBindings,
    ) -> Option<ModuleCommandBindings> {
        let post = self.post.as_ref()?;
        let suffixes = self.suffix_states.get_or_init(|| {
            let mut by_start: BTreeMap<u32, Vec<&StatementBindingStates>> = BTreeMap::new();
            for (span, states) in &self.states {
                by_start.entry(span.start()).or_default().push(states);
            }
            let mut suffix = post.clone();
            let mut snapshots = Vec::with_capacity(by_start.len());
            for (start, states) in by_start.into_iter().rev() {
                for states in states {
                    if !suffix.same_state(&states.before_substitutions) {
                        suffix.join(&states.before_substitutions);
                    }
                    if let Some(before_call) = &states.before_direct_call
                        && !suffix.same_state(before_call)
                    {
                        suffix.join(before_call);
                    }
                }
                snapshots.push((start, suffix.clone()));
            }
            snapshots.reverse();
            snapshots
        });
        let next = suffixes.partition_point(|(start, _)| *start <= definition_span.start());
        let mut entry = suffixes.get(next).map_or(post, |(_, state)| state).clone();
        let mut boundary = module.clone();
        boundary.bindings = Arc::clone(&module.root_boundary_bindings);
        if !entry.same_state(&boundary) {
            entry.join(&boundary);
        }
        Some(entry)
    }

    fn join_region(&mut self, script: &Script, state: &ModuleCommandBindings) {
        crate::ir::for_each_statement(script, &mut |stmt| {
            self.record_before_substitutions(stmt.span(), state);
            self.record_before_direct_call(stmt.span(), state);
        });
    }

    fn record_before_substitutions(
        &mut self,
        span: tcl_lexer::Span,
        state: &ModuleCommandBindings,
    ) {
        self.record(span, state, true);
    }

    fn record_before_direct_call(&mut self, span: tcl_lexer::Span, state: &ModuleCommandBindings) {
        self.record(span, state, false);
    }

    fn record(
        &mut self,
        span: tcl_lexer::Span,
        state: &ModuleCommandBindings,
        substitutions: bool,
    ) {
        self.suffix_states.take();
        if let Some(existing) = self.states.get_mut(&span) {
            if substitutions {
                if !existing.before_substitutions.same_state(state) {
                    existing.before_substitutions.join(state);
                }
            } else {
                match &mut existing.before_direct_call {
                    Some(before_direct_call) => {
                        if !before_direct_call.same_state(state) {
                            before_direct_call.join(state);
                        }
                    }
                    None => existing.before_direct_call = Some(state.clone()),
                }
            }
        } else {
            self.states.insert(
                span,
                StatementBindingStates {
                    before_substitutions: state.clone(),
                    before_direct_call: (!substitutions).then(|| state.clone()),
                },
            );
        }
    }

    pub(crate) fn before_substitutions(
        &self,
        span: tcl_lexer::Span,
    ) -> Option<&ModuleCommandBindings> {
        self.states
            .get(&span)
            .map(|states| &states.before_substitutions)
    }

    pub(crate) fn before_direct_call(
        &self,
        span: tcl_lexer::Span,
    ) -> Option<&ModuleCommandBindings> {
        self.states
            .get(&span)
            .and_then(|states| states.before_direct_call.as_ref())
    }
}

/// Shared ownership for a binding walk. Normal historical analysis leaves the
/// timeline absent; source-order projection records statement snapshots.
struct BindingWalkContext<'a> {
    registry: &'a CommandRegistry,
    retained_roots: &'a mut RetainedBindingRoots,
    timeline: Option<&'a mut SourceBindingTimeline>,
    source_order_mode: bool,
}

/// Invocation-local walk state shared by direct and readable-body transfers.
struct InvocationBindingContext<'a> {
    retained_roots: &'a mut RetainedBindingRoots,
    source_order_mode: bool,
    metadata: BindingInvocationMetadata<'a>,
}

/// Explicit compatibility is separate from a supplied metadata refusal.
#[derive(Clone, Copy)]
enum BindingInvocationMetadata<'a> {
    Standalone,
    Supplied(Option<crate::registry_invocation::InvocationMetadataContext<'a>>),
}

fn interpret_binding_source(
    source: &str,
    offset: u32,
    namespace: &str,
    frame: &crate::var_resolve::VariableExecutionFrame,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    retained: &mut RetainedBindingRoots,
) -> BindingWalkOutcome {
    let config = registry
        .profile()
        .map_or_else(tcl_lexer::LexerConfig::default, |profile| {
            tcl_lexer::LexerConfig::from_grammar(profile.grammar)
        });
    let mut interpretation = SourceCommandBindings {
        deferred: retained.source_roots.clone().into(),
        ..SourceCommandBindings::default()
    };
    let mut post = bindings.clone();
    interpretation.walk_source(
        source,
        offset,
        &mut post,
        &SourceExecutionContext {
            realm: bindings.baseline.invocation_realm,
            compilation: if matches!(
                frame.layout(),
                crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                    | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
            ) {
                procedure_compilation(bindings.baseline.compilation_dialect())
            } else {
                bindings.baseline.native_compilation
            },
            compilation_snapshot: None,
            selected_compilation: None,
            original_variable_compilation: None,
            namespace,
            namespace_key: frame.namespace_identity(),
            config,
            registry,
            frame,
            depth: 0,
            invocation_offset: 0,
            variable_read_owner: None,
            original_written_projection: None,
            written_arguments: None,
            written_values: None,
            written_name_values: None,
            written_representations: None,
            written_objects: None,
            written_method_prefixes: None,
            written_variable_reads: None,
            expression_source: None,
        },
    );
    let mut observed = post.clone();
    for point in &interpretation.points {
        observed.join(&point.state);
    }
    retained.source_roots.extend(interpretation.deferred);
    BindingWalkOutcome { post, observed }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RetainedBindingRoot {
    /// A recovered body must outlive the temporary `Module` used to lower
    /// it, but fixpoint snapshots need only duplicate a handle, not its full
    /// IR tree.
    script: Arc<Script>,
    namespace: crate::ir::ExecutionNamespace,
}

/// Monotone inventory of callable roots discovered while replaying exact,
/// readable Tcl. It is intentionally separate from [`ModuleCommandBindings`]:
/// roots control the fixpoint worklist, while the published lattice and memo
/// identity contain only observable command-table state.
#[derive(Default)]
struct RetainedBindingRoots {
    source_roots: BTreeMap<DeferredImplementationId, DeferredSourceBody>,
    roots: Vec<RetainedBindingRoot>,
    seen: HashSet<RetainedBindingRoot>,
    evaluated_bodies: HashMap<crate::ir::ExecutionNamespace, Vec<RetainedEvaluatedBody>>,
}

struct RetainedEvaluatedBody {
    span: tcl_lexer::Span,
    source: String,
    script: Arc<crate::ir::Script>,
}

impl RetainedBindingRoots {
    fn insert(&mut self, script: &Script, namespace: crate::ir::ExecutionNamespace) {
        let root = RetainedBindingRoot {
            script: Arc::new(script.clone()),
            namespace,
        };
        if self.seen.insert(root.clone()) {
            self.roots.push(root);
        }
    }

    /// Retain every non-top root, or every root when `skip_top` is false.
    fn extend_module_roots(&mut self, module: &Module, skip_top: bool) {
        if !skip_top
            && module.top_level.statements.is_empty()
            && !module.source.is_empty()
            && let Ok(source) = module.source.try_text()
        {
            let namespace = if module.top_level_namespace.is_empty() {
                "::"
            } else {
                &module.top_level_namespace
            };
            let identity = format!("recovered:{namespace}:{source}");
            self.source_roots.insert(
                DeferredImplementationId {
                    command: identity.clone(),
                    generation: 0,
                    allocation: None,
                },
                DeferredSourceBody {
                    realm: module.source_entry.invocation_realm,
                    identity,
                    implementation_generation: 0,
                    source: source.to_owned(),
                    offset: 0,
                    namespace: namespace.to_owned(),
                    namespace_key: module
                        .top_level_namespace_context
                        .clone()
                        .unwrap_or_else(|| SourceNamespaceKey::authored(namespace)),
                    event: None,
                    receiver_method: false,
                    future_frame: None,
                    parameters: Vec::new(),
                    original_parameters: None,
                    statics: None,
                    implementation_allocation: None,
                    source_origin: None,
                    executed_script: None,
                    executed_word: None,
                },
            );
        }
        // Evaluated bodies keep their owning source location so identical Tcl
        // text lowered under different command states cannot collide. The
        // binding walk selects one only from inside the invocation currently
        // being replayed.
        let mut body_units: Vec<_> = module.body_units.iter().collect();
        body_units.sort_by(|a, b| {
            a.1.span
                .start()
                .cmp(&b.1.span.start())
                .then_with(|| a.0.cmp(b.0))
        });
        for (qname, unit) in body_units {
            let Ok(start) = usize::try_from(unit.span.start()) else {
                continue;
            };
            let Ok(end) = usize::try_from(unit.span.end()) else {
                continue;
            };
            let Some(source) = module
                .source
                .try_text()
                .ok()
                .and_then(|source| source.get(start..end))
            else {
                continue;
            };
            let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
            let namespace =
                crate::ir::ExecutionNamespace::exact(if holder.is_empty() { "::" } else { holder });
            self.evaluated_bodies
                .entry(namespace)
                .or_default()
                .push(RetainedEvaluatedBody {
                    span: unit.span,
                    source: source.to_owned(),
                    script: Arc::new(unit.body.clone()),
                });
        }
        for (script, namespace) in module
            .independent_executable_script_roots()
            .into_iter()
            .skip(usize::from(skip_top))
        {
            self.insert(script, namespace);
        }
    }

    fn evaluated_body(
        &self,
        source: &str,
        namespace: &crate::ir::ExecutionNamespace,
        invocation_span: tcl_lexer::Span,
    ) -> Option<Arc<crate::ir::Script>> {
        self.evaluated_bodies
            .get(namespace)?
            .iter()
            .find_map(|body| {
                (body.source == source
                    && body.span.start() >= invocation_span.start()
                    && body.span.end() <= invocation_span.end())
                .then(|| Arc::clone(&body.script))
            })
    }

    fn len(&self) -> usize {
        self.roots.len() + self.source_roots.len()
    }

    fn snapshot(&self) -> Vec<RetainedBindingRoot> {
        self.roots.clone()
    }
}

fn observe_binding_state(
    observed: &mut Option<ModuleCommandBindings>,
    current: &ModuleCommandBindings,
) {
    if let Some(state) = observed {
        // A call with no command-table transition is still an executable
        // observation (the caller's final fallback covers that state), but
        // it must not repeatedly merge an identical sparse lattice point.
        if !state.same_state(current) {
            state.join(current);
        }
    } else {
        *observed = Some(current.clone());
    }
}

// Flow-sensitive recursive join over every structured IR form.
fn collect_normal_binding_states(
    script: &Script,
    registry: &CommandRegistry,
    initial: &ModuleCommandBindings,
    namespace: &crate::ir::ExecutionNamespace,
    retained_roots: &mut RetainedBindingRoots,
) -> BindingWalkOutcome {
    let mut context = BindingWalkContext {
        registry,
        retained_roots,
        timeline: None,
        source_order_mode: false,
    };
    collect_binding_states(script, &mut context, initial, namespace)
}

#[allow(clippy::too_many_lines)]
fn collect_binding_states(
    script: &Script,
    context: &mut BindingWalkContext<'_>,
    initial: &ModuleCommandBindings,
    namespace: &crate::ir::ExecutionNamespace,
) -> BindingWalkOutcome {
    // Recursive walker keeps branch joins and side effects together.
    #[allow(clippy::too_many_lines)]
    fn walk(
        script: &Script,
        context: &mut BindingWalkContext<'_>,
        current: &mut ModuleCommandBindings,
        observed: &mut Option<ModuleCommandBindings>,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        depth: u32,
    ) {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            current.mark_opaque_binding_mutation();
            observe_binding_state(observed, current);
            return;
        }
        // Typed-lowering sites are sparse sidecars, but a large generated
        // script can have many statements and many sites. Index them once for
        // this script instead of scanning every site for every statement.
        // Keep the index borrowed: a repeated synthetic statement span still
        // observes every site attached to that source command, exactly as the
        // former filter loop did.
        let mut binding_sites_by_span: HashMap<_, Vec<_>> = HashMap::new();
        for site in script.command_binding_sites.iter() {
            binding_sites_by_span
                .entry(site.span)
                .or_default()
                .push(site);
        }
        let statement_spans: HashSet<_> = script.statements.iter().map(Statement::span).collect();
        for stmt in &script.statements {
            if !stmt.is_executable_invocation() {
                continue;
            }
            let source_order_entry = context.source_order_mode.then(|| current.clone());
            if let Some(timeline) = &mut context.timeline {
                timeline.record_before_substitutions(stmt.span(), current);
            }
            let conditional_substitutions = context.source_order_mode
                && crate::ir_helpers::evaluated_command_substitution_surfaces(
                    stmt,
                    context.registry,
                )
                .conditional;
            if apply_embedded_transitions(
                stmt,
                context.registry,
                current,
                namespace,
                context.source_order_mode,
            ) {
                observe_binding_state(observed, current);
            }
            if conditional_substitutions
                && source_order_entry
                    .as_ref()
                    .is_some_and(|before| !before.same_state(current))
            {
                // `&&`, `||`, ternary arms, and later elseif conditions have
                // a skipped path. A flat recovered invocation list cannot
                // select one, so retain only the bounded region uncertainty.
                current.mark_opaque_binding_mutation();
                // Embedded-barrier projection replays from this statement's
                // pre-substitution snapshot. Join the uncertainty there too,
                // so a later substitution in the same expression cannot rely
                // on the flat replay having taken an optional transition.
                if let Some(timeline) = &mut context.timeline {
                    timeline.record_before_substitutions(stmt.span(), current);
                }
            }
            if let Some(timeline) = &mut context.timeline {
                timeline.record_before_direct_call(stmt.span(), current);
            }
            let mut binding_site_widened = false;
            for site in binding_sites_by_span
                .get(&stmt.span())
                .into_iter()
                .flatten()
            {
                if !command_binding_site_is_exact(site, current, namespace) {
                    current.mark_opaque_resolution();
                    binding_site_widened = true;
                }
            }
            if binding_site_widened {
                observe_binding_state(observed, current);
            }
            if let Statement::Call { command, .. } | Statement::Barrier { command, .. } = stmt {
                if let Some(statement_namespace) = namespace.for_head_context(command) {
                    let mut invocation_context = InvocationBindingContext {
                        retained_roots: context.retained_roots,
                        source_order_mode: context.source_order_mode,
                        metadata: BindingInvocationMetadata::Standalone,
                    };
                    let observation_was_updated = apply_may_invocation_transitions(
                        stmt,
                        context.registry,
                        current,
                        observed,
                        statement_namespace.as_ref(),
                        namespace,
                        &mut invocation_context,
                    );
                    if !observation_was_updated {
                        observe_binding_state(observed, current);
                    }
                } else {
                    current.mark_opaque_resolution();
                    observe_binding_state(observed, current);
                }
            }
            match stmt {
                Statement::Block { .. } | Statement::UpFrame { .. } => {
                    for (body, body_namespace) in
                        crate::ir_helpers::nested_execution_bodies(stmt, namespace)
                    {
                        walk(body, context, current, observed, &body_namespace, depth + 1);
                    }
                }
                Statement::If {
                    clauses, else_body, ..
                } => {
                    let incoming = current.clone();
                    let mut joined: Option<ModuleCommandBindings> = None;
                    for body in clauses.iter().map(|clause| &clause.body).chain(else_body) {
                        let mut branch = incoming.clone();
                        walk(body, context, &mut branch, observed, namespace, depth + 1);
                        if let Some(state) = &mut joined {
                            if !state.same_state(&branch) {
                                state.join(&branch);
                            }
                        } else {
                            joined = Some(branch);
                        }
                    }
                    if else_body.is_none() {
                        if let Some(state) = &mut joined {
                            if !state.same_state(&incoming) {
                                state.join(&incoming);
                            }
                        } else {
                            joined = Some(incoming.clone());
                        }
                    }
                    *current = joined.unwrap_or(incoming);
                    observe_binding_state(observed, current);
                }
                _ => {
                    for body in nested_bodies(stmt) {
                        let incoming = current.clone();
                        let mut branch = incoming.clone();
                        walk(body, context, &mut branch, observed, namespace, depth + 1);
                        if !current.same_state(&branch) {
                            current.join(&branch);
                        }
                        observe_binding_state(observed, current);
                    }
                }
            }
            if context.source_order_mode
                && matches!(
                    stmt,
                    Statement::If { .. }
                        | Statement::For { .. }
                        | Statement::While { .. }
                        | Statement::Foreach { .. }
                )
            {
                if matches!(
                    stmt,
                    Statement::For { .. } | Statement::While { .. } | Statement::Foreach { .. }
                ) && source_order_entry
                    .as_ref()
                    .is_some_and(|before| !before.same_state(current))
                {
                    // One source pass cannot close a loop-carried command
                    // binding state. Publish an explicitly opaque region
                    // state rather than mistaking the first post-state for a
                    // fixed point.
                    current.mark_opaque_binding_mutation();
                }
                // A structured node can execute its condition/body more than
                // once or select a later branch. Join the post-region state
                // back at the source span so CFG consumers never reuse an
                // earlier precise binding after a transition on another path.
                if let Some(timeline) = &mut context.timeline {
                    timeline.record_before_substitutions(stmt.span(), current);
                    timeline.record_before_direct_call(stmt.span(), current);
                    for body in nested_bodies(stmt) {
                        timeline.join_region(body, current);
                    }
                }
            }
        }
        // A typed dependency without a surviving owner statement cannot be
        // ordered against command-table transitions. Fail closed rather than
        // silently trusting an orphaned sidecar after an IR transform.
        if binding_sites_by_span
            .keys()
            .any(|span| !statement_spans.contains(span))
        {
            current.mark_opaque_resolution();
            observe_binding_state(observed, current);
        }
    }

    let mut current = initial.clone();
    let mut observed = None;
    walk(script, context, &mut current, &mut observed, namespace, 0);
    BindingWalkOutcome {
        observed: observed.unwrap_or_else(|| current.clone()),
        post: current,
    }
}

/// Whether one typed-lowering dependency still denotes exactly the registry
/// implementation consumed by the compiler at this execution point.
fn command_binding_site_is_exact(
    site: &crate::ir::CommandBindingSite,
    bindings: &ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) -> bool {
    let Some(source_namespace) = namespace.for_head_context(&site.binding.name) else {
        return false;
    };
    if let Some(retained) = &site.binding.namespace_context {
        if source_namespace.to_compiled_context().as_ref() != Some(retained) {
            return false;
        }
    } else if !matches!(source_namespace.as_ref(), SourceNamespaceKey::Authored(_)) {
        return false;
    }
    if bindings.target_may_be_unknown(&site.binding.name, source_namespace.as_ref()) {
        return false;
    }
    let targets = bindings.targets(&site.binding.name, source_namespace.as_ref());
    let mut targets = targets.iter();
    let Some(target) = targets.next() else {
        return false;
    };
    if targets.next().is_some() {
        return false;
    }
    target.registry_backed
        && target.prepended.is_empty()
        && nqn(&target.command) == nqn(&site.binding.identity)
}

/// Apply command-table effects from evaluated `[...]` words before the outer
/// statement. The shared inventory is intentionally value-conservative: an
/// incomplete parse or computed command head widens the command domain.
fn apply_embedded_transitions(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    source_order_mode: bool,
) -> bool {
    apply_embedded_transitions_in(
        stmt,
        registry,
        bindings,
        namespace,
        source_order_mode,
        (BindingInvocationMetadata::Standalone, None),
    )
}

fn apply_embedded_transitions_in(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    source_order_mode: bool,
    selection: (
        BindingInvocationMetadata<'_>,
        Option<tcl_lexer::LexerConfig>,
    ),
) -> bool {
    let state = std::cell::RefCell::new(bindings);
    let resolve = |head: &str| state.borrow().resolved_embedded_head(head, namespace);
    let observe = |words: &[crate::ir_helpers::CommandWord], conditional: bool| {
        let skipped = conditional.then(|| (**state.borrow()).clone());
        apply_embedded_command_transition(
            words,
            registry,
            &mut state.borrow_mut(),
            namespace,
            source_order_mode,
            selection.0,
        );
        if let Some(skipped) = skipped {
            state.borrow_mut().join(&skipped);
        }
    };
    let embedded = match selection {
        (BindingInvocationMetadata::Standalone, _) => evaluated_command_substitutions_with_replay(
            stmt,
            registry,
            Some(&resolve),
            Some(&observe),
        ),
        (BindingInvocationMetadata::Supplied(metadata), Some(config)) => {
            crate::ir_helpers::evaluated_command_substitutions_with_replay_and_metadata_context(
                stmt,
                registry,
                Some(&resolve),
                Some(&observe),
                metadata,
                config,
            )
        }
        (BindingInvocationMetadata::Supplied(_), None) => {
            state.borrow_mut().mark_opaque_binding_mutation();
            return true;
        }
    };
    let observed = embedded.opaque || embedded.all_commands().next().is_some();
    if embedded.opaque {
        state.borrow_mut().mark_opaque_binding_mutation();
    }
    observed
}

fn apply_embedded_command_transition(
    words: &[crate::ir_helpers::CommandWord],
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    source_order_mode: bool,
    metadata: BindingInvocationMetadata<'_>,
) {
    let Some(head) = words.first() else {
        return;
    };
    let Some(head_name) = head.literal() else {
        bindings.mark_opaque_binding_mutation();
        return;
    };
    let Some(command_namespace) = namespace.for_head_context(head_name) else {
        bindings.mark_opaque_binding_mutation();
        return;
    };
    let source_may_be_unknown =
        bindings.target_may_be_unknown(head_name, command_namespace.as_ref());
    if source_order_mode
        && bindings
            .targets(head_name, command_namespace.as_ref())
            .iter()
            .any(|target| !target.registry_backed)
    {
        bindings.mark_source_order_user_procedure_call();
    }
    let facts = match metadata {
        BindingInvocationMetadata::Standalone => {
            bindings.resolve_command_words(words, registry, command_namespace.as_ref())
        }
        BindingInvocationMetadata::Supplied(metadata) => bindings
            .resolve_command_words_with_metadata_context(
                words,
                registry,
                metadata,
                command_namespace.as_ref(),
            ),
    };
    apply_resolved_may_transitions(
        facts,
        source_may_be_unknown,
        // A recovered invocation has no structured IR body. If it can
        // evaluate Tcl text, that text may change any command binding.
        true,
        bindings,
        namespace,
    );
}

fn invocation_transition_inputs(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    command_namespace: &(impl NamespaceKeyQuery + ?Sized),
    source_order_mode: bool,
    metadata: BindingInvocationMetadata<'_>,
) -> (bool, Vec<ResolvedBindingInvocation>, bool) {
    let (Statement::Call { command, .. } | Statement::Barrier { command, .. }) = stmt else {
        return (false, Vec::new(), false);
    };
    let source_may_be_unknown = bindings.target_may_be_unknown(command, command_namespace);
    let reaches_user_procedure = source_order_mode
        && bindings
            .targets(command, command_namespace)
            .iter()
            .any(|target| !target.registry_backed);
    (
        source_may_be_unknown,
        match metadata {
            BindingInvocationMetadata::Standalone => {
                bindings.resolve_statement(stmt, registry, command_namespace)
            }
            BindingInvocationMetadata::Supplied(metadata) => bindings
                .resolve_statement_with_metadata_context(
                    stmt,
                    registry,
                    metadata,
                    command_namespace,
                ),
        },
        reaches_user_procedure,
    )
}

fn join_transition_alternative(
    joined: &mut Option<ModuleCommandBindings>,
    alternative: ModuleCommandBindings,
) {
    if let Some(state) = joined {
        if !state.same_state(&alternative) {
            state.join(&alternative);
        }
    } else {
        *joined = Some(alternative);
    }
}

fn apply_may_invocation_transitions(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    observed: &mut Option<ModuleCommandBindings>,
    command_namespace: &(impl NamespaceKeyQuery + ?Sized),
    execution_namespace: &crate::ir_helpers::ExecutionNamespace,
    context: &mut InvocationBindingContext<'_>,
) -> bool {
    let (source_may_be_unknown, invocations, reaches_user_procedure) = invocation_transition_inputs(
        stmt,
        registry,
        bindings,
        command_namespace,
        context.source_order_mode,
        context.metadata,
    );
    let single_exact_invocation = !source_may_be_unknown && invocations.len() == 1;
    let mut joined: Option<ModuleCommandBindings> = source_may_be_unknown.then(|| bindings.clone());
    let mut exact_definition_key = None;
    for invocation in invocations {
        // A single resolved invocation has no alternative to join. Move the
        // state through that transfer directly so its copy-on-write maps stay
        // uniquely owned; cloning here made a linear run of N exact `proc`
        // declarations copy the growing map N times.
        let mut alternative = if single_exact_invocation {
            std::mem::take(bindings)
        } else {
            bindings.clone()
        };
        let non_binding_before =
            single_exact_invocation.then(|| alternative.non_binding_observation_stamp());
        let procedure_body = recover_procedure_definition(
            &invocation,
            execution_namespace,
            alternative.baseline.dialect,
        );
        let precise_procedure = match &procedure_body {
            ProcedureDefinitionReplay::Recovered { qname, module } => {
                let inventory_was_complete = !module.oo_evidence.unretained_executable_roots
                    && std::iter::once(qname)
                        .chain(module.procedures.keys())
                        .all(|name| alternative.procedure_bodies.contains(name));
                if module.oo_evidence.unretained_executable_roots {
                    alternative.mark_opaque_binding_mutation();
                }
                alternative.extend_procedure_bodies(
                    std::iter::once(qname.clone()).chain(module.procedures.keys().cloned()),
                );
                context.retained_roots.extend_module_roots(module, false);
                if single_exact_invocation && inventory_was_complete {
                    exact_definition_key = exact_procedure_definition_key(
                        &invocation.facts,
                        execution_namespace,
                        qname,
                        alternative.baseline.dialect,
                    );
                }
                Some(qname.as_str())
            }
            ProcedureDefinitionReplay::NotProcedure
            | ProcedureDefinitionReplay::KnownError
            | ProcedureDefinitionReplay::Unavailable => None,
        };
        if !matches!(&procedure_body, ProcedureDefinitionReplay::KnownError) {
            apply_declared_binding_transitions(
                &invocation.facts,
                &mut alternative,
                execution_namespace,
                precise_procedure,
                invocation.source_span.start(),
                None,
                None,
            );
        }
        let body_shape_is_selected = invocation_selects_evaluated_body(&invocation.facts);
        if body_shape_is_selected {
            // An authored Proc-lowering spec may both install a procedure and
            // evaluate a body immediately. That body owns an arbitrary nested
            // binding transfer, so the declared one-key transition no longer
            // proves that the invocation changed only the procedure name.
            // Stock Tcl `proc` carries DEFERS_BODY and stays on the hot path.
            exact_definition_key = None;
        }
        let body_selection_is_indeterminate =
            invocation_may_select_evaluated_body(&invocation.facts);
        let readable_body_was_applied = body_shape_is_selected
            && apply_readable_evaluated_body(
                &invocation,
                registry,
                &mut alternative,
                observed,
                execution_namespace,
                context,
            );
        if invocation_body_requires_widening(
            &invocation.facts,
            body_shape_is_selected,
            body_selection_is_indeterminate,
            readable_body_was_applied,
        ) {
            alternative.mark_opaque_binding_mutation();
        }
        if non_binding_before
            .is_some_and(|before| before != alternative.non_binding_observation_stamp())
        {
            // The typed procedure definition also changed another observable
            // axis (for example LOADS_EXTERNAL_UNIT made command bindings
            // opaque). The one-key historical delta is no longer a complete
            // observation, so retain the ordinary full-state join.
            exact_definition_key = None;
        }
        join_transition_alternative(&mut joined, alternative);
    }
    if let Some(state) = joined {
        *bindings = state;
    }
    if reaches_user_procedure {
        bindings.mark_source_order_user_procedure_call();
    }
    observe_exact_procedure_definition(observed, bindings, exact_definition_key)
}

/// Preserve external loading and unreadable body effects beyond declared transfers.
fn invocation_body_requires_widening(
    facts: &tcl_registry::InvocationFacts,
    body_selected: bool,
    selection_indeterminate: bool,
    readable_body_applied: bool,
) -> bool {
    facts
        .traits
        .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
        && !facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions
                    .facts()
                    .iter()
                    .any(|fact| matches!(fact.transition, StateTransition::Package(_)))
            })
        || (facts
            .traits
            .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
            && ((!body_selected && selection_indeterminate)
                || (body_selected && !readable_body_applied)))
}

/// Publish one exact definition into the historical view without a full-map
/// lattice join. Returns whether the caller's ordinary observation is already
/// complete.
fn observe_exact_procedure_definition(
    observed: &mut Option<ModuleCommandBindings>,
    bindings: &ModuleCommandBindings,
    key: Option<String>,
) -> bool {
    let Some(key) = key else {
        return false;
    };
    if let Some(history) = observed {
        history.join_binding_from(bindings, &key);
        return true;
    }

    // No earlier invocation supplied the pre-transition state. For this first
    // exact definition that state is the registry baseline; seed it explicitly
    // before joining the new procedure target.
    let mut history = bindings.clone();
    let mut historical = ModuleCommandBindings::unmodified_bindings(
        &key,
        history.baseline.semantics.binding_names(),
    );
    historical.extend(bindings.bindings.get(&key).cloned().unwrap_or_else(|| {
        ModuleCommandBindings::unmodified_bindings(&key, history.baseline.semantics.binding_names())
    }));
    history.replace(key, historical);
    *observed = Some(history);
    true
}

/// The exact, registry-declared procedure binding established by one
/// invocation, when that is its sole state transition. This deliberately keys
/// the fast path on typed transition data rather than the spelling `proc`.
fn exact_procedure_definition_key(
    facts: &tcl_registry::InvocationFacts,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    precise_procedure: &str,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Option<String> {
    let transitions = facts.state_transitions.declared()?;
    let [fact] = transitions.facts() else {
        return None;
    };
    let StateTransition::CommandBinding(CommandBindingTransition::Define { name, kind }) =
        &fact.transition
    else {
        return None;
    };
    if *kind != CommandBindingDefinitionKind::Procedure {
        return None;
    }
    let key = procedure_publication_key(namespace, name.literal()?, dialect)?;
    (key == precise_procedure).then_some(key)
}

fn apply_resolved_may_transitions(
    invocations: Vec<tcl_registry::InvocationFacts>,
    source_may_be_unknown: bool,
    dynamic_body_is_opaque: bool,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) {
    let mut joined: Option<ModuleCommandBindings> = source_may_be_unknown.then(|| bindings.clone());
    for facts in invocations {
        let mut alternative = bindings.clone();
        if facts
            .traits
            .contains(tcl_registry::Traits::LOADS_EXTERNAL_UNIT)
            && !facts
                .state_transitions
                .declared()
                .is_some_and(|transitions| {
                    transitions
                        .facts()
                        .iter()
                        .any(|fact| matches!(fact.transition, StateTransition::Package(_)))
                })
            || (dynamic_body_is_opaque
                && facts
                    .traits
                    .contains(tcl_registry::Traits::DYNAMIC_EVAL_BODY)
                && (invocation_selects_evaluated_body(&facts)
                    || invocation_may_select_evaluated_body(&facts)))
        {
            alternative.mark_opaque_binding_mutation();
        }
        apply_declared_binding_transitions(
            &facts,
            &mut alternative,
            namespace,
            None,
            0,
            None,
            None,
        );
        if let Some(state) = &mut joined {
            if !state.same_state(&alternative) {
                state.join(&alternative);
            }
        } else {
            joined = Some(alternative);
        }
    }
    if let Some(state) = joined {
        *bindings = state;
    }
}

/// Whether the resolved invocation selects an argument that Tcl evaluates as
/// script. Some ensemble roots carry `DYNAMIC_EVAL_BODY` for only selected
/// subcommands, so the root trait alone cannot make a precise leaf opaque.
fn invocation_selects_evaluated_body(facts: &tcl_registry::InvocationFacts) -> bool {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::frame_effect::FrameArgLayout;
    use tcl_registry::hooks::LoweringHookId;

    if facts.traits.contains(tcl_registry::Traits::DEFERS_BODY) {
        return false;
    }

    matches!(
        facts.operation,
        SemanticOperationId::StructuredLowering(
            LoweringHookId::Apply | LoweringHookId::NamespaceEval
        )
    ) || facts
        .arg_roles
        .iter()
        .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
        || facts.frame_effect.is_some_and(|effect| {
            matches!(
                effect.layout,
                FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::ScriptInSelectedFrame
            )
        })
}

/// Whether a resolved registry head still has an invocation-time choice of a
/// body-bearing leaf. The root's broad dynamic-evaluation trait is meaningful
/// for an indeterminate ensemble subcommand, but must not taint an exact
/// non-body or deferred leaf.
fn invocation_may_select_evaluated_body(facts: &tcl_registry::InvocationFacts) -> bool {
    matches!(
        &facts.subcommand,
        tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
    ) || (!facts.arg_roles_complete && !facts.traits.contains(tcl_registry::Traits::DEFERS_BODY))
}

fn apply_declared_binding_transitions(
    facts: &tcl_registry::InvocationFacts,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    precise_procedure: Option<&str>,
    declaration: u32,
    publication: Option<&command_publication::PreservingCommandPublication>,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let Some(transitions) = facts.state_transitions.declared() else {
        return;
    };
    let interpreted_execution =
        facts.body_execution.is_some() || invocation_selects_evaluated_body(facts);
    if !interpreted_execution && transitions.facts().iter().any(|fact| matches!(&fact.transition, StateTransition::Widen(widening) if widening.domains.contains(&StateTransitionDomain::CommandResolution))) {
        bindings.mark_opaque_resolution();
    }
    for fact in transitions.facts() {
        if let StateTransition::Namespace(transition) = &fact.transition {
            bindings.namespace_resolution.record(transition, namespace);
            apply_namespace_transition(
                bindings,
                transition,
                namespace,
                declaration,
                original_operands,
            );
        } else if let StateTransition::Interpreter(transition) = &fact.transition {
            bindings.retain_created_interpreter_command(transition, declaration);
        } else if let StateTransition::Package(transition) = &fact.transition {
            apply_package_transition(bindings, transition);
        } else if let StateTransition::ObjectDispatch(
            tcl_registry::ObjectDispatchTransition::Configure { target, .. },
        ) = &fact.transition
        {
            Arc::make_mut(&mut bindings.object_instances).invalidate_class_delegates(false);
            Arc::make_mut(&mut bindings.object_instances).invalidate_dispatch(false);
            if let (Some(name), crate::ir_helpers::ExecutionNamespace::Exact(namespace)) =
                (target.literal(), namespace)
            {
                let targets = bindings.targets(name, namespace);
                for target in targets {
                    if let Some(identity) = target.token {
                        Arc::make_mut(&mut bindings.class_definitions).remove(&identity);
                        Arc::make_mut(&mut bindings.bounded_constructions).remove(&identity);
                    }
                    Arc::make_mut(&mut bindings.tainted_object_dispatch).insert(target.command);
                }
            } else {
                Arc::make_mut(&mut bindings.tainted_object_dispatch).insert("*".to_owned());
            }
        } else if let StateTransition::Trace(transition) = &fact.transition {
            bindings.retain_command_observer(transition, namespace);
        }
    }
    if !interpreted_execution
        && transitions.facts().iter().any(|fact| {
            matches!(
                &fact.transition,
                StateTransition::Widen(widening)
                    if widening.domains.contains(&StateTransitionDomain::CommandBindings)
            )
        })
    {
        bindings.mark_dynamic_proc_binding();
    }
    for transition in transitions.command_bindings() {
        apply_may_binding_transition(
            bindings,
            transition,
            namespace,
            precise_procedure,
            declaration,
            publication,
            original_operands,
        );
    }
}

enum ProcedureDefinitionReplay {
    NotProcedure,
    /// Tcl rejects the invocation before installing a command binding.
    KnownError,
    /// A procedure may be installed, but its runtime body is not source-safe.
    Unavailable,
    Recovered {
        qname: String,
        module: Box<Module>,
    },
}

/// Recover the body installed by the registry's typed procedure-definition
/// operation, including arguments prepended by an alias chain. This is the
/// only path that authorises a precise non-registry command target: a bare
/// `Define(Procedure)` fact is intentionally insufficient because consumers
/// could otherwise dispatch to a procedure whose effects were never walked.
fn recover_procedure_definition(
    invocation: &ResolvedBindingInvocation,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> ProcedureDefinitionReplay {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::hooks::LoweringHookId;

    if invocation.facts.operation != SemanticOperationId::StructuredLowering(LoweringHookId::Proc) {
        return ProcedureDefinitionReplay::NotProcedure;
    }
    match invocation.facts.arity_accepts_frozen_arguments() {
        Some(true) => {}
        Some(false) => return ProcedureDefinitionReplay::KnownError,
        None => return ProcedureDefinitionReplay::Unavailable,
    }
    if !invocation.facts.arg_roles_complete {
        return ProcedureDefinitionReplay::Unavailable;
    }
    let role_index = |role| {
        invocation
            .facts
            .arg_roles
            .iter()
            .find_map(|(index, found)| (*found == role).then_some(usize::from(*index)))
    };
    let (Some(params_index), Some(body_index)) = (
        role_index(tcl_registry::ArgRole::ParamList),
        role_index(tcl_registry::ArgRole::Body),
    ) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let Some(name) = invocation
        .facts
        .state_transitions
        .declared()
        .and_then(|transitions| {
            transitions
                .command_bindings()
                .find_map(|transition| match transition {
                    CommandBindingTransition::Define { name, kind }
                        if *kind == CommandBindingDefinitionKind::Procedure =>
                    {
                        name.literal()
                    }
                    _ => None,
                })
        })
    else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let (Some(params), Some(body)) = (
        invocation
            .literal_arguments
            .get(params_index)
            .and_then(Option::as_deref),
        invocation
            .literal_arguments
            .get(body_index)
            .and_then(Option::as_deref),
    ) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let Some(grammar) = dialect.and_then(tcl_registry::InvocationDialect::parameter_grammar) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    if tcl_syntax::formal_params::parse_formal_parameters_in(params, grammar).is_err() {
        return ProcedureDefinitionReplay::KnownError;
    }

    let Some(qname) = procedure_publication_key(namespace, name, dialect) else {
        return ProcedureDefinitionReplay::Unavailable;
    };
    let module = recovered_procedure_source(body, &qname);
    ProcedureDefinitionReplay::Recovered {
        qname,
        module: Box::new(module),
    }
}

/// Apply command-binding transitions from a statically readable script body
/// evaluated by a runtime barrier. Structural commands whose body location or
/// namespace cannot be expressed by the generic frame-effect grammar are
/// selected by their registry-owned semantic operation, never by spelling.
// Registry frame/body layouts require one ordered state transition.
#[allow(clippy::too_many_lines)]
fn apply_readable_evaluated_body(
    invocation: &ResolvedBindingInvocation,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    observed: &mut Option<ModuleCommandBindings>,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    context: &mut InvocationBindingContext<'_>,
) -> bool {
    use tcl_registry::SemanticOperationId;
    use tcl_registry::frame_effect::{FrameArgLayout, FrameLevel};
    use tcl_registry::hooks::LoweringHookId;

    let arg_refs: Vec<&str> = invocation.arguments.iter().map(String::as_str).collect();
    let body_indices =
        registry.arg_indices_for_role(&invocation.command, &arg_refs, tcl_registry::ArgRole::Body);
    let body_interpreter = invocation.facts.body_interpreter.resolve_with(|index| {
        invocation
            .literal_arguments
            .get(index)
            .and_then(Option::as_deref)
    });
    let (sources, body_namespace, procedure_target) = match invocation.facts.operation {
        SemanticOperationId::StructuredLowering(LoweringHookId::Apply) => {
            let Some(argument_count) = invocation.exact_argument_count else {
                return false;
            };
            let Some(lambda) = invocation
                .literal_arguments
                .first()
                .and_then(Option::as_deref)
            else {
                return false;
            };
            let Some(dialect) = bindings.baseline.dialect else {
                return false;
            };
            let Some(grammar) = dialect.parameter_grammar() else {
                return false;
            };
            // This body replay models an ordinary C lambda activation. Jim
            // caller links need their own entered-frame receipt; accepted
            // formal/count syntax cannot supply that activation.
            if grammar != tcl_dialect::ParameterGrammar::Tcl {
                return false;
            }
            let Ok(elements) = dialect.word_values.split_list(lambda) else {
                // Tcl rejects a malformed lambda before entering its body.
                return true;
            };
            if !(2..=3).contains(&elements.len()) {
                return true;
            }
            let Ok(parameters) = tcl_syntax::formal_params::parse_formal_parameters_in(
                elements[0].as_ref(),
                grammar,
            ) else {
                return true;
            };
            let count = tcl_syntax::formal_params::formal_argument_count_shape(
                parameters
                    .iter()
                    .map(|parameter| (parameter.name == "args", parameter.default.is_some())),
                grammar,
            );
            if !count.accepts(argument_count.saturating_sub(1)) {
                // Known arity errors occur before any body command executes.
                return true;
            }
            let body_namespace = match elements.get(2) {
                Some(name) if name.is_empty() => Some("::".to_owned()),
                Some(name) => qualified_namespace("::", name),
                None => Some("::".to_owned()),
            };
            let Some(body_namespace) = body_namespace else {
                return true;
            };
            (
                vec![elements[1].to_string()],
                Some(crate::ir::ExecutionNamespace::exact(body_namespace)),
                true,
            )
        }
        SemanticOperationId::StructuredLowering(LoweringHookId::NamespaceEval) => {
            let Some(argument_count) = invocation.exact_argument_count else {
                return false;
            };
            let Some(&first_body) = body_indices.first() else {
                return false;
            };
            let Some(target_index) = first_body.checked_sub(1) else {
                return false;
            };
            let Some(target) = invocation
                .literal_arguments
                .get(target_index)
                .and_then(Option::as_deref)
            else {
                return false;
            };
            let Some(source) =
                readable_script_argument(invocation, first_body, registry, bindings, namespace)
            else {
                return false;
            };
            if first_body + 1 != argument_count {
                return false;
            }
            let Some(body_namespace) = qualify_execution_name(namespace, target) else {
                // Tcl rejects an empty namespace before evaluating its body.
                return target.is_empty();
            };
            (
                vec![source],
                Some(crate::ir::ExecutionNamespace::exact(body_namespace)),
                false,
            )
        }
        _ => {
            let Some(argument_count) = invocation.exact_argument_count else {
                return false;
            };
            let (first_body, body_namespace) =
                match (invocation.facts.frame_effect, &body_interpreter) {
                    (Some(effect), _) => match effect.layout {
                        FrameArgLayout::ScriptInCurrentFrame => (0, Some(namespace.clone())),
                        FrameArgLayout::ScriptInSelectedFrame => {
                            let words = invocation
                                .literal_arguments
                                .iter()
                                .map(|word| {
                                    word.as_deref().map_or(
                                        tcl_registry::InvocationWord::Dynamic,
                                        tcl_registry::InvocationWord::Literal,
                                    )
                                })
                                .collect::<Vec<_>>();
                            let arguments = tcl_registry::InvocationWords::structured(
                                tcl_registry::InvocationWord::Literal(&invocation.command),
                                &words,
                            )
                            .with_profile(registry.profile())
                            .arguments();
                            let tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                                level_word_len,
                                level,
                            } = effect.resolve_arguments(arguments)
                            else {
                                return false;
                            };
                            let body = &arg_refs[level_word_len..];
                            let body_namespace = match level {
                                FrameLevel::Absolute(0) => {
                                    Some(crate::ir::ExecutionNamespace::exact("::"))
                                }
                                level if level.is_current_frame() => Some(namespace.clone()),
                                // The caller's defining namespace is unavailable in a
                                // per-procedure root. We can still prove a constructed
                                // script harmless to the command table below; any
                                // actual binding transition remains opaque.
                                _ => None,
                            };
                            (arg_refs.len().saturating_sub(body.len()), body_namespace)
                        }
                        FrameArgLayout::AliasPairs | FrameArgLayout::OpaqueCallerVars => {
                            return false;
                        }
                    },
                    // A body executed by a named child interpreter cannot directly
                    // update this module's command table. Still prove the readable
                    // script free of command-binding mutations before preserving
                    // the parent lattice: that remains safe if a child alias later
                    // re-enters a command in the parent interpreter.
                    (None, tcl_registry::InterpreterScope::Named(_)) => {
                        let Some(&first_body) = body_indices.first() else {
                            return false;
                        };
                        (first_body, None)
                    }
                    (
                        None,
                        tcl_registry::InterpreterScope::Current
                        | tcl_registry::InterpreterScope::Any,
                    ) => {
                        return false;
                    }
                };
            if first_body >= argument_count {
                return false;
            }
            let indices: Vec<usize> = if invocation
                .facts
                .traits
                .contains(tcl_registry::Traits::SCRIPT_CONCATENATES_ARGS)
            {
                // Multi-word concat is exact at runtime but reconstructing its
                // Tcl quoting here would create a second concat interpreter.
                if first_body + 1 != argument_count {
                    return false;
                }
                vec![first_body]
            } else if body_indices.is_empty() {
                (first_body..argument_count).collect()
            } else {
                body_indices
            };
            let Some(sources) = indices
                .into_iter()
                .map(|index| {
                    readable_script_argument(invocation, index, registry, bindings, namespace)
                })
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            (sources, body_namespace, false)
        }
    };

    let Some(body_namespace) = body_namespace else {
        return sources
            .iter()
            .all(|source| command_bindings_unchanged_by_script(source, registry, bindings));
    };

    let crate::ir::ExecutionNamespace::Exact(body_namespace) = body_namespace else {
        return sources
            .iter()
            .all(|source| command_bindings_unchanged_by_script(source, registry, bindings));
    };

    for source in sources {
        let execution_namespace = crate::ir::ExecutionNamespace::exact(&body_namespace);
        if let Some(script) = context.retained_roots.evaluated_body(
            &source,
            &execution_namespace,
            invocation.source_span,
        ) {
            let outcome = {
                let mut context = BindingWalkContext {
                    registry,
                    retained_roots: context.retained_roots,
                    timeline: None,
                    source_order_mode: context.source_order_mode,
                };
                collect_binding_states(&script, &mut context, bindings, &execution_namespace)
            };
            observe_binding_state(observed, &outcome.observed);
            *bindings = outcome.post;
            continue;
        }
        let frame = if procedure_target {
            crate::var_resolve::VariableExecutionFrame::Procedure {
                namespace: body_namespace.clone(),
                identity: format!("evaluated:{}", invocation.source_span.start()),
            }
        } else {
            crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
                namespace: body_namespace.clone(),
                identity: format!("evaluated:{}", invocation.source_span.start()),
            }
        };
        let outcome = interpret_binding_source(
            &source,
            0,
            &body_namespace,
            &frame,
            registry,
            bindings,
            context.retained_roots,
        );
        observe_binding_state(observed, &outcome.observed);
        *bindings = outcome.post;
    }
    true
}

/// Prove that a source-safe script cannot alter the closed command-binding
/// state, even when a frame shift leaves its execution namespace unknown.
///
/// Every namespace already represented by a procedure or command binding is
/// checked, plus the global namespace. A fresh sentinel namespace covers
/// transitions whose result depends only on qualification (for example, a
/// definition in a previously unseen caller namespace). The ordinary
/// transition walker remains the semantic owner: this helper does not name or
/// reinterpret any command.
fn command_bindings_unchanged_by_script(
    source: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
) -> bool {
    let mut namespaces = BTreeSet::from([
        "::".to_owned(),
        "::__tcl_compiler_unknown_caller".to_owned(),
    ]);
    for qname in bindings
        .bindings
        .keys()
        .filter_map(|key| key.authored_spelling())
        .chain(bindings.procedure_bodies.iter().map(String::as_str))
    {
        let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
        namespaces.insert(if holder.is_empty() {
            "::".to_owned()
        } else {
            holder.to_owned()
        });
    }
    namespaces.into_iter().all(|namespace| {
        let mut retained_roots = RetainedBindingRoots::default();
        let frame = crate::var_resolve::VariableExecutionFrame::Namespace(namespace.clone());
        let outcome = interpret_binding_source(
            source,
            0,
            &namespace,
            &frame,
            registry,
            bindings,
            &mut retained_roots,
        );
        outcome.post.same_state(bindings) && outcome.observed.same_state(bindings)
    })
}

fn readable_script_argument(
    invocation: &ResolvedBindingInvocation,
    index: usize,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) -> Option<String> {
    readable_script_argument_with_optional_metadata(
        invocation, index, registry, bindings, namespace, None,
    )
}

fn readable_script_argument_with_optional_metadata(
    invocation: &ResolvedBindingInvocation,
    index: usize,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<String> {
    invocation
        .literal_arguments
        .get(index)
        .and_then(Option::as_deref)
        .map(str::to_owned)
        .or_else(|| {
            let source = invocation.arguments.get(index)?;
            let crate::ir_helpers::ExecutionNamespace::Exact(namespace) = namespace else {
                return None;
            };
            constructed_script_words_with_optional_metadata(
                source, registry, bindings, namespace, metadata,
            )
            .map(tcl_syntax::list::join_list)
        })
}

fn qualify_execution_name(
    namespace: &crate::ir_helpers::ExecutionNamespace,
    name: &str,
) -> Option<String> {
    if name.starts_with("::") {
        return Some(tcl_syntax::naming::canonical_written_command(name));
    }
    let crate::ir_helpers::ExecutionNamespace::Exact(namespace) = namespace else {
        return None;
    };
    Some(tcl_syntax::naming::qualify(namespace, name))
}

fn procedure_publication_key(
    namespace: &crate::ir_helpers::ExecutionNamespace,
    name: &str,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Option<String> {
    let constructed = qualify_execution_name(namespace, name)?;
    tcl_registry::native_procedure::published_procedure_key(constructed, dialect)
}

fn qualified_namespace(parent: &str, child: &str) -> Option<String> {
    if child.is_empty() {
        return None;
    }
    // A namespace name does not have the empty-command-tail meaning of a
    // trailing separator. Normalise the written namespace through the shared
    // syntax owner before joining its relative suffix to the constructed key.
    let canonical = nqn(child);
    if child.starts_with("::") {
        Some(canonical)
    } else {
        Some(tcl_syntax::naming::qualify(
            parent,
            canonical.strip_prefix("::").unwrap_or(&canonical),
        ))
    }
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

/// Source-safe constructed argv under actual supplied availability.
/// The existing may-binding proof remains required independently; no original
/// evaluated builder, Native lookup, frame or result object is invented.
pub(crate) fn constructed_script_words_with_metadata_context(
    word: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &str,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<Vec<String>> {
    let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
    constructed_script_words_with_optional_metadata(
        word,
        registry,
        bindings,
        namespace,
        Some(metadata),
    )
}

fn constructed_script_words_with_optional_metadata(
    word: &str,
    registry: &CommandRegistry,
    bindings: &ModuleCommandBindings,
    namespace: &str,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> Option<Vec<String>> {
    let inner = word
        .strip_prefix('[')
        .and_then(|word| word.strip_suffix(']'))?;
    let config = match metadata {
        Some(metadata) => match metadata.source_analysis_input() {
            Some(input) => input.lexer_config(),
            None => tcl_lexer::LexerConfig::from_grammar(
                tcl_registry::InvocationDialect::of_point(metadata.context().environment.point()?)
                    .lexer_grammar,
            ),
        },
        None => bindings.invocation_dialect().map_or_else(
            || tcl_lexer::LexerConfig::for_profile(registry.profile()),
            |dialect| tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        ),
    };
    let commands = crate::segmenter::segment_commands_with_offset_and_config(inner, 0, config);
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial {
        return None;
    }
    let source_map = tcl_lexer::SourceMap::new(inner);
    let tokens = crate::ir::CommandTokens::from_segmented(&source_map, config, command);
    let words: Vec<String> = tokens
        .words()
        .iter()
        .map(|word| {
            match crate::registry_invocation::effective_invocation_word(
                word,
                config.escapes,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
            ) {
                crate::registry_invocation::EffectiveInvocationWord::Literal(value) => Some(value),
                crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(_)
                | crate::registry_invocation::EffectiveInvocationWord::Dynamic
                | crate::registry_invocation::EffectiveInvocationWord::ArrayElementName {
                    ..
                }
                | crate::registry_invocation::EffectiveInvocationWord::Expanded
                | crate::registry_invocation::EffectiveInvocationWord::KnownExpansion(_)
                | crate::registry_invocation::EffectiveInvocationWord::KnownByteExpansion(_)
                | crate::registry_invocation::EffectiveInvocationWord::Opaque => None,
            }
        })
        .collect::<Option<_>>()?;
    let (builder, operands) = words.split_first()?;
    // A dynamic mutation may have replaced the constructor even when its
    // literal spelling has no individually enumerable transition. Requiring
    // whole-module provenance keeps runtime replay and static effect summaries
    // on the same conservative rule.
    if bindings.target_may_be_unknown(builder, namespace) {
        return None;
    }

    let mut projections = BTreeSet::new();
    let targets = bindings.targets(builder, namespace);
    if targets.is_empty() {
        return None;
    }
    for target in targets {
        if !target.registry_backed {
            return None;
        }
        let mut arguments = target
            .prepended
            .iter()
            .map(|word| word.as_registry_word().literal().map(str::to_owned))
            .collect::<Option<Vec<_>>>()?;
        arguments.extend(operands.iter().cloned());
        let argument_refs: Vec<&str> = arguments.iter().map(String::as_str).collect();
        let invocation = match metadata {
            Some(metadata) => tcl_registry::model::assembly::resolve_invocation_in_context(
                registry,
                Some(metadata.context()),
                &target.command,
                &argument_refs,
            ),
            None => registry.resolve_invocation(
                &target.command,
                &argument_refs,
                registry.own_surface_query(),
            ),
        }?;
        let Some(tcl_registry::ReturnElements::ListOfArgs { from }) =
            invocation.semantics.return_elements
        else {
            return None;
        };
        let projected = arguments.get(usize::from(from)..)?.to_vec();
        if projected.first().is_none_or(String::is_empty) {
            return None;
        }
        projections.insert(projected);
    }
    let mut projections = projections.into_iter();
    let unique = projections.next()?;
    projections.next().is_none().then_some(unique)
}

// Exhaustive interpreter of registry command-binding transitions.
#[allow(clippy::too_many_lines)]
fn apply_may_binding_transition(
    bindings: &mut ModuleCommandBindings,
    transition: &CommandBindingTransition,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    precise_procedure: Option<&str>,
    declaration: u32,
    publication: Option<&command_publication::PreservingCommandPublication>,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let preserves =
        publication.is_some_and(|receipt| receipt.preserves(transition, bindings, declaration));
    if !preserves
        && matches!(
            transition,
            CommandBindingTransition::Move { .. }
                | CommandBindingTransition::Delete { .. }
                | CommandBindingTransition::Alias { .. }
                | CommandBindingTransition::Define { .. }
        )
    {
        Arc::make_mut(&mut bindings.object_instances).invalidate_class_delegates(false);
    }
    if !preserves
        && matches!(
            transition,
            CommandBindingTransition::Define { .. }
                | CommandBindingTransition::Move { .. }
                | CommandBindingTransition::Delete { .. }
        )
    {
        Arc::make_mut(&mut bindings.object_instances).invalidate_dispatch(false);
    }
    if bindings.apply_captured_command_mutation(transition, declaration) {
        return;
    }

    if let Some(original) = original_operands {
        let current = match namespace {
            crate::ir_helpers::ExecutionNamespace::SourceContext(current) => Some(current.clone()),
            crate::ir_helpers::ExecutionNamespace::Exact(namespace) => {
                Some(SourceNamespaceKey::authored(namespace))
            }
            crate::ir_helpers::ExecutionNamespace::RuntimeSelected => None,
        };
        if current
            .as_ref()
            .and_then(|current| original.apply(bindings, transition, current, declaration))
            .is_none()
        {
            bindings.mark_unnameable_rebinding_subject();
        }
        return;
    }

    match transition {
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        } => {
            let Some(source_interpreter) = source_interpreter.literal() else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            if !matches!(source_interpreter, "" | "{}") {
                return;
            }
            let Some(alias) = alias.literal() else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let current = bindings
                .native_root_namespace_key()
                .unwrap_or_else(|| SourceNamespaceKey::authored("::"));
            bindings.record_proc_rebound_candidates(
                alias,
                &crate::ir_helpers::ExecutionNamespace::SourceContext(current.clone()),
            );
            let Some(key) = bindings.publication_key_at(
                &current,
                alias,
                namespace_slots::PublicationPurpose::Alias,
            ) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            bindings.rebound_names.insert(key.clone());
            let binding = if is_current_interpreter(target_interpreter) {
                target.literal().map(|target| {
                    let prepended = arguments
                        .iter()
                        .map(TransitionSubject::literal)
                        .map(|argument| {
                            argument.map_or(
                                crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                                |value| {
                                    crate::registry_invocation::EffectiveInvocationWord::Literal(
                                        value.to_owned(),
                                    )
                                },
                            )
                        })
                        .collect();
                    MayBinding::Target(ResolvedCommandTarget {
                        command: target.to_owned(),
                        prepended,
                        registry_backed: true,
                        kind: BindingKind::Alias,
                        implementation_generation: declaration,
                        terminal: false,
                        target_lookup: *target_lookup,
                        token: None,
                        implementation_allocation: None,
                    })
                })
            } else {
                None
            };
            bindings.install(key, binding.unwrap_or(MayBinding::Unknown));
        }
        CommandBindingTransition::Move { from, to } => {
            let (Some(from), Some(to)) = (from.literal(), to.literal()) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            bindings.record_proc_rebound_candidates(from, namespace);
            if !to.is_empty() {
                bindings.record_proc_rebound_candidates(to, namespace);
            }
            let Some(from_namespace) = namespace.for_head_context(from) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let Some(to_context) = namespace.for_head_context(to) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let Some(to_key) = bindings.publication_key_at(
                to_context.as_ref(),
                to,
                namespace_slots::PublicationPurpose::Rename,
            ) else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let from_keys = bindings.source_keys(from, from_namespace.as_ref());
            if from_keys.len() != 1 {
                bindings.mark_unnameable_rebinding_subject();
            }
            let mut moved = BTreeSet::new();
            for from_key in from_keys {
                bindings.rebound_names.insert(from_key.clone());
                moved.extend(
                    bindings
                        .bindings
                        .get(&from_key)
                        .cloned()
                        .unwrap_or_else(|| {
                            ModuleCommandBindings::unmodified_bindings(
                                &from_key,
                                bindings.baseline.semantics.binding_names(),
                            )
                        }),
                );
                bindings.replace(from_key, BTreeSet::from([MayBinding::Missing]));
            }
            bindings.rebound_names.insert(to_key.clone());
            let mut moved = moved.into_iter().collect::<Vec<_>>();
            for binding in &mut moved {
                if let MayBinding::Target(target) = binding
                    && !target.registry_backed
                    && target.terminal
                {
                    if let Some(spelling) = bindings.callable_spelling_for_key(&to_key) {
                        target.command = spelling;
                    }
                    if let Some(token) = &target.token {
                        Arc::make_mut(&mut bindings.objects)
                            .insert(token.clone(), BTreeSet::from([binding.clone()]));
                    }
                }
            }
            bindings.replace(to_key, moved.into_iter().collect());
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            let affects_current = match interpreter.as_ref().and_then(TransitionSubject::literal) {
                None if interpreter.is_some() => {
                    bindings.mark_unnameable_rebinding_subject();
                    return;
                }
                None | Some("" | "{}") => true,
                Some(_) => false,
            };
            if !affects_current {
                return;
            }
            let Some(name) = name.literal() else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            bindings.record_proc_rebound_candidates(name, namespace);
            // `interp alias {} NAME {}` names NAME in the source
            // interpreter's global namespace; bare `rename NAME {}` uses the
            // current command namespace.
            let deletion_namespace = if interpreter.is_some() {
                Some(std::borrow::Cow::Owned(
                    bindings
                        .native_root_namespace_key()
                        .unwrap_or_else(|| SourceNamespaceKey::authored("::")),
                ))
            } else {
                namespace.for_head_context(name)
            };
            let Some(deletion_namespace) = deletion_namespace else {
                bindings.mark_unnameable_rebinding_subject();
                return;
            };
            let keys = bindings.source_keys(name, deletion_namespace.as_ref());
            if keys.len() != 1 {
                bindings.mark_unnameable_rebinding_subject();
            }
            for key in keys {
                bindings.rebound_names.insert(key.clone());
                bindings.remove(key);
            }
        }
        CommandBindingTransition::Define { name, kind } => {
            if let Some(name) = name.literal() {
                let Some(context) = namespace.for_head_context(name) else {
                    bindings.mark_opaque_binding_mutation();
                    return;
                };
                let purpose = if *kind == CommandBindingDefinitionKind::Procedure {
                    namespace_slots::PublicationPurpose::Procedure
                } else {
                    namespace_slots::PublicationPurpose::Command
                };
                let Some(key) = bindings.publication_key_at(context.as_ref(), name, purpose) else {
                    bindings.mark_opaque_binding_mutation();
                    return;
                };
                let Some(label) =
                    crate::command_binding::ModuleCommandBindings::command_key_label(&key)
                else {
                    bindings.mark_opaque_binding_mutation();
                    return;
                };
                let binding = if *kind == CommandBindingDefinitionKind::Procedure {
                    if precise_procedure == Some(label.as_str())
                        && bindings.procedure_bodies.contains(&label)
                    {
                        MayBinding::Target(ResolvedCommandTarget {
                            command: label.clone(),
                            prepended: Vec::new(),
                            registry_backed: false,
                            kind: BindingKind::Proc,
                            implementation_generation: declaration,
                            terminal: true,
                            target_lookup: tcl_registry::AliasTargetLookup::Global,
                            token: Some(CommandToken {
                                runtime: None,
                                origin: label.clone(),
                                declaration,
                                allocation: None,
                            }),
                            implementation_allocation: None,
                        })
                    } else {
                        bindings.mark_opaque_binding_mutation();
                        MayBinding::Unknown
                    }
                } else {
                    MayBinding::Target(ResolvedCommandTarget {
                        command: label.clone(),
                        prepended: Vec::new(),
                        registry_backed: false,
                        kind: if *kind == CommandBindingDefinitionKind::Object {
                            BindingKind::Class
                        } else {
                            BindingKind::Command
                        },
                        implementation_generation: declaration,
                        terminal: true,
                        target_lookup: tcl_registry::AliasTargetLookup::Global,
                        token: Some(CommandToken {
                            runtime: None,
                            origin: label.clone(),
                            declaration,
                            allocation: None,
                        }),
                        implementation_allocation: None,
                    })
                };
                bindings.install(key, binding);
            } else {
                bindings.mark_opaque_binding_mutation();
            }
        }
        CommandBindingTransition::Unknown { .. } => bindings.mark_unnameable_rebinding_subject(),
    }
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

/// The unperturbed binding of `qname` before any rename/proc/alias.
///
/// A name the registry knows is a `Builtin`, global or namespaced alike:
/// `::string` → `string`, a math function's wrapper — from 8.5
/// `expr` dispatches `abs(…)` to `::tcl::mathfunc::abs`, so a `proc` or
/// `rename` of it rebinds the builtin every `abs(…)` reaches (tclsh 8.5 to
/// 9.1 run the module's `proc`) — and a package's command such as
/// `::base32::encode`, which a `proc` of that name, or of `encode` inside
/// `namespace eval base32`, replaces for every caller (tclsh 8.5 to 9.1 run
/// the module's `proc`). A name the registry does not know is `Opaque`.
fn default_binding(qname: &str, registry: &CommandRegistry) -> Binding {
    let bare = qname.strip_prefix("::").unwrap_or(qname);
    let builtin = if bare.contains("::") {
        registry.get(qname).is_some()
    } else {
        registry.get(bare).is_some()
    };
    if builtin {
        Binding::of(BindingKind::Builtin)
    } else {
        Binding::of(BindingKind::Opaque)
    }
}

/// Record a namespace transition's effect on command *resolution*.
///
/// Returns `false` when the target namespace is not literal, which the caller
/// takes as the lattice top: the transition could have landed anywhere.
///
/// An import binds each pattern's *tail* in the target namespace, so a
/// literal, glob-free pattern is exact — `namespace import -force
/// ::evil::answer` shadowing a declared `::ns::answer`. A glob (`::lib::*`)
/// names commands this scan cannot enumerate, so only the *target* namespace
/// becomes opaque, and a name elsewhere — including in the namespace imported
/// *from* — stays trustworthy.
fn apply_package_transition(
    state: &mut ModuleCommandBindings,
    transition: &tcl_registry::model::binding::PackageTransition,
) {
    use tcl_registry::model::binding::PackageTransition;
    match transition {
        PackageTransition::Provide { package, version } => {
            apply_package_provision(state, package, version.as_ref());
        }
        PackageTransition::Require {
            package,
            requirements,
            exact,
        } => {
            let Some(package) = package.literal() else {
                state.mark_opaque_binding_mutation();
                return;
            };
            if state
                .packages
                .get(package)
                .is_some_and(|provisions| provisions.iter().all(|provision| provision.present))
            {
                return;
            }
            if state
                .packages
                .get(package)
                .is_some_and(|provisions| provisions.iter().any(|provision| provision.present))
            {
                // A previously provided version and a loadable version are
                // distinct paths. Do not replace both with loader provenance.
                state.mark_opaque_binding_mutation();
                return;
            }
            let loader = state.baseline.trusted_loaders.get(package).cloned();
            let selected = loader
                .filter(|loader| trusted_loader_is_selected(state, loader, requirements, *exact));
            let Some(loader) = selected else {
                state.mark_opaque_binding_mutation();
                return;
            };
            install_trusted_package_provider(state, package, loader);
        }
        PackageTransition::Ifneeded {
            package,
            script_provided,
            ..
        } if *script_provided => {
            if let Some(package) = package.literal() {
                Arc::make_mut(&mut state.revoked_loaders).insert(package.to_owned());
            } else {
                state.loader_handler_unknown = true;
            }
        }
        PackageTransition::Forget { packages } => {
            for package in packages {
                if let Some(package) = package.literal() {
                    Arc::make_mut(&mut state.packages).remove(package);
                    Arc::make_mut(&mut state.revoked_loaders).insert(package.to_owned());
                } else {
                    state.loader_handler_unknown = true;
                }
            }
        }
        PackageTransition::UnknownHandler { handler: Some(_) }
        | PackageTransition::Prefer { mode: Some(_) } => {
            state.loader_handler_unknown = true;
        }
        PackageTransition::SourceLoad { .. } => {
            state.mark_opaque_binding_mutation();
            state.loader_handler_unknown = true;
        }
        PackageTransition::DiscoveryDependencyChanged { .. } => {
            Arc::make_mut(&mut state.revoked_loaders)
                .extend(state.baseline.trusted_loaders.keys().cloned());
        }
        PackageTransition::Ifneeded { .. }
        | PackageTransition::UnknownHandler { handler: None }
        | PackageTransition::Prefer { mode: None } => {}
    }
}

fn apply_package_provision(
    state: &mut ModuleCommandBindings,
    package: &TransitionSubject,
    version: Option<&TransitionSubject>,
) {
    let Some(package) = package.literal() else {
        state.loader_handler_unknown = true;
        return;
    };
    if let Some(version) = version {
        let advertised = PackageProvision {
            present: true,
            version: version.literal().map(str::to_owned),
            implementation: None,
        };
        let entry = Arc::make_mut(&mut state.packages)
            .entry(package.to_owned())
            .or_insert_with(|| BTreeSet::from([PackageProvision::absent()]));
        *entry = entry
            .iter()
            .map(|provision| {
                if provision.present {
                    provision.clone()
                } else {
                    advertised.clone()
                }
            })
            .collect();
        Arc::make_mut(&mut state.revoked_loaders).insert(package.to_owned());
    }
}

fn trusted_loader_is_selected(
    state: &ModuleCommandBindings,
    loader: &TrustedPackageLoader,
    requirements: &[TransitionSubject],
    exact: bool,
) -> bool {
    if loader.required_core_family.is_some_and(|family| {
        state
            .baseline
            .dialect
            .and_then(tcl_registry::InvocationDialect::family)
            != Some(family)
    }) {
        return false;
    }
    if state.loader_handler_unknown || state.revoked_loaders.contains(&loader.package) {
        return false;
    }
    if !trusted_package_transfer::preload_dependencies_hold(state, loader) {
        return false;
    }
    if requirements.is_empty() {
        return true;
    }
    let (Some(version), Some(release)) = (loader.version.as_deref(), state.baseline.release) else {
        return false;
    };
    requirements
        .iter()
        .all(|requirement| requirement.literal().is_some())
        && requirements
            .iter()
            .filter_map(TransitionSubject::literal)
            .any(|requirement| {
                if exact {
                    tcl_dialect::compare_versions_for(version, requirement, release).is_eq()
                } else {
                    tcl_dialect::version_satisfies_for(version, requirement, release)
                }
            })
}

fn install_trusted_package_namespace_exports(
    state: &mut ModuleCommandBindings,
    loader: &TrustedPackageLoader,
) {
    for (namespace, exports) in &loader.namespace_exports {
        Arc::make_mut(&mut state.namespaces).insert(namespace.clone());
        if exports
            .iter()
            .any(|pattern| !pattern.is_ascii() || pattern.as_bytes().contains(&0))
        {
            Arc::make_mut(&mut state.unknown_export_namespaces).insert(namespace.clone());
        } else {
            Arc::make_mut(&mut state.namespace_exports).insert(
                namespace.clone(),
                BTreeSet::from([exports
                    .iter()
                    .map(|pattern| tcl_core_types::NameBytes::from(pattern.as_bytes()))
                    .collect()]),
            );
        }
    }
    for (namespace, optional) in &loader.optional_namespace_exports {
        if optional
            .iter()
            .any(|pattern| !pattern.is_ascii() || pattern.as_bytes().contains(&0))
        {
            Arc::make_mut(&mut state.unknown_export_namespaces).insert(namespace.clone());
        }
        let exports = Arc::make_mut(&mut state.namespace_exports)
            .entry(namespace.clone())
            .or_insert_with(|| BTreeSet::from([Vec::new()]));
        let with_optional = exports
            .iter()
            .map(|patterns| {
                let mut patterns = patterns.clone();
                patterns.extend(
                    optional
                        .iter()
                        .filter(|pattern| pattern.is_ascii() && !pattern.as_bytes().contains(&0))
                        .map(|pattern| tcl_core_types::NameBytes::from(pattern.as_bytes())),
                );
                patterns
            })
            .collect::<Vec<_>>();
        exports.extend(with_optional);
    }
}

fn install_trusted_package_provider(
    state: &mut ModuleCommandBindings,
    package: &str,
    loader: TrustedPackageLoader,
) {
    // Proof: naming.package.attested-source-loader-transfer
    // docs/design/analysis/name-resolution-proofs/package-attested-source-loader-transfer.md
    let Some(publication_keys) = trusted_package_transfer::publication_keys(state, &loader) else {
        state.mark_opaque_binding_mutation();
        return;
    };
    for command in &loader.command_surface {
        let slot = nqn(command);
        let (namespace, _) = tcl_syntax::naming::key_holder_and_tail(&slot);
        Arc::make_mut(&mut state.namespaces).insert(namespace.to_owned());
        state.install(
            publication_keys[command].clone(),
            MayBinding::Target(ResolvedCommandTarget {
                command: slot.clone(),
                prepended: Vec::new(),
                registry_backed: true,
                kind: BindingKind::Builtin,
                implementation_generation: 0,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: Some(CommandToken {
                    runtime: None,
                    origin: slot,
                    declaration: 0,
                    allocation: None,
                }),
                implementation_allocation: None,
            }),
        );
    }
    for command in &loader.optional_command_surface {
        let slot = nqn(command);
        Arc::make_mut(&mut state.namespaces).insert(command_holder(&slot));
        let original = MayBinding::Target(ResolvedCommandTarget {
            command: slot.clone(),
            prepended: Vec::new(),
            registry_backed: true,
            kind: BindingKind::Builtin,
            implementation_generation: 0,
            terminal: true,
            target_lookup: tcl_registry::AliasTargetLookup::Global,
            token: Some(CommandToken {
                runtime: None,
                origin: slot.clone(),
                declaration: 0,
                allocation: None,
            }),
            implementation_allocation: None,
        });
        state.replace(
            publication_keys[command].clone(),
            BTreeSet::from([original, MayBinding::Missing]),
        );
    }
    install_trusted_package_namespace_exports(state, &loader);
    Arc::make_mut(&mut state.packages).insert(
        package.to_owned(),
        BTreeSet::from([PackageProvision {
            present: true,
            version: loader.version,
            implementation: Some(loader.implementation_id),
        }]),
    );
}

fn apply_namespace_ensure(
    state: &mut ModuleCommandBindings,
    namespace: &tcl_registry::NamespaceTransitionTarget,
    current: &SourceNamespaceKey,
    offset: u32,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    if let (Some(operands), tcl_registry::NamespaceTransitionTarget::Named(subject)) =
        (original_operands, namespace)
        && matches!(
            state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
            Some(tcl_syntax::naming::NativeNameProtocol::C(_))
        )
        && (subject.literal().is_none() || !matches!(current, SourceNamespaceKey::Authored(_)))
    {
        let selected = operands
            .namespace_ensure_input(subject)
            .and_then(|input| state.ensure_original_namespace_key_at(current, input, offset));
        if selected.is_none() {
            state.mark_opaque_resolution();
        }
        return;
    }
    if let SourceNamespaceKey::Authored(_) = current {
        if let Some(SourceNamespaceKey::Authored(mut ancestor)) =
            state.namespace_target_key_at(namespace, current)
        {
            loop {
                Arc::make_mut(&mut state.namespaces).insert(ancestor.clone());
                if ancestor == "::" {
                    break;
                }
                ancestor = command_holder(&ancestor);
            }
        }
    } else if state
        .ensure_namespace_key_at(namespace, current, offset)
        .is_none()
    {
        state.mark_opaque_resolution();
    }
}

fn apply_namespace_path(
    state: &mut ModuleCommandBindings,
    namespace: &tcl_registry::NamespaceTransitionTarget,
    path: &TransitionSubject,
    current: &SourceNamespaceKey,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let Some(name) = state.namespace_target_key_at(namespace, current) else {
        state.mark_opaque_resolution();
        return;
    };
    if matches!(
        state.baseline.execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            .map(tcl_syntax::naming::NamePolicyProtocol::recipe),
        Some(tcl_syntax::naming::NativeNameProtocol::C(version))
            if version >= tcl_dialect::TclVersion::V8_5
    ) {
        let entries = original_operands
            .and_then(|operands| operands.namespace_input(path))
            .and_then(|input| original_namespace_path::select(state, &name, input));
        match entries {
            Some(entries) => {
                Arc::make_mut(&mut state.unknown_namespace_paths).remove(&name);
                Arc::make_mut(&mut state.namespace_paths).insert(name, BTreeSet::from([entries]));
            }
            None => {
                Arc::make_mut(&mut state.unknown_namespace_paths).insert(name);
            }
        }
        return;
    }
    let Some(path) = path
        .literal()
        .and_then(|value| tcl_syntax::list::split_list(value).ok())
    else {
        Arc::make_mut(&mut state.unknown_namespace_paths).insert(name);
        return;
    };
    let Some(entries) = path
        .iter()
        .map(|entry| {
            state.namespace_target_key_at(
                &tcl_registry::NamespaceTransitionTarget::Named(TransitionSubject::Literal(
                    entry.to_string(),
                )),
                &name,
            )
        })
        .collect::<Option<Vec<_>>>()
    else {
        Arc::make_mut(&mut state.unknown_namespace_paths).insert(name);
        return;
    };
    Arc::make_mut(&mut state.unknown_namespace_paths).remove(&name);
    Arc::make_mut(&mut state.namespace_paths).insert(name, BTreeSet::from([entries]));
}

fn apply_namespace_transition(
    state: &mut ModuleCommandBindings,
    transition: &NamespaceTransition,
    execution: &crate::ir_helpers::ExecutionNamespace,
    offset: u32,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    if matches!(
        transition,
        NamespaceTransition::SetPath { .. }
            | NamespaceTransition::SetUnknown { .. }
            | NamespaceTransition::Ensemble { .. }
    ) {
        Arc::make_mut(&mut state.object_instances).invalidate_receiver_dispatcher();
    }
    let Some(current) = execution.for_head_context("") else {
        state.mark_opaque_resolution();
        return;
    };
    let current = current.as_ref();
    match transition {
        NamespaceTransition::Ensure { namespace } => {
            apply_namespace_ensure(state, namespace, current, offset, original_operands);
        }
        NamespaceTransition::Export {
            namespace,
            patterns,
        } => {
            apply_namespace_exports(state, namespace, patterns, current, original_operands);
        }
        NamespaceTransition::Import {
            namespace,
            force,
            patterns,
        } => {
            let Some(name) = state.namespace_target_key_at(namespace, current) else {
                state.mark_opaque_resolution();
                return;
            };
            for pattern in patterns {
                let Some(pattern) = namespace_pattern_operand_bytes(
                    pattern,
                    original_operands,
                    state.baseline.native_entry.is_some(),
                ) else {
                    Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(name.clone());
                    break;
                };
                if !import_pattern(state, &name, pattern, *force) {
                    break;
                }
            }
        }
        NamespaceTransition::Forget {
            namespace,
            patterns,
        } => {
            let Some(name) = state.namespace_target_key_at(namespace, current) else {
                state.mark_opaque_resolution();
                return;
            };
            for pattern in patterns {
                let Some(pattern) = namespace_pattern_operand_bytes(
                    pattern,
                    original_operands,
                    state.baseline.native_entry.is_some(),
                ) else {
                    Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(name.clone());
                    break;
                };
                forget_pattern(state, &name, pattern);
            }
        }
        NamespaceTransition::SetPath { namespace, path } => {
            apply_namespace_path(state, namespace, path, current, original_operands);
        }
        NamespaceTransition::Delete { namespace } => {
            Arc::make_mut(&mut state.object_instances).invalidate_class_delegates(false);
            Arc::make_mut(&mut state.object_instances).invalidate_dispatch(false);
            apply_namespace_deletion(state, namespace, current, offset, original_operands);
        }
        NamespaceTransition::SetUnknown { namespace, handler } => {
            if let Some(name) = state.namespace_target_key_at(namespace, current) {
                let default = is_default_namespace_unknown_handler(state, handler);
                if default {
                    Arc::make_mut(&mut state.namespace_unknown_handlers).remove(&name);
                } else {
                    Arc::make_mut(&mut state.namespace_unknown_handlers).insert(name);
                }
            } else {
                state.mark_opaque_resolution();
            }
        }
        NamespaceTransition::Ensemble { namespace } => {
            if let Some(name) = state.namespace_target_key_at(namespace, current) {
                Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(name);
            } else {
                state.mark_opaque_resolution();
            }
        }
    }
}

fn apply_namespace_deletion(
    state: &mut ModuleCommandBindings,
    namespace: &tcl_registry::NamespaceTransitionTarget,
    current: &SourceNamespaceKey,
    offset: u32,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let name = match (namespace, original_operands) {
        (tcl_registry::NamespaceTransitionTarget::Named(subject), Some(operands)) => operands
            .namespace_input(subject)
            .and_then(|input| state.original_namespace_key_for_input(current, input)),
        _ => state.namespace_target_key_at(namespace, current),
    };
    let Some(name) = name else {
        state.mark_opaque_resolution();
        return;
    };
    // A retained retired activation must not delete a replacement object at
    // the same component path. The deletion target is its actual incarnation.
    if !matches!(name, SourceNamespaceKey::Authored(_)) && !state.namespaces.contains(&name) {
        return;
    }
    crate::variable_bindings::delete_namespace_cells_in_identity(
        Arc::make_mut(&mut state.source_variables),
        &name,
        offset,
    );
    let slots = state
        .all_command_keys()
        .into_iter()
        .filter(|slot| {
            let holder = slot.holder();
            holder.as_ref() == &name || holder.is_descendant_of(&name)
        })
        .collect::<Vec<_>>();
    for slot in slots {
        state.remove(slot);
    }
    Arc::make_mut(&mut state.namespaces).retain(|ns| ns != &name && !ns.is_descendant_of(&name));
    Arc::make_mut(&mut state.namespace_exports)
        .retain(|ns, _| ns != &name && !ns.is_descendant_of(&name));
    Arc::make_mut(&mut state.namespace_paths)
        .retain(|ns, _| ns != &name && !ns.is_descendant_of(&name));
}

/// Exact original values take precedence over logical transition advice.
/// The legacy authored adapter supports explicit ASCII metadata only; a
/// supplied native entry never borrows those bytes for an absent original.
fn namespace_pattern_operand_bytes<'a>(
    subject: &'a TransitionSubject,
    original_operands: Option<&'a original_command_table::OriginalCommandOperands>,
    native_entry: bool,
) -> Option<&'a [u8]> {
    match original_operands {
        Some(operands) => operands
            .namespace_input(subject)
            .map(crate::signature_scan::scope::SignatureSourceNameInput::bytes),
        None => subject.native_bytes().or_else(|| {
            (!native_entry).then_some(())?;
            let metadata = subject.literal()?;
            (metadata.is_ascii() && !metadata.as_bytes().contains(&0))
                .then_some(metadata.as_bytes())
        }),
    }
}

fn apply_namespace_exports(
    state: &mut ModuleCommandBindings,
    namespace: &tcl_registry::NamespaceTransitionTarget,
    patterns: &[TransitionSubject],
    current: &SourceNamespaceKey,
    original_operands: Option<&original_command_table::OriginalCommandOperands>,
) {
    let Some(namespace) = state.namespace_target_key_at(namespace, current) else {
        state.mark_opaque_resolution();
        return;
    };
    let inputs = patterns
        .iter()
        .map(|pattern| {
            namespace_pattern_operand_bytes(
                pattern,
                original_operands,
                state.baseline.native_entry.is_some(),
            )
        })
        .collect::<Option<Vec<_>>>();
    if inputs
        .as_ref()
        .and_then(|inputs| original_namespace_binding::apply_exports(state, &namespace, inputs))
        .is_none()
    {
        Arc::make_mut(&mut state.unknown_export_namespaces).insert(namespace);
    }
}

fn import_pattern(
    state: &mut ModuleCommandBindings,
    destination: &SourceNamespaceKey,
    pattern: &[u8],
    force: Option<bool>,
) -> bool {
    original_namespace_binding::apply_import(state, destination, pattern, force).is_some()
}

fn imported_binding_links(
    state: &mut ModuleCommandBindings,
    slot: &SourceCommandKey,
    implementations: BTreeSet<MayBinding>,
    name_binding: bool,
) -> (BTreeSet<MayBinding>, BTreeSet<MayBinding>) {
    let mut links = BTreeSet::new();
    let mut source_bindings = BTreeSet::new();
    let spelling = state.callable_spelling_for_key(slot);
    for mut implementation in implementations {
        if spelling.is_none()
            && (name_binding
                || matches!(&implementation, MayBinding::Target(target) if target.token.is_none()))
        {
            links.insert(MayBinding::Unknown);
            source_bindings.insert(implementation);
            continue;
        }
        if name_binding && !matches!(implementation, MayBinding::Missing | MayBinding::Unknown) {
            let token = CommandToken {
                runtime: None,
                origin: spelling.clone().unwrap_or_default(),
                declaration: u32::MAX,
                allocation: None,
            };
            Arc::make_mut(&mut state.objects).insert(
                token.clone(),
                BTreeSet::from([MayBinding::Target(ResolvedCommandTarget {
                    command: spelling.clone().unwrap_or_default(),
                    prepended: Vec::new(),
                    registry_backed: false,
                    terminal: false,
                    target_lookup: tcl_registry::AliasTargetLookup::Global,
                    token: None,
                    kind: BindingKind::Alias,
                    implementation_generation: 0,
                    implementation_allocation: None,
                })]),
            );
            links.insert(MayBinding::Imported(token.into()));
            source_bindings.insert(implementation);
            continue;
        }
        match &mut implementation {
            MayBinding::Imported(token) => {
                links.insert(MayBinding::Imported(ImportedCommandBinding {
                    origin: token.origin.clone(),
                    compiler: None,
                }));
            }
            MayBinding::Target(target) => match state.baseline.import_binding {
                Some(tcl_dialect::NamespaceImportBinding::CommandToken) => {
                    let token = target
                        .token
                        .get_or_insert_with(|| CommandToken {
                            runtime: None,
                            origin: spelling.clone().unwrap_or_default(),
                            declaration: 0,
                            allocation: None,
                        })
                        .clone();
                    Arc::make_mut(&mut state.objects)
                        .insert(token.clone(), BTreeSet::from([implementation.clone()]));
                    links.insert(MayBinding::Imported(token.into()));
                }
                Some(tcl_dialect::NamespaceImportBinding::SourceName) => {
                    let token = CommandToken {
                        runtime: None,
                        origin: spelling.clone().unwrap_or_default(),
                        declaration: u32::MAX,
                        allocation: None,
                    };
                    Arc::make_mut(&mut state.objects).insert(
                        token.clone(),
                        BTreeSet::from([MayBinding::Target(ResolvedCommandTarget {
                            command: spelling.clone().unwrap_or_default(),
                            prepended: Vec::new(),
                            registry_backed: false,
                            terminal: false,
                            target_lookup: tcl_registry::AliasTargetLookup::Global,
                            token: None,
                            kind: BindingKind::Alias,
                            implementation_generation: 0,
                            implementation_allocation: None,
                        })]),
                    );
                    links.insert(MayBinding::Imported(token.into()));
                }
                None => {
                    links.insert(MayBinding::Unknown);
                }
            },
            MayBinding::Unknown => {
                links.insert(MayBinding::Unknown);
            }
            MayBinding::Missing => {
                links.insert(MayBinding::Missing);
            }
        }
        source_bindings.insert(implementation);
    }
    (links, source_bindings)
}

fn forget_pattern(
    state: &mut ModuleCommandBindings,
    destination: &SourceNamespaceKey,
    pattern: &[u8],
) {
    if original_namespace_binding::apply_forget(state, destination, pattern).is_none() {
        Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(destination.clone());
    }
}

fn collect_namespace_resolution(
    transition: &NamespaceTransition,
    site: Option<&str>,
    rebound: &mut std::collections::HashSet<String>,
    resolution: &mut bool,
    opaque: &mut std::collections::HashSet<String>,
) -> bool {
    let (namespace, patterns) = match transition {
        NamespaceTransition::Import {
            namespace,
            patterns,
            ..
        }
        | NamespaceTransition::Forget {
            namespace,
            patterns,
        } => (namespace, Some(patterns)),
        NamespaceTransition::SetPath { namespace, .. }
        | NamespaceTransition::SetUnknown { namespace, .. }
        | NamespaceTransition::Ensemble { namespace }
        | NamespaceTransition::Delete { namespace } => (namespace, None),
        // Creating a namespace, or changing its export list, resolves nothing
        // differently for a caller in it.
        NamespaceTransition::Ensure { .. } | NamespaceTransition::Export { .. } => return true,
    };
    *resolution = true;
    let Some(target) = transition_namespace(namespace, site) else {
        return false;
    };
    let Some(patterns) = patterns else {
        opaque.insert(target);
        return true;
    };
    for pattern in patterns {
        match literal_subject(pattern) {
            Some(text) if !text.contains(['*', '?', '[']) => {
                let tail = text.rsplit("::").next().unwrap_or(text);
                rebound.insert(nqn(&join_ns(&target, tail)));
            }
            _ => {
                opaque.insert(target.clone());
            }
        }
    }
    true
}

/// The namespace a [`NamespaceTransitionTarget`] denotes: the invocation's own
/// namespace for `Current`, the literal text for a `Named` operand, and `None`
/// when the operand is not literal (the caller then takes the lattice top).
fn transition_namespace(
    target: &tcl_registry::NamespaceTransitionTarget,
    current: Option<&str>,
) -> Option<String> {
    match target {
        tcl_registry::NamespaceTransitionTarget::Current => current.map(nqn),
        tcl_registry::NamespaceTransitionTarget::Named(subject) => {
            literal_subject(subject).map(nqn)
        }
    }
}

/// `ns` + `tail` as one qualified name, without doubling the separator.
fn join_ns(ns: &str, tail: &str) -> String {
    if ns == "::" {
        format!("::{tail}")
    } else {
        format!("{ns}::{tail}")
    }
}

fn literal_subject(subject: &TransitionSubject) -> Option<&str> {
    subject.literal()
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

fn unavailable_cfg_entry(registry: &CommandRegistry) -> ModuleCommandBindings {
    let mut unknown = ModuleCommandBindings::initial(registry);
    unknown.mark_opaque_binding_mutation();
    unknown.loader_handler_unknown = true;
    unknown
}

fn cfg_entry_state(cfg: &CfgFunction, registry: &CommandRegistry) -> ModuleCommandBindings {
    let mut retained: Option<Arc<BindingBaseline>> = None;
    let registry_key = registry.snapshot().semantic_key();
    for statement in cfg.blocks.values().flat_map(|block| &block.statements) {
        if !statement.is_executable_invocation() {
            continue;
        }
        let tokens = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
            &cfg.command_binding_sites,
            statement,
        );
        let Some(tokens) = tokens else {
            if statement.tokens().is_some()
                || cfg
                    .command_binding_sites
                    .iter()
                    .any(|site| site.span == statement.span() && site.source_tokens.is_some())
            {
                return unavailable_cfg_entry(registry);
            }
            continue;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            continue;
        };
        let baseline = binding
            .lookup_state
            .as_ref()
            .map(|snapshot| &snapshot.state.baseline);
        let Some(baseline) = baseline.filter(|baseline| {
            baseline.registry_snapshot.as_ref() == Some(&registry_key)
                && retained.as_ref().is_none_or(|old| old == *baseline)
        }) else {
            return unavailable_cfg_entry(registry);
        };
        retained = Some(Arc::clone(baseline));
    }
    retained.map_or_else(
        || {
            if cfg.statement_sources.values().any(Option::is_some)
                || cfg.executed_source.is_some()
                || cfg.namespace_context.is_some()
                || !matches!(
                    &cfg.metadata_context,
                    crate::cfg_builder::CfgMetadataContext::Standalone
                )
            {
                unavailable_cfg_entry(registry)
            } else {
                ModuleCommandBindings::initial(registry)
            }
        },
        |baseline| ModuleCommandBindings::initial_entry_from_baseline(registry, baseline),
    )
}

fn transfer_cfg_statement(
    stmt: &Statement,
    state: &mut ModuleCommandBindings,
    registry: &CommandRegistry,
    point: Option<&CfgPointContext>,
    metadata: BindingInvocationMetadata<'_>,
) {
    if !stmt.is_executable_invocation() {
        return;
    }
    let Some(point) = point else {
        state.mark_opaque_binding_mutation();
        return;
    };
    let namespace = &point.namespace;
    apply_embedded_transitions_in(
        stmt,
        registry,
        state,
        namespace,
        false,
        (metadata, point.config),
    );
    if !matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. }) {
        return;
    }
    let Some(command_namespace) = namespace.for_head_context(stmt.canonical_command_or_source())
    else {
        state.mark_opaque_binding_mutation();
        return;
    };
    let mut observed = None;
    let mut retained = RetainedBindingRoots::default();
    let mut context = InvocationBindingContext {
        retained_roots: &mut retained,
        source_order_mode: false,
        metadata,
    };
    apply_may_invocation_transitions(
        stmt,
        registry,
        state,
        &mut observed,
        command_namespace.as_ref(),
        namespace,
        &mut context,
    );
}

impl CommandBinding<'_> {
    fn state_at_block(&self, block: BlockId, stmt_idx: usize) -> ModuleCommandBindings {
        let mut state = self
            .block_entry
            .get(&block)
            .cloned()
            .unwrap_or_else(|| self.seed.clone());
        if let Some(blk) = self.cfg.blocks.get(&block) {
            for (index, stmt) in blk.statements.iter().enumerate().take(stmt_idx) {
                transfer_cfg_statement(
                    stmt,
                    &mut state,
                    self.registry,
                    self.lookup_contexts.point(block, index),
                    CfgLookupContexts::metadata(self.cfg, self.registry),
                );
            }
        }
        state
    }

    /// Project the exact slot's binding without inventing a second resolver.
    #[must_use]
    pub fn binding_at(&self, block: BlockId, stmt_idx: usize, command_name: &str) -> Binding {
        let state = self.state_at_block(block, stmt_idx);
        self.lookup_contexts
            .binding_at(self.cfg, &state, block, stmt_idx, command_name)
    }

    /// Whether the source name still selects its original registry command.
    #[must_use]
    pub fn is_original_builtin_at(
        &self,
        block: BlockId,
        stmt_idx: usize,
        command_name: &str,
    ) -> bool {
        self.binding_at(block, stmt_idx, command_name)
            .is_original_builtin()
    }

    /// Every source spelling changed at a represented program point.
    #[must_use]
    pub fn rebound_names(&self) -> HashSet<String> {
        let mut names = HashSet::new();
        for block in &self.ordered_blocks {
            let mut state = self
                .block_entry
                .get(block)
                .cloned()
                .unwrap_or_else(|| self.seed.clone());
            names.extend(state.changed_command_spellings());
            if let Some(blk) = self.cfg.blocks.get(block) {
                for (index, stmt) in blk.statements.iter().enumerate() {
                    transfer_cfg_statement(
                        stmt,
                        &mut state,
                        self.registry,
                        self.lookup_contexts.point(*block, index),
                        CfgLookupContexts::metadata(self.cfg, self.registry),
                    );
                    names.extend(state.changed_command_spellings());
                }
            }
        }
        names
    }

    /// Whether some executable path has an unbounded command mutation.
    #[must_use]
    pub fn has_wildcard(&self) -> bool {
        self.ordered_blocks.iter().any(|block| {
            let end = self
                .cfg
                .blocks
                .get(block)
                .map_or(0, |block| block.statements.len());
            self.state_at_block(*block, end).opaque_domain
        })
    }
}

fn cfg_seed_with_initial(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    initial: &[(String, Binding)],
) -> ModuleCommandBindings {
    let mut seed = cfg_entry_state(cfg, registry);
    for (name, binding) in initial {
        let name = nqn(name);
        let value = match binding.kind {
            BindingKind::Unknown => MayBinding::Unknown,
            BindingKind::Opaque | BindingKind::Bottom => MayBinding::Missing,
            kind => MayBinding::Target(ResolvedCommandTarget {
                command: binding.target.clone().unwrap_or_else(|| name.clone()),
                prepended: Vec::new(),
                registry_backed: kind == BindingKind::Builtin,
                kind,
                terminal: kind != BindingKind::Alias,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                implementation_generation: 0,
                token: Some(CommandToken {
                    runtime: None,
                    origin: name.clone(),
                    declaration: 0,
                    allocation: None,
                }),
                implementation_allocation: None,
            }),
        };
        if binding.kind == BindingKind::Proc {
            seed.extend_procedure_bodies([name.clone()]);
        }
        seed.install(name, value);
    }
    seed
}

/// Flow-sensitive CFG analysis using the same state and transition kernel as
/// source interpretation and module effect summaries.
#[must_use]
pub fn analyse_command_binding<'a>(
    cfg: &'a CfgFunction,
    registry: &'a CommandRegistry,
    initial: &[(String, Binding)],
) -> CommandBinding<'a> {
    let mut preds: HashMap<BlockId, Vec<BlockId>> =
        cfg.blocks.keys().map(|id| (*id, Vec::new())).collect();
    for &id in cfg.blocks.keys() {
        for succ in cfg.block_successors(id) {
            if let Some(incoming) = preds.get_mut(&succ) {
                incoming.push(id);
            }
        }
    }
    let seed = cfg_seed_with_initial(cfg, registry, initial);
    let order = cfg.reverse_postorder();
    let lookup_contexts = CfgLookupContexts::new(cfg, registry, &seed);
    let mut block_entry = HashMap::<BlockId, ModuleCommandBindings>::new();
    let mut block_exit = HashMap::<BlockId, ModuleCommandBindings>::new();
    loop {
        let mut changed = false;
        for id in &order {
            let mut predecessors = preds
                .get(id)
                .into_iter()
                .flatten()
                .filter_map(|id| block_exit.get(id));
            let mut entry = if *id == cfg.entry {
                seed.clone()
            } else if let Some(first) = predecessors.next() {
                first.clone()
            } else {
                continue;
            };
            for predecessor in predecessors {
                entry.join(predecessor);
            }
            block_entry.insert(*id, entry.clone());
            let mut exit = entry;
            if let Some(block) = cfg.blocks.get(id) {
                for (index, stmt) in block.statements.iter().enumerate() {
                    transfer_cfg_statement(
                        stmt,
                        &mut exit,
                        registry,
                        lookup_contexts.point(*id, index),
                        CfgLookupContexts::metadata(cfg, registry),
                    );
                }
            }
            if block_exit
                .get(id)
                .is_none_or(|previous| !previous.same_state(&exit))
            {
                block_exit.insert(*id, exit);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    CommandBinding {
        block_entry,
        ordered_blocks: order,
        cfg,
        registry,
        seed,
        lookup_contexts,
    }
}

/// Whether every command-*binding* subject this scan saw could be named.
///
/// A two-state fact rather than a fourth `bool` beside [`ModuleCommandMutations::dynamic`]:
/// the states carry the meaning at the use site, and both this summary and
/// its memo key stay inside `clippy::pedantic`'s bool budget without an
/// `#[allow]`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RebindingSubjects {
    /// Every `rename` / `interp alias` / delete subject was one this scan
    /// could spell, so the names it did not record still denote what they did.
    #[default]
    AllNameable,
    /// At least one was not — `rename $a {}`, not `rename foo bar`. The
    /// subject may have been *this* name, so no name is claimable (#2168).
    SomeUnnameable,
}

impl RebindingSubjects {
    /// The axis a `bool`-valued observation stands for.
    #[must_use]
    fn from_unnameable(unnameable: bool) -> Self {
        if unnameable {
            Self::SomeUnnameable
        } else {
            Self::AllNameable
        }
    }
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

impl ProcBindingTrustProjection {
    /// Whether `proc_name` still denotes its declared procedure at an
    /// arbitrary later call site.
    #[must_use]
    pub(crate) fn trusts_proc_binding(&self, proc_name: &str) -> bool {
        !self.dynamic && !self.rebound.contains(&nqn(proc_name))
    }

    /// Whether a dynamic command-binding transition made every procedure
    /// binding untrustworthy.
    #[must_use]
    pub(crate) const fn has_dynamic_binding_transition(&self) -> bool {
        self.dynamic
    }
}

/// Hashes the complete projection [`ModuleCommandMutations::snapshot`] keeps
/// — every field, the sets sorted — so two summaries equal as sets hash
/// alike and the value can join an interned memo key beside the snapshot
/// it was rebuilt from.
impl std::hash::Hash for ModuleCommandMutations {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.snapshot().hash(state);
    }
}

impl ModuleCommandMutations {
    /// Whether any command-table mutation has a dynamic source or target.
    ///
    /// Consumers retaining typed proof declines need to distinguish this
    /// lattice top from a statically known rebinding of one command name.
    #[must_use]
    pub const fn has_dynamic_mutation(&self) -> bool {
        self.dynamic
    }

    /// Whether `name` sits in a namespace whose resolution this scan could
    /// not enumerate — see [`Self::opaque_namespaces`].
    fn import_shadowed(&self, name: &str) -> bool {
        if self.opaque_namespaces.is_empty() {
            return false;
        }
        let qualified = nqn(name);
        let namespace = crate::optimiser::helpers::naming::namespace_from_qualified(&qualified);
        self.opaque_namespaces.contains(&namespace)
    }

    /// Whether any body changes namespace command *resolution* — see
    /// [`Self::resolution_changed`]. A consumer folding a command it resolved
    /// itself must decline when this is true.
    #[must_use]
    pub const fn changes_command_resolution(&self) -> bool {
        self.resolution_changed
    }

    /// Whether any command whose canonical name lies under `prefix` is
    /// rebound — a `rename` source or target, or an `interp alias` name.
    ///
    /// The math-function gate asks this about `::tcl::mathfunc::`: `expr`
    /// resolves `abs(…)` through the command table, so
    /// `rename ::tcl::mathfunc::abs {}` must stop the compiler evaluating it
    /// natively (C Tcl then raises `invalid command name`).
    #[must_use]
    pub fn rebinds_under(&self, prefix: &str) -> bool {
        let bare = prefix.strip_prefix("::").unwrap_or(prefix);
        self.rebound.iter().any(|name| {
            let canonical = name.strip_prefix("::").unwrap_or(name);
            canonical.starts_with(bare)
        })
    }

    /// True when `command_name` is not clobbered by any body mutation —
    /// i.e. the optimiser may still fold it with its original builtin
    /// semantics.
    #[must_use]
    pub fn trusts(&self, command_name: &str) -> bool {
        !self.dynamic && self.observed_binding_is_the_builtin(command_name)
    }

    /// The **named-subject** half of [`Self::trusts`]: whether the module's
    /// own observed bindings leave `command_name` denoting its registry
    /// builtin — no shadowing `proc`, no `rename` or alias onto it, no
    /// import into a namespace this scan could not enumerate.
    ///
    /// Deliberately omits [`Self::dynamic`], the unbounded top, which names
    /// no subject at all and is raised by something as ordinary as one
    /// command head the registry cannot resolve. Every fold that *becomes a
    /// source rewrite* still asks the full [`Self::trusts`]; this narrower
    /// question is what the shared per-unit value lattice asks
    /// ([`crate::sccp::FoldTrust::ObservedBindings`]), so a call the module
    /// itself resolves to a user `proc` can never be evaluated with builtin
    /// semantics there (#2164) — without withdrawing constant folding from
    /// every file that mentions a command the registry does not know.
    #[must_use]
    pub fn observed_binding_is_the_builtin(&self, command_name: &str) -> bool {
        // A rebinding whose subject this scan could not name may have moved
        // *this* name, so nothing is claimable (#2168). Deliberately not the
        // whole of `dynamic`, which also rises for effects that rebind nothing
        // and whose folds #2164 measured as worth keeping.
        self.rebinding_subjects == RebindingSubjects::AllNameable
            && !self.import_shadowed(command_name)
            && !self.names.contains(&nqn(command_name))
    }

    /// Whether this summary reports no command-table mutation that could
    /// change what a *builtin* name denotes — [`Self::trusts`] then answers
    /// `true` for every name, exactly as [`Self::default`] does.
    ///
    /// Consumers that omit the complete mutation snapshot may use this query
    /// to establish agreement with an untouched-module baseline. The function
    /// lattice memo retains the full snapshot and therefore keys separately
    /// for namespace-local shadows and unknown mutation obligations.
    #[must_use]
    pub fn agrees_with_untouched_bindings(&self) -> bool {
        !self.dynamic && self.names.is_empty() && self.opaque_namespaces.is_empty()
    }

    /// Whether any body runs in a namespace chosen at run time while naming a
    /// command relatively.
    ///
    /// A receiver-local command can shadow such a head, so nothing derived
    /// inside one of those frames is trustworthy — not even a fact that reads
    /// no variable, such as the `TclOO` frame constant. Consumers that fold
    /// *inside* a method body ask this in addition to [`Self::trusts`]; the
    /// top level and procedures do not, because a receiver namespace cannot
    /// change what a name resolves to in a namespace this scan can spell.
    #[must_use]
    pub const fn has_runtime_selected_frames(&self) -> bool {
        self.runtime_selected_frames
    }

    /// The everything-is-untrusted lattice top: `trusts` /
    /// `trusts_proc_binding` answer `false` for every name. The sound
    /// stand-in when a consumer has **no whole-module view at all** (the
    /// analyser's isolated per-item body pass) — folding with builtin
    /// semantics is then never permitted.
    #[must_use]
    pub fn distrust_all() -> Self {
        Self {
            names: std::collections::HashSet::new(),
            rebound: std::collections::HashSet::new(),
            dynamic: true,
            rebinding_subjects: RebindingSubjects::SomeUnnameable,
            runtime_selected_frames: true,
            resolution_changed: true,
            opaque_namespaces: std::collections::HashSet::new(),
        }
    }

    /// A canonical, hashable snapshot of this summary — see
    /// [`CommandTrustSnapshot`].
    #[must_use]
    pub fn snapshot(&self) -> CommandTrustSnapshot {
        let mut untrusted_builtins: Vec<String> = self.names.iter().cloned().collect();
        untrusted_builtins.sort_unstable();
        let mut rebound: Vec<String> = self.rebound.iter().cloned().collect();
        rebound.sort_unstable();
        CommandTrustSnapshot {
            untrusted_builtins,
            rebound,
            dynamic: self.dynamic,
            runtime_selected_frames: self.runtime_selected_frames,
            rebinding_subjects: self.rebinding_subjects,
            resolution_changed: self.resolution_changed,
            opaque_namespaces: {
                let mut v: Vec<String> = self.opaque_namespaces.iter().cloned().collect();
                v.sort_unstable();
                v
            },
        }
    }

    /// True when `proc_name` can still be trusted to denote the module
    /// procedure it was declared as at an arbitrary later call site — i.e.
    /// its bare name was never the subject of a later `rename` (as the old
    /// name being moved away *or* the new name a different command moved
    /// onto) or `interp alias` (as the alias name) anywhere in the module.
    ///
    /// Flow-insensitive and whole-module, like [`Self::trusts`]: a
    /// rebinding buried in a proc body only takes effect when that proc
    /// runs, and the cross-proc call order isn't statically known, so any
    /// observed rebinding of the name is treated as live everywhere. This
    /// is what makes it sound to gate the optimiser's proc-call constant
    /// fold (O103) on this query — folding a call to the *original* proc's
    /// constant return would miscompile a script that later does
    /// `rename otherProc thisName` or `interp alias {} thisName {} other`.
    #[must_use]
    pub fn trusts_proc_binding(&self, proc_name: &str) -> bool {
        !self.dynamic && self.observed_proc_binding(proc_name)
    }

    /// Whether the module may rebind any builtin: a shadowing `proc`, a
    /// `rename` or alias onto a builtin's name, or a rebinding whose subject
    /// this scan could not name.
    #[must_use]
    pub fn rebinds_builtins(&self) -> bool {
        !self.names.is_empty() || self.rebinding_subjects != RebindingSubjects::AllNameable
    }

    /// The **named-subject** half of [`Self::trusts_proc_binding`], as
    /// [`Self::observed_binding_is_the_builtin`] is of [`Self::trusts`]: no
    /// `rename` or alias in the module names `proc_name`, and it sits in no
    /// namespace whose resolution this scan could not enumerate. Omits the
    /// unbounded `dynamic` top, which one unresolved command head raises.
    #[must_use]
    pub fn observed_proc_binding(&self, proc_name: &str) -> bool {
        !self.import_shadowed(proc_name) && !self.rebound.contains(&nqn(proc_name))
    }
}

/// A canonical (sorted), hashable form of [`ModuleCommandMutations`], so
/// the whole-module trust fact can ride inside a memoisation key — the
/// analyser's per-item body pass carries it on each deferred body whose
/// text could fold a command substitution, keeping the
/// isolated fragment memo sound when a `rename` elsewhere in the file
/// appears or disappears.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandTrustSnapshot {
    untrusted_builtins: Vec<String>,
    rebound: Vec<String>,
    dynamic: bool,
    /// Part of the key: a memo taken while every rebinding subject was
    /// nameable must not be reused once a `rename $x` appears, or the fold it
    /// carries would be replayed under a binding it never saw (#2168).
    rebinding_subjects: RebindingSubjects,
    /// Part of the key: a memo taken with every frame statically nameable must
    /// not be reused once a receiver-selected frame appears.
    runtime_selected_frames: bool,
    /// Part of the key: a memo taken with namespace command resolution
    /// unperturbed must not be reused once a `namespace import` appears.
    resolution_changed: bool,
    opaque_namespaces: Vec<String>,
}

impl CommandTrustSnapshot {
    /// The registry-neutral evidence this snapshot carries into the value
    /// transfer's analysis context: the rebound builtins, the redefined
    /// procedures, the dynamic flag, and the opaque namespaces.
    #[must_use]
    pub fn binding_evidence(&self) -> tcl_registry::value_transfer::BindingEvidence {
        tcl_registry::value_transfer::BindingEvidence {
            untrusted_builtins: self.untrusted_builtins.clone(),
            rebound: self.rebound.clone(),
            dynamic: self.dynamic,
            opaque_namespaces: self.opaque_namespaces.clone(),
        }
    }

    /// Rebuild the queryable summary this snapshot was taken from.
    #[must_use]
    pub fn to_mutations(&self) -> ModuleCommandMutations {
        ModuleCommandMutations {
            names: self.untrusted_builtins.iter().cloned().collect(),
            rebound: self.rebound.iter().cloned().collect(),
            dynamic: self.dynamic,
            rebinding_subjects: self.rebinding_subjects,
            runtime_selected_frames: self.runtime_selected_frames,
            resolution_changed: self.resolution_changed,
            opaque_namespaces: self.opaque_namespaces.iter().cloned().collect(),
        }
    }
}

/// The builtin a *qualified* definition shadows for bodies that resolve in its
/// namespace, if any.
///
/// `proc ::n::expr` does not rebind the builtin `expr` — its own name is
/// `::n::expr`, which has no builtin default, so the loop above drops it. But
/// an unqualified `expr` inside `::n` resolves to it first, so folding one
/// there with builtin semantics is wrong: `::n::expr` may return its argument
/// verbatim, and O110 rewriting `$r ** 2` into `$r * $r` then changes what the
/// program prints (#2159).
///
/// The answer is the bare tail, which distrusts the builtin for the whole
/// module rather than only inside `::n`. That is deliberately conservative and
/// matches what the `namespace eval ::n { proc expr … }` spelling already
/// does — the two spellings disagreeing is the defect. Narrowing it to the
/// defining namespace needs a resolution namespace at every call site, which
/// `trusts` does not take.
///
/// A namespace-local math function is the same shape one level down: from
/// 8.5 `expr` resolves `tcl::mathfunc::NAME` relative to the namespace it
/// runs in before the global one, so `proc ::ns::tcl::mathfunc::abs` is what
/// `abs(…)` calls inside `::ns` (tclsh 8.5 to 9.1 run it; 8.4 has no wrapper
/// commands). The answer is the global wrapper it shadows, distrusted for
/// the whole module as the `::n::expr` case is.
fn builtin_shadowed_by_qualified_definition(
    name: &str,
    registry: &CommandRegistry,
) -> Option<String> {
    let (holder, tail) = tcl_syntax::naming::key_holder_and_tail(name);
    if holder.is_empty() || tail.is_empty() {
        return None;
    }
    if tcl_registry::mathfunc::is_in_mathfunc_namespace(name) {
        let wrapper = tcl_registry::mathfunc::qualified_name(tail);
        return registry.get(&wrapper).is_some().then(|| nqn(&wrapper));
    }
    (default_binding(tail, registry).kind == BindingKind::Builtin).then(|| nqn(tail))
}

/// Record every name a bare `rename` / `interp alias` argument could
/// resolve to when it runs inside `namespace` — Tcl resolves an
/// unqualified command name against the *current* namespace at the point
/// the `rename`/`interp alias` executes, not the global namespace (a
/// `proc ::ns::doit {} { rename triple double }` renames `::ns::triple`
/// to `::ns::double`, not `::triple`/`::double` — confirmed against
/// tclsh 9.0.4). This scan is flow-insensitive and doesn't know whether a
/// same-named command already exists in `namespace` at that point, so it
/// conservatively records BOTH the namespace-relative and the
/// global-rooted candidate for a bare name — the same sound
/// over-approximation [`collect_tampered_builtins`] already applies.
/// A name that already contains `::` resolves unambiguously (rooted at
/// `::`, matching the optimiser's own `resolve_proc_qname` simplified
/// qualification rule), so only one candidate is recorded for it.
fn insert_rebound_candidates(name: &str, namespace: &str, rebound: &mut impl Extend<String>) {
    if name.contains("::") || namespace == "::" {
        rebound.extend([nqn(name)]);
        return;
    }
    rebound.extend([nqn(&format!("{namespace}::{name}")), nqn(name)]);
}

/// Whether `script` declares anything that could change what a command name
/// resolves to, at any nesting depth.
///
/// This is the question a body in an unnameable frame poses to the *rest* of
/// the module. A mutation there names its subject relatively, so it lands in a
/// namespace this scan cannot spell and no recorded name survives it. A body
/// that mutates nothing moves no name, whatever it calls, and leaves the names
/// the top level and the procedures resolve exactly as they were.
///
/// Registry-driven, like the walk it guards: a statement counts because its
/// declared state transitions touch command bindings or namespace resolution,
/// never because of the command's name. `Define` counts alongside the rest —
/// a `proc` in such a frame lands in a namespace this scan cannot spell, so
/// the widened stance is the right answer for it too.
///
/// Descends through [`crate::ir_helpers::nested_bodies`], the same inventory
/// [`crate::ir_helpers::requires_runtime_command_namespace`] walks, so the two
/// cannot disagree about which bodies belong to the frame. That inventory
/// covers the statically lowered `eval` and `uplevel` forms (`Statement::Block`
/// and `Statement::UpFrame`) as well as the structured ones, so a rebinding
/// hidden in `eval {rename ::string ::saved}` still widens.
///
/// Hitting the depth cap answers `true`: an unwalkable body is treated as one
/// that mutates.
fn declares_command_binding_effect(
    script: &crate::ir::Script,
    registry: &CommandRegistry,
    depth: u32,
) -> bool {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return true;
    }
    script.statements.iter().any(|stmt| {
        statement_declares_command_binding_effect(stmt, registry)
            || crate::ir_helpers::nested_bodies(stmt)
                .into_iter()
                .any(|body| declares_command_binding_effect(body, registry, depth + 1))
    })
}

/// The single-statement half of [`declares_command_binding_effect`].
fn statement_declares_command_binding_effect(stmt: &Statement, registry: &CommandRegistry) -> bool {
    let Some(facts) = invocation_facts(stmt, registry) else {
        return false;
    };
    let Some(transitions) = facts.state_transitions.declared() else {
        return false;
    };
    transitions
        .facts()
        .iter()
        .any(|fact| match &fact.transition {
            StateTransition::CommandBinding(_) | StateTransition::Namespace(_) => true,
            StateTransition::Widen(widening) => widening
                .domains
                .contains(&StateTransitionDomain::CommandBindings),
            StateTransition::Interpreter(_)
            | StateTransition::Package(_)
            | StateTransition::VariableCellAlias(_)
            | StateTransition::Trace(_)
            | StateTransition::ObjectDispatch(_) => false,
        })
}

/// Summarise command-table mutations across the whole module — a
/// CFG-free recursive IR walk over the top-level script *and* every proc
/// / method body, so it can run before per-function CFGs are built.
///
/// Tampered-with core builtins and rebound names generally are reported
/// (see [`collect_tampered_builtins`]).  The result feeds both the
/// optimiser's builtin-fold trust gate ([`ModuleCommandMutations::trusts`])
/// and its proc-call fold trust gate
/// ([`ModuleCommandMutations::trusts_proc_binding`]).
#[must_use]
pub fn scan_module_command_mutations(
    ir_module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> ModuleCommandMutations {
    let command_bindings = ModuleCommandBindings::analyse(ir_module, registry);
    scan_module_command_mutations_with_bindings(ir_module, registry, &command_bindings)
}

/// Join registry-declared namespace-resolution effects with an already-built
/// closed command-binding lattice.
///
/// Compilation-unit construction uses this seam so CFG construction,
/// optimiser trust, and runtime provenance consume one binding analysis. The
/// public convenience wrapper above remains available to callers that do not
/// already retain the prepared lattice.
#[must_use]
pub(crate) fn scan_module_command_mutations_with_bindings(
    ir_module: &crate::ir::Module,
    registry: &CommandRegistry,
    command_bindings: &ModuleCommandBindings,
) -> ModuleCommandMutations {
    let mut projected = command_bindings.mutation_projection(registry);
    for (script, namespace) in ir_module.executable_script_roots() {
        if namespace == crate::ir::ExecutionNamespace::RuntimeSelected
            && crate::ir_helpers::requires_runtime_command_namespace(script, registry)
        {
            projected.runtime_selected_frames = true;
            projected.dynamic |= declares_command_binding_effect(script, registry, 0);
        }
    }
    projected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn analyse(src: &str) -> (CompilationUnit, CommandRegistry) {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &reg, false);
        (cu, reg)
    }

    fn source_proof(
        source: &str,
        head: &str,
        dialect: tcl_registry::InvocationDialect,
    ) -> SourceInvocationBinding {
        let registry = CommandRegistry::build_default();
        let binding = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..SourceAnalysisOptions::default()
            },
        );
        binding.invocation_at_source(head, u32::try_from(source.rfind(head).unwrap()).unwrap())
    }

    fn allocation_source(source: &str) -> SourceCommandBindings {
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &CommandRegistry::build_default(),
            SourceAnalysisOptions {
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                )),
                ..SourceAnalysisOptions::default()
            },
        )
    }

    #[test]
    fn logical_command_layout_keeps_unknown_physical_frames_separate() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&context),
            config,
        );
        let image = tcl_lexer::SourceImage::document("proc subject {} {return VALUE}");
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &image,
            &crate::var_resolve::VariableExecutionFrame::Unknown,
            config,
            context.commands(),
            SourceAnalysisOptions::for_logical_source(&input).unwrap(),
        )
        .unwrap();
        let point = bindings.points.first().expect("original source command");
        assert_eq!(point.state.logical_source_name_advice_input(), Some(&input));
        assert_eq!(
            point.state.variable_frame,
            crate::var_resolve::VariableExecutionFrame::Unknown
        );
        let segment =
            crate::segmenter::segment_commands_image_with_offset_and_config(&image, 0, config)
                .unwrap()
                .remove(0);
        let tokens =
            crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
        let site = CommandAllocationSite {
            source: Arc::clone(point.state.current_source_origin.as_ref().unwrap()),
            offset: segment.span.start(),
        };
        let snapshot = Arc::new(SourceLookupSnapshot::new(point.state.clone()));
        let selected = original_site_operand_layout_advice(
            &site,
            &tokens,
            &snapshot,
            &point.namespace_key,
            config,
        )
        .expect("conditional original Logical layout");
        assert_eq!(
            selected.frame(),
            &crate::var_resolve::VariableExecutionFrame::Unknown
        );
        assert!(selected.targets.iter().all(|target| target.registry_backed));
        assert!(snapshot.state.baseline.native_entry.is_none());
        assert!(snapshot.state.baseline.execution_name_policy.is_none());

        let mut stale = config;
        stale.strict_quoting = !stale.strict_quoting;
        assert!(
            original_site_operand_layout_advice(
                &site,
                &tokens,
                &snapshot,
                &point.namespace_key,
                stale,
            )
            .is_none()
        );
        let mut unowned = snapshot.state.clone();
        Arc::make_mut(&mut unowned.baseline).logical_source_input = None;
        let unowned = Arc::new(SourceLookupSnapshot::new(unowned));
        assert!(
            original_site_operand_layout_advice(
                &site,
                &tokens,
                &unowned,
                &point.namespace_key,
                config,
            )
            .is_none()
        );
        let mut foreign_source = snapshot.state.clone();
        foreign_source.current_source_origin = None;
        let foreign_source = Arc::new(SourceLookupSnapshot::new(foreign_source));
        assert!(
            original_site_operand_layout_advice(
                &site,
                &tokens,
                &foreign_source,
                &point.namespace_key,
                config,
            )
            .is_none()
        );
    }

    #[test]
    fn constructed_builder_argv_preserves_retained_lexer_overlays() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = current.commands();
        let unit = CompilationUnit::build_for_dialect("set x 1", registry, false, "tcl8.6");
        let bindings = ModuleCommandBindings::analyse(&unit.ir_module, registry);
        let input = unit.ir_module.source_metadata_input.as_ref().unwrap();
        let mut config = input.lexer_config();
        config.escapes = tcl_dialect::EscapeSyntax::Tcl84;
        let overlay = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            config,
        );
        let spelling = r"[list \U00000041]";
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, &overlay,
        )
        .unwrap();
        assert_eq!(
            constructed_script_words_with_metadata_context(
                spelling,
                registry,
                &bindings,
                "::",
                Some(metadata),
            ),
            Some(vec!["U00000041".to_owned()]),
        );
        assert_eq!(
            constructed_script_words_with_metadata_context(
                spelling,
                registry,
                &bindings,
                "::",
                Some(current.into()),
            ),
            Some(vec!["A".to_owned()]),
        );
        assert_eq!(
            constructed_script_words(spelling, registry, &bindings, "::"),
            Some(vec!["A".to_owned()]),
        );
        assert_eq!(
            constructed_script_words_with_metadata_context(
                spelling, registry, &bindings, "::", None,
            ),
            None,
        );
    }

    #[test]
    fn literal_procedure_returns_preserve_later_dispatch_points() {
        let source = "proc helper {mode} {return $mode}; helper a; helper b; set final 1";
        let analysis = allocation_source(source);
        for head in ["helper a", "helper b", "set final"] {
            let offset = u32::try_from(source.find(head).unwrap()).unwrap();
            let proof =
                analysis.invocation_at_source(head.split_whitespace().next().unwrap(), offset);
            assert!(proof.proved_target().is_some(), "{head}: {proof:#?}");
        }
    }

    #[test]
    fn hypothetical_lookup_does_not_inherit_a_different_selected_operation() {
        let source = "set x 1";
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let analysis = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(owner.commands().profile()),
            owner.commands(),
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                )),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let actual = analysis.invocation_at_source("set", 0);
        assert!(matches!(
            actual.native_compilation_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. }
        ));
        for proof in [
            analysis.projection_at_source("list", 0),
            analysis.invocation_unpositioned("list"),
        ] {
            assert!(
                proof
                    .proved_target()
                    .is_some_and(|target| nqn(&target.command) == "::list")
            );
            assert_eq!(
                proof.native_compilation_selection(),
                tcl_registry::native_compilation::NativeCompilationSelection::Unknown
            );
            assert!(proof.native_handler_envelope.is_none());
            assert_eq!(
                proof.compiled_candidates,
                [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
            );
        }
        let unknown = analysis.projection_at_source("missing", 0);
        assert_eq!(unknown.targets, [] as [SourceCommandTarget; 0]);
        assert_eq!(
            unknown.native_compilation_selection(),
            tcl_registry::native_compilation::NativeCompilationSelection::Unknown
        );
    }

    #[test]
    fn whole_source_head_cache_preserves_proof_and_invalidates_on_point_changes() {
        let mut bindings = allocation_source("set x 1; set x 2");
        let first = bindings.invocation_unpositioned("set");
        let second = bindings.invocation_unpositioned("set");
        assert_eq!(first, second);
        assert!(Arc::ptr_eq(
            first.lookup_state.as_ref().unwrap(),
            second.lookup_state.as_ref().unwrap()
        ));
        assert_eq!(bindings.unpositioned_projections.0.lock().unwrap().len(), 1);
        let cloned = bindings.clone();
        assert!(cloned.unpositioned_projections.0.lock().unwrap().is_empty());
        assert_eq!(cloned.invocation_unpositioned("set"), first);
        let mut changed = bindings
            .points
            .iter()
            .find(|point| point.dispatch)
            .unwrap()
            .clone();
        changed.state.mark_opaque_binding_mutation();
        bindings.record_point(changed);
        assert!(
            bindings
                .unpositioned_projections
                .0
                .lock()
                .unwrap()
                .is_empty()
        );
        assert!(bindings.invocation_unpositioned("set").unknown);
    }

    #[test]
    fn cached_source_origin_fingerprints_do_not_replace_semantic_identity() {
        let mut left = SourceOriginId::authored(&Arc::from("proc p {} {return LEFT}"));
        let mut right = SourceOriginId::authored(&Arc::from("proc p {} {return RIGHT}"));
        left.fingerprint = 7;
        right.fingerprint = 7;
        assert_ne!(left, right);
        assert_ne!(left.cmp(&right), std::cmp::Ordering::Equal);
        let mut identical = SourceOriginId::authored(&Arc::from("proc p {} {return LEFT}"));
        identical.fingerprint = 7;
        assert_eq!(left, identical);
    }

    #[test]
    fn activation_source_ids_check_full_equality_and_keep_equal_live_clones() {
        let left = Arc::new(SourceOriginId::authored(&Arc::from("left")));
        let right = Arc::new(SourceOriginId::authored(&Arc::from("right")));
        let mut interner = SourceOriginInterner::default();
        let collision_key = interner.intern(&left.kind, 7);
        assert_ne!(collision_key, interner.intern(&right.kind, 7));
        let retained_payload = Arc::clone(&left.kind);
        assert_eq!(collision_key, interner.intern(&retained_payload, 7));
        let first = SourceOriginId::activation_key(&left);
        assert_ne!(first, SourceOriginId::activation_key(&right));
        let clone = Arc::new(left.as_ref().clone());
        assert_eq!(first, SourceOriginId::activation_key(&clone));
        drop(left);
        let equal = Arc::new(clone.as_ref().clone());
        assert_eq!(first, SourceOriginId::activation_key(&equal));
        let derived = Arc::new(SourceOriginId::derived(
            CommandAllocationSite {
                source: Arc::clone(&equal),
                offset: 0,
            },
            vec![1],
            &Arc::from("left"),
            MaterialisedSourceKind::Script,
        ));
        assert_ne!(first, SourceOriginId::activation_key(&derived));
    }

    #[test]
    fn known_concatenated_script_mutates_the_shared_command_table() {
        let source = "eval {rename set} {saved}; set x 1";
        let analysis = allocation_source(source);
        let binding = analysis
            .invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
        assert!(binding.proved_target().is_none());
        assert!(!analysis.has_site(6));
    }

    #[test]
    fn derived_procedure_body_retains_its_source_and_completion() {
        let source = "eval {proc p {} } {{return; rename set saved}}; p; set x 1";
        let analysis = allocation_source(source);
        let call = analysis.invocation_at_source(
            "p",
            u32::try_from(source.find("; p;").unwrap()).unwrap() + 2,
        );
        let target = call.proved_target().unwrap();
        assert!(matches!(
            target
                .implementation_allocation
                .as_ref()
                .unwrap()
                .site
                .source
                .kind(),
            SourceOriginKind::Derived { .. }
        ));
        assert!(!target.matches_authored_implementation(source, 0));
        let final_set = analysis
            .invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
        assert!(
            final_set
                .proved_target()
                .is_some_and(|target| target.registry_backed)
        );
    }

    #[test]
    fn derived_procedure_mutation_reaches_the_authored_continuation() {
        let source = "eval {proc p {} } {{rename set saved}}; p; set x 1";
        let analysis = allocation_source(source);
        let final_set = analysis
            .invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
        assert!(final_set.proved_target().is_none());
    }

    #[test]
    fn repeated_definition_site_allocates_distinct_tokens_after_a_move() {
        let source = "proc make {} {proc ::f {} {}}; make; rename f old; make; list ready";
        let analysis = allocation_source(source);
        let point = analysis
            .invocation_at_source("list", u32::try_from(source.find("list").unwrap()).unwrap());
        let old = point.lookup_command_word("old");
        let new = point.lookup_command_word("f");
        let old = old.proved_target().unwrap().identity.as_ref().unwrap();
        let new = new.proved_target().unwrap().identity.as_ref().unwrap();
        assert_ne!(old, new);
        assert_eq!(
            old.allocation.as_ref().unwrap().incarnation,
            AllocationIncarnation::First
        );
        assert_eq!(
            new.allocation.as_ref().unwrap().incarnation,
            AllocationIncarnation::Second
        );
    }

    #[test]
    fn repeated_fresh_implementation_cannot_become_a_unique_runtime_token() {
        let source = "proc make {} {proc ::f {} {}}; make; make; make; list ready";
        let analysis = allocation_source(source);
        let point = analysis
            .invocation_at_source("list", u32::try_from(source.find("list").unwrap()).unwrap());
        let implementation = point.lookup_command_word("f");
        assert!(implementation.unknown);
        assert!(implementation.proved_target().is_none());
    }

    #[test]
    fn derived_expression_effects_are_resolved_after_native_concatenation() {
        let source = "catch {expr {1 +} {[rename set saved]}}; set x 1";
        let analysis = allocation_source(source);
        let final_set = analysis
            .invocation_at_source("set", u32::try_from(source.rfind("set").unwrap()).unwrap());
        assert!(final_set.proved_target().is_none());
    }

    #[test]
    fn frozen_script_values_execute_in_the_shared_source_world() {
        for source in [
            "set script {rename set saved}; catch $script; set x 1",
            "set script {rename ::set ::saved}; namespace eval n $script; set x 1",
            "set script {rename set saved}; proc p {} $script; p; set x 1",
            "interp alias {} run {} catch {rename set saved}; run; set x 1",
        ] {
            let analysis = allocation_source(source);
            let last = u32::try_from(source.rfind("set x").unwrap()).unwrap();
            assert!(
                analysis
                    .invocation_at_source("set", last)
                    .proved_target()
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn explicit_availability_phase_is_part_of_retained_point_identity() {
        use tcl_dialect::model::InvocationRealm;
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let source = "set x 1";
        let analyse = |invocation_realm| {
            SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::for_file_grammar(registry.profile().unwrap().grammar),
                registry,
                SourceAnalysisOptions {
                    invocation_realm,
                    ..SourceAnalysisOptions::default()
                },
            )
        };
        let loader = analyse(InvocationRealm::RuleLoader).invocation_at_source("set", 0);
        let runtime = analyse(InvocationRealm::InterpreterRuntime).invocation_at_source("set", 0);
        assert_eq!(loader.invocation_realm(), Some(InvocationRealm::RuleLoader));
        assert_eq!(
            runtime.invocation_realm(),
            Some(InvocationRealm::InterpreterRuntime)
        );
        assert_ne!(loader.lookup_state, runtime.lookup_state);
        assert_eq!(
            runtime.lookup_command_word("gets").invocation_realm(),
            Some(InvocationRealm::InterpreterRuntime)
        );
        assert!(
            runtime
                .lookup_command_word("gets")
                .proved_target()
                .is_some()
        );
        assert!(
            runtime
                .lookup_command_word("open")
                .proved_target()
                .is_none()
        );
        let mut joined = loader;
        joined.join(&runtime);
        assert_eq!(joined.invocation_realm(), None);
    }

    #[test]
    fn retained_computed_procedure_body_preserves_its_abrupt_completion() {
        let source = "set script {return; rename set saved}; proc p {} $script; p; set x 1";
        let analysis = allocation_source(source);
        let last = u32::try_from(source.rfind("set x").unwrap()).unwrap();
        assert!(
            analysis
                .invocation_at_source("set", last)
                .proved_target()
                .is_some_and(|target| target.registry_backed)
        );
        let original_body = u32::try_from(source.find("return").unwrap()).unwrap();
        assert!(
            !analysis.has_site(original_body),
            "a stored value does not execute at its original literal location"
        );
    }

    #[test]
    fn lambda_invocation_binds_actual_values_in_the_selected_namespace() {
        let source = "namespace eval n {}; apply {{name} {proc $name {} {}} n} helper; list ready";
        let analysis = allocation_source(source);
        let site = u32::try_from(source.find("list ready").unwrap()).unwrap();
        let point = analysis.invocation_at_source("list", site);
        assert!(
            point
                .lookup_command_word("::n::helper")
                .proved_target()
                .is_some_and(|target| target.kind == BindingKind::Proc),
            "{point:#?}"
        );
        assert!(
            point
                .lookup_command_word("::helper")
                .proved_target()
                .is_none()
        );
    }

    #[test]
    fn lambda_uses_its_actual_caller_frame_and_stops_at_return() {
        let source = "set n 1; apply {{name} {upvar 1 $name v; incr v; return; rename set saved}} n; list $n";
        let analysis = allocation_source(source);
        let site = u32::try_from(source.find("list $n").unwrap()).unwrap();
        let point = analysis.invocation_at_source("list", site);
        assert_eq!(point.evaluated_argument_values, vec![Some("2".to_owned())]);
        assert!(
            point
                .lookup_command_word("set")
                .proved_target()
                .is_some_and(|target| target.registry_backed)
        );
    }

    #[test]
    fn trusted_loader_requires_the_audited_native_engine() {
        use tcl_dialect::model::Family;
        let registry = CommandRegistry::build_default();
        let loader = TrustedPackageLoader {
            required_core_family: Some(Family::Tcl),
            package: "audited".to_owned(),
            version: Some("1.0".to_owned()),
            implementation_id: "audited-implementation".to_owned(),
            command_surface: vec!["::audited::run".to_owned()],
            compiler_hooks: BTreeMap::new(),
            definition_dispatchers: std::collections::BTreeSet::new(),
            optional_command_surface: Vec::new(),
            namespace_exports: BTreeMap::new(),
            optional_namespace_exports: BTreeMap::new(),
            lookup_dependencies: Vec::new(),
            installed_lookup_dependencies: Vec::new(),
            state_dependency_namespaces: Vec::new(),
            modelled_state_variables: Vec::new(),
        };
        for family in [
            Some(Family::Tcl),
            Some(Family::Jim),
            Some(Family::F5Tcl),
            None,
        ] {
            let mut dialect =
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
            dialect.native_family = family;
            let mut state = ModuleCommandBindings::initial_with_options(
                &registry,
                SourceAnalysisOptions {
                    trusted_package_loaders: std::slice::from_ref(&loader),
                    invocation_dialect: Some(dialect),
                    ..SourceAnalysisOptions::default()
                },
                None,
            );
            apply_package_transition(
                &mut state,
                &tcl_registry::model::binding::PackageTransition::Require {
                    package: TransitionSubject::Literal("audited".to_owned()),
                    requirements: Vec::new(),
                    exact: false,
                },
            );
            assert_eq!(
                state.loaded_provider("audited").is_some(),
                family == Some(Family::Tcl),
                "engine {family:?}"
            );
        }
    }

    #[test]
    fn conditional_expression_mutation_reaches_the_selected_body() {
        let source = "if {[rename set saved; expr 1]} {set x 1}";
        let proof = source_proof(
            source,
            "set",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert!(
            proof
                .proved_target()
                .is_none_or(|target| target.command != "::set")
        );
    }

    #[test]
    fn source_pending_return_never_executes_dead_command_restoration() {
        let source = "rename set stockset; proc set {args} {return CUSTOM}; proc p {} {return; rename set {}; rename stockset set}; p; set x 1";
        let binding = source_proof(
            source,
            "set",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert!(
            binding
                .proved_target()
                .is_none_or(|target| !target.registry_backed)
        );
    }

    #[test]
    fn source_invocation_freezes_values_before_later_argument_effects() {
        let source = "set v OLD; list $v [set v NEW]";
        let binding = source_proof(
            source,
            "list",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            binding
                .evaluated_argument_values
                .first()
                .and_then(Option::as_deref),
            Some("OLD")
        );
    }

    #[test]
    fn source_literal_callee_parameters_select_the_actual_caller_alias() {
        let source = "proc bump {name} {upvar 1 $name v; incr v}; set n 1; bump n; list $n";
        let binding = source_proof(
            source,
            "list",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            binding
                .evaluated_argument_values
                .first()
                .and_then(Option::as_deref),
            Some("2"),
            "{binding:#?}"
        );
        assert!(
            binding
                .proved_target()
                .is_some_and(|target| target.command == "::list")
        );
    }

    #[test]
    fn source_catch_resumes_after_a_pending_return_without_running_dead_mutations() {
        let source = "catch {return; rename set {}}; set x 1";
        let binding = source_proof(
            source,
            "set",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert!(
            binding
                .proved_target()
                .is_some_and(|target| target.command == "::set")
        );
    }

    #[test]
    fn source_abrupt_substitution_skips_later_words_and_outer_dispatch() {
        for source in [
            "catch {puts \"[return stop][rename set oldSet]\"}; set final 1",
            "catch {puts $a([return stop][rename set oldSet])}; set final 1",
        ] {
            let binding = source_proof(
                source,
                "set",
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
            );
            assert!(
                binding
                    .proved_target()
                    .is_some_and(|target| target.command == "::set"),
                "{source}"
            );
        }
        let source = "catch {expr {[return stop] + [rename puts oldPuts]}}; puts final";
        let binding = source_proof(
            source,
            "puts",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert!(
            binding
                .proved_target()
                .is_some_and(|target| target.command == "::puts")
        );
    }

    #[test]
    fn source_math_callee_preserves_return_boundary_and_global_effects() {
        let source = "proc ::tcl::mathfunc::f {x} {global y; set y 1; return 5; rename puts dead}; expr {f(0)}; list $y";
        let binding = source_proof(
            source,
            "list",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            binding
                .evaluated_argument_values
                .first()
                .and_then(Option::as_deref),
            Some("1"),
            "{binding:#?}"
        );
        assert!(
            binding
                .lookup_command_word("puts")
                .proved_target()
                .is_some_and(|target| target.command == "::puts")
        );
    }

    #[test]
    fn source_c_import_follows_the_original_command_object() {
        let source = "namespace eval a {proc f {} {return OLD}; namespace export f}\nnamespace eval b {namespace import ::a::f}\nrename ::a::f ::a::saved\nproc ::a::f {} {return NEW}\n::b::f";
        let binding = source_proof(
            source,
            "::b::f",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        let target = binding.proved_target().expect("stable imported command");
        assert_eq!(target.command, "::a::saved");
        assert_eq!(target.identity.as_ref().unwrap().origin, "::a::f");
    }

    #[test]
    fn source_renamed_procedure_executes_its_retained_implementation_generation() {
        let source = "proc p {} {rename set saved_set}\nrename p saved_p\nproc p {} {}\nsaved_p\nset after 1";
        assert!(
            source_proof(
                source,
                "set after",
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
            )
            .proved_target()
            .is_none()
        );
    }

    #[test]
    fn source_jim_import_retargets_the_original_source_name() {
        let source = "namespace eval a {proc f {} {return OLD}; namespace export f}\nnamespace eval b {namespace import ::a::f}\nrename ::a::f ::a::saved\nproc ::a::f {} {return NEW}\n::b::f";
        let binding = source_proof(
            source,
            "::b::f",
            tcl_registry::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            )),
        );
        assert_eq!(
            binding
                .proved_target()
                .expect("name-based imported command")
                .command,
            "::a::f"
        );
    }

    #[test]
    fn source_alias_expands_prefixes_after_late_target_lookup() {
        let source = "interp alias {} outer {} inner A\ninterp alias {} inner {} set B\nouter C";
        let binding = source_proof(
            source,
            "outer",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        let target = binding.proved_target().expect("effective alias target");
        assert_eq!(target.command, "::set");
        assert_eq!(
            target.prepended,
            [
                crate::registry_invocation::EffectiveInvocationWord::Literal("B".to_owned()),
                crate::registry_invocation::EffectiveInvocationWord::Literal("A".to_owned()),
            ]
        );
    }

    #[test]
    fn source_original_empty_and_native_method_classes_have_bounded_construction() {
        for body in ["", "method local {} {return OK}"] {
            let source = format!("oo::class create Dog {{{body}}}\nset dog [Dog new]\nset after 1");
            let binding = source_proof(
                &source,
                "set after",
                tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
            );
            assert_eq!(
                binding
                    .proved_target()
                    .expect("default construction retains command proof")
                    .command,
                "::set"
            );
        }
    }

    #[test]
    fn source_constructor_and_rebound_member_implementations_withdraw_construction_proof() {
        for source in [
            "oo::class create Dog {constructor {} {rename set saved_set}}\nset dog [Dog new]\nset after 1",
            "proc ::oo::define::method {args} {rename set saved_set}\noo::class create Dog {method local {} {}}\nset dog [Dog new]\nset after 1",
            "oo::objdefine ::oo::class {filter hidden}\noo::class create Dog {}\nset dog [Dog new]\nset after 1",
        ] {
            assert!(
                source_proof(
                    source,
                    "set after",
                    tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
                )
                .proved_target()
                .is_none()
            );
        }
    }

    #[test]
    fn source_repeated_body_joins_future_rebindings_before_specialisation() {
        let source = "foreach x {a b} {if {$x eq {a}} {proc set {args} {return CUSTOM}}; set y1}";
        let binding = source_proof(
            source,
            "set y1",
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert!(binding.proved_target().is_none());
    }

    /// A `namespace import` that shadows a proc declared in the same
    /// namespace must distrust that binding — tclsh answers `EVIL` for the
    /// sheet below, so folding `::ns::answer` with the declared body is
    /// wrong. A *globbed* import elsewhere must not distrust the source
    /// namespace it imports *from*, or an ordinary `namespace import ::lib::*`
    /// would stop every fold in `::lib`.
    #[test]
    fn an_import_shadowing_a_declared_proc_distrusts_only_its_namespace() {
        let reg = CommandRegistry::build_default();
        let shadowed = "namespace eval ::evil { proc answer {} {return EVIL} ; namespace export answer }\n             namespace eval ::ns { proc answer {} { return 42 } ; namespace import -force ::evil::answer }\n";
        let cu = CompilationUnit::build_for(shadowed, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts_proc_binding("::ns::answer"),
            "the import shadows the declared proc"
        );
        assert!(
            m.trusts_proc_binding("::evil::answer"),
            "the namespace imported *from* is untouched"
        );

        // A wildcard import makes only its target namespace opaque.
        let wildcard = "namespace eval ::lib { proc helper {} { return 1 } ; namespace export helper }\n             namespace eval ::app { namespace import ::lib::* }\n";
        let cu = CompilationUnit::build_for(wildcard, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts_proc_binding("::app::helper"));
        assert!(
            m.trusts_proc_binding("::lib::helper"),
            "the imported-from namespace still folds"
        );
    }

    /// A `TclOO` method runs in the receiver's namespace, chosen at run time,
    /// so a command it names relatively may resolve to an object-local
    /// implementation. That uncertainty belongs to the frame: it disqualifies
    /// facts derived inside the body, and leaves the names the top level and
    /// the procedures resolve untouched.
    #[test]
    fn an_ordinary_method_body_scopes_its_opacity_to_its_own_frame() {
        let reg = CommandRegistry::build_default();
        for body in [
            "puts hi",
            "my helper",
            "next",
            "set y 1",
            "if {1} { puts hi }",
        ] {
            let src = format!(
                "oo::class create ::A {{\n    method m {{}} {{ {body} }}\n}}\nproc p {{}} {{ return 1 }}\n"
            );
            let cu = CompilationUnit::build_for(&src, &reg, false);
            let m = scan_module_command_mutations(&cu.ir_module, &reg);
            assert!(
                m.has_runtime_selected_frames(),
                "{body}: the frame itself stays opaque"
            );
            assert!(
                m.trusts("string"),
                "{body}: a builtin the module never touches must stay trusted"
            );
            assert!(
                m.trusts_proc_binding("::p"),
                "{body}: a proc declared outside the class must stay trusted"
            );
        }
    }

    /// A binding change from such a frame names its subject where this scan
    /// cannot look, so it carries past the frame and no name in the module
    /// survives it.
    #[test]
    fn a_binding_change_in_a_runtime_selected_frame_distrusts_the_module() {
        let reg = CommandRegistry::build_default();
        for body in [
            "rename puts myputs",
            "proc helper {} { return 1 }",
            "interp alias {} shout {} puts",
            "namespace import ::lib::*",
        ] {
            let src = format!(
                "oo::class create ::A {{\n    method m {{}} {{ {body} }}\n}}\nproc p {{}} {{ return 1 }}\n"
            );
            let cu = CompilationUnit::build_for(&src, &reg, false);
            let m = scan_module_command_mutations(&cu.ir_module, &reg);
            assert!(
                !m.trusts_proc_binding("::p"),
                "{body}: an unspellable subject must distrust the module"
            );
        }
    }

    /// A rebinding does not stop carrying past the frame because it sits in a
    /// statically lowered `eval` or `uplevel`. Those lower to
    /// `Statement::Block` / `Statement::UpFrame`, whose bodies are part of the
    /// frame like any other nested script: `m` really does remove `::string`,
    /// so a fold of `[string length x]` elsewhere would be a miscompile.
    #[test]
    fn a_rebinding_inside_a_lowered_eval_or_uplevel_still_distrusts_the_module() {
        let reg = CommandRegistry::build_default();
        for body in [
            "puts hi; eval {rename ::string ::saved}",
            "puts hi; uplevel 1 {rename ::string ::saved}",
            "puts hi; if {1} { eval {rename ::string ::saved} }",
        ] {
            let src = format!(
                "oo::class create ::A {{\n    method m {{}} {{ {body} }}\n}}\nproc p {{}} {{ return 1 }}\n"
            );
            let cu = CompilationUnit::build_for(&src, &reg, false);
            let m = scan_module_command_mutations(&cu.ir_module, &reg);
            assert!(
                !m.trusts("string"),
                "{body}: a rebinding in a lowered body must distrust the module"
            );
        }
    }

    /// A body whose commands all resolve absolutely is namespace-invariant, so
    /// it takes the ordinary scan and leaves no frame opacity behind.
    #[test]
    fn an_absolute_only_method_body_leaves_no_frame_opacity() {
        let reg = CommandRegistry::build_default();
        let src =
            "oo::class create ::A {\n    method m {} { ::puts hi }\n}\nproc p {} { return 1 }\n";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.has_runtime_selected_frames());
        assert!(m.trusts_proc_binding("::p"));
        assert!(m.trusts("puts"));
    }

    /// The snapshot is the per-procedure lattice memo's trust fact, so every
    /// field of the summary must survive the round trip and take part in the
    /// key: a memo taken under one binding state must never answer for
    /// another. The literal names every field, so a field added to the
    /// summary fails to compile here until it is carried.
    #[test]
    fn the_trust_snapshot_round_trips_every_mutation_field() {
        let every = ModuleCommandMutations {
            names: std::iter::once("llength".to_owned()).collect(),
            rebound: std::iter::once("p".to_owned()).collect(),
            dynamic: true,
            rebinding_subjects: RebindingSubjects::SomeUnnameable,
            runtime_selected_frames: true,
            resolution_changed: true,
            opaque_namespaces: std::iter::once("::ns".to_owned()).collect(),
        };
        assert_eq!(every.snapshot().to_mutations(), every);
        let base = ModuleCommandMutations::default();
        let one_field: [ModuleCommandMutations; 7] = [
            ModuleCommandMutations {
                names: every.names.clone(),
                ..base.clone()
            },
            ModuleCommandMutations {
                rebound: every.rebound.clone(),
                ..base.clone()
            },
            ModuleCommandMutations {
                dynamic: true,
                ..base.clone()
            },
            ModuleCommandMutations {
                rebinding_subjects: RebindingSubjects::SomeUnnameable,
                ..base.clone()
            },
            ModuleCommandMutations {
                runtime_selected_frames: true,
                ..base.clone()
            },
            ModuleCommandMutations {
                resolution_changed: true,
                ..base.clone()
            },
            ModuleCommandMutations {
                opaque_namespaces: every.opaque_namespaces.clone(),
                ..base.clone()
            },
        ];
        for m in &one_field {
            assert_eq!(&m.snapshot().to_mutations(), m);
            assert_ne!(m.snapshot(), base.snapshot(), "{m:?} is part of the key");
        }

        // A scanned summary answers both trust stances alike from the
        // snapshot: `rename $a {}` names no subject, so no builtin is
        // claimable (#2168), and an unknown head raises only the top.
        let reg = CommandRegistry::build_default();
        for src in [
            "set a llength\nrename $a {}\n",
            "someUnknownLibraryCall x\n",
            "proc llength {l} { return 99 }\n",
        ] {
            let cu = CompilationUnit::build_for(src, &reg, false);
            let scanned = scan_module_command_mutations(&cu.ir_module, &reg);
            let restored = scanned.snapshot().to_mutations();
            assert_eq!(restored, scanned, "{src:?}");
            for name in ["llength", "list", "set"] {
                assert_eq!(
                    restored.observed_binding_is_the_builtin(name),
                    scanned.observed_binding_is_the_builtin(name),
                    "{src:?} {name}"
                );
                assert_eq!(
                    restored.trusts(name),
                    scanned.trusts(name),
                    "{src:?} {name}"
                );
            }
        }
    }

    /// The frame fact is part of the memo key: a summary taken with every
    /// frame statically nameable must not be reused once one is not.
    #[test]
    fn the_frame_fact_survives_a_snapshot_round_trip() {
        let reg = CommandRegistry::build_default();
        let src = "oo::class create ::A {\n    method m {} { puts hi }\n}\n";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(m.has_runtime_selected_frames());
        assert!(m.snapshot().to_mutations().has_runtime_selected_frames());
        assert_ne!(
            m.snapshot(),
            ModuleCommandMutations::default().snapshot(),
            "the frame fact must distinguish the two summaries"
        );
    }

    #[test]
    fn independently_analysed_summaries_are_semantically_equal() {
        let (cu, reg) = analyse("interp alias {} e {} expr\ne {1 + 1}");
        let first = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let second = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

        assert!(!Arc::ptr_eq(&first.bindings, &second.bindings));
        assert!(!Arc::ptr_eq(
            &first.root_boundary_bindings,
            &second.root_boundary_bindings
        ));
        assert_eq!(first, second);
        assert!(
            !first.same_state(&second),
            "the analysis-local fast path deliberately requires one shared baseline Arc"
        );
    }

    #[test]
    fn binding_state_forks_copy_procedure_inventory_only_for_a_new_body() {
        let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
        let original = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let mut fork = original.clone();

        assert!(Arc::ptr_eq(
            &original.procedure_bodies,
            &fork.procedure_bodies
        ));
        assert!(original.same_state(&fork));

        assert!(!fork.extend_procedure_bodies(["::first".to_owned()]));
        assert!(
            Arc::ptr_eq(&original.procedure_bodies, &fork.procedure_bodies),
            "re-observing a retained body must not detach the shared inventory"
        );

        assert!(fork.extend_procedure_bodies(["::recovered".to_owned()]));
        assert!(!Arc::ptr_eq(
            &original.procedure_bodies,
            &fork.procedure_bodies
        ));
        assert!(!original.procedure_bodies.contains("::recovered"));
        assert!(fork.procedure_bodies.contains("::recovered"));
        assert!(!original.same_state(&fork));
    }

    #[test]
    fn binding_state_forks_copy_binding_map_only_for_a_changed_entry() {
        let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
        let original = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let mut unchanged = original.clone();

        assert!(Arc::ptr_eq(&original.bindings, &unchanged.bindings));
        assert!(Arc::ptr_eq(
            &original.root_boundary_bindings,
            &unchanged.root_boundary_bindings
        ));
        assert!(!unchanged.join(&original));
        assert!(
            Arc::ptr_eq(&original.bindings, &unchanged.bindings),
            "joining an identical fork must not detach the shared map"
        );

        let (existing_name, existing_bindings) = original
            .bindings
            .iter()
            .next()
            .expect("procedure analysis publishes at least one binding");
        unchanged.replace(existing_name.clone(), existing_bindings.clone());
        assert!(
            Arc::ptr_eq(&original.bindings, &unchanged.bindings),
            "replacing an entry with its current value must not detach the shared map"
        );

        let mut changed = original.clone();
        changed.replace(
            "::recovered".to_owned(),
            BTreeSet::from([MayBinding::Unknown]),
        );
        assert!(!Arc::ptr_eq(&original.bindings, &changed.bindings));
        assert!(!original.bindings.contains_key("::recovered"));
        assert_eq!(
            changed.bindings.get("::recovered"),
            Some(&BTreeSet::from([MayBinding::Unknown]))
        );
        assert!(
            Arc::ptr_eq(
                &original.root_boundary_bindings,
                &changed.root_boundary_bindings
            ),
            "a flow-state mutation must not copy the immutable boundary map"
        );
        assert!(Arc::ptr_eq(
            &original.procedure_bodies,
            &changed.procedure_bodies
        ));
        assert!(!original.same_state(&changed));
    }

    #[test]
    fn exact_procedure_delta_preserves_history_and_root_boundary() {
        let (cu, reg) = analyse("proc first {} {return 1}\nproc second {} {return 2}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let history = bindings
            .bindings
            .get("::first")
            .expect("historical procedure slot");
        let procedure_target = history
            .iter()
            .find_map(|binding| match binding {
                MayBinding::Target(target) if target.kind == BindingKind::Proc => Some(target),
                MayBinding::Target(_)
                | MayBinding::Imported(_)
                | MayBinding::Missing
                | MayBinding::Unknown => None,
            })
            .expect("actual installed procedure");
        assert_eq!(procedure_target.command, "::first");
        assert!(!procedure_target.registry_backed);
        assert!(procedure_target.prepended.is_empty());
        let allocation = procedure_target
            .implementation_allocation
            .as_ref()
            .expect("actual definition allocation");
        assert_eq!(allocation.site.offset, 0);
        assert_eq!(allocation.command, "::first");
        assert_eq!(
            procedure_target
                .token
                .as_ref()
                .and_then(|identity| identity.allocation.as_ref()),
            Some(allocation)
        );
        let procedure_target = MayBinding::Target(procedure_target.clone());

        assert_eq!(
            bindings.bindings.get("::first"),
            Some(&BTreeSet::from([
                MayBinding::Missing,
                procedure_target.clone()
            ])),
            "the historical view includes the state before and after definition"
        );
        assert_eq!(
            bindings.root_boundary_bindings.get("::first"),
            Some(&BTreeSet::from([procedure_target])),
            "only the post-definition binding is replayable at another root"
        );
    }

    #[test]
    fn exact_procedure_delta_retains_non_binding_opacity() {
        let mut reg = CommandRegistry::build_default();
        let mut external_proc = reg.get("proc").expect("core proc spec").clone();
        external_proc.traits |= tcl_registry::Traits::LOADS_EXTERNAL_UNIT;
        reg.insert(external_proc);

        // The leading command ensures the observed state already exists when
        // the exact definition runs. A one-key fast path here would suppress
        // the ordinary observation and lose the external-unit opacity that
        // accompanies the otherwise-exact Define(Procedure).
        let cu =
            CompilationUnit::build_for("set marker 1\nproc created {} {return ok}", &reg, false);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

        assert!(bindings.opaque_binding_mutation);
        assert!(bindings.has_opaque_domain());
        assert!(
            bindings.mutation_projection(&reg).has_dynamic_mutation(),
            "the historical mutation projection must retain external-unit opacity"
        );
    }

    #[test]
    fn exact_procedure_delta_retains_immediate_body_binding_mutation() {
        let mut reg = CommandRegistry::build_default();
        let mut immediate_proc = reg.get("proc").expect("core proc spec").clone();
        immediate_proc
            .traits
            .remove(tcl_registry::Traits::DEFERS_BODY);
        immediate_proc.frame_effect = Some(tcl_registry::FrameEffectSpec {
            level_word: tcl_registry::FrameLevelWord::None,
            layout: tcl_registry::FrameArgLayout::ScriptInCurrentFrame,
        });
        reg.insert(immediate_proc);

        // This authored Proc-lowering operation both defines `created` and
        // immediately evaluates its body. The nested definition changes a
        // second binding without necessarily growing any non-binding axis:
        // both procedure bodies can already be present in the module inventory.
        let cu = CompilationUnit::build_for(
            "set marker 1\nproc created {} {proc set {} {return hijacked}}",
            &reg,
            false,
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

        assert!(
            !bindings.mutation_projection(&reg).trusts("set"),
            "the historical view must retain the immediately evaluated body's builtin redefinition"
        );
    }

    #[test]
    fn narrow_proc_binding_projection_excludes_unavailable_body_opacity() {
        let (cu, reg) = analyse("missing_command argument\nproc retained {} {return ok}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);

        assert!(
            bindings.mutation_projection(&reg).has_dynamic_mutation(),
            "an unresolved command may load code that changes builtin trust"
        );
        assert!(
            bindings
                .proc_binding_trust_projection()
                .trusts_proc_binding("::retained"),
            "unavailable code does not itself prove that a retained proc was rebound"
        );
    }

    #[test]
    fn narrow_proc_binding_projection_tracks_dynamic_rebinding_subjects() {
        let (cu, reg) =
            analyse("set name retained\nrename $name saved\nproc retained {} {return ok}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        let trust = bindings.proc_binding_trust_projection();

        assert!(trust.has_dynamic_binding_transition());
        assert!(!trust.trusts_proc_binding("::retained"));
    }

    #[test]
    fn narrow_proc_binding_projection_retains_static_namespace_candidates() {
        let (cu, reg) = analyse("namespace eval ::n {rename retained saved}");
        let trust =
            ModuleCommandBindings::analyse(&cu.ir_module, &reg).proc_binding_trust_projection();

        assert!(!trust.has_dynamic_binding_transition());
        assert!(!trust.trusts_proc_binding("::n::retained"));
        assert!(
            !trust.trusts_proc_binding("::retained"),
            "the legacy source scan also records a global fallback candidate"
        );
    }

    #[test]
    fn consumed_cfg_commands_retain_the_original_native_entry_baseline() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let (owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = SourceAnalysisEntry {
            native_entry: Some(Arc::new(captured)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = CompilationUnit::build_with_context_registry(
            "set result VALUE",
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            Arc::clone(&context),
        );
        let cfg = &unit.top_level.cfg;
        let statement = &cfg.blocks[&cfg.entry].statements[0];
        assert!(matches!(statement, Statement::AssignConst { .. }));
        assert!(statement.tokens().is_none());
        let original = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
            &cfg.command_binding_sites,
            statement,
        )
        .unwrap();
        assert!(
            original
                .source_binding
                .as_ref()
                .unwrap()
                .proved_execution_target()
                .is_some()
        );
        let selected = analyse_command_binding(cfg, context.commands(), &[]);
        assert!(selected.is_original_builtin_at(cfg.entry, 0, "set"));
        let mut missing = cfg.clone();
        missing.command_binding_sites.clear();
        assert!(missing.statement_sources.values().any(Option::is_some));
        assert!(
            !analyse_command_binding(&missing, context.commands(), &[]).is_original_builtin_at(
                missing.entry,
                0,
                "set"
            )
        );
        let mut conflicting = cfg.clone();
        let mut other = conflicting.command_binding_sites[0].clone();
        other.source_tokens.as_mut().unwrap().argv_texts[0] = "other".into();
        conflicting.command_binding_sites.push(other);
        assert!(
            !analyse_command_binding(&conflicting, context.commands(), &[]).is_original_builtin_at(
                conflicting.entry,
                0,
                "set"
            )
        );
        let mut foreign = context.commands().project_for_profile(profile);
        let mut setter = foreign.get("set").unwrap().clone();
        setter.lowering_hook = None;
        foreign.insert(setter);
        assert!(
            !analyse_command_binding(cfg, &foreign, &[])
                .is_original_builtin_at(cfg.entry, 0, "set")
        );
        drop(owner);
    }

    #[test]
    fn unperturbed_builtin_is_builtin_no_rebound() {
        let (cu, reg) = analyse("string toupper a");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert!(cb.is_original_builtin_at(fu.cfg.entry, 0, "string"));
        assert!(cb.rebound_names().is_empty());
        assert!(!cb.has_wildcard());
    }

    #[test]
    fn class_destroy_makes_the_class_command_opaque() {
        // `Animal destroy` deletes the class command: the binding is Class
        // before the destroy and Opaque after, so a later `Animal new`
        // draws W128.  Definer creation and the destructive method are
        // both registry data (definition_body / oo::object's `destroy`).
        let (cu, reg) = analyse(
            "oo::class create Animal {}
Animal new
Animal destroy
Animal new",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        let animal_calls: Vec<_> = fu.cfg.blocks[&entry]
            .statements
            .iter()
            .enumerate()
            .filter_map(|(index, stmt)| {
                matches!(stmt, Statement::Call { command, .. } | Statement::Barrier { command, .. }
                    if command == "Animal")
                .then_some(index)
            })
            .collect();
        assert_eq!(
            cb.binding_at(entry, animal_calls[0], "Animal").kind,
            BindingKind::Class
        );
        assert_eq!(
            cb.binding_at(entry, animal_calls[2], "Animal").kind,
            BindingKind::Opaque,
            "the class command is deleted after `Animal destroy`"
        );
        assert!(cb.rebound_names().contains("::Animal"));
    }

    #[test]
    fn instance_destroy_makes_the_instance_command_opaque() {
        // `Animal create fido` binds the instance command; `fido destroy`
        // deletes it; the class itself stays bound.
        let (cu, reg) = analyse(
            "oo::class create Animal {}
Animal create fido
fido destroy
fido bark
Animal new",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        let invocation_index = |command: &str, first_arg: &str| {
            fu.cfg.blocks[&entry]
                .statements
                .iter()
                .position(|stmt| {
                    matches!(stmt,
                        Statement::Call { command: source, args, .. }
                        | Statement::Barrier { command: source, args, .. }
                        if source == command
                            && args.first().is_some_and(|arg| arg == first_arg))
                })
                .unwrap()
        };
        assert_eq!(
            cb.binding_at(entry, invocation_index("fido", "bark"), "fido")
                .kind,
            BindingKind::Opaque
        );
        assert_eq!(
            cb.binding_at(entry, invocation_index("Animal", "new"), "Animal")
                .kind,
            BindingKind::Class
        );
    }

    #[test]
    fn closed_static_descriptor_does_not_mask_a_live_object_receiver() {
        let mut reg = CommandRegistry::build_default();
        reg.insert(tcl_registry::CommandSpec {
            name: "Animal",
            state_transitions: Some(tcl_registry::StateTransitionDescriptor::EMPTY),
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let cu = CompilationUnit::build_for(
            "oo::class create Animal {}\nAnimal destroy\nAnimal new",
            &reg,
            false,
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert_eq!(
            cb.binding_at(fu.cfg.entry, 2, "Animal").kind,
            BindingKind::Opaque,
            "the live object binding wins over the shadowed static descriptor"
        );
    }

    #[test]
    fn destroy_as_ordinary_argument_is_not_a_deletion() {
        // A proc named `destroy` taking a class name as an ARGUMENT must
        // not delete anything: the head is the proc, not the class.
        let (cu, reg) = analyse(
            "oo::class create Animal {}
proc destroy {x} { puts $x }
destroy Animal
Animal new",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert_eq!(
            cb.binding_at(entry, 3, "Animal").kind,
            BindingKind::Class,
            "the class survives an unrelated `destroy` call"
        );
    }

    #[test]
    fn snit_type_creation_has_only_its_attested_dispatcher() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let selected =
            crate::provider_fixtures::entry(registry, &[crate::provider_fixtures::Provider::Snit]);
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let source = "package require snit\nsnit::type Dog {}\nDog create d";
        let offset = u32::try_from(source.find("Dog create").unwrap()).unwrap();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            selected.options(),
        );
        let call = bindings.invocation_at_source("Dog", offset);
        assert_eq!(
            call.proved_execution_target().unwrap().kind,
            BindingKind::Command
        );
        assert_eq!(
            call.nominal_definition_name_result(registry),
            Some(tcl_registry::definer::DefinitionDispatcher::SnitType)
        );
        let unprovided = SourceCommandBindings::analyse(source, config, registry);
        assert!(
            unprovided
                .invocation_at_source("Dog", offset)
                .nominal_definition_name_result(registry)
                .is_none()
        );
        let retired_source = "package require snit\nsnit::type Dog {}\nDog destroy\nDog create d";
        let retired_offset = u32::try_from(retired_source.find("Dog create").unwrap()).unwrap();
        let retired = SourceCommandBindings::analyse_with_options(
            retired_source,
            config,
            registry,
            selected.options(),
        );
        assert!(
            retired
                .invocation_at_source("Dog", retired_offset)
                .nominal_definition_name_result(registry)
                .is_none()
        );
    }

    #[test]
    fn rename_deletion_makes_old_name_opaque_flow_sensitively() {
        // `string` is its builtin before the rename, opaque after.
        let (cu, reg) = analyse("string toupper a\nrename string {}\nstring toupper b");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert!(cb.is_original_builtin_at(entry, 0, "string"));
        assert_eq!(
            cb.binding_at(entry, 2, "string").kind,
            BindingKind::Opaque,
            "string is renamed away before stmt 2"
        );
        assert!(cb.rebound_names().contains("::string"));
    }

    #[test]
    fn rename_redirect_moves_binding_to_new_name() {
        let (cu, reg) = analyse("rename string mystr\nmystr toupper b");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        // After the rename: old `string` is opaque, `mystr` inherits the
        // builtin binding `string` denoted.
        assert_eq!(cb.binding_at(entry, 1, "string").kind, BindingKind::Opaque);
        assert_eq!(cb.binding_at(entry, 1, "mystr").kind, BindingKind::Builtin);
    }

    #[test]
    fn proc_redefinition_binds_name_to_proc() {
        let (cu, reg) = analyse("proc string {x} { return $x }\nstring foo");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        let b = cb.binding_at(entry, 1, "string");
        assert_eq!(b.kind, BindingKind::Proc);
        assert_eq!(b.target.as_deref(), Some("::string"));
        assert!(!cb.is_original_builtin_at(entry, 1, "string"));
        assert!(cb.rebound_names().contains("::string"));
    }

    #[test]
    fn dynamic_rename_collapses_to_wildcard() {
        let (cu, reg) = analyse("set x foo\nrename $x bar\nstring toupper a");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert!(cb.has_wildcard(), "dynamic rename sets the wildcard");
        // Under the wildcard everything resolves to Unknown (⊤), never a
        // concrete binding — so no spurious W128 can fire.
        let entry = fu.cfg.entry;
        assert_eq!(cb.binding_at(entry, 2, "string").kind, BindingKind::Unknown);
    }

    #[test]
    fn expanded_embedded_rename_widens_module_bindings() {
        for (source, opaque) in [
            ("set ignored [rename {*}{llength saved_llength}]", false),
            ("set argv [read stdin]; set ignored [rename {*}$argv]", true),
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert_eq!(bindings.has_opaque_domain(), opaque, "{source}");
            if !opaque {
                assert!(
                    bindings
                        .targets("::saved_llength", "::")
                        .iter()
                        .any(|target| target.command == "::llength"),
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn expanded_embedded_command_head_widens_module_bindings() {
        for (source, opaque) in [
            ("set ignored [{*}{rename string saved_string}]", false),
            ("set argv [read stdin]; set ignored [{*}$argv]", true),
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert_eq!(bindings.has_opaque_domain(), opaque, "{source}");
            if !opaque {
                assert!(
                    bindings
                        .targets("::saved_string", "::")
                        .iter()
                        .any(|target| target.command == "::string"),
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn embedded_non_body_ensemble_leaf_preserves_command_trust() {
        let (cu, reg) = analyse("proc p {} {set x [namespace qualifiers ::a::b]}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a resolved non-body namespace leaf must not inherit the root's dynamic-body opacity"
        );
        let mutations = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(mutations.trusts("namespace"));
        assert!(mutations.trusts("self"));
    }

    #[test]
    fn namespace_code_captures_direct_and_embedded_bodies_without_replaying_them() {
        for source in [
            "namespace code {rename set saved_set}",
            "set body {rename set saved_set}\nset callback [namespace code $body]",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "namespace code stores rather than executes its body: {source}"
            );
            assert!(
                !bindings
                    .targets("::saved_set", "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                "a captured namespace code body must not mutate bindings now: {source}"
            );
        }
    }

    #[test]
    fn evaluated_body_interpreter_realm_preserves_only_proven_parent_bindings() {
        for source in [
            "interp create slave\ninterp eval slave {set x 99}",
            "interp create slave\ninterp eval slave {puts child}",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "a harmless readable body in a named child leaves the parent command table unchanged: {source}"
            );
        }

        for source in [
            "interp eval {} {rename set saved_set}",
            "set child slave\ninterp eval $child {set x 99}",
            "interp create slave\ninterp eval slave {rename set saved_set}",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "a current, unknown, or command-mutating child body must fail closed: {source}"
            );
        }
    }

    #[test]
    fn namespace_resolution_transitions_widen_but_creation_and_export_do_not() {
        for source in [
            "namespace eval ::n {}",
            "namespace eval ::n {namespace export *}",
            "set name v\nglobal $name",
            "set name v\nvariable $name",
            "set ns ::n\nnamespace upvar $ns v local",
            "set pattern *\nnamespace export $pattern",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "a namespace or variable-cell transition that preserves lookup must stay closed: {source}"
            );
        }

        for source in [
            "namespace delete ::n",
            "namespace import -force ::m::*",
            "namespace forget ::m::*",
            "namespace path {::m}",
            "namespace unknown handler",
            "namespace ensemble create",
            "set value ::m\nnamespace path $value",
            "set value handler\nnamespace unknown $value",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "a resolution-changing namespace transition must fail closed: {source}"
            );
        }
    }

    #[test]
    fn namespace_resolution_impact_survives_embedding_and_alias_prefixes() {
        for source in [
            "set result [namespace delete ::n]",
            "set result [namespace import -force ::m::*]",
            "set result [namespace forget ::m::*]",
            "set result [namespace path {::m}]",
            "set result [namespace unknown handler]",
            "set result [namespace ensemble create]",
            "interp alias {} mutate {} namespace delete\nmutate ::n",
            "interp alias {} mutate {} namespace import -force\nmutate ::m::*",
            "interp alias {} mutate {} namespace forget\nmutate ::m::*",
            "interp alias {} mutate {} namespace path\nmutate {::m}",
            "interp alias {} mutate {} namespace unknown\nmutate handler",
            "interp alias {} mutate {} namespace ensemble\nmutate create",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "resolved nested dispatch must retain namespace lookup impact: {source}"
            );
            assert!(
                scan_module_command_mutations(&cu.ir_module, &reg).changes_command_resolution(),
                "resolved nested dispatch must reach the shared mutation projection: {source}"
            );
            assert!(
                !scan_module_command_mutations(&cu.ir_module, &reg).has_dynamic_mutation(),
                "namespace lookup provenance must not become an unrelated dynamic proc rebinding: {source}"
            );
        }

        for source in [
            "set result [namespace export *]",
            "interp alias {} ensure_n {} namespace eval ::n {}\nensure_n",
            "interp alias {} export_any {} namespace export\nexport_any *",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "Ensure and Export preserve existing command resolution: {source}"
            );
        }
    }

    #[test]
    fn external_unit_execution_widens_direct_and_embedded_command_bindings() {
        for source in [
            "source external.tcl",
            "set result [source external.tcl]",
            "load extension.so",
            "set result [load extension.so]",
            "auto_load widget",
            "set result [auto_load widget]",
            "auto_import pkg::*",
            "set result [auto_import pkg::*]",
            "package require Example",
            "set result [package require Example]",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "unavailable external code may mutate any command binding: {source}"
            );
        }
    }

    #[test]
    fn direct_unknown_and_unresolved_heads_widen_for_autoload() {
        for source in [
            "unknown missing_command",
            "missing_command argument",
            "set result [missing_command argument]",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "default unknown may autoload a unit that mutates command bindings: {source}"
            );
        }

        let (cu, reg) = analyse("proc local_command {} {return ok}\nlocal_command");
        assert!(
            !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "a retained local procedure is resolved without the unknown path"
        );

        for source in [
            "rename unknown {}\nmissing_command argument",
            "interp alias {} unknown {} puts\nmissing_command argument",
        ] {
            let (cu, reg) = analyse(source);
            assert!(
                !ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
                "a removed or transition-free replacement handler cannot autoload: {source}"
            );
        }

        let (cu, reg) =
            analyse("proc unknown {command args} {rename $command saved_command}\nmissing_command");
        assert!(
            ModuleCommandBindings::analyse(&cu.ir_module, &reg).has_opaque_domain(),
            "a retained dynamic unknown handler body remains opaque"
        );
    }

    #[test]
    fn a_missing_root_binding_dispatches_through_the_live_unknown_handler() {
        let (cu, reg) = analyse(
            "rename unknown old_unknown\n\
             interp alias {} unknown {} rename expr\n\
             proc maybe {} {}\n\
             rename maybe {}\n\
             maybe",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.rebound_names().any(|name| name == "::expr"),
            "calling the missing command must execute the replacement unknown prefix and move expr"
        );
        assert!(
            bindings
                .targets("maybe", "::")
                .iter()
                .any(|target| target.command == "::expr"),
            "the unknown prefix must move expr onto the missing spelling"
        );

        let (cu, reg) = analyse("rename unknown {}\nmissing_command");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(bindings.targets("missing_command", "::").is_empty());
        assert!(!bindings.target_may_be_unknown("missing_command", "::"));
    }

    #[test]
    fn indeterminate_ensemble_dispatch_widens_direct_and_embedded_bodies() {
        for source in [
            "set op eval\nnamespace $op ::n {rename set saved_set}",
            "set op eval\nset result [namespace $op ::n {rename set saved_set}]",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "a dynamic subcommand can select a body-bearing leaf: {source}"
            );
        }
    }

    #[test]
    fn static_apply_replays_binding_changes_in_default_and_explicit_namespaces() {
        for (source, moved_name) in [
            ("apply {{} {rename set saved_set}}", "::saved_set"),
            (
                "namespace eval ::n {}\napply {{} {rename set saved_set} ::n}",
                "::n::saved_set",
            ),
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "a literal apply lambda is exactly replayable: {source}"
            );
            assert!(
                bindings
                    .targets(moved_name, "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                "apply body did not move set to {moved_name}: {source}"
            );
        }
    }

    #[test]
    fn dynamic_apply_lambda_keeps_the_command_domain_opaque() {
        for (source, opaque) in [
            (
                "set lambda {{} {rename set saved_set}}; apply $lambda",
                false,
            ),
            ("set lambda [read stdin]; apply $lambda", true),
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert_eq!(bindings.has_opaque_domain(), opaque, "{source}");
            if !opaque {
                assert!(
                    bindings
                        .targets("::saved_set", "::")
                        .iter()
                        .any(|target| target.command == "::set"),
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn alias_chain_prefix_replays_apply_body_after_arity_validation() {
        let source = "interp alias {} apply_saved {} apply {{} {rename set saved_set}}\n\
                      interp alias {} run_saved {} apply_saved\n\
                      run_saved";
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(!bindings.has_opaque_domain());
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the alias chain must supply apply's lambda"
        );

        for source in [
            "interp alias {} bad {} apply {{x} {rename set saved_set}}\nbad",
            "interp alias {} bad {} apply {{{x y z}} {rename set saved_set}}\nbad",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(!bindings.has_opaque_domain(), "known apply error: {source}");
            assert!(
                !bindings
                    .targets("::saved_set", "::")
                    .iter()
                    .any(|target| target.command == "::set"),
                "an arity or formal-list error must precede the body: {source}"
            );
        }
    }

    #[test]
    fn static_namespace_eval_replays_binding_changes_in_target_namespace() {
        let (cu, reg) = analyse("namespace eval n { rename set saved_set }");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a literal namespace and body are exactly replayable"
        );
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "namespace eval body must run in ::n"
        );
    }

    #[test]
    fn readable_nested_body_retains_intermediate_binding_history() {
        let (cu, reg) =
            analyse("namespace eval ::n { rename ::set ::saved_set; rename ::saved_set ::set }");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the transient binding is observable while the evaluated body runs"
        );
        assert!(
            !bindings
                .root_boundary_bindings
                .get("::saved_set")
                .is_some_and(|states| states.iter().any(|state| matches!(
                    state,
                    MayBinding::Target(target) if target.command == "::set"
                ))),
            "transient history must not become a replayable root boundary"
        );
    }

    #[test]
    fn dynamic_namespace_eval_target_or_body_keeps_the_domain_opaque() {
        for source in [
            "proc change {ns} {namespace eval $ns {rename set saved_set}}",
            "proc change {body} {namespace eval ::n $body}",
            "proc change {old} {namespace eval ::n [list rename $old saved_set]}",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "dynamic namespace eval input must not be replayed as source: {source}"
            );
        }
    }

    #[test]
    fn frozen_namespace_eval_inputs_retain_the_actual_entered_rename() {
        for source in [
            "set ns ::n\nnamespace eval $ns {rename set saved_set}",
            "set body {rename set saved_set}\nnamespace eval ::n $body",
            "set old set\nnamespace eval ::n [list rename $old saved_set]",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings
                    .targets("::n::saved_set", "::")
                    .iter()
                    .any(|target| { target.command == "::set" }),
                "{source}"
            );
        }
    }

    #[test]
    fn alias_chain_prefix_replays_namespace_eval_in_normalised_namespace() {
        let source = "interp alias {} in_n {} namespace eval n::\n\
                      interp alias {} run_in_n {} in_n\n\
                      run_in_n {rename set saved_set}";
        let (cu, reg) = analyse(source);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(!bindings.has_opaque_domain());
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the alias chain must supply namespace eval's normalised target"
        );
    }

    #[test]
    fn empty_namespace_eval_prefix_enters_the_global_body() {
        // Pinned C 8.4–9.1 accepts the empty namespace selector as global.
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_registry::model::ingress::resolve_environment(profile).unit_profile();
            let reg = CommandRegistry::build_default().project_for_profile(profile);
            for (body, moved) in [("rename set saved_set", true), ("\"unterminated", false)] {
                let source = format!(
                    "interp alias {{}} in_empty {{}} namespace eval {{}}\nin_empty {{{body}}}"
                );
                let cu = CompilationUnit::build_for_profile(&source, &reg, false, profile);
                let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
                assert!(!bindings.has_opaque_domain(), "{}: {body}", profile.name);
                assert_eq!(
                    bindings
                        .targets("::saved_set", "::")
                        .iter()
                        .any(|target| target.command == "::set"),
                    moved,
                    "{}: {body}",
                    profile.name,
                );
            }
        }
    }

    #[test]
    fn static_delete_and_recreate_does_not_widen_unrelated_commands() {
        let (cu, reg) = analyse(
            "proc p {} {}\n\
             rename p {}\n\
             proc p {} { set local 1 }",
        );
        assert!(cu.ir_module.redefined_procedures.contains("::p"));
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "a readable discarded procedure body must not poison the command domain"
        );
        assert!(
            !bindings.target_may_be_unknown("set", "::"),
            "the unrelated set command remains exactly known"
        );
    }

    #[test]
    fn absolute_heads_keep_relative_transition_operands_in_the_current_namespace() {
        let (cu, reg) = analyse("namespace eval ::n {::rename ::set saved_set}");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "absolute head lookup must not replace the command's current namespace"
        );
        assert!(
            !bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the relative destination must not be moved into the global namespace"
        );
    }

    #[test]
    fn absolute_proc_head_keeps_current_namespace_for_discarded_body() {
        let (cu, reg) = analyse(
            "namespace eval ::n {\n\
                 ::proc p {} {::rename ::set saved_set}\n\
                 ::proc p {} {}\n\
             }",
        );
        assert!(cu.ir_module.redefined_procedures.contains("::n::p"));
        let history = discarded_procedure_history(&cu.ir_module, &reg);
        assert!(!history.opaque);
        assert!(
            history
                .modules
                .iter()
                .any(|module| module.top_level_namespace == "::n")
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::n::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set")
        );
        assert!(
            !bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set")
        );
    }

    #[test]
    fn consumed_typed_head_is_revalidated_in_future_roots() {
        let (cu, reg) = analyse(
            "proc ::uses_set {} { set x 1 }\n\
             rename ::set ::saved_set",
        );
        assert!(
            cu.ir_module.procedures["::uses_set"]
                .body
                .command_binding_sites
                .iter()
                .next()
                .is_some(),
            "structured lowering must retain the consumed set dependency"
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "the callable procedure may execute after set no longer denotes the compiled identity"
        );
    }

    #[test]
    fn uplevel_global_proc_body_is_recovered_in_its_runtime_namespace() {
        for wrapper in [
            "uplevel #0 {proc p {} {rename expr saved_expr}}",
            "uplevel #0 {eval {proc p {} {rename expr saved_expr}}}",
        ] {
            let source = format!(
                "proc p {{}} {{}}\n\
                 namespace eval ::n {{\n\
                     proc installer {{}} {{ {wrapper} }}\n\
                 }}"
            );
            let (cu, reg) = analyse(&source);
            assert!(
                cu.ir_module.redefined_procedures.contains("::p"),
                "the #0 body is lowered in the same global namespace Tcl selects"
            );
            let history = discarded_procedure_history(&cu.ir_module, &reg);
            assert!(!history.opaque, "literal runtime body is exact: {wrapper}");
            assert!(
                history
                    .modules
                    .iter()
                    .any(|module| module.top_level_namespace == "::"),
                "misqualified retained bodies must be re-lowered globally: {wrapper}"
            );
            assert!(history.rebound_names.contains("::p"));

            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "literal bodies should stay closed: {bindings:#?}"
            );
            assert!(
                bindings
                    .targets("::saved_expr", "::")
                    .iter()
                    .any(|target| target.command == "::expr"),
                "the recovered body must mutate global expr, not ::n::expr: {wrapper}"
            );
            assert!(
                !scan_module_command_mutations(&cu.ir_module, &reg).trusts_proc_binding("::p"),
                "both runtime definitions of ::p make its identity unstable"
            );
        }
    }

    #[test]
    fn unreadable_uplevel_global_proc_body_is_opaque() {
        let (cu, reg) = analyse(
            "namespace eval ::n {\n\
                 proc installer {body} { uplevel #0 {proc p {} $body} }\n\
             }",
        );
        let history = discarded_procedure_history(&cu.ir_module, &reg);
        assert!(
            history.opaque,
            "a misqualified retained procedure cannot borrow an unreadable lexical body"
        );
    }

    #[test]
    fn unavailable_discarded_redefinition_body_keeps_the_domain_opaque() {
        let (cu, reg) = analyse(
            "proc install {replacement} {\n\
                 set p_name {p}\n\
                 proc p {} {}\n\
                 rename p {}\n\
                 proc $p_name {} $replacement\n\
             }\n\
             install {rename set saved_set}\n\
             p",
        );
        assert!(cu.ir_module.redefined_procedures.contains("::p"));
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a runtime-supplied discarded body may mutate any command binding"
        );
    }

    #[test]
    fn direct_and_alias_prefixed_proc_bodies_are_recovered_before_binding() {
        for source in [
            "proc p {} {rename expr saved_expr}\np",
            "interp alias {} makep {} proc p; makep {} {rename expr saved_expr}; p",
            "interp alias {} makep_target {} proc p\n\
             interp alias {} makep {} makep_target\n\
             makep {} {rename expr saved_expr}\n\
             p",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                !bindings.has_opaque_domain(),
                "a literal typed proc body is exactly recoverable: {source}"
            );
            assert!(
                bindings
                    .targets("::saved_expr", "::")
                    .iter()
                    .any(|target| target.command == "::expr"),
                "the recovered procedure body must contribute its expr rename: {source}"
            );
            let mutations = scan_module_command_mutations(&cu.ir_module, &reg);
            assert!(
                !mutations.trusts("expr"),
                "compiler consumers must not trust expr after the recovered body: {source}"
            );
        }
    }

    #[test]
    fn constructed_namespace_eval_defers_procedure_body_effects_to_the_root_fixpoint() {
        let (cu, reg) =
            analyse("namespace eval ::n [list proc mutate {} {rename ::set ::saved_set}]");
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            !bindings.has_opaque_domain(),
            "the constructed script and procedure body are both exact"
        );
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "the constructed procedure must become a future executable root"
        );
    }

    #[test]
    fn constructed_procedure_roots_discovered_by_a_root_reach_the_next_round() {
        let (cu, reg) = analyse(
            "namespace eval ::n [list proc installer {} {\
                 proc late {} {rename ::set ::saved_set}\
             }]",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "a nested definition discovered while replaying installer must schedule late"
        );
    }

    #[test]
    fn constructed_procedure_redefinitions_retain_every_possible_body() {
        let (cu, reg) = analyse(
            "namespace eval ::n [list proc mutate {} {rename ::set ::saved_set}]\n\
             namespace eval ::n [list proc mutate {} {rename ::expr ::saved_expr}]",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        for (alias, target) in [("::saved_set", "::set"), ("::saved_expr", "::expr")] {
            assert!(
                bindings
                    .targets(alias, "::")
                    .iter()
                    .any(|resolved| resolved.command == target),
                "every exact replacement body remains a possible future root: {alias}"
            );
        }
    }

    #[test]
    fn constructed_namespaced_tcloo_method_uses_runtime_receiver_namespace() {
        let (cu, reg) = analyse(
            "namespace eval ::n { oo::class create C {\
                 method mutate {} {set x 1}\
             } }",
        );
        assert_eq!(
            cu.ir_module.methods["::n::C::mutate"].execution_namespace,
            crate::ir::ExecutionNamespace::RuntimeSelected
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "an object-local set may shadow the typed-lowered global builtin"
        );
    }

    #[test]
    fn unretained_tcloo_body_keeps_the_command_domain_opaque() {
        let (cu, reg) = analyse(
            "set ::body {::rename ::set ::saved_set}\n\
             oo::class create C {method mutate {} $::body}",
        );
        assert!(cu.ir_module.oo_evidence.unretained_executable_roots);
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings.has_opaque_domain(),
            "a runtime-callable body absent from the retained root inventory must fail closed"
        );
        assert!(
            scan_module_command_mutations(&cu.ir_module, &reg).has_dynamic_mutation(),
            "the optimiser projection must retain closed-lattice opacity"
        );
    }

    #[test]
    fn constructed_tcloo_method_keeps_absolute_command_heads_exact() {
        let (cu, reg) = analyse(
            "namespace eval ::n [list oo::class create C {\
                 method mutate {} {::rename ::set ::saved_set}\
             }]",
        );
        let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
        assert!(
            bindings
                .targets("::saved_set", "::")
                .iter()
                .any(|target| target.command == "::set"),
            "an absolute command and target remain namespace-invariant"
        );
    }

    #[test]
    fn unavailable_direct_or_alias_prefixed_proc_body_is_opaque() {
        for source in [
            "set body {rename expr saved_expr}\nproc p {} $body\np",
            "set body {rename expr saved_expr}\n\
             interp alias {} makep {} proc p\n\
             makep {} $body\n\
             p",
        ] {
            let (cu, reg) = analyse(source);
            let bindings = ModuleCommandBindings::analyse(&cu.ir_module, &reg);
            assert!(
                bindings.has_opaque_domain(),
                "a procedure target without a retained or recovered body is opaque: {source}"
            );
            assert!(
                bindings.target_may_be_unknown("expr", "::"),
                "the unavailable procedure body must make expr untrusted: {source}"
            );
            assert!(
                !scan_module_command_mutations(&cu.ir_module, &reg).trusts("expr"),
                "the unavailable body makes the whole command-trust projection dynamic: {source}"
            );
        }
    }

    #[test]
    fn try_handler_joins_command_mutation_from_exception_edge() {
        // A fall-through-capable try body may fail before or after the rename.
        // The analysis-only exception edges therefore contribute both the
        // pre-try Builtin state and the body-exit Opaque state to the handler.
        let (cu, reg) = analyse(
            "proc ::p {} {\n try {\n  rename string {}\n } on error {} {\n  string length abc\n }\n}",
        );
        let fu = cu.function("::p").unwrap();
        let handler = fu
            .cfg
            .blocks
            .iter()
            .find_map(|(&id, block)| block.name.starts_with("try_handler").then_some(id))
            .expect("try handler block");
        assert!(
            fu.cfg
                .exception_edges
                .iter()
                .any(|&(_, target)| target == handler),
            "the test must exercise an analysis-only exception edge"
        );

        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert_eq!(
            cb.binding_at(handler, 0, "string").kind,
            BindingKind::Unknown,
            "the handler must not infer either the original or renamed binding"
        );
        assert!(
            !cb.is_original_builtin_at(handler, 0, "string"),
            "an optimisation must not trust the builtin at handler entry"
        );
    }

    #[test]
    fn seed_marks_module_procs_as_proc() {
        // The W128 seed: a name seeded as PROC resolves to Proc at entry.
        let (cu, reg) = analyse("nonbuiltin a b");
        let fu = cu.function("::top").unwrap();
        let seed = vec![(
            "::myproc".to_owned(),
            Binding {
                kind: BindingKind::Proc,
                target: Some("::myproc".to_owned()),
            },
        )];
        let cb = analyse_command_binding(&fu.cfg, &reg, &seed);
        assert_eq!(
            cb.binding_at(fu.cfg.entry, 0, "myproc").kind,
            BindingKind::Proc
        );
    }

    #[test]
    fn module_mutations_distrust_rebound_builtins_only() {
        let reg = CommandRegistry::build_default();
        // A builtin renamed inside a proc body is distrusted everywhere
        // (over-approximation); a fresh user proc untrusts nothing.
        let cu = CompilationUnit::build_for(
            "proc clobber {} { rename string {} }\nproc myproc {} { return 1 }",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts("string"), "string is rebound in a proc body");
        assert!(m.trusts("lappend"), "an untouched builtin stays trusted");
        assert!(m.trusts("myproc"), "a fresh user proc untrusts nothing");

        // An actual unknown Value formal keeps the mutation subject unknown.
        let cu2 = CompilationUnit::build_for("proc clobber {x} {rename $x bar}", &reg, false);
        let m2 = scan_module_command_mutations(&cu2.ir_module, &reg);
        assert!(!m2.trusts("string") && !m2.trusts("lappend"));

        // A frozen absent subject errors before moving any command. Its
        // written substitution does not turn the subject into a dynamic one.
        let cu3 = CompilationUnit::build_for("set x foo\nrename $x bar", &reg, false);
        let m3 = scan_module_command_mutations(&cu3.ir_module, &reg);
        assert!(m3.trusts("string") && m3.trusts("lappend"));
    }

    /// `walk_body_calls` recurses once
    /// per nested `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`
    /// body, with no depth cap of its own. Transitively
    /// bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass,
    /// so this is defence-in-depth / consistency with every other
    /// full-tree walker in this crate, not a currently-reproducible
    /// crash. 1000 levels of source nesting is comfortably past this
    /// cap; the assertion is that `scan_module_command_mutations` returns
    /// at all, not what it returns. Spawns its own big-stack thread since
    /// the lexer/CST/segmenter stages upstream of the lowering cap still
    /// walk the full un-truncated source nesting before that cap trims
    /// it — same rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_walk_body_calls() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = "proc clobber {} {\n".to_owned();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("rename string {}\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        src.push_str("}\n");
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let reg = CommandRegistry::build_default();
                let cu = CompilationUnit::build_for(&src, &reg, false);
                let _ = scan_module_command_mutations(&cu.ir_module, &reg);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn trusts_proc_binding_true_for_untouched_proc() {
        // TP control: a proc never named by any `rename` / `interp alias`
        // is trusted.
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for("proc myproc {} { return 1 }", &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(m.trusts_proc_binding("myproc"));
        assert!(m.trusts_proc_binding("::myproc"));
    }

    #[test]
    fn trusts_proc_binding_false_for_rename_source_and_target() {
        // FP guard: `rename triple double` perturbs BOTH names — `triple`
        // (vacated, no longer denotes what it did) and `double` (now
        // denotes `triple`'s body, not `double`'s own declaration).
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple double\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts_proc_binding("double"), "rename target");
        assert!(!m.trusts_proc_binding("triple"), "rename source");
    }

    #[test]
    fn trusts_proc_binding_false_for_interp_alias_name() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc answer {} { return 42 }\nproc other {} { return 99 }\ninterp alias {} answer {} other\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts_proc_binding("answer"));
        // The alias *target* itself is untouched — still trusted.
        assert!(m.trusts_proc_binding("other"));
    }

    #[test]
    fn trusts_proc_binding_follows_alias_resolved_mutations() {
        let reg = CommandRegistry::build_default();
        let renamed = CompilationUnit::build_for(
            "proc p {} {return P}\nproc q {} {return Q}\n\
             interp alias {} r {} rename\n\
             r p oldp\nr q p\n",
            &reg,
            false,
        );
        let mutations = scan_module_command_mutations(&renamed.ir_module, &reg);
        for name in ["p", "q", "oldp"] {
            assert!(
                !mutations.trusts_proc_binding(name),
                "alias-resolved rename touched {name}"
            );
        }

        let aliased = CompilationUnit::build_for(
            "proc p {} {return P}\nproc q {} {return Q}\n\
             interp alias {} replace {} interp alias {} p {} q\n\
             replace\n",
            &reg,
            false,
        );
        let mutations = scan_module_command_mutations(&aliased.ir_module, &reg);
        assert!(!mutations.trusts_proc_binding("p"));
        assert!(mutations.trusts_proc_binding("q"));
    }

    #[test]
    fn trusts_proc_binding_unaffected_by_unrelated_rename() {
        // TN control: renaming a DIFFERENT proc must not untrust this one —
        // `trusts_proc_binding` is per-name, unlike the whole-module
        // `dynamic` wildcard.
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple somethingElse\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(m.trusts_proc_binding("double"));
    }

    /// A fully-qualified `proc ::n::expr` shadows the builtin for bodies that
    /// resolve in `::n`, so the builtin is no longer foldable (#2159).
    ///
    /// The two spellings used to disagree: `namespace eval ::n { proc expr … }`
    /// distrusted `expr`, while `proc ::n::expr` did not, because the recorded
    /// name is `::n::expr` and only a name whose *own* default is a builtin was
    /// collected. Measured harm before the fix — `tcl explore --show opt`
    /// rewrote
    ///
    /// ```text
    /// proc ::n::f {r} { return [expr {$r ** 2}] }
    /// ```
    ///
    /// into `[expr {$r * $r}]`, and with `proc ::n::expr {s} {return "SHADOW:$s"}`
    /// that changes the program's output from `SHADOW:$r ** 2` to
    /// `SHADOW:$r * $r` on tclsh 9.0.4.
    #[test]
    fn qualified_proc_shadowing_a_builtin_distrusts_it() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "namespace eval ::n {}\nproc ::n::expr {s} { return $s }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts("expr"),
            "a qualified shadow distrusts the builtin"
        );
    }

    /// A qualified shadow installed through an alias is distrusted too.
    ///
    /// `interp alias {} make {} proc ::n::expr` carries the prepended name,
    /// so the source scan never sees the spelling `proc ::n::expr` — but
    /// `ModuleCommandBindings` recovers the definition, and the same tail
    /// projection applies there. tclsh 9.0.4 prints `SHADOW:$r ** 2` for the
    /// program below; without this, O110 still rewrote the body to
    /// `[expr {$r * $r}]`.
    #[test]
    fn an_alias_installed_qualified_shadow_is_distrusted() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "namespace eval ::n {}\ninterp alias {} make {} proc ::n::expr\nmake {s} { return \"SHADOW:$s\" }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts("expr"),
            "an alias-installed qualified shadow distrusts the builtin",
        );
    }

    /// And an alias installing something whose tail is not a builtin leaves
    /// trust alone.
    #[test]
    fn an_alias_installing_a_non_builtin_tail_keeps_trust() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "namespace eval ::n {}\ninterp alias {} make {} proc ::n::helper\nmake {s} { return $s }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(m.trusts("expr"), "an unrelated alias keeps trust");
    }

    /// The same, for a command other than `expr` — the scan is keyed on the
    /// tail having a builtin default, not on any one name.
    #[test]
    fn qualified_shadow_distrust_is_not_specific_to_expr() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "namespace eval ::n {}\nproc ::n::llength {s} { return 99 }\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts("llength"), "any shadowed builtin tail distrusts");
        assert!(m.trusts("expr"), "and only the one that was shadowed");
    }

    /// A qualified `proc` whose tail is *not* a builtin changes nothing —
    /// the guard must not withdraw folding from every namespaced file.
    #[test]
    fn qualified_proc_with_a_non_builtin_tail_keeps_trust() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "namespace eval ::n {}\nproc ::n::helper {s} { return $s }\nproc ::n::f {r} { return [expr {$r ** 2}] }\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(m.trusts("expr"), "an unrelated qualified proc keeps trust");
    }

    #[test]
    fn trusts_proc_binding_false_for_namespace_relative_rename() {
        // FP guard (reported in code review): a bare `rename` argument
        // inside a namespaced proc resolves relative to THAT proc's own
        // namespace, not the global namespace — `rename triple double`
        // inside `proc ::ns::doit` renames `::ns::triple` onto
        // `::ns::double`. An earlier version always rooted the bare names
        // globally (`::triple`/`::double`), so it never distrusted the
        // actually-affected namespaced names.
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "namespace eval ::ns {\n    proc double {n} { expr {$n * 2} }\n    proc triple {n} { expr {$n * 3} }\n}\nproc ::ns::doit {} { rename triple double }\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts_proc_binding("::ns::double"),
            "namespace-relative rename target"
        );
        assert!(
            !m.trusts_proc_binding("::ns::triple"),
            "namespace-relative rename source"
        );
        // The scan is flow-insensitive (it can't know whether `double`/
        // `triple` already existed as GLOBAL commands at the point
        // `::ns::doit` runs), so it conservatively distrusts the
        // global-rooted candidate too — a deliberate, sound
        // over-approximation (a missed fold, never a wrong one),
        // mirroring `collect_tampered_builtins`'s existing philosophy.
        assert!(
            !m.trusts_proc_binding("::double"),
            "global-rooted candidate"
        );
        assert!(
            !m.trusts_proc_binding("::triple"),
            "global-rooted candidate"
        );
    }

    // Regression: `proc max {...}` must not be distrusted. `max`/`min` read
    // like `tcl::mathop` operator words, but real Tcl never registered them
    // there (verified against tclsh 8.6/9.0 — `info commands
    // ::tcl::mathop::*` never lists them); they exist only as unrelated
    // `expr` math functions. A now-fixed registry bug once carried bare
    // `max`/`min` `CommandSpec` entries as if they were `tcl::mathop`
    // members, which made `default_binding` treat them as pre-existing
    // builtins — so a completely ordinary `proc max {...}` looked like it
    // was "renaming a builtin", silently blocking O103 from folding calls to
    // it (caught by `tests/optimiser.rs::interprocedural_constant_folding`).
    #[test]
    fn module_mutations_do_not_distrust_proc_named_like_mathop_word() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc max {a b} {\n    if {$a > $b} { return $a } else { return $b }\n}\nset v [max 3 7]\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            m.trusts("max"),
            "a plain proc sharing a name with an (incorrectly bare-registered) \
             tcl::mathop-lookalike must not be distrusted"
        );
    }
}

#[cfg(test)]
mod source_variable_access_tests {
    use super::*;

    fn analyse(source: &str) -> (SourceCommandBindings, Arc<CommandRegistry>) {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = Arc::clone(context.commands());
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            &registry,
            SourceAnalysisOptions {
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                )),
                ..SourceAnalysisOptions::default()
            },
        );
        (bindings, registry)
    }

    fn reads(bindings: &SourceCommandBindings, source: &str) -> Vec<SourceVariableAccess> {
        bindings.variable_accesses_in_span(tcl_lexer::Span::new(
            0,
            u32::try_from(source.len().saturating_sub(1)).unwrap(),
        ))
    }

    #[test]
    fn full_depth_array_indices_resolve_every_source_read_after_its_index() {
        let source = include_str!("../tests/data/native_deep_array_source.tcl");
        let (bindings, registry) = analyse(source);
        let accesses = reads(&bindings, source);
        let elements = accesses
            .iter()
            .filter(|access| access.original_spelling.starts_with("$a("))
            .collect::<Vec<_>>();
        assert_eq!(elements.len(), 2000);
        for access in elements {
            for context in access.context_alternatives() {
                let place = access.place_in_context(context, &registry);
                let index = place.index.as_ref().expect("resolved native element");
                assert_eq!(index.kind, crate::place::IndexKind::Literal);
                assert_eq!(index.value, "x");
            }
        }
    }

    #[test]
    fn evaluated_indices_freeze_values_before_later_parts_and_read_observers() {
        for (source, spelling, key, observed) in [
            (
                "set k old; set a(oldnew) VALUE; set view $a($k[set k new])",
                "$a($k[set k new])",
                "oldnew",
                false,
            ),
            (
                "proc observe {name key op} {upvar 1 $name value; set value old}; set k new; set a(old) VALUE; trace add variable k read observe; set view $a($k)",
                "$a($k)",
                "old",
                false,
            ),
            (
                "proc observe args {}; set k k; set a(k) VALUE; trace add variable a(k) read observe; set view $a($k)",
                "$a($k)",
                "k",
                true,
            ),
            (
                "set k k; set {a($k)} VALUE; set view ${a($k)}",
                "${a($k)}",
                "$k",
                false,
            ),
        ] {
            let (bindings, registry) = analyse(source);
            let all = reads(&bindings, source);
            let access = all
                .iter()
                .find(|access| access.original_spelling == spelling)
                .unwrap();
            for context in access.context_alternatives() {
                let place = access.place_in_context(context, &registry);
                let index = place.index.as_ref().expect("actual element address");
                assert_eq!(index.kind, crate::place::IndexKind::Literal, "{source}");
                assert_eq!(index.value, key, "{source}");
                assert_eq!(place.observed, observed, "{source}");
                if observed {
                    assert!(!context.read_produces_value(&place, &registry));
                }
            }
        }
    }

    #[test]
    fn evaluated_element_addresses_relocate_and_disagreements_remain_unknown() {
        let source = "proc f {} {set k k; set a(k) VALUE; set view $a($k)}";
        let (bindings, registry) = analyse(source);
        let all = reads(&bindings, source);
        let access = all
            .iter()
            .find(|access| access.original_spelling == "$a($k)")
            .unwrap();
        let context = &access.context_alternatives()[0];
        let place = access.place_in_context(context, &registry);
        let crate::place::CellOwner::Activation(owner) = &place.cell.as_ref().unwrap().owner else {
            panic!("actual local element owner");
        };
        let mut relocation = crate::var_resolve::VariableProofRelocation::default();
        relocation
            .activations
            .insert(owner.clone(), "relocated-read".to_owned());
        let relocated = access.relocated_variables(&relocation);
        let target = relocated.place_in_context(&relocated.context_alternatives()[0], &registry);
        assert_eq!(
            target.cell.as_ref().unwrap().owner,
            crate::place::CellOwner::Activation("relocated-read".to_owned())
        );
        assert_eq!(target.index.as_ref().unwrap().value, "k");
        assert_eq!(relocated.source, access.source);
        assert_eq!(relocated.original_spelling, access.original_spelling);

        let mut conflicting = access.clone();
        let mut other = place;
        other.index = Some(crate::place::Index::literal("different"));
        conflicting.retain_selected_place(context, &other);
        let unresolved = conflicting.place_in_context(context, &registry);
        assert_eq!(
            unresolved.index.as_ref().unwrap().kind,
            crate::place::IndexKind::Dynamic
        );
        assert!(!context.read_produces_value(&unresolved, &registry));
    }

    #[test]
    fn word_variable_sites_equal_the_native_read_inventory_on_every_c_release() {
        let source = r#"set a k; set arr(k) VALUE; set {} EMPTY; puts $a; puts ${a}; puts "$a"; puts "${a}"; puts $arr(k); puts "$arr($a)"; puts ${}"#;
        for release in tcl_dialect::TclVersion::ALL {
            let registry =
                tcl_registry::model::ingress::static_context_for(release.dialect_profile_name())
                    .commands();
            let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(release)),
                    ..SourceAnalysisOptions::default()
                },
            );
            let accesses = reads(&bindings, source);
            let map = tcl_lexer::SourceMap::new(source);
            let mut represented = 0;
            for segment in
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            {
                let tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
                for word in tokens.words() {
                    let Some((_, site)) = word.sole_variable_substitution() else {
                        continue;
                    };
                    let access = SourceVariableAccess::find_at_site(&accesses, site)
                        .unwrap_or_else(|| {
                            panic!("missing exact site on {release:?}: {site:?}; {accesses:#?}")
                        });
                    let at = usize::try_from(site.span.start()).unwrap();
                    let native = tcl_lexer::word_parts::scan_var_ref(source.as_bytes(), at, config)
                        .unwrap()
                        .unwrap();
                    assert_eq!(
                        site.span,
                        native.source_span(source.as_bytes(), at, 0).unwrap()
                    );
                    assert_eq!(access.original_spelling, &source[at..native.next]);
                    represented += 1;
                }
            }
            assert_eq!(represented, 7);
            assert_eq!(
                accesses.len(),
                8,
                "nested index read is independently represented"
            );
        }
    }

    #[test]
    fn argument_read_ownership_follows_evaluation_instead_of_lexical_nesting() {
        let source = "set x 1; proc p {} {puts $x}; puts [expr {$x + 1}]";
        let (bindings, _) = analyse(source);
        let definition = u32::try_from(source.find("proc p").unwrap()).unwrap();
        assert_eq!(
            bindings.variable_accesses_during_argument_evaluation(definition),
            [] as [SourceVariableAccess; 0]
        );
        let outer = u32::try_from(source.rfind("puts [").unwrap()).unwrap();
        let nested = u32::try_from(source.find("expr {").unwrap()).unwrap();
        assert_eq!(
            bindings.variable_accesses_for_invocation_args(outer),
            [] as [SourceVariableAccess; 0]
        );
        assert_eq!(
            bindings.variable_accesses_for_invocation_args(nested),
            [] as [SourceVariableAccess; 0]
        );
        let reached = bindings.variable_accesses_during_argument_evaluation(outer);
        assert_eq!(reached.len(), 1);
        assert!(matches!(
            reached[0].owner,
            SourceVariableEvaluationOwner::NativeExpression { .. }
        ));
        assert_eq!(
            bindings
                .variable_accesses_during_expression_invocation(nested)
                .len(),
            1
        );
        assert_eq!(
            bindings.variable_accesses_during_argument_evaluation(nested),
            [] as [SourceVariableAccess; 0]
        );
    }

    #[test]
    fn script_body_reads_belong_to_enclosing_arguments_but_not_the_script_invocation() {
        let source = "set x 1; puts [catch {puts $x}]";
        let (bindings, _) = analyse(source);
        let outer = u32::try_from(source.find("puts [").unwrap()).unwrap();
        let captured = u32::try_from(source.find("catch {").unwrap()).unwrap();
        assert_eq!(
            bindings
                .variable_accesses_during_argument_evaluation(outer)
                .len(),
            1
        );
        assert_eq!(
            bindings.variable_accesses_during_argument_evaluation(captured),
            [] as [SourceVariableAccess; 0]
        );
        assert_eq!(
            bindings.variable_accesses_during_expression_invocation(captured),
            [] as [SourceVariableAccess; 0]
        );
    }

    #[test]
    fn reads_in_one_word_keep_their_actual_alias_selection() {
        let source = r#"set x X; set y Y; upvar 0 x a; puts "$a[upvar 0 y a]$a""#;
        let (bindings, registry) = analyse(source);
        let accesses = reads(&bindings, source);
        assert_eq!(accesses.len(), 2);
        let places: Vec<_> = accesses
            .iter()
            .map(|access| {
                crate::var_resolve::resolve_substitution_access(
                    &access.original_spelling,
                    &access.variable_context,
                    &registry,
                    tcl_registry::TraceOperation::Read,
                )
            })
            .collect();
        assert_eq!(places[0].name, "x");
        assert_eq!(places[1].name, "y");
        assert_ne!(accesses[0].source, accesses[1].source);
        assert!(
            SourceVariableAccess::find_at_source(&accesses, &accesses[0].source, "$unrecorded",)
                .is_none()
        );
    }

    #[test]
    fn exact_read_query_requires_one_retained_record_at_the_site() {
        let source = "set x X; puts $x";
        let (bindings, _) = analyse(source);
        let accesses = reads(&bindings, source);
        let original = &accesses[0];
        assert!(SourceVariableAccess::find_at_source(&accesses, &original.source, "$x").is_some());
        for spelling in ["$x", "$z"] {
            let mut conflicting = original.clone();
            conflicting.original_spelling = spelling.to_owned();
            let duplicated = [original.clone(), conflicting];
            assert!(
                SourceVariableAccess::find_at_source(&duplicated, &original.source, "$x").is_none()
            );
            assert!(SourceVariableAccess::find_at_site(&duplicated, &original.source).is_none());
        }
    }

    #[test]
    fn array_read_context_follows_index_effects_and_records_nested_reads() {
        let source =
            r#"set k 0; set a(0) FIRST; set a(1) SECOND; puts "$a([set k 1])[set k 0]$a($k)""#;
        let (bindings, registry) = analyse(source);
        let accesses = reads(&bindings, source);
        assert_eq!(accesses.len(), 3, "{accesses:#?}");
        for (spelling, wanted) in [("$a([set k 1])", "1"), ("$a($k)", "0"), ("$k", "0")] {
            let access = accesses
                .iter()
                .find(|access| access.original_spelling == spelling)
                .expect("reached original reference");
            assert_eq!(
                access
                    .variable_context
                    .substitution_literal_value("$k", &registry),
                Some(wanted)
            );
            let start = usize::try_from(access.source.span.start()).unwrap();
            assert_eq!(&source[start..start + spelling.len()], spelling);
        }
    }

    #[test]
    fn skipped_lazy_and_abrupt_reads_have_no_dispatch_fallback() {
        let source =
            "set x 1; expr {0 && $lazy}; catch {return; puts $abrupt}; expr {1 ? $x : $other}";
        let (bindings, _) = analyse(source);
        let accesses = reads(&bindings, source);
        assert_eq!(accesses.len(), 1, "{accesses:#?}");
        assert_eq!(accesses[0].original_spelling, "$x");
    }

    #[test]
    fn read_snapshot_precedes_its_own_observer_effect() {
        let source = "set x 1; proc observer args {}; trace add variable x read observer; puts $x";
        let (bindings, registry) = analyse(source);
        let accesses = reads(&bindings, source);
        assert_eq!(accesses.len(), 1);
        let access = &accesses[0];
        assert!(!access.variable_context.dynamic_bindings);
        let place = crate::var_resolve::resolve_substitution_access(
            &access.original_spelling,
            &access.variable_context,
            &registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(place.observed);
    }
}

#[cfg(test)]
mod source_callable_static_tests {
    use super::*;

    fn analyse(source: &str) -> SourceCommandBindings {
        let context = tcl_registry::model::ingress::static_context_for("jim");
        let registry = context.commands();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            SourceAnalysisOptions {
                invocation_dialect: registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile),
                ..SourceAnalysisOptions::default()
            },
        )
    }

    fn assert_saved_native_set(source: &str) {
        let bindings = analyse(source);
        let site = u32::try_from(source.rfind("saved x").unwrap()).unwrap();
        let target = bindings.invocation_at_source("saved", site);
        assert!(
            target
                .proved_execution_target()
                .is_some_and(|target| target.registry_backed && target.command == "::set"),
            "{target:#?}"
        );
    }

    #[test]
    fn static_formals_update_one_retained_slot_across_actual_calls() {
        assert_saved_native_set(
            "proc p {x} {{x OLD}} {return $x}; p OTHER; eval [p {rename set saved}]; saved x 1",
        );
    }

    #[test]
    fn captured_raw_wrapper_observes_later_alias_retargeting() {
        assert_saved_native_set(
            "set a OLD; set b {rename set saved}; upvar 0 a x; proc p {} {&x} {return $x}; upvar 0 b x; eval [p]; saved x 1",
        );
    }

    #[test]
    fn raw_capture_retains_old_cell_after_global_name_recreation() {
        assert_saved_native_set(
            "set x {rename set saved}; proc p {} {&x} {return $x}; unset x; set x OTHER; eval [p]; saved x 1",
        );
    }
}

#[cfg(test)]
mod source_alias_lookup_tests {
    use super::*;

    #[test]
    fn jim_alias_targets_use_each_actual_callers_namespace() {
        let source = "proc target {} {return ROOT}; namespace eval A {proc target {} {return A}}; alias a target; a; namespace eval A {a}";
        let context = tcl_registry::model::ingress::static_context_for("jim");
        let registry = context.commands();
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            SourceAnalysisOptions {
                invocation_dialect: registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile),
                ..SourceAnalysisOptions::default()
            },
        );
        for (offset, expected) in [
            (source.find("; a;").unwrap() + 2, "::target"),
            (source.rfind("{a}").unwrap() + 1, "::A::target"),
        ] {
            let binding = bindings.invocation_at_source("a", u32::try_from(offset).unwrap());
            assert_eq!(
                binding
                    .proved_target()
                    .map(|target| target.command.as_str()),
                Some(expected)
            );
        }
    }
}

/// Build called-frame operands before descending into the original body.
#[inline(never)]
fn source_call_arguments<'a>(
    target: &'a SourceCommandTarget,
    effective: &'a [crate::registry_invocation::EffectiveInvocationWord],
) -> Vec<Option<&'a str>> {
    let mut actual = target
        .prepended
        .iter()
        .map(|value| value.as_registry_word().literal())
        .collect::<Vec<_>>();
    actual.extend(
        effective
            .iter()
            .skip(1)
            .map(|word| word.as_registry_word().literal()),
    );
    actual
}

#[cfg(test)]
mod compiled_variable_policy_tests {
    use super::SourceAnalysisOptions;
    use tcl_registry::{
        InvocationDialect, native_compiled_variables::LogicalCompiledVariableProvider,
    };
    use tcl_syntax::naming::{NativeCompiledVariableAuthority, NativeCompiledVariableRecipe};

    #[test]
    fn supplied_live_entry_withdraws_missing_compiler_local_policy() {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut entry = crate::environment_ingress::captured_native_entry(profile);
        assert!(entry.compiled_variable_protocol.is_some());
        entry.compiled_variable_protocol = None;
        let options = SourceAnalysisOptions {
            native_entry: Some(&entry),
            invocation_dialect: Some(InvocationDialect::for_version(
                tcl_dialect::TclVersion::V9_0,
            )),
            compiled_variable_provider: Some(LogicalCompiledVariableProvider::Tcl84CoreSimulation),
            ..Default::default()
        };
        assert!(options.compiled_variable_protocol().is_none());
    }

    #[test]
    fn explicit_local_compiler_provider_participates_in_owned_baseline_identity() {
        let mut ordinary = super::BindingBaseline::default();
        ordinary.refresh_fingerprint();
        let mut authored = ordinary.clone();
        authored.compiled_variable_provider =
            Some(LogicalCompiledVariableProvider::Tcl84CoreSimulation);
        authored.refresh_fingerprint();
        assert_ne!(ordinary, authored);
        assert_ne!(ordinary.fingerprint, authored.fingerprint);
        let retained = authored.clone();
        assert_eq!(retained, authored);
        assert_eq!(
            retained.compiled_variable_provider,
            authored.compiled_variable_provider
        );
    }

    #[test]
    fn authored_f5_local_compiler_requires_its_explicit_provider() {
        let f5 = InvocationDialect::of_profile(tcl_dialect::DialectProfile::irules());
        let mut options = SourceAnalysisOptions {
            invocation_dialect: Some(f5),
            ..Default::default()
        };
        assert!(options.compiled_variable_protocol().is_none());
        options.compiled_variable_provider =
            Some(LogicalCompiledVariableProvider::Tcl84CoreSimulation);
        let policy = options.compiled_variable_protocol().unwrap();
        assert_eq!(
            policy.authority(),
            NativeCompiledVariableAuthority::AuthoredSimulation
        );
        assert_eq!(
            policy.recipe(),
            NativeCompiledVariableRecipe::C(tcl_dialect::TclVersion::V8_4)
        );
        options.invocation_dialect = Some(InvocationDialect::for_version(
            tcl_dialect::TclVersion::V9_0,
        ));
        assert!(options.compiled_variable_protocol().is_none());
    }
}

#[cfg(test)]
mod hosted_storage_entry_tests {
    use super::*;

    #[test]
    // Implementation contract: naming.variable.hosted-storage-context
    // docs/design/analysis/name-resolution-proofs/variable-hosted-storage-context.md
    fn hosted_storage_entry_is_independent_of_registry_and_source_name_policy() {
        use tcl_registry::f5::BigIpExecutionContext;
        let stock = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let irules = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let entry = SourceAnalysisEntry {
            hosted_execution_context: Some(BigIpExecutionContext::TmmIRule),
            ..Default::default()
        };
        assert_eq!(
            entry.options().hosted_execution_context,
            entry.hosted_execution_context
        );
        let selected = ModuleCommandBindings::initial_with_options(stock, entry.options(), None);
        assert_eq!(
            selected.source_variables.hosted_execution_context,
            entry.hosted_execution_context
        );
        assert!(
            selected
                .namespaces
                .contains(&SourceNamespaceKey::authored("::static"))
        );
        let catalogue = ModuleCommandBindings::initial_with_options(
            irules,
            SourceAnalysisOptions::default(),
            None,
        );
        assert_eq!(catalogue.source_variables.hosted_execution_context, None);
        let different = SourceAnalysisEntry {
            hosted_execution_context: Some(BigIpExecutionContext::IAppImplementation),
            ..entry.clone()
        };
        let selected =
            ModuleCommandBindings::initial_with_options(irules, different.options(), None);
        assert_eq!(
            selected.source_variables.hosted_execution_context,
            different.hosted_execution_context
        );
        assert_ne!(entry, different);
        assert_ne!(
            entry.options().hosted_execution_context,
            different.options().hosted_execution_context
        );
        assert!(selected.source_variables.authored_tmm_static.is_none());
        assert!(selected.source_variables.execution.is_none());
    }
}

#[cfg(test)]
mod selected_replay_formal_tests {
    use super::*;

    fn invocation(parameters: &str) -> ResolvedBindingInvocation {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0");
        let arguments = vec!["target".to_owned(), parameters.to_owned(), String::new()];
        let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        let facts = registry
            .commands()
            .resolve_invocation("proc", &refs, registry.commands().own_surface_query())
            .unwrap()
            .facts();
        ResolvedBindingInvocation {
            command: "proc".into(),
            source_span: tcl_lexer::Span::new(0, 1),
            facts: Box::new(facts),
            literal_arguments: arguments.iter().cloned().map(Some).collect(),
            exact_argument_count: Some(arguments.len()),
            arguments,
        }
    }

    #[test]
    fn original_procedure_replay_validity_requires_the_selected_formal_grammar() {
        // naming.variable.original-readonly-formal-topology
        // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
        // Registry replay syntax only, without installed procedure or body entry.
        let namespace = crate::ir::ExecutionNamespace::exact("::");
        let c = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0);
        let jim = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        );
        for parameters in ["a::b", "a(1)"] {
            let invocation = invocation(parameters);
            assert!(matches!(
                recover_procedure_definition(&invocation, &namespace, Some(c)),
                ProcedureDefinitionReplay::KnownError
            ));
            assert!(matches!(
                recover_procedure_definition(&invocation, &namespace, Some(jim)),
                ProcedureDefinitionReplay::Recovered { .. }
            ));
            assert!(matches!(
                recover_procedure_definition(&invocation, &namespace, None),
                ProcedureDefinitionReplay::Unavailable
            ));
        }
        let duplicate = invocation("args args");
        assert!(matches!(
            recover_procedure_definition(&duplicate, &namespace, Some(jim)),
            ProcedureDefinitionReplay::KnownError
        ));
        assert!(matches!(
            recover_procedure_definition(&duplicate, &namespace, Some(c)),
            ProcedureDefinitionReplay::Recovered { .. }
        ));
    }
}
