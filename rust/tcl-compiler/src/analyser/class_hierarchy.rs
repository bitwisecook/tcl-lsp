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

//! Class Hierarchy Analysis (`CHA`) for `TclOO`.
//!
//! Builds a complete class hierarchy from the analyser's class
//! index, computes the MRO (two-pass DFS with late-placement
//! deduplication, see [`super::mro::tcloo_linearise`]) for each
//! class, and answers queries about subtype relationships,
//! method providers, and method resolution.
//!
//! Inspired by the CHA techniques used in LLVM and JVM `HotSpot`
//! for devirtualisation and call graph construction.

use std::collections::{HashMap, HashSet};

use super::mro::{build_mro_map, tcloo_linearise};
use super::types::ClassDef;

/// Immutable snapshot of the complete class hierarchy.
///
/// Built once via [`build_class_hierarchy`] and queried via
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

    /// Resolve `TclOO` `next` / `nextto`: the class *after* the current
    /// provider in `class`'s MRO that provides `method`.
    ///
    /// For plain `next`, pass the class currently servicing the method as
    /// `current` and `start_from = None`; the search begins one past
    /// `current`.  For `nextto SomeClass`, pass `start_from =
    /// Some("::SomeClass")`; the search begins *at* that class.  Returns the
    /// qualified name of the next providing class, or `None` when the chain
    /// is exhausted (a `next` past the last provider — a runtime error in
    /// Tcl — surfaces here as "no next").
    #[must_use]
    pub fn next_provider(
        &self,
        class: &str,
        method: &str,
        current: &str,
        start_from: Option<&str>,
    ) -> Option<&str> {
        let mro = self.mro_map.get(class)?;
        let anchor = start_from.unwrap_or(current);
        let anchor_pos = mro.iter().position(|c| c == anchor)?;
        let scan_from = if start_from.is_some() {
            anchor_pos
        } else {
            anchor_pos + 1
        };
        mro.iter().skip(scan_from).find_map(|c| {
            self.classes
                .get(c)
                .filter(|cd| {
                    cd.methods.contains_key(method) || cd.class_methods.contains_key(method)
                })
                .map(|_| c.as_str())
        })
    }

    /// Return all `(class, defining_class)` pairs that
    /// implement `method_name`.  Order matches the iteration
    /// order of `method_providers` (`HashMap` — non-deterministic
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
/// Computes `TclOO` MRO, subclass maps, and method-provider
/// resolution for all
/// classes in the index.
///
/// Names without leading `::` are normalised (when a `::name`
/// match exists in the class map, the leading `::` is added) so
/// downstream lookups don't have to disambiguate.  Cycles in the
/// pure-superclass hierarchy land in `result.errors`; the
/// affected classes get a single-element MRO (themselves only).
/// Resolve a possibly-bare superclass / mixin name written in the body of
/// class `owner` to a qualified name keyed in `classes`.
///
/// A superclass declared bare (`superclass Device`) is resolved the way
/// Tcl would resolve the command: in the **defining class's namespace**,
/// then walking outward to the global namespace, and finally — when the
/// simple name is *globally unique* — to that single class (covering the
/// common `namespace import` idiom without needing per-file import data).
/// An ambiguous simple name (several classes share the tail) stays bare,
/// so no wrong link is ever manufactured.  This fixes cross-file
/// inheritance where a subclass in one file names a base class defined,
/// under a namespace, in another (the `SpiceGenTcl` `superclass Device`
/// shape) — previously left unlinked, silently dropping inherited methods.
fn resolve_super_name(
    name: &str,
    owner_qname: &str,
    classes: &HashMap<String, ClassDef>,
    tail_index: &HashMap<String, Vec<String>>,
) -> String {
    // The MRO builder wants a name back even when nothing resolves (it leaves
    // the edge unlinked rather than dropping the class), so fall back to the
    // written name; the resolution logic itself lives in `resolve_class_name`.
    resolve_class_name(name, owner_qname, |q| classes.contains_key(q), tail_index)
        .unwrap_or_else(|| name.to_string())
}

