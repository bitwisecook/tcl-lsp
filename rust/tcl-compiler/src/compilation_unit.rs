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

//! Shared compilation artefacts for a single source document.
//!
//! Built once per diagnostics cycle, consumed by the analyser,
//! optimiser, shimmer analysis, taint engine, and compiler checks.
//!
//! Hosts the [`CompilationUnit`] / [`FunctionUnit`] facade types and
//! the `build_for` entry point that drives the pipeline (lower → CFG →
//! SSA → def-use → SCCP). Heavier analyses (interprocedural, memory-SSA,
//! rendered-properties) plug in through
//! accessor methods that return `Option<&T>` — `None` when the analysis
//! hasn't been run on this unit yet.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use tcl_registry::CommandRegistry;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::value_transfer::AnalysisTier;
#[cfg(test)]
use tcl_registry::value_transfer::{DeclineReason, DomainFact, FactView};

use crate::cfg::{CfgModule, Function as CfgFunction};
#[cfg(test)]
use crate::cfg_builder::PreparedCfgContext;
use crate::def_use::DefUseFacts;
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Module as IrModule;
use crate::memory_ssa::MemorySsaFunction;
use crate::rendered_properties::RenderedValueProps;
use crate::sccp::SccpResult;
use crate::semantic_analysis::SemanticAnalysisBundle;
use crate::ssa::{SsaFunction, ValueKey};
use crate::taint::TaintLattice;
use crate::types::TypeLattice;

/// Module-wide CFG-determining context (upvar summaries + proc params +
/// global-write summaries) shared by every procedure/method build so each
/// rebuilt CFG matches the whole-module build.  Produced by
/// [`crate::cfg_builder::prepare_cfg_context`]; re-exported here under the
/// same name for this file's existing call sites (the canonical definition
/// lives in [`crate::cfg_builder::CfgContext`]).
type CfgContext = crate::cfg_builder::CfgContext;

/// Memoised per-procedure **body-lowering** callback:
/// `(qualified name, body source) -> lowered body`.  Named so the build entry
/// points that thread it stay readable (and clippy's `type_complexity` has
/// nothing to complain about).
pub type BodyLoweringCache<'a> = dyn Fn(&str, &str) -> crate::ir::Script + 'a;

/// The non-callback inputs to a [`CompilationUnit`] build.
///
/// Grouped into one struct rather than threaded as five positional
/// parameters: `build_with` already takes two callbacks, and every entry
/// point below funnels into it.  `Copy`, so passing it on costs nothing.
#[derive(Clone, Copy)]
pub struct UnitBuildOptions<'a> {
    /// Command registry the whole build resolves against.
    pub registry: &'a CommandRegistry,
    /// `false` gives analyses the fully-inlined top-level CFG; `true`
    /// matches codegen, where top-level `foreach` / `catch` / `try` compile
    /// as opaque calls.
    pub defer_top_level: bool,
    /// Lexer configuration (dialect `{*}` / `}{` tokenisation).
    pub config: tcl_lexer::LexerConfig,
    /// Analysis dialect; `None` for plain Tcl.
    pub dialect: Option<&'static tcl_dialect::DialectProfile>,
    /// Call sites in **other** files that reach this unit's procedures, from
    /// a host with a workspace view (see
    /// [`crate::unit_scope::scan_source_call_sites`]).
    ///
    /// `None` means "no cross-file view available", which is not the same as
    /// "no cross-file callers": the unit is then on its own, and any
    /// registry-declared boundary
    /// ([`crate::unit_scope::scan_unit_linkage`]) disables the
    /// interprocedural seed rather than trusting an unprovable "every caller
    /// is in this file".  `Some` — even `Some(&empty)` — is the host
    /// asserting it enumerated the project, so the merged evidence is the
    /// whole picture.
    pub external_call_sites: Option<&'a crate::unit_scope::CallSiteEvidence>,
    /// The document's own command declarations — its inline `# tcl-lsp:
    /// stub` block and the nearest `<dialect>.tcl.stubs` sidecar, ingested
    /// by
    /// [`analyser::utils::document_declared_surface`](crate::analyser::utils::document_declared_surface).
    ///
    /// A stub is a per-document declaration, so it never enters the shared
    /// `registry`; it rides here instead and is unioned with the catalogue's
    /// answer through [`tcl_registry::model::DocumentCommandSurface`], the
    /// one door onto a document's command surface. `None` is a document that
    /// declares nothing.
    pub declared_commands: Option<&'a tcl_registry::model::DeclaredSurface>,
}

/// Where a consumer reads a place's existence fact
/// ([`FunctionUnit::existence`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExistencePoint {
    /// Just before the statement at this index of the block runs.
    Before(crate::cfg::BlockId, usize),
    /// At the block's exit, where its terminator reads.
    Exit(crate::cfg::BlockId),
    /// Where this version of the place is established.
    Version(crate::ssa::Version),
}

/// Callback type for [`CompilationUnit::with_interprocedural_memoized`].
///
/// Given the exact body ID, its display name and the whole-module
/// [`InterproceduralAnalysis`], returns its (memoised) interprocedural taints,
/// or `None` to fall back to a fresh [`FunctionUnit::interproc_taints`] re-run.
pub type TaintCascadeCallback<'a> = dyn FnMut(
        crate::ir::ProcedureId,
        &str,
        &InterproceduralAnalysis,
    ) -> Option<Arc<HashMap<ValueKey, TaintLattice>>>
    + 'a;

/// Callback for genuine original body taint memo results. The request owns
/// the actual internally computed summary and current source body roster.
pub type OriginalTaintCascadeCallback<'a> = dyn FnMut(
        &OriginalInterproceduralTaintRequest,
        crate::ir::ProcedureId,
        &str,
    ) -> Option<crate::body_analysis_capsule::OriginalInterproceduralTaintReport>
    + 'a;

