//! Optimiser passes — source-rewrite suggestions produced by
//! analysing the compiled IR / CFG / SSA.
//!
//! Ported from `core/compiler/optimiser/` (C30 + follow-up sub-strips
//! C30a … C30j and the per-pass `*-final` / `*-remainder` strips
//! tracked in `docs/rust-rewrite.md`).  This module owns the shared
//! optimiser types (`Optimisation`, `PassContext`, opt priorities,
//! `PassId`, `run_passes`) plus per-pass submodules.  Every pass body
//! is now landed:
//!
//! - [`branch_folding`] — constant branch folding (O101) +
//!   `propagate_into_branches` cascade (`substitute` →
//!   `strength_reduce` → `strlen` → `streq` → `instcombine`).
//! - [`code_sinking`] — O125 sink-set rewrites (paired delete +
//!   insert, falls back to hint-only when the original span is
//!   local-offset).
//! - [`elimination`] — O107 unreachable-block, O108 ADCE, O109 dead
//!   stores, O126 unused-variable assignments.
//! - [`expr_simplify`] — O101 fold + O115 redundant-`expr` unwrap +
//!   the four AST-level rewriters (instcombine / strength reduction /
//!   strlen-simplify / streq-simplify) in [`helpers::expr_simplify`].
//! - [`pattern_recognition`] — O114 incr-idiom + O119 multi-set
//!   packing hint + O104 string-build chain hint (canonical
//!   allocation per `docs/generated/optimisation_codes.md`).
//! - [`propagation`] — O100 var-ref + O100 string-interpolation +
//!   O102 load-forward + O103 static-proc-call folding + O100
//!   return-terminator folding.
//! - [`structure_elimination`] — O112 if/while/for/switch elimination.
//! - [`tail_call`] — O121 bare + return-subst variants, O122 loop hint,
//!   O123 accumulator-candidate hint.
//! - [`unused_procs`] — O124 iRules unreachable-proc commenting.
//! - [`manager`] — `optimise` / `optimise_with_dialect` / `optimise_raw`
//!   façade that runs every `PassId::all()` in canonical order.

pub mod branch_folding;
pub mod code_sinking;
pub mod elimination;
pub mod expr_simplify;
pub mod helpers;
pub mod manager;
pub mod pattern_recognition;
pub mod propagation;
pub mod structure_elimination;
pub mod tail_call;
pub mod unused_procs;

pub use manager::{optimise, optimise_raw, optimise_with_dialect};

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;

use crate::compilation_unit::CompilationUnit;
use crate::interprocedural::InterproceduralAnalysis;
use crate::ir::Module as IrModule;
use crate::ssa::ValueKey;

// ---------------------------------------------------------------------------
// Optimisation diagnostic
// ---------------------------------------------------------------------------

/// A suggested source rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Optimisation {
    /// Diagnostic code (`"O100"` … `"O127"`).
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Source span the rewrite targets.
    pub span: Span,
    /// Suggested replacement text.
    pub replacement: String,
    /// Group id — optimisations in the same group are applied
    /// together (all-or-nothing).
    pub group: Option<u32>,
    /// Hint-only (informational) rather than actionable.
    pub hint_only: bool,
}

impl Optimisation {
    /// Build an actionable optimisation with default group and
    /// `hint_only` flag.
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        span: Span,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            span,
            replacement: replacement.into(),
            group: None,
            hint_only: false,
        }
    }
}

