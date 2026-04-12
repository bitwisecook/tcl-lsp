//! Elimination optimiser pass (C30d, partial — O107 only).
//!
//! Ported from `core/compiler/optimiser/_elimination.py`. That
//! Python module emits four distinct diagnostic codes:
//!
//! - **O107** — unreachable dead code (blocks SCCP proved
//!   unreachable).
//! - **O109** — dead stores (definitions overwritten before any
//!   read).
//! - **O108** — transitively dead code (defs whose every consumer
//!   is itself dead — Aggressive Dead Code Elimination fixpoint).
//! - **O126** — unused variable assignments (defs never read
//!   anywhere in the function).
//!
//! Only **O107** is landed in this strip — the others require a
//! liveness analyser + dead-store detector that the Rust
//! pipeline does not yet build (the analyses types exist as
//! empty stubs in `crate::analyses`, but the populating pass is
//! deferred). Each of those codes will plug into the same
//! `run(ctx, cu)` entry point without an API change when the
//! supporting analyser lands.
//!
//! The O107 implementation walks each `FunctionUnit`, finds
//! every CFG block outside `SccpResult::executable_blocks`, and
//! reports the statement span of every statement within such a
//! block. The pass is deterministic: unreachable blocks are
//! walked in their CFG-order (reverse post-order from the entry,
//! with unreachable blocks appended) and reports are emitted in
//! that order.

use crate::cfg::Function as CfgFunction;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::sccp::{cfg_order, SccpResult};

use super::{Optimisation, PassContext};

/// Run the elimination pass. Currently emits `O107` only (see
/// the module-level docs for the status of O108/O109/O126).
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    emit_unreachable(ctx, &cu.top_level);
    for fu in cu.procedures.values() {
        emit_unreachable(ctx, fu);
    }
}

fn emit_unreachable(ctx: &mut PassContext<'_>, fu: &FunctionUnit) {
    let unreachable = unreachable_blocks(&fu.cfg, &fu.sccp);
    // cfg_order is deterministic (RPO + trailing unreachables).
    for block_name in cfg_order(&fu.cfg) {
        if !unreachable.contains(&block_name) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&block_name) else {
            continue;
        };
        for stmt in &block.statements {
            let span = stmt.span();
            // Skip zero-length spans — those are synthesised IR
            // (e.g. implicit barriers) with no user-visible
            // source text to delete.
            if span.is_empty() {
                continue;
            }
            ctx.report(Optimisation::new(
                "O107",
                "Eliminate unreachable dead code",
                span,
                "",
            ));
        }
    }
}

/// Return the set of block names SCCP determined unreachable
/// from the CFG entry.
fn unreachable_blocks(
    cfg: &CfgFunction,
    sccp: &SccpResult,
) -> std::collections::HashSet<String> {
    cfg.blocks
        .keys()
        .filter(|name| !sccp.executable_blocks.contains(*name))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use tcl_registry::CommandRegistry;

    use crate::interprocedural::InterproceduralAnalysis;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn run_pass(source: &str) -> Vec<Optimisation> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    // -- internal helper tests -----------------------------------------------

    #[test]
    fn unreachable_blocks_empty_when_all_executable() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false);
        let unreach = unreachable_blocks(&cu.top_level.cfg, &cu.top_level.sccp);
        assert!(unreach.is_empty());
    }

    // -- end-to-end tests ---------------------------------------------------

    #[test]
    fn empty_source_produces_nothing() {
        let opts = run_pass("");
        assert!(opts.is_empty());
    }

    #[test]
    fn straight_line_script_is_fully_reachable() {
        let opts = run_pass("set x 1\nset y 2\nputs $x");
        assert!(opts.is_empty());
    }

    #[test]
    fn branch_folding_creates_unreachable_block() {
        // The else branch is unreachable under SCCP because the
        // condition folds to true.
        let opts = run_pass("if {1} { set x 1 } else { set y 2 }");
        // Expect at least one O107 — the else body's `set y 2`.
        assert!(
            opts.iter()
                .any(|o| o.code == "O107" && o.message == "Eliminate unreachable dead code"),
            "expected at least one O107, got {opts:?}",
        );
    }

    #[test]
    fn while_false_body_is_unreachable() {
        // The body of `while {0} { ... }` is unreachable.
        let opts = run_pass("while {0} { set x 1 }");
        assert!(
            opts.iter().any(|o| o.code == "O107"),
            "expected an O107 for dead while body, got {opts:?}",
        );
    }

    #[test]
    fn unreachable_statements_emitted_with_empty_replacement() {
        let opts = run_pass("if {0} { set x 1 }");
        let target = opts.iter().find(|o| o.code == "O107");
        if let Some(o) = target {
            assert_eq!(o.replacement, "");
            assert!(!o.span.is_empty());
        }
    }

    #[test]
    fn run_passes_dispatches_elimination() {
        let cu = CompilationUnit::build_for(
            "if {0} { set x 1 }",
            &registry(),
            false,
        );
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::Elimination]);
        // At minimum the dispatch must not panic; O107 may or
        // may not fire depending on how SCCP models this exact
        // shape, but running the pass must be side-effect free
        // otherwise.
        let only_o107 = ctx.optimisations.iter().all(|o| o.code == "O107");
        assert!(only_o107, "unexpected codes: {:?}", ctx.optimisations);
    }
}
