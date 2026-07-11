#![allow(
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

    clippy::implicit_hasher,
    clippy::doc_markdown,
    clippy::doc_lazy_continuation
)]

//! TclOO method resolution order.
//!
//! Implements the same MRO algorithm as Tcl 8.6 / 9.0's
//! ``tclOOCall.c``:
//!
//! 1. Two-pass DFS matching ``TclOOGetCallContext()``:
//!    - Pass 1 (``BUILDING_MIXINS``): traverse the hierarchy but
//!      only collect classes reached via a mixin edge
//!      (``TRAVERSED_MIXIN``).
//!    - Pass 2: traverse again but only collect classes reached
//!      via non-mixin edges (superclass relationships).
//!    This guarantees all mixin-path classes precede all non-mixin
//!    classes in the final chain.
//!
//! 2. Within each pass, the traversal matches
//!    ``AddSimpleClassChainToCallContext()``: process class-level
//!    mixins first, then add the class itself, then recurse into
//!    superclasses.
//!
//! 3. Late-placement deduplication from
//!    ``AddMethodToCallChain()``: when a class is encountered
//!    again it is *moved to the end* of the list (not skipped).
//!    The comment in `tclOOCall.c` says "methods come as *late*
//!    in the call chain as possible".
//!
//! The algorithm is **identical** between Tcl 8.6 and 9.0; the
//! only 9.0 change is TIP 500 private method support which does
//! not affect traversal order.
//!
//! Reference: ``AddSimpleClassChainToCallContext()`` and
//! ``TclOOGetCallContext()`` in ``generic/tclOOCall.c``.

use std::collections::{HashMap, HashSet};

/// MRO computation error — currently surfaces only when a pure
/// superclass cycle (no mixin edge) is detected.  Mixin cycles
/// are valid in TclOO (e.g. a mixin whose superclass is the
/// class being mixed into) and silently terminate the DFS at
/// the revisit point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MroError {
    /// Human-readable error message.
    pub message: String,
}

impl std::fmt::Display for MroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MroError {}

/// Recursive DFS matching TclOO's
/// `AddSimpleClassChainToCallContext`.
///
/// For each class:
/// 1. Recurse into class-level mixins (setting mixin-path flag).
/// 2. Add the class itself if `MIXIN_CONSISTENT` allows it.
/// 3. Recurse into superclasses (inheriting current mixin-path
///    status).
///
/// Per-pass context for [`tcloo_dfs`].
///
/// Bundles the immutable maps that drive the walk
/// (`mixins_map` / `supers_map`), the mutable accumulator
/// (`result` / `visiting`), and the per-pass `building_mixins`
/// flag.  Only `cls` and `is_mixin_path` vary per recursive
/// call — they stay as direct parameters.
///
/// `building_mixins` corresponds to Tcl's ``BUILDING_MIXINS``:
/// when set, only mixin-path classes are added; when clear,
/// only non-mixin-path classes are added.
struct DfsCtx<'a> {
    mixins_map: &'a HashMap<String, Vec<String>>,
    supers_map: &'a HashMap<String, Vec<String>>,
    result: &'a mut Vec<String>,
    visiting: &'a mut HashSet<String>,
    building_mixins: bool,
    /// Remaining node-visit budget. TclOO's late-placement (remove-and-repush
    /// on re-visit) is order-significant, so a shared sub-DAG is legitimately
    /// re-explored once per reaching path — Θ(2^k) on k stacked diamonds. The
    /// budget bounds that adversarial blow-up without changing the result for
    /// realistic hierarchies (RUST_ISSUE_076).
    budget: usize,
    /// Set when the depth cap or visit budget is exceeded; the linearisation is
    /// then abandoned as too complex rather than hung or stack-overflowed.
    aborted: bool,
}

/// Maximum superclass/mixin recursion depth. A real class hierarchy is never
/// remotely this deep; the cap keeps a pathological (e.g. ~20k-node) linear
/// chain from overflowing the stack. Mirrors `param_traits::MAX_DEPTH`.
const MAX_MRO_DEPTH: usize = 1024;

/// Maximum total node visits during one linearisation. Bounds the Θ(2^k)
/// re-exploration of stacked diamonds (a legal but adversarial shape) so the
/// diagnostics pass degrades gracefully instead of hanging.
const MAX_MRO_VISITS: usize = 200_000;

