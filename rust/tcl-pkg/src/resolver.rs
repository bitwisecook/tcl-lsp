//! Go-style Minimum Version Selection (MVS) resolver.
//!
//! MVS is deterministic and
//! solverless: every `require <pkg> <minver>` declares a minimum and the
//! resolver picks the maximum-of-minimums for each package across the whole
//! graph — no upper bounds, no backtracking, no SAT solver.
//!
//! The resolver is a pure function over data; callers supply a provider mapping
//! `(name, version)` to that package's own transitive requirements. `replace`
//! and `exclude` from the root manifest override/refuse versions; transitive
//! `replace`/`exclude` is deliberately ignored (matching Go).

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::errors::TclPkgError;
use crate::version::Version;

/// A named package at a specific version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRef {
    pub name: String,
    pub version: Version,
}

impl PackageRef {
    #[must_use]
    pub fn new(name: impl Into<String>, version: Version) -> Self {
        Self {
            name: name.into(),
            version,
        }
    }
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
    pub requires: Vec<PackageRef>,
    pub dev: bool,
}

impl std::fmt::Display for ResolvedPackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reference)
    }
}

/// Root manifest `replace` directive: force `name` to `version`.
#[derive(Debug, Clone)]
pub struct ReplaceSpec {
    pub name: String,
    pub version: Version,
}

/// Root manifest `exclude` directive: refuse this exact version.
#[derive(Debug, Clone)]
pub struct ExcludeSpec {
    pub name: String,
    pub version: Version,
}

/// Maps `(name, version)` to that package's transitive requirements.
///
/// Returning an empty vec means the package is a leaf. The closure may return
/// an error to abort resolution (wrapped as a [`TclPkgError`]).
pub type PackageManifestProvider<'a> =
    dyn Fn(&str, &Version) -> Result<Vec<PackageRef>, TclPkgError> + 'a;

/// Inputs to [`resolve`].
pub struct ResolveInput<'a> {
    pub direct: Vec<PackageRef>,
    pub dev_direct: Vec<PackageRef>,
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

const MAX_ITERATIONS: u32 = 10_000;

/// Run MVS over the dependency graph and return the resolved set, sorted by
/// package name.
#[allow(clippy::too_many_lines)] // the three-phase MVS routine in one function
pub fn resolve(input: &ResolveInput) -> Result<Vec<ResolvedPackage>, TclPkgError> {
    let replace_map: HashMap<&str, &ReplaceSpec> = input
        .replaces
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();

    let exclude_set: HashSet<(String, Version)> = input
        .excludes
        .iter()
        .map(|e| (e.name.clone(), e.version.clone()))
        .collect();

    let dev_names: HashSet<&str> = input.dev_direct.iter().map(|r| r.name.as_str()).collect();

    let provider = |name: &str, version: &Version| -> Result<Vec<PackageRef>, TclPkgError> {
        match input.provider {
            Some(p) => p(name, version),
            None => Ok(Vec::new()),
        }
    };

    // Step 1: collect max-of-minimums via BFS.
    let mut minimums: BTreeMap<String, Version> = BTreeMap::new();
    let mut pending: VecDeque<PackageRef> = VecDeque::new();
    // Keyed by name: (version_used, deps), so convergence can detect when a
    // minimum was bumped after deps were fetched.
    let mut requires_map: HashMap<String, (Version, Vec<PackageRef>)> = HashMap::new();

    for reference in &input.direct {
        pending.push_back(reference.clone());
    }
    if input.include_dev {
        for reference in &input.dev_direct {
            pending.push_back(reference.clone());
        }
    }

    let mut iterations: u32 = 0;
    while let Some(reference) = pending.pop_front() {
        iterations += 1;
        if iterations > MAX_ITERATIONS {
            return Err(TclPkgError::resolution(
                "resolver exceeded maximum iterations (possible cycle)",
                &[],
                None,
            ));
        }

        let name = reference.name.clone();
        let mut version = reference.version.clone();

        if let Some(rep) = replace_map.get(name.as_str()) {
            version = rep.version.clone();
        }

        if exclude_set.contains(&(name.clone(), version.clone())) {
            return Err(TclPkgError::resolution(
                format!("package {name} {version} is excluded by the root manifest"),
                &[],
                Some(format!(
                    "remove the exclude directive or adjust the version constraint for {name}"
                )),
            ));
        }

        if let Some(current) = minimums.get(&name)
            && version <= *current
        {
            continue;
        }
        minimums.insert(name.clone(), version.clone());

        let child_refs = provider(&name, &version)?;
        requires_map.insert(name.clone(), (version.clone(), child_refs.clone()));
        for child in child_refs {
            pending.push_back(child);
        }
    }

    // Step 2: convergence — re-process packages whose minimum was bumped after
    // their deps were already fetched.
    let mut dirty = true;
    while dirty {
        dirty = false;
        let snapshot: Vec<(String, Version)> = minimums
            .iter()
            .map(|(n, v)| (n.clone(), v.clone()))
            .collect();
        for (name, wanted) in snapshot {
            let needs = match requires_map.get(&name) {
                None => true,
                Some((recorded, _)) => *recorded < wanted,
            };
            if needs {
                let child_refs = provider(&name, &wanted)?;
                requires_map.insert(name.clone(), (wanted.clone(), child_refs.clone()));
                for child in child_refs {
                    pending.push_back(child);
                }
                dirty = true;
                break;
            }
        }
    }

    // Drain any leftover pending from step 2.
    while let Some(reference) = pending.pop_front() {
        let name = reference.name.clone();
        let mut version = reference.version.clone();
        if let Some(rep) = replace_map.get(name.as_str()) {
            version = rep.version.clone();
        }
        if exclude_set.contains(&(name.clone(), version.clone())) {
            return Err(TclPkgError::resolution(
                format!("package {name} {version} is excluded by the root manifest"),
                &[],
                None,
            ));
        }
        if let Some(current) = minimums.get(&name)
            && version <= *current
        {
            continue;
        }
        minimums.insert(name.clone(), version.clone());
        let child_refs = provider(&name, &version)?;
        requires_map.insert(name.clone(), (version.clone(), child_refs.clone()));
        for child in child_refs {
            pending.push_back(child);
        }
    }

    // Step 3: build the result list (BTreeMap iterates names in sorted order).
    let mut result = Vec::new();
    for (name, version) in &minimums {
        let reqs = requires_map
            .get(name)
            .map(|(_, deps)| deps.clone())
            .unwrap_or_default();
        result.push(ResolvedPackage {
            reference: PackageRef::new(name.clone(), version.clone()),
            requires: reqs,
            dev: dev_names.contains(name.as_str()),
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn pref(name: &str, ver: &str) -> PackageRef {
        PackageRef::new(name, v(ver))
    }

    #[test]
    fn picks_max_of_minimums() {
        let provider = |name: &str, _ver: &Version| -> Result<Vec<PackageRef>, TclPkgError> {
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
        assert_eq!(c.reference.version, v("1.5.0"));
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
        assert_eq!(resolved[0].reference.version, v("2.0.0"));
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
        let provider = |name: &str, ver: &Version| -> Result<Vec<PackageRef>, TclPkgError> {
            match name {
                "a" => Ok(vec![pref("c", "1.0.0")]),
                "b" => Ok(vec![pref("c", "2.0.0")]),
                "c" if *ver >= v("2.0.0") => Ok(vec![pref("d", "3.0.0")]),
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
        assert_eq!(d.reference.version, v("3.0.0"));
    }
}
