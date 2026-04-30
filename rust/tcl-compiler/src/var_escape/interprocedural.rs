//! Interprocedural propagation of var-escape summaries (C33d).
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
//!
//! Mirrors `core/compiler/var_escape/_interprocedural.py`.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::BuildHasher;

use crate::var_escape::types::ProcEscapeSummary;

fn caller_namespace(qname: &str) -> String {
    if qname.is_empty() || qname == "::" {
        return "::".to_string();
    }
    if !qname.starts_with("::") {
        return "::".to_string();
    }
    let trimmed = &qname[2..];
    match trimmed.rfind("::") {
        Some(idx) => format!("::{}", &trimmed[..idx]),
        None => "::".to_string(),
    }
}

/// Return the candidate qualified names *command* might resolve
/// to in the summary table when called from *`caller_qname`*.
///
/// Tcl name lookup for a bare call word searches the caller's
/// namespace, walks up each enclosing namespace, then the global
/// namespace, then the raw bare form. An already-qualified call
/// (starts with `::`) skips the namespace walk.
fn name_candidates(command: &str, caller_qname: &str) -> Vec<String> {
    if command.starts_with("::") {
        return vec![command.to_string()];
    }
    let mut candidates: Vec<String> = Vec::new();
    let mut ns = caller_namespace(caller_qname);
    loop {
        let prefix = if ns == "::" { "" } else { ns.as_str() };
        let candidate = if prefix.is_empty() {
            format!("::{command}")
        } else {
            format!("{prefix}::{command}")
        };
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
        if ns == "::" {
            break;
        }
        ns = caller_namespace(&ns);
    }
    if !candidates.iter().any(|c| c == command) {
        candidates.push(command.to_string());
    }
    candidates
}

fn resolve_callee<S: BuildHasher>(
    command: &str,
    caller_qname: &str,
    summaries: &HashMap<String, ProcEscapeSummary, S>,
) -> Option<String> {
    name_candidates(command, caller_qname)
        .into_iter()
        .find(|c| summaries.contains_key(c))
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

    // Resolve each callee. Drop ones that don't appear in
    // ``summaries`` — those are either builtins (no escape) or
    // unknowns (handled by the intra-procedural pessimistic
    // fallback).
    let mut resolved_callees: HashMap<String, HashSet<String>> = HashMap::new();
    for (qname, summary) in summaries {
        let mut callees: HashSet<String> = HashSet::new();
        for cmd in &summary.direct_callees {
            if let Some(resolved) = resolve_callee(cmd, qname, summaries) {
                if resolved != *qname {
                    callees.insert(resolved);
                }
            }
        }
        resolved_callees.insert(qname.clone(), callees);
    }

    // Worklist fixpoint over transitive sources.
    let mut transitive_sources: HashMap<String, HashSet<String>> = summaries
        .iter()
        .map(|(k, s)| {
            (
                k.clone(),
                s.upvar_source_names.iter().cloned().collect::<HashSet<_>>(),
            )
        })
        .collect();
    let mut transitive_unbounded: HashMap<String, bool> = summaries
        .iter()
        .map(|(k, s)| (k.clone(), s.unbounded_upvar_source))
        .collect();

    // Reverse edges: qname → set of qnames that call it.
    let mut callers_of: HashMap<String, HashSet<String>> = summaries
        .keys()
        .map(|k| (k.clone(), HashSet::new()))
        .collect();
    for (qname, callees) in &resolved_callees {
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

    // Derive the final ``has_fallback`` for each proc.
    //   final = intraproc has_fallback
    //         | (has_call_fallback & not all-callees-resolve)
    let mut downgraded_fallback: HashMap<String, bool> = HashMap::new();
    for (qname, summary) in summaries {
        let call_fallback_final = if summary.has_call_fallback {
            !summary
                .direct_callees
                .iter()
                .all(|cmd| resolve_callee(cmd, qname, summaries).is_some())
        } else {
            false
        };
        downgraded_fallback.insert(qname.clone(), summary.has_fallback || call_fallback_final);
    }

    // Materialise the final per-proc summary.
    let mut result: HashMap<String, ProcEscapeSummary> = HashMap::new();
    for (qname, summary) in summaries {
        let sources = transitive_sources[qname].clone();
        let unbounded = transitive_unbounded[qname];
        let pessimistic = unbounded && !summary.dynamic_barrier;
        let mut new_summary = summary.with_escapes(sources.iter().cloned(), pessimistic);
        new_summary.upvar_source_names = sources.into_iter().collect::<BTreeSet<_>>();
        new_summary.unbounded_upvar_source = unbounded;
        new_summary.has_fallback = downgraded_fallback[qname];
        result.insert(qname.clone(), new_summary);
    }
    result
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
    fn caller_namespace_handles_global() {
        assert_eq!(caller_namespace("::"), "::");
        assert_eq!(caller_namespace("::foo"), "::");
        assert_eq!(caller_namespace("::ns::foo"), "::ns");
        assert_eq!(caller_namespace("::a::b::c"), "::a::b");
        assert_eq!(caller_namespace("bare"), "::");
    }

    #[test]
    fn name_candidates_qualified_call_returns_self() {
        let cands = name_candidates("::ns::leaf", "::caller");
        assert_eq!(cands, vec!["::ns::leaf"]);
    }

    #[test]
    fn name_candidates_walks_namespaces_for_bare_call() {
        let cands = name_candidates("leaf", "::a::b::caller");
        // First the immediate ns, then walk up, then global, then bare.
        assert!(cands.contains(&"::a::b::leaf".to_string()));
        assert!(cands.contains(&"::a::leaf".to_string()));
        assert!(cands.contains(&"::leaf".to_string()));
        assert!(cands.contains(&"leaf".to_string()));
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
        leaf.unbounded_upvar_source = true;
        s.insert("::leaf".to_string(), leaf);
        let r = solve_interprocedural_escape(&s);
        assert!(r["::caller"].dynamic_barrier);
        assert!(r["::caller"].unbounded_upvar_source);
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
        caller.has_call_fallback = true;
        s.insert("::caller".to_string(), caller);
        s.insert("::leaf".to_string(), summary(&[], &[]));
        let r = solve_interprocedural_escape(&s);
        // ::leaf is a tracked proc, so the call_fallback downgrades
        // and final has_fallback stays false (intraproc didn't set
        // has_fallback either).
        assert!(!r["::caller"].has_fallback);
    }

    #[test]
    fn has_call_fallback_keeps_set_when_callee_unresolved() {
        let mut s = HashMap::new();
        let mut caller = summary(&["unknown_external"], &[]);
        caller.has_call_fallback = true;
        s.insert("::caller".to_string(), caller);
        let r = solve_interprocedural_escape(&s);
        // No tracked proc resolves ``unknown_external`` → final
        // has_fallback is true.
        assert!(r["::caller"].has_fallback);
    }

    #[test]
    fn has_fallback_intraproc_value_preserved() {
        let mut s = HashMap::new();
        let mut caller = summary(&[], &[]);
        caller.has_fallback = true;
        s.insert("::caller".to_string(), caller);
        let r = solve_interprocedural_escape(&s);
        assert!(r["::caller"].has_fallback);
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
