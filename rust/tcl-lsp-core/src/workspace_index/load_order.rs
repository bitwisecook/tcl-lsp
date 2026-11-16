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

//! What each document loads: `source` edges and the host resolver, package declarations, the run order and imported path constants.

use super::records::{
    WorkspacePackageIfneeded, WorkspacePackagePrefer, WorkspacePackageProvide,
    WorkspacePackageRequire, WorkspaceSource,
};
use super::{ConstantFolder, Derived, SourceResolver, WorkspaceIndex};
use std::collections::HashMap;
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;

impl WorkspaceIndex {
    /// Install the host's retained-source-expression → document-URI resolver, so
    /// the index can build the [`crate::source_graph::RunOrder`] the
    /// import-lifecycle gates rank cross-document events with.
    ///
    /// The index deliberately holds no URI ↔ filesystem-path mapping of its
    /// own — that is the host's knowledge, and every other `source`-graph
    /// consumer reads the same selected candidate edges
    /// ([`Self::source_ancestor_package_requires_from_resolved`],
    /// [`Self::source_ancestor_prefers_latest_from_resolved`], [`Self::source_seed_map`]).
    /// The order, though, is consulted *inside* the per-call import walk,
    /// where a per-call resolver argument would have to be threaded through
    /// every caller of `resolve_wildcard_import` and every derived view that
    /// builds a `WildcardImportIndex`. Holding it here instead lets the order
    /// be a [`Derived`] view like the rest, built at most once per
    /// [`Self::generation`].
    ///
    /// A plain `fn` pointer rather than a boxed closure: the resolver is a
    /// pure function of the explicit filename and retained source inputs, and
    /// a `fn` keeps the index `Debug + Clone + Default` with no manual impls.
    /// The signature is [`Self::source_seed_map`]'s, so one host resolver
    /// serves both — including its statically-foldable computed-path tier
    /// (`[file join [file dirname [info script]] x.tcl]`), which is as provable
    /// as a literal and is the idiom real multi-file projects actually write.
    /// A path the host cannot prove returns `None` and sequences nothing.
    ///
    /// **Without a resolver the order is empty**: every cross-document event
    /// is then unrankable and every same-document one ordered.
    pub fn set_source_resolver(&mut self, resolve: SourceResolver, fold: ConstantFolder) {
        self.source_resolver = Some(resolve);
        self.constant_folder = Some(fold);
        self.run_order = Derived::default();
        self.imported_constants = Derived::default();
        // The export snapshot captures the order, so it goes with it.
        self.export_snapshot = Derived::default();
    }

