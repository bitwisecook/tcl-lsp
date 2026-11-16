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

//! The class graph across documents: linearisation, method dispatch chains, effective members and override families.

use super::records::{MethodAccess, WorkspaceMethod};
use super::{WorkspaceClass, WorkspaceIndex};
use std::collections::HashSet;
use tcl_compiler::analyser::{MemberRetractionRecord, MemberSide};
use tcl_compiler::ir::MethodKind;
use tcl_lexer::Span;

/// Which receiver side a [`WorkspaceMethod`] is declared on — the same split
/// [`WorkspaceClass::instance_method`] / [`WorkspaceClass::class_method`] make,
/// named once so the member fold and the tombstone lookup agree on it.
#[must_use]
fn method_side(m: &WorkspaceMethod) -> MemberSide {
    if m.kind == MethodKind::ClassMethod.as_str() {
        MemberSide::ClassObject
    } else {
        MemberSide::Instance
    }
}

/// One member of a class as the **workspace** sees it: the declaring record's
/// entry, keyed under the name the class actually dispatches it by once every
/// cross-document retraction and arrival has been applied.
///
/// The raw [`WorkspaceClass::methods`] table is per-record and additive — it
/// records what that record's own body declared and nothing else — so a member
/// moved by a cross-file `oo::define ::C { renamemethod old new }` is still
/// listed as `old` on the defining record and not listed at all on the stub.
/// Resolving one name against the table joins the two halves
/// ([`WorkspaceIndex::dispatch_chain`]); *listing* the table must join them
/// too, or anything that enumerates a class's members (`workspace/symbol`, an
/// outline, a member completion universe) reports the pre-rename name.
///
/// [`WorkspaceIndex::effective_members`] is the one place that fold happens,
/// and the dispatch chain reads it too, so a single rule decides what a class's
/// member set is.
#[derive(Debug, Clone, Copy)]
pub struct EffectiveMember<'a> {
    /// The name the class dispatches this member under.
    pub name: &'a str,
    /// The record whose body, parameters and visibility define the member —
    /// where a `renamemethod`'s *source* was declared, not where the rename is
    /// written.
    pub declaring: &'a WorkspaceClass,
    /// The declaring record's own entry, still keyed under its declared
    /// spelling.  Visibility travels with the body, so this is what a
    /// visibility test must read.
    pub method: &'a WorkspaceMethod,
    /// Document holding the token that spells [`Self::name`] — the declaring
    /// record's document normally, the retracting stub's when the member
    /// arrived through a cross-file `renamemethod`.
    pub name_uri: &'a str,
    /// Byte span of that token, in [`Self::name_uri`]'s source.
    pub name_span: Span,
}

/// Every cross-document member retraction in the workspace, keyed by the
/// `(class qualified name, member name, side)` it removes and valued with the
/// record that wrote it plus the retraction itself.  See
/// [`WorkspaceIndex::retraction_index`].
pub(super) type RetractionIndex<'a> = std::collections::HashMap<
    (&'a str, &'a str, MemberSide),
    (&'a WorkspaceClass, &'a MemberRetractionRecord),
>;

/// Apply the workspace's retraction / arrival fold to one member `declaring`
/// declares, yielding the name the class dispatches it under — or `None` when
/// a cross-document `deletemethod` removed it outright.
///
/// The one place that decision is made; [`WorkspaceIndex::effective_members`]
/// (and through it [`WorkspaceIndex::dispatch_chain`]) and the
/// `workspace/symbol` member walk all route through it, so resolving a name
/// and listing the member set cannot disagree.
///
/// tclsh-proof (8.6.14), for the two arms:
///
/// ```tcl
/// # a.tcl: oo::class create ::C { method old {} { return OLDBODY } }
/// # b.tcl: oo::define ::C { renamemethod old new }
/// info class methods ::C     ;# -> new         (arrival re-keys the member)
/// [::C new] old              ;# -> unknown method "old"
/// # with `deletemethod old` instead:
/// info class methods ::C     ;# -> (empty)     (the member is gone)
/// ```
pub(super) fn effective_member<'a>(
    retractions: &RetractionIndex<'a>,
    declaring: &'a WorkspaceClass,
    method: &'a WorkspaceMethod,
) -> Option<EffectiveMember<'a>> {
    let side = method_side(method);
    // A `deletemethod` / `renamemethod` in *another* document's `oo::define`
    // stub really removes the member. The union is taken
    // per class — a subclass cannot retract an inherited member (real Tcl:
    // `method … does not exist`) — and the tombstone carries the side it
    // removed from, so a `self deletemethod m` never touches the
    // instance-side member.
    let Some((stub, retraction)) = retractions.get(&(
        declaring.qualified_name.as_str(),
        method.name.as_str(),
        side,
    )) else {
        return Some(EffectiveMember {
            name: &method.name,
            declaring,
            method,
            name_uri: &declaring.uri,
            name_span: method.name_span,
        });
    };
    // A `renamemethod old new` *moves* the member: the stub owns no
    // `MethodDef` (the params / body / visibility stay on the declaring
    // record), so the arrival re-keys this entry and the arrival word is the
    // moved member's declaration site.  A plain `deletemethod` — and a
    // `renamemethod old $new` whose destination is computed, which names
    // nothing statically — records no arrival, and the member is simply gone.
    Some(EffectiveMember {
        name: retraction.arrival.as_deref()?,
        declaring,
        method,
        name_uri: &stub.uri,
        name_span: retraction.arrival_span?,
    })
}

