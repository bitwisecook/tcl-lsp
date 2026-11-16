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

//! Fail-closed ownership and execution for the no-nextest smoke fallback.

use std::collections::{BTreeMap, BTreeSet, HashMap};
#[cfg(test)]
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};
use std::sync::atomic::AtomicU64;

use anyhow::{Context, Result, bail};
use cargo_config2::PathAndArgs;
use rustc_lexer::TokenKind;
use serde::Deserialize;

use crate::util::repo_root;

#[path = "smoke_targets/cargo_fixture.rs"]
mod cargo_fixture;
#[path = "smoke_targets/cargo_metadata.rs"]
mod cargo_metadata;
#[path = "smoke_targets/execution.rs"]
mod execution;
#[path = "smoke_targets/fixture_self_test.rs"]
mod fixture_self_test;
#[path = "smoke_targets/manifest.rs"]
mod manifest;
#[path = "smoke_targets/scanner_self_test.rs"]
mod scanner_self_test;
#[path = "smoke_targets/source_scan.rs"]
mod source_scan;
#[path = "smoke_targets/test_artifacts.rs"]
mod test_artifacts;

#[cfg(not(unix))]
use cargo_fixture::{rustc_probe_config, verify_rustc_probe};
#[cfg(unix)]
use cargo_fixture::{rustc_probe_config, verify_rustc_probe};
use cargo_metadata::{cargo_runtime, load_targets};
use execution::{execute_manifest, substring_skips};
#[cfg(not(test))]
use fixture_self_test::cargo_fixture_self_test;
#[cfg(test)]
use fixture_self_test::cargo_fixture_self_test;
use fixture_self_test::{
    cargo_fixture_self_test_inner, cargo_fixture_self_test_subprocess,
    cargo_profile_library_paths_self_test, fixture_collision_self_test,
};
use manifest::{best_owners, is_smoke_named_target, manifest_owners, validate_manifest};
use scanner_self_test::{
    duplicate_manifest_row_self_test, multi_owner_inventory_self_test, smoke_scanner_self_test,
};
#[cfg(test)]
use scanner_self_test::{
    inline_smoke_module_include_self_test, macro_rules_smoke_module_include_self_test,
    namespaced_include_scanner_self_test,
};
use source_scan::{check_source_inventory, record_target_source, scan_smoke_sources};

const MANIFEST: &str = "scripts/dev/smoke-targets.tsv";
const SUPPORTED_KINDS: &[&str] = &["lib", "bin", "test", "example", "bench"];
const LIBRARY_KINDS: &[&str] = &["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"];

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Target {
    package_id: String,
    package: String,
    kind: String,
    name: String,
    source: PathBuf,
    available: bool,
    testable: bool,
    link_path_packages: BTreeSet<String>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    resolve: Resolve,
}

#[derive(Debug, Deserialize)]
struct Resolve {
    nodes: Vec<ResolveNode>,
}

#[derive(Debug, Deserialize)]
struct ResolveNode {
    id: String,
    deps: Vec<ResolveDependency>,
}

#[derive(Debug, Deserialize)]
struct ResolveDependency {
    pkg: String,
    dep_kinds: Vec<DependencyKind>,
}

#[derive(Debug, Deserialize)]
struct DependencyKind {
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    version: String,
    authors: Vec<String>,
    description: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    license: Option<String>,
    license_file: Option<PathBuf>,
    readme: Option<PathBuf>,
    rust_version: Option<String>,
    manifest_path: PathBuf,
    targets: Vec<RawTarget>,
}

#[derive(Debug, Deserialize)]
struct RawTarget {
    kind: Vec<String>,
    name: String,
    src_path: PathBuf,
    #[serde(default, rename = "required-features")]
    required_features: Vec<String>,
    #[serde(default = "default_true")]
    test: bool,
}

#[derive(Clone, Debug)]
struct ManifestRow {
    source_text: String,
    package: String,
    kind: String,
    target_name: String,
}

#[derive(Clone, Copy, Default)]
struct SmokeMarkers {
    target: bool,
    no_smoke_include: bool,
}

#[derive(Default)]
struct SourceIncludes {
    literal_paths: Vec<String>,
    non_literal_count: usize,
    ambiguous_literal_count: usize,
}

#[derive(Clone, Copy)]
struct LexedToken {
    kind: TokenKind,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug)]
struct CargoRuntime {
    target: String,
    target_directory: PathBuf,
    runner: Option<PathAndArgs>,
    rustc: PathAndArgs,
}

#[derive(Debug, Eq, PartialEq)]
struct BuildEnvironment {
    out_dir: PathBuf,
    values: BTreeMap<OsString, OsString>,
    linked_paths: Vec<PathBuf>,
}

struct RuntimeEnvironment {
    values: BTreeMap<OsString, OsString>,
    preserves_dynamic_library_path: bool,
}

#[derive(Debug, Default)]
struct CargoTestArtifacts {
    executables: HashMap<TargetKey, PathBuf>,
    binary_executables: HashMap<(String, String), PathBuf>,
    build_environments: HashMap<String, Vec<BuildEnvironment>>,
    target_directory: PathBuf,
    explicit_target_context: bool,
}

struct HarnessContext<'a> {
    package_root: &'a Path,
    package_environment: &'a BTreeMap<OsString, OsString>,
    rust_runtime_library: &'a Path,
    runner: Option<&'a PathAndArgs>,
    preserves_dynamic_library_path: bool,
}

type TargetKey = (String, String, String, PathBuf);
type TargetIdentity = (String, String, String);
type PackageEnvironments = HashMap<String, BTreeMap<OsString, OsString>>;
type Contract = (
    HashMap<String, PathBuf>,
    PackageEnvironments,
    Vec<Target>,
    Vec<ManifestRow>,
    CargoRuntime,
);