enum OriginalTaintCallback<'borrow, 'data> {
    Legacy(&'borrow mut TaintCascadeCallback<'data>),
    Original(&'borrow mut OriginalTaintCascadeCallback<'data>),
}

// Per-function analysis bundle

/// The single method-body view of `TclOO` instance state, built **once** per
/// method unit from its typed [`crate::ir::MethodDef`] and read by every
/// consumer that needs "which names are auto-bound in this method's frame"
/// bound in this method's frame".
///
/// Three analyses need that fact — [`FunctionUnit::build_for_method`]'s
/// `object_state` (the `[info exists]` fold, I230 / O100 / O101), the
/// analyser's `emit_method_body_diagnostics` `known_bound` set for
/// W210/W211/W220, and the optimiser's `oo_method_constants` escaping set —
/// and parallel channels for it diverge (two copies of the existence fold
/// sourcing parameters from different maps).  All three read this struct off
/// [`FunctionUnit::method_facts`], so they cannot diverge by construction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MethodBodyFacts {
    /// The method's own formal parameter names, in declaration order.
    pub params: Vec<String>,
    /// Compatibility names potentially escaping to object storage. Original
    /// method units retain their actual selected entry context independently;
    /// these labels cannot supply an original cell or binding.
    pub instance_vars: HashSet<String>,
    /// Original source identity keeps String compatibility facts advisory.
    original_source: bool,
    /// Exact original formal declaration, without an execution grant.
    pub(crate) original_parameters:
        Option<Arc<crate::command_binding::formal_topology::OriginalFormalTopology>>,
    /// Genuine entered receiver/provider state, retained before body effects.
    pub(crate) original_receiver_context: Option<
        Arc<crate::command_binding::original_receiver_body_context::OriginalReceiverBodyContext>,
    >,
}

impl MethodBodyFacts {
    /// Build the facts from the method's typed IR — the one construction
    /// site (called by [`FunctionUnit::build_for_method`] and the guarded
    /// path in `build_method_units`).
    #[must_use]
    pub fn from_method(method: &crate::ir::MethodDef) -> Self {
        Self {
            params: method.params.clone(),
            instance_vars: method.instance_vars.clone(),
            original_source: method.body.executed_source.is_some(),
            original_parameters: method.original_parameters.clone(),
            original_receiver_context: method.original_receiver_context.clone(),
        }
    }

    /// Names bound at entry to the method's frame: its own parameters plus
    /// the auto-linked instance variables — the W210-family `known_bound`
    /// set (and, cloned, the seed of its `cross_event_vars` suppression
    /// set: a "setter" method writing an instance var with no local read is
    /// not a dead store, another method reads it later).
    #[must_use]
    pub fn known_bound_at_entry(&self) -> HashSet<String> {
        if self.original_source {
            return self
                .original_parameters
                .as_ref()
                .map_or_else(HashSet::new, |topology| {
                    topology
                        .original_name_fields()
                        .into_iter()
                        .flatten()
                        .filter_map(|field| {
                            std::str::from_utf8(field.name.as_bytes())
                                .ok()
                                .map(str::to_owned)
                        })
                        .collect()
                });
        }
        self.instance_vars
            .iter()
            .chain(self.params.iter())
            .cloned()
            .collect()
    }
}