/// Return the display priority for a given optimisation code.
///
/// Higher priorities are displayed first in editor quick-fix
/// menus. Unknown codes default to `0`.
#[must_use]
pub fn opt_priority(code: &str) -> u8 {
    match code {
        "O126" | "O124" => 10,
        "O112" => 9,
        "O109" => 8,
        "O108" => 7,
        "O107" | "O122" => 6,
        "O121" | "O123" | "O125" | "O119" | "O120" | "O118" | "O117" | "O116" | "O115" | "O114"
        | "O113" | "O110" => 5,
        "O104" => 4,
        "O103" => 3,
        "O102" => 2,
        "O101" => 1,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Pass context
// ---------------------------------------------------------------------------

/// Qualified proc name → `(cfg_function_name, parameter names)`.
///
/// Mirrors the Python `proc_cfgs` mapping — the optimiser reaches
/// for this when it needs to fold a call across procedure
/// boundaries (`_propagation` → `_substitute_expr_proc_calls`).
/// The value is the pair `(qualified_cfg_key, params)`; callers
/// look the CFG up in a [`CompilationUnit`] by name.
pub type ProcCfgs = HashMap<String, ProcCfgEntry>;

/// Value of a [`ProcCfgs`] entry — the qualified CFG key plus the
/// declared parameter names, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcCfgEntry {
    /// Qualified CFG key (same string used to look the procedure
    /// up in [`CompilationUnit::procedures`](crate::compilation_unit::CompilationUnit::procedures)).
    pub cfg_key: String,
    /// Declared parameter names in positional order.
    pub params: Vec<String>,
}

/// Shared mutable state threaded through all optimisation passes.
///
/// The state mirrors the Python `PassContext` field-for-field so
/// passes ported straight from `core/compiler/optimiser/` can
/// consult the same bookkeeping:
///
/// - [`optimisations`](Self::optimisations) — accumulated rewrites.
/// - [`interproc`](Self::interproc) — procedure + method summaries.
/// - [`proc_cfgs`](Self::proc_cfgs) — per-proc CFG + params map used
///   by propagation to fold calls.
/// - [`propagated_branch_uses`](Self::propagated_branch_uses) —
///   `(var, ssa_version)` pairs whose branch use has been
///   propagated, so `_elimination` can drop dead stores feeding
///   those uses.
/// - [`propagated_use_groups`](Self::propagated_use_groups) —
///   group-id assignment for propagated uses, enabling
///   all-or-nothing application of the associated rewrite group.
/// - [`propagated_expr_stmts`](Self::propagated_expr_stmts) —
///   `(block_name, stmt_index)` of propagated expression
///   statements, again to coordinate with `_elimination`.
/// - [`cross_event_vars`](Self::cross_event_vars) — names whose
///   values must be preserved across `when <event>` boundaries
///   (iRules-specific; empty for plain Tcl).
/// - [`next_group`](Self::next_group) — group-id allocator backing
///   [`alloc_group`](Self::alloc_group).
/// - [`ir_module`](Self::ir_module) — structured IR, kept so
///   passes can walk the pre-CFG tree when they need original
///   source positions.
#[derive(Debug, Default)]
pub struct PassContext<'a> {
    /// Full source text (UTF-8).
    pub source: &'a str,
    /// Tcl dialect currently active. Matches the Python
    /// `active_dialect()` thread-local — passed once when the
    /// context is built so gated passes (e.g. O124) can check
    /// for `"f5-irules"` without threading it through every
    /// entry point.
    pub dialect: Option<&'a str>,
    /// Accumulator for produced optimisations.
    pub optimisations: Vec<Optimisation>,
    /// Interprocedural analysis result. Passes consult it to
    /// resolve pure-proc targets, return-value facts, etc.
    pub interproc: InterproceduralAnalysis,
    /// Per-proc CFG + parameter map, for proc-call folding.
    pub proc_cfgs: ProcCfgs,
    /// `(var, ssa_version)` whose branch use has been propagated.
    pub propagated_branch_uses: HashSet<ValueKey>,
    /// Group-id assignment for propagated uses (all-or-nothing
    /// application of the containing rewrite group).
    pub propagated_use_groups: HashMap<ValueKey, u32>,
    /// `(block_name, stmt_index)` of propagated expression
    /// statements.
    pub propagated_expr_stmts: HashSet<(String, usize)>,
    /// Names whose values must be preserved across `when <event>`
    /// boundaries (iRules-specific).
    pub cross_event_vars: HashSet<String>,
    /// Group-id allocator.
    pub next_group: u32,
    /// Optional structured IR — some passes walk the pre-CFG
    /// tree to recover original source positions.
    pub ir_module: Option<&'a IrModule>,
}

impl<'a> PassContext<'a> {
    /// Construct a context bound to `source` and `interproc`. All
    /// other fields start empty / zero; callers populate
    /// [`proc_cfgs`](Self::proc_cfgs),
    /// [`cross_event_vars`](Self::cross_event_vars), and
    /// [`ir_module`](Self::ir_module) before running passes that
    /// consult them.
    #[must_use]
    pub fn new(source: &'a str, interproc: InterproceduralAnalysis) -> Self {
        Self {
            source,
            interproc,
            ..Self::default()
        }
    }

    /// Build a context bound to `source`, `interproc`, and an
    /// explicit Tcl `dialect`.
    #[must_use]
    pub fn with_dialect(
        source: &'a str,
        interproc: InterproceduralAnalysis,
        dialect: Option<&'a str>,
    ) -> Self {
        Self {
            source,
            dialect,
            interproc,
            ..Self::default()
        }
    }

    /// Record an optimisation diagnostic.
    pub fn report(&mut self, opt: Optimisation) {
        self.optimisations.push(opt);
    }

