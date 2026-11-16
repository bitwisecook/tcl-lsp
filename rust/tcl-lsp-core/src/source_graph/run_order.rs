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

//! `RunOrder`: the source-order model over a source forest and the comparisons made under it.

use super::{
    Placed, RunEdge, RunEdgeKind, RunPoint, SourceNavigationOrder, SourceNavigationOrderPurpose,
    SourceOrderContext,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A `String`-keyed map on the [`RunOrder`] hot path.
///
/// Every one of these is probed from inside the cross-document import walk,
/// which runs per (invocation × in-scope import), so the default `SipHash`
/// over an owned URI string is itself a measurable share of a 26 s
/// `textDocument/references` (338 of 439 samples under
/// [`RunOrder::alternatives`] were in `sip::Hasher::write`).  These keys are
/// workspace-local document URIs, never attacker-chosen, so the `HashDoS`
/// resistance `SipHash` buys is not worth its cost here.
pub(super) type UriMap<V> = rustc_hash::FxHashMap<String, V>;

/// The [`UriMap`] of sets — same keys, same reasoning.
pub(super) type UriSet = rustc_hash::FxHashSet<String>;

/// A constructed source-order model, or purpose-bound conditional workspace
/// navigation when built with authentic indexed source contexts. Neither mode
/// is an actual interpreter chronology. The relation both
/// wildcard-import tiers rank cross-document events with (design in
/// `docs/design/analysis/import-order-source-graph.md` §6).
///
/// The coordinate explanation below describes the constructed hypothesis.
/// Actual `WorkspaceIndex` results keep Conditional purpose even when its
/// relative coordinates are unique; they do not authorize event suppression.
/// The one exception is two straight-line positions of the same indexed
/// document, which that document's own sequence orders
/// ([`SourceNavigationOrderPurpose::DocumentSequence`]).
///
/// # Why an order exists at all
///
/// Nothing in a Tcl source tree says which of two files runs first, which is
/// why every cross-document import-lifecycle event was passed *unordered* and
/// the shared decision functions abstained toward answering: a foreign
/// `namespace export` counted, a foreign `namespace export -clear` revoked
/// nothing, a foreign `namespace forget` revoked nothing.  Two statements are
/// the exception:
///
/// - a **`source`** is an ordinary statement of the sourcing file that runs
///   the whole sourced file *at that point*, so the DFS of the `source` forest
///   **is** the run order ([`RunEdgeKind::Source`]);
/// - a **`package require`** guarantees the providing file has run by the time
///   it returns — but not that it ran *there*, since a package already loaded
///   makes the require evaluate nothing ([`RunEdgeKind::PackageRequire`]).
///
/// # The relation
///
/// Each point is lifted to its root-ward key: the offsets of the statements
/// that entered each document down the path, then the point's own offset.  Two
/// keys are compared element-wise; the first index where they differ is their
/// **deepest common document**, and there the existing single-document rules
/// apply unchanged — [`tcl_compiler::analyser::indirection::in_effect_within`]
/// for [`Self::has_run`], [`crate::namespace_import::load_order`] for
/// [`Self::cmp_run`].  One relation, both questions, so the gate and the
/// conflict rule cannot drift (design §6.1: a `has_run` predicate alone is not
/// enough — the folds rank events against *each other*, and a partial order
/// has no `max`).
///
/// A `package require` edge changes nothing about that projection; it only
/// marks the position it contributes as a **bound** rather than a fact
/// ([`Placed::exact`]), and [`Self::trusted`] then reads off the half of the
/// comparison the bound survives.
///
/// # Where it abstains, deliberately
///
/// `None` — incomparable — everywhere the order is not a *fact*, which the
/// callers fold to the same "abstain toward answering" direction `at: None`
/// took before:
///
/// - the two documents sit in **different trees** (no path joins them) — the
///   overwhelmingly common case, and exactly today's behaviour;
/// - either document is **ambiguous**: entered from two different statements,
///   or on a cycle.  Tcl tolerates re-sourcing a file, so a document with two
///   entry points has no unique position and must not be given one;
/// - either document is unknown to the graph;
/// - the answer would rest on the *late* side of a `package require` bound —
///   see [`Self::trusted`].
///
/// Two points in the **same** document never consult the graph at all: their
/// order is already fixed by the same rule.  A workspace with no resolvable
/// edge therefore orders nothing across documents.
///
/// Only *literal*, resolvable `source` targets become `source` edges — the
/// caller filters those — because `source $dir/x.tcl` sequences nothing; the
/// caller's soundness gates on `package require` edges are on
/// [`crate::workspace_index::WorkspaceIndex::package_run_edges`].  A statement
/// written inside a proc body is kept and judged by `in_effect_within` like
/// any other body-local statement, the same leniency
/// [`ancestor_prefer_latest_raised`] already applies to the same edges.
#[derive(Debug, Clone, Default)]
pub struct RunOrder {
    /// None is the explicitly constructed compatibility model. Workspace
    /// navigation always retains Some, including unavailable source owners.
    source_contexts: Option<UriMap<SourceOrderContext>>,
    /// Root-ward path of every document the graph knows: root-first positions
    /// of the `source` statements that enter the next document down.  Empty
    /// for a root.
    pub(super) paths: UriMap<Vec<Placed>>,
    /// Root document of every keyed document (itself, for a root).
    root_of: UriMap<String>,
    /// Documents with no unique position — re-sourced from two sites, or on a
    /// cycle.  Every cross-document comparison touching one abstains.
    pub(super) ambiguous: UriSet,
    /// [`Self::alternatives`]'s answer, per **document** rather than per root.
    ///
    /// The `package require` sites that load a document's tree are a fact about
    /// its root, and whether the document may use them is a fact about its own
    /// ambiguity — both fixed at build time.  Resolving that per call meant up
    /// to three hash probes on owned URI strings inside the import walk, and
    /// the `package_entries.is_empty()` fast path that was supposed to make it
    /// free vanished the moment *any* document in the workspace contributed a
    /// package edge.  Folding it into one probe of one map costs
    /// nothing extra: the site vectors are shared with `Arc`, so a root's
    /// entry is stored once however many documents it covers, and a workspace
    /// with no package edge leaves the map empty and every probe a miss.
    package_alternatives: UriMap<Arc<[OwnedRunPoint]>>,
}

/// A [`RunPoint`] the order has to keep past the borrow it was built from —
/// the `package require` sites in [`RunOrder::package_entries`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OwnedRunPoint {
    pub(super) uri: String,
    pub(super) at: u32,
    pub(super) enclosing_body: Option<tcl_lexer::Span>,
}