/// Owner-aware resolution of a written class / superclass / mixin `name` to
/// a qualified class name, mirroring how Tcl resolves a command: an exact
/// hit, then `::name`, then a walk **outward from the owning class's
/// namespace** to the global namespace, and finally — only when the simple
/// (tail) name is *globally unique* — that single class (the `namespace
/// import` idiom).  Returns `None` when nothing resolves or the tail is
/// ambiguous, so callers stay **sound-by-abstention** (never manufacture a
/// wrong cross-file link).
///
/// `is_known` tests membership of a candidate qualified name in the class
/// universe (a `HashMap`/`HashSet` `contains`); `tail_index` maps each
/// simple name to the qualified names sharing it — build it once with
/// [`build_tail_index`] and reuse across many resolutions.
pub fn resolve_class_name<S: std::hash::BuildHasher>(
    name: &str,
    owner_qname: &str,
    is_known: impl Fn(&str) -> bool,
    tail_index: &HashMap<String, Vec<String>, S>,
) -> Option<String> {
    if is_known(name) {
        return Some(name.to_string());
    }
    // Walk the owner's namespace ancestry, then global.
    let owner_ns = owner_qname.rsplit_once("::").map_or("", |(head, _)| head);
    let mut ns = owner_ns.to_string();
    loop {
        let cand = if ns.is_empty() {
            format!("::{}", name.trim_start_matches("::"))
        } else {
            format!("{}::{}", ns.trim_end_matches("::"), name.trim_start_matches("::"))
        };
        if is_known(&cand) {
            return Some(cand);
        }
        if ns.is_empty() {
            break;
        }
        ns = ns.rsplit_once("::").map_or(String::new(), |(head, _)| head.to_string());
    }
    // Globally-unique simple-name match (the `namespace import` case).
    let tail = name.rsplit("::").next().unwrap_or(name);
    match tail_index.get(tail) {
        Some(qs) if qs.len() == 1 => Some(qs[0].clone()),
        _ => None,
    }
}

/// Build the simple-name (tail) → qualified-names index that
/// [`resolve_class_name`] consults for the unique-tail fallback.
pub fn build_tail_index<'a>(
    qnames: impl Iterator<Item = &'a String>,
) -> HashMap<String, Vec<String>> {
    let mut tail_index: HashMap<String, Vec<String>> = HashMap::new();
    for qname in qnames {
        let tail = qname.rsplit("::").next().unwrap_or(qname);
        tail_index.entry(tail.to_string()).or_default().push(qname.clone());
    }
    tail_index
}

/// Build the supers/mixins maps used by the MRO algorithm.
fn build_supers_mixins_maps(
    classes: &HashMap<String, ClassDef>,
) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
    let mut supers_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut mixins_map: HashMap<String, Vec<String>> = HashMap::new();
    // tail (simple name) → qualified class names sharing it.
    let tail_index = build_tail_index(classes.keys());
    let normalise = |owner: &str, names: &[String]| -> Vec<String> {
        names
            .iter()
            .map(|p| resolve_super_name(p, owner, classes, &tail_index))
            .collect()
    };
    for (qname, cd) in classes {
        supers_map.insert(qname.clone(), normalise(qname, &cd.superclasses));
        if !cd.mixins.is_empty() {
            mixins_map.insert(qname.clone(), normalise(qname, &cd.mixins));
        }
    }
    (supers_map, mixins_map)
}

/// Build the method-provider map: `(class, method) -> first ancestor
/// in MRO order that provides the method`.
fn build_method_providers(
    classes: &HashMap<String, ClassDef>,
    mro_map: &HashMap<String, Vec<String>>,
) -> HashMap<(String, String), String> {
    let mut method_providers: HashMap<(String, String), String> = HashMap::new();
    for (qname, cd) in classes {
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
                if let Some(ancestor) = classes.get(ancestor_name)
                    && (ancestor.methods.contains_key(&method_name)
                        || ancestor.class_methods.contains_key(&method_name))
                {
                    method_providers
                        .insert((qname.clone(), method_name.clone()), ancestor_name.clone());
                    break;
                }
            }
        }
    }
    method_providers
}

