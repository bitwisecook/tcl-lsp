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
//! SSA → def-use → SCCP). Heavier analyses (interprocedural,
//! memory-SSA, execution-intent, rendered-properties) plug in through
//! accessor methods that return `Option<&T>` — `None` when the analysis
//! hasn't been run on this unit yet.

use std::collections::{BTreeSet, HashMap, HashSet};

use tcl_registry::CommandRegistry;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::cfg_builder::build_cfg;
use crate::def_use::{DefUseResult, build_def_use_chains};
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Module as IrModule;
use crate::lowering::lower_to_ir_with_config;
use crate::memory_ssa::{MemorySsaFunction, build_memory_ssa};
use crate::rendered_properties::{RenderedValueProps, propagate_rendered_props};
use crate::sccp::{SccpResult, sccp_with_extra_escaping};
use crate::ssa::{SsaFunction, ValueKey, build_ssa};
use crate::taint::{TaintLattice, propagate_taints};
use crate::type_infer::propagate_types;
use crate::types::TypeLattice;

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

/// Callback type for [`CompilationUnit::with_interprocedural_memoized`].
///
/// Given a procedure's qualified name and the whole-module
/// [`InterproceduralAnalysis`], returns its (memoised) interprocedural taints,
/// or `None` to fall back to a fresh [`FunctionUnit::interproc_taints`] re-run.
pub type TaintCascadeCallback<'a> =
    dyn FnMut(&str, &InterproceduralAnalysis) -> Option<HashMap<ValueKey, TaintLattice>> + 'a;

