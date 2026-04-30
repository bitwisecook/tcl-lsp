#![allow(clippy::implicit_hasher, clippy::too_many_lines, clippy::doc_markdown)]

//! Class Hierarchy Analysis (CHA) for TclOO — Rust port of
//! `core/analysis/class_hierarchy.py`.
//!
//! Builds a complete class hierarchy from the analyser's class
//! index, computes the MRO (two-pass DFS with late-placement
//! deduplication, see [`super::mro::tcloo_linearise`]) for each
//! class, and answers queries about subtype relationships,
//! method providers, and method resolution.
//!
//! Inspired by the CHA techniques used in LLVM and JVM HotSpot
//! for devirtualisation and call graph construction.

use std::collections::{HashMap, HashSet};

use super::mro::{build_mro_map, tcloo_linearise};
use super::types::ClassDef;

/// Immutable snapshot of the complete class hierarchy.
///
/// Mirrors `ClassHierarchy` in
/// `core/analysis/class_hierarchy.py`.  Built once via
/// [`build_class_hierarchy`] and queried via
/// [`Self::is_subtype`] / [`Self::method_target`] /
/// [`Self::all_implementations`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassHierarchy {
    /// All known classes, keyed by qualified name.
    pub classes: HashMap<String, ClassDef>,
    /// Class → linearised MRO (including self).
    pub mro_map: HashMap<String, Vec<String>>,
    /// Class → direct subclasses.
    pub subclasses: HashMap<String, HashSet<String>>,
    /// Class → all descendants (transitive closure).
    pub transitive_subtypes: HashMap<String, HashSet<String>>,
    /// `(class, method_name)` → qualified name of the class
    /// that provides the method in the MRO chain.  Used for
    /// ``next`` / ``nextto`` resolution and devirtualisation.
    pub method_providers: HashMap<(String, String), String>,
    /// Hierarchy errors (cycles, inconsistent linearisation).
    pub errors: Vec<String>,
}

impl ClassHierarchy {
    /// Return `true` when `child` is a subtype of `parent`
    /// (transitively, via the linearised MRO).
    #[must_use]
    pub fn is_subtype(&self, child: &str, parent: &str) -> bool {
        self.mro_map
            .get(child)
            .is_some_and(|mro| mro.iter().any(|c| c == parent))
    }

    /// Resolve which class provides `method_name` for
    /// `class_name`, walking the MRO chain.  Returns the
    /// qualified name of the providing class, or `None` if no
    /// class on the chain implements the method.
    #[must_use]
    pub fn method_target(&self, class_name: &str, method_name: &str) -> Option<&str> {
        self.method_providers
            .get(&(class_name.to_string(), method_name.to_string()))
            .map(String::as_str)
    }

    /// Return all `(class, defining_class)` pairs that
    /// implement `method_name`.  Order matches the iteration
    /// order of `method_providers` (HashMap — non-deterministic
    /// across runs; sort externally if determinism matters).
    #[must_use]
    pub fn all_implementations(&self, method_name: &str) -> Vec<(String, String)> {
        self.method_providers
            .iter()
            .filter(|((_cls, meth), _provider)| meth == method_name)
            .map(|((cls, _meth), provider)| (cls.clone(), provider.clone()))
            .collect()
    }
}