/// `is_mixin_path` corresponds to Tcl's ``TRAVERSED_MIXIN``:
/// `true` when the class was reached via a mixin edge.
fn tcloo_dfs(cls: &str, ctx: &mut DfsCtx<'_>, is_mixin_path: bool, depth: usize) {
    // Bound the walk: an over-deep chain would overflow the stack and stacked
    // diamonds would explode the visit count. Abort gracefully (RUST_ISSUE_076).
    if depth > MAX_MRO_DEPTH || ctx.budget == 0 {
        ctx.aborted = true;
        return;
    }
    ctx.budget -= 1;

    if ctx.visiting.contains(cls) {
        // Cycles through mixins are valid in TclOO; just skip.
        return;
    }
    ctx.visiting.insert(cls.to_string());

    // 1. Process class-level mixins (enter mixin path).
    if let Some(mixins) = ctx.mixins_map.get(cls) {
        for mixin in mixins.clone() {
            tcloo_dfs(&mixin, ctx, true, depth + 1);
        }
    }

    // 2. Add own class with MIXIN_CONSISTENT gate:
    //    Pass 1 (building_mixins=true):  only add if on mixin path
    //    Pass 2 (building_mixins=false): only add if NOT on mixin path
    if ctx.building_mixins == is_mixin_path {
        if let Some(pos) = ctx.result.iter().position(|c| c == cls) {
            ctx.result.remove(pos);
        }
        ctx.result.push(cls.to_string());
    }

    // 3. Process superclasses (inherit mixin-path status).
    if let Some(supers) = ctx.supers_map.get(cls) {
        for parent in supers.clone() {
            tcloo_dfs(&parent, ctx, is_mixin_path, depth + 1);
        }
    }

    ctx.visiting.remove(cls);
}

/// Detect a pure superclass cycle that `start` itself participates in.
///
/// TclOO's DFS silently skips visited nodes (needed for mixin
/// cycles), but a pure superclass cycle *through `start`* should be reported
/// as an error so downstream consumers can surface the problem.  A cycle that
/// is merely *reachable* from `start` — among its ancestors, not including
/// `start` — is not `start`'s error: reporting it would abandon `start`'s
/// whole MRO (including acyclic parents) and mis-fire W308 on genuinely
/// inherited methods (issue 155).
fn has_super_cycle(start: &str, supers_map: &HashMap<String, Vec<String>>) -> bool {
    // Two sets, not one: `on_path` is the current DFS stack (a back-edge to it
    // is a real cycle) and `explored` memoises nodes fully proven acyclic (so a
    // shared sub-DAG is visited once, not once per reaching path — the previous
    // single path-scoped set was Θ(2^k) on k stacked diamonds; RUST_ISSUE_076).
    // A depth over `MAX_MRO_DEPTH` is treated as a cycle rather than overflowing
    // the stack on a pathological linear chain.
    fn recurse(
        cls: &str,
        start: &str,
        supers_map: &HashMap<String, Vec<String>>,
        on_path: &mut HashSet<String>,
        explored: &mut HashSet<String>,
        depth: usize,
    ) -> bool {
        if depth > MAX_MRO_DEPTH {
            return true;
        }
        if on_path.contains(cls) {
            // A back-edge marks a cycle, but only a cycle that returns to
            // `start` means `start` *itself* participates in one.  A cycle
            // among ancestors that does not include `start` (`C : A`, with
            // `A ↔ B` mutually super-classed) must not fail `start`'s own
            // linearisation — the MRO DFS breaks such a cycle via its visited
            // set, letting `start` still resolve its acyclic parents (issue
            // 155).  `start` is on the path for the whole walk (it is the
            // root), so any descendant that can reach it is caught here.
            return cls == start;
        }
        if explored.contains(cls) {
            return false; // already proven to not reach `start` from here
        }
        on_path.insert(cls.to_string());
        if let Some(parents) = supers_map.get(cls) {
            for parent in parents {
                if recurse(parent, start, supers_map, on_path, explored, depth + 1) {
                    return true;
                }
            }
        }
        on_path.remove(cls);
        explored.insert(cls.to_string());
        false
    }
    let mut on_path = HashSet::new();
    let mut explored = HashSet::new();
    recurse(start, start, supers_map, &mut on_path, &mut explored, 0)
}