// Per-function analysis bundle

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
    pub def_use: DefUseResult,
    /// SCCP result: lattice values, executable blocks, constant
    /// branches.
    pub sccp: SccpResult,
    /// Type lattice values per SSA definition.
    ///
    /// Computed by the type-propagation pass. Absent entries are
    /// implicitly `TypeLattice::unknown()`.
    pub types: HashMap<ValueKey, TypeLattice>,
    /// Inferred return type — the join of the types produced at every
    /// executable `Return` terminator.  `Unknown` when the function
    /// has no executable return value.  Computed by
    /// [`crate::type_infer::infer_function_return_type`].
    pub return_type: TypeLattice,
    /// Taint lattice values per SSA definition.
    ///
    /// Computed by the intra-procedural taint-propagation pass.
    /// Absent entries are implicitly clean (untainted).
    pub taints: HashMap<ValueKey, TaintLattice>,
    /// Rendered-string-property lattice values per SSA definition.
    ///
    /// Computed by `propagate_rendered_props`. Absent entries are
    /// implicitly `RenderedValueProps::bottom()`.
    pub rendered_props: HashMap<ValueKey, RenderedValueProps>,
    /// Optional memory-SSA annotations (populated on demand).
    pub memory_ssa: Option<MemorySsaFunction>,
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
            },
        )
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
        // The registry encodes the analysis dialect's Tcl version, which fixes
        // how a bare leading-zero literal (`08`, `010`) is read when SCCP folds
        // `==`/`!=` (octal in tcl8.x / F5 / EDA, decimal in tcl9.0).
        let mut sccp = sccp_with_extra_escaping(
            &cfg,
            &ssa,
            param_constants,
            Some(registry.leading_zero_is_octal()),
            extra_global_escaping,
            crate::sccp::TraceInputs {
                registry,
                traced_variables: trace_facts.traced_variables,
                has_dynamic_variable_trace: trace_facts.has_dynamic_variable_trace,
            },
        );
        // Surface `[info exists X]` / `[array exists X]`
        // folds (parameter → exists, never-defined non-param → absent)
        // as constant branches so the optimiser's O101 fold / DCE sees
        // them. The analyser's I230 uses the same fold via
        // `existence_constant_branches`; the SCCP pass proper has no
        // parameter/existence facts to fold them itself.
        let param_set: std::collections::HashSet<&str> =
            params.iter().map(String::as_str).collect();
        sccp.constant_branches
            .extend(crate::sccp::existence_constant_branches(&cfg, &param_set));
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
            &cfg,
            &ssa,
            &sccp,
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
            def_use,
            sccp,
            types,
            return_type,
            taints,
            rendered_props,
            memory_ssa: None,
            complexity_guarded: false,
            base_offset: 0,
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
            def_use: DefUseResult::default(),
            sccp: SccpResult::default(),
            types: HashMap::new(),
            return_type: TypeLattice::unknown(),
            taints: HashMap::new(),
            rendered_props: HashMap::new(),
            memory_ssa: None,
            complexity_guarded: true,
            base_offset: 0,
        }
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

    /// Populate memory-SSA on demand. Returns `self` for chaining.
    #[must_use]
    pub fn with_memory_ssa(mut self, registry: &tcl_registry::CommandRegistry) -> Self {
        self.memory_ssa = Some(build_memory_ssa(&self.ssa, registry));
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
            &self.cfg,
            &self.ssa,
            &self.sccp,
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
    /// (`::apply#N`, `::namespace-eval::NS#N`). Kept **separate** from both
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
    /// Lowers with the default (Tcl-8.5+) lexer config; use
    /// [`Self::build_for_with_config`] to honour a document's dialect.
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
    #[must_use]
    pub fn build_for_with_config(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        Self::build_for_inner(source, registry, defer_top_level, config, "", None, None)
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
        registry: &CommandRegistry,
        defer_top_level: bool,
        config: tcl_lexer::LexerConfig,
        dialect: &str,
        cache: &mut ProcLatticeCache<'_>,
    ) -> Self {
        Self::build_for_inner(
            source,
            registry,
            defer_top_level,
            config,
            dialect,
            Some(cache),
            None,
        )
    }

    /// Like [`Self::build_for_memoized`] but also threads a memoised per-procedure
    /// **body-lowering** callback (SRV-INCREMENTAL Task 3) into the lowering phase,
    /// so an unchanged top-level proc's body IR is reused across edits.  The caller
    /// installs it only for context-free files (see [`crate::lowering::Lowerer`]'s
    /// `body_cache`); byte-identity is guarded by the corpus differential gates.
    pub fn build_for_memoized_with_body_cache(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        config: tcl_lexer::LexerConfig,
        dialect: &str,
        cache: &mut ProcLatticeCache<'_>,
        body_cache: &dyn Fn(&str, &str) -> crate::ir::Script,
    ) -> Self {
        Self::build_for_inner(
            source,
            registry,
            defer_top_level,
            config,
            dialect,
            Some(cache),
            Some(body_cache),
        )
    }

    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::type_complexity
    )]
    fn build_for_inner(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        config: tcl_lexer::LexerConfig,
        dialect: &str,
        mut cache: Option<&mut ProcLatticeCache<'_>>,
        body_cache: Option<&dyn Fn(&str, &str) -> crate::ir::Script>,
    ) -> Self {
        let mut ir_module = match body_cache {
            Some(bc) => crate::lowering::lower_to_ir_with_body_cache(source, registry, config, bc),
            None => lower_to_ir_with_config(source, registry, config),
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
        let cfg_module = build_cfg(&ir_module, defer_top_level);
        // Collect call-site literal arg values per user proc so each
        // callee's SCCP can fold a param every caller passes the same literal
        // for (interprocedural constant propagation).
        let call_site_constants = collect_call_site_constants(&cfg_module, &ir_module.procedures);
        // Fully-qualified names of every class defined in the unit, sourced from
        // the signature scanner so this build and the incremental/db build
        // derive an identical set (⇒ identical OBJECT-constructor typing on both
        // paths).  `known_classes` (sorted) is also what each `LatticeRequest`
        // carries into the memo key, so a class-set change invalidates the
        // per-procedure lattices.
        let known_class_set = collect_known_classes(source, registry);
        let known_classes: Vec<String> = {
            let mut v: Vec<String> = known_class_set.iter().cloned().collect();
            v.sort_unstable();
            v
        };
        // Every name any procedure/method declares via `global NAME` —
        // top-level bare names already live in the global frame, so a call
        // to such a procedure can reassign one mid-run even though the
        // top-level body never mentions it via `global` itself. See
        // `crate::var_observability::scan_module_global_names`.
        let top_level_extra_escaping =
            crate::var_observability::scan_module_global_names(&ir_module);
        // Whole-module variable-trace fact — computed once by lowering
        // and stored on `ir_module`, so every per-function build below is a
        // cheap reference pass-through, not a recomputation. `traced_variable_names`
        // is the `Vec` form (already sorted — `BTreeSet` iterates in order)
        // `LatticeRequest`'s memo key carries, mirroring `known_classes`.
        let trace_facts = ModuleTraceFacts {
            traced_variables: &ir_module.traced_variables,
            has_dynamic_variable_trace: ir_module.has_dynamic_variable_trace,
        };
        let traced_variable_names: Vec<String> =
            ir_module.traced_variables.iter().cloned().collect();
        let top_level = FunctionUnit::build_full(
            "::top",
            cfg_module.top_level.clone(),
            FunctionBuildInputs {
                params: &[],
                registry,
                param_constants: None,
                known_classes: &known_class_set,
                extra_global_escaping: &top_level_extra_escaping,
                trace_facts,
            },
        );
        // Module-wide upvar/param context — the CFG-determining context a
        // procedure body is rebuilt under.  Computed once and shared by every
        // memoised request (and the methods below), so the offset-0 CFG the
        // memo rebuilds is identical to this whole-module build's.  Only needed
        // on the memoised path or when methods are present.
        let cfg_context =
            (cache.is_some() || !ir_module.methods.is_empty() || !ir_module.body_units.is_empty())
                .then(|| crate::cfg_builder::prepare_cfg_context(&ir_module));
        let mut procedures: HashMap<String, FunctionUnit> = HashMap::new();
        for (qname, cfg) in &cfg_module.procedures {
            let params = ir_module
                .procedures
                .get(qname)
                .map_or(&[][..], |p| p.params.as_slice());
            let param_constants =
                params_constants_from_call_sites(params, &call_site_constants, qname);
            let proc = ir_module.procedures.get(qname);
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
            // Route through the memo only when (a) a cache is present, (b) the
            // procedure has a real body, (c) the module context is available,
            // and (d) the seeds encode into the hashable key form.
            let memoised = match (cache.as_mut(), proc, cfg_context.as_ref(), encoded_pc) {
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
                        dialect,
                        param_constants: &encoded_pc,
                        known_classes: &known_classes,
                        traced_variables: &traced_variable_names,
                        has_dynamic_variable_trace: ir_module.has_dynamic_variable_trace,
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
            let fu = memoised.unwrap_or_else(|| {
                FunctionUnit::build_with_param_constants_and_classes(
                    qname,
                    cfg.clone(),
                    params,
                    registry,
                    param_constants.as_ref(),
                    &known_class_set,
                    trace_facts,
                )
            });
            procedures.insert(qname.clone(), fu);
        }
        let methods = Self::build_method_units(
            &ir_module,
            cfg_context.as_ref(),
            &known_class_set,
            registry,
            trace_facts,
        );
        let body_units = Self::build_body_units(
            &ir_module,
            cfg_context.as_ref(),
            &known_class_set,
            registry,
            trace_facts,
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
    ) -> HashMap<String, FunctionUnit> {
        if ir_module.methods.is_empty() {
            return HashMap::new();
        }
        let (upvar_procs, proc_params, global_write_procs) =
            cfg_context.expect("cfg_context computed when methods are present");
        ir_module
            .methods
            .iter()
            .map(|(mqname, method)| {
                let cfg = crate::cfg_builder::build_cfg_function_with_upvars(
                    mqname,
                    &method.body,
                    true,
                    upvar_procs.clone(),
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
                    FunctionUnit::trivial_guarded(mqname, cfg)
                } else {
                    FunctionUnit::build_with_param_constants_and_classes(
                        mqname,
                        cfg,
                        &method.params,
                        registry,
                        None,
                        known_class_set,
                        trace_facts,
                    )
                };
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
                };
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
        let interproc = crate::interprocedural::build_interprocedural_analysis(
            &self.ir_module,
            registry,
            dialect,
            crate::interprocedural::ObjectTypeMap(&object_types),
        );

        // Re-run taint with the new summary + dialect. We borrow
        // `interproc` immutably while each function unit re-runs
        // `propagate_taints`.
        self.top_level.taints = propagate_taints(
            &self.top_level.cfg,
            &self.top_level.ssa,
            &self.top_level.sccp,
            registry,
            Some(&self.top_level.rendered_props),
            Some(&interproc),
            dialect,
            None,
            None,
        );
        for fu in self.procedures.values_mut() {
            fu.taints = propagate_taints(
                &fu.cfg,
                &fu.ssa,
                &fu.sccp,
                registry,
                Some(&fu.rendered_props),
                Some(&interproc),
                dialect,
                None,
                None,
            );
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
        let interproc = crate::interprocedural::build_interprocedural_analysis(
            &self.ir_module,
            registry,
            dialect,
            crate::interprocedural::ObjectTypeMap(&object_types),
        );

        // Top level is built fresh (no offset-0 lattice key), so its taint
        // re-run stays inline.
        let top_taints = self
            .top_level
            .interproc_taints(registry, &interproc, dialect);
        self.top_level.taints = top_taints;

        // Compute each procedure's taints (memoised via `taint_cb`, or fresh on a
        // miss) before mutating, so the immutable `&self.procedures` borrow the
        // fallback needs doesn't overlap the write-back.
        let mut new_taints: Vec<(String, HashMap<ValueKey, TaintLattice>)> =
            Vec::with_capacity(self.procedures.len());
        for (qname, fu) in &self.procedures {
            let taints = taint_cb(qname, &interproc)
                .unwrap_or_else(|| fu.interproc_taints(registry, &interproc, dialect));
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

    /// Populate memory-SSA on the top-level and every procedure.
    #[must_use]
    pub fn with_memory_ssa(mut self, registry: &tcl_registry::CommandRegistry) -> Self {
        self.top_level = self.top_level.with_memory_ssa(registry);
        let mut out: HashMap<String, FunctionUnit> = HashMap::with_capacity(self.procedures.len());
        for (k, fu) in self.procedures.drain() {
            out.insert(k, fu.with_memory_ssa(registry));
        }
        self.procedures = out;
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
    /// `tcl-lsp-db::proc_taint_solve`'s shimmer-family top-up loop: that
    /// memoised query's main per-function loop still iterates
    /// [`Self::analysable_functions`] (proc-only), unlike
    /// `compiler_checks::run_all_checks_with_solved_and_patterns`'s direct
    /// path, which iterates the wider [`Self::all_body_function_units`]
    /// (filtered) and so needs no separate top-up — adding this iterator's
    /// output there too would re-run `shimmer_family_checks` a second time
    /// over the methods/body units `function_nontaint_checks` already covers.
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

/// Per-arg-position call-site literal evidence for one callee.
#[derive(Default)]
struct ArgConsts {
    /// At least one call passed a non-literal (`$`/`[`) value here.
    unknown: bool,
    /// Distinct literal values seen at this position.
    values: std::collections::HashSet<String>,
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

/// Collect literal arg values per user-proc call site across the whole
/// module's CFGs (top-level + every proc body, statements already flattened).
/// Returns `{callee_qname -> {arg_index -> ArgConsts}}`.
fn collect_call_site_constants(
    cfg_module: &CfgModule,
    procedures: &HashMap<String, crate::ir::Procedure>,
) -> HashMap<String, HashMap<usize, ArgConsts>> {
    use crate::ir::Statement;
    let mut out: HashMap<String, HashMap<usize, ArgConsts>> = HashMap::new();
    // Resolve a call command word to a user-proc qualified name.
    let resolve = |cmd: &str| -> Option<String> {
        for cand in [
            cmd.to_string(),
            format!("::{cmd}"),
            format!("::{}", cmd.trim_start_matches(':')),
        ] {
            let qn = if cand.starts_with("::") {
                cand
            } else {
                format!("::{cand}")
            };
            if procedures.contains_key(&qn) {
                return Some(qn);
            }
        }
        None
    };
    let funcs = std::iter::once(&cfg_module.top_level).chain(cfg_module.procedures.values());
    for func in funcs {
        for block in func.blocks.values() {
            for stmt in &block.statements {
                let Statement::Call { command, args, .. } = stmt else {
                    continue;
                };
                let Some(target) = resolve(command.as_str()) else {
                    continue;
                };
                // A call that omits a (defaulted) parameter uses its default,
                // an unknown value at that slot — poison every param position
                // this call doesn't provide so a single literal at another
                // call site can't bind it (omitted args are treated as
                // unknown).
                let nparams = procedures.get(&target).map_or(0, |p| p.params.len());
                let by_idx = out.entry(target).or_default();
                for (i, arg) in args.iter().enumerate() {
                    let slot = by_idx.entry(i).or_default();
                    if arg.contains(['$', '[']) {
                        slot.unknown = true;
                    } else {
                        slot.values.insert(arg.clone());
                    }
                }
                for i in args.len()..nparams {
                    by_idx.entry(i).or_default().unknown = true;
                }
            }
        }
    }
    out
}

/// Build the SCCP `param_constants` seed for `qname` from collected call-site
/// literals: bind `(param, 0)` only when every caller passes the same single
/// literal at that position.
fn params_constants_from_call_sites(
    params: &[String],
    call_site_constants: &HashMap<String, HashMap<usize, ArgConsts>>,
    qname: &str,
) -> Option<HashMap<(String, crate::ssa::Version), crate::analyses::LatticeValue>> {
    use crate::analyses::{ConstValue, LatticeValue};
    let by_idx = call_site_constants.get(qname)?;
    let mut consts: HashMap<(String, crate::ssa::Version), LatticeValue> = HashMap::new();
    for (i, pname) in params.iter().enumerate() {
        if pname == "args" {
            break;
        }
        let Some(slot) = by_idx.get(&i) else {
            continue;
        };
        if slot.unknown || slot.values.len() != 1 {
            continue;
        }
        let val = slot.values.iter().next().unwrap().clone();
        consts.insert(
            (pname.clone(), 0),
            LatticeValue::Const(ConstValue::String(val)),
        );
    }
    if consts.is_empty() {
        None
    } else {
        Some(consts)
    }
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
        let m = lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::default());
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
        let cu =
            CompilationUnit::build_for("set x 1", &registry(), false).with_memory_ssa(&registry());
        assert!(cu.top_level.memory_ssa.is_some());
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
}
