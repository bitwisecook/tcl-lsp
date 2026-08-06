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

//! Workspace `source` graph helpers.
//!
//! A multi-file Tcl project commonly has an *entry* file that runs the
//! `package require`s and then `source`s its individual modules.  Those
//! modules use the required packages' commands without a `package require`
//! of their own, which the single-file W120 check would flag as missing (see
//! [`crate::package_resolver`] and the analyser's
//! `emit_missing_package_require_diagnostics`).
//!
//! To resolve that, the server builds a workspace-wide `source` graph from the
//! `source FILE` statements the analyser records per document, then lets a
//! module inherit the `package require`s of every file that (transitively)
//! `source`s it.  This module owns the two pure, filesystem-free pieces of that
//! resolution: the lexical path resolution ([`resolve_source_target`]) and the
//! reverse-reachability walk ([`ancestor_requires`]).  The server supplies the
//! URI ↔ path conversion, keeping this logic testable without any real files.

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
use std::path::{Component, Path, PathBuf};

/// Resolve a literal `source` path argument written in `parent_file` to the
/// absolute path it refers to.
///
/// A relative `raw_path` resolves against the **directory** of `parent_file`
/// (Tcl's `source` is relative to the sourcing script's location); an absolute
/// `raw_path` passes through unchanged.  The result is lexically normalised —
/// `.` and `..` segments are folded without touching the filesystem, so a
/// symlink is never followed and the call is cheap and deterministic.
#[must_use]
pub fn resolve_source_target(parent_file: &Path, raw_path: &str) -> PathBuf {
    resolve_under(
        parent_file.parent().unwrap_or_else(|| Path::new("")),
        raw_path,
    )
}

/// Resolve `raw_path` relative to the directory `base_dir` (absolute paths pass
/// through), lexically normalised.  Used both for a `source` target (relative
/// to the sourcing file's directory) and for a configured project entry-point
/// path (relative to the workspace folder root).
#[must_use]
pub fn resolve_under(base_dir: &Path, raw_path: &str) -> PathBuf {
    let raw = Path::new(raw_path);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base_dir.join(raw)
    };
    lexically_normalise(&joined)
}

/// Fold `.` and `..` segments lexically (no filesystem access). A leading `..`
/// with nothing to pop is preserved so a path that escapes its root still
/// resolves to a stable, comparable form.
fn lexically_normalise(path: &Path) -> PathBuf {
    let mut out: Vec<Component<'_>> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                _ => out.push(comp),
            },
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return PathBuf::from(".");
    }
    out.iter().collect()
}

/// The union of `package require` names from every node that transitively
/// reaches `target` through the `source` graph.
///
/// `edges` are `(parent, child)` pairs meaning *`parent` `source`s `child`*;
/// `requires` maps a node to the package names it `package require`s.  The walk
/// follows edges in reverse (child → its parents), so `target` inherits the
/// requires of every ancestor that sources it, directly or transitively.  Node
/// identity is the caller's string key (the server uses document URIs); cycles
/// are handled by a visited set.  The result is sorted and de-duplicated.
#[must_use]
pub fn ancestor_requires<S: BuildHasher>(
    target: &str,
    edges: &[(String, String)],
    requires: &HashMap<String, Vec<String>, S>,
) -> Vec<String> {
    // child -> parents that source it.
    let mut parents_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for (parent, child) in edges {
        parents_of
            .entry(child.as_str())
            .or_default()
            .push(parent.as_str());
    }
    let mut ancestors: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = vec![target];
    let mut visited: HashSet<&str> = HashSet::new();
    while let Some(node) = stack.pop() {
        if !visited.insert(node) {
            continue;
        }
        if let Some(parents) = parents_of.get(node) {
            for &parent in parents {
                ancestors.insert(parent);
                stack.push(parent);
            }
        }
    }
    let mut out: Vec<String> = ancestors
        .iter()
        .filter_map(|a| requires.get(*a))
        .flatten()
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

/// One `source` edge, with the execution-order facts a state question needs.
///
/// [`ancestor_requires`] only asks *whether* one file reaches another, so a
/// bare `(parent, child)` pair is enough for it.  An **interpreter-state**
/// question — "had this statement already run by the time the child loaded?"
/// — additionally needs where in the parent the `source` sits, because a
/// statement written after it has not run yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEdge {
    /// The sourcing document's key.
    pub parent: String,
    /// The sourced document's key.
    pub child: String,
    /// Byte offset of the `source` statement in `parent`.
    pub at: u32,
    /// Span of the innermost proc/class body containing the `source`
    /// statement in `parent`; `None` at load level.
    pub enclosing_body: Option<tcl_lexer::Span>,
}