    /// Allocate and return the next group identifier.
    ///
    /// Group IDs are assigned to the `group` field of related
    /// [`Optimisation`]s so downstream consumers apply them
    /// as an all-or-nothing unit (e.g. a branch-use propagation
    /// + its dead-store elimination).
    pub fn alloc_group(&mut self) -> u32 {
        let g = self.next_group;
        self.next_group += 1;
        g
    }

    /// Reset the per-function scratch state (`propagated_*`) so
    /// the same context can drive multiple functions in sequence.
    /// Python's manager does this at the top of
    /// `_process_function`.
    pub fn reset_function_state(&mut self) {
        self.propagated_branch_uses.clear();
        self.propagated_use_groups.clear();
        self.propagated_expr_stmts.clear();
    }
}

// ---------------------------------------------------------------------------
// Pass registry
// ---------------------------------------------------------------------------

/// Identifier for one optimisation pass. Pass bodies land as
/// follow-up strips; this enum is the public surface callers use
/// to select a subset of passes or sequence them in a custom
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassId {
    /// Fold constant branches — dead-branch elimination.
    BranchFolding,
    /// Eliminate dead stores, unreachable blocks.
    Elimination,
    /// Simplify `expr {…}` bodies.
    ExprSimplify,
    /// Pattern-recognise idiomatic code into higher-level forms.
    PatternRecognition,
    /// Copy / constant propagation.
    Propagation,
    /// Structure-elimination (collapse trivial if / switch / loop).
    StructureElimination,
    /// Tail-call detection / rewrite.
    TailCall,
    /// Remove unused procedures.
    UnusedProcs,
    /// Move computations into their uses (code sinking).
    CodeSinking,
}

impl PassId {
    /// All pass identifiers in their default execution order.
    #[must_use]
    pub const fn all() -> [Self; 9] {
        [
            Self::Propagation,
            Self::BranchFolding,
            Self::StructureElimination,
            Self::ExprSimplify,
            Self::PatternRecognition,
            Self::Elimination,
            Self::CodeSinking,
            Self::TailCall,
            Self::UnusedProcs,
        ]
    }

    /// Short text form matching the Python pass module names.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BranchFolding => "branch_folding",
            Self::Elimination => "elimination",
            Self::ExprSimplify => "expr_simplify",
            Self::PatternRecognition => "pattern_recognition",
            Self::Propagation => "propagation",
            Self::StructureElimination => "structure_elimination",
            Self::TailCall => "tail_call",
            Self::UnusedProcs => "unused_procs",
            Self::CodeSinking => "code_sinking",
        }
    }
}

