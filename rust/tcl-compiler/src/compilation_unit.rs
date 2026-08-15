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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use tcl_registry::CommandRegistry;
use tcl_registry::dialects::DialectSet;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::cfg_builder::build_cfg;
use crate::def_use::{DefUseResult, build_def_use_chains};
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Module as IrModule;
use crate::memory_ssa::{MemorySsaFunction, build_memory_ssa};
use crate::rendered_properties::{RenderedValueProps, propagate_rendered_props};
use crate::sccp::{SccpResult, sccp_with_extra_escaping};
use crate::semantic_analysis::SemanticAnalysisBundle;
use crate::ssa::{SsaFunction, ValueKey, build_ssa};
use crate::taint::{TaintGraph, TaintLattice, propagate_taints};
use crate::type_infer::propagate_types;
use crate::types::TypeLattice;
use crate::unit_scope::{
    build_extra_call_site_scan_contexts, collect_call_site_constants,
    params_constants_from_call_sites,
};

/// Module-wide CFG-determining context (upvar summaries + proc params +
/// global-write summaries) shared by every procedure/method build so each
/// rebuilt CFG matches the whole-module build.  Produced by
/// [`crate::cfg_builder::prepare_cfg_context`]; re-exported here under the
/// same name for this file's existing call sites (the canonical definition
/// lives in [`crate::cfg_builder::CfgContext`]).
type CfgContext = crate::cfg_builder::CfgContext;

/// One procedure's **offset-0** baseline-lattice build request, handed to the
/// [`ProcLatticeCache`] callback by [`CompilationUnit::build_for_memoized`].
///
/// The body has already been normalised to offset 0 (every span shifted by
/// `-body_offset`), so a shifted-but-unchanged procedure produces an identical
/// request — the salsa-native memo (`tcl-lsp-db`'s `function_lattice`) keys on
/// it position-independently and the builder rebases the returned unit back to
/// the procedure's real offset.  Carries exactly what
/// [`crate::cfg_builder::build_cfg_function_with_upvars`] +
/// [`FunctionUnit::build`] consume: the offset-0 body, the qualified name, the
/// parameter list, the module-wide upvar / proc-param context (so the rebuilt
/// CFG is identical to the whole-module build's), and the analysis dialect.
pub struct LatticeRequest<'a> {
    /// Qualified procedure name (e.g. `::foo::bar`).
    pub qname: &'a str,
    /// The procedure body, normalised to offset 0.
    pub body: &'a crate::ir::Script,
    /// The procedure's declared parameters.
    pub params: &'a [String],
    /// Module-wide `proc -> upvar summary` context (from
    /// [`crate::cfg_builder::prepare_cfg_context`]).
    pub upvar_procs: &'a HashMap<String, crate::cfg_builder::upvar_info::UpvarInfo>,
    /// Module-wide `proc -> params` context (from
    /// [`crate::cfg_builder::prepare_cfg_context`]).
    pub proc_params: &'a HashMap<String, Vec<String>>,
    /// Module-wide `proc -> outer-scope write summary` context (from
    /// [`crate::cfg_builder::prepare_cfg_context`]).
    pub global_write_procs:
        &'a HashMap<String, crate::cfg_builder::global_write_info::GlobalWriteInfo>,
    /// Analysis dialect — selects the registry the lattice pipeline runs under.
    pub dialect: &'a str,
    /// Interprocedural caller-uniform-literal SCCP seeds for this procedure
    /// (from [`params_constants_from_call_sites`]), encoded in the
    /// deterministic, hashable `(param, version, string)` form the memo key
    /// interns (see [`encode_param_constants`]).  Empty means "no seeds".
    /// Position-independent — keyed by parameter name + SSA version, never by
    /// span — so it rebases trivially with the rest of the offset-0 unit.
    pub param_constants: &'a [(String, u32, String)],
    /// Fully-qualified names of every class defined in the compilation unit
    /// (sorted), so the type-propagation pass can recognise a `TclOO` / itcl
    /// constructor call (`Foo new`) and type it `OBJECT(::ns::Foo)`.  A
    /// whole-unit fact, identical for every procedure, so it is folded into the
    /// memo key: adding or removing a class anywhere invalidates each
    /// procedure's lattice (a new class can change any body's constructor
    /// typing).  Sourced from [`crate::signature_scan`] so the standalone and
    /// incremental builds derive an identical set from the same source.
    pub known_classes: &'a [String],
    /// Literal variable-trace target names from [`crate::ir::Module::
    /// traced_variables`] (sorted, mirroring `known_classes`) — a
    /// whole-module fact SCCP's trace-safety gate needs (see
    /// [`crate::sccp::sccp`]).  Folded into the memo key like
    /// `known_classes`: a trace installed anywhere in the module can change
    /// any procedure's SCCP result, so adding/removing one must invalidate
    /// every cached lattice, not just the procedure whose body carries the
    /// `trace` call.
    pub traced_variables: &'a [String],
    /// [`crate::ir::Module::has_dynamic_variable_trace`] — `true` when a
    /// variable-trace install/remove call targets a non-literal name
    /// anywhere in the module, which SCCP must treat as "every variable is
    /// potentially traced". Whole-module, folded into the memo key
    /// alongside `traced_variables`.
    pub has_dynamic_variable_trace: bool,
}

/// Salsa-native per-procedure lattice memo used by
/// [`CompilationUnit::build_for_memoized`].
///
/// `cache(request)` returns the **offset-0** [`FunctionUnit`] for `request`
/// (building it on a miss, reusing a memoised one on a hit).  The builder
/// rebases the returned unit to the procedure's real position, so the result is
/// byte-identical to [`CompilationUnit::build_for_with_config`]; only the
/// redundant lattice recompute for an unchanged procedure body is skipped.  The
/// caller owns the backing store and its eviction policy.
pub type ProcLatticeCache<'a> = dyn FnMut(&LatticeRequest<'_>) -> FunctionUnit + 'a;

/// Memoised per-procedure **body-lowering** callback (SRV-INCREMENTAL Task 3):
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
    /// Analysis dialect key; `""` for plain Tcl.
    pub dialect: &'a str,
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
}

/// Callback type for [`CompilationUnit::with_interprocedural_memoized`].
///
/// Given a procedure's qualified name and the whole-module
/// [`InterproceduralAnalysis`], returns its (memoised) interprocedural taints,
/// or `None` to fall back to a fresh [`FunctionUnit::interproc_taints`] re-run.
pub type TaintCascadeCallback<'a> =
    dyn FnMut(&str, &InterproceduralAnalysis) -> Option<Arc<HashMap<ValueKey, TaintLattice>>> + 'a;

// Per-function analysis bundle

/// The single method-body view of `TclOO` instance state, built **once** per
/// method unit from its typed [`crate::ir::MethodDef`] and read by every
/// consumer that needs "which names are auto-bound in this method's frame"
/// (issue #1174).
///
/// Before this carrier existed the same fact reached method-body analyses
/// through three independent channels — [`FunctionUnit::build_for_method`]'s
/// `object_state` (the `[info exists]` fold, I230 / O100 / O101), the
/// analyser's `emit_method_body_diagnostics` rebuilding `known_bound` from the
/// IR for W210/W211/W220, and the optimiser's `oo_method_constants` handing
/// `MethodDef::instance_vars` to `sccp_with_extra_escaping` — exactly the
/// parallel-channel shape that produced issue #1129 (two copies of the
/// existence fold sourcing parameters from different maps).  All three now
/// read this struct off [`FunctionUnit::method_facts`], so they cannot diverge
/// by construction.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MethodBodyFacts {
    /// The method's own formal parameter names, in declaration order.
    pub params: Vec<String>,
    /// Names auto-bound to out-of-frame *object* storage on entry — the
    /// class-wide cross-definition-block union lowering computes
    /// ([`crate::ir::MethodDef::instance_vars`]).
    pub instance_vars: HashSet<String>,
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
        }
    }

    /// Names bound at entry to the method's frame: its own parameters plus
    /// the auto-linked instance variables — the W210-family `known_bound`
    /// set (and, cloned, the seed of its `cross_event_vars` suppression
    /// set: a "setter" method writing an instance var with no local read is
    /// not a dead store, another method reads it later).
    #[must_use]
    pub fn known_bound_at_entry(&self) -> HashSet<String> {
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
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionUnit {
    /// Qualified function name (e.g. `::top`, `::foo::bar`).
    pub name: String,
    /// Control-flow graph.
    pub cfg: CfgFunction,
    /// SSA form.
    pub ssa: SsaFunction,
    /// Def-use chains.
    ///
    /// Shared behind an `Arc` (like `types` / `taints` / `rendered_props`)
    /// because it is **span-free**: [`crate::lattice_rebase::rebase_function_unit`]
    /// never touches it, so a memoised unit taken from the salsa
    /// `function_lattice` cache and rebased to a new offset keeps the very same
    /// lattice. Deep-copying it per procedure per read was pure waste
    /// (issue #1159).
    pub def_use: Arc<DefUseResult>,
    /// SCCP result: lattice values, executable blocks, constant
    /// branches.
    pub sccp: SccpResult,
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
    /// space, so the consumers ([`crate::sccp::existence_constant_branches`]'s
    /// existence fold, the W210 / W211 / W220 emitters, and the optimiser's
    /// O101 / O109 / O126) read it in `O(1)` and abstain.  See
    /// [`crate::dynamic_names`].
    pub dynamic_names: crate::dynamic_names::DynamicNameBarrier,
    /// Single source of truth for the deep-analysis complexity guard: when
    /// `true` (CFG block count **or** body bytes over the ceiling), `ssa` and
    /// the dataflow lattices are trivial and **every** per-proc diagnostic /
    /// optimiser pass must skip this function (consult the flag, not the cfg,
    /// so byte-large-but-block-light generated bodies are guarded
    /// consistently).
    pub complexity_guarded: bool,
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
    /// [`crate::ir::MethodDef`] (issue #1174) — `None` for procs, lambdas,
    /// `namespace eval` bodies, and the top level, none of which have any.
    ///
    /// Span-free (names only), so offset rebasing never touches it.  Behind
    /// an `Arc` like the other shared lattices — the analyser and optimiser
    /// consumers read it many times per unit.
    pub method_facts: Option<Arc<MethodBodyFacts>>,
    /// Target-neutral executable/world semantic facts for this function.
    ///
    /// The sidecar records an explicit availability or decline when the
    /// current linear executable-IR compatibility layer cannot faithfully
    /// represent this source shape. Scalar SSA remains [`Self::ssa`] and the
    /// optional alias-aware cell SSA remains [`Self::memory_ssa`].
    pub semantic_facts: SemanticAnalysisBundle,
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
}

/// The analysis inputs threaded into a [`FunctionUnit`] build beyond the
/// unit's own `name` / `cfg`, grouped into one parameter so the deep-build
/// entry point stays under clippy's `too_many_arguments` ceiling — mirroring
/// [`ModuleTraceFacts`]'s "one bundled whole-unit context" shape — and so the
/// SSA / SCCP / type / taint passes read a single context rather than eight
/// positional arguments.
#[derive(Clone, Copy)]
struct FunctionBuildInputs<'a> {
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
    /// Names auto-bound to out-of-frame *object* storage on entry — a
    /// `TclOO` method body's [`crate::ir::MethodDef::instance_vars`].  `None`
    /// for procs, lambdas, and the top level, none of which have any.  The
    /// `[info exists]` fold must abstain on these (issue #1129).
    object_state: Option<&'a HashSet<String>>,
}

