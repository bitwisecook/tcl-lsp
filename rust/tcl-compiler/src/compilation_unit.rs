//! Shared compilation artefacts for a single source document.
//!
//! Built once per diagnostics cycle, consumed by the analyser,
//! optimiser, shimmer analysis, taint engine, and compiler checks.
//!
//! Ported from `core/compiler/compilation_unit.py` (C31). This
//! strip lands the [`CompilationUnit`] / [`FunctionUnit`] facade
//! types and the `build_for` entry point that drives the landed
//! pipeline (lower → CFG → SSA → def-use → SCCP). Heavier
//! analyses (interprocedural, memory-SSA, execution-intent,
//! rendered-properties) plug in through accessor methods that
//! return `Option<&T>` — `None` when the analysis hasn't been
//! run on this unit yet.
//!
//! The Python facade also owns class-name extraction and
//! connection-scope analysis; those are follow-ups.

use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::cfg_builder::build_cfg;
use crate::def_use::{DefUseResult, build_def_use_chains};
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Module as IrModule;
use crate::lowering::lower_to_ir_with_config;
use crate::memory_ssa::{MemorySsaFunction, build_memory_ssa};
use crate::rendered_properties::{RenderedValueProps, propagate_rendered_props};
use crate::sccp::{SccpResult, sccp};
use crate::ssa::{SsaFunction, ValueKey, build_ssa};
use crate::taint::{TaintLattice, propagate_taints};
use crate::type_infer::propagate_types;
use crate::types::TypeLattice;

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
    /// consistently). Mirrors Python's `FunctionUnit.complexity_guarded`.
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
            &std::collections::HashMap<crate::ssa::ValueKey, crate::analyses::LatticeValue>,
        >,
    ) -> Self {
        Self::build_with_param_constants_and_classes(
            name,
            cfg,
            params,
            registry,
            param_constants,
            &HashSet::new(),
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
    #[must_use]
    pub fn build_with_param_constants_and_classes(
        name: impl Into<String>,
        cfg: CfgFunction,
        params: &[String],
        registry: &CommandRegistry,
        param_constants: Option<
            &std::collections::HashMap<crate::ssa::ValueKey, crate::analyses::LatticeValue>,
        >,
        known_classes: &HashSet<String>,
    ) -> Self {
        // Complexity guard (block-count half): a pathologically large body
        // would cost seconds of SSA + dataflow for near-zero findings, so skip
        // the deep analysis and flag the unit. The body-byte half is applied by
        // the callers that have the body span (see `build_for_with_config`).
        // Backstop for every path through here — `build`, methods, and the
        // salsa `function_lattice` callbacks. Mirrors Python's `force_guard or
        // is_complexity_guarded` short-circuit in `analyse_function`.
        if crate::ssa::is_complexity_guarded(&cfg) {
            return Self::trivial_guarded(name, cfg);
        }
        let ssa = build_ssa(&cfg, registry);
        let def_use = build_def_use_chains(&ssa, Some(&cfg));
        // The registry encodes the analysis dialect's Tcl version, which fixes
        // how a bare leading-zero literal (`08`, `010`) is read when SCCP folds
        // `==`/`!=` (octal in tcl8.x / F5 / EDA, decimal in tcl9.0).
        let mut sccp = sccp(
            &cfg,
            &ssa,
            param_constants,
            Some(registry.leading_zero_is_octal()),
        );
        // SYNC-MAY31-3: surface `[info exists X]` / `[array exists X]`
        // folds (parameter → exists, never-defined non-param → absent)
        // as constant branches so the optimiser's O101 fold / DCE sees
        // them. The analyser's I230 uses the same fold via
        // `existence_constant_branches`; the SCCP pass proper has no
        // parameter/existence facts to fold them itself.
        let param_set: std::collections::HashSet<&str> =
            params.iter().map(String::as_str).collect();
        sccp.constant_branches
            .extend(crate::sccp::existence_constant_branches(&cfg, &param_set));
        let types = propagate_types(&cfg, &ssa, &sccp, registry, known_classes);
        let return_type = crate::type_infer::infer_function_return_type(
            &cfg,
            &sccp,
            &types,
            registry,
            known_classes,
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
        let ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry.clone());
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
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        tcl_lexer::Span::new(s as u32, e as u32)
    }

    /// Recover the absolute byte position of `pos` by adding
    /// [`Self::base_offset`] (see [`Self::abs_span`]).
    #[must_use]
    pub fn abs_pos(&self, pos: u32) -> u32 {
        if self.base_offset == 0 {
            return pos;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            (i64::from(pos) + self.base_offset).max(0) as u32
        }
    }

    /// Populate memory-SSA on demand. Returns `self` for chaining.
    #[must_use]
    pub fn with_memory_ssa(mut self) -> Self {
        self.memory_ssa = Some(build_memory_ssa(&self.ssa));
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
    /// `TclOO` method bodies lowered to per-method [`FunctionUnit`]s
    /// (SF-2). Keyed by `{class_qname}::{method_name}`; empty for
    /// non-OO sources. Kept separate from [`Self::procedures`] so the
    /// per-proc diagnostic passes are unaffected — only the optimiser's
    /// O126 gate iterates these. Mirrors Python's
    /// `CompilationUnit.methods`.
    pub methods: HashMap<String, FunctionUnit>,
    /// Interprocedural summary (optional — populated when the
    /// interprocedural pass has been run).
    pub interproc: Option<InterproceduralAnalysis>,
    /// Cross-event variable scope analysis.  ``Some`` when at
    /// least one ``::when::*`` procedure is in
    /// [`Self::procedures`]; ``None`` for non-iRules sources or
    /// any source with no ``when`` blocks.  Mirrors Python's
    /// ``CompilationUnit.connection_scope``.  Lands alongside
    /// **C41d7** (the IRULE4005 emitter).
    pub connection_scope: Option<crate::connection_scope::ConnectionScope>,
}

impl CompilationUnit {
    /// Build a [`CompilationUnit`] by running the landed pipeline
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
    /// document's dialect (`SYNC-MAY19-dialect-contextvar`, strip 3).
    #[must_use]
    pub fn build_for_with_config(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        config: tcl_lexer::LexerConfig,
    ) -> Self {
        Self::build_for_inner(source, registry, defer_top_level, config, "", None)
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
        )
    }

    #[allow(clippy::too_many_lines)]
    fn build_for_inner(
        source: &str,
        registry: &CommandRegistry,
        defer_top_level: bool,
        config: tcl_lexer::LexerConfig,
        dialect: &str,
        mut cache: Option<&mut ProcLatticeCache<'_>>,
    ) -> Self {
        let mut ir_module = lower_to_ir_with_config(source, registry, config);
        // C36d/e/f: specialise Option-shape factories before any
        // other module-level passes so the synthesised child procs
        // appear in module.procedures for the inline_uplevel pass
        // and CFG construction.
        crate::specialise_factories::specialise_factories(&mut ir_module, registry);
        // C34e: run the inline_uplevel pass before CFG construction so
        // every passthrough callsite is replaced with a Statement::Block
        // that splices the body inline.
        crate::inline_uplevel::inline_uplevel_passthrough(&mut ir_module, registry);
        let cfg_module = build_cfg(&ir_module, defer_top_level);
        // D3-P2: collect call-site literal arg values per user proc so each
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
        let top_level = FunctionUnit::build_with_param_constants_and_classes(
            "::top",
            cfg_module.top_level.clone(),
            &[],
            registry,
            None,
            &known_class_set,
        );
        // Module-wide upvar/param context — the CFG-determining context a
        // procedure body is rebuilt under.  Computed once and shared by every
        // memoised request (and the methods below), so the offset-0 CFG the
        // memo rebuilds is identical to this whole-module build's.  Only needed
        // on the memoised path or when methods are present.
        let cfg_context = (cache.is_some() || !ir_module.methods.is_empty())
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
            // what catches it. Mirrors Python's `byte_guarded ||
            // is_complexity_guarded(cfg)` in the compilation-unit build.
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
                (Some(memo), Some(proc), Some((upvar_procs, proc_params)), Some(encoded_pc)) => {
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
                        dialect,
                        param_constants: &encoded_pc,
                        known_classes: &known_classes,
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
                )
            });
            procedures.insert(qname.clone(), fu);
        }
        // SF-2: lower TclOO method bodies (populated in
        // `ir_module.methods` by lowering) to per-method
        // `FunctionUnit`s, using the same CFG → SSA → analysis
        // pipeline as procs. Kept in a separate map so the per-proc
        // diagnostic passes (which iterate `procedures`) are
        // unaffected — only the interproc purity summary and the O126
        // optimiser gate consume methods. Gated on a non-empty method
        // set so non-OO sources skip the upvar-context scan entirely.
        let methods: HashMap<String, FunctionUnit> = if ir_module.methods.is_empty() {
            HashMap::new()
        } else {
            let (upvar_procs, proc_params) = cfg_context
                .as_ref()
                .expect("cfg_context computed when methods are present");
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
                            &known_class_set,
                        )
                    };
                    (mqname.clone(), fu)
                })
                .collect()
        };
        // **C41d7.** Build the cross-event scope from the
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
        Self {
            source: source.to_owned(),
            ir_module,
            cfg_module,
            top_level,
            procedures,
            methods,
            interproc: None,
            connection_scope,
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
        let interproc = crate::interprocedural::build_interprocedural_analysis(
            &self.ir_module,
            registry,
            dialect,
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
        let interproc = crate::interprocedural::build_interprocedural_analysis(
            &self.ir_module,
            registry,
            dialect,
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
    pub fn with_memory_ssa(mut self) -> Self {
        self.top_level = self.top_level.with_memory_ssa();
        let mut out: HashMap<String, FunctionUnit> = HashMap::with_capacity(self.procedures.len());
        for (k, fu) in self.procedures.drain() {
            out.insert(k, fu.with_memory_ssa());
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

    /// Like [`Self::functions`] but skips the functions the complexity guard
    /// excluded from deep analysis (oversized bodies). Per-proc diagnostic and
    /// optimiser passes iterate this so a guarded body — whose `ssa` and
    /// dataflow lattices are trivial — contributes no findings and costs no
    /// CFG walk. The relative order of the remaining functions is unchanged.
    pub fn analysable_functions(&self) -> impl Iterator<Item = &FunctionUnit> {
        self.functions().filter(|fu| !fu.complexity_guarded)
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
    if !source.contains("class") {
        return HashSet::new();
    }
    crate::signature_scan::extract_signatures(source, registry)
        .classes
        .into_keys()
        .collect()
}

/// Collect literal arg values per user-proc call site across the whole
/// module's CFGs (top-level + every proc body, statements already flattened).
/// Mirrors `_collect_call_site_constants`.  Returns
/// `{callee_qname -> {arg_index -> ArgConsts}}`.
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
                // call site can't bind it.  Mirrors the intent of treating
                // omitted args as `_UNKNOWN_ARG`.
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
/// literal at that position.  Mirrors `_params_constants_from_call_sites`.
fn params_constants_from_call_sites(
    params: &[String],
    call_site_constants: &HashMap<String, HashMap<usize, ArgConsts>>,
    qname: &str,
) -> Option<HashMap<crate::ssa::ValueKey, crate::analyses::LatticeValue>> {
    use crate::analyses::{ConstValue, LatticeValue};
    let by_idx = call_site_constants.get(qname)?;
    let mut consts: HashMap<crate::ssa::ValueKey, LatticeValue> = HashMap::new();
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
    param_constants: Option<&HashMap<crate::ssa::ValueKey, crate::analyses::LatticeValue>>,
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
) -> Option<HashMap<crate::ssa::ValueKey, crate::analyses::LatticeValue>> {
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
        let (_, proc_params) = crate::cfg_builder::prepare_cfg_context(&m);
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
        // SF-2 (SYNC-JUN02-1): TclOO method bodies get their own
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
        // SYNC-MAY31-7: the `switch -- $col` subject lowers to an
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
        // SYNC-MAY31-2: a `switch -glob`/`-regexp` arm body is now a
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
        let cu = CompilationUnit::build_for("set x 1", &registry(), false).with_memory_ssa();
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
