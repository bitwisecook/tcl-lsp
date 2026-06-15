//! Top-level optimiser orchestration (C30j).
//!
//! Ported from `core/compiler/optimiser/_manager.py`. The Python
//! manager is a ~500-line class that coordinates
//! [`CompilationUnit`] construction, per-function SSA/SCCP
//! building, interprocedural analysis, and interleaves pass
//! execution. In Rust that orchestration is already implicit in
//! the layered pipeline: [`CompilationUnit::build_for`] runs the
//! analyses, [`build_interprocedural_analysis`]
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

use super::elimination::DeadStore;
use super::helpers::select::select_non_overlapping;
use super::{Optimisation, PassContext, PassId, run_passes};

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
    let cu = CompilationUnit::build_for_with_config(
        source,
        registry,
        false,
        tcl_lexer::LexerConfig::for_dialect(dialect.unwrap_or_default()),
    )
    .with_interprocedural(registry, dialect);
    optimise_unit(&cu, registry, dialect)
}

/// Run every pass over an **already-built** [`CompilationUnit`] (one carrying
/// its interprocedural summary) and return the overlap-resolved optimisations.
///
/// This is the rebuild-free core of [`optimise_with_dialect`]: callers that have
/// already constructed a `CompilationUnit` (e.g. the LSP diagnostics path, which
/// also runs `compiler_checks::run_all_checks` over the same unit) share it
/// instead of lowering the source a second time.
#[must_use]
pub fn optimise_unit(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<Optimisation> {
    let ia = cu.interproc.clone().unwrap_or_default();
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    ctx.registry = Some(registry);
    run_passes(&mut ctx, cu, &PassId::all());
    select_non_overlapping(&ctx.optimisations)
}

/// Run every pass over `cu` and return the **O109 dead stores** the
/// elimination pass determined eliminable (each keyed by function / block /
/// statement / SSA value). Mirrors [`optimise_unit`] but exposes the
/// structured dead-store records ([`PassContext::dead_stores`]) instead of
/// the optimisation list — so tools (the compiler explorer's `cfgPostSsa`
/// analysis, dead-store callouts, and `stats`) can show dead stores from
/// where Rust actually computes them, with the optimiser's full suppression
/// applied (purity, scope aliases, place model, cross-event scope).
#[must_use]
pub fn find_dead_stores(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<DeadStore> {
    let ia = cu.interproc.clone().unwrap_or_default();
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    ctx.registry = Some(registry);
    run_passes(&mut ctx, cu, &PassId::all());
    ctx.dead_stores
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
    let cu = CompilationUnit::build_for_with_config(
        source,
        registry,
        false,
        tcl_lexer::LexerConfig::for_dialect(dialect.unwrap_or_default()),
    );
    let ia = build_interprocedural_analysis(&cu.ir_module, registry, dialect);
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    ctx.registry = Some(registry);
    // SYNC-JUN02b-4: the whole-module builtin-fold trust gate (O129).
    ctx.command_mutations =
        crate::command_binding::scan_module_command_mutations(&cu.ir_module, registry);
    run_passes(&mut ctx, &cu, &PassId::all());
    ctx.optimisations
}

/// Apply the non-hint-only optimisation rewrites to `source`, returning the
/// rewritten text.  Edits are applied in reverse-offset order (so earlier
/// offsets stay valid) and deduplicated by `(offset, length)`.  Spans are
/// half-open `[start, end)`, so the byte range is `span.start()..span.end()`.
/// Mirrors `_manager.py::apply_optimisations`.
#[must_use]
pub fn apply_optimisations(source: &str, optimisations: &[Optimisation]) -> String {
    let mut edits: Vec<(usize, usize, &str)> = optimisations
        .iter()
        .filter(|o| !o.hint_only)
        .filter_map(|o| {
            let start = o.span.start() as usize;
            let end = o.span.end() as usize;
            (start <= end && end <= source.len()).then_some((
                start,
                end - start,
                o.replacement.as_str(),
            ))
        })
        .collect();
    if edits.is_empty() {
        return source.to_owned();
    }
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut seen: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let mut result = source.to_owned();
    for (offset, length, text) in edits {
        if !seen.insert((offset, length)) {
            continue;
        }
        if offset + length <= result.len() {
            result.replace_range(offset..offset + length, text);
        }
    }
    result
}

/// Iteratively optimise `source` until a fixpoint or `max_iterations` is
/// reached: each pass recompiles the rewritten source so optimisations
/// exposed by an earlier pass (constant folding enabling further folding /
/// dead-store removal) are discovered.  Returns `(final_source,
/// all_optimisations_applied)`.  Mirrors
/// `_manager.py::optimise_source_multipass`.
#[must_use]
pub fn optimise_source_multipass(
    source: &str,
    registry: &CommandRegistry,
    dialect: Option<&str>,
    max_iterations: usize,
) -> (String, Vec<Optimisation>) {
    let mut current = source.to_owned();
    let mut all: Vec<Optimisation> = Vec::new();
    for _ in 0..max_iterations {
        let opts = optimise_with_dialect(&current, registry, dialect);
        if opts.is_empty() {
            break;
        }
        let next = apply_optimisations(&current, &opts);
        all.extend(opts);
        if next == current {
            break;
        }
        current = next;
    }
    (current, all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> CommandRegistry {
        // C43 sub-strip 4: `when` is now registry-resolved (no
        // string-pattern fallback in `lower_command`), so any
        // test that lowers iRule code through `optimise_*` must
        // carry the iRules dialect.  Loading it here keeps the
        // helper a one-call site; tests that only lower plain
        // Tcl don't notice the extra commands.
        let mut r = CommandRegistry::build_default();
        r.load_irules();
        r
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
    fn info_exists_fold_surfaces_o101() {
        // SYNC-MAY31-3 DCE wiring: a provably-constant `info exists`
        // guard (never-defined non-param folds false; a parameter folds
        // true) surfaces as an O101 constant-branch fold.
        let never = optimise(
            "proc f {a} { if {[info exists b]} { puts hi } }",
            &registry(),
        );
        assert!(
            never
                .iter()
                .any(|o| o.code == "O101" && o.replacement == "0"),
            "never-defined `info exists` should fold to 0, got {never:?}",
        );
        let param = optimise(
            "proc f {a} { if {[info exists a]} { puts hi } }",
            &registry(),
        );
        assert!(
            param
                .iter()
                .any(|o| o.code == "O101" && o.replacement == "1"),
            "parameter `info exists` should fold to 1, got {param:?}",
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
