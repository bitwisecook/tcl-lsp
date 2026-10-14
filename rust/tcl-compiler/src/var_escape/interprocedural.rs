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

//! Interprocedural propagation of var-escape summaries.
//!
//! The intra-procedural pass ([`super::analyse_script`]) records
//! each proc's own `upvar_source_names` — the literal caller-frame
//! names it reaches via `upvar <positive-level>`. A caller whose
//! local name matches must spill that local to the frame so the
//! callee's alias can resolve.
//!
//! This module runs a worklist fixpoint over the static call graph
//! built from each proc's `direct_callees`. After convergence,
//! each proc's `upvar_source_names` is the union of its own
//! sources and every transitive callee's sources. The caller's
//! summary is then augmented: any of its local vars whose names
//! appear in the callee set are marked `Frame`.
//!
//! `unbounded_upvar_source` propagates the same way.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use std::hash::BuildHasher;

use crate::var_escape::types::{EscapeFlags, ProcEscapeSummary};

/// Exact retained source-call edges; labels carry no body-selection authority.
#[derive(Debug, Clone, Default)]
pub struct EscapeCallGraph {
    callees: HashMap<tcl_core_types::BodyFrameId, HashSet<tcl_core_types::BodyFrameId>>,
    unresolved: HashSet<tcl_core_types::BodyFrameId>,
}

impl EscapeCallGraph {
    /// Analytical edges between exact body coordinates; this grants no runtime entry.
    #[must_use]
    pub fn from_edges(
        edges: impl IntoIterator<Item = (tcl_core_types::BodyFrameId, tcl_core_types::BodyFrameId)>,
        unresolved: impl IntoIterator<Item = tcl_core_types::BodyFrameId>,
    ) -> Self {
        let mut graph = Self {
            unresolved: unresolved.into_iter().collect(),
            ..Self::default()
        };
        for (caller, callee) in edges {
            graph.callees.entry(caller).or_default().insert(callee);
        }
        graph
    }

    pub(crate) fn for_module(
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
    ) -> Self {
        use tcl_core_types::BodyFrameId;
        let mut graph = Self::default();
        let bodies = std::iter::once((
            BodyFrameId::Top(module.procedure_ids.module_id()),
            &module.top_level,
        ))
        .chain(
            module
                .procedures
                .iter()
                .map(|(id, body)| (BodyFrameId::Body(*id), &body.body)),
        )
        .chain(
            module
                .body_units
                .iter()
                .map(|(id, body)| (BodyFrameId::Body(*id), &body.body)),
        )
        .chain(
            module
                .methods
                .iter()
                .map(|(id, body)| (BodyFrameId::Body(*id), &body.body)),
        );
        for (caller, body) in bodies {
            graph.callees.entry(caller).or_default();
            let mut pending = vec![body];
            while let Some(script) = pending.pop() {
                for statement in &script.statements {
                    pending.extend(statement.child_scripts());
                    if let crate::ir::Statement::Call {
                        command, tokens, ..
                    } = statement
                    {
                        if let Some(tokens) = tokens.as_ref() {
                            graph.record_call(caller, command, tokens, module, registry);
                        } else {
                            graph.unresolved.insert(caller);
                        }
                    }
                    let Some(tokens) = statement.tokens() else {
                        continue;
                    };
                    let calls = tokens
                        .source_binding
                        .as_ref()
                        .and_then(|binding| {
                            binding
                                .original_invocation_metadata_for_module(tokens, module, registry)
                        })
                        .and_then(|metadata| {
                            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                                tokens,
                                module.lexer_config,
                                registry,
                                metadata,
                            )
                        });
                    match calls {
                        Some(calls) => {
                            for call in calls {
                                if let Some(tokens) = call.tokens.as_ref() {
                                    graph.record_call(
                                        caller,
                                        &call.command,
                                        tokens,
                                        module,
                                        registry,
                                    );
                                } else {
                                    graph.unresolved.insert(caller);
                                }
                            }
                        }
                        None => {
                            graph.unresolved.insert(caller);
                        }
                    }
                }
            }
        }
        graph
    }

    fn record_call(
        &mut self,
        caller: tcl_core_types::BodyFrameId,
        command: &str,
        tokens: &crate::ir::CommandTokens,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
    ) {
        if let Some(ids) =
            crate::registry_invocation::original_source_procedure_ids_for_word_for_module(
                tokens, command, module, registry,
            )
        {
            self.callees
                .entry(caller)
                .or_default()
                .extend(ids.into_iter().map(tcl_core_types::BodyFrameId::Body));
            return;
        }
        let frameless = tokens.source_binding.as_ref()
            .and_then(|binding| binding.original_invocation_metadata_for_module(tokens, module, registry))
            .is_some_and(|metadata| {
                crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(registry, metadata, tokens)
                    .is_some_and(|invocation| invocation.facts.traits.contains(tcl_registry::Traits::FRAMELESS_RUNTIME))
                    || crate::registry_invocation::normal_representation_invocation_with_metadata_context(registry, Some(metadata), tokens)
                        .is_some_and(|invocation| invocation.is_frameless_runtime())
            });
        if !frameless {
            self.unresolved.insert(caller);
        }
    }
}

