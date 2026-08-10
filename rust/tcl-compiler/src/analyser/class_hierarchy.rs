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
use super::types::{ClassDef, MethodDef};

/// The name a `constructor` body carries wherever a member is identified by
/// name — `ClassDef::constructors`' own `MethodDef::name`, the tail of the
/// `Method` scope `walk_method_body` opens for it, and the label C Tcl's
/// `info object call` / `self method` report for that frame (tclsh 9.0.4).
pub const CONSTRUCTOR_MEMBER: &str = "<constructor>";

/// [`CONSTRUCTOR_MEMBER`]'s twin for a `destructor` body.
pub const DESTRUCTOR_MEMBER: &str = "<destructor>";

/// The member of `class_def` named `member`, treating the two nameless
/// slots by their synthetic labels ([`CONSTRUCTOR_MEMBER`] /
/// [`DESTRUCTOR_MEMBER`]) and an ordinary name as an instance method,
/// then a class-side one.
///
/// The lookup every `next`-resolution consumer needs once
/// [`ClassHierarchy::member_next_provider`] has named the providing class:
/// keeping it here means a constructor's `MethodDef` is fetched the same
/// way in go-to-definition, find-references, and the arity check (issue
/// #923 idx 37). The *effective* constructor is the last declared one, the
/// same "most recent declaration wins" rule
/// [`ClassHierarchy::constructor_provider`] applies.
#[must_use]
pub fn class_member_def<'a>(class_def: &'a ClassDef, member: &str) -> Option<&'a MethodDef> {
    match member {
        CONSTRUCTOR_MEMBER => class_def.constructors.last(),
        DESTRUCTOR_MEMBER => class_def.destructor.as_ref(),
        _ => class_def
            .methods
            .get(member)
            .or_else(|| class_def.class_methods.get(member)),
    }
}

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

    /// Resolve which class provides an *explicit* constructor for
    /// `class_name`, walking the MRO chain — mirrors [`Self::method_target`],
    /// but for `TclOO`'s single per-class constructor slot
    /// (`ClassDef::constructors`) rather than a named method.
    ///
    /// Returns `None` when no class on the chain declares an *effective*
    /// one: `TclOO` falls back to a permissive default constructor
    /// (inherited from `oo::object`) that accepts and ignores any number
    /// of arguments both when no `constructor` was ever written
    /// (confirmed against tclsh 9.0.4: `oo::class create Foo {}` then `Foo
    /// new 1 2 3` succeeds) **and** when the effective (most recent)
    /// `constructor` declaration has a literally empty body — `TclOO`
    /// treats `constructor {a b} {}` as if no constructor were declared at
    /// all (confirmed against tclsh 9.0.4: `info class constructor Foo`
    /// returns empty, and `Foo new`/`Foo new 1 2 3` both succeed for that
    /// class; a body with any content at all — even a single space or a
    /// comment — *is* a real, arity-enforcing constructor). `source` is the
    /// full document text `body_span` indexes into (see
    /// [`is_empty_method_body`]) — this hierarchy holds no source text of
    /// its own. Callers must treat `None` as "any argument count is
    /// valid", not "the constructor takes no arguments".
    #[must_use]
    pub fn constructor_provider(&self, class_name: &str, source: &str) -> Option<&str> {
        let mro = self.mro_map.get(class_name)?;
        mro.iter().find_map(|ancestor| {
            self.classes
                .get(ancestor)
                .filter(|cd| {
                    cd.constructors
                        .last()
                        .is_some_and(|c| !is_empty_method_body(source, c.body_span))
                })
                .map(|_| ancestor.as_str())
        })
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

    /// Resolve `TclOO` `next` / `nextto` from *within* `class`'s own
    /// constructor: the class after `class` in its MRO that provides an
    /// *effective* constructor (mirrors [`Self::next_provider`], but for the
    /// unnamed constructor slot rather than a named method — "effective"
    /// carries the same empty-body caveat as [`Self::constructor_provider`],
    /// whose doc explains it).
    ///
    /// For plain `next`, pass `start_from = None`; the search begins one
    /// past `class` itself. For `nextto SomeClass`, pass `start_from =
    /// Some("::SomeClass")`; the search begins *at* that class. `source` is
    /// the document text `body_span`s index into. Returns `None` when the
    /// chain is exhausted or `class` is unknown.
    #[must_use]
    pub fn constructor_next_provider(
        &self,
        class: &str,
        start_from: Option<&str>,
        source: &str,
    ) -> Option<&str> {
        let mro = self.mro_map.get(class)?;
        let anchor = start_from.unwrap_or(class);
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
                    cd.constructors
                        .last()
                        .is_some_and(|ctor| !is_empty_method_body(source, ctor.body_span))
                })
                .map(|_| c.as_str())
        })
    }

    /// Resolve `TclOO` `next` / `nextto` from *within* `class`'s own
    /// destructor: the class after `class` in its MRO that declares a
    /// destructor.
    ///
    /// Unlike [`Self::constructor_next_provider`], a destructor's
    /// "effective" test is plain existence (`ClassDef::destructor.is_some()`)
    /// — `TclOO`'s empty-body-elides-the-constructor quirk is verified
    /// (against tclsh 9.0.4) for constructors specifically; no equivalent
    /// claim has been checked for destructors, so an explicitly empty
    /// destructor body is conservatively still treated as a real override
    /// here rather than assumed to share the constructor's special case.
    ///
    /// Same `start_from` convention as [`Self::constructor_next_provider`].
    /// Returns `None` when the chain is exhausted or `class` is unknown.
    #[must_use]
    pub fn destructor_next_provider(&self, class: &str, start_from: Option<&str>) -> Option<&str> {
        let mro = self.mro_map.get(class)?;
        let anchor = start_from.unwrap_or(class);
        let anchor_pos = mro.iter().position(|c| c == anchor)?;
        let scan_from = if start_from.is_some() {
            anchor_pos
        } else {
            anchor_pos + 1
        };
        mro.iter().skip(scan_from).find_map(|c| {
            self.classes
                .get(c)
                .filter(|cd| cd.destructor.is_some())
                .map(|_| c.as_str())
        })
    }

    /// Resolve `TclOO` `next` / `nextto` from inside **any** member body,
    /// routing the two nameless slots to their own providers.
    ///
    /// `member` is the enclosing member as the analyser names it — a
    /// method's own name, or the synthetic [`CONSTRUCTOR_MEMBER`] /
    /// [`DESTRUCTOR_MEMBER`] label a nameless body carries (the same
    /// `<constructor>` / `<destructor>` spelling C Tcl's own `info object
    /// call` reports, and the one `walk_method_body` builds a constructor
    /// scope's name from).
    ///
    /// The single entry point for `next` resolution, so go-to-definition,
    /// find-references, and the `next`-arity check cannot disagree about
    /// which slot a `next` chains through. Before it existed all three went
    /// straight to [`Self::next_provider`], whose per-ancestor filter only
    /// consults `methods`/`class_methods`, so a `next` inside a
    /// `constructor` — the ordinary way a subclass forwards to its
    /// superclass's constructor — resolved to nothing at all and its arity
    /// went unchecked, while the identical `next` inside a plain `method`
    /// worked (issue #923 idx 37; a real `next` there does dispatch, pinned
    /// against tclsh 9.0.4).
    ///
    /// `source` is the document text `body_span`s index into, needed only
    /// for the constructor slot's empty-body rule — see
    /// [`Self::constructor_provider`].
    #[must_use]
    pub fn member_next_provider(
        &self,
        class: &str,
        member: &str,
        current: &str,
        start_from: Option<&str>,
        source: &str,
    ) -> Option<&str> {
        match member {
            CONSTRUCTOR_MEMBER => self.constructor_next_provider(class, start_from, source),
            DESTRUCTOR_MEMBER => self.destructor_next_provider(class, start_from),
            _ => self.next_provider(class, member, current, start_from),
        }
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

    /// Every method name callable on `class_name` — instance and class
    /// methods declared anywhere on its linearised MRO — sorted and
    /// deduplicated.  Feeds the W308 "did you mean…?" suggestion list;
    /// empty for an unknown class.
    #[must_use]
    pub fn known_methods(&self, class_name: &str) -> Vec<String> {
        let mut out: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        if let Some(mro) = self.mro_map.get(class_name) {
            for ancestor in mro {
                if let Some(cd) = self.classes.get(ancestor) {
                    out.extend(cd.methods.keys().cloned());
                    out.extend(cd.class_methods.keys().cloned());
                    // A configurable class — the `oo::configurable` metaclass,
                    // or any class carrying `property` declarations — answers
                    // `configure`/`cget` for its properties, so those method
                    // words are known even though no `method` body defines
                    // them (the accessors are generated by the property).
                    if cd.metaclass == "oo::configurable" || !cd.properties.is_empty() {
                        out.insert("configure".to_owned());
                        out.insert("cget".to_owned());
                    }
                }
            }
        }
        out.into_iter().collect()
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
///
/// Whether a `TclOO` method/constructor body span is *effectively* empty —
/// `{}` / `[]` / `""` with **nothing at all** between the delimiters.
/// `TclOO` treats exactly this shape as "no constructor was written" (see
/// [`ClassHierarchy::constructor_provider`]'s doc comment); any body
/// content at all, even a single space or a comment, keeps it real
/// (confirmed against tclsh 9.0.4).
///
/// `body_span` follows this project's general delimited-word convention
/// (`docs/kcs/kcs-issue-highlight-drops-closing-delimiter.md`): for
/// non-empty content the span covers the opening delimiter through the
/// *last inner character*, excluding the closer (e.g. `{ }`'s span slices
/// to `"{ "`, two bytes); for genuinely empty content the span instead
/// extends one further to sit *on* the closer (`{}`'s span slices to the
/// full `"{}"`, also two bytes, but ending on `}` rather than before it).
/// So an empty body is exactly the two-byte slice `"{}"` / `"[]"` /
/// `"\"\""` — no non-empty content is ever pair-shaped like that (a
/// single-inner-character body such as `{x}` slices to `"{x"`, not `"{}"`)
/// — a cheap, unambiguous string-equality test rather than re-parsing.
pub(in crate::analyser) fn is_empty_method_body(source: &str, body_span: tcl_lexer::Span) -> bool {
    let start = body_span.start() as usize;
    let end = body_span.end() as usize;
    matches!(source.get(start..end), Some("{}" | "[]" | "\"\""))
}

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
    // C Tcl resolves a bare `superclass`/`mixin` name relative to the enclosing
    // (`oo::define` call-site) namespace in exactly two scopes — the current
    // namespace, then global — with NO walk through intermediate ancestors.
    // Verified against the VM's `cmd_oo::resolve_class` and C's
    // `GetClassInOuterContext` (`tclOODefineCmds.c`).  The former ancestor walk
    // manufactured a wrong inheritance edge (e.g. a bare `superclass Base` in
    // `::a::b::Sub` linked to `::a::Base` even though real Tcl errors — `Base`
    // is reachable from neither `::a::b` nor global).  Uses the shared command
    // -resolution candidate order so class-name resolution can never diverge
    // from it (a class name *is* a command name).
    let owner_ns = owner_qname.rsplit_once("::").map_or("", |(head, _)| head);
    for cand in crate::naming::bareword_resolution_candidates(owner_ns, name) {
        if is_known(&cand) {
            return Some(cand);
        }
    }
    // Globally-unique simple-name match (the `namespace import` case).
    //
    // Tradeoff (deliberate, retained): this can manufacture a *wrong* edge
    // when `name` refers to a base that isn't in the class universe (e.g. an
    // external/library class the index never saw) yet exactly one *unrelated*
    // indexed class happens to share the tail — the fallback then links to
    // that unrelated class.  We keep it because (a) it is the only thing that
    // resolves the common `namespace import` idiom where a subclass names a
    // namespaced base bare (the `SpiceGenTcl` `superclass Device` shape), and
    // (b) it stays sound-by-abstention on the far more common failure mode: a
    // tail shared by two or more indexed classes never links (returns `None`).
    // The precondition — a globally *unique* tail that is nonetheless the
    // wrong class — is rare in practice.  Revisit if false links surface.
    let tail = name.rsplit("::").next().unwrap_or(name);
    match tail_index.get(tail) {
        Some(qs) if qs.len() == 1 => Some(qs[0].clone()),
        _ => None,
    }
}

/// Resolve a class name *as written at a call site* (no owning-class
/// context — `resolve_class_name` is the owner-aware variant for
/// superclass / mixin edges) to a key of `classes`: an exact hit, the
/// global-qualified form of its canonical spelling (colon-run rule, #934),
/// or — last — the unique class sharing its tail (the `namespace import`
/// idiom).  `None` when unresolved or the tail is ambiguous, so callers
/// stay sound-by-abstention.  The single implementation behind the
/// analyser's method-validation keying and the LSP's definer-head
/// resolution (M4.2 dedup — three near-copies once drifted here).
pub fn resolve_written_class_name<V, S: std::hash::BuildHasher>(
    name: &str,
    classes: &HashMap<String, V, S>,
) -> Option<String> {
    if classes.contains_key(name) {
        return Some(name.to_owned());
    }
    let canonical = tcl_syntax::naming::canonical_written_command(name);
    let qualified = if canonical.starts_with("::") {
        canonical.clone()
    } else {
        format!("::{canonical}")
    };
    if classes.contains_key(&qualified) {
        return Some(qualified);
    }
    let tail = name.rsplit("::").next().unwrap_or(name);
    let mut matches = classes
        .keys()
        .filter(|k| tcl_syntax::naming::key_tail(k) == tail);
    let first = matches.next()?;
    matches.next().is_none().then_some(first.clone())
}

/// Build the simple-name (tail) → qualified-names index that
/// [`resolve_class_name`] consults for the unique-tail fallback.
pub fn build_tail_index<'a>(
    qnames: impl Iterator<Item = &'a String>,
) -> HashMap<String, Vec<String>> {
    let mut tail_index: HashMap<String, Vec<String>> = HashMap::new();
    for qname in qnames {
        // `qname` is a constructed key — construction-inverse tail (#934).
        let tail = crate::naming::key_tail(qname);
        tail_index
            .entry(tail.to_string())
            .or_default()
            .push(qname.clone());
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

/// Whether the combined, already-normalised superclass/mixin graph reachable
/// from `start` revisits a node on its active path. The shared MRO primitive
/// deliberately cuts off mixin revisits, but an analysis hierarchy records
/// every relation cycle as an error so downstream proofs never mistake that
/// truncated order for a complete method chain.
fn has_relation_cycle(
    start: &str,
    supers_map: &HashMap<String, Vec<String>>,
    mixins_map: &HashMap<String, Vec<String>>,
) -> bool {
    fn visit(
        class_name: &str,
        supers_map: &HashMap<String, Vec<String>>,
        mixins_map: &HashMap<String, Vec<String>>,
        active: &mut HashSet<String>,
        complete: &mut HashSet<String>,
    ) -> bool {
        if active.contains(class_name) {
            return true;
        }
        if complete.contains(class_name) {
            return false;
        }
        active.insert(class_name.to_owned());
        let relations = supers_map
            .get(class_name)
            .into_iter()
            .flatten()
            .chain(mixins_map.get(class_name).into_iter().flatten());
        for relation in relations {
            if visit(relation, supers_map, mixins_map, active, complete) {
                return true;
            }
        }
        active.remove(class_name);
        complete.insert(class_name.to_owned());
        false
    }

    visit(
        start,
        supers_map,
        mixins_map,
        &mut HashSet::new(),
        &mut HashSet::new(),
    )
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
        if has_relation_cycle(qname, &supers_map, &mixins_map) {
            let error = format!("cycle detected in class relation hierarchy for {qname}");
            if !errors.contains(&error) {
                errors.push(error);
            }
        }
    }
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
    //
    // A class is a subtype of both its superclasses **and** its mixins — the
    // MRO (and thus `is_subtype` / `method_target`) already treats a mixin as
    // a supertype, so the subclass map must be built from both edges too, or
    // it disagrees with `is_subtype` (asymmetrically, depending on which file
    // a mixing-in subclass happens to live in).  Union supers + mixins here.
    let mut direct_subs: HashMap<String, HashSet<String>> = HashMap::new();
    for qname in classes.keys() {
        direct_subs.insert(qname.clone(), HashSet::new());
    }
    for qname in classes.keys() {
        let parents = supers_map
            .get(qname)
            .into_iter()
            .flatten()
            .chain(mixins_map.get(qname).into_iter().flatten());
        for parent in parents {
            if let Some(set) = direct_subs.get_mut(parent) {
                set.insert(qname.clone());
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

    // `errors` is accumulated while walking `classes.keys()` (HashMap order), so
    // its `Vec` order is otherwise nondeterministic.  Sort it: the order carries
    // no meaning, and a stable order keeps two logically-equal hierarchies
    // `==` — which is what lets salsa backdate `project_class_index` on a
    // body-only edit instead of needlessly recomputing dependent token queries.
    errors.sort_unstable();

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
            name: crate::naming::key_tail(qname).to_string(),
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
                    params_computed: false,
                    name_span: span(),
                    body_span: span(),
                    kind: "method".to_string(),
                    is_self_method: false,
                    visibility: "public".to_string(),
                    doc: String::new(),
                    forward_target: None,
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
    fn mixin_recorded_as_subclass_edge() {
        // `B` mixes `M`.  `is_subtype(B, M)` is true (the MRO places the
        // mixin as a supertype), so the subclass map must list `B` under
        // `M` too — otherwise the two views disagree.
        let classes = map(vec![
            cls("::M", &[], &[], &[]),
            cls("::A", &[], &[], &[]),
            cls("::B", &["::A"], &["::M"], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        assert!(h.is_subtype("::B", "::M"));
        assert!(
            h.subclasses["::M"].contains("::B"),
            "{:?}",
            h.subclasses["::M"]
        );
        assert!(h.subclasses["::A"].contains("::B"));
        // Transitive closure agrees.
        assert!(h.transitive_subtypes["::M"].contains("::B"));
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
    fn relation_cycle_errors_distinguish_a_mixin_cycle_from_a_diamond() {
        let cyclic = build_class_hierarchy(map(vec![
            cls("::A", &[], &["::B"], &[]),
            cls("::B", &[], &["::A"], &[]),
            cls("::C", &[], &["::A"], &[]),
        ]));
        assert!(!cyclic.errors.is_empty());

        let diamond = build_class_hierarchy(map(vec![
            cls("::Root", &[], &[], &[]),
            cls("::Left", &["::Root"], &[], &[]),
            cls("::Right", &["::Root"], &[], &[]),
            cls("::Leaf", &["::Left", "::Right"], &[], &[]),
        ]));
        assert!(diamond.errors.is_empty());
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

    /// `constructor_provider` mirrors `method_target` but over
    /// `ClassDef::constructors` rather than `methods`.
    ///
    /// Real source text `is_empty_method_body` slices into: byte 0-1 is
    /// `"{}"` (an empty body), byte 2-4 is `"{x"` (`is_empty_method_body`'s
    /// documented non-empty-content slice shape — the span covers the
    /// opener through the last inner byte, excluding the closer).
    const CTOR_SRC: &str = "{}{x}";
    const EMPTY_BODY: Span = Span::new(0, 2);
    const NON_EMPTY_BODY: Span = Span::new(2, 4);

    /// A real (non-empty-bodied) constructor.
    fn cls_with_ctor(qname: &str, supers: &[&str]) -> ClassDef {
        cls_with_ctor_body(qname, supers, NON_EMPTY_BODY)
    }

    fn cls_with_ctor_body(qname: &str, supers: &[&str], body_span: Span) -> ClassDef {
        let mut cd = cls(qname, supers, &[], &[]);
        cd.constructors.push(MethodDef {
            name: "<constructor>".to_string(),
            params: Vec::new(),
            params_computed: false,
            name_span: span(),
            body_span,
            kind: "constructor".to_string(),
            is_self_method: false,
            visibility: "public".to_string(),
            doc: String::new(),
            forward_target: None,
        });
        cd
    }

    #[test]
    fn constructor_provider_none_when_no_class_declares_one() {
        let classes = map(vec![cls("::A", &[], &[], &[])]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.constructor_provider("::A", CTOR_SRC), None);
    }

    #[test]
    fn constructor_provider_own_class() {
        let classes = map(vec![cls_with_ctor("::A", &[])]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.constructor_provider("::A", CTOR_SRC), Some("::A"));
    }

    #[test]
    fn constructor_provider_inherited_from_superclass() {
        // ::B has no constructor of its own; ::A's is inherited.
        let classes = map(vec![
            cls_with_ctor("::A", &[]),
            cls("::B", &["::A"], &[], &[]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.constructor_provider("::B", CTOR_SRC), Some("::A"));
    }

    #[test]
    fn constructor_provider_own_overrides_inherited() {
        let classes = map(vec![
            cls_with_ctor("::A", &[]),
            cls_with_ctor("::B", &["::A"]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.constructor_provider("::B", CTOR_SRC), Some("::B"));
    }

    #[test]
    fn constructor_provider_none_for_empty_body_constructor() {
        // `constructor {a b} {}` (a literally empty body) is `TclOO`'s way
        // of writing "no constructor" — confirmed against tclsh 9.0.4:
        // `info class constructor` returns empty and `new` accepts any
        // argument count.
        let classes = map(vec![cls_with_ctor_body("::A", &[], EMPTY_BODY)]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.constructor_provider("::A", CTOR_SRC), None);
    }

    #[test]
    fn constructor_provider_falls_back_to_superclass_when_own_is_empty_bodied() {
        let classes = map(vec![
            cls_with_ctor("::A", &[]),
            cls_with_ctor_body("::B", &["::A"], EMPTY_BODY),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.constructor_provider("::B", CTOR_SRC), Some("::A"));
    }

    #[test]
    fn is_empty_method_body_matches_only_the_exact_delimiter_pair() {
        assert!(is_empty_method_body(CTOR_SRC, EMPTY_BODY));
        assert!(!is_empty_method_body(CTOR_SRC, NON_EMPTY_BODY));
        assert!(is_empty_method_body("[]", Span::new(0, 2)));
        assert!(is_empty_method_body("\"\"", Span::new(0, 2)));
        // Out-of-range span — must not panic, must not report empty.
        assert!(!is_empty_method_body("{}", Span::new(0, 5)));
    }

    // `constructor_next_provider` / `destructor_next_provider` — issue #992's
    // constructor/destructor next-chain lens support.

    #[test]
    fn constructor_next_provider_none_when_class_unknown() {
        let classes = map(vec![cls_with_ctor("::A", &[])]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.constructor_next_provider("::Nope", None, CTOR_SRC), None);
    }

    #[test]
    fn constructor_next_provider_direct_superclass() {
        // `Sub`'s own `next` (no explicit target) resolves one past `Sub`
        // itself in `Sub`'s own MRO — its direct superclass `Base`.
        let classes = map(vec![
            cls_with_ctor("::Base", &[]),
            cls_with_ctor("::Sub", &["::Base"]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(
            h.constructor_next_provider("::Sub", None, CTOR_SRC),
            Some("::Base")
        );
    }

    #[test]
    fn constructor_next_provider_skips_ancestor_with_no_effective_constructor() {
        // `Mid` declares no constructor of its own (a pure pass-through), so
        // `Sub`'s `next` must skip past it and reach `Base`'s — the same
        // "effective provider" skip `constructor_provider` already performs,
        // just starting one class later.
        let classes = map(vec![
            cls_with_ctor("::Base", &[]),
            cls("::Mid", &["::Base"], &[], &[]),
            cls_with_ctor("::Sub", &["::Mid"]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(
            h.constructor_next_provider("::Sub", None, CTOR_SRC),
            Some("::Base")
        );
    }

    #[test]
    fn constructor_next_provider_skips_ancestor_with_empty_bodied_constructor() {
        // `Mid` declares an explicit but empty-bodied constructor — `TclOO`
        // treats that as "no constructor", so `next` from `Sub` must skip
        // past it too, exactly like `constructor_provider` does.
        let classes = map(vec![
            cls_with_ctor("::Base", &[]),
            cls_with_ctor_body("::Mid", &["::Base"], EMPTY_BODY),
            cls_with_ctor("::Sub", &["::Mid"]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(
            h.constructor_next_provider("::Sub", None, CTOR_SRC),
            Some("::Base")
        );
    }

    #[test]
    fn constructor_next_provider_none_when_chain_exhausted() {
        let classes = map(vec![cls_with_ctor("::Sub", &[])]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.constructor_next_provider("::Sub", None, CTOR_SRC), None);
    }

    #[test]
    fn constructor_next_provider_nextto_explicit_target() {
        // `nextto Grandparent` jumps straight to the named class, skipping
        // `Base` even though `Base` also has an effective constructor.
        let classes = map(vec![
            cls_with_ctor("::Grandparent", &[]),
            cls_with_ctor("::Base", &["::Grandparent"]),
            cls_with_ctor("::Sub", &["::Base"]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(
            h.constructor_next_provider("::Sub", Some("::Grandparent"), CTOR_SRC),
            Some("::Grandparent")
        );
    }

    /// A class with a real (non-empty) destructor.
    fn cls_with_dtor(qname: &str, supers: &[&str]) -> ClassDef {
        let mut cd = cls(qname, supers, &[], &[]);
        cd.destructor = Some(MethodDef {
            name: "<destructor>".to_string(),
            params: Vec::new(),
            params_computed: false,
            name_span: span(),
            body_span: NON_EMPTY_BODY,
            kind: "destructor".to_string(),
            visibility: "public".to_string(),
            doc: String::new(),
            forward_target: None,
            is_self_method: false,
        });
        cd
    }

    #[test]
    fn destructor_next_provider_direct_superclass() {
        let classes = map(vec![
            cls_with_dtor("::Base", &[]),
            cls_with_dtor("::Sub", &["::Base"]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.destructor_next_provider("::Sub", None), Some("::Base"));
    }

    #[test]
    fn destructor_next_provider_skips_ancestor_with_no_destructor() {
        let classes = map(vec![
            cls_with_dtor("::Base", &[]),
            cls("::Mid", &["::Base"], &[], &[]),
            cls_with_dtor("::Sub", &["::Mid"]),
        ]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.destructor_next_provider("::Sub", None), Some("::Base"));
    }

    #[test]
    fn destructor_next_provider_none_when_chain_exhausted() {
        let classes = map(vec![cls_with_dtor("::Sub", &[])]);
        let h = build_class_hierarchy(classes);
        assert_eq!(h.destructor_next_provider("::Sub", None), None);
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
        assert_eq!(
            h.method_target("::Ns::Sub", "inherited"),
            Some("::Ns::Base")
        );
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
        assert_eq!(
            h.method_target("::Core", "genSPICEString"),
            Some("::SpiceGenTcl::Device")
        );
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

    // Class-name resolution follows C Tcl's one-hop rule (current ns, then
    // global), never an ancestor walk.  `resolve_from` mirrors the analyser's
    // real call.
    fn resolve_from(name: &str, owner: &str, keys: &[&str]) -> Option<String> {
        let key_strings: Vec<String> = keys.iter().copied().map(String::from).collect();
        let known: std::collections::HashSet<String> = key_strings.iter().cloned().collect();
        let tail_index = build_tail_index(key_strings.iter());
        resolve_class_name(name, owner, |q| known.contains(q), &tail_index)
    }

    /// FP guard (the ancestor-walk bug): a bare `superclass Base` in
    /// `::a::b::Sub` where `Base` exists only at `::a::Base` (an *ancestor*, not
    /// the current ns or global) and the tail is ambiguous must NOT link — real
    /// Tcl errors there.  Before the fix this wrongly returned `::a::Base`.
    #[test]
    fn superclass_resolution_abstains_on_ancestor_only_base() {
        let got = resolve_from(
            "Base",
            "::a::b::Sub",
            &["::a::Base", "::x::Base", "::a::b::Sub"],
        );
        assert_eq!(
            got, None,
            "must abstain, not climb to an ancestor namespace"
        );
    }

    /// TP: a base in the class's *own* namespace resolves one-hop.
    #[test]
    fn superclass_same_namespace_base_resolves() {
        let got = resolve_from("Base", "::a::Sub", &["::a::Base", "::a::Sub"]);
        assert_eq!(got.as_deref(), Some("::a::Base"));
    }

    /// TP: a global base resolves (current-ns miss falls through to global).
    #[test]
    fn superclass_global_base_resolves() {
        let got = resolve_from("Base", "::a::b::Sub", &["::Base", "::a::b::Sub"]);
        assert_eq!(got.as_deref(), Some("::Base"));
    }

    /// TP (regression guard): the cross-file `namespace import` idiom — a
    /// subclass names a namespaced base bare when the base is unique — still
    /// links via the sound-by-abstention unique-tail fallback.
    #[test]
    fn superclass_cross_file_unique_tail_links() {
        let got = resolve_from(
            "Device",
            "::spice::sub::Sub",
            &["::spice::Device", "::spice::sub::Sub"],
        );
        assert_eq!(got.as_deref(), Some("::spice::Device"));
    }

    /// TN: an absolute `::`-qualified name is taken exactly.
    #[test]
    fn superclass_absolute_name_is_exact() {
        let got = resolve_from("::a::Base", "::x::Sub", &["::a::Base", "::x::Sub"]);
        assert_eq!(got.as_deref(), Some("::a::Base"));
    }
}