/// Run a sequence of optimisation passes over `ctx`, accumulating
/// diagnostics in `ctx.optimisations`.
///
/// Dispatches each requested [`PassId`] to its landed pass body.
///
/// Pass coverage:
///
/// - [`PassId::BranchFolding`] → [`branch_folding::run`] — O101
///   constant-fold + the `propagate_into_branches` cascade
///   (`substitute` → `strength_reduce` → `strlen` → `streq` → `instcombine`).
/// - [`PassId::UnusedProcs`] → [`unused_procs::run`] — O124
///   iRules unreachable-proc commenting.
/// - [`PassId::StructureElimination`] →
///   [`structure_elimination::run`] — O112 structural elimination.
/// - [`PassId::Elimination`] → [`elimination::run`] — O107
///   unreachable-block + O108 ADCE + O109 dead stores + O126
///   unused-variable assignments.
/// - [`PassId::ExprSimplify`] → [`expr_simplify::run`] — walks
///   both `Statement::ExprEval` and `Statement::AssignExpr`.
///   Emits O101 fold + O115 redundant-`expr` unwrap on standalone
///   `expr` statements; on `AssignExpr` (`set name [expr {…}]`)
///   it can additionally emit O110 (instcombine identities) and
///   O113 (strength reduction) directly, before the
///   `branch_folding::propagate_into_branches` cascade gets a
///   shot at branch conditions.
/// - [`PassId::Propagation`] → [`propagation::run`] — O100
///   constant-var-ref + O100 string-interpolation + O102 load
///   forwarding + O103 static-proc-call folding + O100
///   return-terminator folding.
/// - [`PassId::PatternRecognition`] →
///   [`pattern_recognition::run`] — O114 incr-idiom + O119 multi-set
///   packing hint + O104 string-build chain hint (canonical
///   allocation per `docs/generated/optimisation_codes.md`;
///   earlier Rust commits emitted O122 here, colliding with
///   `tail_call`).
/// - [`PassId::TailCall`] → [`tail_call::run`] — O121 (bare +
///   return-subst variants) + O122 loop-conversion hint + O123
///   accumulator-candidate hint.
/// - [`PassId::CodeSinking`] → [`code_sinking::run`] — O125
///   sink-set rewrites (paired delete + insert; falls back to
///   hint-only when the original `set` span is local-offset).
pub fn run_passes(ctx: &mut PassContext<'_>, cu: &CompilationUnit, passes: &[PassId]) {
    for pass in passes {
        match pass {
            PassId::BranchFolding => branch_folding::run(ctx, cu),
            PassId::UnusedProcs => unused_procs::run(ctx, cu),
            PassId::StructureElimination => structure_elimination::run(ctx, cu),
            PassId::Elimination => elimination::run(ctx, cu),
            PassId::ExprSimplify => expr_simplify::run(ctx, cu),
            PassId::Propagation => propagation::run(ctx, cu),
            PassId::PatternRecognition => pattern_recognition::run(ctx, cu),
            PassId::TailCall => tail_call::run(ctx, cu),
            PassId::CodeSinking => code_sinking::run(ctx, cu),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optimisation_new_defaults() {
        let o = Optimisation::new("O105", "message", Span::new(0, 5), "replacement");
        assert_eq!(o.code, "O105");
        assert!(o.group.is_none());
        assert!(!o.hint_only);
    }

    #[test]
    fn opt_priority_known_and_unknown() {
        assert_eq!(opt_priority("O126"), 10);
        assert_eq!(opt_priority("O112"), 9);
        assert_eq!(opt_priority("O104"), 4);
        assert_eq!(opt_priority("unknown"), 0);
    }

    #[test]
    fn pass_context_records_diagnostics() {
        let interproc = InterproceduralAnalysis::default();
        let mut ctx = PassContext::new("set x 1", interproc);
        ctx.report(Optimisation::new("O105", "m", Span::new(0, 1), "x"));
        assert_eq!(ctx.optimisations.len(), 1);
    }

    #[test]
    fn pass_context_default_fields_are_empty() {
        let ctx = PassContext::default();
        assert!(ctx.proc_cfgs.is_empty());
        assert!(ctx.propagated_branch_uses.is_empty());
        assert!(ctx.propagated_use_groups.is_empty());
        assert!(ctx.propagated_expr_stmts.is_empty());
        assert!(ctx.cross_event_vars.is_empty());
        assert_eq!(ctx.next_group, 0);
        assert!(ctx.ir_module.is_none());
    }

    #[test]
    fn alloc_group_produces_monotonic_ids() {
        let mut ctx = PassContext::default();
        assert_eq!(ctx.alloc_group(), 0);
        assert_eq!(ctx.alloc_group(), 1);
        assert_eq!(ctx.alloc_group(), 2);
        assert_eq!(ctx.next_group, 3);
    }

    #[test]
    fn reset_function_state_clears_propagation_scratch() {
        let mut ctx = PassContext::default();
        ctx.propagated_branch_uses.insert(("x".into(), 1));
        ctx.propagated_use_groups.insert(("x".into(), 1), 0);
        ctx.propagated_expr_stmts.insert(("b".into(), 3));
        // cross_event_vars and next_group are intentionally
        // preserved across functions.
        ctx.cross_event_vars.insert("static::foo".into());
        ctx.next_group = 7;

        ctx.reset_function_state();
        assert!(ctx.propagated_branch_uses.is_empty());
        assert!(ctx.propagated_use_groups.is_empty());
        assert!(ctx.propagated_expr_stmts.is_empty());
        assert_eq!(ctx.cross_event_vars.len(), 1);
        assert_eq!(ctx.next_group, 7);
    }

    #[test]
    fn proc_cfg_entry_roundtrip() {
        let mut ctx = PassContext::default();
        ctx.proc_cfgs.insert(
            "::foo".into(),
            ProcCfgEntry {
                cfg_key: "::foo".into(),
                params: vec!["a".into(), "b".into()],
            },
        );
        let entry = ctx.proc_cfgs.get("::foo").unwrap();
        assert_eq!(entry.params, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn pass_id_all_covers_each_variant() {
        let all = PassId::all();
        assert_eq!(all.len(), 9);
    }

    #[test]
    fn pass_id_str_names() {
        assert_eq!(PassId::BranchFolding.as_str(), "branch_folding");
        assert_eq!(PassId::UnusedProcs.as_str(), "unused_procs");
    }

    #[test]
    fn run_passes_empty_source_produces_nothing() {
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for("", &registry, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run_passes(&mut ctx, &cu, &PassId::all());
        assert!(ctx.optimisations.is_empty());
    }

    #[test]
    fn run_passes_empty_list_is_noop() {
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for("set x 1", &registry, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run_passes(&mut ctx, &cu, &[]);
        assert!(ctx.optimisations.is_empty());
    }
}