/// Analysis artefacts for one function (top-level or procedure).
///
/// `PartialEq` enables salsa early-cutoff: when [`crate`]'s salsa-native
/// [`function_lattice`](../../tcl_lsp_db/fn.function_lattice.html) query rebuilds
/// a procedure whose interned body changed but whose lattice came out identical,
/// the equal comparison lets dependents skip re-execution.
#[derive(Debug, Clone)]
pub struct FunctionUnit {
    original_lattice_worker: Option<Arc<original_lattice_worker::OriginalFunctionLatticeResult>>,
    original_lattice_callback_admission: Option<bool>,
    /// Complete immutable metadata generation retained by the actual build.
    #[doc(hidden)]
    pub source_metadata_input: Option<crate::analyser::ResolvedAnalysisInput>,
    /// Exact source lexer configuration, including body-coordinate policy.
    #[doc(hidden)]
    pub source_config: tcl_lexer::LexerConfig,
    /// Qualified function name (e.g. `::top`, `::foo::bar`).
    pub name: String,
    /// Retained module body identity; the top-level script leaves this absent.
    pub body_id: Option<crate::ir::ProcedureId>,
    /// Control-flow graph.
    pub cfg: CfgFunction,
    /// SSA form.
    pub ssa: SsaFunction,
    /// Shared symbol/version graph with its current exact storage-key interpretation.
    /// Source relocation retains the graph; storage relocation rebinds its interpretation.
    pub def_use: DefUseFacts,
    /// SCCP result: lattice values, executable blocks, constant
    /// branches.
    pub sccp: SccpResult,
    /// Lazy immutable analysis-purpose values; caches never affect structural Eq.
    pub semantic_value_projection: Arc<crate::sccp::SemanticValueProjection>,
    /// Exact projection issued by this function's construction or central
    /// graph invalidation. A substituted public cache is not that producer.
    retained_semantic_value_owner: Option<Arc<crate::sccp::SemanticValueProjection>>,
    /// Type lattice values per SSA definition.
    ///
    /// Computed by the type-propagation pass. Absent entries are
    /// implicitly `TypeLattice::unknown()`.
    ///
    /// Shared behind an `Arc` — see [`Self::def_use`].
    pub types: Arc<HashMap<ValueKey, TypeLattice>>,
    /// Inferred return type — the join of the types produced at every
    /// executable `Return` terminator.  `Unknown` when the function
    /// has no executable return value.  Computed by
    /// [`crate::type_infer::infer_function_return_type`].
    pub return_type: TypeLattice,
    /// Taint lattice values per SSA definition.
    ///
    /// Computed by the intra-procedural taint-propagation pass, then replaced
    /// by the interprocedural re-run ([`Self::interproc_taints`], memoised by
    /// `tcl-lsp-db`'s `taint_cascade`).
    ///
    /// Shared behind an `Arc` — see [`Self::def_use`]. That also makes the
    /// `taint_cascade` memo hit a refcount bump rather than a deep copy of the
    /// whole per-procedure taint map.
    pub taints: Arc<HashMap<ValueKey, TaintLattice>>,
    /// Rendered-string-property lattice values per SSA definition.
    ///
    /// Computed by `propagate_rendered_props`. Absent entries are
    /// implicitly `RenderedValueProps::bottom()`.
    ///
    /// Shared behind an `Arc` — see [`Self::def_use`].
    pub rendered_props: Arc<HashMap<ValueKey, RenderedValueProps>>,
    /// Optional memory-SSA annotations (populated on demand).
    pub memory_ssa: Option<MemorySsaFunction>,
    /// Whether this function accesses variables whose *name* is computed at
    /// run time (`set $var v` / `[set $name]` / `unset $n`).
    ///
    /// Three flags, no name set — a dynamic access clobbers the whole name
    /// space, so the consumers (the existence rung's clobbers, the W210 /
    /// W211 / W220 emitters, and the optimiser's O101 / O109 / O126) read it
    /// in `O(1)` and abstain.  See
    /// [`crate::dynamic_names`].
    pub dynamic_names: crate::dynamic_names::DynamicNameBarrier,
    /// Single source of truth for the deep-analysis complexity guard: when
    /// `true` (CFG block count **or** body bytes over the ceiling), `ssa` and
    /// the dataflow lattices are trivial and **every** per-proc diagnostic /
    /// optimiser pass must skip this function (consult the flag, not the cfg,
    /// so byte-large-but-block-light generated bodies are guarded
    /// consistently).
    pub complexity_guarded: bool,
    /// The precision tier the unit's lattices were computed at: the deep
    /// tier for every full build, the request's own tier for a request
    /// below it — the existence rung is a deep-tier fact, so such a request
    /// computes none — and [`AnalysisTier::ComplexityGuarded`] for a unit
    /// over the complexity ceiling, whose lattices are trivial.
    /// [`Self::existence`] answers `Unavailable` with it below the deep tier.
    pub tier: AnalysisTier,
    /// Byte offset to add to this unit's (otherwise relative) spans to recover
    /// **absolute** source positions (Approach B — offset-aware consumers).
    ///
    /// `0` for a unit built directly at its real source position (the top level
    /// and the uncached `build_for_with_config` path).  For a memoised procedure
    /// the unit is built at **offset 0** (a shifted-but-unchanged body interns to
    /// the same `function_lattice` key) and this carries the procedure's real body
    /// offset, so consumers recover absolute spans with [`Self::abs_span`] /
    /// [`Self::abs_pos`] instead of the build rebasing every span.  Span-free
    /// lattices (`types`/`taints`/`rendered_props`/`def_use`, keyed by
    /// `ValueKey`) are unaffected.
    pub base_offset: i64,
    /// The method-body instance-state view, for a unit built from a
    /// [`crate::ir::MethodDef`] — `None` for procs, lambdas,
    /// `namespace eval` bodies, and the top level, none of which have any.
    ///
    /// Span-free (names only), so offset rebasing never touches it.  Behind
    /// an `Arc` like the other shared lattices — the analyser and optimiser
    /// consumers read it many times per unit.
    pub method_facts: Option<Arc<MethodBodyFacts>>,
    /// Actual selected event body, independent of the function's report name.
    /// This conditional source frame owns no executing worker or current value.
    pub irules_event_body: Option<Arc<crate::ir::SourceIrulesEventBody>>,
    /// Original hosted procedure source card, without an entered worker or ABI.
    pub original_hosted_procedure_body:
        Option<Arc<crate::command_binding::OriginalHostedProcedureBody>>,
    /// Target-neutral executable/world semantic facts for this function.
    ///
    /// The sidecar records an explicit availability or decline when the
    /// current linear executable-IR compatibility layer cannot faithfully
    /// represent this source shape. Scalar SSA remains [`Self::ssa`] and the
    /// optional alias-aware cell SSA remains [`Self::memory_ssa`].
    pub semantic_facts: SemanticAnalysisBundle,
}

impl PartialEq for FunctionUnit {
    fn eq(&self, other: &Self) -> bool {
        self.source_metadata_input == other.source_metadata_input
            && self.source_config == other.source_config
            && self.name == other.name
            && self.body_id == other.body_id
            && self.cfg == other.cfg
            && self.ssa == other.ssa
            && self.def_use == other.def_use
            && self.sccp == other.sccp
            && self.semantic_value_projection == other.semantic_value_projection
            && self.retained_semantic_value_owner == other.retained_semantic_value_owner
            && self.types == other.types
            && self.return_type == other.return_type
            && self.taints == other.taints
            && self.rendered_props == other.rendered_props
            && self.memory_ssa == other.memory_ssa
            && self.dynamic_names == other.dynamic_names
            && self.complexity_guarded == other.complexity_guarded
            && self.tier == other.tier
            && self.base_offset == other.base_offset
            && self.method_facts == other.method_facts
            && self.irules_event_body == other.irules_event_body
            && self.original_hosted_procedure_body == other.original_hosted_procedure_body
            && self.semantic_facts == other.semantic_facts
    }
}

// The inventory witness lives with the types so all private fields can be
// enumerated exhaustively without exposing their values to reflection clients.
pub(crate) mod durable_field_inventory {
    use super::{CompilationUnit, FunctionUnit};

    macro_rules! durable_inventory {
        ($witness:ident, $fields:ident, $ty:ident, $($field:ident),+ $(,)?) => {
            /// Every durable field of the type, as `Type::field`.
            pub const $fields: &[&str] =
                &[$(concat!(stringify!($ty), "::", stringify!($field))),+];

            fn $witness(value: &$ty) {
                let $ty { $($field: _),+ } = value;
            }
        };
    }

