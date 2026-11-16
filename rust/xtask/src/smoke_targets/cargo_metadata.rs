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

//! What Cargo reports about the workspace: its targets, the features each is built with, package environments and link-path closures.

use super::{
    CargoRuntime, LIBRARY_KINDS, Metadata, Package, PackageEnvironments, ResolveNode,
    SUPPORTED_KINDS, Target, command_output,
};
use anyhow::{Context, Result, anyhow, bail};
use cargo_config2::Config as CargoConfig;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(super) fn rustc_host(root: &Path) -> Result<String> {
    let mut command = Command::new("rustc");
    command.arg("-vV").current_dir(root);
    let output = command_output(&mut command)?;
    let stdout = String::from_utf8(output.stdout).context("rustc -vV output is not UTF-8")?;
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("rustc -vV did not report a host triple"))
}

pub(super) fn cargo_runtime(root: &Path) -> Result<CargoRuntime> {
    let config = CargoConfig::load_with_cwd(root).context("loading Cargo configuration")?;
    let mut targets = config
        .build_target_for_config(std::iter::empty::<&str>())
        .context("resolving Cargo build target")?;
    if targets.len() != 1 {
        bail!("the smoke fallback requires one Cargo build target, found {targets:?}");
    }
    let target = targets.pop().context("Cargo resolved no build target")?;
    let runner = config
        .runner(&target)
        .context("resolving Cargo target runner")?;
    Ok(CargoRuntime {
        target: target.triple().to_owned(),
        target_directory: config
            .build
            .target_dir
            .clone()
            .unwrap_or_else(|| root.join("target")),
        runner,
        rustc: config.rustc().clone(),
    })
}

pub(super) fn metadata(root: &Path, host: &str) -> Result<Metadata> {
    let mut command = Command::new("cargo");
    command
        .args([
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            host,
        ])
        .current_dir(root);
    let output = command_output(&mut command)?;
    serde_json::from_slice(&output.stdout).context("parsing cargo metadata")
}

fn record_target_features(
    stdout: &str,
    display_to_name: &HashMap<String, String>,
    features: &mut HashMap<String, BTreeSet<String>>,
) {
    for line in stdout.lines().filter(|line| !line.is_empty()) {
        let Some((display, raw_features)) = line.split_once('\t') else {
            continue;
        };
        let Some(package_name) = display_to_name.get(display) else {
            continue;
        };
        let raw_features = raw_features
            .strip_suffix(" (*)")
            .unwrap_or(raw_features)
            .trim();
        if let Some(enabled) = features.get_mut(package_name) {
            enabled.extend(
                raw_features
                    .split(',')
                    .filter(|feature| !feature.is_empty())
                    .map(ToOwned::to_owned),
            );
        }
    }
}

fn record_command_target_features(
    command: &mut Command,
    display_to_name: &HashMap<String, String>,
    features: &mut HashMap<String, BTreeSet<String>>,
) -> Result<()> {
    let output = command_output(command)?;
    let stdout = String::from_utf8(output.stdout).context("cargo tree output is not UTF-8")?;
    record_target_features(&stdout, display_to_name, features);
    Ok(())
}

