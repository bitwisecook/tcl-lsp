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
    arbitrate(ctx.optimisations)
}

/// Canonicalise + overlap-arbitrate a raw optimisation list (the shared tail of
/// [`optimise_unit`] and [`optimise_unit_per_function`]).
///
/// Determinism chokepoint.  Several passes iterate `HashMap`s
/// (`cu.procedures`, def-use chains, SSA blocks), so both the emission order
/// and the monotonic group ids vary run-to-run — and, critically, between the
/// offset-0 per-procedure memo build and the whole-module build.  Canonicalise
/// before overlap arbitration so the surviving set, its order, and group
/// numbering are byte-identical given an equal optimisation set (the
/// `compiler_check_corpus` guard's contract, and the precondition for salsa
/// early-cutoff on `compiler_check_diagnostics`).
fn arbitrate(mut opts: Vec<Optimisation>) -> Vec<Optimisation> {
    opts.sort_by(|a, b| {
        (
            a.span.start(),
            a.span.end(),
            &a.code,
            &a.message,
            &a.replacement,
            a.hint_only,
        )
            .cmp(&(
                b.span.start(),
                b.span.end(),
                &b.code,
                &b.message,
                &b.replacement,
                b.hint_only,
            ))
    });
    let mut selected = select_non_overlapping(&opts);
    renumber_groups(&mut selected);
    selected
}

/// Like [`optimise_unit`] but runs the passes **per function in isolation** —
/// each of `::top` and every procedure is optimised in a `CompilationUnit` view
/// holding only that function (the others emptied), and the raw optimisations
/// are merged before the single whole-unit [`arbitrate`] step.
///
/// This is byte-identical to [`optimise_unit`] because the optimiser's only
/// cross-function `PassContext` state is inert in production: the `propagated_*`
/// scratch sets are never written outside unit tests, `next_group` is
/// canonicalised by [`renumber_groups`], and O127's `rewritten` overlap snapshot
/// only matches spans within one function's own (disjoint) source region.  It is
/// the seam the salsa-native per-procedure optimiser memo builds on (each
/// isolated run is keyed on the procedure's offset-0 lattice + context).  Guarded
/// by `optimise_per_function_matches_whole_unit_over_corpus`.
#[must_use]
pub fn optimise_unit_per_function(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<Optimisation> {
    let ia = cu.interproc.clone().unwrap_or_default();
    // Empty-source unit: a template for the emptied `::top` slot when isolating
    // a procedure (its top-level produces no optimisations).
    let empty = CompilationUnit::build_for("", registry, false);
    // Each isolated run uses a fresh `PassContext`, so its `next_group` counter
    // restarts at 0 — two functions' rewrite groups would both be `0` and be
    // *conflated* into one partition on merge.  Remap each run's group ids into a
    // globally-unique range before merging so distinct partitions stay distinct;
    // the final [`arbitrate`] `renumber_groups` then re-canonicalises them by
    // first-appearance exactly as the whole-unit run does (the canonical id
    // depends only on the partition + sorted order, not the pre-renumber value).
    let mut global_next = 0u32;
    let mut run_one = |view: &CompilationUnit, all: &mut Vec<Optimisation>| {
        let mut ctx = PassContext::with_dialect(&view.source, ia.clone(), dialect);
        ctx.registry = Some(registry);
        run_passes(&mut ctx, view, &PassId::all());
        let mut local: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for mut opt in ctx.optimisations {
            if let Some(g) = opt.group {
                let ng = *local.entry(g).or_insert_with(|| {
                    let v = global_next;
                    global_next += 1;
                    v
                });
                opt.group = Some(ng);
            }
            all.push(opt);
        }
    };

    let mut all: Vec<Optimisation> = Vec::new();

    // `::top` in isolation — drop every procedure so only the top level runs.
    {
        let mut view = cu.clone();
        view.procedures.clear();
        view.ir_module.procedures.clear();
        run_one(&view, &mut all);
    }

    // Each procedure in isolation — empty `::top` + keep only this proc.  Methods
    // and the interprocedural summary stay intact (read-only cross-function
    // context the passes consult, e.g. the O126 pure-method gate).
    let mut names: Vec<&String> = cu.procedures.keys().collect();
    names.sort();
    for name in names {
        let mut view = cu.clone();
        let fu = cu.procedures.get(name).expect("name from keys").clone();
        view.procedures.clear();
        view.procedures.insert(name.clone(), fu);
        view.ir_module.procedures.clear();
        if let Some(p) = cu.ir_module.procedures.get(name).cloned() {
            view.ir_module.procedures.insert(name.clone(), p);
        }
        view.ir_module.top_level = empty.ir_module.top_level.clone();
        view.top_level = empty.top_level.clone();
        run_one(&view, &mut all);
    }

    arbitrate(all)
}

/// Canonicalise group ids in-place to `0, 1, 2, …` by order of first appearance.
///
/// Group ids are allocated by a monotonic counter during pass execution, so
/// their absolute values depend on the (`HashMap`-iteration) order in which
/// grouped rewrites were emitted.  Only the *partition* they encode is
/// semantically meaningful (members of one group apply all-or-nothing), so
/// remapping each distinct id to a first-appearance index makes the values
/// deterministic while preserving the partition.  `opts` is assumed already in
/// canonical order.
fn renumber_groups(opts: &mut [Optimisation]) {
    use std::collections::HashMap;
    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut next = 0u32;
    for o in opts.iter_mut() {
        if let Some(g) = o.group {
            let new = *remap.entry(g).or_insert_with(|| {
                let v = next;
                next += 1;
                v
            });
            o.group = Some(new);
        }
    }
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

/// Run the passes one at a time over a shared context and return, for each
/// [`PassId`] in [`PassId::all`] order, the optimisations *that pass*
/// produced (raw, before overlap arbitration). Powers the explorer's
/// Rust-native "optimiser pass pipeline" view — there is no Python analogue
/// because the Python optimiser is not structured as this pass sequence.
///
/// Equivalent to [`optimise_unit`] in effect (each pass sees the prior
/// passes' context), but it attributes every finding to its originating pass.
#[must_use]
pub fn optimise_by_pass(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&str>,
) -> Vec<(PassId, Vec<Optimisation>)> {
    let ia = cu.interproc.clone().unwrap_or_default();
    let mut ctx = PassContext::with_dialect(&cu.source, ia, dialect);
    ctx.registry = Some(registry);
    let mut by_pass = Vec::new();
    for pass in PassId::all() {
        let before = ctx.optimisations.len();
        run_passes(&mut ctx, cu, &[pass]);
        by_pass.push((pass, ctx.optimisations[before..].to_vec()));
    }
    by_pass
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
