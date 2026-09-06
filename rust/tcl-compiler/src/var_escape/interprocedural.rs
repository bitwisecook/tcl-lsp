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

use super::helpers::is_frameless_runtime_command;
use std::hash::BuildHasher;

use crate::var_escape::types::{EscapeFlags, ProcEscapeSummary};

/// Resolve a call's head word to the qname of a tracked proc using the
/// compiler's shared Tcl command-resolution rule.
/// Returns `None` for builtins / unknown commands (absent from
/// `summaries`). Exposed `pub(crate)` so the inliner (`crate::inlining`)
/// resolves call sites with the same rules.
pub(crate) fn resolve_callee<S: BuildHasher>(
    command: &str,
    caller_qname: &str,
    summaries: &HashMap<String, ProcEscapeSummary, S>,
) -> Option<String> {
    crate::interprocedural::resolve_internal_call_with(command, caller_qname, |qname| {
        summaries.contains_key(qname)
    })
}

/// Return a new map of summaries with callee-induced escapes
/// folded in. The input *summaries* is the per-proc
/// (intra-procedural) result; the output is keyed identically
/// and is what codegen should consume.
#[must_use]
pub fn solve_interprocedural_escape<S: BuildHasher>(
    summaries: &HashMap<String, ProcEscapeSummary, S>,
) -> HashMap<String, ProcEscapeSummary> {
    if summaries.is_empty() {
        return HashMap::new();
    }

    // Resolve each callee; drop ones absent from `summaries` (builtins —
    // no escape — or unknowns handled by the intraproc pessimistic fallback).
    let mut resolved_callees: HashMap<String, HashSet<String>> = HashMap::new();
    for (qname, summary) in summaries {
        let mut callees: HashSet<String> = HashSet::new();
        for cmd in &summary.direct_callees {
            if let Some(resolved) = resolve_callee(cmd, qname, summaries)
                && resolved != *qname
            {
                callees.insert(resolved);
            }
        }
        resolved_callees.insert(qname.clone(), callees);
    }

    let (transitive_sources, transitive_unbounded) =
        propagate_transitive_sources(summaries, &resolved_callees);

    // Final `has_fallback` = intraproc has_fallback | (has_call_fallback &
    // not all-callees-resolve).
    let mut downgraded_fallback: HashMap<String, bool> = HashMap::new();
    for (qname, summary) in summaries {
        let call_fallback_final = if summary.has_call_fallback() {
            !summary
                .direct_callees
                .iter()
                .all(|cmd| resolve_callee(cmd, qname, summaries).is_some())
        } else {
            false
        };
        downgraded_fallback.insert(qname.clone(), summary.has_fallback() || call_fallback_final);
    }

    // Materialise the final per-proc summary.
    let mut result: HashMap<String, ProcEscapeSummary> = HashMap::new();
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

    downgrade_non_pure_leaf_callers(&mut result);
    result
}

/// Worklist fixpoint that flows each proc's `upvar` source set (and the
/// unbounded-source flag) up to every transitive caller. Returns the
/// per-proc transitive source set and unbounded flag.
fn propagate_transitive_sources<S: BuildHasher>(
    summaries: &HashMap<String, ProcEscapeSummary, S>,
    resolved_callees: &HashMap<String, HashSet<String>>,
) -> (HashMap<String, HashSet<String>>, HashMap<String, bool>) {
    let mut transitive_sources: HashMap<String, HashSet<String>> = summaries
        .iter()
        .map(|(k, s)| (k.clone(), s.upvar_source_names.iter().cloned().collect()))
        .collect();
    let mut transitive_unbounded: HashMap<String, bool> = summaries
        .iter()
        .map(|(k, s)| (k.clone(), s.unbounded_upvar_source()))
        .collect();

    // Reverse edges: qname → set of qnames that call it.
    let mut callers_of: HashMap<String, HashSet<String>> = summaries
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

    let mut worklist: VecDeque<String> = summaries.keys().cloned().collect();
    let mut in_worklist: HashSet<String> = summaries.keys().cloned().collect();
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
    result: &mut HashMap<String, ProcEscapeSummary, S>,
) {
    let mut changed = true;
    while changed {
        changed = false;
        let qnames: Vec<String> = result.keys().cloned().collect();
        for qname in qnames {
            if !result[&qname].pure_leaf {
                continue;
            }
            let callees: Vec<String> = result[&qname].direct_callees.iter().cloned().collect();
            let downgrade = callees.iter().any(|callee| {
                result.get(callee).map_or_else(
                    || {
                        let bare = callee.strip_prefix("::").unwrap_or(callee);
                        !is_frameless_runtime_command(bare)
                    },
                    |c| !c.pure_leaf,
                )
            });
            if downgrade {
                result.get_mut(&qname).expect("qname in result").pure_leaf = false;
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

    #[test]
    fn empty_input_returns_empty() {
        let result = solve_interprocedural_escape(&HashMap::new());
        assert!(result.is_empty());
    }

    #[test]
    fn caller_inherits_callee_upvar_sources() {
        // ::caller → ::leaf; ::leaf reaches caller frame's ``x``.
        let mut s = HashMap::new();
        s.insert("::caller".to_string(), summary(&["::leaf"], &[]));
        s.insert("::leaf".to_string(), summary(&[], &["x"]));
        let r = solve_interprocedural_escape(&s);
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
        let r = solve_interprocedural_escape(&s);
        assert!(r["::caller"].is_frame("x"));
    }

    #[test]
    fn unbounded_callee_marks_caller_pessimistic() {
        let mut s = HashMap::new();
        s.insert("::caller".to_string(), summary(&["::leaf"], &[]));
        let mut leaf = summary(&[], &[]);
        leaf.flags.insert(EscapeFlags::UNBOUNDED_UPVAR_SOURCE);
        s.insert("::leaf".to_string(), leaf);
        let r = solve_interprocedural_escape(&s);
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
        let r = solve_interprocedural_escape(&s);
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
        let r = solve_interprocedural_escape(&s);
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
        let r = solve_interprocedural_escape(&s);
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
        let r = solve_interprocedural_escape(&s);
        assert!(r["::caller"].has_fallback());
    }

    #[test]
    fn self_recursion_does_not_loop() {
        // ::recur → ::recur — the resolver drops self-edges.
        let mut s = HashMap::new();
        s.insert("::recur".to_string(), summary(&["::recur"], &["x"]));
        let r = solve_interprocedural_escape(&s);
        assert!(r["::recur"].upvar_source_names.contains("x"));
    }
}