fn workspace_target_features(
    metadata: &Metadata,
    host: &str,
    root: &Path,
) -> Result<HashMap<String, BTreeSet<String>>> {
    let members: HashSet<&str> = metadata
        .workspace_members
        .iter()
        .map(String::as_str)
        .collect();
    let mut display_to_name = HashMap::new();
    let mut features = HashMap::new();
    let proc_macro_packages: BTreeSet<String> = metadata
        .packages
        .iter()
        .filter(|package| {
            members.contains(package.id.as_str())
                && package
                    .targets
                    .iter()
                    .any(|target| target.kind.iter().any(|kind| kind == "proc-macro"))
        })
        .map(|package| package.name.clone())
        .collect();
    for package in &metadata.packages {
        if !members.contains(package.id.as_str()) {
            continue;
        }
        let package_root = package
            .manifest_path
            .parent()
            .context("package manifest has no parent")?;
        let display = format!(
            "{} v{} ({})",
            package.name,
            package.version,
            package_root.display()
        );
        display_to_name.insert(display, package.name.clone());
        if package
            .targets
            .iter()
            .any(|target| target.kind.iter().any(|kind| kind == "proc-macro"))
        {
            display_to_name.insert(
                format!(
                    "{} v{} (proc-macro) ({})",
                    package.name,
                    package.version,
                    package_root.display()
                ),
                package.name.clone(),
            );
        }
        features.insert(package.name.clone(), BTreeSet::new());
    }

    let mut workspace_args = vec![
        "tree".to_owned(),
        "--workspace".to_owned(),
        "--locked".to_owned(),
        "--target".to_owned(),
        host.to_owned(),
        "--edges".to_owned(),
        "normal,dev,no-proc-macro".to_owned(),
        "--depth".to_owned(),
        "0".to_owned(),
        "--prefix".to_owned(),
        "none".to_owned(),
        "--format".to_owned(),
        "{p}\t{f}".to_owned(),
    ];
    for package in &proc_macro_packages {
        workspace_args.extend(["--exclude".to_owned(), package.clone()]);
    }
    let mut command = Command::new("cargo");
    command.args(&workspace_args).current_dir(root);
    record_command_target_features(&mut command, &display_to_name, &mut features)?;
    for package in proc_macro_packages {
        // Replace this package's host-unit features, then merge the full
        // ordinary dependency graph from its isolated workspace-root unit.
        features
            .get_mut(&package)
            .context("procedural-macro workspace package is missing from the feature map")?
            .clear();
        let mut command = Command::new("cargo");
        command
            .args([
                "tree",
                "-p",
                &package,
                "--locked",
                "--target",
                host,
                "--edges",
                "normal,dev,no-proc-macro",
                "--prefix",
                "none",
                "--format",
                "{p}\t{f}",
            ])
            .current_dir(root);
        record_command_target_features(&mut command, &display_to_name, &mut features)?;
    }
    Ok(features)
}

pub(super) fn canonical_target_kinds(raw_kinds: &[String]) -> BTreeSet<String> {
    let mut kinds: BTreeSet<String> = raw_kinds
        .iter()
        .filter(|kind| SUPPORTED_KINDS.contains(&kind.as_str()))
        .cloned()
        .collect();
    if raw_kinds
        .iter()
        .any(|kind| LIBRARY_KINDS.contains(&kind.as_str()))
    {
        kinds.insert("lib".to_owned());
    }
    kinds
}

pub(super) fn package_environment(package: &Package) -> Result<BTreeMap<OsString, OsString>> {
    let version = package
        .version
        .split('+')
        .next()
        .unwrap_or(&package.version);
    let (core, pre) = version.split_once('-').unwrap_or((version, ""));
    let mut components = core.split('.');
    let major = components.next().context("package version has no major")?;
    let minor = components.next().context("package version has no minor")?;
    let patch = components.next().context("package version has no patch")?;
    if components.next().is_some() {
        bail!("package version has more than three core components: {version}");
    }
    let manifest_dir = package
        .manifest_path
        .parent()
        .context("package manifest has no parent")?;
    let values = [
        (
            "CARGO_MANIFEST_DIR",
            manifest_dir.as_os_str().to_os_string(),
        ),
        (
            "CARGO_MANIFEST_PATH",
            package.manifest_path.as_os_str().to_os_string(),
        ),
        (
            "CARGO_PKG_AUTHORS",
            OsString::from(package.authors.join(":")),
        ),
        (
            "CARGO_PKG_DESCRIPTION",
            OsString::from(package.description.as_deref().unwrap_or_default()),
        ),
        (
            "CARGO_PKG_HOMEPAGE",
            OsString::from(package.homepage.as_deref().unwrap_or_default()),
        ),
        (
            "CARGO_PKG_LICENSE",
            OsString::from(package.license.as_deref().unwrap_or_default()),
        ),
        (
            "CARGO_PKG_LICENSE_FILE",
            package
                .license_file
                .as_deref()
                .unwrap_or_else(|| Path::new(""))
                .as_os_str()
                .to_os_string(),
        ),
        ("CARGO_PKG_NAME", OsString::from(&package.name)),
        (
            "CARGO_PKG_README",
            package
                .readme
                .as_deref()
                .unwrap_or_else(|| Path::new(""))
                .as_os_str()
                .to_os_string(),
        ),
        (
            "CARGO_PKG_REPOSITORY",
            OsString::from(package.repository.as_deref().unwrap_or_default()),
        ),
        (
            "CARGO_PKG_RUST_VERSION",
            OsString::from(package.rust_version.as_deref().unwrap_or_default()),
        ),
        ("CARGO_PKG_VERSION", OsString::from(&package.version)),
        ("CARGO_PKG_VERSION_MAJOR", OsString::from(major)),
        ("CARGO_PKG_VERSION_MINOR", OsString::from(minor)),
        ("CARGO_PKG_VERSION_PATCH", OsString::from(patch)),
        ("CARGO_PKG_VERSION_PRE", OsString::from(pre)),
    ];
    let environment: BTreeMap<OsString, OsString> = values
        .into_iter()
        .map(|(name, value)| (OsString::from(name), value))
        .collect();
    Ok(environment)
}

