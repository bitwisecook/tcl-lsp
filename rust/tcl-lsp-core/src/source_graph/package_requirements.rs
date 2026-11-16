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

//! The `package require`s a document gains from its source ancestors and descendants, and the `package prefer latest` latch.

use super::{RunEdge, RunEdgeKind};
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

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
    let mut requirements = ancestor_requirements(target, edges, requires);
    requirements.sort();
    requirements
}

/// Traverse source ancestors while preserving each caller-owned requirement
/// key. Graph reachability supplies advisory inheritance, not package loading
/// or source execution. Document identifiers are distinct from Tcl names.
#[must_use]
pub fn ancestor_requirements<K: Clone + Eq + std::hash::Hash, S: BuildHasher>(
    target: &str,
    edges: &[(String, String)],
    requires: &HashMap<String, Vec<K>, S>,
) -> Vec<K> {
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
    let mut ancestors = ancestors.into_iter().collect::<Vec<_>>();
    ancestors.sort_unstable();
    let mut seen = HashSet::new();
    let out: Vec<K> = ancestors
        .iter()
        .filter_map(|a| requires.get(*a))
        .flatten()
        .filter(|key| seen.insert((*key).clone()))
        .cloned()
        .collect();
    out
}

/// A package a document acquires by `source`ing another file, together with
/// **where in this document** it becomes available.
///
/// The position is what separates this from a plain package name: `source`
/// runs the child inline at that statement, so a command needing the package
/// is satisfied *after* the `source` and unsatisfied *before* it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedRequirement<K> {
    /// The `package require`d name, as written in the sourced file.
    pub name: K,
    /// Byte offset, **in the document that asked**, of the `source` statement
    /// that brings the package in.
    pub at: u32,
    /// Span of the innermost proc/class body containing that `source`
    /// statement; `None` at load level.  Feeds
    /// [`tcl_compiler::analyser::indirection::in_effect_within`], so a
    /// load-level `source` counts for a call written anywhere inside a proc
    /// body (the whole file loads before any body runs).
    pub enclosing_body: Option<tcl_lexer::Span>,
}

/// Positioned package-name reporting for compatibility consumers.
pub type PlacedRequire = PlacedRequirement<String>;

/// The `package require`s a document acquires from the files it (transitively)
/// `source`s — the **up** direction of the graph.
///
/// `edges` are the workspace's resolved `source` edges; only
/// [`RunEdgeKind::Source`] edges are followed, because only a `source` inlines
/// the child's body into the parent's run at a known position (a
/// `package require` edge merely bounds when the provider ran, which says
/// nothing about *this* document acquiring it).
///
/// Every package required anywhere below a direct edge `target -> child` is
/// attributed to that edge's position in `target`.  Depth beyond the first hop
/// is **not** re-gated: proving that `child`'s own `source` really executes
/// would need conditional-execution analysis this graph does not have, so the
/// walk takes the whole subtree and the caller treats the result as
/// suppression evidence rather than proof.  That is the deliberate direction
/// to be wrong in here — a missed suppression is a false "requires
/// `package require X`" on code that runs fine.
///
/// Cycles (`a` sources `b` sources `a`) terminate on the visited set, and a
/// file sourced twice simply contributes its packages at both positions.
/// `target`'s *own* `package require`s are not included — those are the
/// single-document question the analyser already answers.
#[must_use]
pub fn descendant_requires<S: BuildHasher>(
    target: &str,
    edges: &[RunEdge],
    requires: &HashMap<String, Vec<String>, S>,
) -> Vec<PlacedRequire> {
    let mut requirements = descendant_requirements(target, edges, requires);
    requirements.sort_by(|a, b| a.name.cmp(&b.name).then(a.at.cmp(&b.at)));
    requirements
}

/// Traverse positioned source descendants without converting the requirement
/// keys. Original byte policies and document-owned metadata remain separate
/// from this graph's advisory execution bounds.
#[must_use]
pub fn descendant_requirements<K: Clone + Eq + std::hash::Hash, S: BuildHasher>(
    target: &str,
    edges: &[RunEdge],
    requires: &HashMap<String, Vec<K>, S>,
) -> Vec<PlacedRequirement<K>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for edge in edges
        .iter()
        .filter(|e| e.parent == target && e.kind == RunEdgeKind::Source && e.child != target)
    {
        // Everything reachable from this direct child, attributed to the
        // position of *this* `source` statement in `target`.
        let mut stack: Vec<&str> = vec![edge.child.as_str()];
        let mut visited: HashSet<&str> = HashSet::new();
        visited.insert(target);
        while let Some(node) = stack.pop() {
            if !visited.insert(node) {
                continue;
            }
            if let Some(names) = requires.get(node) {
                out.extend(
                    names
                        .iter()
                        .filter(|name| seen.insert(((*name).clone(), edge.at, edge.enclosing_body)))
                        .map(|name| PlacedRequirement {
                            name: name.clone(),
                            at: edge.at,
                            enclosing_body: edge.enclosing_body,
                        }),
                );
            }
            for next in edges
                .iter()
                .filter(|e| e.parent == node && e.kind == RunEdgeKind::Source)
            {
                stack.push(next.child.as_str());
            }
        }
    }
    out
}

/// Whether the **interpreter-global** `package prefer latest` latch is already
/// raised by the time `target` is loaded, given the raises recorded per
/// document.
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
    edges: &[RunEdge],
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
