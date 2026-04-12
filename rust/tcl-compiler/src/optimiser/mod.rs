//! Optimiser passes — source-rewrite suggestions produced by
//! analysing the compiled IR / CFG / SSA.
//!
//! Ported from `core/compiler/optimiser/` (C30). This module
//! owns the shared optimiser types (`Optimisation`,
//! `PassContext`, opt priorities, `PassId`, `run_passes`) plus
//! per-pass submodules. Pass bodies land as follow-up strips
//! (C30a–j):
//!
//! - **C30a** — [`branch_folding`] — constant branch folding,
//!   powered by SCCP's
//!   [`ConstantBranch`](crate::sccp::ConstantBranch) output.
//! - C30b–j — elimination, expr-simplify, pattern-recognition,
//!   propagation, structure-elimination, tail-call, unused-procs,
//!   code-sinking, and the manager — still stubbed.

pub mod branch_folding;

use tcl_lexer::Span;

use crate::compilation_unit::CompilationUnit;
use crate::interprocedural::InterproceduralAnalysis;

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
        "O121" | "O123" | "O125" | "O119" | "O120" | "O118" | "O117" | "O116" | "O115"
        | "O114" | "O113" | "O110" => 5,
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

/// Shared mutable state threaded through all optimisation passes.
#[derive(Debug, Default)]
pub struct PassContext<'a> {
    /// Full source text (UTF-8).
    pub source: &'a str,
    /// Accumulator for produced optimisations.
    pub optimisations: Vec<Optimisation>,
    /// Interprocedural analysis result. Passes consult it to
    /// resolve pure-proc targets, return-value facts, etc.
    pub interproc: InterproceduralAnalysis,
}

impl<'a> PassContext<'a> {
    /// Construct a context bound to `source` and `interproc`.
    #[must_use]
    pub fn new(source: &'a str, interproc: InterproceduralAnalysis) -> Self {
        Self {
            source,
            optimisations: Vec::new(),
            interproc,
        }
    }

    /// Record an optimisation diagnostic.
    pub fn report(&mut self, opt: Optimisation) {
        self.optimisations.push(opt);
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
/// Passes whose body has not landed yet are silently skipped —
/// the caller's pass list stays stable as follow-up strips plug
/// in.
///
/// Currently landed passes:
///
/// - [`PassId::BranchFolding`] → [`branch_folding::run`] (C30a).
pub fn run_passes(ctx: &mut PassContext<'_>, cu: &CompilationUnit, passes: &[PassId]) {
    for pass in passes {
        match pass {
            PassId::BranchFolding => branch_folding::run(ctx, cu),
            // Remaining passes are deferred follow-ups; see the
            // module docs for the landing plan.
            PassId::Elimination
            | PassId::ExprSimplify
            | PassId::PatternRecognition
            | PassId::Propagation
            | PassId::StructureElimination
            | PassId::TailCall
            | PassId::UnusedProcs
            | PassId::CodeSinking => {}
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
    fn run_passes_skips_deferred_passes_silently() {
        use tcl_registry::CommandRegistry;
        let registry = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for("set x 1", &registry, false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        // Requesting only deferred passes must leave the
        // context untouched.
        run_passes(
            &mut ctx,
            &cu,
            &[
                PassId::Elimination,
                PassId::ExprSimplify,
                PassId::Propagation,
                PassId::PatternRecognition,
                PassId::StructureElimination,
                PassId::TailCall,
                PassId::UnusedProcs,
                PassId::CodeSinking,
            ],
        );
        assert!(ctx.optimisations.is_empty());
    }
}
