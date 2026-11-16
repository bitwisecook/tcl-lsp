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

//! The worklist that runs per-procedure summary inference to convergence.

use super::call_flows::resolved_callees;
use super::summary_inference::{InferProcSummaryFn, infer_proc_summary_with_source};
use super::{ProcSummaryInputs, ProcTaintSummary};
use crate::compilation_unit::CompilationUnit;
use crate::taint::TaintSourceContext;
use std::collections::{HashMap, HashSet};
use tcl_registry::CommandRegistry;

/// Run the per-procedure return-taint summary fixpoint to convergence — the
/// dominant cost of [`solve_interprocedural_taints`] (~95% of `run_all_checks`
/// on a large file). `infer_proc_summary(P)` is a pure function of `P`'s body and
/// its *callees'* summaries, so this is driven by a dirty set: a procedure is
/// re-inferred only once one of its direct callees' summaries changed on the
/// previous pass (`callers[Q]` = the procedures that directly call `Q`, from the
/// interprocedural call graph). The lattice is monotone, so this converges to the
/// same fixpoint a full round-robin would — guarded in debug by a final
/// round-robin pass and by the `compiler_check` corpus differential. Without a
/// call graph every procedure is re-queued, exactly reproducing the round-robin.
///
/// Split out of [`solve_interprocedural_taints`] so the cheap entry-taint worklist
/// stays separate, and so this — the expensive, per-procedure-memoisable phase —
/// can be served by a salsa-memoised variant: the per-proc
/// `infer` step is injectable via [`InferProcSummaryFn`], so the LSP db can plug
/// in a `proc_summary_cascade` query keyed on each proc's offset-0 body + its
/// callees' summaries. The worklist itself (monotone convergence, the call-graph
/// dirty-set, mutual-recursion handling) is unchanged; only `infer` is redirected.
/// The debug-only fixpoint guard deliberately re-runs the **real**
/// [`infer_proc_summary`] (not `infer_fn`), so it validates the worklist's
/// convergence *and* an injected memo's correctness at once — a stale memo entry
/// trips the same assertion a missed call-graph edge would.
///
/// Append `val` to `map[key]`, deduplicated — the caller-edge insert for the
/// reverse call graph in [`converge_summaries_with`].
fn push_unique_edge(
    map: &mut HashMap<crate::ir::ProcedureId, Vec<crate::ir::ProcedureId>>,
    key: crate::ir::ProcedureId,
    val: crate::ir::ProcedureId,
) {
    let entry = map.entry(key).or_default();
    if !entry.contains(&val) {
        entry.push(val);
    }
}

fn infer_current_summary(
    cu: &CompilationUnit,
    qname: crate::ir::ProcedureId,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    known: &HashSet<crate::ir::ProcedureId>,
    summaries: &HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
    infer_fn: &mut InferProcSummaryFn<'_>,
) -> Option<ProcTaintSummary> {
    let fu = cu.function_for_body(Some(qname))?;
    let proc = cu
        .original_procedure_for_body(qname)
        .expect("current original procedure body");
    let source = TaintSourceContext::for_unit_may(registry, cu, fu);
    if source.metadata_context().is_some() {
        Some(infer_fn(cu, qname, interproc, known, summaries))
    } else {
        Some(infer_proc_summary_with_source(ProcSummaryInputs {
            qname: &proc.qualified_name,
            params: &proc.params,
            fu,
            registry,
            interproc,
            source,
            source_formals: TaintSourceContext::formals_for_unit_body(cu, qname, registry),
            known,
            summaries,
        }))
    }
}

fn seed_taint_summaries(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    proc_names: &[&crate::ir::ProcedureId],
) -> HashMap<crate::ir::ProcedureId, ProcTaintSummary> {
    let mut summaries: HashMap<crate::ir::ProcedureId, ProcTaintSummary> = HashMap::new();
    for qname in proc_names {
        let proc = cu
            .original_procedure_for_body(**qname)
            .expect("current original procedure body");
        summaries.insert(
            **qname,
            ProcTaintSummary::untainted_for_source(
                &proc.qualified_name,
                &proc.params,
                TaintSourceContext::formals_for_unit_body(cu, **qname, registry),
            ),
        );
    }
    summaries
}

fn taint_summary_callers(
    cu: &CompilationUnit,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    proc_names: &[&crate::ir::ProcedureId],
    known: &HashSet<crate::ir::ProcedureId>,
    registry: &CommandRegistry,
) -> Option<HashMap<crate::ir::ProcedureId, Vec<crate::ir::ProcedureId>>> {
    interproc.map(|ia| {
        let mut map: HashMap<crate::ir::ProcedureId, Vec<crate::ir::ProcedureId>> = HashMap::new();
        for (caller, summary) in &ia.procedures {
            for callee in &summary.direct_calls {
                push_unique_edge(&mut map, *callee, *caller);
            }
        }
        for qname in proc_names {
            for callee in resolved_callees(cu, Some(**qname), known, registry) {
                // `resolved_callees` returns owned names; the map borrows from
                // `known`'s keys, which outlive it and hold the same strings.
                if known.contains(&callee) {
                    push_unique_edge(&mut map, callee, **qname);
                }
            }
        }
        map
    })
}