fn link_path_package_closures(metadata: &Metadata) -> HashMap<String, BTreeSet<String>> {
    let nodes: HashMap<&str, &ResolveNode> = metadata
        .resolve
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    metadata
        .workspace_members
        .iter()
        .map(|root| {
            let mut packages = BTreeSet::from([root.clone()]);
            let mut seen = HashSet::from([root.clone()]);
            let mut queue = VecDeque::from([(root.as_str(), true)]);
            while let Some((package_id, include_dev)) = queue.pop_front() {
                let Some(node) = nodes.get(package_id) else {
                    continue;
                };
                for dependency in &node.deps {
                    let contributes_link_paths = dependency.dep_kinds.iter().any(|kind| {
                        kind.kind.is_none()
                            || kind.kind.as_deref() == Some("build")
                            || (include_dev && kind.kind.as_deref() == Some("dev"))
                    });
                    if !contributes_link_paths {
                        continue;
                    }
                    if seen.insert(dependency.pkg.clone()) {
                        packages.insert(dependency.pkg.clone());
                        queue.push_back((dependency.pkg.as_str(), false));
                    }
                }
            }
            (root.clone(), packages)
        })
        .collect()
}

pub(super) fn load_targets(
    root: &Path,
    target: &str,
) -> Result<(HashMap<String, PathBuf>, PackageEnvironments, Vec<Target>)> {
    let metadata = metadata(root, target)?;
    let features = workspace_target_features(&metadata, target, root)?;
    let link_path_packages = link_path_package_closures(&metadata);
    let members: HashSet<&str> = metadata
        .workspace_members
        .iter()
        .map(String::as_str)
        .collect();
    let mut package_roots = HashMap::new();
    let mut package_environments = HashMap::new();
    let mut targets = Vec::new();
    for package in &metadata.packages {
        if !members.contains(package.id.as_str()) {
            continue;
        }
        let package_root = package
            .manifest_path
            .parent()
            .context("package manifest has no parent")?
            .to_path_buf();
        package_roots.insert(package.name.clone(), package_root);
        package_environments.insert(package.name.clone(), package_environment(package)?);
        let enabled = features
            .get(&package.name)
            .with_context(|| format!("missing feature context for {}", package.name))?;
        let package_link_paths = link_path_packages.get(&package.id).with_context(|| {
            format!("missing link-path dependency closure for {}", package.name)
        })?;
        for raw_target in &package.targets {
            let available = raw_target
                .required_features
                .iter()
                .all(|feature| enabled.contains(feature));
            for kind in canonical_target_kinds(&raw_target.kind) {
                targets.push(Target {
                    package_id: package.id.clone(),
                    package: package.name.clone(),
                    kind,
                    name: raw_target.name.clone(),
                    source: raw_target.src_path.clone(),
                    available,
                    testable: raw_target.test,
                    link_path_packages: package_link_paths.clone(),
                });
            }
        }
    }
    Ok((package_roots, package_environments, targets))
}
