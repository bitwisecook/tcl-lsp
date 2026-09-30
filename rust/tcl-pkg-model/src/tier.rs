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

//! A package's distance from the workspace root.
//!
//! [`DependencyTier`] says how far a package sits: the workspace's own
//! package is the root, a package the root manifest names in `require` is
//! direct, one reached only through another package is transitive, and one
//! named only in `dev-require` (or reached only through such a package) is
//! development. The tier is *computed* from the lockfile's graph and the
//! root manifest's requirements, never claimed: a package's own manifest
//! has no say in it.
//!
//! The workspace's own package is the one tier this module does not decide,
//! because it is a fact about where a directory sits, not about the graph:
//! the caller that found the root manifest knows it is the root.

use std::collections::BTreeSet;

pub use tcl_dialect::model::DependencyTier;

use crate::lockfile::LockFile;
use crate::manifest::{ManifestAst, Requirement};

/// The tier `package` has in `lock`'s dependency graph under `root`'s
/// requirements, or `None` when the lockfile does not list it.
///
/// A package the root manifest names in `require` is
/// [`DependencyTier::Direct`], whatever else requires it. Otherwise a
/// package reached through the `require` graph is
/// [`DependencyTier::Transitive`]; one named in `dev-require`, or reached
/// only through such a package, is [`DependencyTier::Development`]. A package
/// the lockfile lists but no requirement reaches — a stale entry — is
/// [`DependencyTier::Transitive`]: nothing places it nearer, so it takes the
/// least a listed package gets.
#[must_use]
pub fn dependency_tier(
    root: &ManifestAst,
    lock: &LockFile,
    package: &str,
) -> Option<DependencyTier> {
    lock.lookup(package)?;
    if names(&root.requires, package) {
        return Some(DependencyTier::Direct);
    }
    if reachable(lock, &root.requires).contains(package) {
        return Some(DependencyTier::Transitive);
    }
    if names(&root.dev_requires, package) || reachable(lock, &root.dev_requires).contains(package) {
        return Some(DependencyTier::Development);
    }
    Some(DependencyTier::Transitive)
}

fn names(requirements: &[Requirement], package: &str) -> bool {
    requirements
        .iter()
        .any(|requirement| requirement.name == package)
}

/// Every package `roots` name or the lockfile's `requires` edges reach from
/// them.
fn reachable<'a>(lock: &'a LockFile, roots: &'a [Requirement]) -> BTreeSet<&'a str> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut pending: Vec<&str> = roots
        .iter()
        .map(|requirement| requirement.name.as_str())
        .collect();
    while let Some(name) = pending.pop() {
        if !seen.insert(name) {
            continue;
        }
        if let Some(locked) = lock.lookup(name) {
            pending.extend(locked.required_names());
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::{LockedPackage, SourceSpec};
    use crate::manifest::load_manifest_text;

    fn root() -> ManifestAst {
        load_manifest_text(
            "package myapp\nversion 1.0.0\nrequire json 1.0.0\nrequire net 2.0.0\n\
             dev-require tcltest 2.5.0\ndev-require lint 1.0.0\n",
            None,
        )
        .expect("the root manifest reads")
    }

    fn locked(name: &str, requires: &[&str], dev: bool) -> LockedPackage {
        LockedPackage {
            name: name.to_owned(),
            version: "1.0.0".to_owned(),
            source: SourceSpec::new("tarball", ""),
            integrity: String::new(),
            size: 0,
            requires: requires.iter().map(|entry| (*entry).to_owned()).collect(),
            provides: Vec::new(),
            license: String::new(),
            dev,
        }
    }

    /// `myapp` requires `json` and `net`; `net` requires `json` (so `json`
    /// is direct *and* reached) and `sock`, which requires `deep`. Its
    /// dev-requirements are `tcltest`, which requires `helper`, and `lint`,
    /// which requires `json` (reached by a regular route too) and `only_lint`.
    fn lock() -> LockFile {
        let mut lock = LockFile::new("myapp", ">=8.6");
        lock.packages = vec![
            locked("json", &[], false),
            locked("net", &["json@1.0.0", "sock@1.0.0"], false),
            locked("sock", &["deep@1.0.0"], false),
            locked("deep", &[], false),
            locked("tcltest", &["helper@1.0.0"], true),
            locked("helper", &[], false),
            locked("lint", &["json@1.0.0", "only_lint@1.0.0"], true),
            locked("only_lint", &[], false),
            locked("stale", &[], false),
        ];
        lock
    }

    fn tier(package: &str) -> Option<DependencyTier> {
        dependency_tier(&root(), &lock(), package)
    }

    /// A package the root names is direct even when another package also
    /// requires it: the root's own requirement is what places it.
    #[test]
    fn a_package_the_root_requires_is_direct() {
        assert_eq!(tier("json"), Some(DependencyTier::Direct));
        assert_eq!(tier("net"), Some(DependencyTier::Direct));
    }

    #[test]
    fn a_package_reached_only_through_another_is_transitive() {
        assert_eq!(tier("sock"), Some(DependencyTier::Transitive));
        assert_eq!(
            tier("deep"),
            Some(DependencyTier::Transitive),
            "two hops from the root"
        );
    }

    /// `dev-require` names development packages, and so does the graph
    /// behind them — until a regular route reaches the package too.
    #[test]
    fn a_package_named_only_in_dev_require_is_development() {
        assert_eq!(tier("tcltest"), Some(DependencyTier::Development));
        assert_eq!(tier("lint"), Some(DependencyTier::Development));
        assert_eq!(
            tier("helper"),
            Some(DependencyTier::Development),
            "reached only through a development package"
        );
        assert_eq!(tier("only_lint"), Some(DependencyTier::Development));
    }

    #[test]
    fn a_regular_route_outranks_a_development_one() {
        let mut manifest = root();
        manifest.dev_requires.push(Requirement {
            name: "deep".to_owned(),
            minimum: crate::version::Version::parse("1.0.0").expect("version"),
            source_url: None,
            dev: true,
        });
        assert_eq!(
            dependency_tier(&manifest, &lock(), "deep"),
            Some(DependencyTier::Transitive),
            "`deep` is also a development requirement, but not only one"
        );
    }

    /// A listed package nothing reaches takes the least a listed package
    /// gets, and one the lockfile does not list has no tier at all.
    #[test]
    fn a_stale_entry_is_transitive_and_an_unlisted_package_has_no_tier() {
        assert_eq!(tier("stale"), Some(DependencyTier::Transitive));
        assert_eq!(tier("myapp"), None, "the root is not in its own lockfile");
        assert_eq!(tier("stranger"), None);
    }

    /// A cycle in the lockfile's edges ends: each package is visited once.
    #[test]
    fn a_cycle_in_the_lockfile_terminates() {
        let mut lock = lock();
        lock.packages = vec![
            locked("json", &["net@1.0.0"], false),
            locked("net", &["json@1.0.0", "loop_a@1.0.0"], false),
            locked("loop_a", &["loop_b@1.0.0"], false),
            locked("loop_b", &["loop_a@1.0.0"], false),
        ];
        assert_eq!(
            dependency_tier(&root(), &lock, "loop_b"),
            Some(DependencyTier::Transitive)
        );
    }
}