/// Whether the **interpreter-global** `package prefer latest` latch is already
/// raised by the time `target` is loaded, given the raises recorded per
/// document (issue #1253).
///
/// `package prefer` is a monotone latch on interpreter state, so a raise in a
/// file that runs first really does change a later file's version selection.
/// Which file runs first is not knowable in general — but along the `source`
/// graph it is a static fact: `source CHILD` loads the whole child *at that
/// statement*, so everything the parent already ran is in effect for it, and
/// nothing the parent runs afterwards is.
///
/// `raises[node]` holds the offsets of that document's **unconditional**
/// `package prefer latest` statements (a conditional one may not run, and the
/// caller abstains toward the interpreter default by omitting it).
///
/// Two ways the latch is up when a node is entered:
///
/// * a parent raised it **before** the `source` statement — order-gated with
///   [`tcl_compiler::analyser::indirection::in_effect_within`], so a raise at
///   the parent's load level still counts for a `source` written inside one of
///   its proc bodies (the whole file loads before any body runs);
/// * a parent was **itself** entered with the latch up, in which case it was
///   up for the parent's whole execution and so for every file it sources.
///
/// Cycles terminate on the visited set.  `target`'s *own* raises are not
/// consulted — those are the single-document question
/// (`package_resolver::package_prefer_at`), which is position-sensitive within
/// the document.
#[must_use]
pub fn ancestor_prefer_latest_raised<S: BuildHasher>(
    target: &str,
    edges: &[SourceEdge],
    raises: &HashMap<String, Vec<u32>, S>,
) -> bool {
    use tcl_compiler::analyser::indirection::in_effect_within;

    let no_raises: Vec<u32> = Vec::new();
    let raised_before = |node: &str, at: u32, body: Option<tcl_lexer::Span>| {
        raises
            .get(node)
            .unwrap_or(&no_raises)
            .iter()
            .any(|&established| in_effect_within(established, at, body))
    };
    // Seed: every child whose parent had already raised the latch when it
    // sourced them.
    let mut entered_raised: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = Vec::new();
    for edge in edges {
        if raised_before(&edge.parent, edge.at, edge.enclosing_body)
            && entered_raised.insert(edge.child.as_str())
        {
            stack.push(edge.child.as_str());
        }
    }
    // Propagate: a node entered with the latch up passes it to *every* file it
    // sources, wherever the `source` sits — the latch was already up for the
    // node's whole execution.
    while let Some(node) = stack.pop() {
        if node == target {
            return true;
        }
        for edge in edges.iter().filter(|e| e.parent == node) {
            if entered_raised.insert(edge.child.as_str()) {
                stack.push(edge.child.as_str());
            }
        }
    }
    entered_raised.contains(target)
}

/// One statement's position in the workspace's **execution timeline**: the
/// document it is written in, its byte offset there, and the innermost
/// proc/class body containing it (`None` at load level).
///
/// The three facts every import-lifecycle event and every query point already
/// carries, named once so [`RunOrder`] can take a pair of them rather than six
/// loose arguments.  Within one document the triple is exactly what
/// [`tcl_compiler::analyser::indirection::in_effect_within`] and
/// [`crate::namespace_import::load_order`] read; across documents it is what
/// [`RunOrder`] lifts onto the `source` forest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunPoint<'a> {
    /// Document the statement is written in.
    pub uri: &'a str,
    /// Byte offset of the statement within `uri`.
    pub at: u32,
    /// Span of the innermost proc/class body containing it; `None` at load
    /// level.
    pub enclosing_body: Option<tcl_lexer::Span>,
}

