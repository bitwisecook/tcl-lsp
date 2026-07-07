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
}