/// Return a new map of summaries with callee-induced escapes
/// folded in. The input *summaries* is the per-proc
/// (intra-procedural) result; the output is keyed identically
/// and is what codegen should consume.
///
/// Dialect-blind: the `pure_leaf` downgrade below classifies a callee with
/// no summary against plain Tcl. Callers holding the unit's registry should
/// use [`solve_interprocedural_escape_with_registry`] (#2179).
#[must_use]
pub fn solve_interprocedural_escape<S: BuildHasher>(
    summaries: &HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary, S>,
    graph: &EscapeCallGraph,
) -> HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary> {
    solve_interprocedural_escape_with_registry(summaries, graph, super::helpers::default_registry())
}

/// Registry-aware form of [`solve_interprocedural_escape`].
///
/// The registry decides which summary-less callees are frameless, and so
/// which callers keep `pure_leaf` — the inliner's predicate. `tcl8.6` marks
/// `lassign`, `lrepeat`, `lreverse`, `namespace`, `puts` and `throw`
/// frameless where `f5-irules` marks none of them, so the blind answer is
/// the permissive one.
#[must_use]
pub fn solve_interprocedural_escape_with_registry<S: BuildHasher>(
    summaries: &HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary, S>,
    graph: &EscapeCallGraph,
    _registry: &tcl_registry::CommandRegistry,
) -> HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary> {
    if summaries.is_empty() {
        return HashMap::new();
    }

    let resolved_callees: HashMap<_, _> = summaries
        .keys()
        .map(|caller| {
            let callees = graph
                .callees
                .get(caller)
                .into_iter()
                .flatten()
                .filter(|callee| summaries.contains_key(callee))
                .copied()
                .collect();
            (*caller, callees)
        })
        .collect();

    let (transitive_sources, transitive_unbounded) =
        propagate_transitive_sources(summaries, &resolved_callees);

    // Final `has_fallback` = intraproc has_fallback | (has_call_fallback &
    // not all-callees-resolve).
    let mut downgraded_fallback: HashMap<tcl_core_types::BodyFrameId, bool> = HashMap::new();
    for (qname, summary) in summaries {
        let call_fallback_final = summary.has_call_fallback() && graph.unresolved.contains(qname);
        downgraded_fallback.insert(qname.clone(), summary.has_fallback() || call_fallback_final);
    }

    // Materialise the final per-proc summary.
    let mut result: HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary> = HashMap::new();
    for (qname, summary) in summaries {
        let sources = transitive_sources[qname].clone();
        let unbounded = transitive_unbounded[qname];
        let pessimistic = unbounded && !summary.dynamic_barrier();
        let mut new_summary = summary.with_escapes(sources.iter().cloned(), pessimistic);
        new_summary.upvar_source_names = sources.into_iter().collect::<BTreeSet<_>>();
        new_summary
            .flags
            .set(EscapeFlags::UNBOUNDED_UPVAR_SOURCE, unbounded);
        new_summary
            .flags
            .set(EscapeFlags::HAS_FALLBACK, downgraded_fallback[qname]);
        result.insert(qname.clone(), new_summary);
    }

    downgrade_non_pure_leaf_callers(&mut result, graph);
    result
}