/// Build the [`ClassHierarchy`] from a flat class-def index.  Runs
/// MRO linearisation, computes direct + transitive subclass sets, and
/// resolves method-provider chains.
#[must_use]
pub fn build_class_hierarchy<S: std::hash::BuildHasher>(
    classes: HashMap<String, ClassDef, S>,
) -> ClassHierarchy {
    // Normalise to the default-hasher map used by `ClassHierarchy::classes`
    // so the rest of the routine (and the public field) stay hasher-agnostic.
    let classes: HashMap<String, ClassDef> = classes.into_iter().collect();
    let (supers_map, mixins_map) = build_supers_mixins_maps(&classes);

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

    let method_providers = build_method_providers(&classes, &mro_map);

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

    #[test]
    fn bare_superclass_links_via_namespace_ancestry() {
        // A subclass in `::Ns::Sub` names its base bare (`Base`); the base
        // lives at `::Ns::Base`. Ancestry resolution links them so the
        // inherited method resolves (previously left unlinked).
        let classes = map(vec![
            cls("::Ns::Base", &[], &[], &["inherited"]),
            cls("::Ns::Sub", &["Base"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.mro_map["::Ns::Sub"], vec!["::Ns::Sub", "::Ns::Base"]);
        assert_eq!(h.method_target("::Ns::Sub", "inherited"), Some("::Ns::Base"));
    }

    #[test]
    fn bare_superclass_links_via_unique_tail_across_namespaces() {
        // The SpiceGenTcl shape: a global class (`::Core`, from an
        // `namespace import`) names a base defined under a namespace
        // (`::SpiceGenTcl::Device`). The simple name is globally unique, so
        // the tail match links them and the inherited method resolves.
        let classes = map(vec![
            cls("::SpiceGenTcl::Device", &[], &[], &["genSPICEString"]),
            cls("::Core", &["Device"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.method_target("::Core", "genSPICEString"), Some("::SpiceGenTcl::Device"));
    }

    #[test]
    fn next_provider_walks_and_nextto_restarts() {
        // C -> B -> A, all define `m`.
        let classes = map(vec![
            cls("::A", &[], &[], &["m"]),
            cls("::B", &["::A"], &[], &["m"]),
            cls("::C", &["::B"], &[], &["m"]),
        ]);
        let h = build_class_hierarchy(classes);
        // `next` from C's m → B, then A, then exhausted.
        assert_eq!(h.next_provider("::C", "m", "::C", None), Some("::B"));
        assert_eq!(h.next_provider("::C", "m", "::B", None), Some("::A"));
        assert_eq!(h.next_provider("::C", "m", "::A", None), None);
        // `nextto ::A` jumps straight to A.
        assert_eq!(h.next_provider("::C", "m", "::C", Some("::A")), Some("::A"));
    }

    #[test]
    fn next_provider_skips_classes_without_the_method() {
        // C -> B -> A; only C and A define `m`. `next` from C skips B → A.
        let classes = map(vec![
            cls("::A", &[], &[], &["m"]),
            cls("::B", &["::A"], &[], &[]),
            cls("::C", &["::B"], &[], &["m"]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.next_provider("::C", "m", "::C", None), Some("::A"));
    }

    #[test]
    fn ambiguous_bare_superclass_stays_unlinked() {
        // Two classes share the tail `Base`; a bare `superclass Base` from an
        // unrelated namespace must NOT be linked to either (no wrong guess).
        let classes = map(vec![
            cls("::A::Base", &[], &[], &["m"]),
            cls("::B::Base", &[], &[], &["m"]),
            cls("::C::Sub", &["Base"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        // Sub's MRO contains only itself + the unresolved bare `Base` leaf;
        // the method does not resolve to either candidate.
        assert_eq!(h.method_target("::C::Sub", "m"), None);
    }
}
