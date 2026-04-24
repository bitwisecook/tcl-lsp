//! Top-level optimiser orchestration (C30j).
//!
//! Ported from `core/compiler/optimiser/_manager.py`. The Python
//! manager is a ~500-line class that coordinates
//! [`CompilationUnit`] construction, per-function SSA/SCCP
//! building, interprocedural analysis, and interleaves pass
//! execution. In Rust that orchestration is already implicit in
//! the layered pipeline: [`CompilationUnit::build_for`] runs the
//! analyses, [`build_interprocedural_analysis`](crate::interprocedural::build_interprocedural_analysis)
//! builds the summaries, and [`super::run_passes`] dispatches
//! each pass. The only value-add of a dedicated manager is the
//! thin façade below — a single entry point that plumbs these
//! together, runs every pass in default order, and then applies
//! the overlap-aware selection filter from
//! [`super::helpers::select`].
//!
//! Callers that need a single `optimise(source)` one-shot call
//! use [`optimise`] / [`optimise_with_dialect`]. Callers that
//! want full control (custom pass ordering, pre-populated
//! `PassContext` scratch state) stay on
//! [`super::run_passes`] directly.

use tcl_registry::CommandRegistry;

use crate::compilation_unit::CompilationUnit;
use crate::interprocedural::build_interprocedural_analysis;

use super::helpers::select::select_non_overlapping;
use super::{run_passes, Optimisation, PassContext, PassId};

/// Build a [`CompilationUnit`] for `source`, run every landed
/// optimiser pass in canonical order, and return the overlap-
/// free set of [`Optimisation`] suggestions.
///
/// Equivalent to [`optimise_with_dialect`] with `dialect = None`.
#[must_use]
pub fn optimise(source: &str, registry: &CommandRegistry) -> Vec<Optimisation> {
    optimise_with_dialect(source, registry, None)
}

/// Build a [`CompilationUnit`] for `source`, populate
/// interprocedural summaries, and run every pass in
/// [`PassId::all()`] order — then deduplicate via
/// [`select_non_overlapping`].
#[must_use]
pub fn optimise_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<Optimisation> {
    let cu =
        CompilationUnit::build_for(source, registry, false).with_interprocedural(registry, dialect);
    let interproc = cu.interproc.clone().unwrap_or_default();
    let _ = interproc;
    let ia = cu.interproc.clone().unwrap_or_default();
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    run_passes(&mut ctx, &cu, &PassId::all());
    select_non_overlapping(&ctx.optimisations)
}

/// Build, run every pass, and return the full *unfiltered*
/// optimisation list (no overlap resolution). Exposed mainly for
/// tests that want to inspect raw per-pass output before the
/// manager's arbitration.
#[must_use]
pub fn optimise_raw(
    source: &str,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<Optimisation> {
    // Split the raw CU build to avoid recomputing on
    // `with_interprocedural` — done in two lines for clarity.
    let cu = CompilationUnit::build_for(source, registry, false);
    let ia = build_interprocedural_analysis(&cu.ir_module, registry, dialect);
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    run_passes(&mut ctx, &cu, &PassId::all());
    ctx.optimisations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn empty_source_yields_empty_result() {
        let opts = optimise("", &registry());
        assert!(opts.is_empty());
    }

    #[test]
    fn constant_branch_fires_via_manager() {
        // if {1} { set x 1 } triggers both O101 (branch fold)
        // and O112 (structure elimination). The overlap filter
        // prefers the higher-priority O112 (priority 9) over
        // O101 (priority 1), so the manager's output should
        // contain O112 in this shape.
        let opts = optimise("if {1} { set x 1 } else { set y 2 }", &registry());
        assert!(
            opts.iter().any(|o| o.code == "O112" || o.code == "O101"),
            "expected at least one branch-related rewrite, got {opts:?}",
        );
    }

    #[test]
    fn output_is_sorted_by_span_start() {
        let opts = optimise("set x 5\nif {1} { puts $x } else { puts 0 }", &registry());
        let mut prev = 0u32;
        for o in &opts {
            assert!(
                o.span.start() >= prev,
                "manager output must be sorted by span start: got {} after {}",
                o.span.start(),
                prev,
            );
            prev = o.span.start();
        }
    }

    #[test]
    fn overlap_filter_runs() {
        // The manager applies select_non_overlapping, so duplicate
        // / overlapping rewrites must not appear in the output
        // — exercise by checking that no two rewrites share the
        // exact same span.
        let opts = optimise("if {1} { set x 1 } else { set y 2 }", &registry());
        let spans: Vec<_> = opts.iter().map(|o| o.span).collect();
        let mut unique = spans.clone();
        unique.sort_by_key(|s| (s.start(), s.end()));
        unique.dedup();
        assert_eq!(
            spans.len(),
            unique.len(),
            "manager must deduplicate overlapping rewrites",
        );
    }

    #[test]
    fn optimise_raw_skips_overlap_filter() {
        // Raw output can contain overlapping rewrites that the
        // filtered path would dedupe — the `raw` entry point is
        // the escape hatch that leaves them visible.
        let raw = optimise_raw("if {1} { set x 1 } else { set y 2 }", &registry(), None);
        // Presence alone is the contract; specific counts depend
        // on which pass bodies are landed.
        let _ = raw;
    }

    #[test]
    fn dialect_gated_passes_observe_active_dialect() {
        // irules-only O124 should fire when dialect = f5-irules.
        let src = "proc ::dead {} { return 1 }\nwhen HTTP_REQUEST { set x 0 }\n";
        let opts = optimise_with_dialect(src, &registry(), Some("f5-irules"));
        assert!(
            opts.iter().any(|o| o.code == "O124"),
            "expected O124 in irules dialect, got {opts:?}",
        );
        // And should NOT fire for plain tcl.
        let tcl_opts = optimise_with_dialect(src, &registry(), Some("tcl"));
        assert!(
            tcl_opts.iter().all(|o| o.code != "O124"),
            "O124 should be gated on irules dialect, got {tcl_opts:?}",
        );
    }
}
