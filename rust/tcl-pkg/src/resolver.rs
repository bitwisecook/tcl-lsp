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

//! Minimum version selection for tclpkg dependencies.
//!
//! The source catalogue supplies locations, so candidates are the concrete
//! versions named by the dependency graph. Legacy minimums use tclpkg
//! ordering. Source-derived Tcl constraints can bound or exclude those
//! candidates before any selected package is materialised.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::errors::TclPkgError;
use crate::version::{DependencyVersion, Version};

/// A package with its dependency operand preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDependency {
    pub name: String,
    pub version: DependencyVersion,
}

impl PackageDependency {
    #[must_use]
    pub fn new(name: impl Into<String>, version: DependencyVersion) -> Self {
        Self {
            name: name.into(),
            version,
        }
    }
}

impl std::fmt::Display for PackageDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

/// A named package at a specific version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRef {
    pub name: String,
    pub version: String,
}

impl std::fmt::Display for PackageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

/// One entry in the resolved dependency graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPackage {
    pub reference: PackageRef,
    pub requires: Vec<PackageDependency>,
    pub dev: bool,
}

impl std::fmt::Display for ResolvedPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.reference.fmt(f)
    }
}

/// Root manifest `replace` directive, forcing a concrete version.
#[derive(Debug, Clone)]
pub struct ReplaceSpec {
    pub name: String,
    pub version: Version,
}

/// Root manifest `exclude` directive, refusing a concrete version.
#[derive(Debug, Clone)]
pub struct ExcludeSpec {
    pub name: String,
    pub version: Version,
}

/// Maps a selected package and its version to its transitive requirements.
pub type PackageManifestProvider<'a> =
    dyn Fn(&str, &str) -> Result<Vec<PackageDependency>, TclPkgError> + 'a;

/// Inputs to [`resolve`].
pub struct ResolveInput<'a> {
    pub direct: Vec<PackageDependency>,
    pub dev_direct: Vec<PackageDependency>,
    pub replaces: Vec<ReplaceSpec>,
    pub excludes: Vec<ExcludeSpec>,
    pub provider: Option<&'a PackageManifestProvider<'a>>,
    pub include_dev: bool,
}

impl Default for ResolveInput<'_> {
    fn default() -> Self {
        Self {
            direct: Vec::new(),
            dev_direct: Vec::new(),
            replaces: Vec::new(),
            excludes: Vec::new(),
            provider: None,
            include_dev: true,
        }
    }
}

const MAX_ITERATIONS: usize = 10_000;
const TCL: tcl_dialect::TclVersion = tcl_dialect::TclVersion::V9_1;

#[derive(Clone, Copy)]
enum Candidate<'a> {
    Minimum(&'a Version),
    Tcl(&'a str),
}

impl<'a> Candidate<'a> {
    fn tcl_spelling(self) -> Option<&'a str> {
        let value = match self {
            Self::Minimum(version) => version.original.as_str(),
            Self::Tcl(version) => version,
        };
        tcl_dialect::validate_version_for(value, TCL).then_some(value)
    }

    fn spelling(self) -> String {
        match self {
            Self::Minimum(version) => version.to_string(),
            Self::Tcl(version) => tcl_dialect::package_version_prefix_for(version, TCL).to_owned(),
        }
    }

    fn selected_spelling(self, tcl_order: bool) -> String {
        if tcl_order && let Some(value) = self.tcl_spelling() {
            return tcl_dialect::package_version_prefix_for(value, TCL).to_owned();
        }
        self.spelling()
    }

    fn cmp(self, other: Self, tcl_order: bool) -> Ordering {
        if tcl_order && let (Some(left), Some(right)) = (self.tcl_spelling(), other.tcl_spelling())
        {
            return tcl_dialect::compare_versions_for(left, right, TCL);
        }
        match (self, other) {
            (Self::Minimum(left), Self::Minimum(right)) => left.cmp(right),
            _ => self.spelling().cmp(&other.spelling()),
        }
    }

    fn matches(self, required: &DependencyVersion) -> bool {
        match required {
            DependencyVersion::Minimum(minimum) => match self {
                Self::Minimum(version) => version >= minimum,
                Self::Tcl(version) => {
                    if let Some(minimum) = Candidate::Minimum(minimum).tcl_spelling() {
                        tcl_dialect::compare_versions_for(version, minimum, TCL).is_ge()
                    } else {
                        Version::parse(version).is_ok_and(|version| version >= *minimum)
                    }
                }
            },
            DependencyVersion::Tcl(requirement) => self
                .tcl_spelling()
                .is_some_and(|version| requirement.matches(version)),
        }
    }
}