/// One class record's `(exports, unexports)` pair **for `side`** — the sided
/// visibility lookup [`WorkspaceIndex::dispatch_chain`] reads.
///
/// `exports`/`unexports` are the instance-side record by contract and
/// `class_exports`/`class_unexports` the class-object-side one; naming the
/// choice once is what keeps a `self unexport m` from ever silencing an
/// identically-named instance method.
fn visibility_sets_for(c: &WorkspaceClass, side: MemberSide) -> (&[String], &[String]) {
    match side {
        MemberSide::Instance => (&c.exports, &c.unexports),
        MemberSide::ClassObject => (&c.class_exports, &c.class_unexports),
    }
}

/// The owner-resolved class edges [`WorkspaceIndex::class_edges`] builds once
/// and [`Self::linearise`] answers any number of receivers from.
struct ClassEdges {
    supers_map: std::collections::HashMap<String, Vec<String>>,
    mixins_map: std::collections::HashMap<String, Vec<String>>,
}

impl ClassEdges {
    /// `class_q`'s full linearisation and its **mixin-free** twin — the
    /// `superclass` spine alone.
    ///
    /// The two are needed together because C's call-chain builder treats the
    /// paths differently: each mixin is entered with a *fresh copy* of the
    /// dispatch flags, so a mixin's `unexport` empties only its own branch
    /// while the same word on the spine decides the whole dispatch.  Both
    /// come from the one canonical
    /// [`tcl_syntax::mro::tcloo_linearise`] over the same resolved edges, so
    /// they cannot disagree about which qualified name a bare `superclass
    /// Device` meant.  Empty when the hierarchy is cyclic or too complex to
    /// linearise (the shared budget guard) — consumers abstain rather than
    /// guess.
    fn linearise(&self, class_q: &str) -> (Vec<String>, Vec<String>) {
        (
            tcl_syntax::mro::tcloo_linearise(class_q, &self.supers_map, &self.mixins_map)
                .unwrap_or_default(),
            self.spine(class_q),
        )
    }

    /// `class_q`'s mixin-free linearisation — the `superclass` spine alone.
    pub(super) fn spine(&self, class_q: &str) -> Vec<String> {
        let no_mixins: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        tcl_syntax::mro::tcloo_linearise(class_q, &self.supers_map, &no_mixins).unwrap_or_default()
    }

    /// The spines `TclOO`'s call-chain builder walks for a receiver of class
    /// `class_q`, in the order it walks them: every reachable `mixin` branch,
    /// then the receiver's own spine. C enters each mixin with a fresh copy
    /// of the dispatch
    /// flags and so lets a branch's `export` / `unexport` govern that branch
    /// alone.
    fn branch_spines(&self, class_q: &str, linearisation: &[String]) -> Vec<Vec<String>> {
        let mut roots: Vec<String> = Vec::new();
        let mut queue: std::collections::VecDeque<String> =
            std::collections::VecDeque::from([class_q.to_owned()]);
        let mut seen: std::collections::HashSet<String> =
            std::collections::HashSet::from([class_q.to_owned()]);
        while let Some(root) = queue.pop_front() {
            for spine_q in self.spine(&root) {
                for mixin in self.mixins_map.get(&spine_q).into_iter().flatten() {
                    if seen.insert(mixin.clone()) {
                        roots.push(mixin.clone());
                        queue.push_back(mixin.clone());
                    }
                }
            }
        }
        // The linearisation is the order the chain builder reaches these
        // classes; a root it does not list sorts last but keeps its discovery
        // order.
        roots.sort_by_key(|root| {
            linearisation
                .iter()
                .position(|c| c == root)
                .unwrap_or(usize::MAX)
        });
        roots
            .into_iter()
            .map(|root| self.spine(&root))
            .chain(std::iter::once(self.spine(class_q)))
            .collect()
    }
}

impl WorkspaceIndex {
    fn resolve_class_relation(&self, owner: &WorkspaceClass, written: &str) -> Option<String> {
        let lookup = owner.relation_lookups.get(written)?.as_ref()?;
        for candidate in lookup.candidates()? {
            let mut reports = self
                .classes()
                .filter(|c| {
                    !c.source_name_ambiguous
                        && c.source_name.as_ref().is_some_and(|source| {
                            source.policy() == lookup.policy() && source.slot() == &candidate
                        })
                })
                .map(|c| c.qualified_name.as_str());
            let Some(report) = reports.next() else {
                continue;
            };
            if reports.any(|other| other != report) {
                return None;
            }
            if self
                .classes()
                .filter(|c| c.qualified_name == report)
                .any(|c| {
                    c.source_name_ambiguous
                        || c.source_name.as_ref().is_none_or(|source| {
                            source.policy() != lookup.policy() || source.slot() != &candidate
                        })
                })
            {
                return None;
            }
            return Some(report.to_owned());
        }
        None
    }