impl ModuleTraceFacts<'_> {
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
        }
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
    ) -> Self {
        Self::build_with_param_constants(name, cfg, params, registry, None)
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
    ) -> Self {
        Self::build_with_param_constants_and_classes(
            name,
            cfg,
            params,
            registry,
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
        registry: &CommandRegistry,
        param_constants: Option<
            &std::collections::HashMap<
                (String, crate::ssa::Version),
                crate::analyses::LatticeValue,
            >,
        >,
        known_classes: &HashSet<String>,
        trace_facts: ModuleTraceFacts<'_>,
    ) -> Self {
        let no_extra_escaping = HashSet::new();
        Self::build_full(
            name,
            cfg,
            FunctionBuildInputs {
                params,
                registry,
                param_constants,
                known_classes,
                extra_global_escaping: &no_extra_escaping,
                trace_facts,
                object_state: None,
            },
        )
    }

    /// Build the compilation unit's **top-level** body unit — no parameters,
    /// no interprocedural seeds, no object state, but a non-empty
    /// `extra_global_escaping` (top-level names already live in the global
    /// frame, so another procedure's `global NAME` can reassign them; see
    /// [`crate::sccp::sccp_with_extra_escaping`]).
    #[must_use]
    pub fn build_top_level(
        cfg: CfgFunction,
        registry: &CommandRegistry,
        known_classes: &HashSet<String>,
        extra_global_escaping: &HashSet<String>,
        trace_facts: ModuleTraceFacts<'_>,
    ) -> Self {
        Self::build_full(
            "::top",
            cfg,
            FunctionBuildInputs {
                params: &[],
                registry,
                param_constants: None,
                known_classes,
                extra_global_escaping,
                trace_facts,
                object_state: None,
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
    /// in the body, so without it `[info exists x]` folded to "always
    /// absent" — issue #1129), and — via [`Self::method_facts`], the single
    /// carrier built here (issue #1174) — the analyser's W210/W211/W220
    /// `known_bound` set and the optimiser's method-constants escaping set.
    #[must_use]
    pub fn build_for_method(
        name: impl Into<String>,
        cfg: CfgFunction,
        method: &crate::ir::MethodDef,
        registry: &CommandRegistry,
        known_classes: &HashSet<String>,
        trace_facts: ModuleTraceFacts<'_>,
    ) -> Self {
        let facts = Arc::new(MethodBodyFacts::from_method(method));
        let no_extra_escaping = HashSet::new();
        let mut unit = Self::build_full(
            name,
            cfg,
            FunctionBuildInputs {
                params: &facts.params,
                registry,
                param_constants: None,
                known_classes,
                extra_global_escaping: &no_extra_escaping,
                trace_facts,
                object_state: Some(&facts.instance_vars),
            },
        );
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
    fn build_full(
        name: impl Into<String>,
        cfg: CfgFunction,
        inputs: FunctionBuildInputs<'_>,
    ) -> Self {
        let FunctionBuildInputs {
            params,
            registry,
            param_constants,
            known_classes,
            extra_global_escaping,
            trace_facts,
            object_state,
        } = inputs;
        // Complexity guard (block-count half): a pathologically large body
        // would cost seconds of SSA + dataflow for near-zero findings, so skip
        // the deep analysis and flag the unit. The body-byte half is applied by
        // the callers that have the body span (see `build_for_with_config`).
        // Backstop for every path through here — `build`, methods, and the
        // salsa `function_lattice` callbacks.
        if crate::ssa::is_complexity_guarded(&cfg) {
            return Self::trivial_guarded(name, cfg);
        }
        let ssa = build_ssa(&cfg, registry);
        let def_use = build_def_use_chains(&ssa, Some(&cfg));
        // The registry carries its dialect profile's fold policy: the octal
        // rule, which fixes how a bare leading-zero literal (`08`, `010`) is
        // read when SCCP folds `==`/`!=` — octal in the 8.x runtimes (tcl8.x /
        // F5 / EDA), decimal in the 9.x runtimes (tcl9.0/9.1 and bpf), and
        // `None` (abstain) when there is no Tcl runtime to have an opinion
        // (f5-bigip, an unknown dialect) — plus whether the dialect's `expr`
        // grammar carries the iRules word operators, so `if {$x contains
        // "cd"}` folds under `f5-irules`. A hand-assembled registry without a
        // profile keeps the historical loaded-packs octal derivation.
        // Dynamic-name facts (issue #923 audit cluster C10): a `set $var v`
        // means any name may be defined, an `unset $n` that any name may have
        // stopped existing — so the existence fold below must abstain in that
        // direction rather than hand the optimiser a wrong constant branch.
        //
        // Computed *before* SCCP because a dynamic write / destroy blinds the
        // value lattice too (issue #1374): after `set $name v` any variable in
        // the frame may hold any value, so no definition is a trustworthy
        // constant. Reuse the "every variable is externally mutable" switch a
        // dynamic trace target already throws — same lattice consequence, one
        // chokepoint — so O100 / O101 / branch folds never propagate a value
        // across the barrier. A dynamic *read* only observes, so it leaves
        // the value lattice alone.
        // Split the `[…]` texts this walk re-reads under the same dialect the
        // lowering used, so the barrier and the IR agree on word boundaries —
        // `registry` is the document's own profile-built registry here, and
        // its profile is what `LexerConfig::for_dialect` reads (issue #1393).
        let dynamic_names = crate::dynamic_names::dynamic_name_barrier(
            &cfg,
            registry,
            crate::dynamic_names::lexer_config_for(registry),
        );
        let mut sccp = sccp_with_extra_escaping(
            &cfg,
            &ssa,
            param_constants,
            crate::tcl_expr_eval::FoldPolicy::from_registry(registry),
            extra_global_escaping,
            crate::sccp::TraceInputs {
                registry,
                traced_variables: trace_facts.traced_variables,
                has_dynamic_variable_trace: trace_facts.has_dynamic_variable_trace
                    || dynamic_names.writes
                    || dynamic_names.destroys,
            },
        );
        // Surface `[info exists X]` / `[array exists X]`
        // folds (parameter → exists, never-defined non-param → absent)
        // as constant branches so the optimiser's O101 fold / DCE sees
        // them. The analyser's I230 uses the same fold via
        // `existence_constant_branches`; the SCCP pass proper has no
        // parameter/existence facts to fold them itself.  A method body's
        // instance variables are handed over too, so the fold abstains on
        // object state instead of calling it absent (issue #1129).
        sccp.constant_branches
            .extend(crate::sccp::existence_constant_branches(
                &cfg,
                crate::sccp::ExistenceFrame {
                    params,
                    object_state,
                },
                registry,
                dynamic_names,
            ));
        let types = propagate_types(
            &cfg,
            &ssa,
            &sccp,
            registry,
            known_classes,
            extra_global_escaping,
            trace_facts,
        );
        let return_type = crate::type_infer::infer_function_return_type(
            &cfg,
            &sccp,
            &types,
            registry,
            known_classes,
            &ssa,
        );
        let rendered_props = propagate_rendered_props(&cfg, &ssa, &sccp, registry);
        let taints = propagate_taints(
            &TaintGraph::new(&cfg, &ssa, &sccp),
            registry,
            Some(&rendered_props),
            None,
            None,
            None,
            None,
        );
        Self {
            name: name.into(),
            cfg,
            ssa,
            def_use: Arc::new(def_use),
            sccp,
            types: Arc::new(types),
            return_type,
            taints: Arc::new(taints),
            rendered_props: Arc::new(rendered_props),
            memory_ssa: None,
            dynamic_names,
            complexity_guarded: false,
            base_offset: 0,
            method_facts: None,
            semantic_facts: SemanticAnalysisBundle::unavailable(DialectSet::empty()),
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
            name: name.into(),
            cfg,
            ssa,
            def_use: Arc::default(),
            sccp: SccpResult::default(),
            types: Arc::default(),
            return_type: TypeLattice::unknown(),
            taints: Arc::default(),
            rendered_props: Arc::default(),
            memory_ssa: None,
            // A guarded unit's lattices are all trivial and every per-proc
            // pass skips it, so the barrier stays clear (never consulted).
            dynamic_names: crate::dynamic_names::DynamicNameBarrier::default(),
            complexity_guarded: true,
            base_offset: 0,
            method_facts: None,
            semantic_facts: SemanticAnalysisBundle::unavailable(DialectSet::empty()),
        }
    }

    /// Whether this function's [`Self::dynamic_names`] barrier forbids any
    /// pass that *moves, folds, or deletes* a variable's value across other
    /// statements (issue #1374).
    ///
    /// A computed variable name (`set $name v`, `unset $n`, `[set $v]`)
    /// can create, destroy, or observe **any** variable in the frame, so no
    /// "sole reaching definition", "write-only chain", or "not referenced
    /// later" fact derived from spelled names is trustworthy anywhere in the
    /// function.  Consulted by the value-motion passes — O102 load
    /// forwarding, O104 / O130 chain folding, O119 multi-`set` packing, and
    /// O125 code sinking — which abstain for the whole function when it
    /// answers `true`, matching the abstention O109 / O126 elimination and
    /// SCCP's existence fold already apply.
    #[must_use]
    pub const fn dynamic_barrier_blocks_value_motion(&self) -> bool {
        !self.dynamic_names.is_clear()
    }

    /// Recover the absolute span of `span` (a span carried by this unit's
    /// `cfg`/`ssa`/`sccp`) by adding [`Self::base_offset`].  A no-op when the
    /// unit was built at its real position (`base_offset == 0`); the
    /// offset-aware seam for the memoised (offset-0) path.
    #[must_use]
    pub fn abs_span(&self, span: tcl_lexer::Span) -> tcl_lexer::Span {
        if self.base_offset == 0 {
            return span;
        }
        let s = (i64::from(span.start()) + self.base_offset).max(0);
        let e = (i64::from(span.end()) + self.base_offset).max(0);
        // `s`/`e` are clamped to `>= 0`; a value past `u32::MAX` is a degenerate
        // out-of-range offset, so saturate rather than wrap.
        let s = u32::try_from(s).unwrap_or(u32::MAX);
        let e = u32::try_from(e).unwrap_or(u32::MAX);
        tcl_lexer::Span::new(s, e)
    }

    /// Recover the absolute byte position of `pos` by adding
    /// [`Self::base_offset`] (see [`Self::abs_span`]).
    #[must_use]
    pub fn abs_pos(&self, pos: u32) -> u32 {
        if self.base_offset == 0 {
            return pos;
        }
        // Clamped to `>= 0`; saturate a degenerate out-of-range offset.
        u32::try_from((i64::from(pos) + self.base_offset).max(0)).unwrap_or(u32::MAX)
    }

    /// Populate memory-SSA on demand under `dialect`. Returns `self` for
    /// chaining.
    #[must_use]
    pub fn with_memory_ssa(
        mut self,
        registry: &tcl_registry::CommandRegistry,
        dialect: DialectSet,
    ) -> Self {
        self.memory_ssa = Some(build_memory_ssa(&self.ssa, registry, dialect));
        self
    }

    /// Attach semantic facts for a compilation unit's top-level script.
    ///
    /// The top-level script is modelled as evaluated in a fresh interpreter
    /// whose dispatch state is the registry baseline; every other unit kind
    /// runs after arbitrary interposed history and must use
    /// [`Self::with_semantic_analysis`] with an unknown-world entry contract.
    #[must_use]
    pub fn with_top_level_semantic_analysis(
        self,
        registry: &CommandRegistry,
        dialect: DialectSet,
        script: &crate::ir::Script,
    ) -> Self {
        self.with_semantic_analysis(
            registry,
            dialect,
            Some(script),
            crate::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
        )
    }

    /// Attach source-faithful, target-neutral semantic facts under an
    /// explicitly selected dialect and dispatch entry contract.
    ///
    /// A missing source script remains an explicit unavailable state. A
    /// structured source shape outside the executable compatibility subset is
    /// retained as a typed decline, never converted into guessed facts.
    #[must_use]
    pub fn with_semantic_analysis(
        mut self,
        registry: &CommandRegistry,
        dialect: DialectSet,
        script: Option<&crate::ir::Script>,
        entry_assumption: crate::dispatch_proof::DispatchEntryAssumption,
    ) -> Self {
        self.semantic_facts = script.map_or_else(
            || SemanticAnalysisBundle::unavailable(dialect),
            |script| {
                SemanticAnalysisBundle::build_for_interactive_analysis(
                    registry,
                    dialect,
                    script,
                    entry_assumption,
                )
            },
        );
        self
    }

    /// Re-run interprocedural taint propagation for this unit against `ia`,
    /// returning the taint lattice (it does **not** mutate `self.taints`).
    ///
    /// The taint lattice is keyed by SSA [`ValueKey`] (variable + version) and is
    /// **span-free**, so the result is offset-independent: an offset-0 baseline
    /// unit yields byte-identical taints to its rebased counterpart.  This is the
    /// property the salsa-native `taint_cascade` memo relies on — it propagates
    /// over the offset-0 baseline and installs the result into the rebased unit.
    #[must_use]
    pub fn interproc_taints(
        &self,
        registry: &CommandRegistry,
        ia: &InterproceduralAnalysis,
        dialect: Option<&str>,
    ) -> HashMap<ValueKey, TaintLattice> {
        propagate_taints(
            &TaintGraph::new(&self.cfg, &self.ssa, &self.sccp),
            registry,
            Some(&self.rendered_props),
            Some(ia),
            dialect,
            None,
            None,
        )
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
#[derive(Debug, Clone, PartialEq)]
pub struct CompilationUnit {
    /// Source text (kept so downstream passes that need raw
    /// lexing can re-scan ranges without reparsing).
    pub source: String,
    /// IR module produced by lowering.
    pub ir_module: IrModule,
    /// Module-level CFG.
    pub cfg_module: CfgModule,
    /// Top-level script analysis.
    pub top_level: FunctionUnit,
    /// Per-procedure analyses keyed by qualified name.
    pub procedures: HashMap<String, FunctionUnit>,
    /// `TclOO` method bodies lowered to per-method [`FunctionUnit`]s.
    /// Keyed by `{class_qname}::{method_name}`; empty for
    /// non-OO sources. Kept separate from [`Self::procedures`] so the
    /// per-proc diagnostic passes are unaffected — only the optimiser's
    /// O126 gate iterates these.
    pub methods: HashMap<String, FunctionUnit>,
    /// Synthetic *body units* — the bodies of `apply` lambdas and
    /// `namespace eval` blocks, lowered to [`FunctionUnit`]s so the
    /// static-analysis pipeline reaches inside them (see
    /// [`crate::ir::Module::body_units`]). Keyed by a synthetic qualified name
    /// (`::apply#N`, or `::NS::namespace-eval#N` — `NS` *prefixes* the marker
    /// so the qname's enclosing namespace, the same "everything before the
    /// last `::`" convention every proc/method qname uses, is the namespace
    /// the block actually targets). Kept **separate** from both
    /// [`Self::procedures`] and [`Self::methods`] so no existing consumer —
    /// codegen, the optimiser/minifier, the per-proc or OO diagnostic passes —
    /// changes behaviour; only analyses that explicitly opt in (via
    /// [`Self::body_function_units`]) read them. Empty for the overwhelmingly
    /// common source with no `apply`/`namespace eval` body.
    pub body_units: HashMap<String, FunctionUnit>,
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
}

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
    /// The seed each procedure was actually built under, keyed by qualified
    /// name and encoded as the same sorted `(param, version, literal)`
    /// triples the lattice memo key interns.  Only procedures that received
    /// a seed appear.  Sorted so the explorer's rendering is deterministic.
    pub param_constants_by_proc: BTreeMap<String, Vec<(String, u32, String)>>,
}

/// Whole-module facts every per-function build in a [`CompilationUnit`] build
/// consumes, gathered once.
struct ModuleWideFacts {
    /// Fully-qualified names of every class defined in the unit.
    known_class_set: HashSet<String>,
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

impl ModuleWideFacts {
    fn collect(source: &str, ir_module: &IrModule, registry: &CommandRegistry) -> Self {
        // Classes come from the signature scanner so this build and the
        // incremental/db build derive an identical set (⇒ identical
        // OBJECT-constructor typing on both paths).
        let known_class_set = collect_known_classes(source, registry);
        let mut known_classes: Vec<String> = known_class_set.iter().cloned().collect();
        known_classes.sort_unstable();
        Self {
            known_class_set,
            known_classes,
            // A top-level bare name already lives in the global frame, so a
            // call to a procedure that declares it `global` can reassign it
            // mid-run even though the top-level body never says `global`
            // itself. See `crate::var_observability::scan_module_global_names`.
            top_level_extra_escaping: crate::var_observability::scan_module_global_names(ir_module),
            traced_variable_names: ir_module.traced_variables.iter().cloned().collect(),
        }
    }
}

/// Lower `source` to IR, run the module-level pre-CFG passes, and build the
/// module CFG — the front half of a [`CompilationUnit`] build, shared by every
/// entry point.
fn lower_and_build_cfg(
    source: &str,
    options: UnitBuildOptions<'_>,
    body_cache: Option<&BodyLoweringCache<'_>>,
) -> (IrModule, CfgModule) {
    let registry = options.registry;
    let mut ir_module = match body_cache {
        Some(bc) => crate::lowering::lower_to_ir_with_body_cache(
            source,
            registry,
            options.config,
            options.dialect,
            bc,
        ),
        None => crate::lowering::lower_to_ir_with_dialect(
            source,
            registry,
            options.config,
            options.dialect,
        ),
    };
    // Specialise Option-shape factories before any other
    // module-level passes so the synthesised child procs
    // appear in module.procedures for the inline_uplevel pass
    // and CFG construction.
    crate::specialise_factories::specialise_factories(&mut ir_module, registry);
    // Run the inline_uplevel pass before CFG construction so
    // every passthrough callsite is replaced with a Statement::Block
    // that splices the body inline.
    crate::inline_uplevel::inline_uplevel_passthrough(&mut ir_module, registry);
    let cfg_module = build_cfg(&ir_module, options.defer_top_level);
    (ir_module, cfg_module)
}

/// Resolve everything the interprocedural constant seed needs to know about
/// *who can call this unit's procedures* (see [`crate::unit_scope`]):
/// the merged call-site evidence, the whole-module command-binding trust
/// lattice, and the registry-declared unit boundaries the file crosses.
///
/// Returned as a tuple rather than a ready-made
/// [`crate::unit_scope::UnitCallerView`] because that view *borrows* the
/// mutations lattice, which is produced here — the caller owns both and pairs
/// them up.
fn resolve_unit_scope(
    ir_module: &IrModule,
    cfg_module: &CfgModule,
    cfg_context: Option<&CfgContext>,
    options: UnitBuildOptions<'_>,
) -> (
    crate::unit_scope::CallSiteEvidence,
    crate::command_binding::ModuleCommandMutations,
    tcl_registry::Traits,
) {
    let registry = options.registry;
    // Bare CFGs for every TclOO method and synthetic body unit (`apply`
    // lambda, `namespace eval` body) — the call-site scan must walk these as
    // *callers* too, even though neither is itself seeded with
    // `param_constants` (`build_method_units`/`build_body_units` always pass
    // `None`): a call from inside one of these bodies to an ordinary user proc
    // is a real call site whose argument can vary between call sites, exactly
    // like a bare top-level/proc-body call.
    let extra_callers = build_extra_call_site_scan_contexts(ir_module, cfg_context);
    // Collect call-site literal arg values per user proc so each callee's SCCP
    // can fold a param every caller passes the same literal for
    // (interprocedural constant propagation).
    let mut call_sites = collect_call_site_constants(
        cfg_module,
        &extra_callers,
        &ir_module.procedures,
        &ir_module.namespace_imports,
        registry,
        options.dialect,
    );
    // Fold in the call sites a host with a cross-file view supplied — callers
    // in *other* files, which this single-source unit can never see for itself
    // (issue #977). Merging is monotone: extra evidence can retract a fold,
    // never manufacture one.
    if let Some(external) = options.external_call_sites {
        call_sites.merge_from(external);
    }
    // Whole-module `rename` / `interp alias` / dynamic-redefinition trust
    // fact — reused (not duplicated) from the optimiser's identical O103
    // proc-call-fold trust gate, so the interprocedural param-constant seed
    // never trusts a callee whose binding could have moved.
    let command_mutations =
        crate::command_binding::scan_module_command_mutations(ir_module, registry);
    // Which registry-declared unit boundaries this file crosses — the generic
    // replacement for the old hardcoded `package provide` check.
    let linkage = crate::unit_scope::scan_unit_linkage(ir_module, registry, options.dialect);
    (call_sites, command_mutations, linkage)
}

/// Module-wide, read-only inputs [`build_procedure_units`] shares across
/// every procedure in the loop.  Grouped into one struct so the extracted
/// helper takes two parameters instead of a dozen.
struct ProcedureBuildContext<'a> {
    ir_module: &'a IrModule,
    cfg_module: &'a CfgModule,
    cfg_context: Option<&'a CfgContext>,
    registry: &'a CommandRegistry,
    dialect: &'a str,
    /// Merged in-unit + cross-file call-site evidence
    /// ([`crate::unit_scope::CallSiteEvidence`]).
    call_sites: &'a crate::unit_scope::CallSiteEvidence,
    /// Who else can call these procedures
    /// ([`crate::unit_scope::UnitCallerView`]).
    caller_view: &'a crate::unit_scope::UnitCallerView<'a>,
    known_class_set: &'a HashSet<String>,
    known_classes: &'a [String],
    traced_variable_names: &'a [String],
    trace_facts: ModuleTraceFacts<'a>,
    procedure_entry: crate::dispatch_proof::DispatchEntryAssumption,
}

/// Build one [`FunctionUnit`] per procedure: seed its SCCP with the
/// caller-uniform literals [`crate::unit_scope`] proved, route the build
/// through the per-procedure lattice memo when one is available, and skip
/// both for an oversized body.
fn build_procedure_units(
    ctx: &ProcedureBuildContext<'_>,
    mut cache: Option<&mut ProcLatticeCache<'_>>,
) -> BuiltProcedureUnits {
    let mut procedures: HashMap<String, FunctionUnit> = HashMap::new();
    let mut param_constants_by_proc: BTreeMap<String, Vec<(String, u32, String)>> = BTreeMap::new();
    for (qname, cfg) in &ctx.cfg_module.procedures {
        let params = ctx
            .ir_module
            .procedures
            .get(qname)
            .map_or(&[][..], |p| p.params.as_slice());
        let param_constants =
            params_constants_from_call_sites(params, ctx.call_sites, qname, ctx.caller_view);
        let proc = ctx.ir_module.procedures.get(qname);
        // Complexity guard (block-count or body-byte half): skip both the
        // memo and the deep analysis for an oversized body. A flat
        // generated proc is block-light yet byte-huge, so the byte test is
        // what catches it.
        let body_bytes = proc.map_or(0usize, |p| {
            p.span.end().saturating_sub(p.span.start()) as usize
        });
        if body_bytes > crate::ssa::DEEP_ANALYSIS_BODY_BYTES
            || crate::ssa::is_complexity_guarded(cfg)
        {
            procedures.insert(
                qname.clone(),
                FunctionUnit::trivial_guarded(qname, cfg.clone()),
            );
            continue;
        }
        let body_offset = proc.map_or(0, |p| p.span.start());
        // Encode the interprocedural seeds into the hashable form the memo
        // key carries.  The interproc `param_constants` are folded *into*
        // the key (not a fresh-build escape hatch as before), so a procedure
        // with caller-uniform literals still engages the memo — its key
        // simply also distinguishes the seeds, so it rebuilds iff a caller's
        // literal at that position changes.  `None` means a seed shape we
        // can't intern (defensive; the current producer only emits string
        // consts) → build fresh.
        let encoded_pc = encode_param_constants(param_constants.as_ref());
        // Keep the seed for the explorer's interprocedural view: it is the
        // one fact that explains why a condition on a parameter folded —
        // and, by its absence, that an indirect or cross-file call site
        // withdrew it.  The non-empty filter keeps the map to procedures
        // that were actually seeded.
        if let Some(encoded) = encoded_pc.as_ref().filter(|e| !e.is_empty()) {
            param_constants_by_proc.insert(qname.clone(), encoded.clone());
        }
        // Route through the memo only when (a) a cache is present, (b) the
        // procedure has a real body, (c) the module context is available,
        // and (d) the seeds encode into the hashable key form.
        let memoised = match (cache.as_mut(), proc, ctx.cfg_context, encoded_pc) {
            (
                Some(memo),
                Some(proc),
                Some((upvar_procs, proc_params, global_write_procs)),
                Some(encoded_pc),
            ) => {
                // Normalise the body to offset 0 so a shifted-but-unchanged
                // procedure produces an identical request (memo hit).
                let mut body = proc.body.clone();
                crate::lattice_rebase::rebase_script(&mut body, -i64::from(body_offset));
                let mut fu = memo(&LatticeRequest {
                    qname,
                    body: &body,
                    params,
                    upvar_procs,
                    proc_params,
                    global_write_procs,
                    dialect: ctx.dialect,
                    param_constants: &encoded_pc,
                    known_classes: ctx.known_classes,
                    traced_variables: ctx.traced_variable_names,
                    has_dynamic_variable_trace: ctx.ir_module.has_dynamic_variable_trace,
                });
                // Rebase the offset-0 memo result to the procedure's real
                // position so every consumer sees **absolute** spans without
                // needing offset-awareness — robust against new/upstream
                // diagnostic & optimiser passes that read `fu.cfg` spans
                // directly (`base_offset` stays 0; `abs_span` is identity).
                crate::lattice_rebase::rebase_function_unit(&mut fu, i64::from(body_offset));
                Some(fu)
            }
            _ => None,
        };
        let mut fu = memoised.unwrap_or_else(|| {
            FunctionUnit::build_with_param_constants_and_classes(
                qname,
                cfg.clone(),
                params,
                ctx.registry,
                param_constants.as_ref(),
                ctx.known_class_set,
                ctx.trace_facts,
            )
        });
        // A memoised unit carries an offset-0 executable sidecar. Rebuild the
        // source-bearing portion after the normal CFG/SSA rebase so retained
        // word and provenance spans match this procedure's real source
        // position. Structural node identities and registry facts remain
        // position-independent; only source coordinates are refreshed.
        fu = fu.with_semantic_analysis(
            ctx.registry,
            DialectSet::parse(ctx.dialect).unwrap_or_else(DialectSet::empty),
            proc.map(|procedure| &procedure.body),
            ctx.procedure_entry,
        );
        procedures.insert(qname.clone(), fu);
    }
    BuiltProcedureUnits {
        procedures,
        param_constants_by_proc,
    }
}

/// [`build_procedure_units`]'s two outputs: the units themselves, and the
/// interprocedural seed each was built under (explorer provenance only —
/// the pipeline consumes the seed where it is produced).
struct BuiltProcedureUnits {
    procedures: HashMap<String, FunctionUnit>,
    param_constants_by_proc: BTreeMap<String, Vec<(String, u32, String)>>,
}

impl CompilationUnit {
    /// Build a [`CompilationUnit`] by running the pipeline
    /// end-to-end: `lower_to_ir` → `build_cfg` → per-function
    /// SSA / def-use / SCCP.
    ///
    /// `defer_top_level = false` gives analyses the fully-inlined
    /// CFG; passing `true` matches the codegen behaviour where
    /// top-level `foreach` / `catch` / `try` are compiled as
    /// opaque calls.
    ///
    /// **Dialect-blind**: lowers with the default (Tcl-8.5+) lexer config and
    /// an empty [`UnitBuildOptions::dialect`], so word tokenisation, the
    /// expression grammar, and the fold policy are all plain Tcl.  Use it only
    /// where the document genuinely has no dialect (tests, dialect-agnostic
    /// helpers); a caller that knows the dialect must use
    /// [`Self::build_for_dialect`], or an iRules word operator will neither
    /// fold nor draw a constant-condition diagnostic.
    #[must_use]
    pub fn build_for(source: &str, registry: &CommandRegistry, defer_top_level: bool) -> Self {
        Self::build_for_with_config(
            source,
            registry,
            defer_top_level,
            tcl_lexer::LexerConfig::default(),
        )
    }

    /// Like [`Self::build_for`] but lowers with an explicit dialect
    /// [`tcl_lexer::LexerConfig`] so `{*}` / `}{` tokenisation follows the
    /// document's dialect.
    ///
    /// The *analysis* dialect is still empty — this only fixes word
    /// tokenisation.  Prefer [`Self::build_for_dialect`], which derives the
    /// same config from the dialect name and additionally gives the lowering
    /// and the lattice pipeline that dialect's expression grammar and fold
    /// policy.
    #[must_use]
    pub fn build_for_with_config(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        Self::build_with(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level,
                config,
                dialect: "",
                external_call_sites: None,
            },
            None,
            None,
            crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
        )
    }

    /// Build for a document whose dialect is known — the entry point every
    /// production consumer holding a dialect string should use.
    ///
    /// Derives the lexer config from `dialect`
    /// ([`tcl_lexer::LexerConfig::for_dialect`]) *and* records the dialect in
    /// [`UnitBuildOptions::dialect`], so all three dialect-sensitive layers
    /// agree: word tokenisation, the expression grammar the lowering parses
    /// conditions with, and the fold policy the lattice pipeline runs under.
    /// Pass `""` for plain Tcl.
    ///
    /// `registry` must be the registry for the same dialect
    /// ([`tcl_registry::registry_for_dialect`]).
    #[must_use]
    pub fn build_for_dialect(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        dialect: &str,
    ) -> Self {
        Self::build_with(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level,
                config: tcl_lexer::LexerConfig::for_dialect(dialect),
                dialect,
                external_call_sites: None,
            },
            None,
            None,
            crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
        )
    }

    /// Build under an explicit [`UnitBuildOptions`] — the entry point a host
    /// with a cross-file view uses to supply
    /// [`UnitBuildOptions::external_call_sites`].
    #[must_use]
    pub fn build_with_options(source: &str, options: UnitBuildOptions<'_>) -> Self {
        Self::build_with(
            source,
            options,
            None,
            None,
            crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
        )
    }

    /// Build under an explicit sealed-load contract for procedure entry.
    ///
    /// This is the only production-facing path that may narrow procedure
    /// bodies from `UnknownWorld`. The caller must own the complete load graph
    /// and sealed interpreter asserted by `entry`; ordinary hosted/LSP builds
    /// use [`Self::build_with_options`] and continue to abstain.
    #[must_use]
    pub fn build_with_sealed_procedure_entry(
        source: &str,
        options: UnitBuildOptions<'_>,
        entry: crate::dispatch_proof::SealedLoadGraphEntry,
    ) -> Self {
        Self::build_with(
            source,
            options,
            None,
            None,
            crate::dispatch_proof::DispatchEntryAssumption::SealedLoadGraph(entry),
        )
    }

    /// Like [`Self::build_for_with_config`] but routes each procedure's
    /// per-function lattice build through `cache`, a salsa-native memo (see
    /// [`ProcLatticeCache`] / [`LatticeRequest`]).
    ///
    /// Each procedure's body is normalised to offset 0 and handed to `cache`,
    /// which returns the (possibly memoised) offset-0 [`FunctionUnit`]; the
    /// builder then rebases it to the procedure's real position.  A
    /// shifted-but-unchanged body produces an identical request, so it is a
    /// memo hit (and reused, rebased).  The result is byte-identical to
    /// [`Self::build_for_with_config`]; only the redundant SSA/SCCP/type/
    /// rendered recompute for an unchanged procedure body is skipped.
    /// Interprocedural `param_constants` (caller-uniform-literal SCCP seeds) are
    /// folded into the memo key, so a procedure with them still memoises (it
    /// rebuilds only when a caller's literal at that position changes).  The top
    /// level and methods are always built fresh (no stable offset-0 key); the
    /// cross-function interproc taint re-run still runs over the whole unit in
    /// [`Self::with_interprocedural`].
    pub fn build_for_memoized(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: &mut ProcLatticeCache<'_>,
    ) -> Self {
        Self::build_with(
            source,
            options,
            Some(cache),
            None,
            crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
        )
    }

    /// Like [`Self::build_for_memoized`] but also threads a memoised per-procedure
    /// **body-lowering** callback (SRV-INCREMENTAL Task 3) into the lowering phase,
    /// so an unchanged top-level proc's body IR is reused across edits.  The caller
    /// installs it only for context-free files (see [`crate::lowering::Lowerer`]'s
    /// `body_cache`); byte-identity is guarded by the corpus differential gates.
    pub fn build_for_memoized_with_body_cache(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: &mut ProcLatticeCache<'_>,
        body_cache: &BodyLoweringCache<'_>,
    ) -> Self {
        Self::build_with(
            source,
            options,
            Some(cache),
            Some(body_cache),
            crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
        )
    }

    fn build_with(
        source: &str,
        options: UnitBuildOptions<'_>,
        cache: Option<&mut ProcLatticeCache<'_>>,
        body_cache: Option<&BodyLoweringCache<'_>>,
        procedure_entry: crate::dispatch_proof::DispatchEntryAssumption,
    ) -> Self {
        let UnitBuildOptions {
            registry,
            dialect,
            external_call_sites,
            ..
        } = options;
        let (ir_module, cfg_module) = lower_and_build_cfg(source, options, body_cache);
        // Module-wide upvar/param context — the CFG-determining context a
        // procedure body is rebuilt under.  Computed once and shared by every
        // memoised request, the methods/body-units below, and the call-site
        // scan's extra caller contexts, so the offset-0 CFG the memo rebuilds
        // is identical to this whole-module build's.  Only needed on the
        // memoised path or when the call-site scan has extra caller contexts
        // to build (methods, body units, `uplevel #0` bodies).
        let cfg_context = (cache.is_some()
            || crate::unit_scope::needs_extra_call_site_scan_contexts(&ir_module))
        .then(|| crate::cfg_builder::prepare_cfg_context(&ir_module));
        let (call_site_constants, command_mutations, linkage) =
            resolve_unit_scope(&ir_module, &cfg_module, cfg_context.as_ref(), options);
        let caller_view = crate::unit_scope::UnitCallerView {
            linkage,
            has_cross_file_evidence: external_call_sites.is_some(),
            command_mutations: &command_mutations,
        };
        let ModuleWideFacts {
            known_class_set,
            known_classes,
            top_level_extra_escaping,
            traced_variable_names,
        } = ModuleWideFacts::collect(source, &ir_module, registry);
        // Whole-module variable-trace fact — computed once by lowering
        // and stored on `ir_module`, so every per-function build below is a
        // cheap reference pass-through, not a recomputation.
        let trace_facts = ModuleTraceFacts {
            traced_variables: &ir_module.traced_variables,
            has_dynamic_variable_trace: ir_module.has_dynamic_variable_trace,
        };
        let semantic_dialect = DialectSet::parse(dialect).unwrap_or_else(DialectSet::empty);
        let top_level = FunctionUnit::build_top_level(
            cfg_module.top_level.clone(),
            registry,
            &known_class_set,
            &top_level_extra_escaping,
            trace_facts,
        )
        .with_top_level_semantic_analysis(registry, semantic_dialect, &ir_module.top_level);
        let built = build_procedure_units(
            &ProcedureBuildContext {
                ir_module: &ir_module,
                cfg_module: &cfg_module,
                cfg_context: cfg_context.as_ref(),
                registry,
                dialect,
                call_sites: &call_site_constants,
                caller_view: &caller_view,
                known_class_set: &known_class_set,
                known_classes: &known_classes,
                traced_variable_names: &traced_variable_names,
                trace_facts,
                procedure_entry,
            },
            cache,
        );
        let mut procedures = built.procedures;
        let methods = Self::build_method_units(
            &ir_module,
            cfg_context.as_ref(),
            &known_class_set,
            registry,
            trace_facts,
            semantic_dialect,
        );
        let body_units = Self::build_body_units(
            &ir_module,
            cfg_context.as_ref(),
            &known_class_set,
            registry,
            trace_facts,
            semantic_dialect,
        );
        // Build the cross-event scope from the
        // ``::when::*`` subset of procedures.  ``None`` when no
        // ``when`` block is present so non-iRules consumers
        // skip the (empty) sweep.
        let connection_scope = {
            let when_procs: HashMap<String, FunctionUnit> = procedures
                .iter()
                .filter(|(qn, _)| qn.starts_with("::when::"))
                .map(|(qn, fu)| (qn.clone(), fu.clone()))
                .collect();
            if when_procs.is_empty() {
                None
            } else {
                Some(crate::connection_scope::build_connection_scope(&when_procs))
            }
        };
        Self::drop_cross_event_existence_folds(&mut procedures, connection_scope.as_ref());
        Self {
            source: source.to_owned(),
            ir_module,
            cfg_module,
            top_level,
            procedures,
            methods,
            body_units,
            interproc: None,
            connection_scope,
            caller_scope: UnitCallerScope {
                linkage: caller_view.linkage,
                has_cross_file_evidence: caller_view.has_cross_file_evidence,
                call_sites: call_site_constants,
                param_constants_by_proc: built.param_constants_by_proc,
            },
        }
    }

    /// Lower `TclOO` method bodies (populated in `ir_module.methods` by
    /// lowering) to per-method [`FunctionUnit`]s, using the same CFG → SSA →
    /// analysis pipeline as procs.  Kept in a separate map so the per-proc
    /// diagnostic passes (which iterate `procedures`) are unaffected — only the
    /// interproc purity summary and the O126 optimiser gate consume methods.
    /// Returns an empty map for non-OO sources (skipping the upvar-context scan).
    fn build_method_units(
        ir_module: &IrModule,
        cfg_context: Option<&CfgContext>,
        known_class_set: &HashSet<String>,
        registry: &CommandRegistry,
        trace_facts: ModuleTraceFacts<'_>,
        semantic_dialect: DialectSet,
    ) -> HashMap<String, FunctionUnit> {
        if ir_module.methods.is_empty() {
            return HashMap::new();
        }
        let (upvar_procs, proc_params, global_write_procs) =
            cfg_context.expect("cfg_context computed when methods are present");
        // Issue #1177: a callee reached via `my` / `next` is a method, never
        // in the upvar-procs table (the dispatch does not name its target),
        // so its `upvar 1 $refvar …` caller-frame definition was invisible
        // and every defs-based check treated the dispatch as a no-op — a
        // false always-false I230 on `info exists` of an upvar-defined local
        // (oracle, tclsh 9.0.4 / 8.6.14: after `my Reference? $lookup ref`
        // returns true, `[info exists ref]` in the caller is 1).  Widen the
        // dispatch sites of exactly the methods whose reachable dispatch
        // surface meets a caller-frame-reaching (or unanalysable) class —
        // the same per-method barrier the optimiser's propagation gate
        // consumes, so the two stay one evidence rule — through synthetic
        // frame-effect entries fed to the per-call-site machinery procs
        // already use.
        let dispatch_barrier = crate::optimiser::method_barrier::compute(ir_module, registry);
        let widened_upvar_procs = {
            let mut map = upvar_procs.clone();
            map.extend(crate::cfg_builder::upvar_info::oo_dispatch_widening_entries(registry));
            map
        };
        ir_module
            .methods
            .iter()
            .map(|(mqname, method)| {
                let method_upvar_procs = if dispatch_barrier.allows_locals(mqname) {
                    upvar_procs
                } else {
                    &widened_upvar_procs
                };
                let cfg = crate::cfg_builder::build_cfg_function_with_upvars(
                    mqname,
                    &method.body,
                    true,
                    method_upvar_procs.clone(),
                    proc_params.clone(),
                    global_write_procs.clone(),
                );
                // Body-byte half of the complexity guard (the block-count
                // half is applied inside `build`); skip an oversized
                // generated method body the same way as a procedure.
                let body_bytes = method
                    .span
                    .map_or(0usize, |s| s.end().saturating_sub(s.start()) as usize);
                let fu = if body_bytes > crate::ssa::DEEP_ANALYSIS_BODY_BYTES {
                    // The guarded unit still carries its method facts: the
                    // deep lattices are skipped, but every consumer of "which
                    // names are bound in this method's frame" must read the
                    // same carrier as the deep path (issue #1174).
                    let mut fu = FunctionUnit::trivial_guarded(mqname, cfg);
                    fu.method_facts = Some(Arc::new(MethodBodyFacts::from_method(method)));
                    fu
                } else {
                    FunctionUnit::build_for_method(
                        mqname,
                        cfg,
                        method,
                        registry,
                        known_class_set,
                        trace_facts,
                    )
                }
                .with_semantic_analysis(
                    registry,
                    semantic_dialect,
                    Some(&method.body),
                    crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
                );
                (mqname.clone(), fu)
            })
            .collect()
    }

    /// Lower the synthetic *body units* (`apply` lambdas and `namespace eval`
    /// bodies, populated in [`crate::ir::Module::body_units`] by lowering) to
    /// per-body [`FunctionUnit`]s, using the same CFG → SSA → analysis pipeline
    /// as procs and methods.
    ///
    /// Each body runs in a *fresh* frame (its own locals), exactly like a proc
    /// or method body — so the CFG is built with `is_proc = true` and the
    /// module upvar/param context, matching [`Self::build_method_units`]. Kept
    /// in a separate map so no existing consumer (codegen, optimiser, per-proc
    /// diagnostics) is affected; only [`Self::body_function_units`] surfaces
    /// them. Returns an empty map when the module has no body units (the common
    /// case), skipping all work.
    fn build_body_units(
        ir_module: &IrModule,
        cfg_context: Option<&CfgContext>,
        known_class_set: &HashSet<String>,
        registry: &CommandRegistry,
        trace_facts: ModuleTraceFacts<'_>,
        semantic_dialect: DialectSet,
    ) -> HashMap<String, FunctionUnit> {
        if ir_module.body_units.is_empty() {
            return HashMap::new();
        }
        let (upvar_procs, proc_params, global_write_procs) =
            cfg_context.expect("cfg_context computed when body units are present");
        ir_module
            .body_units
            .iter()
            .map(|(qname, proc)| {
                let cfg = crate::cfg_builder::build_cfg_function_with_upvars(
                    qname,
                    &proc.body,
                    true,
                    upvar_procs.clone(),
                    proc_params.clone(),
                    global_write_procs.clone(),
                );
                // Same body-byte complexity guard as procs/methods: a huge
                // generated lambda body contributes trivial lattices instead of
                // a deep (and slow) analysis.
                let body_bytes = proc.span.end().saturating_sub(proc.span.start()) as usize;
                let fu = if body_bytes > crate::ssa::DEEP_ANALYSIS_BODY_BYTES {
                    FunctionUnit::trivial_guarded(qname, cfg)
                } else {
                    FunctionUnit::build_with_param_constants_and_classes(
                        qname,
                        cfg,
                        &proc.params,
                        registry,
                        None,
                        known_class_set,
                        trace_facts,
                    )
                }
                .with_semantic_analysis(
                    registry,
                    semantic_dialect,
                    Some(&proc.body),
                    crate::dispatch_proof::DispatchEntryAssumption::UnknownWorld,
                );
                (qname.clone(), fu)
            })
            .collect()
    }

    /// Cross-event existence post-pass: `existence_constant_branches` ran per
    /// function (before the connection scope existed) and folded
    /// `[info exists VAR]` → false for any VAR not defined *in that event*.
    /// That is unsound for an iRules cross-event variable (set in another
    /// `when` handler), so drop those folds from `::when::*` procs now that the
    /// connection scope is known — otherwise O101 rewrites
    /// `if {[info exists ans_cleared]}` to `if {0}` even though a sibling event
    /// set it (a miscompile).
    fn drop_cross_event_existence_folds(
        procedures: &mut HashMap<String, FunctionUnit>,
        connection_scope: Option<&crate::connection_scope::ConnectionScope>,
    ) {
        let Some(cs) = connection_scope else {
            return;
        };
        let cross: HashSet<&str> = cs
            .cross_event_defs
            .iter()
            .chain(cs.cross_event_imports.iter())
            .map(String::as_str)
            .collect();
        if cross.is_empty() {
            return;
        }
        for (qn, fu) in procedures.iter_mut() {
            if !qn.starts_with("::when::") {
                continue;
            }
            fu.sccp.constant_branches.retain(|cb| {
                let mut vars = HashSet::new();
                crate::connection_scope::scan_info_exists(&cb.condition, &mut vars);
                // Keep the fold only if it does not query a cross-event var.
                !vars.iter().any(|v| cross.contains(v.as_str()))
            });
        }
    }

    /// Populate [`InterproceduralAnalysis`] via
    /// [`crate::interprocedural::build_interprocedural_analysis`]. Call after
    /// [`Self::build_for`] when a consumer (optimiser, compiler-checks)
    /// needs proc summaries.
    ///
    /// Re-runs `propagate_taints` on every function unit using the
    /// freshly-built summary + the requested `dialect`, so
    /// inter-procedural taint transfer and dialect-specific source
    /// handling take effect.
    #[must_use]
    pub fn with_interprocedural(
        mut self,
        registry: &CommandRegistry,
        dialect: Option<&str>,
    ) -> Self {
        // Object-handle → class map (SSA/VTA-derived) so a `$g walk … -command
        // cb` instance-method callback becomes a call-graph / reachability edge.
        let object_types = crate::object_types::object_handle_classes(&self, registry);
        // The unit's own proven command-identity facts, so the call-graph scan
        // classifies a rebound head as the command it is (issue #1275).
        let identities = crate::head_identity::command_head_identities_with_config(
            &self.source,
            tcl_lexer::LexerConfig::for_dialect(dialect.unwrap_or_default()),
            registry,
        );
        let interproc = crate::interprocedural::build_interprocedural_analysis(
            &self.ir_module,
            registry,
            dialect,
            crate::interprocedural::ObjectTypeMap(&object_types),
            &identities,
        );

        // Re-run taint with the new summary + dialect. We borrow
        // `interproc` immutably while each function unit re-runs
        // `propagate_taints`.
        self.top_level.taints = Arc::new(propagate_taints(
            &TaintGraph::new(
                &self.top_level.cfg,
                &self.top_level.ssa,
                &self.top_level.sccp,
            ),
            registry,
            Some(&self.top_level.rendered_props),
            Some(&interproc),
            dialect,
            None,
            None,
        ));
        for fu in self.procedures.values_mut() {
            fu.taints = Arc::new(propagate_taints(
                &TaintGraph::new(&fu.cfg, &fu.ssa, &fu.sccp),
                registry,
                Some(&fu.rendered_props),
                Some(&interproc),
                dialect,
                None,
                None,
            ));
        }

        self.interproc = Some(interproc);
        self
    }

    /// Like [`Self::with_interprocedural`] but routes each **procedure's**
    /// interprocedural taint re-run through `taint_cb`, a salsa-native memo (the
    /// `tcl-lsp-db` `taint_cascade` query).  The interprocedural summary is still
    /// built whole-module here (it is the memo's *input*); only the per-procedure
    /// `propagate_taints` re-run is memoised, so a procedure whose baseline and
    /// reachable-callee summaries are unchanged across an edit reuses its cached
    /// taints instead of re-propagating.
    ///
    /// `taint_cb(qname, &interproc)` returns the procedure's taints, or `None` to
    /// fall back to a fresh [`FunctionUnit::interproc_taints`] (e.g. a procedure
    /// the lattice memo didn't intern).  The top level is always re-run fresh.
    /// Byte-identical to [`Self::with_interprocedural`] provided `taint_cb`
    /// reproduces `propagate_taints` against the full summary — guarded by the
    /// `compiler_check` corpus differential and the taint-cascade edit tests.
    #[must_use]
    pub fn with_interprocedural_memoized(
        mut self,
        registry: &CommandRegistry,
        dialect: Option<&str>,
        taint_cb: &mut TaintCascadeCallback<'_>,
    ) -> Self {
        let object_types = crate::object_types::object_handle_classes(&self, registry);
        // The unit's own proven command-identity facts, so the call-graph scan
        // classifies a rebound head as the command it is (issue #1275).
        let identities = crate::head_identity::command_head_identities_with_config(
            &self.source,
            tcl_lexer::LexerConfig::for_dialect(dialect.unwrap_or_default()),
            registry,
        );
        let interproc = crate::interprocedural::build_interprocedural_analysis(
            &self.ir_module,
            registry,
            dialect,
            crate::interprocedural::ObjectTypeMap(&object_types),
            &identities,
        );

        // Top level is built fresh (no offset-0 lattice key), so its taint
        // re-run stays inline.
        let top_taints = self
            .top_level
            .interproc_taints(registry, &interproc, dialect);
        self.top_level.taints = Arc::new(top_taints);

        // Compute each procedure's taints (memoised via `taint_cb`, or fresh on a
        // miss) before mutating, so the immutable `&self.procedures` borrow the
        // fallback needs doesn't overlap the write-back.
        let mut new_taints: Vec<(String, Arc<HashMap<ValueKey, TaintLattice>>)> =
            Vec::with_capacity(self.procedures.len());
        for (qname, fu) in &self.procedures {
            let taints = taint_cb(qname, &interproc)
                .unwrap_or_else(|| Arc::new(fu.interproc_taints(registry, &interproc, dialect)));
            new_taints.push((qname.clone(), taints));
        }
        for (qname, taints) in new_taints {
            if let Some(fu) = self.procedures.get_mut(&qname) {
                fu.taints = taints;
            }
        }

        self.interproc = Some(interproc);
        self
    }

    /// Populate memory-SSA on the top-level and every procedure under
    /// `dialect`.
    #[must_use]
    pub fn with_memory_ssa(
        mut self,
        registry: &tcl_registry::CommandRegistry,
        dialect: DialectSet,
    ) -> Self {
        self.top_level = self.top_level.with_memory_ssa(registry, dialect);
        let mut out: HashMap<String, FunctionUnit> = HashMap::with_capacity(self.procedures.len());
        for (k, fu) in self.procedures.drain() {
            out.insert(k, fu.with_memory_ssa(registry, dialect));
        }
        self.procedures = out;
        self
    }

    /// Rebuild source-faithful executable facts, including world-state SSA,
    /// for Explorer and other explicit deep-inspection consumers.
    ///
    /// Ordinary interactive analysis intentionally avoids materialising a
    /// world graph when no reusable-value proof can consume it.  A caller
    /// requesting this method instead receives either the full graph or the
    /// existing typed availability/decline for every retained top-level and
    /// procedure script.
    #[must_use]
    pub fn with_deep_semantic_analysis(
        mut self,
        registry: &tcl_registry::CommandRegistry,
        dialect: DialectSet,
    ) -> Self {
        use crate::dispatch_proof::DispatchEntryAssumption;
        self.top_level.semantic_facts = SemanticAnalysisBundle::build(
            registry,
            dialect,
            &self.ir_module.top_level,
            DispatchEntryAssumption::PristineRegistryWorld,
        );
        for (name, unit) in &mut self.procedures {
            let Some(procedure) = self.ir_module.procedures.get(name) else {
                unit.semantic_facts = SemanticAnalysisBundle::unavailable(dialect);
                continue;
            };
            unit.semantic_facts = SemanticAnalysisBundle::build(
                registry,
                dialect,
                &procedure.body,
                DispatchEntryAssumption::UnknownWorld,
            );
        }
        for (name, unit) in &mut self.methods {
            let Some(method) = self.ir_module.methods.get(name) else {
                unit.semantic_facts = SemanticAnalysisBundle::unavailable(dialect);
                continue;
            };
            unit.semantic_facts = SemanticAnalysisBundle::build(
                registry,
                dialect,
                &method.body,
                DispatchEntryAssumption::UnknownWorld,
            );
        }
        for (name, unit) in &mut self.body_units {
            let Some(body) = self.ir_module.body_units.get(name) else {
                unit.semantic_facts = SemanticAnalysisBundle::unavailable(dialect);
                continue;
            };
            unit.semantic_facts = SemanticAnalysisBundle::build(
                registry,
                dialect,
                &body.body,
                DispatchEntryAssumption::UnknownWorld,
            );
        }
        self
    }

    /// Return the function unit for a qualified name, searching
    /// top-level + procedures.
    #[must_use]
    pub fn function(&self, name: &str) -> Option<&FunctionUnit> {
        if name == "::top" {
            return Some(&self.top_level);
        }
        self.procedures.get(name)
    }

    /// Iterate over every function unit in the module: the top-level unit
    /// first, then procedures **in qualified-name order**.
    ///
    /// The procedure store is a `HashMap`, whose iteration order is not
    /// stable across runs; sorting here gives every consumer
    /// (taint/gvn/shimmer/callouts/stats…) a deterministic function order,
    /// so the per-function diagnostics they emit are reproducible.
    pub fn functions(&self) -> impl Iterator<Item = &FunctionUnit> {
        let mut procs: Vec<&FunctionUnit> = self.procedures.values().collect();
        procs.sort_by(|a, b| a.name.cmp(&b.name));
        std::iter::once(&self.top_level).chain(procs)
    }

    /// The **narrowest** function unit whose source range covers `offset`
    /// (absolute): procedures, `TclOO` methods, and synthetic body units
    /// (`apply` lambdas, `namespace eval` bodies) all participate; the
    /// top-level unit is the containing fallback.  This is the locator a
    /// program-point query (e.g. the constant-`$cmd` value-provenance
    /// settlement) uses to find the SSA form that actually contains a
    /// site — a site inside a `namespace eval` body lives in that body's
    /// own unit, not in `::top`'s statement list.
    #[must_use]
    pub fn function_unit_at(&self, offset: u32) -> &FunctionUnit {
        let procs = self
            .procedures
            .iter()
            .filter_map(|(qname, fu)| Some((self.ir_module.procedures.get(qname)?.span, fu)));
        let methods = self
            .methods
            .iter()
            .filter_map(|(qname, fu)| Some((self.ir_module.methods.get(qname)?.span?, fu)));
        let bodies = self
            .body_units
            .iter()
            .filter_map(|(qname, fu)| Some((self.ir_module.body_units.get(qname)?.span, fu)));
        let mut best: Option<(u32, &FunctionUnit)> = None;
        for (range, fu) in procs.chain(methods).chain(bodies) {
            if !(range.start() <= offset && offset <= range.end()) {
                continue;
            }
            let width = range.end() - range.start();
            if best.is_none_or(|(bw, _)| width < bw) {
                best = Some((width, fu));
            }
        }
        best.map_or(&self.top_level, |(_, fu)| fu)
    }

    /// Instance-variable names in scope for the named function when it is a
    /// `TclOO`/snit *method* body (class-level `variable` declarations plus
    /// the method's own) — the alias-shaped names the shimmer thunking pass
    /// abstains on (their writes escape to the object, so the local SSA
    /// version chain is not the whole story).  `None` for procs and the top
    /// level.
    #[must_use]
    pub fn method_instance_vars(
        &self,
        function_name: &str,
    ) -> Option<&std::collections::HashSet<String>> {
        self.ir_module
            .methods
            .get(function_name)
            .map(|m| &m.instance_vars)
    }

    /// Like [`Self::functions`] but skips the functions the complexity guard
    /// excluded from deep analysis (oversized bodies). Per-proc diagnostic and
    /// optimiser passes iterate this so a guarded body — whose `ssa` and
    /// dataflow lattices are trivial — contributes no findings and costs no
    /// CFG walk. The relative order of the remaining functions is unchanged.
    pub fn analysable_functions(&self) -> impl Iterator<Item = &FunctionUnit> {
        self.functions().filter(|fu| !fu.complexity_guarded)
    }

    /// Iterate every *body-bearing* function unit whose body is statically
    /// analysable: the top-level, every procedure, every `TclOO` method, **and**
    /// every synthetic body unit (`apply` lambda / `namespace eval` body).
    ///
    /// This is the coverage-complete iterator for analyses that must reach
    /// *inside* every statically-known frame (e.g. regex-source tracking),
    /// unlike [`Self::functions`] which — for backwards compatibility with the
    /// per-proc diagnostic passes — yields only top-level + procedures. Order:
    /// top-level, procedures (name-sorted), then methods and body units
    /// (name-sorted) for reproducibility.
    pub fn all_body_function_units(&self) -> impl Iterator<Item = &FunctionUnit> {
        let mut extra: Vec<&FunctionUnit> = self
            .methods
            .values()
            .chain(self.body_units.values())
            .collect();
        extra.sort_by(|a, b| a.name.cmp(&b.name));
        self.functions().chain(extra)
    }

    /// Like [`Self::all_body_function_units`] but skips complexity-guarded
    /// bodies — the coverage-complete counterpart to
    /// [`Self::analysable_functions`].
    ///
    /// `compiler_checks::run_all_checks` (SCCP / GVN / shimmer / taint) used
    /// to iterate [`Self::analysable_functions`], which — per that method's
    /// "backwards compatibility" note — never reached `TclOO` method bodies
    /// or synthetic `apply`/`namespace eval` body units: a tainted value
    /// flowing into `puts` (or any other sink) *inside a method* produced no
    /// diagnostic at all, even though the optimiser's whole-module pass
    /// already iterates `cu.methods` directly (`optimiser::manager`) and
    /// `all_body_function_units` already exists for other coverage-complete
    /// analyses (regex-source tracking). This is the drop-in replacement
    /// that closes that gap without reintroducing a guarded body (whose
    /// trivial lattices would contribute noise, not findings) — see
    /// `compiler_checks::push_taint_and_module_checks`.
    pub fn analysable_body_function_units(&self) -> impl Iterator<Item = &FunctionUnit> {
        self.all_body_function_units()
            .filter(|fu| !fu.complexity_guarded)
    }

    /// The synthetic body units (`apply` / `namespace eval`) alone, name-sorted.
    pub fn body_function_units(&self) -> impl Iterator<Item = &FunctionUnit> {
        let mut v: Vec<&FunctionUnit> = self.body_units.values().collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v.into_iter()
    }

    /// Every `TclOO` method and synthetic body unit (`apply` lambda,
    /// `namespace eval` body) that isn't complexity-guarded — the extra half
    /// of [`Self::all_body_function_units`] beyond [`Self::analysable_functions`]
    /// (top-level + procedures), name-sorted for reproducibility.
    ///
    /// Exists for a pass that already reaches top-level + procedures through
    /// [`Self::analysable_functions`] elsewhere and wants *only* these two
    /// extra function kinds added, without double-visiting top-level or a
    /// procedure or double-counting a diagnostic. Used by
    /// `tcl-lsp-db::proc_taint_solve`'s `function_nontaint_checks` top-up loop:
    /// that memoised query's main per-function loop still iterates
    /// [`Self::analysable_functions`] (proc-only), unlike
    /// `compiler_checks::run_all_checks_with_solved_and_patterns`'s direct
    /// path, which iterates the wider [`Self::all_body_function_units`]
    /// (filtered) and so needs no separate top-up — adding this iterator's
    /// output there too would re-run `function_nontaint_checks` a second time
    /// over the methods/body units the direct path already covers.
    pub fn analysable_methods_and_body_units(&self) -> impl Iterator<Item = &FunctionUnit> {
        let mut v: Vec<&FunctionUnit> = self
            .methods
            .values()
            .chain(self.body_units.values())
            .filter(|fu| !fu.complexity_guarded)
            .collect();
        v.sort_by(|a, b| a.name.cmp(&b.name));
        v.into_iter()
    }
}

/// Fully-qualified names of every class defined in `source`.
///
/// Sourced from [`crate::signature_scan`] (which records `oo::class create` and
/// `itcl::class` definitions) so the standalone analyser build and the
/// incremental/db build derive an *identical* set from the same source — the
/// precondition for the two paths agreeing on constructor (`Foo new`) typing.
/// Gated on a cheap substring probe (`class`, the only token both class-defining
/// heads share) so the overwhelmingly common non-OO source skips the scan
/// entirely.  A false-positive probe just runs the (still fast) scan; the probe
/// can never miss a real definition because both heads contain `class`.
fn collect_known_classes(source: &str, registry: &CommandRegistry) -> HashSet<String> {
    // Cheap gate before the full signature scan.  A class is created by
    // `oo::class` / `itcl::class` (both contain "class") **or** by another stock
    // `TclOO` metaclass — `oo::configurable` / `oo::abstract` / `oo::singleton`,
    // none of which contain the word "class" — so also admit any `oo::` head
    // (issue #797: an `oo::configurable` class was skipped here, leaving its
    // `[Class new]` untyped).  snit definers (`snit::type` / `snit::widget` /
    // `snit::widgetadaptor`) contain neither "class" nor "oo::", so admit
    // `snit::` too, or a pure-snit file's `[Name create obj]` stays untyped.
    if !source.contains("class") && !source.contains("oo::") && !source.contains("snit::") {
        return HashSet::new();
    }
    crate::signature_scan::extract_signatures(source, registry)
        .classes
        .into_keys()
        .collect()
}

/// Encode interprocedural `param_constants` (caller-uniform-literal SCCP seeds)
/// into the deterministic, hashable form interned into the salsa-native
/// `FnLatticeKey`.  [`params_constants_from_call_sites`] only ever emits
/// `Const(String)` seeds, so the encoding is a sorted vec of `(param, version,
/// string)` triples; sorting makes the encoding independent of hash-map
/// iteration order so equal seeds always intern to the same key.
///
/// `None` input (no seeds) encodes to an empty vec.  Returns `None` only if a
/// seed has some other lattice shape — a shape we can't faithfully intern, so
/// the caller falls back to a fresh build rather than silently dropping it.
/// Defensive: the current producer never emits a non-string seed.
#[must_use]
pub(crate) fn encode_param_constants(
    param_constants: Option<&HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>>,
) -> Option<Vec<(String, u32, String)>> {
    use crate::analyses::{ConstValue, LatticeValue};
    let Some(map) = param_constants else {
        return Some(Vec::new());
    };
    let mut out = Vec::with_capacity(map.len());
    for ((name, version), val) in map {
        let LatticeValue::Const(ConstValue::String(s)) = val else {
            return None;
        };
        out.push((name.clone(), *version, s.clone()));
    }
    out.sort();
    Some(out)
}

/// Inverse of [`encode_param_constants`]: rebuild the SCCP seed map a
/// `function_lattice` build feeds to [`FunctionUnit::build_with_param_constants`].
/// An empty slice (no seeds) decodes to `None`.
#[must_use]
pub fn decode_param_constants(
    encoded: &[(String, u32, String)],
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    use crate::analyses::{ConstValue, LatticeValue};
    if encoded.is_empty() {
        return None;
    }
    Some(
        encoded
            .iter()
            .map(|(name, version, s)| {
                (
                    (name.clone(), *version),
                    LatticeValue::Const(ConstValue::String(s.clone())),
                )
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    /// Regression: a *short* procedure name shared by two procedures must resolve
    /// in `prepare_cfg_context` deterministically (qualified-name-sorted-last
    /// wins), not by `HashMap` iteration order — otherwise the `function_lattice`
    /// memo key flakes and the per-procedure lattice cache hits or misses by
    /// random seed (found via the offset-invariance experiments).
    #[test]
    fn prepare_cfg_context_short_name_collision_is_deterministic() {
        let reg = registry();
        let src = "namespace eval ::a { proc x {p1} { set q $p1 } }
                   namespace eval ::b { proc x {p2 extra} { set q $p2 } }
";
        let m =
            crate::lowering::lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::default());
        let (_, proc_params, _) = crate::cfg_builder::prepare_cfg_context(&m);
        // `::b::x` sorts after `::a::x`, so the short name `x` deterministically
        // resolves to `::b::x`'s parameters on every run.
        assert_eq!(
            proc_params.get("x"),
            Some(&vec!["p2".to_string(), "extra".to_string()]),
        );
    }

    #[test]
    fn build_for_empty_source() {
        let cu = CompilationUnit::build_for("", &registry(), false);
        assert_eq!(cu.source, "");
        assert_eq!(cu.top_level.name, "::top");
        assert!(cu.procedures.is_empty());
    }

    /// Issue #1174: every method unit carries exactly one instance-state
    /// carrier (`method_facts`), and its content matches the typed IR — the
    /// invariant that keeps the existence fold (I230/O100/O101), the
    /// W210/W211/W220 `known_bound` set, and the optimiser's method-constants
    /// escaping set reading the same struct.
    #[test]
    fn method_units_carry_the_single_method_facts_carrier() {
        let src = "oo::class create C {\n\
                       variable state\n\
                       method m {arg} { return $arg }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.methods.get("::C::m").expect("method unit built");
        let facts = fu
            .method_facts
            .as_deref()
            .expect("a method unit must carry method_facts");
        assert_eq!(facts.params, vec!["arg".to_string()]);
        assert!(facts.instance_vars.contains("state"));
        let known = facts.known_bound_at_entry();
        assert!(known.contains("arg") && known.contains("state"));
        // The IR the carrier was built from agrees (same construction site).
        let ir = cu.ir_module.methods.get("::C::m").expect("method IR");
        assert_eq!(facts.instance_vars, ir.instance_vars);
        assert_eq!(facts.params, ir.params);
    }

    /// TN for #1174: bodies with no object state — the top level and plain
    /// procedures — carry no `method_facts`, so no consumer can mistake a
    /// proc frame for a method frame.
    #[test]
    fn non_method_units_carry_no_method_facts() {
        let src = "proc p {a} { return $a }\nset x 1\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        assert!(cu.top_level.method_facts.is_none());
        let fu = cu.procedures.get("::p").expect("proc unit built");
        assert!(fu.method_facts.is_none());
    }

    #[test]
    fn build_for_single_statement() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false);
        assert!(!cu.top_level.cfg.blocks.is_empty());
        // SCCP should mark entry executable for a non-empty CFG.
        assert!(
            cu.top_level
                .sccp
                .executable_blocks
                .contains(&cu.top_level.ssa.entry)
        );
    }

    /// Regression: a top-level bare name reassigned by *another* procedure's
    /// own `global NAME` must never resolve to a `Const` in the top-level
    /// unit's own SCCP lattice — top-level names already live in the global
    /// frame, so the write is visible there even though the top-level body
    /// itself never declares `global`. Confirmed against tclsh 8.6/9.0 as a
    /// real miscompile before `scan_module_global_names` fed into
    /// `sccp_with_extra_escaping`: the optimiser proposed folding a later
    /// `puts $g` / `if {$g == …}` to the stale pre-call literal.
    #[test]
    fn top_level_var_touched_by_callee_global_is_overdefined() {
        let reg = registry();
        let src = "set g 4\nproc helper {} { global g\nset g 17 }\nhelper\n";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let sym = cu
            .top_level
            .ssa
            .var_symbol("g")
            .expect("top-level `g` should be interned");
        let all_overdefined = cu
            .top_level
            .sccp
            .values
            .iter()
            .filter(|((s, _), _)| *s == sym)
            .all(|(_, lv)| !matches!(lv, crate::analyses::LatticeValue::Const(_)));
        assert!(
            all_overdefined,
            "expected every `g` lattice entry to be non-Const, got {:?}",
            cu.top_level
                .sccp
                .values
                .iter()
                .filter(|((s, _), _)| *s == sym)
                .collect::<Vec<_>>()
        );
    }

    /// Regression, dual-ported-variable flavour of the above: C Tcl's
    /// interpreter-linked globals (`tcl_precision`, `auto_path`, `env`,
    /// `tcl_platform`, …) are ordinary Tcl variables from the analysed
    /// script's point of view — `global`/`set` on them lowers exactly like
    /// any other name — so they must get exactly the same
    /// `scan_module_global_names` protection as `g` above, with no
    /// special-casing needed anywhere in the compiler. `tcl_precision` is
    /// registered in `tcl_registry::special_vars` (confirmed by the
    /// `for name in [...]` list in that crate's own tests), so this also
    /// locks in that the read-side SCCP protection and the write-side
    /// `special_var_write_effect` side-effect tagging (`side_effects.rs`)
    /// compose correctly on the same variable rather than one substituting
    /// for the other.
    #[test]
    fn top_level_dual_ported_var_touched_by_callee_global_is_overdefined() {
        let reg = registry();
        let src = "set tcl_precision 4\nproc helper {} { global tcl_precision\nset tcl_precision 17 }\nhelper\n";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let sym = cu
            .top_level
            .ssa
            .var_symbol("tcl_precision")
            .expect("top-level `tcl_precision` should be interned");
        let all_overdefined = cu
            .top_level
            .sccp
            .values
            .iter()
            .filter(|((s, _), _)| *s == sym)
            .all(|(_, lv)| !matches!(lv, crate::analyses::LatticeValue::Const(_)));
        assert!(
            all_overdefined,
            "expected every `tcl_precision` lattice entry to be non-Const, got {:?}",
            cu.top_level
                .sccp
                .values
                .iter()
                .filter(|((s, _), _)| *s == sym)
                .collect::<Vec<_>>()
        );
    }

    /// Control: a top-level name *no* procedure ever `global`-declares must
    /// still fold to a genuine `Const` — the whole-module scan must not
    /// over-widen every top-level variable (that would be a precision
    /// regression, not a soundness fix).
    #[test]
    fn top_level_var_untouched_by_any_callee_still_folds() {
        let reg = registry();
        let src = "set safe_const 42\nproc other {} { puts unrelated }\nother\n";
        let cu = CompilationUnit::build_for(src, &reg, false);
        let sym = cu
            .top_level
            .ssa
            .var_symbol("safe_const")
            .expect("top-level `safe_const` should be interned");
        let has_const =
            cu.top_level.sccp.values.iter().any(|((s, _), lv)| {
                *s == sym && matches!(lv, crate::analyses::LatticeValue::Const(_))
            });
        assert!(has_const, "expected `safe_const` to still fold to a Const");
    }

    #[test]
    fn complexity_guard_flags_byte_huge_proc() {
        // A normal proc is analysed (not guarded), with real SSA + SCCP.
        let normal = CompilationUnit::build_for("proc small {} { set x 1 }", &registry(), false);
        let small = normal.function("::small").expect("::small built");
        assert!(!small.complexity_guarded);
        assert!(!small.ssa.blocks.is_empty());

        // A block-light but byte-huge body (one statement with a ~270 KB
        // literal) trips the body-byte half of the complexity guard: the unit
        // is flagged and carries a trivial SSA, so the O(blocks·vars) walk and
        // the dataflow passes never run on it.
        let big_literal = "A".repeat(270_000);
        let src = format!("proc big {{}} {{ set x \"{big_literal}\" }}");
        let cu = CompilationUnit::build_for(&src, &registry(), false);
        let big = cu.function("::big").expect("::big built");
        assert!(big.complexity_guarded, "byte-huge body must be guarded");
        assert!(big.ssa.blocks.is_empty(), "guarded unit has trivial SSA");
        assert!(big.sccp.values.is_empty());
        // Guarded functions are excluded from the analysable-function view the
        // per-proc diagnostic and optimiser passes iterate.
        assert!(
            cu.analysable_functions().all(|fu| fu.name != "::big"),
            "guarded ::big must be skipped by analysable_functions"
        );
    }

    #[test]
    fn build_for_captures_procedures() {
        let cu = CompilationUnit::build_for("proc greet {name} {puts $name}", &registry(), false);
        assert!(!cu.procedures.is_empty());
        assert!(cu.function("::greet").is_some());
        assert!(cu.function("::top").is_some());
        // No OO methods in a plain proc source.
        assert!(cu.methods.is_empty());
    }

    #[test]
    fn build_for_lowers_oo_methods_to_function_units() {
        // TclOO method bodies get their own
        // FunctionUnit (CFG → SSA → analysis) in `cu.methods`, kept
        // separate from `procedures` so the optimiser's O126 gate can
        // iterate them. `procedures` stays free of method qnames.
        let src = "oo::class create Counter {\n\
                   \x20   variable n\n\
                   \x20   method bump {} { incr n }\n\
                   \x20   method get {} { return $n }\n\
                   }\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let mut keys: Vec<&String> = cu.methods.keys().collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                &"::Counter::bump".to_string(),
                &"::Counter::get".to_string()
            ],
            "method units: {keys:?}",
        );
        // Each method unit carries a real analysis pipeline (its CFG
        // has the entry block at minimum).
        let get = &cu.methods["::Counter::get"];
        assert_eq!(get.name, "::Counter::get");
        assert!(!get.cfg.blocks.is_empty());
        // Methods are NOT mixed into the per-proc map.
        assert!(!cu.procedures.contains_key("::Counter::get"));
    }

    #[test]
    fn switch_subject_counts_as_param_use() {
        // The `switch -- $col` subject lowers to an
        // `ExprNode::Raw` branch condition.  The expr var-scan now
        // recovers `$col`, so the parameter's def-use chain has a
        // live (terminator) use instead of looking dead — the precise
        // path behind W214 (unused parameter).  `col` is referenced
        // *only* as the switch subject here, so a live chain proves
        // the Raw scan reached the def-use builder.
        let cu = CompilationUnit::build_for(
            "proc p {col} { switch -- $col { a {set y 1} } }",
            &registry(),
            false,
        );
        let fu = cu.function("::p").expect("proc ::p should exist");
        let col_live = fu
            .def_use
            .chains
            .iter()
            .any(|(k, c)| k.0 == "col" && !c.is_dead());
        assert!(
            col_live,
            "switch subject `$col` should register a live use; chains: {:?}",
            fu.def_use.chains.keys().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn switch_glob_arm_body_read_counts_as_param_use() {
        // A `switch -glob`/`-regexp` arm body is now a
        // real analysed CFG region (it used to vanish into a barrier).
        // A parameter referenced *only* inside a glob-arm body therefore
        // has a live def-use chain — the precise path behind W214.
        // Multi-arg arm form (the single-braced-body form is a separate,
        // pre-existing lowering gap affecting every mode equally).
        for (mode, pat) in [("-glob", "a*"), ("-regexp", "a.*")] {
            let src = format!("proc p {{val}} {{ switch {mode} -- $col {pat} {{puts $val}} }}");
            let cu = CompilationUnit::build_for(&src, &registry(), false);
            let fu = cu.function("::p").expect("proc ::p should exist");
            let val_live = fu
                .def_use
                .chains
                .iter()
                .any(|(k, c)| k.0 == "val" && !c.is_dead());
            assert!(
                val_live,
                "{mode} arm-body read `$val` should register a live use; chains: {:?}",
                fu.def_use.chains.keys().collect::<Vec<_>>(),
            );
        }
    }

    #[test]
    fn return_type_infers_int_literal() {
        let cu = CompilationUnit::build_for("proc f {} { return 1 }", &registry(), false);
        let fu = cu.function("::f").expect("proc ::f");
        assert_eq!(fu.return_type, TypeLattice::of(tcl_registry::TclType::Int));
    }

    #[test]
    fn return_type_infers_string_literal() {
        let cu = CompilationUnit::build_for("proc f {} { return \"hi\" }", &registry(), false);
        let fu = cu.function("::f").expect("proc ::f");
        assert_eq!(
            fu.return_type,
            TypeLattice::of(tcl_registry::TclType::String)
        );
    }

    #[test]
    fn return_type_follows_local_var() {
        let cu = CompilationUnit::build_for("proc f {} { set x 1; return $x }", &registry(), false);
        let fu = cu.function("::f").expect("proc ::f");
        assert_eq!(fu.return_type, TypeLattice::of(tcl_registry::TclType::Int));
    }

    #[test]
    fn return_type_joins_branches_to_common_int() {
        let cu = CompilationUnit::build_for(
            "proc f {a} { if {$a} { return 1 } else { return 2 } }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").expect("proc ::f");
        assert_eq!(fu.return_type, TypeLattice::of(tcl_registry::TclType::Int));
    }

    #[test]
    fn return_type_partial_return_widens_via_fallthrough() {
        // `if {$a} { return 1 }` with no else: the false path falls off
        // the end of the body (Tcl returns the last command's result),
        // so the joined return type must not be a confident `Int` — the
        // fall-through exit widens it to Overdefined.
        let cu =
            CompilationUnit::build_for("proc f {a} { if {$a} { return 1 } }", &registry(), false);
        let fu = cu.function("::f").expect("proc ::f");
        assert_eq!(
            fu.return_type,
            TypeLattice::overdefined(),
            "partial-return proc must widen, got {:?}",
            fu.return_type,
        );
    }

    #[test]
    fn with_memory_ssa_populates_optional() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false)
            .with_memory_ssa(&registry(), DialectSet::TCL86);
        assert!(cu.top_level.memory_ssa.is_some());
    }

    #[test]
    fn semantic_bundle_keeps_dialect_and_defers_unneeded_world_state() {
        let linear = CompilationUnit::build_for_dialect("puts hello", &registry(), false, "tcl8.6");
        let facts = &linear.top_level.semantic_facts;
        assert_eq!(facts.dialect(), DialectSet::TCL86);
        let executable = facts.executable();
        assert_eq!(executable.invocations().count(), 1);
        assert!(executable.world_state_ssa().is_none());
        assert!(matches!(
            executable,
            crate::semantic_analysis::ExecutableAnalysisAvailability::WorldStateNotRequired { .. }
        ));
        assert_eq!(executable.completion_inputs().count(), 1);
        assert_eq!(executable.effect_inputs().count(), 1);

        let structured = CompilationUnit::build_for_dialect(
            "if {1} { puts hello }",
            &registry(),
            false,
            "tcl8.6",
        );
        let structured = structured.top_level.semantic_facts.executable();
        assert!(structured.function().is_some());
        assert_eq!(structured.opaque_regions().count(), 1);
        assert!(structured.world_state_ssa().is_none());

        let ir = crate::lowering::lower_to_ir("puts hello", &registry());
        let explicit = crate::semantic_analysis::SemanticAnalysisBundle::build(
            &registry(),
            DialectSet::TCL86,
            &ir.top_level,
            crate::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
        );
        assert!(explicit.executable().world_state_ssa().is_some());

        let legacy = CompilationUnit::build_for("puts hello", &registry(), false);
        assert!(matches!(
            legacy.top_level.semantic_facts.executable(),
            crate::semantic_analysis::ExecutableAnalysisAvailability::DialectUnavailable {
                dialect
            } if *dialect == DialectSet::empty()
        ));
    }

    #[test]
    fn deep_semantic_analysis_materialises_every_retained_body_sidecar() {
        let deep = CompilationUnit::build_for_dialect(
            "proc p {} { puts hello }\nnamespace eval ::n { puts body }\n",
            &registry(),
            false,
            "tcl8.6",
        )
        .with_deep_semantic_analysis(&registry(), DialectSet::TCL86);
        // The top-level source contains a structural `namespace eval` whose
        // completion switch is not yet representable by the common world-SSA
        // planner. Deep inspection must still retain executable facts and the
        // typed `WorldStateDeclined` outcome rather than requiring a graph.
        assert!(
            deep.top_level
                .semantic_facts
                .executable()
                .function()
                .is_some()
        );
        assert!(
            deep.procedures
                .get("::p")
                .unwrap()
                .semantic_facts
                .executable()
                .function()
                .is_some()
        );
        assert!(
            deep.body_units.values().all(|unit| unit
                .semantic_facts
                .executable()
                .function()
                .is_some())
        );
    }

    #[test]
    fn functions_iterator_yields_top_plus_procs() {
        let cu = CompilationUnit::build_for(
            "proc foo {} {return 1}\nproc bar {} {return 2}",
            &registry(),
            false,
        );
        let count = cu.functions().count();
        assert_eq!(count, cu.procedures.len() + 1);
    }

    /// Issue #969 / interprocedural call-site literal seeding (TP/FP/TN/FN
    /// suite for [`collect_call_site_constants`] / [`params_constants_from_call_sites`]).
    mod call_site_param_constants {
        use super::*;
        use crate::analyses::LatticeValue;

        /// FN regression: a call from inside a `TclOO` method body to an
        /// ordinary user proc is a real caller with a differing argument,
        /// but methods are built in a *separate* pass that runs after (and
        /// was invisible to) the call-site scan — the same "call site
        /// silently vanishes from the evidence" failure as issue #969's own
        /// root cause, reached through a method body instead of
        /// namespace-blind recursion or a `catch`/`uplevel` body. Before
        /// `build_extra_call_site_scan_contexts`, this scan only ever saw
        /// the one external `helper a` call, so it wrongly seeded `mode` as
        /// the constant `"a"`.
        #[test]
        fn call_site_inside_tcloo_method_body_is_not_missed() {
            let reg = registry();
            let src = "
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                oo::class create Widget {
                    method go {} { helper b }
                }
                helper a
                [Widget new] go
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let f = cu.procedures.get("::helper").expect("helper analysed");
            assert!(
                !folds_condition_mentioning(f, "mode"),
                "helper is called with both \"a\" (external) and \"b\" (from the method body); must not fold: {:?}",
                f.sccp.constant_branches,
            );
        }

        /// FN regression: a call reached only via a `namespace import` alias
        /// is a real caller with a differing argument, but
        /// `resolve_internal_call` alone only tries the caller's own
        /// namespace and the global one — it doesn't know about imports. The
        /// same "call site silently vanishes from the evidence" failure as
        /// issue #969's own root cause, reached through an imported bare
        /// name instead of namespace-blind recursion, a `catch`/`uplevel`
        /// body, or a `TclOO` method body.
        #[test]
        fn call_site_via_namespace_import_alias_is_not_missed() {
            let reg = registry();
            let src = "
                namespace eval ::lib {
                    namespace export helper
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                }
                namespace eval ::app {
                    namespace import ::lib::helper
                    proc go {} { helper b }
                }
                ::lib::helper a
                ::app::go
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let f = cu.procedures.get("::lib::helper").expect("helper analysed");
            assert!(
                !folds_condition_mentioning(f, "mode"),
                "helper is called with both \"a\" (direct) and \"b\" (via the ::app import); must not fold: {:?}",
                f.sccp.constant_branches,
            );
        }

        /// `::lib::helper` has two resolvable callers — one direct, one
        /// through a `namespace import ::lib::*` wildcard alias — and both
        /// pass `"prod"`.  Carries no `namespace export`: the scan resolves
        /// the import from the recorded directive (export *enforcement* is
        /// not modelled), and an export would cross the `EXPORTS_COMMAND`
        /// boundary the sibling test below exercises deliberately.
        const WILDCARD_IMPORT_SRC: &str = "
            namespace eval ::lib {
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                }
            }
            namespace eval ::app {
                namespace import ::lib::*
                proc go {} { helper prod }
            }
            ::lib::helper prod
            ::app::go
        ";

        /// TP control: a wildcard `namespace import ::lib::*` alias must
        /// still resolve correctly (not just exact-name imports), and the
        /// mechanism must still fold when every resolved caller — direct and
        /// imported — genuinely agrees on the literal.
        #[test]
        fn wildcard_namespace_import_alias_resolves_and_still_folds_when_uniform() {
            let reg = registry();
            let cu = CompilationUnit::build_for(WILDCARD_IMPORT_SRC, &reg, false);
            let f = cu.procedures.get("::lib::helper").expect("helper analysed");
            assert!(
                folds_condition_mentioning(f, "mode"),
                "both the direct and the wildcard-imported caller pass \"prod\": {:?}",
                f.sccp.constant_branches,
            );
        }

        /// FP guard (issue #977): adding `namespace export *` to the very
        /// same source must stop the fold — and, unlike a `source` boundary,
        /// must keep stopping it even when a host supplies a workspace view.
        /// An export publishes `::lib::helper` for *any* other unit to import
        /// and call with a different literal, including one in a different
        /// checkout that no project enumeration can reach.
        #[test]
        fn exported_namespace_declines_the_seed_even_with_a_workspace_view() {
            let reg = registry();
            let exported = WILDCARD_IMPORT_SRC.replace(
                "namespace eval ::lib {",
                "namespace eval ::lib {\n                namespace export *",
            );
            for cu in [
                CompilationUnit::build_for(&exported, &reg, false),
                build_closed_world(&exported, &reg),
            ] {
                let f = cu.procedures.get("::lib::helper").expect("helper analysed");
                assert!(
                    !folds_condition_mentioning(f, "mode"),
                    "`namespace export *` publishes helper beyond any enumerable project: {:?}",
                    f.sccp.constant_branches,
                );
            }
        }

        /// Pinning test for issue #979: a proc reached only through a
        /// `namespace ensemble` `-map` redirection is a real caller the
        /// call-site scan cannot resolve — it has no model of ensemble
        /// dispatch. tclsh8.6/9.0-confirmed: with `namespace ensemble
        /// create -command myens -map {go helper}`, `myens go dev` prints
        /// `helper mode=dev`, so `mode` genuinely varies and must not fold.
        ///
        /// Nothing pinned that today. What makes it safe is *indirect*: the
        /// registry marks `namespace ensemble` with `Traits::EXPORTS_COMMAND`,
        /// which `params_constants_from_call_sites` treats as a boundary
        /// publishing the file's commands to callers it cannot enumerate, so
        /// the whole module declines seeding. Sound but blunt — and a future
        /// registry edit narrowing that trait (or a precision follow-up that
        /// starts resolving *some* ensemble maps) would silently reopen
        /// issue #969's exact false-fold shape here. This test guards that
        /// mechanism, whatever replaces it: the fold must stay off unless a
        /// real ensemble-map resolution lands.
        #[test]
        fn ensemble_map_redirected_caller_does_not_fold_issue_979() {
            let reg = registry();
            let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                }
                helper prod
                helper prod
                namespace ensemble create -command myens -map {go helper}
                myens go dev
            ";
            for cu in [
                CompilationUnit::build_for(src, &reg, false),
                build_closed_world(src, &reg),
            ] {
                let f = cu.procedures.get("::helper").expect("helper analysed");
                assert!(
                    !folds_condition_mentioning(f, "mode"),
                    "`myens go dev` reaches helper with \"dev\" (tclsh8.6/9.0-confirmed), \
                     so mode is not caller-invariant: {:?}",
                    f.sccp.constant_branches,
                );
            }
        }

        /// FP guard (Codex review, PR #970): a `TclOO` method body resolves
        /// bare commands against the GLOBAL namespace, never the class's own
        /// namespace — tclsh8.6-confirmed live: `[::foo::Widget new] go`
        /// (method body `helper b`) calls `::helper`, never
        /// `::foo::Widget::helper`, even though the latter exists and is
        /// exactly what naively deriving the caller's namespace from the
        /// method's own qualified name (`::foo::Widget::go` → `::foo::Widget`)
        /// would try first. Before forcing method bodies to resolve against
        /// global, this misattributed the method's call to
        /// `::foo::Widget::helper` (folding a condition on a call that never
        /// actually happens) while simultaneously losing it as evidence for
        /// the real target `::helper` (which then wrongly folded on its one
        /// remaining, external-only literal).
        #[test]
        fn method_body_bare_call_resolves_against_global_not_class_namespace() {
            let reg = registry();
            let src = "
                namespace eval ::foo {
                    oo::class create Widget {
                        method go {} { helper b }
                    }
                }
                namespace eval ::foo::Widget {
                    proc helper {mode} {
                        if {$mode eq \"WRONG\"} { set r 1 } else { set r 2 }
                    }
                }
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                helper a
                [::foo::Widget new] go
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let global_helper = cu.procedures.get("::helper").expect("::helper analysed");
            let ns_helper = cu
                .procedures
                .get("::foo::Widget::helper")
                .expect("::foo::Widget::helper analysed");
            assert!(
                !folds_condition_mentioning(global_helper, "mode"),
                "::helper is called with both \"a\" and \"b\" (the method body, once \
                 correctly resolved to global): {:?}",
                global_helper.sccp.constant_branches,
            );
            assert!(
                !folds_condition_mentioning(ns_helper, "mode"),
                "::foo::Widget::helper is never actually called by real Tcl semantics, \
                 so no call-site evidence should ever reach it: {:?}",
                ns_helper.sccp.constant_branches,
            );
        }

        /// FN regression (issue #980, Codex review of PR #970): `uplevel #0
        /// { … }`'s body resolves bare commands against the GLOBAL
        /// namespace — tclsh8.6/9.0-confirmed live: `uplevel #0 { helper b
        /// }` inside `::foo::runIt` prints `GLOBAL helper mode=b`, never the
        /// `::foo::helper` sitting in the enclosing namespace.
        /// `Statement::UpFrame`'s body survives CFG construction as a block
        /// *statement*, which `scan_cfg_callers` (walking only
        /// `Call`/`Barrier`) skips entirely, so this real, differing call
        /// site vanished from `::helper`'s evidence and its one remaining
        /// caller's `"a"` folded the condition.
        ///
        /// `build_extra_call_site_scan_contexts` now builds a bare CFG for
        /// every *absolute* shift-`0` `UpFrame` body, resolved as `"::top"`
        /// — mirroring how a `TclOO` method body is forced global.
        ///
        /// `uplevel N` for any relative (non-`#0`) level stays out of scope:
        /// the target frame's namespace depends on the live call stack,
        /// which single-file static analysis cannot decide — a documented,
        /// permanent approximation.
        #[test]
        fn uplevel_zero_body_resolves_against_global_not_enclosing_namespace() {
            let reg = registry();
            let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"WRONG\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        uplevel #0 { helper b }
                    }
                }
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                helper a
                ::foo::runIt
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let global_helper = cu.procedures.get("::helper").expect("::helper analysed");
            let ns_helper = cu
                .procedures
                .get("::foo::helper")
                .expect("::foo::helper analysed");
            assert!(
                !folds_condition_mentioning(global_helper, "mode"),
                "::helper is called with both \"a\" and \"b\" (uplevel #0, once correctly \
                 resolved to global): {:?}",
                global_helper.sccp.constant_branches,
            );
            assert!(
                !folds_condition_mentioning(ns_helper, "mode"),
                "::foo::helper is never actually called by real Tcl semantics: {:?}",
                ns_helper.sccp.constant_branches,
            );
        }

        /// MISCOMPILE regression (adversarial review): `uplevel 0 { … }` is
        /// the *relative* current-frame form and must NOT be treated as the
        /// absolute global form. Lowering encoded `#0` and `0` as the same
        /// `frame_shift == 0`, so this body was resolved against `"::top"`
        /// and `::foo::helper` lost its only varying call site — folding a
        /// branch that real Tcl reaches both ways.
        ///
        /// Oracle (tclsh8.6 and tclsh9.0, `review-probes/up1.tcl`): inside
        /// `::foo::runIt`, `uplevel #0 { helper b }` prints `GLOBAL helper`
        /// while `uplevel 0 { helper c }` prints `FOO helper`.
        #[test]
        fn uplevel_bare_zero_body_resolves_against_the_enclosing_namespace() {
            let reg = registry();
            let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        uplevel 0 { helper b }
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu
                .procedures
                .get("::foo::helper")
                .expect("::foo::helper analysed");
            assert!(
                !folds_condition_mentioning(helper, "mode"),
                "`uplevel 0` runs in the current frame, so ::foo::helper sees both \
                 \"a\" (direct) and \"b\" (the uplevel body): {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// Approximation pin (issue #980): a *relative* `uplevel N { … }`
        /// keeps resolving against the enclosing unit's own namespace. The
        /// target frame's namespace depends on the live call stack, so this
        /// is a deliberate, permanent approximation — but it must stay an
        /// approximation that still *counts* the call site, not one that
        /// drops it. `::foo::helper` here sees both `"a"` and `"b"`.
        #[test]
        fn uplevel_relative_body_keeps_the_enclosing_units_namespace() {
            let reg = registry();
            let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        uplevel 1 { helper b }
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu
                .procedures
                .get("::foo::helper")
                .expect("::foo::helper analysed");
            assert!(
                !folds_condition_mentioning(helper, "mode"),
                "::foo::helper sees both \"a\" (direct) and \"b\" (the relative uplevel \
                 body, approximated to the enclosing namespace): {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FN guard for the `Traits::DEFINES_PROCEDURE` body-recursion skip
        /// (issue #980): a definition body is no longer re-walked from the
        /// definition site, so its call sites must still arrive through the
        /// defined procedure's *own* CFG. A conditionally-defined proc is
        /// the shape most at risk — its `proc` call is not a plain top-level
        /// statement.
        #[test]
        fn conditionally_defined_proc_body_call_sites_are_still_counted() {
            let reg = registry();
            let src = "
                proc helper {mode} {
                    if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                }
                if {[info exists ::env(X)]} {
                    proc runIt {} { helper b }
                }
                helper a
                runIt
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu.procedures.get("::helper").expect("::helper analysed");
            assert!(
                !folds_condition_mentioning(helper, "mode"),
                "::helper is called with both \"a\" and \"b\" (the conditionally-defined \
                 runIt's body): {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FP guard (Codex review, PR #970): `namespace eval ::other { … }`
        /// runs its body in `::other`, never the enclosing proc's own
        /// namespace — tclsh8.6-confirmed live: a bare call inside such a
        /// block, nested arbitrarily deep inside an unrelated proc, still
        /// resolves against `::other`. Before threading the block's real
        /// target namespace into its body-unit qname
        /// (`register_body_unit`/`lower_namespace_eval`), every
        /// `namespace eval` body unit's qname reduced to the *global*
        /// namespace regardless of its actual target, so this call would
        /// have resolved (or misattributed) against global instead of
        /// `::other`.
        #[test]
        fn namespace_eval_body_nested_in_a_proc_resolves_against_its_own_namespace() {
            let reg = registry();
            let src = "
                namespace eval ::other {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                }
                proc runIt {} {
                    namespace eval ::other { helper b }
                }
                ::other::helper a
                runIt
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu
                .procedures
                .get("::other::helper")
                .expect("::other::helper analysed");
            assert!(
                !folds_condition_mentioning(helper, "mode"),
                "::other::helper is called with both \"a\" (direct) and \"b\" (via the \
                 namespace eval block nested inside runIt): {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// MISCOMPILE regression (adversarial review): `apply {params body
        /// ns}`'s third element names the namespace the body runs in, so a
        /// bare command word inside it resolves against *that* namespace.
        /// `lower_apply` computed the right `body_ns` and lowered the body
        /// against it, then registered the body unit under the bare `apply`
        /// marker — whose qname puts it in the global namespace. The call
        /// site was attributed to a `::helper` that does not exist and
        /// vanished from `::foo::helper`'s evidence, folding a branch real
        /// Tcl reaches both ways.
        ///
        /// Oracle (tclsh8.6 and tclsh9.0, `review-probes/ap3_run.tcl`):
        /// inside `::foo::runIt`, `apply {{x} { helper $x } ::foo} b`
        /// returns 2 — the `else` arm of `::foo::helper` — while the direct
        /// `::foo::helper a` returns 1.
        #[test]
        fn apply_with_a_namespace_element_resolves_the_body_against_it() {
            let reg = registry();
            let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        apply {{x} { helper $x } ::foo} b
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu
                .procedures
                .get("::foo::helper")
                .expect("::foo::helper analysed");
            assert!(
                !folds_condition_mentioning(helper, "mode"),
                "::foo::helper is called with both \"a\" (direct) and \"b\" (through the \
                 ::foo-pinned lambda): {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// The two-element form is unchanged: with no namespace element,
        /// `apply` runs the body in the *global* namespace, not the caller's
        /// (Tcl `apply` manual). So the enclosing namespace's `helper` is
        /// genuinely never called from the lambda and keeps its fold.
        #[test]
        fn apply_without_a_namespace_element_still_resolves_globally() {
            let reg = registry();
            let src = "
                namespace eval ::foo {
                    proc helper {mode} {
                        if {$mode eq \"a\"} { set r 1 } else { set r 2 }
                    }
                    proc runIt {} {
                        apply {{x} { helper $x }} b
                    }
                }
                ::foo::helper a
                ::foo::runIt
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu
                .procedures
                .get("::foo::helper")
                .expect("::foo::helper analysed");
            assert!(
                folds_condition_mentioning(helper, "mode"),
                "an unpinned lambda body resolves globally, so ::foo::helper only ever \
                 sees the direct \"a\": {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// True if any constant-branch condition recorded for `fu` mentions
        /// `needle` (a variable name) — the ambient SCCP-fold check the I230
        /// diagnostic, and the O101/O107 optimiser suggestions, all key off.
        /// Build `src` under a **closed world**: an empty cross-file evidence
        /// set, which is the host asserting "I enumerated the project and no
        /// other file calls into this one".  The pre-issue-#977 semantics for
        /// a file that is genuinely the whole program.
        fn build_closed_world(src: &str, reg: &CommandRegistry) -> CompilationUnit {
            let empty = crate::unit_scope::CallSiteEvidence::default();
            CompilationUnit::build_with_options(
                src,
                UnitBuildOptions {
                    registry: reg,
                    defer_top_level: false,
                    config: tcl_lexer::LexerConfig::default(),
                    dialect: "",
                    external_call_sites: Some(&empty),
                },
            )
        }

        fn folds_condition_mentioning(fu: &FunctionUnit, needle: &str) -> bool {
            fu.sccp
                .constant_branches
                .iter()
                .any(|b| b.condition.contains(needle))
        }

        /// FN (was silently wrong before the fix): a proc declared inside a
        /// `namespace eval` block recurses into itself by its *bare* name.
        /// The old resolver only ever tried global-qualified spellings of the
        /// command word, so it could never match the proc's namespaced
        /// qualified name — the recursive call (whose argument necessarily
        /// varies call to call) silently vanished from the call-site scan,
        /// leaving only the one external caller's literal `0` visible.
        /// `params_constants_from_call_sites` then (wrongly) seeded `count`
        /// as the compile-time constant `0`, folding the always-alternating
        /// `$count & 1` parity check to a fixed `false` — exactly the
        /// reported false positive.
        #[test]
        fn namespaced_recursive_proc_parity_check_is_not_constant_folded() {
            let reg = registry();
            let src = "
                namespace eval ::graph {
                    proc dfs {count} {
                        if {$count & 1} {
                            set parity odd
                        } else {
                            set parity even
                        }
                        if {$count < 3} {
                            dfs [expr {$count + 1}]
                        }
                    }
                }
                ::graph::dfs 0
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let dfs = cu.procedures.get("::graph::dfs").expect("dfs analysed");
            assert!(
                !folds_condition_mentioning(dfs, "count"),
                "recursive parity check on `count` must not fold to a constant: {:?}",
                dfs.sccp.constant_branches,
            );
        }

        /// TN control: the same shape at the *top level* (no namespace) was
        /// already sound before the fix (a bare recursive call already
        /// resolved to the right, un-namespaced qualified name), and must
        /// stay sound after it — the fix must not regress the case it
        /// didn't need to change.
        #[test]
        fn top_level_recursive_proc_parity_check_is_not_constant_folded() {
            let reg = registry();
            let src = "
                proc count_up {n} {
                    if {$n & 1} { set p odd } else { set p even }
                    if {$n < 3} { count_up [expr {$n + 1}] }
                }
                count_up 0
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let f = cu.procedures.get("::count_up").expect("count_up analysed");
            assert!(
                !folds_condition_mentioning(f, "n"),
                "recursive parity check on `n` must not fold to a constant: {:?}",
                f.sccp.constant_branches,
            );
        }

        /// TP control: the interprocedural seed must still fire for a
        /// genuinely proc-call-invariant parameter — two callers (one via a
        /// same-namespace bare call, proving the namespace-aware resolver
        /// still positively resolves, not just negatively declines) passing
        /// the identical literal.
        #[test]
        fn two_same_namespace_callers_with_uniform_literal_still_folds() {
            let reg = registry();
            let src = "
                namespace eval ::a {
                    proc go {mode} {
                        if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                    }
                    proc caller1 {} { go prod }
                    proc caller2 {} { go prod }
                }
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let go = cu.procedures.get("::a::go").expect("go analysed");
            assert!(
                folds_condition_mentioning(go, "mode"),
                "two same-namespace callers passing the identical literal should still fold: {:?}",
                go.sccp.constant_branches,
            );
        }

        /// FP guard: two same-leaf-name procs in different namespaces must
        /// never be conflated by a bare same-namespace call — `::a::go`'s
        /// only caller passes `"same"`; `::b::go`'s only caller passes
        /// `"different"`. Each must fold to *its own* literal, not the
        /// other's (which the old namespace-blind resolver could not even
        /// attempt, since it never matched either bare call to a real proc).
        #[test]
        fn sibling_namespace_procs_with_same_leaf_name_are_not_conflated() {
            let reg = registry();
            let src = "
                namespace eval ::a {
                    proc go {mode} {
                        if {$mode eq \"same\"} { set r 1 } else { set r 2 }
                    }
                    proc caller {} { go same }
                }
                namespace eval ::b {
                    proc go {mode} {
                        if {$mode eq \"same\"} { set r 1 } else { set r 2 }
                    }
                }
                ::b::go different
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let a_go = cu.procedures.get("::a::go").expect("::a::go analysed");
            let b_go = cu.procedures.get("::b::go").expect("::b::go analysed");
            assert!(
                folds_condition_mentioning(a_go, "mode"),
                "::a::go's sole caller passes a uniform literal, should fold: {:?}",
                a_go.sccp.constant_branches,
            );
            assert!(
                b_go.sccp
                    .constant_branches
                    .iter()
                    .any(|b| b.condition.contains("mode") && !b.value),
                "::b::go's sole caller passes \"different\", condition should fold false: {:?}",
                b_go.sccp.constant_branches,
            );
        }

        /// FP guard: a `rename` that could redirect a *different* command
        /// onto the callee's own name must disqualify the seed, even though
        /// every call site this scan can see (all made before the rename,
        /// textually) passes a uniform literal — `trusts_proc_binding` is
        /// flow-insensitive/whole-module by design (matching the identical
        /// gate already trusted for the optimiser's O103 proc-call fold), so
        /// it can't assume the calls happen before the rename takes effect.
        #[test]
        fn rename_onto_callee_name_disqualifies_call_site_seed() {
            let reg = registry();
            let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                proc other {mode} {
                    if {$mode eq \"prod\"} { set y 3 } else { set y 4 }
                }
                helper prod
                helper prod
                rename other helper
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu.procedures.get("::helper").expect("helper analysed");
            assert!(
                !folds_condition_mentioning(helper, "mode"),
                "a callee whose name is later rebound must not fold on stale call-site evidence: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// TN control and documented scope boundary: a dynamic (non-literal)
        /// call-site head elsewhere in the module is simply unresolvable —
        /// it contributes no evidence for or against any proc's params,
        /// exactly like any other call this scan can't attribute to a
        /// specific callee. `helper`'s own two uniform-literal callers still
        /// fold. Closing the residual "the dynamic call might secretly
        /// target `helper` too" gap would need a value-set fact for `$cmd`
        /// this pass runs too early in the pipeline to have (it produces
        /// the very SCCP seed such a fact would depend on) — a pre-existing
        /// limitation of this call-site scan, not a regression.
        #[test]
        fn dynamic_dispatch_elsewhere_does_not_disqualify_an_unrelated_seed() {
            let reg = registry();
            let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
                set cmd helper
                $cmd prod
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu.procedures.get("::helper").expect("helper analysed");
            assert!(
                folds_condition_mentioning(helper, "mode"),
                "an unrelated dynamic call site must not disqualify helper's own uniform-literal seed: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FN regression: a call site embedded inside a `catch { … }` body is
        /// a real caller with a differing argument, but `catch`'s body is an
        /// `ArgRole::Body` argument of the *builtin* `catch` — never a user
        /// proc — so a flat, one-level call-site walk resolves `catch`,
        /// finds no matching proc, and moves on without ever noticing the
        /// `isEven 4` sitting inside it. Before recursing into `ArgRole::Body`
        /// arguments, this scan only ever saw the one external `isEven 3`
        /// call, so it wrongly seeded `n` as the constant `3`.
        #[test]
        fn call_site_inside_catch_body_is_not_missed() {
            let reg = registry();
            let src = "
                proc is_even {n} {
                    if {$n % 2 == 0} { return 1 } else { return 0 }
                }
                proc main {} {
                    is_even 3
                    catch { is_even 4 }
                }
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let f = cu.procedures.get("::is_even").expect("is_even analysed");
            assert!(
                !folds_condition_mentioning(f, "n"),
                "is_even is called with both 3 and 4 (the latter inside `catch`); must not fold: {:?}",
                f.sccp.constant_branches,
            );
        }

        /// FN regression, `uplevel` flavour: a literal `uplevel {…}` body is
        /// an `ArgRole::Body` argument of the builtin `uplevel`, exactly like
        /// `catch`'s.
        #[test]
        fn call_site_inside_uplevel_body_is_not_missed() {
            let reg = registry();
            let src = "
                proc is_even {n} {
                    if {$n % 2 == 0} { return 1 } else { return 0 }
                }
                proc main {} {
                    is_even 3
                    uplevel 1 { is_even 4 }
                }
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let f = cu.procedures.get("::is_even").expect("is_even analysed");
            assert!(
                !folds_condition_mentioning(f, "n"),
                "is_even is called with both 3 and 4 (the latter inside `uplevel`); must not fold: {:?}",
                f.sccp.constant_branches,
            );
        }

        /// TN control: an unrelated call inside a `catch` body must not
        /// disqualify a *different*, genuinely call-site-invariant proc's
        /// seed — the recursive Body-arg scan must attribute evidence to the
        /// right callee, not blanket-disqualify everything the way the
        /// (reverted) dynamic-dispatch wildcard did.
        #[test]
        fn unrelated_catch_body_does_not_disqualify_a_different_proc() {
            let reg = registry();
            let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                proc noisy {} {
                    catch { nonexistentCommand abc }
                }
                helper prod
                helper prod
                noisy
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu.procedures.get("::helper").expect("helper analysed");
            assert!(
                folds_condition_mentioning(helper, "mode"),
                "an unrelated catch body must not disqualify helper's own uniform-literal seed: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FP guard: a `package provide` file may export procs another file
        /// calls with a different literal — this (single-file) compilation
        /// unit can never see that caller, so it must never seed from the
        /// call sites it happens to see locally.
        #[test]
        fn package_provide_file_disqualifies_every_seed() {
            let reg = registry();
            let src = "
                package provide mylib 1.0
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu.procedures.get("::helper").expect("helper analysed");
            assert!(
                !folds_condition_mentioning(helper, "mode"),
                "a package-providing file must not seed from locally-visible call sites: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// TN control: a plain script with no `package provide` keeps
        /// folding — the guard above must be scoped to the specific
        /// evidence (`package provide` presence), not silently disable the
        /// whole mechanism.
        #[test]
        fn non_package_file_is_unaffected_by_package_provide_guard() {
            let reg = registry();
            let src = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu.procedures.get("::helper").expect("helper analysed");
            assert!(
                folds_condition_mentioning(helper, "mode"),
                "no package-provide in this file, seed should still fold: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FP guard (Codex review, PR #970): `package provide` merely
        /// *mentioned* in a comment must not disable the interprocedural
        /// seed — the guard now checks the lowered IR for a real, resolved
        /// invocation, not a raw-text substring match over the whole file.
        #[test]
        fn package_provide_mentioned_only_in_a_comment_does_not_disqualify() {
            let reg = registry();
            let src = "
                # this file does not package provide anything itself
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu.procedures.get("::helper").expect("helper analysed");
            assert!(
                folds_condition_mentioning(helper, "mode"),
                "a comment merely mentioning the phrase must not disqualify: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// FN guard (Codex review, PR #970): a real `package provide`
        /// invocation must still disqualify the seed even when it's spelled
        /// with unusual whitespace or fully namespace-qualified — cases the
        /// old `source.contains("package provide")` substring check missed.
        #[test]
        fn package_provide_with_unusual_spelling_still_disqualifies() {
            let reg = registry();
            let src = "
                ::package\tprovide mylib 1.0
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }
                }
                helper prod
                helper prod
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let helper = cu.procedures.get("::helper").expect("helper analysed");
            assert!(
                !folds_condition_mentioning(helper, "mode"),
                "a real (if oddly-spelled) package provide must still disqualify: {:?}",
                helper.sccp.constant_branches,
            );
        }

        /// Issue #977 (the two-file repro, verbatim): `lib.tcl` has **no**
        /// `package provide`, so PR #970's guard never fires; its only two
        /// visible callers both pass `"prod"`, so `mode` was seeded as that
        /// constant and the condition folded — even though `main.tcl`
        /// `source`s it and calls `helper dev`.
        mod cross_file {
            use super::*;
            use crate::unit_scope::CallSiteEvidence;

            /// `lib.tcl` from the issue: a plain library file, no `package
            /// provide`, whose in-file callers agree.
            const LIB: &str = "
                proc helper {mode} {
                    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                }
                helper prod
                helper prod
            ";

            /// `main.tcl` from the issue: sources the library and calls its
            /// proc with a literal `lib.tcl` never sees.
            const MAIN: &str = "
                source lib.tcl
                helper dev
            ";

            /// Cross-file evidence a host builds from `other`, resolved
            /// against the procedures `LIB` declares.
            fn evidence_from(other: &str, reg: &CommandRegistry) -> CallSiteEvidence {
                let known: HashSet<String> = ["::helper".to_owned()].into_iter().collect();
                crate::unit_scope::scan_source_call_sites(other, reg, "", &known, &[])
            }

            fn build_with_evidence(
                src: &str,
                reg: &CommandRegistry,
                evidence: &CallSiteEvidence,
            ) -> CompilationUnit {
                CompilationUnit::build_with_options(
                    src,
                    UnitBuildOptions {
                        registry: reg,
                        defer_top_level: false,
                        config: tcl_lexer::LexerConfig::default(),
                        dialect: "",
                        external_call_sites: Some(evidence),
                    },
                )
            }

            /// FP guard — the reported bug. With `main.tcl`'s `helper dev`
            /// merged in, `mode` has two distinct literals and must not fold.
            #[test]
            fn sourcing_file_with_a_differing_literal_retracts_the_fold() {
                let reg = registry();
                let evidence = evidence_from(MAIN, &reg);
                let cu = build_with_evidence(LIB, &reg, &evidence);
                let helper = cu.procedures.get("::helper").expect("helper analysed");
                assert!(
                    !folds_condition_mentioning(helper, "mode"),
                    "main.tcl calls helper with \"dev\"; lib.tcl must not fold on \"prod\": {:?}",
                    helper.sccp.constant_branches,
                );
            }

            /// The evidence itself, before any lattice: the cross-file scan
            /// must attribute `helper dev` to `::helper` at position 0.
            #[test]
            fn cross_file_scan_attributes_the_call_to_the_library_proc() {
                let reg = registry();
                let evidence = evidence_from(MAIN, &reg);
                let helper = evidence.get("::helper").expect("helper call recorded");
                assert_eq!(helper.uniform_literal_at(0), Some("dev"));
            }

            /// TP control — the mechanism must still fold when the other file
            /// agrees. `main.tcl` passing `prod` too leaves one literal
            /// across the whole project, so the seed is sound and fires.
            #[test]
            fn sourcing_file_agreeing_on_the_literal_still_folds() {
                let reg = registry();
                let evidence = evidence_from("source lib.tcl\nhelper prod\n", &reg);
                let cu = build_with_evidence(LIB, &reg, &evidence);
                let helper = cu.procedures.get("::helper").expect("helper analysed");
                assert!(
                    folds_condition_mentioning(helper, "mode"),
                    "every caller in the project passes \"prod\"; the seed is sound: {:?}",
                    helper.sccp.constant_branches,
                );
            }

            /// TN control — a workspace file that never mentions `helper`
            /// contributes no evidence and must not disturb the fold.
            #[test]
            fn unrelated_workspace_file_contributes_nothing() {
                let reg = registry();
                let evidence = evidence_from("proc other {x} { return $x }\nother 1\n", &reg);
                assert!(evidence.is_empty(), "no call to ::helper was written");
                let cu = build_with_evidence(LIB, &reg, &evidence);
                let helper = cu.procedures.get("::helper").expect("helper analysed");
                assert!(
                    folds_condition_mentioning(helper, "mode"),
                    "an unrelated file must not retract a sound fold: {:?}",
                    helper.sccp.constant_branches,
                );
            }

            /// FN guard — a cross-file caller reached through a *dynamic*
            /// argument is still a caller. `helper $mode` in `main.tcl`
            /// poisons position 0, so the fold is retracted even though no
            /// second literal was ever written.
            #[test]
            fn cross_file_dynamic_argument_poisons_the_position() {
                let reg = registry();
                let evidence = evidence_from("set m dev\nhelper $m\n", &reg);
                let cu = build_with_evidence(LIB, &reg, &evidence);
                let helper = cu.procedures.get("::helper").expect("helper analysed");
                assert!(
                    !folds_condition_mentioning(helper, "mode"),
                    "a cross-file caller with a non-literal argument must retract the fold: {:?}",
                    helper.sccp.constant_branches,
                );
            }

            /// FN guard — a cross-file *deferred* caller (`after 0 helper`,
            /// a `-command` callback, `trace add variable`) invokes the proc
            /// with words appended at runtime, so no position is uniform.
            /// The callback slot comes from `ArgRole::CommandPrefix`, not a
            /// command-name list in the compiler.
            #[test]
            fn cross_file_command_prefix_callback_poisons_every_position() {
                let reg = registry();
                let evidence = evidence_from("after 0 helper\n", &reg);
                let cu = build_with_evidence(LIB, &reg, &evidence);
                let helper = cu.procedures.get("::helper").expect("helper analysed");
                assert!(
                    !folds_condition_mentioning(helper, "mode"),
                    "a deferred cross-file callback appends unknown words: {:?}",
                    helper.sccp.constant_branches,
                );
            }

            /// FN guard — a cross-file `rename` moves `helper`'s binding, so
            /// a call reaching it need not be one any scan attributed to it.
            /// `command_mutations` only sees the file being compiled, so
            /// without this the rebinding was invisible across files.
            #[test]
            fn cross_file_rename_poisons_the_callee() {
                let reg = registry();
                let evidence = evidence_from("rename helper legacy_helper\n", &reg);
                let cu = build_with_evidence(LIB, &reg, &evidence);
                let helper = cu.procedures.get("::helper").expect("helper analysed");
                assert!(
                    !folds_condition_mentioning(helper, "mode"),
                    "another file renamed helper; its binding has moved: {:?}",
                    helper.sccp.constant_branches,
                );
            }

            /// A `source`ing file is itself a boundary: `lib.tcl` on its own
            /// (no workspace view) has no boundary at all and still folds —
            /// the documented, accepted limit of a standalone single-file
            /// build — but the *sourcing* file's own procs do not, because
            /// `source` declares `LOADS_EXTERNAL_UNIT`.
            #[test]
            fn a_file_that_sources_another_declines_the_seed_on_its_own() {
                let reg = registry();
                let src = "
                    source lib.tcl
                    proc local {mode} {
                        if {$mode eq \"prod\"} { set r 1 } else { set r 2 }
                    }
                    local prod
                    local prod
                ";
                let cu = CompilationUnit::build_for(src, &reg, false);
                let f = cu.procedures.get("::local").expect("local analysed");
                assert!(
                    !folds_condition_mentioning(f, "mode"),
                    "the sourced file's script can call ::local with anything: {:?}",
                    f.sccp.constant_branches,
                );
                // …and with a workspace view that says otherwise, it folds.
                let empty = CallSiteEvidence::default();
                let closed = build_with_evidence(src, &reg, &empty);
                let f = closed.procedures.get("::local").expect("local analysed");
                assert!(
                    folds_condition_mentioning(f, "mode"),
                    "with the project enumerated, the boundary is no longer a blind spot: {:?}",
                    f.sccp.constant_branches,
                );
            }
        }

        /// FN regression pinned at the `LatticeValue` level (not just
        /// "no constant branch"): the seed for a genuinely-recursive
        /// parameter must never even reach `Const` for the parameter's
        /// version-0 lattice entry, confirming the fix operates at the SCCP
        /// seed itself (the deepest layer), not merely suppressing the
        /// downstream diagnostic.
        #[test]
        fn recursive_param_version_zero_is_never_const_in_lattice() {
            let reg = registry();
            let src = "
                namespace eval ::graph {
                    proc dfs {count} {
                        if {$count & 1} { set p odd } else { set p even }
                        if {$count < 3} { dfs [expr {$count + 1}] }
                    }
                }
                ::graph::dfs 0
            ";
            let cu = CompilationUnit::build_for(src, &reg, false);
            let dfs = cu.procedures.get("::graph::dfs").expect("dfs analysed");
            let sym = dfs
                .ssa
                .var_symbol("count")
                .expect("count should be interned");
            let v0 = dfs.sccp.values.get(&(sym, 0));
            assert!(
                !matches!(v0, Some(LatticeValue::Const(_))),
                "recursive param's version-0 lattice entry must not be Const, got {v0:?}",
            );
        }
    }
}

#[cfg(test)]
mod smoke {
    use tcl_registry::CommandRegistry;

    use super::CompilationUnit;

    /// The canonical "lex -> parse -> segment -> IR -> CFG -> codegen-ready"
    /// pipeline on one tiny script, exercised through the same top-level
    /// entry point every real caller (analyser, optimiser, explorer) uses.
    #[test]
    fn smoke_compile_canonical_snippet() {
        let registry = CommandRegistry::build_default();
        let src = "proc double {n} { return [expr {$n * 2}] }\nset r [double 21]\n";
        let unit = CompilationUnit::build_for(src, &registry, false);
        assert!(
            unit.procedures.contains_key("::double"),
            "expected a lowered ::double procedure, got {:?}",
            unit.procedures.keys().collect::<Vec<_>>()
        );
        assert!(
            !unit.top_level.cfg.blocks.is_empty(),
            "top-level script must lower to a non-empty CFG"
        );
    }
}