    durable_inventory!(
        witness_function_unit,
        FUNCTION_UNIT_FIELDS,
        FunctionUnit,
        original_lattice_worker,
        original_lattice_callback_admission,
        source_metadata_input,
        source_config,
        name,
        body_id,
        cfg,
        ssa,
        def_use,
        sccp,
        semantic_value_projection,
        retained_semantic_value_owner,
        types,
        return_type,
        taints,
        rendered_props,
        memory_ssa,
        dynamic_names,
        complexity_guarded,
        tier,
        base_offset,
        method_facts,
        irules_event_body,
        original_hosted_procedure_body,
        semantic_facts,
    );

    durable_inventory!(
        witness_compilation_unit,
        COMPILATION_UNIT_FIELDS,
        CompilationUnit,
        optimiser_source_owner,
        source,
        ir_module,
        cfg_module,
        command_mutations,
        source_class_facts,
        top_level,
        procedures,
        methods,
        body_units,
        carried_installed_lowerings,
        source_body_cfgs,
        interproc,
        connection_scope,
        caller_scope,
        declared_commands,
        transfers,
    );

    /// Read-only exhaustive witness for both durable compiler types.
    ///
    /// The field-name inventory and each no-`..` destructure share one declaration.
    /// Private retained inputs remain private; consumers review their names through
    /// the inventory without obtaining authority to replace their values.
    pub fn assert_durable_field_inventory(unit: &CompilationUnit) {
        witness_function_unit(&unit.top_level);
        witness_compilation_unit(unit);
    }
}

/// Whole-module variable-trace fact that [`crate::sccp::sccp`] needs —
/// [`crate::ir::Module::traced_variables`] /
/// [`crate::ir::Module::has_dynamic_variable_trace`], threaded through
/// unchanged from `Module`. Bundled into one parameter (mirroring
/// `known_classes`'s "whole-unit fact, identical for every procedure"
/// shape) so [`FunctionUnit::build_with_param_constants_and_classes`]
/// stays under the clippy `too_many_arguments` ceiling.
#[derive(Debug, Clone, Copy)]
pub struct ModuleTraceFacts<'a> {
    /// [`crate::ir::Module::traced_variables`].
    pub traced_variables: &'a BTreeSet<String>,
    /// [`crate::ir::Module::has_dynamic_variable_trace`].
    pub has_dynamic_variable_trace: bool,
    /// [`crate::ir::Module::deferred_writes`]: the names the module's
    /// callback scripts write. The value lattices read it beside the traced
    /// names; the taint lattice does not, since a callback's write is no
    /// trace's read of tainted data.
    pub deferred_writes: &'a crate::ir::DeferredWrites,
}

/// The whole-module facts a per-procedure build runs under: the variable
/// traces and the analysis context every value-transfer answer is keyed
/// on. One value per module, shared by every memoised request; the context
/// joins [`ModuleTraceFacts`] once every constructor of that struct carries
/// it.
#[derive(Debug, Clone, Copy)]
pub struct ModuleAnalysisFacts<'a> {
    /// The module's variable-trace facts.
    pub trace: ModuleTraceFacts<'a>,
    /// The module's analysis context.
    pub analysis_context: &'a crate::value_transfer::AnalysisContextKey,
    /// The command trust the context's
    /// [`crate::command_binding::CommandTrustSnapshot`] stands for, built
    /// once per module rather than per procedure: the memo's interned
    /// context holds it rebuilt from the snapshot, and a whole-module build
    /// passes the scan the snapshot was taken from, which the snapshot
    /// round-trips to. Every route the shared lattice runs folds under it
    /// with the [`crate::sccp::FoldTrust::ObservedBindings`] stance.
    pub command_trust: &'a crate::command_binding::ModuleCommandMutations,
}

/// The two document-level facts a unit build always reads together: the
/// command registry (which carries the document's profile and specs) and
/// the lexer config its text is re-read under. Carried as one value so the
/// pair cannot be split — a registry holding one document's profile beside
/// a config holding another's is exactly the two-grammar drift the ingress
/// settlement forbids (`docs/design/registry/dialect-profile-model.md` §2.5).
#[derive(Clone, Copy)]
pub struct UnitDialect<'a> {
    /// The document's command registry.
    pub registry: &'a CommandRegistry,
    /// The document's body-lexing config.
    pub config: tcl_lexer::LexerConfig,
    /// Actual source availability; standalone callers explicitly supply none.
    pub source_metadata_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
}

/// Complete memo inputs beyond the function's name, CFG and parameters.
/// The event descriptor remains separate from normalized body geometry.
#[derive(Clone, Copy)]
pub struct FunctionLatticeInputs<'a> {
    /// Actual Registry and body lexer configuration.
    pub dialect: UnitDialect<'a>,
    /// Independently established caller-uniform parameter constants.
    pub param_constants:
        Option<&'a HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>>,
    /// Known class names from the complete compilation unit.
    pub known_classes: &'a HashSet<String>,
    /// Whole-module variable trace inventory.
    pub trace_facts: ModuleTraceFacts<'a>,
    /// Exact upstream value-transfer memo identity, separate from source admission.
    pub analysis_context: &'a crate::value_transfer::AnalysisContextKey,
    /// Exact module mutation obligations retained in the memo key.
    pub command_trust: &'a crate::command_binding::ModuleCommandMutations,
    /// Sealed original event and the actual normalized body used for this build.
    pub event_body: Option<(
        &'a Arc<crate::ir::SourceIrulesEventBody>,
        &'a crate::ir::Script,
    )>,
}

/// A procedure's interprocedural parameter seeds, keyed by parameter and
/// version.
type ParamSeeds<'a> = Option<
    &'a std::collections::HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>,
>;