#[cfg(debug_assertions)]
fn assert_taint_summaries_converged(
    cu: &CompilationUnit,
    proc_names: &[&crate::ir::ProcedureId],
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    known: &HashSet<crate::ir::ProcedureId>,
    summaries: &HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
) {
    for qname in proc_names {
        if let Some(fu) = cu.function_for_body(Some(**qname)) {
            let proc = cu
                .original_procedure_for_body(**qname)
                .expect("current original procedure body");
            let reinferred = infer_proc_summary_with_source(ProcSummaryInputs {
                qname: &proc.qualified_name,
                params: &proc.params,
                fu,
                registry,
                interproc,
                source: TaintSourceContext::for_unit_may(registry, cu, fu),
                source_formals: TaintSourceContext::formals_for_unit_body(cu, **qname, registry),
                known,
                summaries,
            });
            debug_assert_eq!(
                summaries.get(*qname),
                Some(&reinferred),
                "interproc taint summary fixpoint under-converged for `{qname:?}` \
                 (call graph missed a callee-summary edge)"
            );
        }
    }
}

/// `registry` and `dialect` feed only the debug-only round-robin guard below, so
/// in a release build (where `debug_assertions` is off and the guard is compiled
/// out) they are genuinely unused.
#[cfg_attr(not(debug_assertions), allow(unused_variables))]
pub fn converge_summaries_with(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    _dialect: Option<&'static tcl_dialect::DialectProfile>,
    infer_fn: &mut InferProcSummaryFn<'_>,
) -> HashMap<crate::ir::ProcedureId, ProcTaintSummary> {
    let mut proc_names: Vec<&crate::ir::ProcedureId> = cu
        .ir_module
        .original_source_procedure_bodies()
        .filter_map(|(id, _)| cu.function_for_body(Some(*id)).map(|_| id))
        .collect();
    proc_names.sort();

    // Seed summaries (all scenarios untainted).
    let mut summaries = seed_taint_summaries(cu, registry, &proc_names);
    let known: HashSet<crate::ir::ProcedureId> = summaries.keys().copied().collect();

    // `callers[Q]` = procedures that directly call `Q`; when `Q`'s summary changes
    // its callers are re-queued. Without a call graph, re-queue everything.
    //
    // Two sources, unioned.  `InterproceduralAnalysis::direct_calls`
    // is the declared call graph, but it misses a callee reached through a
    // command substitution the analyser recorded as a plain value —
    // `symbolNodeOf` in `set n [$t get [symbolNodeOf …] …]`, or a self-call
    // inside `[expr {[fib …]}]`.  [`resolved_callees`] scans the very CFG
    // statements the inference resolves calls from, so it supplies exactly the
    // edges `direct_calls` drops there.  A missed edge is not a wrong answer —
    // the completion sweep below still reaches the true least fixpoint — but it
    // costs a whole extra round-robin round to discover, which is the expensive
    // thing this solve does.
    let callers = taint_summary_callers(cu, interproc, &proc_names, &known, registry);
    let mut dirty: HashSet<crate::ir::ProcedureId> = proc_names.iter().map(|q| **q).collect();
    // Outer fixpoint-completion loop. The dirty-set worklist below converges
    // fast *when the call graph is complete*, but `callers` is derived from
    // `direct_calls`, which misses a callee edge buried in a nested command
    // substitution — e.g. `symbolNodeOf` inside `[$t get [symbolNodeOf …] …]`,
    // or a self-call inside `[expr {[fib …]}]`. With a missed edge the worklist
    // can empty one step short of the fixpoint (a taint false-negative the debug
    // guard below flags). After the worklist settles we run a full round-robin
    // pass and re-queue every proc whose summary still moves; the lattice is
    // monotone over a finite domain, so this reaches the true least fixpoint and
    // terminates. When the call graph *is* complete the completion pass finds no
    // movement and breaks after one clean sweep — the common, cheap case.
    loop {
        while !dirty.is_empty() {
            let mut next: HashSet<crate::ir::ProcedureId> = HashSet::new();
            for qname in &proc_names {
                if !dirty.contains(*qname) {
                    continue;
                }
                let Some(inferred) = infer_current_summary(
                    cu, **qname, registry, interproc, &known, &summaries, infer_fn,
                ) else {
                    continue;
                };
                if summaries.get(*qname) != Some(&inferred) {
                    summaries.insert(**qname, inferred);
                    // Re-queue `qname` itself (a self-recursive proc reads its own
                    // summary) plus its known callers from the reverse call graph.
                    next.insert(**qname);
                    match &callers {
                        Some(map) => next.extend(map.get(*qname).into_iter().flatten()),
                        None => next.extend(proc_names.iter().map(|q| **q)),
                    }
                }
            }
            dirty = next;
        }
        // Completion pass: a full round-robin catches any proc still short of the
        // fixpoint because the worklist's `callers` map missed an inbound edge.
        let mut moved: HashSet<crate::ir::ProcedureId> = HashSet::new();
        for qname in &proc_names {
            let Some(inferred) = infer_current_summary(
                cu, **qname, registry, interproc, &known, &summaries, infer_fn,
            ) else {
                continue;
            };
            if summaries.get(*qname) != Some(&inferred) {
                summaries.insert(**qname, inferred);
                moved.insert(**qname);
            }
        }
        if moved.is_empty() {
            break;
        }
        dirty = moved;
    }

    // Debug-only soundness guard: one full round-robin pass must find the dirty-set
    // fixpoint already stable. If this fires, the call graph (`direct_calls`) missed
    // a callee-summary dependency and the worklist under-converged (a taint
    // false-negative risk). Zero cost in release; proven clean over the tcllib corpus.
    #[cfg(debug_assertions)]
    assert_taint_summaries_converged(cu, &proc_names, registry, interproc, &known, &summaries);

    summaries
}