    /// The owner-aware direct parents (superclasses + mixins) of `qname`,
    /// unioned across **every** indexed definition of the class.  A cross-file
    /// `oo::define ::C { ... }` records a second `::C` entry that names no
    /// `superclass`; unioning here keeps the real class's parent edges from
    /// being hidden when such a stub happens to be the first match (without
    /// the union the parent walk picks an arbitrary duplicate and silently
    /// drops the hierarchy).
    fn resolved_parents_of(&self, qname: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for c in self.classes().filter(|c| c.qualified_name == qname) {
            for s in c.superclasses.iter().chain(c.mixins.iter()) {
                if let Some(p) = self.resolve_class_relation(c, s)
                    && seen.insert(p.clone())
                {
                    out.push(p);
                }
            }
        }
        out
    }

    /// The workspace classes that `wc`'s written superclasses + mixins
    /// resolve to, **owner-aware** — each name is resolved relative to
    /// `wc.qualified_name`'s namespace (ancestry → global → unique tail) via
    /// [`resolve_class_name`], never by a bare global tail guess.  Used for
    /// cross-file **supertype** resolution.
    #[must_use]
    pub fn supertype_classes<'a>(&'a self, wc: &WorkspaceClass) -> Vec<&'a WorkspaceClass> {
        let mut out: Vec<&WorkspaceClass> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for name in wc.superclasses.iter().chain(wc.mixins.iter()) {
            let Some(q) = self.resolve_class_relation(wc, name) else {
                continue;
            };
            if !seen.insert(q.clone()) {
                continue;
            }
            out.extend(self.classes().filter(|c| c.qualified_name == q));
        }
        out
    }

    /// Workspace classes that declare `class_qname` as a direct superclass
    /// or mixin, resolving each written super/mixin name **owner-aware**
    /// (relative to the declaring class) so an ambiguous bare name never
    /// manufactures a subtype edge.  Used for cross-file **subtype**
    /// resolution.
    #[must_use]
    pub fn subclasses_of<'a>(&'a self, class_qname: &str) -> Vec<&'a WorkspaceClass> {
        self.classes()
            .filter(|c| {
                c.superclasses
                    .iter()
                    .chain(c.mixins.iter())
                    .any(|s| self.resolve_class_relation(c, s).as_deref() == Some(class_qname))
            })
            .collect()
    }

    /// The **class linearisation** of `class_q` — the order `TclOO` searches
    /// classes for a method implementation (mixins fully linearised first,
    /// then the class, then superclasses; diamond duplicates keep their
    /// late placement — tclsh 9.0.4-pinned via `info object call`).
    ///
    /// A thin workspace adapter over the canonical
    /// [`tcl_syntax::mro::tcloo_linearise`]: the super / mixin edges are
    /// resolved **owner-aware** ([`resolve_class_name`]) and unioned
    /// across every indexed record of each class (an `oo::define` stub
    /// must not hide the creation site's edges).  Empty when the
    /// hierarchy is cyclic or too complex to linearise (the shared
    /// budget guard) — consumers abstain rather than guess.
    #[must_use]
    pub fn class_linearisation(&self, class_q: &str) -> Vec<String> {
        self.class_linearisation_and_spine(class_q).0
    }

    /// [`Self::class_linearisation`] and its **mixin-free** twin — the
    /// `superclass` spine alone — from one edge-map build.
    ///
    /// The two are needed together because C's call-chain builder treats the
    /// paths differently: each mixin is entered with a *fresh copy* of the
    /// dispatch flags, so a mixin's `unexport` empties only its own branch
    /// while the same word on the spine decides the whole dispatch.  Resolving
    /// the edge maps is the expensive half and is O(classes) on its own, so it
    /// is deliberately not paid twice — see
    /// [`Self::subclass_provided_methods`] for what per-call rebuilding costs
    /// the diagnostics worker.
    fn class_linearisation_and_spine(&self, class_q: &str) -> (Vec<String>, Vec<String>) {
        self.class_edges().linearise(class_q)
    }

    /// The owner-resolved `superclass` / `mixin` edge maps over every indexed
    /// class — the expensive half of [`Self::class_linearisation_and_spine`],
    /// built once so a caller with many receivers to linearise does not pay
    /// O(classes) name resolution per receiver (see
    /// [`Self::subclass_provided_methods`] for what per-call rebuilding costs
    /// the diagnostics worker).
    fn class_edges(&self) -> ClassEdges {
        // Build the resolved edge maps over the classes reachable from
        // `class_q` (bounded: every indexed class at worst).
        let mut supers_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut mixins_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for c in self.classes() {
            let owner = c.qualified_name.as_str();
            let resolve = |name: &str| self.resolve_class_relation(c, name);
            let supers = supers_map.entry(owner.to_owned()).or_default();
            for s in c.superclasses.iter().filter_map(|s| resolve(s)) {
                if !supers.contains(&s) {
                    supers.push(s);
                }
            }
            let mixins = mixins_map.entry(owner.to_owned()).or_default();
            for m in c.mixins.iter().filter_map(|m| resolve(m)) {
                if !mixins.contains(&m) {
                    mixins.push(m);
                }
            }
        }
        ClassEdges {
            supers_map,
            mixins_map,
        }
    }

    /// The project-wide **subclass-provided method** view behind the
    /// template-method W308 abstention: for every class, the
    /// instance-side member names dispatchable on some **other** class whose
    /// linearisation contains it, so a base's `my Render` can be refuted by
    /// the concrete subclass that writes `Render` in a sibling document.
    ///
    /// One pass, deliberately: the class-name universe and the owner-resolved
    /// super/mixin edge maps are built **once** and every class is linearised
    /// against them.  Calling [`Self::class_linearisation`] per class instead
    /// rebuilds those maps each time — O(classes²) resolution work — and doing
    /// that on the diagnostics worker after every publish is enough to take
    /// the Performance suite from its ~5-minute baseline to a 60-minute
    /// timeout.  Per-class member names are memoised across linearisations for
    /// the same reason.
    ///
    /// `private` members are excluded — `TclOO` hides them even from an
    /// ancestor's own `my` dispatch — as are class-object-side members
    /// (`classmethod` / `self method`): `my` inside an instance method
    /// dispatches instance-side.  A class whose hierarchy cannot be
    /// linearised (cyclic, or over the shared complexity budget) contributes
    /// nothing: the abstention needs evidence, and none is available for it.
    #[must_use]
    pub fn subclass_provided_methods(
        &self,
    ) -> std::collections::BTreeMap<String, std::collections::BTreeSet<String>> {
        let mut supers_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut mixins_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for c in self.classes() {
            let owner = c.qualified_name.as_str();
            let resolve = |name: &str| self.resolve_class_relation(c, name);
            let supers = supers_map.entry(owner.to_owned()).or_default();
            for s in c.superclasses.iter().filter_map(|s| resolve(s)) {
                if !supers.contains(&s) {
                    supers.push(s);
                }
            }
            let mixins = mixins_map.entry(owner.to_owned()).or_default();
            for m in c.mixins.iter().filter_map(|m| resolve(m)) {
                if !mixins.contains(&m) {
                    mixins.push(m);
                }
            }
        }
        let qnames: std::collections::BTreeSet<String> = self
            .live_classes()
            .map(|c| c.qualified_name.clone())
            .collect();
        let mut member_names: std::collections::HashMap<
            String,
            std::collections::BTreeSet<String>,
        > = std::collections::HashMap::new();
        let mut map: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> =
            std::collections::BTreeMap::new();
        for d in &qnames {
            let lin =
                tcl_syntax::mro::tcloo_linearise(d, &supers_map, &mixins_map).unwrap_or_default();
            if lin.iter().all(|c| c == d) {
                continue;
            }
            let mut dispatchable: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            for x in &lin {
                if !member_names.contains_key(x) {
                    let names = self
                        .effective_members(x)
                        .into_iter()
                        .filter(|m| {
                            m.method.kind != MethodKind::ClassMethod.as_str() && !m.method.private
                        })
                        .map(|m| m.name.to_owned())
                        .collect();
                    member_names.insert(x.clone(), names);
                }
                dispatchable.extend(member_names[x].iter().cloned());
            }
            if dispatchable.is_empty() {
                continue;
            }
            for ancestor in lin.iter().filter(|c| c.as_str() != d.as_str()) {
                map.entry(ancestor.clone())
                    .or_default()
                    .extend(dispatchable.iter().cloned());
            }
        }
        map
    }

    /// The C-Tcl-faithful **method dispatch chain** for an instance of
    /// `receiver_class` calling `method` under `access`: the linearisation's
    /// classes that define an instance-receiver implementation of `method`,
    /// in dispatch order,
    /// visibility-filtered —
    ///
    /// * [`MethodAccess::External`] keeps only **exported**
    ///   implementations (an unexported method is not externally
    ///   callable: `unknown method`, tclsh 9.0.4-pinned);
    /// * [`MethodAccess::Internal`] keeps exported + unexported;
    ///   `private` definitions are visible only when the defining class
    ///   *is* the receiver's own class (`TclOO` private scoping).
    ///
    /// The **first** record is the implementation the call actually
    /// enters — go-to-definition's single target; the rest is the `next`
    /// chain.  Several records of one class (creation site + `oo::define`
    /// stubs) are all kept, adjacent, when each defines the method; a
    /// class exported/unexported by a *different* record than the definer
    /// honours the union of that class's records (last state is
    /// load-order-dependent across files, so any exporting record keeps
    /// the method dispatchable — the navigation-permissive reading).
    #[must_use]
    pub fn method_dispatch_chain<'a>(
        &'a self,
        receiver_class: &str,
        method: &str,
        access: MethodAccess,
    ) -> Vec<&'a WorkspaceClass> {
        self.dispatch_chain(receiver_class, method, access, MemberSide::Instance)
    }

    /// The dispatch chain for a call on the **class's own command**
    /// (`::C cm`) rather than on an instance — the class-object-side twin of
    /// [`Self::method_dispatch_chain`].
    ///
    /// Same rules, read against the other side's tables: `class_method`
    /// declarations instead of `instance_method` ones,
    /// [`WorkspaceClass::class_exports`] / [`WorkspaceClass::class_unexports`]
    /// instead of the instance pair, and [`MemberSide::ClassObject`]
    /// tombstones.  Without it a `self unexport m` would be invisible across
    /// files: the flip would have no channel, and the workspace would keep
    /// resolving a `::C m` the interpreter answers with `unknown method "m"`
    /// (tclsh 9.0.4 / 8.6.14).
    ///
    /// One `TclOO` rule is class-side only: a stock `self method` lives on the
    /// class object that declared it and a **subclass's** class command never
    /// reaches it (`Gadget make` against a parent's `self method make` errors
    /// `unknown method "make"` on 8.6 and 9.0.4), whereas an `ooutil`
    /// `classmethod` does propagate.  Both share the `"classmethod"` receiver
    /// kind, so a `self method` is kept only when the providing record *is* the
    /// receiver class.
    #[must_use]
    pub fn class_method_dispatch_chain<'a>(
        &'a self,
        receiver_class: &str,
        method: &str,
        access: MethodAccess,
    ) -> Vec<&'a WorkspaceClass> {
        self.dispatch_chain(receiver_class, method, access, MemberSide::ClassObject)
    }

    /// Whether the workspace's **class-side visibility union** leaves no
    /// dispatchable implementation of `method` on `class_q`'s own command
    /// under `access` — the *suppression* half of the class-side channel,
    /// consulted by the in-document tier before it answers for a class its
    /// own document declares.
    ///
    /// The revival direction already crosses files: a `self export` written
    /// next door reaches every query through
    /// [`Self::class_method_dispatch_chain`].  Suppression does not travel the
    /// same way, because the declaring document's in-document provider
    /// resolves the member locally and returns before the workspace chain
    /// runs — without this consultation a cross-file `self unexport Cm` /
    /// `self deletemethod Cm` would suppress `C Cm` for every document except
    /// the one the author is editing.  It is deliberately a thin reading of
    /// the same chain fold the cross-file tier resolves through — one decision
    /// function, so the two tiers cannot diverge (the established
    /// `exported_at_import_site` pattern).
    ///
    /// `false` is an abstention as well as a "not suppressed": a class the
    /// index holds no record of, or whose hierarchy the shared linearisation
    /// declines (cycle / budget), yields no suppression evidence, and the
    /// in-document answer stands.  The chain itself carries the standing
    /// unordered-cross-file caveat — any exporting record keeps the member
    /// dispatchable — so an unordered flip pair abstains toward answering.
    #[must_use]
    pub fn class_member_dispatch_suppressed(
        &self,
        class_q: &str,
        method: &str,
        access: MethodAccess,
    ) -> bool {
        if !self.classes().any(|c| c.qualified_name == class_q) {
            return false;
        }
        if self.class_linearisation(class_q).is_empty() {
            return false;
        }
        self.class_method_dispatch_chain(class_q, method, access)
            .is_empty()
    }

    /// The one dispatch-chain walk both sides go through — see
    /// [`Self::method_dispatch_chain`] for the rules and
    /// [`Self::class_method_dispatch_chain`] for what the class side changes.
    ///
    /// Sharing the walk is what keeps the two sides' answers consistent: the
    /// record ordering, the retraction gate, the effective-export union and the
    /// visibility filter are decided once, and `side` only selects *which*
    /// table each of them reads.
    fn dispatch_chain<'a>(
        &'a self,
        receiver_class: &str,
        method: &str,
        access: MethodAccess,
        side: MemberSide,
    ) -> Vec<&'a WorkspaceClass> {
        let edges = self.class_edges();
        self.dispatch_chain_over(&edges, receiver_class, method, access, side)
    }

    /// [`Self::dispatch_chain`] over edge maps the caller already has — the
    /// form a bulk query uses so the O(classes) edge resolution behind
    /// [`Self::class_edges`] is paid once for many receivers.
    fn dispatch_chain_over<'a>(
        &'a self,
        edges: &ClassEdges,
        receiver_class: &str,
        method: &str,
        access: MethodAccess,
        side: MemberSide,
    ) -> Vec<&'a WorkspaceClass> {
        let mut out: Vec<&WorkspaceClass> = Vec::new();
        let linearisation = edges.linearise(receiver_class).0;
        // The receiver's **effective** export flag, read once for the whole
        // walk.  A subclass can `export` / `unexport` a name it
        // inherits without redeclaring it: `export` / `unexport` accept a name
        // their class does not define and create a body-less table entry whose
        // only content is the flag, so the flag comes from the most specific
        // spine class that *mentions* the member while the implementation
        // still comes from the first class that declares a body.  Oracle,
        // byte-identical on tclsh 8.6.16 and 9.0.4:
        //
        //     oo::class create Base   { method tick {} { return base } }
        //     oo::class create Child  { superclass Base }
        //     oo::define Child { unexport tick }
        //     [Child new] tick    ;# -> unknown method "tick"
        //     oo::class create Base3  { method tock {} { return b3 } ; unexport tock }
        //     oo::class create Child3 { superclass Base3 ; export tock }
        //     [Child3 new] tock   ;# -> b3
        let branches: Vec<(Vec<String>, Option<bool>)> = if access == MethodAccess::External {
            edges
                .branch_spines(receiver_class, &linearisation)
                .into_iter()
                .map(|spine| {
                    let state = self.spine_export_state(&spine, method, side);
                    (spine, state)
                })
                .collect()
        } else {
            Vec::new()
        };
        for class_q in linearisation {
            // Several records of one class (its creation site plus every
            // `oo::define` stub, possibly spread over files) are all kept, and
            // the *first* is what go-to-definition answers with — so the order
            // has to be a property of the workspace, not of when each document
            // happened to be indexed.  Document URI then source position is
            // that stable order.
            let records = self.class_records(&class_q);
            // The class-level effective export union **for this side**: any
            // record exporting the name keeps it callable; explicit unexports
            // matter only when no record exports it.  The two sides never share
            // a set — a `self unexport m` must not silence an identically-named
            // instance method, and vice versa.
            let any_exports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).0.iter().any(|e| e == method));
            let any_unexports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).1.iter().any(|e| e == method));
            // The class's member set — retractions applied, arrivals
            // re-keyed — is decided once by [`Self::effective_members`]
            // rather than here.  A `deletemethod`ed member is
            // absent from it, so the chain for that name is empty exactly as
            // it is for a method no record defines; a `renamemethod`ed one is
            // present under its destination and absent under its source.
            for em in self.effective_members(&class_q) {
                if em.name != method || method_side(em.method) != side {
                    continue;
                }
                // A stock `self method` is not inherited: the class object
                // that declared it is the only one whose command reaches it
                // (`Gadget make` against a parent's `self method make` ->
                // `unknown method "make"`, 8.6 / 9.0.4). An `ooutil`
                // `classmethod` shares the receiver kind but does propagate.
                if side == MemberSide::ClassObject
                    && em.method.is_self_method
                    && em.declaring.qualified_name != receiver_class
                {
                    continue;
                }
                // Effective export across the class's records: an explicit
                // `export` anywhere wins, else an explicit `unexport`
                // anywhere, else the definer's own effective state.  (True
                // cross-file order is load-order; explicit-export-wins is the
                // navigation-permissive reading.)  Visibility travels with the
                // *body*, not an arrival name's leading-capital default
                // (oracle, tclsh 9.0.4 / 8.6.14: `oo::class create ::R4
                // {method Priv {} {…}; renamemethod Priv pub}` leaves `info
                // class methods ::R4` empty while `-private` lists `pub`), so
                // the source member's own record is what this reads.
                //
                // …and a provider answers to the effective flag of the
                // **branch** that reaches it, because C enters each mixin with
                // a fresh copy of the dispatch flags and so lets a branch's
                // `export` / `unexport` govern that branch alone.
                let branch_state = branches
                    .iter()
                    .find(|(spine, _)| spine.contains(&class_q))
                    .and_then(|(_, state)| *state);
                let exported = match branch_state {
                    Some(exported) => exported,
                    None if any_exports => true,
                    None if any_unexports => false,
                    None => em.method.exported,
                };
                let visible = match access {
                    MethodAccess::External => exported && !em.method.private,
                    MethodAccess::Internal => !em.method.private || class_q == receiver_class,
                };
                if visible {
                    out.push(em.declaring);
                }
            }
        }
        out
    }

    /// Which of `receivers` dispatch `method` **externally** into `family` —
    /// the bulk form of "does [`Self::method_dispatch_chain`] land on one of
    /// these classes", answering every receiver from one
    /// [`Self::class_edges`] build.
    ///
    /// The caller is the cross-file method-family pass, which needs the answer
    /// per inheriting class before it scans each document: a
    /// captured `[self]` object command in a subclass body is a call site of
    /// the family's declaration only when that subclass can actually dispatch
    /// the name.  Asking [`Self::method_dispatch_chain`] once per inheritor
    /// would re-resolve every class edge each time — the O(classes²) shape
    /// [`Self::subclass_provided_methods`] documents the cost of.
    #[must_use]
    pub fn external_dispatch_receivers<'a>(
        &self,
        receivers: impl IntoIterator<Item = &'a str>,
        family: &[&str],
        method: &str,
        is_classmethod: bool,
    ) -> std::collections::HashSet<String> {
        let side = if is_classmethod {
            MemberSide::ClassObject
        } else {
            MemberSide::Instance
        };
        let edges = self.class_edges();
        receivers
            .into_iter()
            .filter(|cq| {
                // Non-empty is not enough: the chain must land *in this
                // family*.  A receiver whose external dispatch reaches some
                // other class's implementation (a live `mixin` branch ahead of
                // the spine, say) is dispatching that other member, and its
                // capture belongs to that family instead.
                self.dispatch_chain_over(&edges, cq, method, MethodAccess::External, side)
                    .first()
                    .is_some_and(|entry| family.contains(&entry.qualified_name.as_str()))
            })
            .map(ToOwned::to_owned)
            .collect()
    }

    /// The export flag an external dispatch reads for `method` along `spine`
    /// — the mixin-free linearisation from
    /// [`Self::class_linearisation_and_spine`] — or `None` when no class on it
    /// mentions the name at all, so there is no state to read and the
    /// per-record union in [`Self::dispatch_chain`] stands.
    ///
    /// "Mentions" is deliberately wider than "declares": `export` / `unexport`
    /// record a name their class need not define, and that body-less entry is
    /// the whole point of this walk.  Within one class the
    /// records are unordered across files, so the same
    /// explicit-export-wins precedence [`Self::dispatch_chain`] applies to a
    /// declaring class applies here too.
    fn spine_export_state(&self, spine: &[String], method: &str, side: MemberSide) -> Option<bool> {
        for class_q in spine {
            let records = self.class_records(class_q);
            let any_exports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).0.iter().any(|e| e == method));
            let any_unexports = records
                .iter()
                .any(|c| visibility_sets_for(c, side).1.iter().any(|e| e == method));
            if any_exports {
                return Some(true);
            }
            if any_unexports {
                return Some(false);
            }
            if let Some(em) = self
                .effective_members(class_q)
                .into_iter()
                .find(|em| em.name == method && method_side(em.method) == side)
            {
                return Some(em.method.exported);
            }
        }
        None
    }

    /// Every record of the class `class_q`, in the workspace's stable order.
    ///
    /// Several records of one class (its creation site plus every
    /// `oo::define` stub, possibly spread over files) are all kept, and the
    /// *first* is what go-to-definition answers with — so the order has to be
    /// a property of the workspace, not of when each document happened to be
    /// indexed.  Document URI then source position is that order.
    #[must_use]
    pub fn class_records<'a>(&'a self, class_q: &str) -> Vec<&'a WorkspaceClass> {
        let mut records: Vec<&WorkspaceClass> = self
            .classes()
            .filter(|c| c.qualified_name == class_q)
            .collect();
        records.sort_by_key(|c| (c.uri.as_str(), c.name_span.start(), c.name_span.end()));
        records
    }

    /// The members of `class_q` as the workspace sees them: every record's own
    /// declarations, with the class's cross-document retractions applied and
    /// its arrivals re-keyed.  Both receiver sides, in
    /// [`Self::class_records`] order then declaration order — filter on
    /// [`EffectiveMember::method`]'s `kind` (via `WorkspaceClass`'s own
    /// instance/class split) for one side.
    ///
    /// This is the single rule for "which members does this class have":
    /// [`Self::dispatch_chain`] resolves one name against it and every
    /// *enumeration* (`workspace/symbol`, an outline, a member completion
    /// universe) lists it, so the two cannot disagree: a member moved by a
    /// cross-file `renamemethod` is enumerated under the name it dispatches
    /// as, not the one its declaring record spells.
    ///
    /// Inheritance is **not** applied: these are the class's own members.
    /// Walk [`Self::class_linearisation`] for the inherited set.
    ///
    /// Carries the tombstone channel's unordered-cross-file caveat: true load
    /// order is not knowable from the index, so a retraction recorded by any
    /// record of the class applies to every record of it.
    #[must_use]
    pub fn effective_members<'a>(&'a self, class_q: &str) -> Vec<EffectiveMember<'a>> {
        let retractions = self.retraction_index();
        let mut out: Vec<EffectiveMember<'a>> = Vec::new();
        for declaring in self.class_records(class_q) {
            out.extend(
                declaring
                    .methods
                    .iter()
                    .filter_map(|method| effective_member(&retractions, declaring, method)),
            );
        }
        out
    }

    /// Every cross-document member retraction in the workspace, keyed by the
    /// `(class, member, side)` it removes — the lookup table the member fold
    /// ([`effective_member`]) applies.
    ///
    /// Built once per query rather than rescanned per member: a retraction
    /// recorded by *any* record of a class applies to every record of it, so
    /// the naive form is a scan of the class's records per declared method,
    /// which is quadratic in a workspace's class count on a path
    /// (`workspace/symbol`) that runs per keystroke.  The map is empty in the
    /// overwhelmingly common case — no document retracts anything — and the
    /// walk then costs no more than a plain member scan.
    ///
    /// Ties are broken by the workspace's stable record order
    /// ([`Self::class_records`]), so which stub an arrival is attributed to
    /// does not depend on indexing order.
    fn retraction_index(&self) -> RetractionIndex<'_> {
        self.retraction_index_excluding(&HashSet::new())
    }

    /// Build the cross-document member fold without records contributed by
    /// excluded document slots. Filtering after the map is folded is unsound:
    /// a stable first retraction from an excluded document may have displaced
    /// a later live retraction for the same member.
    pub(super) fn retraction_index_excluding(
        &self,
        excluded_slots: &HashSet<usize>,
    ) -> RetractionIndex<'_> {
        let mut records: Vec<&WorkspaceClass> = self
            .docs
            .iter()
            .enumerate()
            .filter(|(slot, _)| !excluded_slots.contains(slot))
            .flat_map(|(_, doc)| doc.classes.iter())
            .filter(|c| !c.retracted_members.is_empty())
            .collect();
        if records.is_empty() {
            return RetractionIndex::default();
        }
        records.sort_by_key(|c| (c.uri.as_str(), c.name_span.start(), c.name_span.end()));
        let mut out = RetractionIndex::default();
        for c in records {
            for r in &c.retracted_members {
                out.entry((c.qualified_name.as_str(), r.member.as_str(), r.side))
                    .or_insert((c, r));
            }
        }
        out
    }

    /// The **cross-file override family** of `method` seeded at
    /// `seed_class`: every indexed class that directly defines `method` and
    /// sits in the same subtype-connected component as `seed_class` (or the
    /// ancestor that provides `method` to it).
    ///
    /// This is the workspace-wide analogue of the single-document override
    /// family used by method rename: a method (re)defined up or down the
    /// hierarchy is one polymorphic name, so renaming it must touch every
    /// class that defines it across the whole workspace.  Superclass/mixin
    /// edges are resolved **owner-aware** (via [`resolve_class_name`]), so an
    /// ambiguous bare parent name never fabricates a connection.  The
    /// returned set always includes the seed's provider and is empty only
    /// when `method` is neither defined nor inherited from any indexed
    /// class reachable from `seed_class`.
    #[must_use]
    pub fn method_override_family<'a>(
        &'a self,
        seed_class: &str,
        method: &str,
    ) -> Vec<&'a WorkspaceClass> {
        let family = self.method_family_qnames(seed_class, method);
        let family_set: std::collections::HashSet<&str> =
            family.iter().map(String::as_str).collect();
        self.classes()
            .filter(|c| family_set.contains(c.qualified_name.as_str()))
            .collect()
    }

    /// Indexed classes that **inherit** `method` from the override family of
    /// `(seed_class, method)` but do not define it themselves — the pure
    /// inheritors whose `my method` / `$obj method` sites a rename must also
    /// rewrite, even though they contribute no declaration.
    ///
    /// A class is included only when it inherits `method` (some ancestor
    /// defines it) **and every** method-defining ancestor it can reach is in
    /// the family.  That keeps the result sound under multiple inheritance:
    /// if a class could resolve `method` to a definer *outside* the family
    /// (a disjoint same-named method), it is abstained on rather than risk an
    /// over-rename.  The workspace index carries ancestry but not a full
    /// cross-file MRO, so this is deliberately conservative.
    #[must_use]
    pub fn method_inheritor_classes<'a>(
        &'a self,
        seed_class: &str,
        method: &str,
    ) -> Vec<&'a WorkspaceClass> {
        let family = self.method_family_qnames(seed_class, method);
        if family.is_empty() {
            return Vec::new();
        }
        let family_set: std::collections::HashSet<&str> =
            family.iter().map(String::as_str).collect();
        let parents = |qname: &str| self.resolved_parents_of(qname);
        let defines = |qname: &str| {
            self.classes()
                .any(|c| c.qualified_name == qname && c.defines_method(method))
        };
        self.classes()
            .filter(|c| {
                // A definer is handled by the family itself, not here.
                if c.defines_method(method) || family_set.contains(c.qualified_name.as_str()) {
                    return false;
                }
                // Every method-defining ancestor this class can reach.
                let mut stack = parents(&c.qualified_name);
                let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
                let mut defining_ancestors: Vec<String> = Vec::new();
                while let Some(p) = stack.pop() {
                    if seen.insert(p.clone()) {
                        if defines(&p) {
                            defining_ancestors.push(p.clone());
                        }
                        stack.extend(parents(&p));
                    }
                }
                // Inherits `method` (has a definer ancestor) and cannot resolve
                // it to a definer outside the family.
                !defining_ancestors.is_empty()
                    && defining_ancestors
                        .iter()
                        .all(|a| family_set.contains(a.as_str()))
            })
            .collect()
    }

    /// The qualified names of the override family of `(seed_class, method)`:
    /// every indexed class that directly defines `method` and sits in the
    /// same subtype-connected component as `seed_class` (or the ancestor that
    /// provides `method` to it).  Shared by [`Self::method_override_family`]
    /// and [`Self::method_inheritor_classes`].  Empty when `method` is neither
    /// defined nor inherited from any indexed class reachable from
    /// `seed_class`.
    fn method_family_qnames(&self, seed_class: &str, method: &str) -> Vec<String> {
        // Owner-aware direct parents (superclasses + mixins) of each class,
        // unioned across every indexed definition (a cross-file `oo::define`
        // stub must not hide the real class's parents).
        let parents = |qname: &str| self.resolved_parents_of(qname);
        // `parent` is a (transitive) ancestor of `child`.
        let is_ancestor = |child: &str, parent: &str| -> bool {
            let mut stack = parents(child);
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            while let Some(p) = stack.pop() {
                if p == parent {
                    return true;
                }
                if seen.insert(p.clone()) {
                    stack.extend(parents(&p));
                }
            }
            false
        };
        let connected = |a: &str, b: &str| a == b || is_ancestor(a, b) || is_ancestor(b, a);
        // A member that *arrived* through a cross-file `renamemethod` counts
        // as defined for family purposes: the class really has it, the
        // declaring record just spells it under the source name.
        let class_defines = |qname: &str| {
            self.classes().any(|c| {
                c.qualified_name == qname && (c.defines_method(method) || c.arrives_method(method))
            })
        };
        // Seed: the class under the cursor if it defines `method`, else the
        // nearest ancestor that does (any definer ancestor is in the same
        // family, so the first one found seeds it).
        let seed = if class_defines(seed_class) {
            seed_class.to_string()
        } else {
            let mut stack = parents(seed_class);
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut found = None;
            while let Some(p) = stack.pop() {
                if class_defines(&p) {
                    found = Some(p);
                    break;
                }
                if seen.insert(p.clone()) {
                    stack.extend(parents(&p));
                }
            }
            match found {
                Some(p) => p,
                None => return Vec::new(),
            }
        };
        // Every indexed definer of `method` (qualified names, de-duplicated).
        let definers: Vec<String> = {
            let mut ds: Vec<String> = self
                .classes()
                .filter(|c| c.defines_method(method) || c.arrives_method(method))
                .map(|c| c.qualified_name.clone())
                .collect();
            ds.sort();
            ds.dedup();
            ds
        };
        // Grow the weakly-connected component of definers containing `seed`.
        let mut family = vec![seed];
        let mut changed = true;
        while changed {
            changed = false;
            for d in &definers {
                if family.iter().any(|f| f == d) {
                    continue;
                }
                if family.iter().any(|f| connected(f, d)) {
                    family.push(d.clone());
                    changed = true;
                }
            }
        }
        family
    }
}