/// The analysis inputs threaded into a [`FunctionUnit`] build beyond the
/// unit's own `name` / `cfg`, grouped into one parameter so the deep-build
/// entry point stays under clippy's `too_many_arguments` ceiling — mirroring
/// [`ModuleTraceFacts`]'s "one bundled whole-unit context" shape — and so the
/// SSA / SCCP / type / taint passes read a single context rather than eight
/// positional arguments.
#[derive(Clone, Copy)]
struct FunctionBuildInputs<'a> {
    analysis_body: Option<crate::sccp::AnalysisBody>,
    is_event_body: bool,
    /// Initial presence facts are separate from actual source frame ownership.
    initial_state: crate::sccp::ExistenceInitialState,
    source_metadata_input: Option<&'a crate::analyser::ResolvedAnalysisInput>,
    /// Actual frame entry; synthetic body units must not infer it from their names.
    entry_context: Option<&'a crate::var_resolve::ResolveContext>,
    /// The document's body-lexing config: what every re-read of a `[…]`
    /// text inside the build (the dynamic-name barrier) lexes under.
    pub config: tcl_lexer::LexerConfig,
    /// Formal parameter names (seed `[info exists]` folds and param constants).
    params: &'a [String],
    /// The analysis dialect's command registry.
    registry: &'a CommandRegistry,
    /// Known per-parameter constant folds, when the caller has them.
    param_constants: Option<
        &'a std::collections::HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>,
    >,
    /// Classes in scope, for `OBJECT` typing of `new` / `create` calls.
    known_classes: &'a HashSet<String>,
    /// Extra names SCCP must treat as escaping (a top-level module fact).
    extra_global_escaping: &'a HashSet<String>,
    /// Whole-module variable-trace facts.
    trace_facts: ModuleTraceFacts<'a>,
    /// The analysis context's memo identity — the module's command-binding
    /// evidence and the registry, tier, and evaluator generations — when the
    /// caller carries one; the value-transfer driver runs detached otherwise.
    analysis_context: Option<&'a crate::value_transfer::AnalysisContextKey>,
    /// Whole-module command-mutation trust
    /// ([`crate::command_binding::ModuleCommandMutations`]) — which command
    /// names still denote the builtin they spell. The value-transfer driver
    /// answers for a resolved head only when this trusts it, so a module
    /// with a shadowing `proc llength …` gets no `[llength …]` fold anywhere
    /// in the lattice rather than one that contradicts the optimiser's own
    /// proc-call fold (#2164).
    command_trust: &'a crate::command_binding::ModuleCommandMutations,
    /// Names auto-bound to out-of-frame *object* storage on entry — a
    /// `TclOO` method body's [`crate::ir::MethodDef::instance_vars`].  `None`
    /// for procs, lambdas, and the top level, none of which have any.  The
    /// `[info exists]` fold must abstain on these.
    object_state: Option<&'a HashSet<String>>,
    /// Whether this is the compilation unit's **top-level** body — the
    /// interpreter's initial global frame.  The `[info exists]` fold must
    /// abstain on the registry's special variables there.
    initial_global: bool,
    /// The module's procedures, where the build holds them: what a
    /// parameter default or a callee's transfer summary is read from.
    procedures: Option<&'a crate::interprocedural::ModuleProcedures<'a>>,
}

impl<'a> FunctionBuildInputs<'a> {
    fn value_fact_inputs(
        self,
        policy: crate::tcl_expr_eval::FoldPolicy,
    ) -> crate::sccp::ValueFactInputs<'a> {
        crate::sccp::ValueFactInputs {
            param_constants: self.param_constants,
            policy,
            extra_escaping: self.extra_global_escaping,
            trace: crate::sccp::TraceInputs {
                source_metadata_input: self.source_metadata_input,
                registry: self.registry,
                traced_variables: self.trace_facts.traced_variables,
                has_dynamic_variable_trace: self.trace_facts.has_dynamic_variable_trace,
                deferred_writes: self.trace_facts.deferred_writes,
                analysis_context: self.analysis_context,
                existence: None,
            },
            folds: Some(crate::sccp::BuiltinFoldInputs {
                source_metadata_input: self.source_metadata_input,
                registry: self.registry,
                mutations: self.command_trust,
                dialect: None,
                defining_class: None,
                registry_engine: false,
                trust: crate::sccp::FoldTrust::ObservedBindings,
                proven_pure_parameters: false,
            }),
        }
    }

    /// The frame facts the existence rung enters the function `name` with.
    /// It reads the computed names per statement, so it takes the module's
    /// own trace fact rather than the widened one the values run under; an
    /// iRules `when` handler adds the names its connection may hold.
    fn existence_entry(&self, _name: &str) -> crate::sccp::ExistenceEntry<'a> {
        crate::sccp::ExistenceEntry {
            initial_state: self.initial_state,
            params: self.params,
            object_state: self.object_state,
            initial_global: self.initial_global,
            connection_scoped: self
                .analysis_context
                .map(|key| &key.connection_scoped)
                .filter(|_| self.is_event_body),
            dynamic_trace: self.trace_facts.has_dynamic_variable_trace
                || self.trace_facts.deferred_writes.any,
            config: self.config,
            caller_places: None,
        }
    }
}

impl<'a> ModuleTraceFacts<'a> {
    /// The trace facts lowering recorded on `module`.
    #[must_use]
    pub fn of(module: &'a IrModule) -> Self {
        Self {
            traced_variables: &module.traced_variables,
            has_dynamic_variable_trace: module.has_dynamic_variable_trace,
            deferred_writes: &module.deferred_writes,
        }
    }

    /// Whether the trace owner says a module trace may observe `name`.
    #[must_use]
    pub fn variable_may_be_traced(&self, name: &str) -> bool {
        crate::command_binding::trace_registrations::variable_may_be_traced(
            self.traced_variables,
            self.has_dynamic_variable_trace,
            name,
        )
    }