fn select_version<'a>(
    requirements: &'a [DependencyVersion],
    replacement: Option<&'a Version>,
) -> Option<Candidate<'a>> {
    if let Some(replacement) = replacement {
        let candidate = Candidate::Minimum(replacement);
        // Replacements retain their historical override of plain minimums.
        return requirements
            .iter()
            .all(|requirement| {
                matches!(requirement, DependencyVersion::Minimum(_))
                    || candidate.matches(requirement)
            })
            .then_some(candidate);
    }
    let minima = requirements
        .iter()
        .filter_map(|requirement| match requirement {
            DependencyVersion::Minimum(version) => Some(version),
            DependencyVersion::Tcl(_) => None,
        });
    let tcl = requirements
        .iter()
        .filter_map(|requirement| match requirement {
            DependencyVersion::Minimum(_) => None,
            DependencyVersion::Tcl(requirement) => Some(requirement),
        });
    if tcl.clone().next().is_none() {
        return minima.max().map(Candidate::Minimum);
    }
    minima
        .map(Candidate::Minimum)
        .chain(tcl.flat_map(|requirement| requirement.candidates().map(Candidate::Tcl)))
        .filter(|candidate| {
            requirements
                .iter()
                .all(|required| candidate.matches(required))
        })
        .min_by(|left, right| left.cmp(*right, true))
}

