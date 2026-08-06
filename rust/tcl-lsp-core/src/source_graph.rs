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
}