/// Worklist fixpoint that flows each proc's `upvar` source set (and the
/// unbounded-source flag) up to every transitive caller. Returns the
/// per-proc transitive source set and unbounded flag.
fn propagate_transitive_sources<S: BuildHasher>(
    summaries: &HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary, S>,
    resolved_callees: &HashMap<tcl_core_types::BodyFrameId, HashSet<tcl_core_types::BodyFrameId>>,
) -> (
    HashMap<tcl_core_types::BodyFrameId, HashSet<String>>,
    HashMap<tcl_core_types::BodyFrameId, bool>,
) {
    let mut transitive_sources: HashMap<tcl_core_types::BodyFrameId, HashSet<String>> = summaries
        .iter()
        .map(|(k, s)| (k.clone(), s.upvar_source_names.iter().cloned().collect()))
        .collect();
    let mut transitive_unbounded: HashMap<tcl_core_types::BodyFrameId, bool> = summaries
        .iter()
        .map(|(k, s)| (k.clone(), s.unbounded_upvar_source()))
        .collect();

    // Reverse edges: qname → set of qnames that call it.
    let mut callers_of: HashMap<tcl_core_types::BodyFrameId, HashSet<tcl_core_types::BodyFrameId>> =
        summaries
            .keys()
            .map(|k| (k.clone(), HashSet::new()))
            .collect();
    for (qname, callees) in resolved_callees {
        for callee in callees {
            callers_of
                .entry(callee.clone())
                .or_default()
                .insert(qname.clone());
        }
    }

    let mut worklist: VecDeque<tcl_core_types::BodyFrameId> = summaries.keys().cloned().collect();
    let mut in_worklist: HashSet<tcl_core_types::BodyFrameId> = summaries.keys().cloned().collect();
    while let Some(qname) = worklist.pop_front() {
        in_worklist.remove(&qname);
        let current_sources = transitive_sources[&qname].clone();
        let current_unbounded = transitive_unbounded[&qname];
        let callers = callers_of.get(&qname).cloned().unwrap_or_default();
        for caller in callers {
            let mut changed = false;
            let caller_set = transitive_sources
                .get_mut(&caller)
                .expect("caller in summaries");
            for s in &current_sources {
                if caller_set.insert(s.clone()) {
                    changed = true;
                }
            }
            if current_unbounded {
                let cu = transitive_unbounded
                    .get_mut(&caller)
                    .expect("caller in summaries");
                if !*cu {
                    *cu = true;
                    changed = true;
                }
            }
            if changed && !in_worklist.contains(&caller) {
                worklist.push_back(caller.clone());
                in_worklist.insert(caller);
            }
        }
    }
    (transitive_sources, transitive_unbounded)
}