    /// No `Module` in hand (a standalone per-function build) — behaviourally
    /// identical to "nothing is traced".
    #[must_use]
    pub fn none() -> Self {
        // A `'static` empty set is safe to hand out as `'a` for any `'a`: an
        // immutable, never-mutated `BTreeSet` needs no real backing storage
        // lifetime tie, only a place to point.
        static EMPTY: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();
        Self {
            traced_variables: EMPTY.get_or_init(BTreeSet::new),
            has_dynamic_variable_trace: false,
            deferred_writes: &crate::ir::NO_DEFERRED_WRITES,
        }
    }
}

#[derive(Clone, Copy)]
enum RetainedSemanticDepth {
    Interactive,
    Full,
}

// Optional phase measurements carry no source, cache or admission authority.
#[cfg(any(test, debug_assertions))]
#[derive(Default)]
struct SourceFunctionPhaseTotals {
    label: &'static str,
    functions: usize,
    blocks: usize,
    statements: usize,
    micros: [u128; 10],
}

#[cfg(any(test, debug_assertions))]
std::thread_local! {
    static SOURCE_FUNCTION_PHASE_TOTALS: std::cell::RefCell<Vec<SourceFunctionPhaseTotals>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

mod body_units;
mod build_entry;
mod build_stages;
mod conditional_original_body;
mod function_build;
mod function_queries;
mod interprocedural_analysis;
mod lattice_request;
mod memory_and_semantic_facts;
mod module_facts;
mod module_lowering_memo;
pub use module_lowering_memo::{LoweredModule, ModuleLoweringCache, ModuleLoweringInputs};
mod procedure_units;
mod source_lowering;
mod template_instantiation;
mod unit_queries;

#[cfg(test)]
use function_build::conditional_function_source_entry;
pub(crate) use function_build::function_source_entry;
pub use lattice_request::{LatticeRequest, ProcLatticeCache, decode_param_constants};
pub use module_facts::RetainedSourceClassFacts;
#[cfg(test)]
use module_facts::collect_known_classes;
#[cfg(test)]
use source_lowering::lower_and_build_cfg;
#[cfg(test)]
use source_lowering::resolve_unit_scope;

#[cfg(any(test, debug_assertions))]
struct SourcePipelineTrace {
    start: Option<std::time::Instant>,
    label: &'static str,
    bytes: usize,
    procedures: usize,
}

#[cfg(any(test, debug_assertions))]
impl SourcePipelineTrace {
    fn new(label: &'static str, bytes: usize) -> Self {
        let start = tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(std::time::Instant::now);
        if start.is_some() {
            SOURCE_FUNCTION_PHASE_TOTALS.with(|stack| {
                stack.borrow_mut().push(SourceFunctionPhaseTotals {
                    label,
                    ..Default::default()
                });
            });
        }
        Self {
            start,
            label,
            bytes,
            procedures: 0,
        }
    }

    fn phase(&self, stage: &'static str) {
        if let Some(start) = self.start {
            eprintln!(
                "COMPILER_PIPELINE_PHASE pipeline={} bytes={} procedures={} stage={stage} us={}",
                self.label,
                self.bytes,
                self.procedures,
                start.elapsed().as_micros()
            );
        }
    }
}

#[cfg(any(test, debug_assertions))]
fn print_function_phase_totals(totals: &SourceFunctionPhaseTotals, final_summary: bool) {
    eprintln!(
        "COMPILER_FUNCTION_PHASE_TOTALS pipeline={} final={final_summary} functions={} blocks={} statements={} prepare_us={} ssa_us={} def_use_us={} dynamic_names_us={} sccp_us={} existence_us={} types_us={} rendered_us={} instance_classes_us={} taint_us={}",
        totals.label,
        totals.functions,
        totals.blocks,
        totals.statements,
        totals.micros[0],
        totals.micros[1],
        totals.micros[2],
        totals.micros[3],
        totals.micros[4],
        totals.micros[5],
        totals.micros[6],
        totals.micros[7],
        totals.micros[8],
        totals.micros[9]
    );
}

#[cfg(any(test, debug_assertions))]
impl Drop for SourcePipelineTrace {
    fn drop(&mut self) {
        if self.start.is_some() {
            SOURCE_FUNCTION_PHASE_TOTALS.with(|stack| {
                if let Some(totals) = stack.borrow_mut().pop()
                    && totals.functions != 0
                {
                    print_function_phase_totals(&totals, true);
                }
            });
        }
    }
}

#[cfg(any(test, debug_assertions))]
struct SourceFunctionPhaseTrace {
    checkpoint: Option<std::time::Instant>,
    micros: [u128; 10],
}

#[cfg(any(test, debug_assertions))]
impl SourceFunctionPhaseTrace {
    fn new() -> Self {
        let enabled = SOURCE_FUNCTION_PHASE_TOTALS.with(|stack| !stack.borrow().is_empty());
        Self {
            checkpoint: enabled.then(std::time::Instant::now),
            micros: [0; 10],
        }
    }

    fn phase(&mut self, index: usize) {
        if let Some(checkpoint) = self.checkpoint.as_mut() {
            self.micros[index] += checkpoint.elapsed().as_micros();
            *checkpoint = std::time::Instant::now();
        }
    }

    fn finish(self, cfg: &CfgFunction) {
        if self.checkpoint.is_none() {
            return;
        }
        SOURCE_FUNCTION_PHASE_TOTALS.with(|stack| {
            if let Some(totals) = stack.borrow_mut().last_mut() {
                totals.functions += 1;
                totals.blocks += cfg.blocks.len();
                totals.statements += cfg
                    .blocks
                    .values()
                    .map(|block| block.statements.len())
                    .sum::<usize>();
                for (total, micros) in totals.micros.iter_mut().zip(self.micros) {
                    *total += micros;
                }
                if totals.functions.is_multiple_of(64) {
                    print_function_phase_totals(totals, false);
                }
            }
        });
    }
}

// Module-level compilation unit

/// Complete compilation artefacts for a source document.
///
/// Built once, consumed many times across the diagnostics cycle.
///
/// `PartialEq` enables the salsa-native [`compilation_unit`] query (in
/// `tcl-lsp-db`) to return `Arc<CompilationUnit>` — both diagnostics consumers
/// share one build per edit — using salsa's equality-backed memoisation, exactly
/// as `FunctionUnit` does for [`function_lattice`].
#[derive(Debug, Clone)]
pub struct CompilationUnit {
    optimiser_source_owner: Option<optimiser_source_owner::OriginalOptimiserPreparedUnit>,
    /// Source text (kept so downstream passes that need raw
    /// lexing can re-scan ranges without reparsing).
    pub source: String,
    /// IR module produced by lowering.
    pub ir_module: IrModule,
    /// Module-level CFG.
    pub cfg_module: CfgModule,
    /// Whole-module command trust projected from the binding summary already
    /// prepared for CFG construction.
    pub command_mutations: crate::command_binding::ModuleCommandMutations,
    /// Complete descriptive source class roster retained by the genuine CU build.
    /// Synthetic units explicitly retain absence; names grant no Native identity.
    pub source_class_facts: Option<Arc<RetainedSourceClassFacts>>,
    /// Top-level script analysis.
    pub top_level: FunctionUnit,
    /// Per-procedure analyses joined to their retained module body ID.
    pub procedures: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    /// Method analyses keyed by their actual Module-issued body ID.
    /// Member kind, declaring source owner and display labels remain separate.
    pub methods: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    /// Anonymous and selected-namespace body analyses keyed by retained body ID.
    /// [`Self::body_function_units`] exposes them with their actual source owners.
    pub body_units: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    /// Analyses of the Installed lowerings that readonly authored body units
    /// carry, keyed by each lowering's own body ID. [`Self::body_units`]
    /// holds the analysis units; [`Self::body_lowering_functions`] reaches both.
    pub carried_installed_lowerings: HashMap<crate::ir::ProcedureId, FunctionUnit>,
    /// Genuine prepared child CFGs, retained before any function analysis.
    source_body_cfgs: HashMap<crate::ir::ProcedureId, CfgFunction>,
    /// Interprocedural summary (optional — populated when the
    /// interprocedural pass has been run).
    pub interproc: Option<InterproceduralAnalysis>,
    /// Cross-event variable scope analysis (consumed by the IRULE4005
    /// emitter).  ``Some`` when at least one ``::when::*`` procedure is
    /// in [`Self::procedures`]; ``None`` for non-iRules sources or any
    /// source with no ``when`` blocks.
    pub connection_scope: Option<crate::connection_scope::ConnectionScope>,
    /// Who else can call this unit's procedures — the inputs the
    /// interprocedural constant seed was (or was not) allowed to trust.
    /// Surfaced by the compiler explorer's **Unit Scope** view; see
    /// [`crate::unit_scope`].
    pub caller_scope: UnitCallerScope,
    /// The document's own command declarations
    /// ([`UnitBuildOptions::declared_commands`]), owned so every pass that
    /// runs *after* the build — [`Self::with_interprocedural`] above all —
    /// asks the same surface the lowering did. Empty for a document that
    /// declares nothing.
    pub declared_commands: tcl_registry::model::DeclaredSurface,
    /// Each procedure's transfer summary: what a call to it does to its
    /// caller's places.
    pub transfers: crate::interprocedural::TransferSummaries,
}
// Preserve the existing semantic/cache equality projection exactly. The new
// producer receipt is validated independently, never a new Salsa dependency.
impl PartialEq for CompilationUnit {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.ir_module == other.ir_module
            && self.cfg_module == other.cfg_module
            && self.command_mutations == other.command_mutations
            && self.source_class_facts == other.source_class_facts
            && self.top_level == other.top_level
            && self.procedures == other.procedures
            && self.methods == other.methods
            && self.body_units == other.body_units
            && self.carried_installed_lowerings == other.carried_installed_lowerings
            && self.source_body_cfgs == other.source_body_cfgs
            && self.interproc == other.interproc
            && self.connection_scope == other.connection_scope
            && self.caller_scope == other.caller_scope
            && self.declared_commands == other.declared_commands
            && self.transfers == other.transfers
    }
}