/// Return the method resolution order for `class_name`.
///
/// Uses TclOO's two-pass DFS + late-placement algorithm.
/// `superclasses_map` maps class name → direct superclasses;
/// `mixins_map` maps class name → mixin classes (processed
/// before supers).  Returns the linearised chain with mixin-path
/// classes first, then non-mixin classes.
///
/// Raises [`MroError`] on pure superclass cycles.
///
/// # Errors
///
/// Returns an [`MroError`] when a pure superclass cycle (no
/// mixin edge) is detected.  Cycles through mixins are not
/// errors — they're valid in TclOO and the DFS terminates
/// silently at the revisit point.
pub fn tcloo_linearise(
    class_name: &str,
    superclasses_map: &HashMap<String, Vec<String>>,
    mixins_map: &HashMap<String, Vec<String>>,
) -> Result<Vec<String>, MroError> {
    if has_super_cycle(class_name, superclasses_map) {
        return Err(MroError {
            message: format!("cycle detected in superclass hierarchy for {class_name}"),
        });
    }

    let mut result: Vec<String> = Vec::new();
    let mut budget = MAX_MRO_VISITS;

    // Pass 1: BUILDING_MIXINS — only collect mixin-path classes.
    let mut visiting = HashSet::new();
    let mut ctx = DfsCtx {
        mixins_map,
        supers_map: superclasses_map,
        result: &mut result,
        visiting: &mut visiting,
        building_mixins: true,
        budget,
        aborted: false,
    };
    tcloo_dfs(class_name, &mut ctx, false, 0);
    if ctx.aborted {
        return Err(MroError {
            message: format!("class hierarchy for {class_name} is too complex to linearise"),
        });
    }
    budget = ctx.budget;

    // Pass 2: collect non-mixin-path classes (shares result list
    // for late-placement dedup).
    let mut visiting = HashSet::new();
    let mut ctx = DfsCtx {
        mixins_map,
        supers_map: superclasses_map,
        result: &mut result,
        visiting: &mut visiting,
        building_mixins: false,
        budget,
        aborted: false,
    };
    tcloo_dfs(class_name, &mut ctx, false, 0);
    if ctx.aborted {
        return Err(MroError {
            message: format!("class hierarchy for {class_name} is too complex to linearise"),
        });
    }

    Ok(result)
}

