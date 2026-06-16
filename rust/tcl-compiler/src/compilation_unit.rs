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

use std::collections::HashMap;

use tcl_registry::CommandRegistry;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::cfg_builder::build_cfg;
use crate::def_use::{DefUseResult, build_def_use_chains};
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Module as IrModule;
use crate::lowering::lower_to_ir_with_config;
use crate::memory_ssa::{MemorySSAFunction, build_memory_ssa};
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

// ---------------------------------------------------------------------------
// Per-function analysis bundle
// ---------------------------------------------------------------------------

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
    pub memory_ssa: Option<MemorySSAFunction>,
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
        let ssa = build_ssa(&cfg, registry);
        let def_use = build_def_use_chains(&ssa, Some(&cfg));
        let mut sccp = sccp(&cfg, &ssa, param_constants);
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
        let types = propagate_types(&cfg, &ssa, &sccp, registry);
        let return_type =
            crate::type_infer::infer_function_return_type(&cfg, &sccp, &types, registry);
        let rendered_props = propagate_rendered_props(&cfg, &ssa, &sccp, registry);
        let taints = propagate_taints(
            &cfg,
            &ssa,
            &sccp,
            registry,
            Some(&rendered_props),
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
        }
    }

    /// Populate memory-SSA on demand. Returns `self` for chaining.
    #[must_use]
    pub fn with_memory_ssa(mut self) -> Self {
        self.memory_ssa = Some(build_memory_ssa(&self.ssa));
        self
    }
}

// ---------------------------------------------------------------------------
// Module-level compilation unit
// ---------------------------------------------------------------------------

/// Complete compilation artefacts for a source document.
///
/// Built once, consumed many times across the diagnostics cycle.
#[derive(Debug, Clone)]
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
    /// Procedures with interprocedural `param_constants`, the top level, and
    /// methods are always built fresh (no stable offset-0 key); the
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
        let top_level = FunctionUnit::build("::top", cfg_module.top_level.clone(), &[], registry);
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
            let body_offset = proc.map_or(0, |p| p.span.start());
            // Route through the memo only when (a) a cache is present, (b) the
            // procedure has a real body, (c) the module context is available,
            // and (d) there are no interprocedural `param_constants` — those
            // depend on call sites elsewhere in the module, so the body alone
            // does not determine the unit; build those fresh.
            let memoised = match (cache.as_mut(), proc, cfg_context.as_ref()) {
                (Some(memo), Some(proc), Some((upvar_procs, proc_params)))
                    if param_constants.is_none() =>
                {
                    // Normalise the body to offset 0 so a shifted-but-unchanged
                    // procedure produces an identical request (memo hit); rebase
                    // the returned unit back to the procedure's real position.
                    let mut body = proc.body.clone();
                    crate::lattice_rebase::rebase_script(&mut body, -i64::from(body_offset));
                    let mut fu = memo(&LatticeRequest {
                        qname,
                        body: &body,
                        params,
                        upvar_procs,
                        proc_params,
                        dialect,
                    });
                    crate::lattice_rebase::rebase_function_unit(&mut fu, i64::from(body_offset));
                    Some(fu)
                }
                _ => None,
            };
            let fu = memoised.unwrap_or_else(|| {
                FunctionUnit::build_with_param_constants(
                    qname,
                    cfg.clone(),
                    params,
                    registry,
                    param_constants.as_ref(),
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
                    (
                        mqname.clone(),
                        FunctionUnit::build(mqname, cfg, &method.params, registry),
                    )
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
            );
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
}

/// Per-arg-position call-site literal evidence for one callee.
#[derive(Default)]
struct ArgConsts {
    /// At least one call passed a non-literal (`$`/`[`) value here.
    unknown: bool,
    /// Distinct literal values seen at this position.
    values: std::collections::HashSet<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
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