pub(crate) mod original_lattice_worker;
pub use original_lattice_worker::{OriginalFunctionLatticeInputs, OriginalFunctionLatticeTemplate};
mod optimiser_source_owner;
pub(crate) use optimiser_source_owner::OriginalOptimiserPreparedUnit;
pub use optimiser_source_owner::{
    OriginalInterproceduralTaintRequest, OriginalOptimiserSourceOrigin,
};

/// The unit-scope facts a build resolved, kept on the finished
/// [`CompilationUnit`] so the explorer — and anyone debugging a missing or
/// unexpected fold — can see exactly why the interprocedural seed fired or
/// declined.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitCallerScope {
    /// Registry-declared unit boundaries this file crosses
    /// ([`crate::unit_scope::scan_unit_linkage`]).
    pub linkage: tcl_registry::Traits,
    /// Whether the host supplied a cross-file view
    /// ([`UnitBuildOptions::external_call_sites`]).
    pub has_cross_file_evidence: bool,
    /// The merged (in-unit + cross-file) call-site evidence the seed read.
    pub call_sites: crate::unit_scope::CallSiteEvidence,
    /// Narrow declared-procedure identity trust projected from the prepared
    /// command-binding summary. Unlike the unit's `command_mutations`, this
    /// excludes transitive opacity introduced by an unresolved/autoloaded
    /// command in a retained body. Consumers must combine it with an exact
    /// flow-sensitive call-site binding.
    pub(crate) proc_binding_trust: crate::command_binding::ProcBindingTrustProjection,
}