impl OwnedRunPoint {
    fn as_point(&self) -> RunPoint<'_> {
        RunPoint {
            uri: &self.uri,
            at: self.at,
            enclosing_body: self.enclosing_body,
        }
    }
}

impl RunOrder {
    /// Build an explicitly constructed source-order hypothesis. This pure
    /// compatibility ingress authenticates no execution or original source.
    /// Workspace navigation uses `for_source_navigation` and typed answers.
    ///
    /// `edges` are `(parent, child)` with the parent-side position of the
    /// statement that brings `child` into the run; the caller has already
    /// dropped non-literal and unresolvable `source` targets and applied the
    /// `package require` soundness gates.  A child `source`d from two distinct
    /// sites — or from itself — is marked ambiguous, as is everything below it
    /// and every document on a cycle.
    ///
    /// [`RunEdgeKind::PackageRequire`] edges are kept **out** of the tree: a
    /// package is loaded once, by whichever require runs first, so a provider
    /// required from twenty files has twenty upper bounds and no unique
    /// position.  They are recorded as [`Self::alternatives`] instead, which is
    /// all the bound can support.  A document that is both `source`d and
    /// `package require`d **is** ambiguous — it would then have a tree position
    /// claiming it ran exactly there and a bound saying it had already run
    /// earlier, and those can contradict.
    #[must_use]
    pub fn build(edges: &[RunEdge]) -> Self {
        Self::build_with_source_refusals(edges, UriSet::default())
    }

