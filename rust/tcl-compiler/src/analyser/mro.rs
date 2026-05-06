#![allow(
    clippy::implicit_hasher,
    clippy::doc_markdown,
    clippy::doc_lazy_continuation
)]

//! TclOO method resolution order — Rust port of
//! `core/analysis/mro.py`.
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
}

/// `is_mixin_path` corresponds to Tcl's ``TRAVERSED_MIXIN``:
/// `true` when the class was reached via a mixin edge.
fn tcloo_dfs(cls: &str, ctx: &mut DfsCtx<'_>, is_mixin_path: bool) {
    if ctx.visiting.contains(cls) {
        // Cycles through mixins are valid in TclOO; just skip.
        return;
    }
    ctx.visiting.insert(cls.to_string());

    // 1. Process class-level mixins (enter mixin path).
    if let Some(mixins) = ctx.mixins_map.get(cls) {
        for mixin in mixins.clone() {
            tcloo_dfs(&mixin, ctx, true);
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
            tcloo_dfs(&parent, ctx, is_mixin_path);
        }
    }

    ctx.visiting.remove(cls);
}

/// Detect a pure superclass cycle starting at `start`.
///
/// Mirrors Python's ``_has_super_cycle`` inside
/// ``tcloo_linearise``.  TclOO's DFS silently skips visited
/// nodes (needed for mixin cycles), but pure superclass cycles
/// should be reported as errors so downstream consumers can
/// surface the problem.
fn has_super_cycle(start: &str, supers_map: &HashMap<String, Vec<String>>) -> bool {
    fn recurse(
        cls: &str,
        supers_map: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if visited.contains(cls) {
            return true;
        }
        visited.insert(cls.to_string());
        if let Some(parents) = supers_map.get(cls) {
            for parent in parents {
                if recurse(parent, supers_map, visited) {
                    return true;
                }
            }
        }
        visited.remove(cls);
        false
    }
    let mut visited = HashSet::new();
    recurse(start, supers_map, &mut visited)
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

    // Pass 1: BUILDING_MIXINS — only collect mixin-path classes.
    let mut visiting = HashSet::new();
    let mut ctx = DfsCtx {
        mixins_map,
        supers_map: superclasses_map,
        result: &mut result,
        visiting: &mut visiting,
        building_mixins: true,
    };
    tcloo_dfs(class_name, &mut ctx, false);

    // Pass 2: collect non-mixin-path classes (shares result list
    // for late-placement dedup).
    let mut visiting = HashSet::new();
    let mut ctx = DfsCtx {
        mixins_map,
        supers_map: superclasses_map,
        result: &mut result,
        visiting: &mut visiting,
        building_mixins: false,
    };
    tcloo_dfs(class_name, &mut ctx, false);

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