#[derive(Default)]
struct SmokeInventory {
    sources: BTreeSet<String>,
    owners: HashMap<String, BTreeSet<TargetIdentity>>,
}

fn default_true() -> bool {
    true
}

fn command_output(command: &mut Command) -> Result<Output> {
    let rendered = format!("{command:?}");
    let output = command
        .output()
        .with_context(|| format!("running {rendered}"))?;
    if output.status.success() {
        return Ok(output);
    }
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    print!("{}", String::from_utf8_lossy(&output.stdout));
    bail!("{rendered} exited with {}", output.status)
}

fn check_contract(root: &Path) -> Result<Contract> {
    let manifest = root.join(MANIFEST);
    let runtime = cargo_runtime(root)?;
    let (package_roots, package_environments, targets) = load_targets(root, &runtime.target)?;
    let inventory = scan_smoke_sources(root, &targets)?;
    let rows = validate_manifest(root, &manifest, &package_roots, &targets, &inventory)?;
    check_source_inventory(root, &rows, &inventory, &targets)?;
    Ok((package_roots, package_environments, targets, rows, runtime))
}

struct Fixture {
    root: PathBuf,
}

fn self_test() -> Result<()> {
    let root = Path::new("/repo/example");
    fixture_collision_self_test()?;
    cargo_profile_library_paths_self_test()?;
    let targets = vec![
        Target {
            package_id: "example".to_owned(),
            package: "example".to_owned(),
            kind: "lib".to_owned(),
            name: "example".to_owned(),
            source: root.join("src/lib.rs"),
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        },
        Target {
            package_id: "example".to_owned(),
            package: "example".to_owned(),
            kind: "bin".to_owned(),
            name: "example".to_owned(),
            source: root.join("src/main.rs"),
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        },
        Target {
            package_id: "example".to_owned(),
            package: "example".to_owned(),
            kind: "test".to_owned(),
            name: "new".to_owned(),
            source: root.join("tests/new.rs"),
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        },
    ];
    let owners = best_owners(&root.join("tests/new.rs"), &targets);
    if owners.len() != 1 || owners[0].kind != "test" || owners[0].name != "new" {
        bail!("integration target ownership self-test failed");
    }
    if best_owners(&root.join("src/shared.rs"), &targets).len() != 2 {
        bail!("ambiguous library/binary ownership self-test failed");
    }
    duplicate_manifest_row_self_test()?;
    let mut inventory = SmokeInventory::default();
    record_target_source(&mut inventory, "smoke_tests.rs".to_owned(), &targets[0]);
    let owners = manifest_owners(
        &root.join("smoke_tests.rs"),
        "smoke_tests.rs",
        &targets,
        &inventory,
    );
    if owners.len() != 1 || owners[0].kind != "lib" {
        bail!("path-attributed module target ownership self-test failed");
    }
    multi_owner_inventory_self_test()?;
    let renamed_integration = Target {
        package_id: "example".to_owned(),
        package: "example".to_owned(),
        kind: "test".to_owned(),
        name: "ordinary".to_owned(),
        source: root.join("tests/file_smoke.rs"),
        available: true,
        testable: false,
        link_path_packages: BTreeSet::new(),
    };
    let convention_named_integration = Target {
        name: "file_smoke".to_owned(),
        testable: true,
        ..renamed_integration.clone()
    };
    if is_smoke_named_target(&renamed_integration)
        || !is_smoke_named_target(&convention_named_integration)
    {
        bail!("effective integration target-name self-test failed");
    }
    let entries = vec![
        ("smoke_fast".to_owned(), "test".to_owned()),
        ("nested::smoke_nested".to_owned(), "test".to_owned()),
        ("long_smoke_corpus".to_owned(), "test".to_owned()),
        ("deep".to_owned(), "test".to_owned()),
    ];
    let selected = entries[..2].to_vec();
    if substring_skips(&entries, &selected) != Some(vec!["long_smoke_corpus".to_owned()]) {
        bail!("Cargo substring exclusion self-test failed");
    }
    let collision_entries = vec![
        ("outer::smoke_long_smoke".to_owned(), "test".to_owned()),
        ("long_smoke".to_owned(), "test".to_owned()),
    ];
    if substring_skips(&collision_entries, &collision_entries[..1]).is_some() {
        bail!("overlapping Cargo substring exclusion self-test failed");
    }
    smoke_scanner_self_test()?;
    cargo_fixture_self_test()
}

/// Validate or execute the exact Cargo smoke fallback.
pub fn run(operation: &str, package: Option<&str>) -> Result<ExitCode> {
    let root = repo_root();
    match operation {
        "check" => {
            if package.is_some() {
                bail!("--package is only valid with smoke-targets run or list");
            }
            self_test()?;
            check_contract(&root)?;
            println!("targeted Cargo smoke ownership contract passed");
        }
        "run" | "list" => {
            let (package_roots, package_environments, targets, mut rows, runtime) =
                check_contract(&root)?;
            if let Some(package) = package {
                if !package_roots.contains_key(package) {
                    bail!("unknown Cargo package '{package}'");
                }
                rows.retain(|row| row.package == package);
            }
            execute_manifest(
                &root,
                &package_roots,
                &package_environments,
                &rows,
                &targets,
                &runtime,
                operation == "list",
            )?;
        }
        "fixture-self-test" if package.is_none() => cargo_fixture_self_test_inner()?,
        "fixture-self-test-isolated" if package.is_none() => cargo_fixture_self_test_subprocess()?,
        _ => bail!("unknown smoke-targets operation '{operation}'; expected check, run, or list"),
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests;