/// Build a [`ClassHierarchy`] from a dict of class definitions.
///
/// Mirrors `build_class_hierarchy` in
/// `core/analysis/class_hierarchy.py`.  Computes TclOO MRO,
/// subclass maps, and method-provider resolution for all
/// classes in the index.
///
/// Names without leading `::` are normalised (when a `::name`
/// match exists in the class map, the leading `::` is added) so
/// downstream lookups don't have to disambiguate.  Cycles in the
/// pure-superclass hierarchy land in `result.errors`; the
/// affected classes get a single-element MRO (themselves only).
#[must_use]
pub fn build_class_hierarchy(classes: HashMap<String, ClassDef>) -> ClassHierarchy {
    // Build separate superclasses and mixins maps for the
    // TclOO DFS algorithm.
    let mut supers_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut mixins_map: HashMap<String, Vec<String>> = HashMap::new();

    let normalise = |names: &[String], classes: &HashMap<String, ClassDef>| -> Vec<String> {
        names
            .iter()
            .map(|p| {
                if p.starts_with("::") {
                    p.clone()
                } else {
                    let candidate = format!("::{p}");
                    if classes.contains_key(&candidate) {
                        candidate
                    } else {
                        p.clone()
                    }
                }
            })
            .collect()
    };

    for (qname, cd) in &classes {
        supers_map.insert(qname.clone(), normalise(&cd.superclasses, &classes));
        if !cd.mixins.is_empty() {
            mixins_map.insert(qname.clone(), normalise(&cd.mixins, &classes));
        }
    }

    // Compute MRO for every class.  ``build_mro_map`` walks
    // ``supers_map.keys()`` only; classes that appear as parents
    // but aren't in the index get a single-element MRO when
    // queried via ``tcloo_linearise`` directly — for our
    // ``classes``-only walk that's fine.
    let (mro_map_raw, errors_pass1) = build_mro_map(&supers_map, &mixins_map);

    // Backfill: classes whose MRO landed in errors get a
    // single-element chain (themselves only) so downstream
    // lookups don't return ``None``.
    let mut mro_map: HashMap<String, Vec<String>> = mro_map_raw;
    let mut errors = errors_pass1;
    for qname in classes.keys() {
        if !mro_map.contains_key(qname) {
            mro_map.insert(qname.clone(), vec![qname.clone()]);
        }
    }
    // Also include single-pass-fail classes: re-run linearise
    // for any class missing a successful MRO entry above; this
    // catches the case where ``build_mro_map`` short-circuited
    // the iteration on a cycle and never visited some classes.
    for qname in classes.keys() {
        if mro_map[qname] == vec![qname.clone()] && !classes[qname].superclasses.is_empty() {
            // Try a fresh linearise; if it still errors, leave
            // the single-element MRO and record the error.
            match tcloo_linearise(qname, &supers_map, &mixins_map) {
                Ok(mro) => {
                    mro_map.insert(qname.clone(), mro);
                }
                Err(e) => {
                    if !errors.iter().any(|s| s == &e.message) {
                        errors.push(e.message);
                    }
                }
            }
        }
    }

    // Build direct-subclass map.  Initialise with empty sets
    // for every class so callers see a non-None entry even when
    // a class has no subclasses.
    let mut direct_subs: HashMap<String, HashSet<String>> = HashMap::new();
    for qname in classes.keys() {
        direct_subs.insert(qname.clone(), HashSet::new());
    }
    for qname in classes.keys() {
        if let Some(parents) = supers_map.get(qname) {
            for parent in parents {
                if let Some(set) = direct_subs.get_mut(parent) {
                    set.insert(qname.clone());
                }
            }
        }
    }

    // Build transitive-subtype closure via BFS over direct subs.
    let mut transitive: HashMap<String, HashSet<String>> = HashMap::new();
    for qname in classes.keys() {
        let mut visited: HashSet<String> = HashSet::new();
        let mut stack: Vec<String> = direct_subs
            .get(qname)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect();
        while let Some(child) = stack.pop() {
            if visited.contains(&child) {
                continue;
            }
            visited.insert(child.clone());
            if let Some(grandchildren) = direct_subs.get(&child) {
                for g in grandchildren {
                    stack.push(g.clone());
                }
            }
        }
        transitive.insert(qname.clone(), visited);
    }

    // Build method-provider map.  For each class, collect every
    // method name reachable via the MRO chain, then walk the MRO
    // again to find the first provider.
    let mut method_providers: HashMap<(String, String), String> = HashMap::new();
    for (qname, cd) in &classes {
        let mro = mro_map
            .get(qname)
            .cloned()
            .unwrap_or_else(|| vec![qname.clone()]);
        let mut all_methods: HashSet<String> = HashSet::new();
        for n in cd.methods.keys() {
            all_methods.insert(n.clone());
        }
        for n in cd.class_methods.keys() {
            all_methods.insert(n.clone());
        }
        for ancestor_name in &mro {
            if let Some(ancestor) = classes.get(ancestor_name) {
                for n in ancestor.methods.keys() {
                    all_methods.insert(n.clone());
                }
                for n in ancestor.class_methods.keys() {
                    all_methods.insert(n.clone());
                }
            }
        }
        for method_name in all_methods {
            for ancestor_name in &mro {
                if let Some(ancestor) = classes.get(ancestor_name) {
                    if ancestor.methods.contains_key(&method_name)
                        || ancestor.class_methods.contains_key(&method_name)
                    {
                        method_providers
                            .insert((qname.clone(), method_name.clone()), ancestor_name.clone());
                        break;
                    }
                }
            }
        }
    }

    ClassHierarchy {
        classes,
        mro_map,
        subclasses: direct_subs,
        transitive_subtypes: transitive,
        method_providers,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::types::MethodDef;
    use tcl_lexer::Span;

    fn span() -> Span {
        Span::new(0, 0)
    }

    fn cls(qname: &str, supers: &[&str], mixins: &[&str], methods: &[&str]) -> ClassDef {
        let mut cd = ClassDef {
            name: qname.rsplit("::").next().unwrap_or(qname).to_string(),
            qualified_name: qname.to_string(),
            name_span: span(),
            body_span: span(),
            superclasses: supers.iter().map(|s| (*s).to_string()).collect(),
            mixins: mixins.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        };
        for m in methods {
            cd.methods.insert(
                (*m).to_string(),
                MethodDef {
                    name: (*m).to_string(),
                    params: Vec::new(),
                    name_span: span(),
                    body_span: span(),
                    kind: "method".to_string(),
                    visibility: "public".to_string(),
                    doc: String::new(),
                },
            );
        }
        cd
    }

    fn map(classes: Vec<ClassDef>) -> HashMap<String, ClassDef> {
        classes
            .into_iter()
            .map(|c| (c.qualified_name.clone(), c))
            .collect()
    }

    #[test]
    fn empty_input_yields_empty_hierarchy() {
        let h = build_class_hierarchy(HashMap::new());
        assert!(h.classes.is_empty());
        assert!(h.mro_map.is_empty());
        assert!(h.subclasses.is_empty());
        assert!(h.errors.is_empty());
    }

    #[test]
    fn single_class_self_mro() {
        let classes = map(vec![cls("::A", &[], &[], &["m"])]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.mro_map["::A"], vec!["::A"]);
        assert_eq!(h.method_target("::A", "m"), Some("::A"));
    }

    #[test]
    fn single_inheritance_method_resolution() {
        // B inherits from A; A defines ``m``; B's lookup of
        // ``m`` resolves to A.
        let classes = map(vec![
            cls("::A", &[], &[], &["m"]),
            cls("::B", &["::A"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.mro_map["::B"], vec!["::B", "::A"]);
        assert_eq!(h.method_target("::B", "m"), Some("::A"));
        assert!(h.is_subtype("::B", "::A"));
        assert!(!h.is_subtype("::A", "::B"));
    }

    #[test]
    fn diamond_method_resolution_with_late_placement() {
        // D → B, C → A.  TclOO late-placement puts A after C.
        // B defines ``m`` so D's ``m`` resolves to B.
        let classes = map(vec![
            cls("::A", &[], &[], &[]),
            cls("::B", &["::A"], &[], &["m"]),
            cls("::C", &["::A"], &[], &[]),
            cls("::D", &["::B", "::C"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.mro_map["::D"], vec!["::D", "::B", "::C", "::A"]);
        assert_eq!(h.method_target("::D", "m"), Some("::B"));
    }

    #[test]
    fn mixin_provider_takes_precedence() {
        // B mixes M; M defines ``m``; B's ``m`` resolves to M.
        let classes = map(vec![
            cls("::A", &[], &[], &[]),
            cls("::M", &[], &[], &["m"]),
            cls("::B", &["::A"], &["::M"], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.mro_map["::B"], vec!["::M", "::B", "::A"]);
        assert_eq!(h.method_target("::B", "m"), Some("::M"));
    }

    #[test]
    fn direct_subclasses_recorded() {
        let classes = map(vec![
            cls("::A", &[], &[], &[]),
            cls("::B", &["::A"], &[], &[]),
            cls("::C", &["::A"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        let a_subs = &h.subclasses["::A"];
        assert!(a_subs.contains("::B"));
        assert!(a_subs.contains("::C"));
        assert_eq!(a_subs.len(), 2);
        // Leaves have empty subclass sets, never None.
        assert!(h.subclasses["::B"].is_empty());
    }

    #[test]
    fn transitive_subtypes_via_bfs() {
        let classes = map(vec![
            cls("::A", &[], &[], &[]),
            cls("::B", &["::A"], &[], &[]),
            cls("::C", &["::B"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        let a_descendants = &h.transitive_subtypes["::A"];
        assert!(a_descendants.contains("::B"));
        assert!(a_descendants.contains("::C"));
        assert_eq!(a_descendants.len(), 2);
    }

    #[test]
    fn cycle_collected_as_error_not_panic() {
        let classes = map(vec![
            cls("::A", &["::B"], &[], &[]),
            cls("::B", &["::A"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        assert!(!h.errors.is_empty());
        // Cycle classes get single-element MRO fallback.
        assert_eq!(h.mro_map["::A"], vec!["::A"]);
    }

    #[test]
    fn unqualified_parent_normalised_when_match_exists() {
        // ``::B`` lists ``A`` (no ``::``) as a superclass; the
        // normalisation step adds the leading ``::`` because
        // ``::A`` is in the class map.
        let classes = map(vec![
            cls("::A", &[], &[], &["m"]),
            cls("::B", &["A"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.mro_map["::B"], vec!["::B", "::A"]);
        assert_eq!(h.method_target("::B", "m"), Some("::A"));
    }

    #[test]
    fn method_target_returns_none_for_unknown_method() {
        let classes = map(vec![cls("::A", &[], &[], &[])]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.method_target("::A", "nope"), None);
    }

    #[test]
    fn all_implementations_lists_all_classes_with_method() {
        let classes = map(vec![
            cls("::A", &[], &[], &["m"]),
            cls("::B", &["::A"], &[], &["m"]),
        ]);
        let h = build_class_hierarchy(classes);
        let mut impls = h.all_implementations("m");
        impls.sort();
        assert_eq!(impls.len(), 2);
        // ``::A`` provides its own m.
        assert!(impls.contains(&("::A".to_string(), "::A".to_string())));
        // ``::B`` overrides; B's m provider is ::B.
        assert!(impls.contains(&("::B".to_string(), "::B".to_string())));
    }
}