    fn build_with_source_refusals(edges: &[RunEdge], mut ambiguous: UriSet) -> Self {
        let mut parent_of: HashMap<&str, (&str, Placed)> = HashMap::new();
        let mut documents: HashSet<&str> = HashSet::new();
        let mut package_entries: HashMap<String, Vec<OwnedRunPoint>> = HashMap::new();
        for edge in edges {
            documents.insert(edge.parent.as_str());
            documents.insert(edge.child.as_str());
            let entry = Placed {
                at: edge.at,
                enclosing_body: edge.enclosing_body,
                exact: edge.kind.is_exact(),
            };
            if edge.parent == edge.child {
                ambiguous.insert(edge.child.clone());
                continue;
            }
            if edge.kind == RunEdgeKind::PackageRequire {
                package_entries
                    .entry(edge.child.clone())
                    .or_default()
                    .push(OwnedRunPoint {
                        uri: edge.parent.clone(),
                        at: edge.at,
                        enclosing_body: edge.enclosing_body,
                    });
                continue;
            }
            match parent_of.entry(edge.child.as_str()) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert((edge.parent.as_str(), entry));
                }
                std::collections::hash_map::Entry::Occupied(slot) => {
                    // A second, different entry site: no unique position.
                    if *slot.get() != (edge.parent.as_str(), entry) {
                        ambiguous.insert(edge.child.clone());
                    }
                }
            }
        }
        // A document that is both `source`d and `package require`d has two
        // incompatible stories about when it ran; keep neither.
        for child in package_entries.keys() {
            if parent_of.contains_key(child.as_str()) {
                ambiguous.insert(child.clone());
            }
        }
        let mut paths: UriMap<Vec<Placed>> = UriMap::default();
        let mut root_of: UriMap<String> = UriMap::default();
        for &doc in &documents {
            let mut chain: Vec<Placed> = Vec::new();
            let mut seen: HashSet<&str> = HashSet::new();
            let mut cursor = doc;
            let root = loop {
                if !seen.insert(cursor) {
                    // A `source` cycle: no document on it has a position.
                    for &node in &seen {
                        ambiguous.insert(node.to_owned());
                    }
                    break None;
                }
                match parent_of.get(cursor) {
                    Some(&(parent, entry)) => {
                        chain.push(entry);
                        cursor = parent;
                    }
                    None => break Some(cursor),
                }
            };
            let Some(root) = root else { continue };
            // Built child-first; the key is read root-first.
            chain.reverse();
            paths.insert(doc.to_owned(), chain);
            root_of.insert(doc.to_owned(), root.to_owned());
        }
        // Ambiguity is inherited: a document below an ambiguous one has no
        // unique position either.
        let inherited: Vec<String> = paths
            .keys()
            .filter(|doc| {
                let mut cursor = doc.as_str();
                while let Some(&(parent, _)) = parent_of.get(cursor) {
                    if ambiguous.contains(parent) {
                        return true;
                    }
                    cursor = parent;
                }
                false
            })
            .cloned()
            .collect();
        ambiguous.extend(inherited);
        let package_alternatives =
            Self::fold_package_alternatives(&documents, &root_of, &ambiguous, &package_entries);
        Self {
            source_contexts: None,
            paths,
            root_of,
            ambiguous,
            package_alternatives,
        }
    }

    /// Reuse the same source forest/cycle/re-source/package-bound algorithm,
    /// but retain the actual indexed source owners and conditional purpose.
    pub(crate) fn for_source_navigation(
        edges: &[RunEdge],
        contexts: impl Iterator<Item = (String, SourceOrderContext)>,
    ) -> Self {
        let source_contexts: UriMap<_> = contexts.collect();
        let mut unknown = UriSet::default();
        let edges = edges
            .iter()
            .filter_map(|edge| {
                let Some(site) = source_contexts
                    .get(&edge.parent)
                    .and_then(|context| SourceOrderContext::edge_site(context, edge))
                else {
                    unknown.insert(edge.child.clone());
                    return None;
                };
                Some(RunEdge {
                    enclosing_body: site.body,
                    ..edge.clone()
                })
            })
            .collect::<Vec<_>>();
        let mut order = Self::build_with_source_refusals(&edges, unknown);
        order.source_contexts = Some(source_contexts);
        order
    }

    /// Source relation for navigation, retaining its own purpose. Two
    /// straight-line positions of one indexed document are ordered by that
    /// document's sequence ([`SourceNavigationOrderPurpose::DocumentSequence`]);
    /// every other source/package path stays Conditional and establishes no
    /// executed event.
    #[must_use]
    pub fn navigation_precedence(
        &self,
        event: RunPoint<'_>,
        query: RunPoint<'_>,
    ) -> Option<SourceNavigationOrder<bool>> {
        if let Some(known) = self.document_sequence(event, query, |event, query| {
            tcl_compiler::analyser::indirection::in_effect_within(event, query, None)
        }) {
            return Some(known);
        }
        let answer = self.precedence_relation(event, query)?;
        Some(self.source_navigation_answer(answer))
    }

    /// Source-model ranking; only a constructed model or one document's
    /// straight-line sequence can become a conflict.
    #[must_use]
    pub fn navigation_comparison(
        &self,
        a: RunPoint<'_>,
        b: RunPoint<'_>,
    ) -> Option<SourceNavigationOrder<std::cmp::Ordering>> {
        use crate::namespace_import::load_order;
        if let Some(known) =
            self.document_sequence(a, b, |a, b| load_order(a, false).cmp(&load_order(b, false)))
        {
            return Some(known);
        }
        self.site_point(a)?;
        self.site_point(b)?;
        let answer = self.comparison_relation(a, b)?;
        Some(self.source_navigation_answer(answer))
    }

    /// The end of `uri` — after that document completes normally — for a
    /// consumer with no query point of its own, when the document's
    /// sequence admits it. None keeps such a consumer conditional.
    #[must_use]
    pub(crate) fn document_end<'a>(&self, uri: &'a str) -> Option<RunPoint<'a>> {
        let at = self.source_contexts.as_ref()?.get(uri)?.sequence_end()?;
        Some(RunPoint {
            uri,
            at,
            enclosing_body: None,
        })
    }

    /// Whether this is purpose-bound workspace navigation rather than an
    /// explicitly constructed model. Such an order withholds a removal whose
    /// own position against the query point is not known.
    #[must_use]
    pub(crate) const fn is_source_navigation(&self) -> bool {
        self.source_contexts.is_some()
    }

    /// The same-document straight-line relation:
    /// both points in one indexed document and both sequence positions of
    /// it. Supplied body spans are ignored; the retained inventory decides.
    fn document_sequence<T>(
        &self,
        a: RunPoint<'_>,
        b: RunPoint<'_>,
        relation: impl FnOnce(u32, u32) -> T,
    ) -> Option<SourceNavigationOrder<T>> {
        if a.uri != b.uri {
            return None;
        }
        let context = self.source_contexts.as_ref()?.get(a.uri)?;
        (context.sequence_position(a.at) && context.sequence_position(b.at)).then(|| {
            SourceNavigationOrder {
                relation: relation(a.at, b.at),
                purpose: SourceNavigationOrderPurpose::DocumentSequence,
            }
        })
    }

    fn source_navigation_answer<T>(&self, answer: T) -> SourceNavigationOrder<T> {
        SourceNavigationOrder {
            relation: answer,
            purpose: if self.source_contexts.is_some() {
                SourceNavigationOrderPurpose::Conditional
            } else {
                SourceNavigationOrderPurpose::Constructed
            },
        }
    }

    /// Conditional compatibility within an actually represented statement
    /// horizon. An unentered sibling/deferred body is never a root event.
    pub(crate) fn source_advice_statement_precedence(
        &self,
        event: RunPoint<'_>,
        query: RunPoint<'_>,
    ) -> Option<bool> {
        let event = self.original_point(event)?;
        let query = self.original_point(query)?;
        if event.enclosing_body.is_some()
            && (event.uri != query.uri || event.enclosing_body != query.enclosing_body)
        {
            return None;
        }
        let (event, query) = self.common_original_frame(event, query)?;
        let relation = tcl_compiler::analyser::indirection::in_effect_within(
            event.at,
            query.at,
            query.enclosing_body,
        );
        Self::trusted(relation, event, query).then_some(relation)
    }

    pub(crate) fn source_advice_statement_comparison(
        &self,
        a: RunPoint<'_>,
        b: RunPoint<'_>,
    ) -> Option<std::cmp::Ordering> {
        let a = self.site_point(a)?;
        let b = self.site_point(b)?;
        if (a.enclosing_body.is_some() || b.enclosing_body.is_some())
            && (a.uri != b.uri || a.enclosing_body != b.enclosing_body)
        {
            return None;
        }
        self.comparison_relation(a, b)
    }

    /// A declared readonly end-of-document query, not an original command or
    /// executed completion. Unknown tails/entry remain debt at the caller.
    pub(crate) fn source_advice_after_document(
        &self,
        event: RunPoint<'_>,
        uri: &str,
    ) -> Option<bool> {
        let contexts = self.source_contexts.as_ref()?;
        let context = contexts.get(uri)?;
        let event = self.original_point(event)?;
        // Root text ending does not enter a deferred procedure/method body.
        if event.enclosing_body.is_some() {
            return None;
        }
        let query = RunPoint {
            uri,
            at: u32::try_from(context.image.bytes().len()).ok()?,
            enclosing_body: None,
        };
        let (event, query) = self.common_original_frame(event, query)?;
        let relation = tcl_compiler::analyser::indirection::in_effect_within(
            event.at,
            query.at,
            query.enclosing_body,
        );
        Self::trusted(relation, event, query).then_some(relation)
    }

    /// Resolve [`Self::alternatives`] for every document up front: the root
    /// walk and the ambiguity test do not depend on the point being asked
    /// about, so they belong here rather than on the per-call path.
    ///
    /// A root's site vector is shared by every document under it, so a tree is
    /// stored once however deep it is.  A document the graph knows only as a
    /// `package require` target is its own root, so it is keyed too.  An
    /// ambiguous document is left out entirely — that is the abstention the
    /// old per-call `self.ambiguous.contains` performed, moved rather than
    /// dropped.
    fn fold_package_alternatives(
        documents: &HashSet<&str>,
        root_of: &UriMap<String>,
        ambiguous: &UriSet,
        package_entries: &HashMap<String, Vec<OwnedRunPoint>>,
    ) -> UriMap<Arc<[OwnedRunPoint]>> {
        let mut out: UriMap<Arc<[OwnedRunPoint]>> = UriMap::default();
        if package_entries.is_empty() {
            return out;
        }
        let shared: HashMap<&str, Arc<[OwnedRunPoint]>> = package_entries
            .iter()
            .map(|(root, sites)| (root.as_str(), Arc::from(sites.as_slice())))
            .collect();
        for &doc in documents {
            if ambiguous.contains(doc) {
                continue;
            }
            let root = root_of.get(doc).map_or(doc, String::as_str);
            if let Some(sites) = shared.get(root) {
                out.insert(doc.to_owned(), Arc::clone(sites));
            }
        }
        out
    }

    /// Legacy explicitly constructed model comparison, without an execution
    /// receipt. Always None for purpose-bound workspace source navigation;
    /// use `navigation_precedence` to retain its conditional relation.
    ///
    /// The gating question — the leniency is
    /// [`tcl_compiler::analyser::indirection::in_effect_within`]'s, applied in
    /// the two points' deepest common document: a statement at the load level
    /// of some document counts as having run for a query inside one of that
    /// document's proc bodies, however far below it is written, because the
    /// whole file loads before any body runs.
    #[must_use]
    pub fn has_run(&self, event: RunPoint<'_>, query: RunPoint<'_>) -> Option<bool> {
        if self.source_contexts.is_some() {
            return None;
        }
        self.precedence_relation(event, query)
    }

    fn precedence_relation(&self, event: RunPoint<'_>, query: RunPoint<'_>) -> Option<bool> {
        self.decide(event, query, |e, q| {
            let ran =
                tcl_compiler::analyser::indirection::in_effect_within(e.at, q.at, q.enclosing_body);
            Self::trusted(ran, e, q).then_some(ran)
        })
    }

    /// Legacy explicitly constructed model ranking. Always None for
    /// purpose-bound workspace source navigation; `navigation_comparison`
    /// retains conditional coordinates without conflict/completion authority.
    ///
    /// The ranking question the latest-wins folds and the import-conflict rule
    /// need.  Deliberately *not* [`Self::has_run`]'s leniency: a conflict
    /// cancels an install, so counting a maybe-never-run body statement as
    /// having run would invent conflicts and drop aliases the program really
    /// has — the reasoning on [`crate::namespace_import::load_order`].
    #[must_use]
    pub fn cmp_run(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<std::cmp::Ordering> {
        if self.source_contexts.is_some() {
            return None;
        }
        self.comparison_relation(a, b)
    }

    fn comparison_relation(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<std::cmp::Ordering> {
        use crate::namespace_import::load_order;
        use std::cmp::Ordering;
        self.decide(a, b, |pa, pb| {
            let ord = load_order(pa.at, pa.enclosing_body.is_some())
                .cmp(&load_order(pb.at, pb.enclosing_body.is_some()));
            let usable = match ord {
                // `Equal` claims both directions at once, so it needs both.
                Ordering::Equal => pa.exact && pb.exact,
                other => Self::trusted(other == Ordering::Less, pa, pb),
            };
            usable.then_some(ord)
        })
    }

    /// Whether a comparison that came out as `a_first` is a **fact** given how
    /// each side was placed.
    ///
    /// A [`Placed`] that is not [`exact`](Placed::exact) is an *upper bound*:
    /// the statement ran at or before that offset, never after it (a
    /// `package require` finds an already-loaded package and evaluates
    /// nothing).  So each answer needs one side pinned:
    ///
    /// - "`a` ran first" survives `a` sliding earlier, but not `b` sliding
    ///   earlier — it needs `b` exact;
    /// - "`a` did not run first" survives `b` sliding earlier, but not `a`
    ///   sliding earlier — it needs `a` exact.
    ///
    /// Two bounded sides settle nothing, which is why two documents that a
    /// `package require` each brought in are not ranked against each other at
    /// all.
    #[must_use]
    pub fn trusted(a_first: bool, a: Placed, b: Placed) -> bool {
        if a_first { b.exact } else { a.exact }
    }

    /// Put the two points in one document and let `judge` read the pair, over
    /// every placement that could settle the question — the shared body of
    /// [`Self::has_run`] and [`Self::cmp_run`], so both see the same
    /// placements and can only differ in the rule they apply to them.
    ///
    /// The **direct** frame — both points lifted onto the `source` forest and
    /// met at their deepest common document — is the whole answer whenever it
    /// exists, and it is exact on both sides.  It is also the only placement
    /// considered then: a document reachable directly is not also reached
    /// through a `package require` bound (`build` marks such a document
    /// ambiguous), so there is nothing to disagree with.
    ///
    /// Otherwise, a point whose tree the workspace enters by `package require`
    /// is **substituted** by each require site that loads it: the site is a
    /// position by which that whole tree had certainly run, so it stands in for
    /// the point with [`Placed::exact`] cleared.  Only one side is ever
    /// substituted — two bounded sides can settle nothing anyway
    /// ([`Self::trusted`]), and two trees that each require the other would let
    /// one placement's `Less` sit beside another's `Greater`.
    ///
    /// The first placement `judge` accepts wins, and that is not a shortcut:
    /// with one side bounded, `trusted` admits exactly one of the two answers,
    /// so no two placements can accept opposite ones.
    fn decide<T>(
        &self,
        a: RunPoint<'_>,
        b: RunPoint<'_>,
        judge: impl Fn(Placed, Placed) -> Option<T>,
    ) -> Option<T> {
        // A URI-level package bound cannot replace a refused original point.
        // Constructed inputs retain original_point's identity behavior.
        let a = self.original_point(a)?;
        let b = self.original_point(b)?;
        if let Some((x, y)) = self.common_frame(a, b) {
            return judge(x, y);
        }
        let a_alts = self.alternatives(a);
        let b_alts = self.alternatives(b);
        if !a_alts.is_empty() && !b_alts.is_empty() {
            return None;
        }
        let blur = |p: Placed| Placed { exact: false, ..p };
        for site in a_alts {
            if let Some((x, y)) = self.common_frame(site.as_point(), b)
                && let Some(answer) = judge(blur(x), y)
            {
                return Some(answer);
            }
        }
        for site in b_alts {
            if let Some((x, y)) = self.common_frame(a, site.as_point())
                && let Some(answer) = judge(x, blur(y))
            {
                return Some(answer);
            }
        }
        None
    }

    /// The `package require` sites that load the tree `p` sits in, if the
    /// workspace enters that tree by `package require` at all.
    ///
    /// A point's tree is identified by its root: a file the provider `source`s
    /// runs when the provider does, so it is bounded by the same require
    /// sites.
    /// One probe: which document a point sits in is the only thing left to
    /// look up, because the root walk and the ambiguity test do not depend on
    /// the point's offset and are folded into
    /// [`Self::package_alternatives`] at build time.
    pub(super) fn alternatives(&self, p: RunPoint<'_>) -> &[OwnedRunPoint] {
        self.package_alternatives
            .get(p.uri)
            .map_or(&[], |sites| &**sites)
    }

    fn original_point<'a>(&self, point: RunPoint<'a>) -> Option<RunPoint<'a>> {
        let Some(contexts) = &self.source_contexts else {
            return Some(point);
        };
        contexts.get(point.uri)?.ordered_point(point)
    }

    /// [`Self::original_point`] admitting only a point an issued site owns: a
    /// comparison cancels an install Tcl performs, so it needs that site.
    fn site_point<'a>(&self, point: RunPoint<'a>) -> Option<RunPoint<'a>> {
        let Some(contexts) = &self.source_contexts else {
            return Some(point);
        };
        let site = contexts.get(point.uri)?.site(point.at)?;
        Some(RunPoint {
            enclosing_body: site.body,
            ..point
        })
    }

    /// Both points as positions in their deepest common document — the single
    /// projection [`Self::has_run`] and [`Self::cmp_run`] share, so the gate
    /// and the conflict rule read one relation and differ only in which
    /// single-document rule they then apply.
    fn common_frame(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<(Placed, Placed)> {
        let a = self.original_point(a)?;
        let b = self.original_point(b)?;
        self.common_original_frame(a, b)
    }

    fn common_original_frame(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<(Placed, Placed)> {
        let own = |p: RunPoint<'_>| Placed {
            at: p.at,
            enclosing_body: p.enclosing_body,
            exact: true,
        };
        // Same document: already ordered, and ordered without the graph — a
        // re-sourced or cyclic file still sequences its own statements.
        if a.uri == b.uri {
            return Some((own(a), own(b)));
        }
        if self.ambiguous.contains(a.uri) || self.ambiguous.contains(b.uri) {
            return None;
        }
        // Different trees have no order at all — the common case, and exactly
        // the pre-source-graph behaviour.
        if self.root_of.get(a.uri)? != self.root_of.get(b.uri)? {
            return None;
        }
        let path_a = self.paths.get(a.uri)?;
        let path_b = self.paths.get(b.uri)?;
        for depth in 0..path_a.len().min(path_b.len()) {
            if path_a[depth] != path_b[depth] {
                return Some((path_a[depth], path_b[depth]));
            }
        }
        // One path is a prefix of the other: the shallower point's own
        // document holds the `source` statement that entered the deeper one's
        // branch, so that statement is the deeper point's position there.
        Some(match path_a.len().cmp(&path_b.len()) {
            std::cmp::Ordering::Less => (own(a), path_b[path_a.len()]),
            std::cmp::Ordering::Greater => (path_a[path_b.len()], own(b)),
            // Equal-length shared paths with different URIs cannot happen: a
            // path entry identifies the `source` statement, which identifies
            // the document it entered.
            std::cmp::Ordering::Equal => return None,
        })
    }
}