/// Compute MRO for every class in the hierarchy.
///
/// Returns `(mro_map, errors)` where `mro_map` maps each class
/// name to its linearised MRO and `errors` is a list of error
/// messages for classes whose hierarchy is inconsistent.
#[must_use]
pub fn build_mro_map(
    superclasses_map: &HashMap<String, Vec<String>>,
    mixins_map: &HashMap<String, Vec<String>>,
) -> (HashMap<String, Vec<String>>, Vec<String>) {
    let mut mro_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut errors: Vec<String> = Vec::new();
    for cls in superclasses_map.keys() {
        if mro_map.contains_key(cls) {
            continue;
        }
        match tcloo_linearise(cls, superclasses_map, mixins_map) {
            Ok(mro) => {
                mro_map.insert(cls.clone(), mro);
            }
            Err(e) => {
                errors.push(e.message);
            }
        }
    }
    (mro_map, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supers(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(k, v)| {
                (
                    (*k).to_string(),
                    v.iter().map(|s| (*s).to_string()).collect(),
                )
            })
            .collect()
    }

    fn empty_mixins() -> HashMap<String, Vec<String>> {
        HashMap::new()
    }

    #[test]
    fn single_class_no_parents() {
        let s = supers(&[("A", &[])]);
        let mro = tcloo_linearise("A", &s, &empty_mixins()).unwrap();
        assert_eq!(mro, vec!["A"]);
    }

    #[test]
    fn single_inheritance() {
        let s = supers(&[("B", &["A"]), ("A", &[])]);
        let mro = tcloo_linearise("B", &s, &empty_mixins()).unwrap();
        assert_eq!(mro, vec!["B", "A"]);
    }

    #[test]
    fn chain_of_three() {
        let s = supers(&[("C", &["B"]), ("B", &["A"]), ("A", &[])]);
        let mro = tcloo_linearise("C", &s, &empty_mixins()).unwrap();
        assert_eq!(mro, vec!["C", "B", "A"]);
    }

    #[test]
    fn diamond_inheritance() {
        // ``D → B, C → A`` — TclOO's late-placement puts ``A``
        // after ``C`` (instead of after ``B``), because ``A`` is
        // re-visited via C and moved to the end.
        let s = supers(&[("D", &["B", "C"]), ("B", &["A"]), ("C", &["A"]), ("A", &[])]);
        let mro = tcloo_linearise("D", &s, &empty_mixins()).unwrap();
        assert_eq!(mro, vec!["D", "B", "C", "A"]);
    }

    #[test]
    fn cycle_raises_mro_error() {
        let s = supers(&[("A", &["B"]), ("B", &["A"])]);
        let err = tcloo_linearise("A", &s, &empty_mixins()).unwrap_err();
        assert!(err.message.contains("cycle"));
    }

    #[test]
    fn self_cycle_raises_mro_error() {
        let s = supers(&[("A", &["A"])]);
        let err = tcloo_linearise("A", &s, &empty_mixins()).unwrap_err();
        assert!(err.message.contains("cycle"));
    }

    #[test]
    fn class_above_an_unrelated_cycle_still_linearises() {
        // `A ↔ B` is a real cycle; `C : {A, Base}` merely *reaches* it but is
        // not part of it. Linearising C must NOT error — the DFS breaks the
        // A/B cycle via its visited set, so C still resolves `Base` (and its
        // methods), rather than dropping C's whole MRO and firing a spurious
        // W308 (issue 155).
        let s = supers(&[
            ("A", &["B"]),
            ("B", &["A"]),
            ("C", &["A", "Base"]),
            ("Base", &[]),
        ]);
        // A and B themselves are in the cycle → still an error.
        assert!(tcloo_linearise("A", &s, &empty_mixins()).is_err());
        assert!(tcloo_linearise("B", &s, &empty_mixins()).is_err());
        // C is above the cycle, not in it → linearises, and Base is present.
        let mro = tcloo_linearise("C", &s, &empty_mixins())
            .expect("C is not itself part of the A/B cycle");
        assert!(mro.contains(&"C".to_string()));
        assert!(
            mro.contains(&"Base".to_string()),
            "acyclic parent must resolve: {mro:?}"
        );
    }

    /// `k` stacked diamonds: `L{i} → {A{i}, B{i}}`, both `A{i}` and `B{i}`
    /// point at `L{i+1}`. The bottom `L{k}` is reachable by 2^k paths — the
    /// shape that made the old path-scoped DFS explode (RUST_ISSUE_076).
    fn stacked_diamonds(k: usize) -> HashMap<String, Vec<String>> {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        for i in 0..k {
            m.insert(format!("L{i}"), vec![format!("A{i}"), format!("B{i}")]);
            m.insert(format!("A{i}"), vec![format!("L{}", i + 1)]);
            m.insert(format!("B{i}"), vec![format!("L{}", i + 1)]);
        }
        m.insert(format!("L{k}"), vec![]);
        m
    }

    #[test]
    fn moderate_stacked_diamonds_linearise_ok() {
        // FP-guard: an acyclic shared DAG (10 diamonds) must NOT be reported as
        // a cycle, and must linearise successfully within the visit budget.
        let s = stacked_diamonds(10);
        let mro = tcloo_linearise("L0", &s, &empty_mixins()).expect("acyclic DAG must linearise");
        assert_eq!(mro.first().map(String::as_str), Some("L0"));
        assert_eq!(mro.last().map(String::as_str), Some("L10"));
    }

    #[test]
    fn adversarial_stacked_diamonds_do_not_hang() {
        // RUST_ISSUE_076: ~40 stacked diamonds is 2^40 paths — the old DFS
        // would hang. The visit budget makes it abort gracefully with a
        // "too complex" error. The test completing at all is the assertion.
        let s = stacked_diamonds(40);
        let err = tcloo_linearise("L0", &s, &empty_mixins()).unwrap_err();
        assert!(err.message.contains("too complex"), "{}", err.message);
    }

    #[test]
    fn deep_linear_chain_does_not_overflow() {
        // RUST_ISSUE_076: a ~5000-deep superclass chain would overflow the
        // stack. The depth cap bounds recursion (reported as a cycle rather
        // than a crash). Completing without a stack overflow is the point.
        let mut s: HashMap<String, Vec<String>> = HashMap::new();
        for i in 0..5000 {
            s.insert(format!("C{i}"), vec![format!("C{}", i + 1)]);
        }
        s.insert("C5000".to_string(), vec![]);
        let result = tcloo_linearise("C0", &s, &empty_mixins());
        assert!(
            result.is_err(),
            "over-deep chain must degrade, not overflow"
        );
    }

    #[test]
    fn unknown_parent_treated_as_leaf() {
        // Parent ``Z`` not in the supers map — DFS just visits
        // it and stops.  No error.
        let s = supers(&[("A", &["Z"])]);
        let mro = tcloo_linearise("A", &s, &empty_mixins()).unwrap();
        assert_eq!(mro, vec!["A", "Z"]);
    }

    #[test]
    fn mixin_before_class_oo_14_8() {
        // OO 14.8: mixin path classes precede non-mixin classes.
        // ``B`` mixes ``M``; ``M`` and ``A`` both via mixin
        // means M's chain comes first.
        let s = supers(&[("B", &["A"]), ("M", &[]), ("A", &[])]);
        let mut mixins = HashMap::new();
        mixins.insert("B".to_string(), vec!["M".to_string()]);
        let mro = tcloo_linearise("B", &s, &mixins).unwrap();
        assert_eq!(mro, vec!["M", "B", "A"]);
    }

    #[test]
    fn wshape_multi_inheritance_matches_tclsh() {
        // A W-shaped diamond that stresses late-placement across two
        // multi-parent classes sharing a middle base:
        //   Z → {K1, K2};  K1 → {AA, BB};  K2 → {BB, CC}
        // Verified against real TclOO (`tclsh9.0`, `info class call`):
        //   ::Z ::K1 ::AA ::K2 ::BB ::CC
        // `BB` (first reached via K1) is moved *after* K2 when re-reached via
        // K2's chain — the TclOO "as late as possible" placement.
        let s = supers(&[
            ("Z", &["K1", "K2"]),
            ("K1", &["AA", "BB"]),
            ("K2", &["BB", "CC"]),
            ("AA", &[]),
            ("BB", &[]),
            ("CC", &[]),
        ]);
        let mro = tcloo_linearise("Z", &s, &empty_mixins()).unwrap();
        assert_eq!(mro, vec!["Z", "K1", "AA", "K2", "BB", "CC"]);
    }

    #[test]
    fn c3_inconsistent_hierarchy_resolves_like_tclsh() {
        // The decisive TclOO-vs-C3 case: `X → A,B` and `Y → B,A` disagree on the
        // A/B order, so Python/Perl **C3 linearisation raises an error**. TclOO's
        // DFS + late-placement instead resolves it deterministically. Verified
        // against real TclOO (`tclsh9.0`, `info class call`):
        //   ::Z ::X ::Y ::B ::A
        // Confirms we implement TclOO's actual algorithm, not C3.
        let s = supers(&[
            ("Z", &["X", "Y"]),
            ("X", &["A", "B"]),
            ("Y", &["B", "A"]),
            ("A", &[]),
            ("B", &[]),
        ]);
        let mro = tcloo_linearise("Z", &s, &empty_mixins()).unwrap();
        assert_eq!(mro, vec!["Z", "X", "Y", "B", "A"]);
    }

    #[test]
    fn no_mixins_same_as_supers_only() {
        // With empty mixins, MRO = parent chain.
        let s = supers(&[("D", &["C"]), ("C", &["B"]), ("B", &["A"]), ("A", &[])]);
        let mro = tcloo_linearise("D", &s, &empty_mixins()).unwrap();
        assert_eq!(mro, vec!["D", "C", "B", "A"]);
    }

    #[test]
    fn build_mro_map_all_classes() {
        let s = supers(&[("A", &[]), ("B", &["A"]), ("C", &["B"])]);
        let (mro_map, errors) = build_mro_map(&s, &empty_mixins());
        assert!(errors.is_empty());
        assert_eq!(mro_map["A"], vec!["A"]);
        assert_eq!(mro_map["B"], vec!["B", "A"]);
        assert_eq!(mro_map["C"], vec!["C", "B", "A"]);
    }

    #[test]
    fn build_mro_map_collects_errors_for_cycles() {
        let s = supers(&[("A", &["B"]), ("B", &["A"]), ("C", &[])]);
        let (mro_map, errors) = build_mro_map(&s, &empty_mixins());
        assert!(!errors.is_empty());
        // C is independent — should still get a clean MRO.
        assert_eq!(mro_map["C"], vec!["C"]);
    }
}