/// A position inside **one** document — what [`RunOrder`] reduces a pair of
/// [`RunPoint`]s to once it has found their deepest common document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// Byte offset within the common document.
    pub at: u32,
    /// Enclosing proc/class body of that offset, in the same document.
    pub enclosing_body: Option<tcl_lexer::Span>,
}

/// The **load order the `source` graph proves** — the single relation both
/// wildcard-import tiers rank cross-document events with (issue #1104 item 3,
/// #1116 item 6; design in `docs/design/import-order-source-graph.md` §6).
///
/// # Why an order exists at all
///
/// Nothing in a Tcl source tree says which of two files runs first, which is
/// why every cross-document import-lifecycle event was passed *unordered* and
/// the shared decision functions abstained toward answering: a foreign
/// `namespace export` counted, a foreign `namespace export -clear` revoked
/// nothing, a foreign `namespace forget` revoked nothing.  A `source`
/// statement is the exception — it is an ordinary statement of the sourcing
/// file that runs the whole sourced file *at that point*, so the DFS of the
/// `source` forest **is** the run order, and two statements in one tree are
/// genuinely comparable.
///
/// # The relation
///
/// Each point is lifted to its root-ward key: the offsets of the `source`
/// statements that entered each document down the path, then the point's own
/// offset.  Two keys are compared element-wise; the first index where they
/// differ is their **deepest common document**, and there the existing
/// single-document rules apply unchanged —
/// [`tcl_compiler::analyser::indirection::in_effect_within`] for
/// [`Self::has_run`], [`crate::namespace_import::load_order`] for
/// [`Self::cmp_run`].  One relation, both questions, so the gate and the
/// conflict rule cannot drift (design §6.1: a `has_run` predicate alone is not
/// enough — the folds rank events against *each other*, and a partial order
/// has no `max`).
///
/// # Where it abstains, deliberately
///
/// `None` — incomparable — everywhere the order is not a *fact*, which the
/// callers fold to the same "abstain toward answering" direction `at: None`
/// took before:
///
/// - the two documents sit in **different trees** (no `source` path joins
///   them) — the overwhelmingly common case, and exactly today's behaviour;
/// - either document is **ambiguous**: reachable by two different `source`
///   statements, or on a `source` cycle.  Tcl tolerates re-sourcing a file, so
///   a document with two entry points has no unique position and must not be
///   given one;
/// - either document is unknown to the graph.
///
/// Two points in the **same** document never consult the graph at all: they
/// were already ordered, by the same rule, before this type existed.  A
/// workspace with no resolvable `source` edge therefore behaves byte-identically
/// to the pre-#1104-item-3 tiers.
///
/// Only *literal*, resolvable `source` targets become edges — the caller
/// filters those — because `source $dir/x.tcl` sequences nothing.  A `source`
/// written inside a proc body is kept and judged by `in_effect_within` like
/// any other body-local statement, the same leniency
/// [`ancestor_prefer_latest_raised`] already applies to the same edges.
#[derive(Debug, Clone, Default)]
pub struct RunOrder {
    /// Root-ward path of every document the graph knows: root-first positions
    /// of the `source` statements that enter the next document down.  Empty
    /// for a root.
    paths: HashMap<String, Vec<Placed>>,
    /// Root document of every keyed document (itself, for a root).
    root_of: HashMap<String, String>,
    /// Documents with no unique position — re-sourced from two sites, or on a
    /// cycle.  Every cross-document comparison touching one abstains.
    ambiguous: HashSet<String>,
}