/// Transitive `pure_leaf` fixpoint. A proc stays `pure_leaf` only
/// if every direct callee is itself `pure_leaf` — or an unresolved but
/// known frameless runtime builtin (`puts` / `list` / `string` / …), which
/// captures no caller locals and introduces no upvar alias, so a wrapper
/// around it is still safe to splice. Iterates to a fixpoint (bounded by
/// the proc count).
fn downgrade_non_pure_leaf_callers<S: BuildHasher>(
    result: &mut HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary, S>,
    graph: &EscapeCallGraph,
) {
    let mut changed = true;
    while changed {
        changed = false;
        let ids: Vec<_> = result.keys().copied().collect();
        for id in ids {
            if !result[&id].pure_leaf {
                continue;
            }
            let downgrade = graph.unresolved.contains(&id)
                || graph
                    .callees
                    .get(&id)
                    .into_iter()
                    .flatten()
                    .any(|callee| result.get(callee).is_none_or(|summary| !summary.pure_leaf));
            if downgrade {
                result
                    .get_mut(&id)
                    .expect("retained body in summaries")
                    .pure_leaf = false;
                changed = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::var_escape::types::EscapeTag;

    fn summary(direct_callees: &[&str], upvar_sources: &[&str]) -> ProcEscapeSummary {
        ProcEscapeSummary {
            direct_callees: direct_callees.iter().map(|s| (*s).to_string()).collect(),
            upvar_source_names: upvar_sources.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        }
    }

    fn fixture_solve(
        labels: &HashMap<String, ProcEscapeSummary>,
    ) -> HashMap<String, ProcEscapeSummary> {
        let issuer = tcl_core_types::ProcedureIdIssuer::default();
        let ids: HashMap<_, _> = labels
            .keys()
            .map(|label| {
                (
                    label.clone(),
                    tcl_core_types::BodyFrameId::Body(issuer.allocate()),
                )
            })
            .collect();
        let mut edges = Vec::new();
        let mut unresolved = Vec::new();
        for (label, summary) in labels {
            for callee in &summary.direct_callees {
                if let Some(id) = ids.get(callee) {
                    edges.push((ids[label], *id));
                } else {
                    unresolved.push(ids[label]);
                }
            }
        }
        let values: HashMap<_, _> = labels
            .iter()
            .map(|(label, summary)| (ids[label], summary.clone()))
            .collect();
        let graph = EscapeCallGraph::from_edges(edges, unresolved);
        let result = solve_interprocedural_escape(&values, &graph);
        labels
            .keys()
            .map(|label| (label.clone(), result[&ids[label]].clone()))
            .collect()
    }

    #[test]
    fn empty_input_returns_empty() {
        let result = solve_interprocedural_escape(&HashMap::new(), &EscapeCallGraph::default());
        assert!(result.is_empty());
    }

    #[test]
    fn caller_inherits_callee_upvar_sources() {
        // ::caller → ::leaf; ::leaf reaches caller frame's ``x``.
        let mut s = HashMap::new();
        s.insert("::caller".to_string(), summary(&["::leaf"], &[]));
        s.insert("::leaf".to_string(), summary(&[], &["x"]));
        let r = fixture_solve(&s);
        let caller = &r["::caller"];
        assert!(caller.upvar_source_names.contains("x"));
    }

    #[test]
    fn caller_with_matching_local_gets_frame_tag() {
        // ::caller has local ``x``; ::leaf names ``x`` as upvar
        // source. Result: caller's ``x`` is Frame.
        let mut s = HashMap::new();
        let mut caller = summary(&["::leaf"], &[]);
        caller.tags.insert("x".to_string(), EscapeTag::Local);
        s.insert("::caller".to_string(), caller);
        s.insert("::leaf".to_string(), summary(&[], &["x"]));
        let r = fixture_solve(&s);
        assert!(r["::caller"].is_frame("x"));
    }

    #[test]
    fn unbounded_callee_marks_caller_pessimistic() {
        let mut s = HashMap::new();
        s.insert("::caller".to_string(), summary(&["::leaf"], &[]));
        let mut leaf = summary(&[], &[]);
        leaf.flags.insert(EscapeFlags::UNBOUNDED_UPVAR_SOURCE);
        s.insert("::leaf".to_string(), leaf);
        let r = fixture_solve(&s);
        assert!(r["::caller"].dynamic_barrier());
        assert!(r["::caller"].unbounded_upvar_source());
    }

    #[test]
    fn transitive_through_chain() {
        // a → b → c; c reaches caller frame's ``x``. After fixpoint,
        // both a and b carry ``x``.
        let mut s = HashMap::new();
        s.insert("::a".to_string(), summary(&["::b"], &[]));
        s.insert("::b".to_string(), summary(&["::c"], &[]));
        s.insert("::c".to_string(), summary(&[], &["x"]));
        let r = fixture_solve(&s);
        assert!(r["::a"].upvar_source_names.contains("x"));
        assert!(r["::b"].upvar_source_names.contains("x"));
    }

    #[test]
    fn has_call_fallback_downgrades_when_all_callees_resolve() {
        let mut s = HashMap::new();
        let mut caller = summary(&["::leaf"], &[]);
        caller.flags.insert(EscapeFlags::HAS_CALL_FALLBACK);
        s.insert("::caller".to_string(), caller);
        s.insert("::leaf".to_string(), summary(&[], &[]));
        let r = fixture_solve(&s);
        // ::leaf is a tracked proc, so the call_fallback downgrades
        // and final has_fallback stays false (intraproc didn't set
        // has_fallback either).
        assert!(!r["::caller"].has_fallback());
    }

    #[test]
    fn has_call_fallback_keeps_set_when_callee_unresolved() {
        let mut s = HashMap::new();
        let mut caller = summary(&["unknown_external"], &[]);
        caller.flags.insert(EscapeFlags::HAS_CALL_FALLBACK);
        s.insert("::caller".to_string(), caller);
        let r = fixture_solve(&s);
        // No tracked proc resolves ``unknown_external`` → final
        // has_fallback is true.
        assert!(r["::caller"].has_fallback());
    }

    #[test]
    fn has_fallback_intraproc_value_preserved() {
        let mut s = HashMap::new();
        let mut caller = summary(&[], &[]);
        caller.flags.insert(EscapeFlags::HAS_FALLBACK);
        s.insert("::caller".to_string(), caller);
        let r = fixture_solve(&s);
        assert!(r["::caller"].has_fallback());
    }

    #[test]
    fn self_recursion_does_not_loop() {
        // ::recur → ::recur — the resolver drops self-edges.
        let mut s = HashMap::new();
        s.insert("::recur".to_string(), summary(&["::recur"], &["x"]));
        let r = fixture_solve(&s);
        assert!(r["::recur"].upvar_source_names.contains("x"));
    }
}