/// Whole-module facts every per-function build in a [`CompilationUnit`] build
/// consumes, gathered once.
struct ModuleWideFacts {
    /// Fully-qualified names of every class defined in the unit.
    known_class_set: Arc<HashSet<String>>,
    source_class_facts: Option<Arc<RetainedSourceClassFacts>>,
    /// [`Self::known_class_set`], sorted — the form each `LatticeRequest`
    /// carries into its memo key, so a class-set change invalidates the
    /// per-procedure lattices.
    known_classes: Vec<String>,
    /// Every name any procedure/method declares via `global NAME`.
    top_level_extra_escaping: HashSet<String>,
    /// [`crate::ir::Module::traced_variables`] as a sorted `Vec` (a `BTreeSet`
    /// iterates in order), mirroring `known_classes`.
    traced_variable_names: Vec<String>,
}

#[derive(Clone, Copy)]
struct OriginalBodyCacheInputs<'a> {
    legacy: Option<&'a BodyLoweringCache<'a>>,
    original: Option<&'a crate::lowering::OriginalBodyLoweringCache<'a>>,
    module: Option<&'a ModuleLoweringCache<'a>>,
}

#[derive(Clone, Copy)]
struct OriginalLoweringCaches<'a> {
    legacy: Option<&'a BodyLoweringCache<'a>>,
    source: Option<&'a crate::lowering::SourceBodySyntaxCache<'a>>,
    original: Option<&'a crate::lowering::OriginalBodyLoweringCache<'a>>,
    integrity: &'a std::cell::Cell<bool>,
}

/// Select an explicitly requested default metadata generation.
/// Actual source consumers retain their `ResolvedAnalysisInput` instead;
/// this static handle cannot substitute for its keyed store or availability.
#[must_use]
pub fn semantic_context(
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<SemanticContext> {
    dialect.map(SemanticContext::for_profile)
}

/// Module-wide, read-only inputs the non-procedure body builds
/// ([`CompilationUnit::build_method_units`] and
/// [`CompilationUnit::build_body_units`]) share. Bundled so each builder
/// takes three parameters rather than eight.
#[derive(Clone, Copy)]
struct BodyUnitContext<'a> {
    registry: &'a CommandRegistry,
    known_class_set: &'a HashSet<String>,
    trace_facts: ModuleTraceFacts<'a>,
    /// Whole-module command-mutation trust — see
    /// [`FunctionBuildInputs::command_trust`].
    command_trust: &'a crate::command_binding::ModuleCommandMutations,
    config: tcl_lexer::LexerConfig,
    native_source: Option<&'a crate::interprocedural::ModuleProcedures<'a>>,
}

#[derive(Clone, Copy)]
struct DeclaredBodyMemoContext<'a> {
    prepared: Option<&'a CfgContext>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    known_classes: &'a [String],
    traced_variables: &'a [String],
    analysis_context: &'a crate::value_transfer::AnalysisContextKey,
}

/// Module-wide, read-only inputs [`build_procedure_units`] shares across
/// every procedure in the loop.  Grouped into one struct so the extracted
/// helper takes two parameters instead of a dozen.
struct ProcedureBuildContext<'a> {
    ir_module: &'a IrModule,
    cfg_module: &'a CfgModule,
    cfg_context: Option<&'a CfgContext>,
    registry: &'a CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    known_class_set: &'a HashSet<String>,
    known_classes: &'a [String],
    traced_variable_names: &'a [String],
    trace_facts: ModuleTraceFacts<'a>,
    /// The module's analysis context key, shared by every procedure. Its
    /// [`crate::command_binding::CommandTrustSnapshot`] is the module's
    /// command trust, which every procedure's lattice — memoised or not —
    /// folds under (see
    /// [`FunctionUnit::build_with_param_constants_and_classes_under`]).
    analysis_context: &'a crate::value_transfer::AnalysisContextKey,
    /// The module's command-mutation scan the context's snapshot was taken
    /// from: the trust a fresh procedure build folds under.
    command_trust: &'a crate::command_binding::ModuleCommandMutations,
    /// Procedures whose CFG has module-derived instance-option writes. Their
    /// annotated CFG cannot be reconstructed from the body-only lattice memo.
    tainted_global_writes: &'a HashMap<crate::ir::ProcedureId, HashSet<String>>,
    /// The module's procedures, which a lattice reading another procedure —
    /// a parameter default, a callee's transfer summary — reads. The memo
    /// keys on one procedure's body, so a lattice that read them is built
    /// here instead ([`crate::sccp::SccpResult::reads_module`]).
    procedures: Option<&'a crate::interprocedural::ModuleProcedures<'a>>,
}

impl<'a> ProcedureBuildContext<'a> {
    /// The whole-module facts every procedure of the module builds under.
    fn module_facts(&self) -> ModuleAnalysisFacts<'a> {
        ModuleAnalysisFacts {
            trace: self.trace_facts,
            analysis_context: self.analysis_context,
            command_trust: self.command_trust,
        }
    }
}

impl CompilationUnit {
    pub(crate) fn original_lattice_callbacks_are_current(&self) -> bool {
        std::iter::once(&self.top_level)
            .chain(self.procedures.values())
            .chain(self.methods.values())
            .chain(self.body_units.values())
            .chain(self.carried_installed_lowerings.values())
            .all(|function| function.original_lattice_callback_admission != Some(false))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod smoke;

/// A unit built for the *string* `"jim"` is built under Jim's grammar and
/// hands codegen the name `jim` — not the permissive fallback's grammar and
/// the name `tcl`, which would bypass the centralised resolution entirely.
#[cfg(test)]
mod jim_ingress_tests;

#[cfg(test)]
mod original_receiver_method_facts_tests;

#[cfg(test)]
mod conditional_original_body_tests;

#[cfg(test)]
mod selected_class_roster_tests;

#[cfg(test)]
mod original_native_declaration_diagnostic_owner_tests;