    /// Candidate source edges using the same genuine expression plans and
    /// retained inventories as import folding, namespace seeds and run order.
    /// These are conditional source routes, not completed file evaluation.
    #[must_use]
    pub fn resolved_source_edges(&self) -> Vec<crate::source_graph::RunEdge> {
        let imports = self.imported_constants();
        let empty = tcl_compiler::auto_path_eval::FoldedPathConstants::default();
        self.source_resolver
            .into_iter()
            .flat_map(|resolve| {
                self.sources()
                    .filter_map(|source| {
                        resolve(
                            &source.uri,
                            &source.raw_path,
                            source.original_path_expression.as_ref(),
                            source.range.start(),
                            self.path_constant_assignments(&source.uri),
                            imports.get(&source.uri).unwrap_or(&empty),
                        )
                        .map(|child| crate::source_graph::RunEdge {
                            parent: source.uri.clone(),
                            child,
                            at: source.range.start(),
                            enclosing_body: source.enclosing_body,
                            kind: crate::source_graph::RunEdgeKind::Source,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Package names available through the installed conditional source edge
    /// resolver. No raw reported path or dialect label selects an operation.
    #[must_use]
    pub fn source_ancestor_package_requires_from_resolved(&self, target_uri: &str) -> Vec<String> {
        let edges = self
            .resolved_source_edges()
            .into_iter()
            .map(|edge| (edge.parent, edge.child))
            .collect::<Vec<_>>();
        let mut requires = std::collections::HashMap::<String, Vec<String>>::new();
        for required in self.package_requires() {
            requires
                .entry(required.uri.clone())
                .or_default()
                .push(required.name.clone());
        }
        crate::source_graph::ancestor_requires(target_uri, &edges, &requires)
    }

    /// Source-order package preference advice over the installed, selected
    /// source edges, independently of actual interpreter package state.
    #[must_use]
    pub fn source_ancestor_prefers_latest_from_resolved(&self, target_uri: &str) -> bool {
        if self.package_prefers().next().is_none() {
            return false;
        }
        self.source_ancestor_prefers_latest_for_edges(target_uri, &self.resolved_source_edges())
    }

    fn source_ancestor_prefers_latest_for_edges(
        &self,
        target_uri: &str,
        edges: &[crate::source_graph::RunEdge],
    ) -> bool {
        let mut raises = std::collections::HashMap::<String, Vec<u32>>::new();
        for prefer in self.package_prefers() {
            raises
                .entry(prefer.uri.clone())
                .or_default()
                .push(prefer.at);
        }
        crate::source_graph::ancestor_prefer_latest_raised(target_uri, edges, &raises)
    }

    /// Conditional source navigation order, retaining the exact original
    /// source/body/schema/input owners and no executed event completion.
    /// The existing forest is built at most once per
    /// [`Self::generation`] from the `source` graph
    /// ([`Self::set_source_resolver`]) **and** the `package require` graph
    /// ([`Self::package_run_edges`]).
    ///
    /// The `source` half is empty when no resolver is installed or when no
    /// `source` statement resolves: a target the resolver cannot place —
    /// `source $dir/x.tcl` with `$dir` unknown — names no document statically
    /// and sequences nothing.  The package half needs **no** resolver: a
    /// `package require NAME` names its provider through the index's own
    /// `package provide` records, so it works on a host that installs none.
    pub(super) fn run_order(&self) -> Arc<crate::source_graph::RunOrder> {
        self.run_order.get_or_build(|| {
            let source_edges = self.resolved_source_edges().into_iter();
            let edges: Vec<crate::source_graph::RunEdge> =
                source_edges.chain(self.package_run_edges()).collect();
            crate::source_graph::RunOrder::for_source_navigation(
                &edges,
                self.docs
                    .iter()
                    .filter_map(|doc| Some((doc.uri.clone(), doc.source_order_context.clone()?))),
            )
        })
    }

    /// Per-document **imported** path constants: the values a
    /// document's source-graph ancestors establish before it runs, agreed
    /// across every route that reaches it.
    ///
    /// Built as a joint fixpoint of edges and imports, in whole rounds so a
    /// shipped answer is always consistent with the shipped map:
    ///
    /// 1. Resolve **every** `source` row against the current imports (round
    ///    one: none) — no per-row caching across rounds, because a later
    ///    round can legitimately *shrink* an import (a newly resolved
    ///    in-edge whose parent does not supply a name drops that name from
    ///    the agreement), and a cached resolution from a richer map would
    ///    then be an answer the final map cannot reproduce.
    /// 2. From the resolved edges, compute what each parent **provides** to
    ///    each child: the parent's own writes *positioned before the
    ///    `source` statement* (a later assignment has not happened yet when
    ///    the child runs), folded by the host with the parent's own imports
    ///    — which precede the parent's whole execution, so they carry
    ///    unconditionally.
    /// 3. A child imports a name only when **every** in-edge supplies it
    ///    with an identical value — the agreement rule that carries OSVVM,
    ///    whose `StartUpShared.tcl` is sourced by eleven simulator scripts
    ///    each assigning `::osvvm::OsvvmScriptDirectory` the same directory.
    ///    One dissenting or silent route kills the name: whichever parent
    ///    actually ran, a kept value is the value.  Step 2-then-3 repeats to
    ///    its own fixpoint over the round's **fixed** edge set, where
    ///    provided maps only grow and agreement over a fixed edge set is
    ///    monotone.
    ///
    /// Rounds repeat until the map is stable (edges resolved under map M
    /// reproduce M), almost always round two.  A workspace that has not
    /// converged by the cap — constructible only from pathological
    /// import-unlocked cycles — ships **no** imports rather than a map its
    /// own edges disagree with.
    pub(super) fn imported_constants(
        &self,
    ) -> Arc<HashMap<String, tcl_compiler::auto_path_eval::FoldedPathConstants>> {
        const ROUND_CAP: usize = 5;
        self.imported_constants.get_or_build(|| {
            let (Some(resolve), Some(fold)) = (self.source_resolver, self.constant_folder) else {
                return HashMap::new();
            };
            let empty = tcl_compiler::auto_path_eval::FoldedPathConstants::default();
            let rows: Vec<&WorkspaceSource> = self.sources().collect();
            let mut imports: HashMap<String, tcl_compiler::auto_path_eval::FoldedPathConstants> =
                HashMap::new();
            for _round in 0..ROUND_CAP {
                // Step 1: the round's edge set, from scratch.
                let edges: Vec<(&WorkspaceSource, String)> = rows
                    .iter()
                    .filter_map(|s| {
                        let child = resolve(
                            &s.uri,
                            &s.raw_path,
                            s.original_path_expression.as_ref(),
                            s.range.start(),
                            self.path_constant_assignments(&s.uri),
                            imports.get(&s.uri).unwrap_or(&empty),
                        )?;
                        // Self-sourcing provides nothing new.
                        (child != s.uri).then_some((*s, child))
                    })
                    .collect();
                // Steps 2 + 3 to their own fixpoint over this fixed edge set.
                let mut next: HashMap<String, tcl_compiler::auto_path_eval::FoldedPathConstants> =
                    HashMap::new();
                for _inner in 0..ROUND_CAP {
                    let mut candidates: HashMap<
                        &str,
                        Vec<tcl_compiler::auto_path_eval::FoldedPathConstants>,
                    > = HashMap::new();
                    for (s, child) in &edges {
                        // A `source` inside a proc or method body runs when
                        // that body is *called*, not at its lexical position
                        // — `set dir /a; proc load {} {source child.tcl};
                        // set dir /b; load` reaches the child with `/b`.
                        // The position gate below reads lexical order, so a
                        // body-local route instead contributes an **empty**
                        // candidate: it still reaches the child, and
                        // intersecting the agreement with an empty map
                        // correctly drops every name for it.
                        if s.enclosing_body.is_some() {
                            candidates.entry(child.as_str()).or_default().push(
                                tcl_compiler::auto_path_eval::FoldedPathConstants::empty(
                                    self.path_constant_assignments(&s.uri).naming_policy(),
                                ),
                            );
                            continue;
                        }
                        let before = self
                            .path_constant_assignments(&s.uri)
                            .before(s.range.start());
                        // From `next` only — never the previous round's map:
                        // the inner iteration is a fixpoint **from below**
                        // over this round's fixed edge set, and seeding it
                        // with values the current agreement may drop would
                        // break exactly the monotonicity that makes it
                        // terminate.
                        let parent_imports = next.get(&s.uri).unwrap_or(&empty);
                        candidates.entry(child.as_str()).or_default().push(fold(
                            &s.uri,
                            &before,
                            parent_imports,
                        ));
                    }
                    let stepped: HashMap<
                        String,
                        tcl_compiler::auto_path_eval::FoldedPathConstants,
                    > = candidates
                        .into_iter()
                        .map(|(child, maps)| {
                            (
                                child.to_owned(),
                                tcl_compiler::auto_path_eval::FoldedPathConstants::agreement(&maps),
                            )
                        })
                        .collect();
                    if stepped == next {
                        break;
                    }
                    next = stepped;
                }
                if next == imports {
                    return imports;
                }
                imports = next;
            }
            // Cap hit without convergence: conservative empty map.
            HashMap::new()
        })
    }

    /// The imported constants for one document — the map
    /// [`SourceResolver`] receives as its sixth argument, exposed so hosts
    /// can hand the very same view to their other per-document consumers
    /// (document links, a fresh analysis's own edges).
    #[must_use]
    pub fn imported_path_constants_for(
        &self,
        uri: &str,
    ) -> tcl_compiler::auto_path_eval::FoldedPathConstants {
        self.imported_constants()
            .get(uri)
            .cloned()
            .unwrap_or_default()
    }

    /// The `package require` half of the load order: one
    /// [`crate::source_graph::RunEdgeKind::PackageRequire`] edge per require
    /// site whose package this workspace provably provides (design §3.4).
    ///
    /// A `package require NAME` that returns has left `NAME` loaded, so the
    /// providing document's statements have all run — even though the require
    /// itself may have evaluated nothing.  That is a genuine ordering fact and
    /// the only one available on package-structured code, where the `source`
    /// statements that would sequence the same files are written
    /// `source [file join $dir x.tcl]` inside a `pkgIndex.tcl` and fold to
    /// nothing.
    ///
    /// # The soundness gates
    ///
    /// Each drops a require site rather than guessing, and each corresponds to
    /// one of the three uncertainties the design records:
    ///
    /// 1. **`auto_path` is mutable** (design §3.4 / uncertainty 1) — the
    ///    provider is identified by `package provide`, and the search path
    ///    that decides *which* copy of a package wins is a runtime value.  So
    ///    a package **two** indexed documents provide yields no edge at all:
    ///    with two directories on `auto_path`, their order alone decides which
    ///    file runs (oracle: tclsh 8.6.14 / 9.0.4, two directories each
    ///    holding a `mylib2 1.0`, `::lib::p` answers `from A` or `from B`
    ///    purely by `lappend auto_path` order — and only one of them exports).
    /// 2. **A package may already be loaded** (uncertainty 2) — handled by the
    ///    *edge kind*, not by dropping the edge: see
    ///    [`crate::source_graph::RunEdgeKind::PackageRequire`].
    /// 3. **`ifneeded` bodies are arbitrary** (uncertainty 3) — a package this
    ///    workspace registers a `package ifneeded` script for yields no edge.
    ///    The script is what actually runs, it runs later and in the global
    ///    namespace, and it may load something else entirely (oracle: one
    ///    `pkgIndex.tcl` whose body branches on the environment gives
    ///    `::c::p` → `real` or `stub`, with and without the provider's
    ///    `namespace export`, for the same `package require`).
    ///
    /// Plus the gates the `source` half already applies in spirit: a
    /// **conditional** require *or provide* may never run (the shim idiom
    /// [`tcl_compiler::analyser::types::PackageProvide::conditional`]
    /// documents); a **non-literal** package name
    /// (`package require $pkg`) names nothing statically; and a document that
    /// requires the package it itself provides is a self-edge, which
    /// [`crate::source_graph::RunOrder::build`] discards.
    pub(super) fn package_run_edges(&self) -> Vec<crate::source_graph::RunEdge> {
        use std::collections::hash_map::Entry;
        // name -> the single indexed provider, or `None` once a second one
        // has been seen (gate 1).
        let mut provider: std::collections::HashMap<
            &tcl_registry::native_package::NativePackageNameKey,
            Option<&str>,
        > = std::collections::HashMap::new();
        if self
            .package_provides()
            .any(|provide| !provide.conditional && provide.original_name.is_none())
        {
            return Vec::new();
        }
        for pp in self.package_provides().filter(|pp| !pp.conditional) {
            let Some(name) = pp.original_name.as_ref() else {
                continue;
            };
            match provider.entry(name) {
                Entry::Vacant(slot) => {
                    slot.insert(Some(pp.uri.as_str()));
                }
                Entry::Occupied(mut slot) => {
                    if *slot.get() != Some(pp.uri.as_str()) {
                        slot.insert(None);
                    }
                }
            }
        }
        // Gate 3: a registered load script owns the package's loading.
        for pi in self.package_ifneededs() {
            if let Some(name) = pi.original_name.as_ref() {
                provider.insert(name, None);
            } else {
                return Vec::new();
            }
        }
        self.package_requires()
            .filter(|pr| !pr.conditional)
            .filter_map(|pr| {
                let child = (*provider.get(pr.original_name.as_ref()?)?)?;
                Some(crate::source_graph::RunEdge {
                    parent: pr.uri.clone(),
                    child: child.to_owned(),
                    at: pr.at,
                    enclosing_body: pr.enclosing_body,
                    kind: crate::source_graph::RunEdgeKind::PackageRequire,
                })
            })
            .collect()
    }

    /// Every indexed `source FILE` reference.
    pub fn sources(&self) -> impl Iterator<Item = &WorkspaceSource> {
        self.docs.iter().flat_map(|doc| doc.sources.iter())
    }

    /// Current source-action input from an actual indexed row. Reporting path
    /// text/ranges cannot replace the retained FILE operand and source owner.
    #[must_use]
    pub fn source_file_reference(
        &self,
        source: &WorkspaceSource,
    ) -> Option<(
        u64,
        &tcl_compiler::auto_path_eval::OriginalSourcePathExpression,
    )> {
        let document = self.docs.get(*self.slots.get(&source.uri)?)?;
        let source = document
            .sources
            .iter()
            .find(|row| std::ptr::eq(*row, source))?;
        let original = source.original_path_expression.as_ref()?;
        original.file_reference_value()?;
        original
            .matches_assignments(&document.path_constant_assignments)
            .then_some(())?;
        Some((document.revision, original))
    }

    /// Join a proposed host file action to the same indexed source generation
    /// and independently obtained current analysis before publishing edits.
    #[must_use]
    pub fn source_file_reference_is_current(
        &self,
        uri: &str,
        revision: u64,
        original: &tcl_compiler::auto_path_eval::OriginalSourcePathExpression,
        analysis: &AnalysisResult,
    ) -> bool {
        self.document_revision(uri) == Some(revision)
            && original.matches_analysis(analysis)
            && self.sources().any(|row| {
                row.uri == uri
                    && row.original_path_expression.as_ref() == Some(original)
                    && self
                        .source_file_reference(row)
                        .is_some_and(|(held, _)| held == revision)
            })
    }

    /// `uri`'s raw single-assignment path-constant candidates
    /// ([`AnalysisResult::path_constant_assignments`], carried per document);
    /// empty for an unindexed URI.  The source-edge resolver chain-folds
    /// these with the parent's own path before resolving a computed `source`
    /// argument.
    #[must_use]
    pub fn path_constant_assignments(
        &self,
        uri: &str,
    ) -> &tcl_compiler::auto_path_eval::PathConstantAssignments {
        self.slots.get(uri).map_or(
            tcl_compiler::auto_path_eval::PathConstantAssignments::unknown(),
            |&slot| &self.docs[slot].path_constant_assignments,
        )
    }

    /// Every indexed `package require NAME` declaration.
    pub fn package_requires(&self) -> impl Iterator<Item = &WorkspacePackageRequire> {
        self.docs.iter().flat_map(|doc| doc.package_requires.iter())
    }

    /// Every indexed `package provide NAME` declaration.
    pub fn package_provides(&self) -> impl Iterator<Item = &WorkspacePackageProvide> {
        self.docs.iter().flat_map(|doc| doc.package_provides.iter())
    }

    /// Every indexed `package ifneeded NAME VERSION SCRIPT` registration.
    pub fn package_ifneededs(&self) -> impl Iterator<Item = &WorkspacePackageIfneeded> {
        self.docs
            .iter()
            .flat_map(|doc| doc.package_ifneededs.iter())
    }

    /// Every indexed unconditional `package prefer latest`.
    pub fn package_prefers(&self) -> impl Iterator<Item = &WorkspacePackagePrefer> {
        self.docs.iter().flat_map(|doc| doc.package_prefers.iter())
    }

    /// The package names `uri` `package require`s, de-duplicated. Used to seed
    /// the workspace W120 refinement from an explicitly configured project
    /// entry file.
    #[must_use]
    pub fn package_requires_for(&self, uri: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .package_requires()
            .filter(|pr| pr.uri == uri)
            .map(|pr| pr.name.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// Original package keys retained in a document's source inventory.
    /// This metadata grants no completed require or package loading.
    #[must_use]
    pub fn original_package_requires_for(
        &self,
        uri: &str,
    ) -> Vec<tcl_registry::native_package::NativePackageNameKey> {
        let mut keys = Vec::new();
        for required in self
            .package_requires()
            .filter(|required| required.uri == uri)
        {
            if let Some(key) = required.original_name.as_ref()
                && !keys.contains(key)
            {
                keys.push(key.clone());
            }
        }
        keys
    }

    /// Complete requirement advice retaining independently supplied source
    /// identity, constraints and preference. This proves no loader executed.
    #[must_use]
    pub fn original_package_requirement_advice_for(
        &self,
        uri: &str,
        default: crate::package_resolver::PackagePrefer,
    ) -> Vec<crate::package_resolver::PackageRequirementAdvice> {
        self.package_requires()
            .filter(|required| required.uri == uri)
            .map(|required| required.original_advice(default))
            .collect()
    }

    /// Advisory source ancestors preserving their original package keys.
    /// Graph identity uses document URIs; Tcl name geometry stays in the key.
    #[must_use]
    pub fn source_ancestor_original_package_requires(
        &self,
        target_uri: &str,
        resolve: impl Fn(&str, &str) -> Option<String>,
    ) -> Vec<tcl_registry::native_package::NativePackageNameKey> {
        let edges = self
            .sources()
            .filter(|source| source.is_literal)
            .filter_map(|source| {
                resolve(&source.uri, &source.raw_path).map(|child| (source.uri.clone(), child))
            })
            .collect::<Vec<_>>();
        let mut requirements: std::collections::HashMap<String, Vec<_>> =
            std::collections::HashMap::new();
        for required in self.package_requires() {
            if let Some(key) = required.original_name.as_ref() {
                requirements
                    .entry(required.uri.clone())
                    .or_default()
                    .push(key.clone());
            }
        }
        crate::source_graph::ancestor_requirements(target_uri, &edges, &requirements)
    }

    /// The union of `package require` names from every document that
    /// transitively `source`s `target_uri`.
    ///
    /// `resolve(parent_uri, raw_path)` maps a literal `source` path written in
    /// `parent_uri` to the child document's URI (the server supplies the
    /// URI ↔ path conversion); a `None` return drops that unresolvable edge.
    /// Only literal `source` targets are followed — a `source $dir/x.tcl` whose
    /// path is computed at runtime cannot be resolved statically.  The
    /// reachability walk and requires union live in
    /// [`crate::source_graph::ancestor_requires`].
    #[must_use]
    pub fn source_ancestor_package_requires(
        &self,
        target_uri: &str,
        resolve: impl Fn(&str, &str) -> Option<String>,
    ) -> Vec<String> {
        let edges: Vec<(String, String)> = self
            .sources()
            .filter(|s| s.is_literal)
            .filter_map(|s| resolve(&s.uri, &s.raw_path).map(|child| (s.uri.clone(), child)))
            .collect();
        let mut requires: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for pr in self.package_requires() {
            requires
                .entry(pr.uri.clone())
                .or_default()
                .push(pr.name.clone());
        }
        crate::source_graph::ancestor_requires(target_uri, &edges, &requires)
    }

    /// Whether the interpreter-global `package prefer latest` latch is already
    /// raised by the time `target_uri` is loaded, because a document that
    /// (transitively) `source`s it raised it first.
    ///
    /// `resolve` maps a literal `source` path written in a document to the
    /// child document's URI, exactly as for
    /// [`Self::source_ancestor_package_requires`]; a `None` return drops that
    /// edge, and only literal `source` targets are followed.
    ///
    /// The state is interpreter-global, so this is genuinely *this* document's
    /// answer — it just is not a fact about this document's own text.  The
    /// order rule and the two ways the latch can already be up live in
    /// [`crate::source_graph::ancestor_prefer_latest_raised`].
    #[must_use]
    pub fn source_ancestor_prefers_latest(
        &self,
        target_uri: &str,
        resolve: impl Fn(&str, &str) -> Option<String>,
    ) -> bool {
        // Nothing raises the latch anywhere: skip building the graph. This is
        // the overwhelmingly common case (the default is `stable` and most
        // workspaces never write `package prefer` at all), and it keeps a
        // per-`package require` query free.
        if self.package_prefers().next().is_none() {
            return false;
        }
        let edges: Vec<crate::source_graph::RunEdge> = self
            .sources()
            .filter(|s| s.is_literal)
            .filter_map(|s| {
                resolve(&s.uri, &s.raw_path).map(|child| crate::source_graph::RunEdge {
                    parent: s.uri.clone(),
                    child,
                    at: s.range.start(),
                    enclosing_body: s.enclosing_body,
                    kind: crate::source_graph::RunEdgeKind::Source,
                })
            })
            .collect();
        self.source_ancestor_prefers_latest_for_edges(target_uri, &edges)
    }

    /// The **source-site namespace seeds** per sourced document: for
    /// every `source` statement `resolve` can place (the closure maps
    /// `(parent-uri, reported-path, original-expression)` to the child's URI — handling
    /// relative literals and statically-foldable computed
    /// paths), the child URI maps to the set of namespaces it is sourced
    /// under.  `source` runs the file in the caller's namespace, so a child
    /// sourced from `namespace eval ::x` must be (re-)analysed seeded with
    /// `::x` for the index to hold its true runtime names.
    ///
    /// Transitivity needs no composition here: once a child's *seeded*
    /// analysis is merged into the index, its own recorded `source` sites
    /// already carry the composed namespace.
    #[must_use]
    pub fn source_seed_map(
        &self,
        resolve: impl Fn(
            &str,
            &str,
            Option<&tcl_compiler::auto_path_eval::OriginalSourcePathExpression>,
            u32,
            &tcl_compiler::auto_path_eval::PathConstantAssignments,
            &tcl_compiler::auto_path_eval::FoldedPathConstants,
        ) -> Option<String>,
    ) -> std::collections::HashMap<String, std::collections::BTreeSet<String>> {
        let imports = self.imported_constants();
        let empty = tcl_compiler::auto_path_eval::FoldedPathConstants::default();
        let mut out: std::collections::HashMap<String, std::collections::BTreeSet<String>> =
            std::collections::HashMap::new();
        for src in self.sources() {
            let Some(child) = resolve(
                &src.uri,
                &src.raw_path,
                src.original_path_expression.as_ref(),
                src.range.start(),
                self.path_constant_assignments(&src.uri),
                imports.get(&src.uri).unwrap_or(&empty),
            ) else {
                continue;
            };
            // Self-sourcing carries no new namespace view.
            if child == src.uri {
                continue;
            }
            let seed = if src.site_namespace.is_empty() {
                "::".to_owned()
            } else {
                src.site_namespace.clone()
            };
            out.entry(child).or_default().insert(seed);
        }
        out
    }

    /// Whether any `source` statement is indexed at all — the cheap guard
    /// that lets the server skip the re-homing pass entirely for
    /// workspaces that never `source`.
    #[must_use]
    pub fn has_source_edges(&self) -> bool {
        self.sources().next().is_some()
    }
}