impl RunOrder {
    /// Build the order from the workspace's resolved `source` edges.
    ///
    /// `edges` are `(parent, child)` with the parent-side position of the
    /// `source` statement; the caller has already dropped non-literal and
    /// unresolvable targets.  A child entered from two distinct sites — or from
    /// itself — is marked ambiguous, as is everything below it and every
    /// document on a cycle.
    #[must_use]
    pub fn build(edges: &[SourceEdge]) -> Self {
        let mut parent_of: HashMap<&str, (&str, Placed)> = HashMap::new();
        let mut ambiguous: HashSet<String> = HashSet::new();
        let mut documents: HashSet<&str> = HashSet::new();
        for edge in edges {
            documents.insert(edge.parent.as_str());
            documents.insert(edge.child.as_str());
            let entry = Placed {
                at: edge.at,
                enclosing_body: edge.enclosing_body,
            };
            if edge.parent == edge.child {
                ambiguous.insert(edge.child.clone());
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
        let mut paths: HashMap<String, Vec<Placed>> = HashMap::new();
        let mut root_of: HashMap<String, String> = HashMap::new();
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
        Self {
            paths,
            root_of,
            ambiguous,
        }
    }

    /// Whether the statement at `event` had already run when the statement at
    /// `query` ran; `None` when the two have no static order.
    ///
    /// The gating question — the leniency is
    /// [`tcl_compiler::analyser::indirection::in_effect_within`]'s, applied in
    /// the two points' deepest common document: a statement at the load level
    /// of some document counts as having run for a query inside one of that
    /// document's proc bodies, however far below it is written, because the
    /// whole file loads before any body runs.
    #[must_use]
    pub fn has_run(&self, event: RunPoint<'_>, query: RunPoint<'_>) -> Option<bool> {
        let (e, q) = self.common_frame(event, query)?;
        Some(tcl_compiler::analyser::indirection::in_effect_within(
            e.at,
            q.at,
            q.enclosing_body,
        ))
    }

    /// Which of two statements ran first under the **strict** "did this
    /// definitely already run" order ([`crate::namespace_import::load_order`]);
    /// `None` when they are incomparable.
    ///
    /// The ranking question the latest-wins folds and the import-conflict rule
    /// need.  Deliberately *not* [`Self::has_run`]'s leniency: a conflict
    /// cancels an install, so counting a maybe-never-run body statement as
    /// having run would invent conflicts and drop aliases the program really
    /// has — the reasoning on [`crate::namespace_import::load_order`].
    #[must_use]
    pub fn cmp_run(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<std::cmp::Ordering> {
        use crate::namespace_import::load_order;
        let (pa, pb) = self.common_frame(a, b)?;
        Some(
            load_order(pa.at, pa.enclosing_body.is_some())
                .cmp(&load_order(pb.at, pb.enclosing_body.is_some())),
        )
    }

    /// Both points as positions in their deepest common document — the single
    /// projection [`Self::has_run`] and [`Self::cmp_run`] share, so the gate
    /// and the conflict rule read one relation and differ only in which
    /// single-document rule they then apply.
    fn common_frame(&self, a: RunPoint<'_>, b: RunPoint<'_>) -> Option<(Placed, Placed)> {
        let own = |p: RunPoint<'_>| Placed {
            at: p.at,
            enclosing_body: p.enclosing_body,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn reqs(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(uri, pkgs)| {
                (
                    (*uri).to_owned(),
                    pkgs.iter().map(|p| (*p).to_owned()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn resolves_relative_source_against_parent_dir() {
        let got = resolve_source_target(Path::new("/proj/app.tcl"), "lib/util.tcl");
        assert_eq!(got, PathBuf::from("/proj/lib/util.tcl"));
    }

    #[test]
    fn resolves_dot_and_dotdot_segments() {
        let got = resolve_source_target(Path::new("/proj/sub/main.tcl"), "../lib/./util.tcl");
        assert_eq!(got, PathBuf::from("/proj/lib/util.tcl"));
    }

    #[test]
    fn absolute_source_passes_through() {
        let got = resolve_source_target(Path::new("/proj/app.tcl"), "/opt/x/y.tcl");
        assert_eq!(got, PathBuf::from("/opt/x/y.tcl"));
    }

    #[test]
    fn inherits_requires_from_direct_parent() {
        // app.tcl requires Tk and sources lib.tcl.
        let edges = vec![("app".to_owned(), "lib".to_owned())];
        let requires = reqs(&[("app", &["Tk", "http"]), ("lib", &[])]);
        let got = ancestor_requires("lib", &edges, &requires);
        assert_eq!(got, vec!["Tk".to_owned(), "http".to_owned()]);
    }

    #[test]
    fn inherits_transitively_through_a_chain() {
        // app -> mid -> leaf; leaf inherits from both app and mid.
        let edges = vec![
            ("app".to_owned(), "mid".to_owned()),
            ("mid".to_owned(), "leaf".to_owned()),
        ];
        let requires = reqs(&[("app", &["Tk"]), ("mid", &["json"]), ("leaf", &[])]);
        let mut got = ancestor_requires("leaf", &edges, &requires);
        got.sort();
        assert_eq!(got, vec!["Tk".to_owned(), "json".to_owned()]);
    }

    #[test]
    fn unions_requires_from_multiple_entry_points() {
        // Both a.tcl and b.tcl source shared.tcl with different requires.
        let edges = vec![
            ("a".to_owned(), "shared".to_owned()),
            ("b".to_owned(), "shared".to_owned()),
        ];
        let requires = reqs(&[("a", &["Tk"]), ("b", &["http"]), ("shared", &[])]);
        let got = ancestor_requires("shared", &edges, &requires);
        assert_eq!(got, vec!["Tk".to_owned(), "http".to_owned()]);
    }

    #[test]
    fn tolerates_cycles() {
        // a -> b -> a (pathological); must not loop forever.
        let edges = vec![
            ("a".to_owned(), "b".to_owned()),
            ("b".to_owned(), "a".to_owned()),
        ];
        let requires = reqs(&[("a", &["Tk"]), ("b", &["http"])]);
        let got = ancestor_requires("a", &edges, &requires);
        assert_eq!(got, vec!["Tk".to_owned(), "http".to_owned()]);
    }

    #[test]
    fn a_root_with_no_ancestors_inherits_nothing() {
        let edges = vec![("app".to_owned(), "lib".to_owned())];
        let requires = reqs(&[("app", &["Tk"])]);
        assert!(ancestor_requires("app", &edges, &requires).is_empty());
    }

    // ---------------------------------------------------------------------
    // `package prefer latest` across the source graph (issue #1253 item 1).
    //
    // tclsh-proof (8.6.14) that the latch really is interpreter-global and
    // crosses `source`:  with `lib.tcl` holding `puts [package prefer]`,
    //
    //   # app.tcl:  package prefer latest ; source lib.tcl
    //   -> latest
    //   # app.tcl:  source lib.tcl ; package prefer latest
    //   -> stable
    // ---------------------------------------------------------------------

    fn edge(parent: &str, child: &str, at: u32) -> SourceEdge {
        SourceEdge {
            parent: parent.to_owned(),
            child: child.to_owned(),
            at,
            enclosing_body: None,
        }
    }

    fn raises(pairs: &[(&str, &[u32])]) -> HashMap<String, Vec<u32>> {
        pairs
            .iter()
            .map(|(uri, offs)| ((*uri).to_owned(), offs.to_vec()))
            .collect()
    }

    /// TP: a raise written **above** the `source` is in force in the child.
    #[test]
    fn a_raise_before_the_source_reaches_the_child() {
        let edges = vec![edge("app", "lib", 30)];
        assert!(ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("app", &[0])])
        ));
    }

    /// TN: a raise written **below** the `source` has not run when the child
    /// loads, so the child keeps the default.
    #[test]
    fn a_raise_after_the_source_does_not_reach_the_child() {
        let edges = vec![edge("app", "lib", 0)];
        assert!(!ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("app", &[30])])
        ));
    }

    /// TP: the latch is transitive — once up on entry to a node it is up for
    /// everything that node sources, wherever the `source` sits.
    #[test]
    fn the_latch_travels_the_whole_graph_once_raised() {
        let edges = vec![edge("app", "mid", 30), edge("mid", "leaf", 0)];
        assert!(ancestor_prefer_latest_raised(
            "leaf",
            &edges,
            &raises(&[("app", &[0])])
        ));
    }

    /// TN: a raise in a node the target is **not** reachable from changes
    /// nothing — the graph, not the workspace, decides.
    #[test]
    fn an_unrelated_raise_does_not_reach_the_target() {
        let edges = vec![edge("app", "lib", 30), edge("other", "sibling", 30)];
        assert!(!ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("other", &[0])])
        ));
    }

    /// TP: a `source` written inside a proc body still sees a raise written
    /// later at the parent's load level — the whole file loads before any
    /// body runs, the `in_effect_within` rule every other cross-document
    /// ordering question uses.
    #[test]
    fn a_body_source_sees_a_later_load_level_raise() {
        let edges = vec![SourceEdge {
            parent: "app".to_owned(),
            child: "lib".to_owned(),
            at: 10,
            enclosing_body: Some(tcl_lexer::Span::new(0, 20)),
        }];
        assert!(ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("app", &[30])])
        ));
        // …but a raise inside that same body, after the `source`, has not run.
        assert!(!ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("app", &[15])])
        ));
    }

    /// A cycle terminates, and the target is still answered.
    #[test]
    fn the_prefer_walk_tolerates_cycles() {
        let edges = vec![edge("a", "b", 30), edge("b", "a", 0)];
        assert!(ancestor_prefer_latest_raised(
            "b",
            &edges,
            &raises(&[("a", &[0])])
        ));
        assert!(!ancestor_prefer_latest_raised("b", &edges, &raises(&[])));
    }

    /// TN: the target's *own* raises are not this question — that is the
    /// position-sensitive single-document answer.
    #[test]
    fn the_targets_own_raise_is_not_an_ancestor_raise() {
        let edges = vec![edge("app", "lib", 30)];
        assert!(!ancestor_prefer_latest_raised(
            "lib",
            &edges,
            &raises(&[("lib", &[0])])
        ));
    }

    // ---------------------------------------------------------------------
    // The `source`-graph load order (issue #1104 item 3, #1116 item 6).
    //
    // Oracle for the whole shape, byte-identical on tclsh 8.6.14 and 9.0.4 —
    // `source` inlines the sourced file's whole body at the `source`
    // statement's position, so the DFS of the source forest is the run order:
    //
    //   # src.tcl:  namespace eval ::src { proc p {} {return P} }
    //   # exp.tcl:  namespace eval ::src { namespace export p }
    //   # imp.tcl:  namespace eval ::dst { namespace import ::src::* }
    //
    //   # app.tcl:  source src.tcl ; source exp.tcl ; source imp.tcl
    //   ::dst::p   -> P                       (export ran before the import)
    //   # app.tcl:  source src.tcl ; source imp.tcl ; source exp.tcl
    //   ::dst::p   -> invalid command name "::dst::p"   (it did not)
    // ---------------------------------------------------------------------

    fn point(uri: &str, at: u32) -> RunPoint<'_> {
        RunPoint {
            uri,
            at,
            enclosing_body: None,
        }
    }

    fn body_point(uri: &str, at: u32, body: (u32, u32)) -> RunPoint<'_> {
        RunPoint {
            uri,
            at,
            enclosing_body: Some(tcl_lexer::Span::new(body.0, body.1)),
        }
    }

    /// TP: two files sourced by one parent are ordered by their `source`
    /// statements — everything in the first has run when the second loads.
    #[test]
    fn siblings_are_ordered_by_their_source_statements() {
        let order = RunOrder::build(&[edge("app", "exp", 10), edge("app", "imp", 20)]);
        assert_eq!(
            order.has_run(point("exp", 999), point("imp", 0)),
            Some(true),
            "exp.tcl is sourced first, so all of it has run when imp.tcl loads",
        );
        assert_eq!(
            order.has_run(point("imp", 0), point("exp", 999)),
            Some(false),
            "…and nothing in imp.tcl has run while exp.tcl is still loading",
        );
    }

    /// TP: a parent statement written above the `source` has run inside the
    /// child; one written below has not.
    #[test]
    fn a_parent_statement_is_ordered_against_the_child_body() {
        let order = RunOrder::build(&[edge("app", "lib", 50)]);
        assert_eq!(order.has_run(point("app", 10), point("lib", 5)), Some(true));
        assert_eq!(
            order.has_run(point("app", 90), point("lib", 5)),
            Some(false)
        );
        // …and the child's own statements have run for a parent statement
        // written below the `source`.
        assert_eq!(order.has_run(point("lib", 5), point("app", 90)), Some(true));
        assert_eq!(
            order.has_run(point("lib", 5), point("app", 10)),
            Some(false)
        );
    }

    /// TP: the leniency at the common document is `in_effect_within`'s — a
    /// child sourced from a parent's load level has run for a query inside
    /// one of that parent's proc bodies, however far below the `source` is
    /// written.
    #[test]
    fn a_body_local_query_observes_a_later_top_level_source() {
        let order = RunOrder::build(&[edge("app", "lib", 900)]);
        assert_eq!(
            order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
            Some(true),
            "the whole file loads — sourcing lib.tcl included — before any body runs",
        );
        // …but a `source` written inside that same body, after the query,
        // is an ordinary later statement of the running script.
        let order = RunOrder::build(&[SourceEdge {
            parent: "app".to_owned(),
            child: "lib".to_owned(),
            at: 15,
            enclosing_body: Some(tcl_lexer::Span::new(0, 20)),
        }]);
        assert_eq!(
            order.has_run(point("lib", 5), body_point("app", 10, (0, 20))),
            Some(false),
        );
    }

    /// TP: a grandchild is ordered against a sibling of its parent, through
    /// the deepest common document.
    #[test]
    fn a_grandchild_is_ordered_through_the_common_document() {
        // app sources exp @10, then mid @20; mid sources leaf @5.
        let order = RunOrder::build(&[
            edge("app", "exp", 10),
            edge("app", "mid", 20),
            edge("mid", "leaf", 5),
        ]);
        assert_eq!(
            order.has_run(point("exp", 999), point("leaf", 0)),
            Some(true)
        );
        assert_eq!(
            order.has_run(point("leaf", 0), point("exp", 999)),
            Some(false)
        );
    }

    /// TN: documents in different trees have no order — the pre-source-graph
    /// behaviour, and what an unrelated pair of files still gets.
    #[test]
    fn different_trees_are_incomparable() {
        let order = RunOrder::build(&[edge("app", "lib", 10), edge("other", "helper", 10)]);
        assert_eq!(order.has_run(point("lib", 0), point("helper", 0)), None);
        assert_eq!(order.cmp_run(point("lib", 0), point("helper", 0)), None);
    }

    /// TN: an empty graph orders nothing across documents…
    #[test]
    fn an_empty_graph_orders_nothing_across_documents() {
        let order = RunOrder::default();
        assert_eq!(order.has_run(point("a", 0), point("b", 0)), None);
        // …and a document the graph has never heard of is equally unordered.
        let order = RunOrder::build(&[edge("app", "lib", 10)]);
        assert_eq!(order.has_run(point("lib", 0), point("stray", 0)), None);
    }

    /// TP: two statements in the *same* document stay ordered regardless of
    /// the graph — the rule that predates this type, unchanged.
    #[test]
    fn one_document_is_ordered_without_the_graph() {
        let order = RunOrder::default();
        assert_eq!(order.has_run(point("a", 10), point("a", 20)), Some(true));
        assert_eq!(order.has_run(point("a", 20), point("a", 10)), Some(false));
        // Even for a re-sourced file, whose cross-document position abstains.
        let order = RunOrder::build(&[edge("app", "lib", 10), edge("other", "lib", 10)]);
        assert_eq!(
            order.has_run(point("lib", 10), point("lib", 20)),
            Some(true)
        );
    }

    /// TN (CRITICAL): a file reachable from two different `source` sites has
    /// no unique position, so every cross-document comparison touching it
    /// abstains — Tcl tolerates re-sourcing, and inventing an order would
    /// silently drop references.
    #[test]
    fn a_re_sourced_document_has_no_position() {
        let order = RunOrder::build(&[edge("app", "lib", 10), edge("app", "lib", 90)]);
        assert_eq!(order.has_run(point("lib", 0), point("app", 50)), None);
        // Two different parents, same story.
        let order = RunOrder::build(&[edge("app", "lib", 10), edge("tool", "lib", 10)]);
        assert_eq!(order.has_run(point("lib", 0), point("app", 50)), None);
    }

    /// TN: the ambiguity is inherited — a document below a re-sourced one has
    /// no unique position either.
    #[test]
    fn ambiguity_is_inherited_by_descendants() {
        let order = RunOrder::build(&[
            edge("app", "lib", 10),
            edge("app", "lib", 90),
            edge("lib", "deep", 5),
        ]);
        assert_eq!(order.has_run(point("deep", 0), point("app", 50)), None);
    }

    /// TN: a `source` cycle gives nothing on it a position, and terminates.
    #[test]
    fn a_source_cycle_abstains() {
        let order = RunOrder::build(&[edge("a", "b", 10), edge("b", "a", 10)]);
        assert_eq!(order.has_run(point("a", 0), point("b", 0)), None);
        // A self-`source` likewise.
        let order = RunOrder::build(&[edge("a", "a", 10), edge("a", "b", 20)]);
        assert_eq!(order.has_run(point("b", 0), point("a", 50)), None);
    }

    /// `cmp_run` is the strict order: a body-local statement never counts as
    /// having run at another document's load level, where `has_run`'s
    /// leniency says it may have.
    #[test]
    fn cmp_run_is_strict_where_has_run_is_lenient() {
        let order = RunOrder::build(&[edge("app", "lib", 900)]);
        let body_stmt = body_point("app", 10, (0, 20));
        // Lenient: the body statement is treated as having run for a query in
        // the sourced child (the body may have been entered).
        assert_eq!(order.has_run(body_stmt, point("lib", 5)), Some(true));
        // Strict: it is *not* known to have run before the child loaded.
        assert_eq!(
            order.cmp_run(body_stmt, point("lib", 5)),
            Some(std::cmp::Ordering::Greater),
            "a body-local statement sorts after every load-level one",
        );
        // Two load-level statements in different files rank by their `source`
        // statements, which is genuine execution order.
        let order = RunOrder::build(&[edge("app", "one", 10), edge("app", "two", 20)]);
        assert_eq!(
            order.cmp_run(point("one", 999), point("two", 0)),
            Some(std::cmp::Ordering::Less),
        );
    }

    /// Nothing runs before itself, at either level — what keeps an import
    /// from conflicting with itself once the order spans documents.
    #[test]
    fn a_point_does_not_precede_itself() {
        let order = RunOrder::build(&[edge("app", "lib", 10)]);
        assert_eq!(
            order.cmp_run(point("lib", 7), point("lib", 7)),
            Some(std::cmp::Ordering::Equal),
        );
        assert_eq!(order.has_run(point("lib", 7), point("lib", 7)), Some(false));
    }
}