/// Run minimum version selection and return packages sorted by name.
///
/// # Errors
/// Returns an error when any dependency has no candidate, a candidate is
/// excluded, the provider fails, or graph traversal exceeds its work bound.
pub fn resolve(input: &ResolveInput<'_>) -> Result<Vec<ResolvedPackage>, TclPkgError> {
    let replaces: HashMap<_, _> = input
        .replaces
        .iter()
        .map(|r| (r.name.as_str(), &r.version))
        .collect();
    let dev_names: HashSet<_> = input.dev_direct.iter().map(|r| r.name.as_str()).collect();
    let mut constraints: BTreeMap<String, Vec<DependencyVersion>> = BTreeMap::new();
    let mut selected: BTreeMap<String, ResolvedPackage> = BTreeMap::new();
    let mut conflicting = BTreeSet::new();
    let mut pending = input.direct.clone();
    if input.include_dev {
        pending.extend(input.dev_direct.iter().cloned());
    }
    let mut iterations = 0;
    while !pending.is_empty() {
        let mut dirty = BTreeSet::new();
        for PackageDependency { name, version } in std::mem::take(&mut pending) {
            let requirements = constraints.entry(name.clone()).or_default();
            if !requirements.contains(&version) {
                requirements.push(version);
                dirty.insert(name);
            }
        }
        for name in dirty {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                return Err(TclPkgError::resolution(
                    "resolver exceeded maximum iterations (possible cycle)",
                    &[],
                    None,
                ));
            }
            let requirements = &constraints[&name];
            let Some(candidate) =
                select_version(requirements, replaces.get(name.as_str()).copied())
            else {
                conflicting.insert(name);
                continue;
            };
            conflicting.remove(&name);
            let tcl_order = requirements
                .iter()
                .any(|v| matches!(v, DependencyVersion::Tcl(_)));
            let version = candidate.selected_spelling(tcl_order);
            if input.excludes.iter().any(|exclusion| {
                exclusion.name == name
                    && candidate
                        .cmp(Candidate::Minimum(&exclusion.version), tcl_order)
                        .is_eq()
            }) {
                return Err(TclPkgError::resolution(
                    format!("package {name} {version} is excluded by the root manifest"),
                    &[],
                    Some(format!(
                        "remove the exclude directive or adjust the version constraint for {name}"
                    )),
                ));
            }
            if selected
                .get(&name)
                .is_some_and(|package| package.reference.version == version)
            {
                continue;
            }
            let requires = input
                .provider
                .map_or_else(|| Ok(Vec::new()), |provider| provider(&name, &version))?;
            pending.extend(requires.iter().cloned());
            let package = ResolvedPackage {
                dev: dev_names.contains(name.as_str()),
                reference: PackageRef {
                    name: name.clone(),
                    version,
                },
                requires,
            };
            selected.insert(name, package);
        }
    }
    if let Some(name) = conflicting.first() {
        let requirements = constraints[name]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        return Err(TclPkgError::resolution(
            format!(
                "no version of package {name} satisfies {}",
                requirements.join(", ")
            ),
            &[],
            None,
        ));
    }
    Ok(selected.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn pref(name: &str, ver: &str) -> PackageDependency {
        PackageDependency::new(name, DependencyVersion::Minimum(v(ver)))
    }

    #[test]
    fn picks_max_of_minimums() {
        let provider = |name: &str, _ver: &str| -> Result<Vec<PackageDependency>, TclPkgError> {
            if name == "a" {
                Ok(vec![pref("c", "1.0.0")])
            } else if name == "b" {
                Ok(vec![pref("c", "1.5.0")])
            } else {
                Ok(vec![])
            }
        };
        let input = ResolveInput {
            direct: vec![pref("a", "1.0.0"), pref("b", "1.0.0")],
            provider: Some(&provider),
            ..Default::default()
        };
        let resolved = resolve(&input).unwrap();
        let c = resolved.iter().find(|r| r.reference.name == "c").unwrap();
        assert_eq!(c.reference.version, "1.5.0");
    }

    #[test]
    fn leaves_resolve_directly() {
        let input = ResolveInput {
            direct: vec![pref("json", "1.0.0")],
            ..Default::default()
        };
        let resolved = resolve(&input).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].reference.name, "json");
    }

    #[test]
    fn replace_overrides_version() {
        let input = ResolveInput {
            direct: vec![pref("json", "1.0.0")],
            replaces: vec![ReplaceSpec {
                name: "json".to_string(),
                version: v("2.0.0"),
            }],
            ..Default::default()
        };
        let resolved = resolve(&input).unwrap();
        assert_eq!(resolved[0].reference.version, "2.0.0");
    }

    #[test]
    fn exclude_refuses_version() {
        let input = ResolveInput {
            direct: vec![pref("http", "2.9.7")],
            excludes: vec![ExcludeSpec {
                name: "http".to_string(),
                version: v("2.9.7"),
            }],
            ..Default::default()
        };
        let err = resolve(&input).unwrap_err();
        assert!(err.to_string().contains("excluded by the root manifest"));
    }

    #[test]
    fn dev_flag_propagates() {
        let input = ResolveInput {
            direct: vec![pref("json", "1.0.0")],
            dev_direct: vec![pref("tcltest", "2.5.0")],
            ..Default::default()
        };
        let resolved = resolve(&input).unwrap();
        let dev = resolved
            .iter()
            .find(|r| r.reference.name == "tcltest")
            .unwrap();
        assert!(dev.dev);
    }

    #[test]
    fn include_dev_false_skips() {
        let input = ResolveInput {
            direct: vec![pref("json", "1.0.0")],
            dev_direct: vec![pref("tcltest", "2.5.0")],
            include_dev: false,
            ..Default::default()
        };
        let resolved = resolve(&input).unwrap();
        assert!(resolved.iter().all(|r| r.reference.name != "tcltest"));
    }

    #[test]
    fn convergence_rewalks_bumped_minimum() {
        // a needs c@1.0 (which needs d@1.0); b needs c@2.0 (which needs d@3.0).
        // c converges to 2.0, so d must end up at 3.0.
        let provider = |name: &str, ver: &str| -> Result<Vec<PackageDependency>, TclPkgError> {
            match name {
                "a" => Ok(vec![pref("c", "1.0.0")]),
                "b" => Ok(vec![pref("c", "2.0.0")]),
                "c" if v(ver) >= v("2.0.0") => Ok(vec![pref("d", "3.0.0")]),
                "c" => Ok(vec![pref("d", "1.0.0")]),
                _ => Ok(vec![]),
            }
        };
        let input = ResolveInput {
            direct: vec![pref("a", "1.0.0"), pref("b", "1.0.0")],
            provider: Some(&provider),
            ..Default::default()
        };
        let resolved = resolve(&input).unwrap();
        let d = resolved.iter().find(|r| r.reference.name == "d").unwrap();
        assert_eq!(d.reference.version, "3.0.0");
    }
    fn tcl(name: &str, operand: &str) -> PackageDependency {
        PackageDependency::new(name, DependencyVersion::parse(operand).unwrap())
    }

    #[test]
    fn tcl_ranges_and_alternatives_retain_their_bounds() {
        let dependencies = |first, second| ResolveInput {
            direct: vec![tcl("json", first), tcl("json", second)],
            ..Default::default()
        };
        assert!(resolve(&dependencies("-tcl 1.2", "-tcl 2.0")).is_err());
        assert!(resolve(&dependencies("-tcl 1.2-2.0", "-exact 2.0")).is_err());
        assert_eq!(
            resolve(&dependencies("-tcl 1.2-", "-tcl 2.0")).unwrap()[0]
                .reference
                .version,
            "2.0"
        );
        assert_eq!(
            resolve(&dependencies("-tcl 1.0 2.0-2.5", "-tcl 2.1")).unwrap()[0]
                .reference
                .version,
            "2.1"
        );
    }

    #[test]
    fn tcl_exact_rejects_prereleases_and_uses_the_shared_version_width() {
        let exact = |operand, prerelease| ResolveInput {
            direct: vec![tcl("json", operand), tcl("json", prerelease)],
            ..Default::default()
        };
        assert!(resolve(&exact("-exact 1.2", "-exact 1.2b1")).is_err());
        assert_eq!(
            resolve(&exact(
                "-exact 18446744073709551616.1+custom",
                "-exact 18446744073709551616.1"
            ))
            .unwrap()[0]
                .reference
                .version,
            "18446744073709551616.1"
        );
    }
}
