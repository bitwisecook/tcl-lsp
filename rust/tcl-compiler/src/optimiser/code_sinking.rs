//! Code-sinking optimiser pass — **O125** (C30i, scaffolding).
//!
//! Ported from `core/compiler/optimiser/_code_sinking.py`. The
//! Python pass emits `O125` ("Sink definition into decision
//! block") for assignments that can be safely moved down into
//! the branch of an `if` / `switch` that is the only consumer.
//! The algorithm requires three supporting analyses the Rust
//! side does not yet expose at sufficient fidelity:
//!
//! 1. **Side-effect classification of the assignment's
//!    right-hand side** — the SCCP evaluator is AST-level; what
//!    we need is whether the *original statement text* can
//!    silently be moved (no barriers, no reads of
//!    soon-to-be-reassigned vars).
//! 2. **Scanning the remaining statements of the enclosing
//!    script for a use of the sunk variable** — requires
//!    `VarReferenceScanner` output per statement-range. The
//!    Rust `var_refs` crate scans full scripts; per-statement
//!    segmentation is pending.
//! 3. **Deepest-sink target detection** — walks the decision
//!    tree looking for the innermost body that references the
//!    variable. Recursive; needs the same per-statement
//!    variable-use output.
//!
//! This strip lands the [`run`] entry point and its recursive
//! IR walker so `PassId::CodeSinking` dispatches cleanly. It
//! emits no diagnostics today — every landing site is intentionally
//! stubbed pending (1)–(3). Once those land, the emission sites
//! will plug into this walker without an API change.

use crate::compilation_unit::CompilationUnit;
use crate::ir::{Script, Statement};

use super::PassContext;

/// Run the code-sinking pass. Walks every function's IR but
/// emits no `O125` diagnostics yet — see the module docs for the
/// analyses this pass is waiting on.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    walk_script(ctx, &cu.ir_module.top_level);
    for proc in cu.ir_module.procedures.values() {
        walk_script(ctx, &proc.body);
    }
}

fn walk_script(ctx: &mut PassContext<'_>, script: &Script) {
    for stmt in &script.statements {
        walk_statement(ctx, stmt);
    }
}

fn walk_statement(ctx: &mut PassContext<'_>, stmt: &Statement) {
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(ctx, &c.body);
            }
            if let Some(b) = else_body {
                walk_script(ctx, b);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(ctx, init);
            walk_script(ctx, next);
            walk_script(ctx, body);
        }
        Statement::While { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Foreach { body, .. } => walk_script(ctx, body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(ctx, body);
            for h in handlers {
                walk_script(ctx, &h.body);
            }
            if let Some(fb) = finally_body {
                walk_script(ctx, fb);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    walk_script(ctx, b);
                }
            }
            if let Some(b) = default_body {
                walk_script(ctx, b);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandRegistry;

    use crate::interprocedural::InterproceduralAnalysis;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn walker_produces_no_diagnostics_today() {
        // Until the supporting analyses land, the pass is
        // intentionally a no-emit walker — it must not panic
        // and must not produce spurious rewrites.
        let cu = CompilationUnit::build_for(
            "set x 1\nif {$cond} { puts $x } else { puts no }",
            &registry(),
            false,
        );
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        run(&mut ctx, &cu);
        assert!(ctx.optimisations.is_empty());
    }

    #[test]
    fn run_passes_dispatches_code_sinking_cleanly() {
        let cu = CompilationUnit::build_for("set x 1", &registry(), false);
        let mut ctx = PassContext::new(&cu.source, InterproceduralAnalysis::default());
        super::super::run_passes(&mut ctx, &cu, &[super::super::PassId::CodeSinking]);
        assert!(ctx.optimisations.is_empty());
    }
}
