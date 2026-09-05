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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, anyhow, bail};
use cargo_config2::{Config as CargoConfig, PathAndArgs};
use rustc_lexer::{LiteralKind, TokenKind};
use serde::Deserialize;
use serde_json::Value;
use syn::ext::IdentExt;

use crate::util::repo_root;

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

fn harness_environment(
    package: &BTreeMap<OsString, OsString>,
    runtime: &BTreeMap<OsString, OsString>,
) -> BTreeMap<OsString, OsString> {
    let mut environment = runtime.clone();
    // Cargo's manifest-derived package variables are authoritative even when
    // a build script tries to emit a reserved name through rustc-env. Runtime
    // values without a package counterpart, including OUT_DIR and an authored
    // CARGO_MANIFEST_LINKS, remain intact.
    environment.extend(package.clone());
    environment
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

fn rustc_host(root: &Path) -> Result<String> {
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

fn cargo_runtime(root: &Path) -> Result<CargoRuntime> {
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

fn metadata(root: &Path, host: &str) -> Result<Metadata> {
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
    for package in &metadata.packages {
        if !members.contains(package.id.as_str()) {
            continue;
        }
        let package_root = package
            .manifest_path
            .parent()
            .context("package manifest has no parent")?;
        display_to_name.insert(
            format!(
                "{} v{} ({})",
                package.name,
                package.version,
                package_root.display()
            ),
            package.name.clone(),
        );
        features.insert(package.name.clone(), BTreeSet::new());
    }

    let mut command = Command::new("cargo");
    command
        .args([
            "tree",
            "--workspace",
            "--locked",
            "--target",
            host,
            "--edges",
            "normal,dev,no-proc-macro",
            "--depth",
            "0",
            "--prefix",
            "none",
            "--format",
            "{p}\t{f}",
        ])
        .current_dir(root);
    let output = command_output(&mut command)?;
    let stdout = String::from_utf8(output.stdout).context("cargo tree output is not UTF-8")?;
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
    Ok(features)
}

fn canonical_target_kinds(raw_kinds: &[String]) -> BTreeSet<String> {
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

fn package_environment(package: &Package) -> Result<BTreeMap<OsString, OsString>> {
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
    let procedural_macros: HashSet<&str> = metadata
        .packages
        .iter()
        .filter(|package| {
            package
                .targets
                .iter()
                .any(|target| target.kind.iter().any(|kind| kind == "proc-macro"))
        })
        .map(|package| package.id.as_str())
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
                    if !contributes_link_paths
                        || procedural_macros.contains(dependency.pkg.as_str())
                    {
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

fn load_targets(
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

fn ownership_rank(source: &Path, target: &Target) -> Option<u8> {
    if source == target.source {
        return Some(3);
    }
    if target.source.extension() == Some(OsStr::new("rs")) {
        let sibling_modules = target.source.with_extension("");
        if source.starts_with(sibling_modules) {
            return Some(2);
        }
    }
    if matches!(target.kind.as_str(), "test" | "example" | "bench") {
        return None;
    }
    let root_name = target.source.file_name()?;
    let root_parent = target.source.parent()?;
    if root_name == OsStr::new("main.rs")
        && root_parent.parent()?.file_name() == Some(OsStr::new("bin"))
    {
        return source.starts_with(root_parent).then_some(2);
    }
    if matches!(root_name.to_str(), Some("lib.rs" | "main.rs")) && source.starts_with(root_parent) {
        return Some(1);
    }
    None
}

fn best_owners<'a>(source: &Path, targets: &'a [Target]) -> Vec<&'a Target> {
    let ranked: Vec<(u8, &Target)> = targets
        .iter()
        .filter_map(|target| ownership_rank(source, target).map(|rank| (rank, target)))
        .collect();
    let Some(best) = ranked.iter().map(|(rank, _)| *rank).max() else {
        return Vec::new();
    };
    ranked
        .into_iter()
        .filter_map(|(rank, target)| (rank == best).then_some(target))
        .collect()
}

fn target_identity(target: &Target) -> TargetIdentity {
    (
        target.package.clone(),
        target.kind.clone(),
        target.name.clone(),
    )
}

fn manifest_owners<'a>(
    source: &Path,
    relative: &str,
    targets: &'a [Target],
    inventory: &SmokeInventory,
) -> Vec<&'a Target> {
    let Some(identities) = inventory.owners.get(relative) else {
        return best_owners(source, targets);
    };
    targets
        .iter()
        .filter(|target| identities.contains(&target_identity(target)))
        .collect()
}

fn is_smoke_named_target(target: &Target) -> bool {
    target.name == "smoke" || target.name.ends_with("_smoke")
}

fn read_manifest(manifest: &Path) -> Result<Vec<ManifestRow>> {
    let text =
        fs::read_to_string(manifest).with_context(|| format!("reading {}", manifest.display()))?;
    let mut rows = Vec::new();
    let mut seen = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 4 {
            bail!(
                "{}:{}: expected four tab-separated fields",
                manifest.display(),
                index + 1
            );
        }
        let line_number = index + 1;
        if let Some(first_line) =
            seen.insert((fields[0], fields[1], fields[2], fields[3]), line_number)
        {
            bail!(
                "{}:{line_number}: duplicate smoke target row (first declared at line {first_line})",
                manifest.display()
            );
        }
        rows.push(ManifestRow {
            source_text: fields[0].to_owned(),
            package: fields[1].to_owned(),
            kind: fields[2].to_owned(),
            target_name: fields[3].to_owned(),
        });
    }
    Ok(rows)
}

fn validate_manifest(
    root: &Path,
    manifest: &Path,
    package_roots: &HashMap<String, PathBuf>,
    targets: &[Target],
    inventory: &SmokeInventory,
) -> Result<Vec<ManifestRow>> {
    let rows = read_manifest(manifest)?;
    let mut errors = Vec::new();
    let mut declared = BTreeSet::new();
    for row in &rows {
        declared.insert((
            row.package.clone(),
            row.kind.clone(),
            row.target_name.clone(),
        ));
        let source = root.join(&row.source_text);
        if !source.is_file() {
            errors.push(format!("missing smoke source: {}", row.source_text));
            continue;
        }
        if !package_roots.contains_key(&row.package) {
            errors.push(format!(
                "unknown Cargo package '{}' for {}",
                row.package, row.source_text
            ));
            continue;
        }
        if !SUPPORTED_KINDS.contains(&row.kind.as_str()) {
            errors.push(format!(
                "invalid smoke target kind '{}' for {}",
                row.kind, row.source_text
            ));
            continue;
        }
        let package_targets: Vec<Target> = targets
            .iter()
            .filter(|target| target.package == row.package)
            .cloned()
            .collect();
        let owners = manifest_owners(&source, &row.source_text, &package_targets, inventory);
        let exact: Vec<&Target> = owners
            .iter()
            .filter(|owner| owner.kind == row.kind && owner.name == row.target_name)
            .copied()
            .collect();
        if exact.len() == 1 {
            if !exact[0].testable {
                errors.push(format!(
                    "smoke source {} belongs to Cargo target {}:{} with test = false",
                    row.source_text, row.kind, row.target_name
                ));
            }
            continue;
        }
        if owners.is_empty() {
            errors.push(format!(
                "no Cargo target owns smoke source {}",
                row.source_text
            ));
        } else if owners.len() > 1 {
            let names = owners
                .iter()
                .map(|owner| format!("{}:{}", owner.kind, owner.name))
                .collect::<Vec<_>>()
                .join(", ");
            errors.push(format!(
                "ambiguous Cargo target ownership for {}: {names}; move the smoke test to a target root or integration test",
                row.source_text
            ));
        } else {
            errors.push(format!(
                "smoke source {} belongs to {}:{}, not {}:{}",
                row.source_text, owners[0].kind, owners[0].name, row.kind, row.target_name
            ));
        }
    }

    for target in targets
        .iter()
        .filter(|target| target.testable && is_smoke_named_target(target))
    {
        let identity = (
            target.package.clone(),
            target.kind.clone(),
            target.name.clone(),
        );
        if !declared.contains(&identity) {
            errors.push(format!(
                "smoke-named Cargo target {} {}:{} has no smoke-targets.tsv row",
                target.package, target.kind, target.name
            ));
        }
    }
    if errors.is_empty() {
        Ok(rows)
    } else {
        bail!("{}", errors.join("\n"))
    }
}

fn tracked_rust_sources(root: &Path) -> Result<Vec<String>> {
    let mut command = Command::new("git");
    command
        .args(["ls-files", "--", ":(glob)rust/**/*.rs"])
        .current_dir(root);
    let output = command_output(&mut command)?;
    Ok(String::from_utf8(output.stdout)
        .context("git ls-files output is not UTF-8")?
        .lines()
        .map(ToOwned::to_owned)
        .collect())
}

fn meta_applies_test_attribute(meta: &syn::Meta) -> bool {
    if meta
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
    {
        return true;
    }
    let syn::Meta::List(list) = meta else {
        return false;
    };
    if !list.path.is_ident("cfg_attr") {
        return false;
    }
    list.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .is_ok_and(|nested| nested.iter().skip(1).any(meta_applies_test_attribute))
}

fn has_test_attribute(attributes: &[syn::Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| meta_applies_test_attribute(&attribute.meta))
}

fn module_directory(source: &Path) -> Result<PathBuf> {
    let parent = source
        .parent()
        .with_context(|| format!("Rust source has no parent: {}", source.display()))?;
    let stem = source
        .file_stem()
        .and_then(OsStr::to_str)
        .with_context(|| format!("Rust source has no UTF-8 stem: {}", source.display()))?;
    if matches!(stem, "lib" | "main" | "mod") {
        Ok(parent.to_path_buf())
    } else {
        Ok(parent.join(stem))
    }
}

fn lexically_normalise(path: &Path) -> PathBuf {
    let mut normalised = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalised.pop() {
                    normalised.push(component.as_os_str());
                }
            }
            _ => normalised.push(component.as_os_str()),
        }
    }
    normalised
}

fn collect_module_paths(meta: &syn::Meta, paths: &mut Vec<String>) {
    if meta.path().is_ident("path") {
        if let syn::Meta::NameValue(value) = meta
            && let syn::Expr::Lit(value) = &value.value
            && let syn::Lit::Str(value) = &value.lit
        {
            paths.push(value.value());
        }
        return;
    }
    let syn::Meta::List(list) = meta else {
        return;
    };
    if !list.path.is_ident("cfg_attr") {
        return;
    }
    if let Ok(nested) = list
        .parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
    {
        for attribute in nested.iter().skip(1) {
            collect_module_paths(attribute, paths);
        }
    }
}

fn module_sources(module: &syn::ItemMod, directory: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut configured = Vec::new();
    for attribute in &module.attrs {
        collect_module_paths(&attribute.meta, &mut configured);
    }
    let has_direct_path = module
        .attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("path"));
    let mut sources: Vec<(PathBuf, PathBuf)> = configured
        .into_iter()
        .map(|path| {
            let source = lexically_normalise(&directory.join(path));
            let child_directory = source
                .parent()
                .map_or_else(|| directory.to_path_buf(), Path::to_path_buf);
            (source, child_directory)
        })
        .filter(|(path, _)| path.is_file())
        .collect();
    if !has_direct_path {
        let name = module.ident.unraw().to_string();
        sources.extend(
            [
                directory.join(format!("{name}.rs")),
                directory.join(&name).join("mod.rs"),
            ]
            .into_iter()
            .filter(|path| path.is_file())
            .map(|path| (path, directory.join(&name))),
        );
    }
    sources.sort();
    sources.dedup();
    sources
}

fn inline_module_directories(module: &syn::ItemMod, directory: &Path) -> Vec<PathBuf> {
    let mut configured = Vec::new();
    for attribute in &module.attrs {
        collect_module_paths(&attribute.meta, &mut configured);
    }
    let has_direct_path = module
        .attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("path"));
    let mut directories: Vec<PathBuf> = configured
        .into_iter()
        .map(|path| lexically_normalise(&directory.join(path)))
        .collect();
    if !has_direct_path {
        directories.push(directory.join(module.ident.unraw().to_string()));
    }
    directories.sort();
    directories.dedup();
    directories
}

fn collect_smoke_test_sources(
    source: &Path,
    directory: &Path,
    items: &[syn::Item],
    inside_smoke_module: bool,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    found: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                if has_test_attribute(&function.attrs)
                    && (inside_smoke_module
                        || function.sig.ident.unraw().to_string().starts_with("smoke"))
                {
                    found.insert(source.to_path_buf());
                }
            }
            syn::Item::Mod(module) => {
                let module_is_smoke =
                    inside_smoke_module || module.ident.unraw().to_string().starts_with("smoke");
                if let Some((_, items)) = &module.content {
                    for child_directory in inline_module_directories(module, directory) {
                        collect_smoke_test_sources(
                            source,
                            &child_directory,
                            items,
                            module_is_smoke,
                            visited,
                            found,
                        )?;
                    }
                } else {
                    for (child, child_directory) in module_sources(module, directory) {
                        collect_source_smoke_tests_at(
                            &child,
                            &child_directory,
                            module_is_smoke,
                            visited,
                            found,
                        )?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn collect_source_smoke_tests(
    source: &Path,
    inside_smoke_module: bool,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    found: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    collect_source_smoke_tests_at(
        source,
        &module_directory(source)?,
        inside_smoke_module,
        visited,
        found,
    )
}

fn collect_source_smoke_tests_at(
    source: &Path,
    directory: &Path,
    inside_smoke_module: bool,
    visited: &mut HashSet<(PathBuf, PathBuf, bool)>,
    found: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    if !visited.insert((
        source.to_path_buf(),
        directory.to_path_buf(),
        inside_smoke_module,
    )) {
        return Ok(());
    }
    let text = fs::read_to_string(source)
        .with_context(|| format!("reading Rust module {}", source.display()))?;
    let parsed = syn::parse_file(&text)
        .with_context(|| format!("parsing Rust module {}", source.display()))?;
    let (markers, source_includes) = source_smoke_contract(source, &text)
        .with_context(|| format!("checking smoke markers in {}", source.display()))?;
    if markers.target {
        found.insert(source.to_path_buf());
    }
    if let Some(parent) = source.parent() {
        for path in source_includes.literal_paths {
            let included_path = lexically_normalise(&parent.join(path));
            if !included_path.is_file() {
                continue;
            }
            let included_text = fs::read_to_string(&included_path)
                .with_context(|| format!("reading Rust include {}", included_path.display()))?;
            if syn::parse_file(&included_text).is_err() {
                // An expression- or statement-only include cannot add a
                // top-level libtest item. Parseable item includes are walked
                // regardless of whether they occur directly or inside a
                // macro body.
                continue;
            }
            let child_directory = included_path
                .parent()
                .map_or_else(|| directory.to_path_buf(), Path::to_path_buf);
            collect_source_smoke_tests_at(
                &included_path,
                &child_directory,
                inside_smoke_module,
                visited,
                found,
            )?;
        }
    }
    collect_smoke_test_sources(
        source,
        directory,
        &parsed.items,
        inside_smoke_module,
        visited,
        found,
    )
}

fn source_smoke_markers(text: &str) -> Result<SmokeMarkers> {
    let mut offset = 0;
    let mut line_start = 0;
    let mut line_number = 1;
    let mut markers = SmokeMarkers::default();
    for token in rustc_lexer::tokenize(text) {
        let end = offset + token.len;
        let lexeme = &text[offset..end];
        if token.kind == TokenKind::LineComment {
            let body = lexeme
                .strip_prefix("//")
                .context("Rust lexer returned a malformed line comment")?
                .trim();
            let standalone = text[line_start..offset]
                .chars()
                .all(|character| matches!(character, ' ' | '\t'));
            if body == "tcl-lsp-smoke-target" && standalone {
                markers.target = true;
            } else if body == "tcl-lsp-no-smoke-include" && standalone {
                markers.no_smoke_include = true;
            } else if body.starts_with("tcl-lsp-") && body.contains("smoke") {
                bail!(
                    "line {line_number}: invalid smoke marker; expected exactly '// tcl-lsp-smoke-target' or '// tcl-lsp-no-smoke-include' on a standalone line"
                );
            }
        }
        line_number += lexeme.bytes().filter(|byte| *byte == b'\n').count();
        if let Some(last_newline) = lexeme.rfind('\n') {
            line_start = offset + last_newline + 1;
        }
        offset = end;
    }
    if markers.target && markers.no_smoke_include {
        bail!("source has contradictory smoke-target and no-smoke-include markers");
    }
    Ok(markers)
}

fn delimiter_close(kind: TokenKind) -> Option<TokenKind> {
    match kind {
        TokenKind::OpenParen => Some(TokenKind::CloseParen),
        TokenKind::OpenBrace => Some(TokenKind::CloseBrace),
        TokenKind::OpenBracket => Some(TokenKind::CloseBracket),
        _ => None,
    }
}

fn source_include_macros(text: &str) -> Result<SourceIncludes> {
    let mut offset = 0;
    let mut tokens = Vec::new();
    for token in rustc_lexer::tokenize(text) {
        let end = offset + token.len;
        if !matches!(
            token.kind,
            TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment { .. }
        ) {
            tokens.push(LexedToken {
                kind: token.kind,
                start: offset,
                end,
            });
        }
        offset = end;
    }

    let mut includes = SourceIncludes::default();
    for (index, token) in tokens.iter().enumerate() {
        let name = &text[token.start..token.end];
        if !matches!(token.kind, TokenKind::Ident | TokenKind::RawIdent)
            || !matches!(name, "include" | "r#include")
            || index
                .checked_sub(1)
                .is_some_and(|previous| matches!(tokens[previous].kind, TokenKind::Dollar))
            || tokens
                .get(index + 1)
                .is_none_or(|next| next.kind != TokenKind::Not)
        {
            continue;
        }
        let Some(open) = tokens.get(index + 2) else {
            includes.non_literal_count += 1;
            continue;
        };
        let Some(close) = delimiter_close(open.kind) else {
            includes.non_literal_count += 1;
            continue;
        };
        let mut closes = vec![close];
        let mut close_index = None;
        for (candidate_index, candidate) in tokens.iter().enumerate().skip(index + 3) {
            if let Some(expected) = delimiter_close(candidate.kind) {
                closes.push(expected);
            } else if matches!(
                candidate.kind,
                TokenKind::CloseParen | TokenKind::CloseBrace | TokenKind::CloseBracket
            ) {
                let Some(expected) = closes.pop() else {
                    bail!("unbalanced delimiter while scanning include! invocation");
                };
                if candidate.kind != expected {
                    bail!("mismatched delimiter while scanning include! invocation");
                }
                if closes.is_empty() {
                    close_index = Some(candidate_index);
                    break;
                }
            }
        }
        let close_index = close_index.context("unterminated include! invocation")?;
        let body = &tokens[index + 3..close_index];
        let literal = body.first().filter(|_| body.len() == 1).and_then(|body| {
            if matches!(
                body.kind,
                TokenKind::Literal {
                    kind: LiteralKind::Str { .. } | LiteralKind::RawStr { .. },
                    ..
                }
            ) {
                syn::parse_str::<syn::LitStr>(&text[body.start..body.end]).ok()
            } else {
                None
            }
        });
        if let Some(literal) = literal {
            includes.literal_paths.push(literal.value());
        } else {
            includes.non_literal_count += 1;
        }
    }
    Ok(includes)
}

fn source_smoke_contract(source: &Path, text: &str) -> Result<(SmokeMarkers, SourceIncludes)> {
    let markers = source_smoke_markers(text)?;
    let includes = source_include_macros(text)?;
    let parent = source
        .parent()
        .with_context(|| format!("Rust source has no parent: {}", source.display()))?;
    let missing_literal_count = includes
        .literal_paths
        .iter()
        .filter(|path| !lexically_normalise(&parent.join(path)).is_file())
        .count();
    let unresolved_count = includes.non_literal_count + missing_literal_count;
    if markers.no_smoke_include && unresolved_count != 1 {
        bail!(
            "a no-smoke-include marker must classify exactly one unresolved include! invocation; found {unresolved_count}"
        );
    }
    if unresolved_count > 0 && !markers.target && !markers.no_smoke_include {
        bail!(
            "{unresolved_count} unresolved include! invocation(s) require an exact standalone '// tcl-lsp-smoke-target' or '// tcl-lsp-no-smoke-include' comment"
        );
    }
    Ok((markers, includes))
}

fn relative_source(root: &Path, source: &Path) -> Result<String> {
    Ok(source
        .strip_prefix(root)
        .with_context(|| {
            format!(
                "discovered smoke source is outside workspace: {}",
                source.display()
            )
        })?
        .to_string_lossy()
        .replace('\\', "/"))
}

fn record_target_owner(inventory: &mut SmokeInventory, relative: String, target: &Target) {
    inventory
        .owners
        .entry(relative)
        .or_default()
        .insert(target_identity(target));
}

fn record_target_source(inventory: &mut SmokeInventory, relative: String, target: &Target) {
    inventory.sources.insert(relative.clone());
    record_target_owner(inventory, relative, target);
}

fn scan_target_smoke_sources(
    root: &Path,
    target: &Target,
    inventory: &mut SmokeInventory,
) -> Result<()> {
    let smoke_named = is_smoke_named_target(target);
    if smoke_named {
        record_target_source(inventory, relative_source(root, &target.source)?, target);
    }
    let target_directory = target.source.parent().with_context(|| {
        format!(
            "Cargo target source has no parent: {}",
            target.source.display()
        )
    })?;
    let mut visited = HashSet::new();
    let mut found = BTreeSet::new();
    collect_source_smoke_tests_at(
        &target.source,
        target_directory,
        smoke_named,
        &mut visited,
        &mut found,
    )?;
    for (source, _, _) in visited {
        if source.starts_with(root) {
            record_target_owner(inventory, relative_source(root, &source)?, target);
        }
    }
    for source in found {
        record_target_source(inventory, relative_source(root, &source)?, target);
    }
    Ok(())
}

fn scan_smoke_sources(root: &Path, targets: &[Target]) -> Result<SmokeInventory> {
    let mut inventory = SmokeInventory::default();
    for relative in tracked_rust_sources(root)? {
        let source = root.join(&relative);
        let text = fs::read_to_string(&source).with_context(|| format!("reading {relative}"))?;
        if source_smoke_contract(&source, &text)
            .with_context(|| format!("checking {relative}"))?
            .0
            .target
        {
            inventory.sources.insert(relative);
        }
    }
    for target in targets.iter().filter(|target| target.testable) {
        scan_target_smoke_sources(root, target, &mut inventory)?;
    }
    Ok(inventory)
}

fn inventory_entries(
    root: &Path,
    inventory: &SmokeInventory,
    targets: &[Target],
) -> BTreeSet<(String, TargetIdentity)> {
    inventory
        .sources
        .iter()
        .flat_map(|source| {
            let owners: Vec<TargetIdentity> = inventory.owners.get(source).map_or_else(
                || {
                    best_owners(&root.join(source), targets)
                        .into_iter()
                        .map(target_identity)
                        .collect()
                },
                |owners| owners.iter().cloned().collect(),
            );
            owners.into_iter().map(|owner| (source.clone(), owner))
        })
        .collect()
}

fn inventory_entry_text(entry: &(String, TargetIdentity)) -> String {
    let (source, (package, kind, target)) = entry;
    format!("{source} [{package} {kind}:{target}]")
}

fn check_source_inventory(
    root: &Path,
    rows: &[ManifestRow],
    inventory: &SmokeInventory,
    targets: &[Target],
) -> Result<()> {
    let expected_sources: BTreeSet<String> =
        rows.iter().map(|row| row.source_text.clone()).collect();
    let expected_entries: BTreeSet<(String, TargetIdentity)> = rows
        .iter()
        .map(|row| {
            (
                row.source_text.clone(),
                (
                    row.package.clone(),
                    row.kind.clone(),
                    row.target_name.clone(),
                ),
            )
        })
        .collect();
    let actual_entries = inventory_entries(root, inventory, targets);
    if expected_sources == inventory.sources && expected_entries == actual_entries {
        return Ok(());
    }
    let missing_sources: Vec<_> = inventory
        .sources
        .difference(&expected_sources)
        .cloned()
        .collect();
    let stale_sources: Vec<_> = expected_sources
        .difference(&inventory.sources)
        .cloned()
        .collect();
    let missing_entries: Vec<_> = actual_entries
        .difference(&expected_entries)
        .map(inventory_entry_text)
        .collect();
    let stale_entries: Vec<_> = expected_entries
        .difference(&actual_entries)
        .map(inventory_entry_text)
        .collect();
    bail!(
        "smoke-targets.tsv inventory drift; missing sources: [{}]; stale sources: [{}]; missing target rows: [{}]; stale target rows: [{}]",
        missing_sources.join(", "),
        stale_sources.join(", "),
        missing_entries.join(", "),
        stale_entries.join(", ")
    )
}

fn target_selector(target: &Target) -> (String, String) {
    if target.kind == "lib" {
        (target.kind.clone(), String::new())
    } else {
        (target.kind.clone(), target.name.clone())
    }
}

fn target_groups(rows: &[ManifestRow], targets: &[Target]) -> Result<Vec<Vec<Target>>> {
    let available: HashMap<(&str, &str, &str), &Target> = targets
        .iter()
        .filter(|target| target.available && target.testable)
        .map(|target| {
            (
                (
                    target.package.as_str(),
                    target.kind.as_str(),
                    target.name.as_str(),
                ),
                target,
            )
        })
        .collect();
    let mut groups: Vec<Vec<Target>> = Vec::new();
    let mut group_indexes = HashMap::new();
    let mut seen = HashSet::new();
    for row in rows {
        let identity = (
            row.package.as_str(),
            row.kind.as_str(),
            row.target_name.as_str(),
        );
        let Some(target) = available.get(&identity) else {
            continue;
        };
        if !seen.insert(identity) {
            continue;
        }
        let selector = target_selector(target);
        let index = *group_indexes.entry(selector).or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[index].push((*target).clone());
    }
    reject_ineligible_collisions(&groups, targets)?;
    Ok(groups)
}

fn has_ineligible_collision(group: &[Target], targets: &[Target]) -> bool {
    let selector = target_selector(&group[0]);
    targets.iter().any(|target| {
        target_selector(target) == selector && (!target.available || !target.testable)
    })
}

fn reject_ineligible_collisions(groups: &[Vec<Target>], targets: &[Target]) -> Result<()> {
    if let Some(group) = groups
        .iter()
        .find(|group| group[0].kind != "lib" && has_ineligible_collision(group, targets))
    {
        let (kind, name) = target_selector(&group[0]);
        bail!(
            "smoke selector {kind}:{name} collides with an unavailable or test=false target; rename one target so Cargo can preserve workspace features without selecting the disabled peer"
        );
    }
    Ok(())
}

fn uses_automatic_workspace_selection(group: &[Target], targets: &[Target]) -> bool {
    group[0].kind == "lib" && has_ineligible_collision(group, targets)
}

fn cargo_target_args(kind: &str, target_name: &str, full_workspace: bool) -> Vec<String> {
    let mut args = vec![
        "test".to_owned(),
        "--workspace".to_owned(),
        "--locked".to_owned(),
    ];
    if full_workspace {
        return args;
    }
    if kind == "lib" {
        args.push("--lib".to_owned());
    } else {
        args.extend([format!("--{kind}"), target_name.to_owned()]);
    }
    args
}

fn combined_cargo_target_args(groups: &[&[Target]]) -> Vec<String> {
    let mut args = cargo_target_args("", "", true);
    let selectors: BTreeSet<(String, String)> = groups
        .iter()
        .map(|group| target_selector(&group[0]))
        .collect();
    for (kind, name) in selectors {
        if kind == "lib" {
            args.push("--lib".to_owned());
        } else {
            args.extend([format!("--{kind}"), name]);
        }
    }
    args
}

fn target_key(target: &Target) -> TargetKey {
    (
        target.package_id.clone(),
        target.kind.clone(),
        target.name.clone(),
        target.source.clone(),
    )
}

fn linked_path(value: &str) -> PathBuf {
    const KINDS: &[&str] = &["all", "crate", "dependency", "framework", "native"];
    value
        .split_once('=')
        .filter(|(kind, _)| KINDS.contains(kind))
        .map_or_else(|| PathBuf::from(value), |(_, path)| PathBuf::from(path))
}

fn record_build_environment(message: &Value, artifacts: &mut CargoTestArtifacts) -> Result<()> {
    let package_id = message
        .get("package_id")
        .and_then(Value::as_str)
        .context("build-script result has no package ID")?;
    let out_dir = message
        .get("out_dir")
        .and_then(Value::as_str)
        .context("build-script result has no OUT_DIR")?;
    let mut values = BTreeMap::from([(OsString::from("OUT_DIR"), OsString::from(out_dir))]);
    for pair in message
        .get("env")
        .and_then(Value::as_array)
        .context("build-script result has no environment array")?
    {
        let pair = pair
            .as_array()
            .filter(|pair| pair.len() == 2)
            .context("build-script environment row is not a key/value pair")?;
        let key = pair[0]
            .as_str()
            .context("build-script environment key is not a string")?;
        let value = pair[1]
            .as_str()
            .context("build-script environment value is not a string")?;
        values.insert(OsString::from(key), OsString::from(value));
    }
    let linked_paths = message
        .get("linked_paths")
        .and_then(Value::as_array)
        .context("build-script result has no linked-path array")?
        .iter()
        .map(|path| {
            path.as_str()
                .map(linked_path)
                .context("build-script linked path is not a string")
        })
        .collect::<Result<Vec<_>>>()?;
    let environments = artifacts
        .build_environments
        .entry(package_id.to_owned())
        .or_default();
    environments.push(BuildEnvironment {
        out_dir: PathBuf::from(out_dir),
        values,
        linked_paths,
    });
    Ok(())
}

fn record_compiler_artifact(message: &Value, artifacts: &mut CargoTestArtifacts) -> Result<()> {
    let Some(executable) = message.get("executable").and_then(Value::as_str) else {
        return Ok(());
    };
    let raw_target = message
        .get("target")
        .context("compiler artifact has no target")?;
    let name = raw_target
        .get("name")
        .and_then(Value::as_str)
        .context("compiler artifact target has no name")?;
    let source = raw_target
        .get("src_path")
        .and_then(Value::as_str)
        .context("compiler artifact target has no source path")?;
    let raw_kinds: Vec<String> = raw_target
        .get("kind")
        .and_then(Value::as_array)
        .context("compiler artifact target has no kinds")?
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    let package_id = message
        .get("package_id")
        .and_then(Value::as_str)
        .context("compiler artifact has no package ID")?;
    let is_test = message.pointer("/profile/test").and_then(Value::as_bool) == Some(true);
    if !is_test && raw_kinds.iter().any(|kind| kind == "bin") {
        artifacts.binary_executables.insert(
            (package_id.to_owned(), name.to_owned()),
            PathBuf::from(executable),
        );
    }
    if is_test {
        for kind in canonical_target_kinds(&raw_kinds) {
            artifacts.executables.insert(
                (
                    package_id.to_owned(),
                    kind,
                    name.to_owned(),
                    PathBuf::from(source),
                ),
                PathBuf::from(executable),
            );
        }
    }
    Ok(())
}

fn cargo_test_executables(
    root: &Path,
    args: &[String],
    target: Option<&str>,
    extra_env: &BTreeMap<OsString, OsString>,
) -> Result<CargoTestArtifacts> {
    let mut command = Command::new("cargo");
    command.args(args);
    if let Some(target) = target {
        command.args(["--target", target]);
    }
    command
        .args(["--no-run", "--message-format=json"])
        .current_dir(root)
        .envs(extra_env);
    let output = command_output(&mut command)?;
    let stdout = String::from_utf8(output.stdout).context("Cargo JSON output is not UTF-8")?;
    let mut artifacts = CargoTestArtifacts::default();
    for line in stdout.lines() {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if message.get("reason").and_then(Value::as_str) == Some("build-script-executed") {
            record_build_environment(&message, &mut artifacts)?;
        } else if message.get("reason").and_then(Value::as_str) == Some("compiler-artifact") {
            record_compiler_artifact(&message, &mut artifacts)?;
        }
    }
    Ok(artifacts)
}

impl CargoTestArtifacts {
    fn executable(&self, target: &Target) -> Option<&PathBuf> {
        self.executables.get(&target_key(target))
    }

    fn runtime_environment(
        &self,
        target: &Target,
        executable: &Path,
    ) -> Result<RuntimeEnvironment> {
        let mut environment = BTreeMap::new();
        let profile_dir = executable
            .parent()
            .and_then(Path::parent)
            .context("test executable has no Cargo profile directory")?;
        if let Some(candidates) = self.build_environments.get(&target.package_id) {
            let matching = matching_build_environments(candidates, profile_dir);
            let Some(first) = matching.first() else {
                bail!(
                    "no build-script environment for {} matches {}",
                    target.package,
                    executable.display()
                );
            };
            if matching.iter().any(|candidate| *candidate != *first) {
                bail!(
                    "multiple build-script environments for {} match {}",
                    target.package,
                    executable.display()
                );
            }
            environment.extend(first.values.clone());
        }
        let variable = OsString::from(dynamic_library_variable());
        let preserves_dynamic_library_path = environment.contains_key(&variable);
        // Cargo's test-harness environment sorts qualifying link-search paths;
        // this is distinct from the linker-directive order seen by rustc.
        // The fixture below differentially checks this ordering against Cargo.
        let linked_paths: BTreeSet<PathBuf> = target
            .link_path_packages
            .iter()
            .flat_map(|package_id| {
                self.build_environments
                    .get(package_id)
                    .into_iter()
                    .flatten()
            })
            .filter(|environment| environment.out_dir.starts_with(profile_dir))
            .flat_map(|environment| {
                environment
                    .linked_paths
                    .iter()
                    .map(|path| lexically_normalise(path))
                    .filter(|path| path.starts_with(&self.target_directory))
            })
            .collect();
        if !linked_paths.is_empty() && !preserves_dynamic_library_path {
            environment.insert(
                variable,
                env::join_paths(linked_paths)
                    .context("joining build-script dynamic library search path")?,
            );
        }
        if matches!(target.kind.as_str(), "test" | "bench") {
            for ((package_id, name), path) in &self.binary_executables {
                if package_id == &target.package_id {
                    environment.insert(
                        OsString::from(format!("CARGO_BIN_EXE_{name}")),
                        path.as_os_str().to_os_string(),
                    );
                }
            }
        }
        Ok(RuntimeEnvironment {
            values: environment,
            preserves_dynamic_library_path,
        })
    }

    fn needs_explicit_target(&self, targets: &[Target]) -> Result<bool> {
        for target in targets {
            let Some(executable) = self.executable(target) else {
                continue;
            };
            let profile_dir = executable
                .parent()
                .and_then(Path::parent)
                .context("test executable has no Cargo profile directory")?;
            for package_id in &target.link_path_packages {
                let Some(candidates) = self.build_environments.get(package_id) else {
                    continue;
                };
                let matching: Vec<&BuildEnvironment> =
                    matching_build_environments(candidates, profile_dir)
                        .into_iter()
                        .filter(|environment| {
                            package_id == &target.package_id
                                || environment
                                    .linked_paths
                                    .iter()
                                    .map(|path| lexically_normalise(path))
                                    .any(|path| path.starts_with(&self.target_directory))
                        })
                        .collect();
                if let Some(first) = matching.first()
                    && matching.iter().any(|candidate| *candidate != *first)
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

fn matching_build_environments<'a>(
    candidates: &'a [BuildEnvironment],
    profile_dir: &Path,
) -> Vec<&'a BuildEnvironment> {
    candidates
        .iter()
        .filter(|candidate| candidate.out_dir.starts_with(profile_dir))
        .collect()
}

fn cargo_test_artifacts(
    root: &Path,
    args: &[String],
    targets: &[Target],
    runtime: &CargoRuntime,
    extra_env: &BTreeMap<OsString, OsString>,
) -> Result<CargoTestArtifacts> {
    let configured_target_directory = extra_env
        .get(OsStr::new("CARGO_TARGET_DIR"))
        .map_or_else(|| runtime.target_directory.clone(), PathBuf::from);
    let target_directory = if configured_target_directory.is_absolute() {
        configured_target_directory
    } else {
        root.join(configured_target_directory)
    };
    let target_directory = lexically_normalise(&target_directory);
    let mut artifacts = cargo_test_executables(root, args, None, extra_env)?;
    artifacts.target_directory.clone_from(&target_directory);
    if !artifacts.needs_explicit_target(targets)? {
        return Ok(artifacts);
    }
    println!("==> Cargo build-script contexts overlap; rebuilding for the resolved target");
    let mut rebuilt = cargo_test_executables(root, args, Some(&runtime.target), extra_env)?;
    rebuilt.target_directory = target_directory;
    if rebuilt.needs_explicit_target(targets)? {
        bail!("build-script environments remain ambiguous under explicit target selection");
    }
    rebuilt.explicit_target_context = true;
    Ok(rebuilt)
}

fn rust_runtime_library(root: &Path, target: &str, rustc: &PathAndArgs) -> Result<PathBuf> {
    let mut command = Command::new(&rustc.path);
    command
        .args(&rustc.args)
        .arg("--print")
        .arg("sysroot")
        .current_dir(root);
    let output = command_output(&mut command)?;
    let sysroot = String::from_utf8(output.stdout)
        .context("rustc sysroot output is not UTF-8")?
        .trim()
        .to_owned();
    Ok(PathBuf::from(sysroot)
        .join("lib")
        .join("rustlib")
        .join(target)
        .join("lib"))
}

fn dynamic_library_variable() -> &'static str {
    if cfg!(target_os = "windows") {
        "PATH"
    } else if cfg!(target_os = "macos") {
        "DYLD_FALLBACK_LIBRARY_PATH"
    } else {
        "LD_LIBRARY_PATH"
    }
}

fn cargo_profile_library_paths(executable: &Path) -> Result<(PathBuf, PathBuf)> {
    let artifact_directory = executable
        .parent()
        .context("test executable has no parent")?;
    let profile = if matches!(
        artifact_directory.file_name(),
        Some(name) if name == "deps" || name == "examples"
    ) {
        artifact_directory
            .parent()
            .context("Cargo artifact directory has no profile parent")?
    } else {
        artifact_directory
    };
    Ok((profile.to_path_buf(), profile.join("deps")))
}

fn harness_command(executable: &Path, context: &HarnessContext<'_>) -> Result<Command> {
    let variable = dynamic_library_variable();
    let joined = if context.preserves_dynamic_library_path {
        context
            .package_environment
            .get(OsStr::new(variable))
            .context("preserved build-script dynamic library path is missing")?
            .clone()
    } else {
        let mut paths = context
            .package_environment
            .get(OsStr::new(variable))
            .map_or_else(Vec::new, |value| env::split_paths(value).collect());
        let (profile, dependencies) = cargo_profile_library_paths(executable)?;
        paths.push(profile);
        paths.push(dependencies);
        paths.push(context.rust_runtime_library.to_path_buf());
        if let Some(existing) = env::var_os(variable) {
            paths.extend(env::split_paths(&existing));
        }
        env::join_paths(paths).context("joining dynamic library search path")?
    };
    let mut command = if let Some(runner) = context.runner {
        let mut command = Command::new(&runner.path);
        command.args(&runner.args).arg(executable);
        command
    } else {
        Command::new(executable)
    };
    for (name, _) in env::vars_os() {
        if name.to_string_lossy().starts_with("CARGO_BIN_EXE_") {
            command.env_remove(name);
        }
    }
    command
        .current_dir(context.package_root)
        .envs(context.package_environment)
        .env(variable, joined);
    Ok(command)
}

fn test_entries(output: &[u8]) -> Result<Vec<(String, String)>> {
    let stdout = String::from_utf8(output.to_vec()).context("libtest list is not UTF-8")?;
    Ok(stdout
        .lines()
        .filter_map(|line| {
            let (name, kind) = line.rsplit_once(": ")?;
            matches!(kind, "test" | "benchmark").then(|| (name.to_owned(), kind.to_owned()))
        })
        .collect())
}

fn selected_entries(entries: &[(String, String)], target: &Target) -> Vec<(String, String)> {
    if is_smoke_named_target(target) {
        return entries.to_vec();
    }
    entries
        .iter()
        .filter(|(name, _)| name.starts_with("smoke") || name.contains("::smoke"))
        .cloned()
        .collect()
}

fn substring_skips(
    entries: &[(String, String)],
    selected: &[(String, String)],
) -> Option<Vec<String>> {
    let selected_names: BTreeSet<&str> = selected.iter().map(|(name, _)| name.as_str()).collect();
    let candidates: BTreeSet<&str> = entries
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| name.contains("smoke"))
        .collect();
    let skips: Vec<String> = candidates
        .difference(&selected_names)
        .map(|name| (*name).to_owned())
        .collect();
    let effective: BTreeSet<&str> = candidates
        .iter()
        .copied()
        .filter(|name| !skips.iter().any(|skip| name.contains(skip)))
        .collect();
    (effective == selected_names).then_some(skips)
}

fn run_harness(
    target: &Target,
    executable: &Path,
    context: &HarnessContext<'_>,
    list_only: bool,
    quiet: bool,
) -> Result<Vec<(String, String)>> {
    let bench_mode = (target.kind == "bench").then_some("--test");
    let mut listing = harness_command(executable, context)?;
    listing.args(bench_mode).arg("--list");
    let entries = test_entries(&command_output(&mut listing)?.stdout)?;
    let selected = selected_entries(&entries, target);
    if list_only {
        for (name, kind) in &selected {
            println!("{name}: {kind}");
        }
        return Ok(selected);
    }

    if is_smoke_named_target(target) {
        let mut command = harness_command(executable, context)?;
        command.args(bench_mode);
        if quiet {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
            command_output(&mut command)?;
        } else {
            let status = command.status().context("running smoke test harness")?;
            if !status.success() {
                bail!("{} exited with {status}", executable.display());
            }
        }
        return Ok(selected);
    }

    if let Some(skips) = substring_skips(&entries, &selected) {
        let mut command = harness_command(executable, context)?;
        command.args(bench_mode).arg("smoke");
        for skip in skips {
            command.args(["--skip", &skip]);
        }
        if quiet {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
            command_output(&mut command)?;
        } else {
            let status = command.status().context("running smoke test harness")?;
            if !status.success() {
                bail!("{} exited with {status}", executable.display());
            }
        }
        return Ok(selected);
    }

    for (name, _) in &selected {
        let mut command = harness_command(executable, context)?;
        command.args(bench_mode).args([name, "--exact"]);
        if quiet {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
        }
        command_output(&mut command)?;
        if !quiet {
            println!("PASS {name}");
        }
    }
    Ok(selected)
}

fn execute_manifest(
    root: &Path,
    package_roots: &HashMap<String, PathBuf>,
    package_environments: &PackageEnvironments,
    rows: &[ManifestRow],
    targets: &[Target],
    runtime: &CargoRuntime,
    list_only: bool,
) -> Result<()> {
    let groups = target_groups(rows, targets)?;
    let rust_runtime_library = rust_runtime_library(root, &runtime.target, &runtime.rustc)?;
    let extra_env = BTreeMap::new();
    let explicit_groups: Vec<&[Target]> = groups
        .iter()
        .filter(|group| !uses_automatic_workspace_selection(group, targets))
        .map(Vec::as_slice)
        .collect();
    let automatic_groups: Vec<&[Target]> = groups
        .iter()
        .filter(|group| uses_automatic_workspace_selection(group, targets))
        .map(Vec::as_slice)
        .collect();
    let explicit_artifacts = if explicit_groups.is_empty() {
        CargoTestArtifacts::default()
    } else {
        let args = combined_cargo_target_args(&explicit_groups);
        println!("==> cargo test --workspace selected smoke targets --no-run");
        let selected_targets: Vec<Target> = explicit_groups
            .iter()
            .flat_map(|group| *group)
            .cloned()
            .collect();
        cargo_test_artifacts(root, &args, &selected_targets, runtime, &extra_env)?
    };
    let automatic_artifacts = if automatic_groups.is_empty() {
        CargoTestArtifacts::default()
    } else {
        let args = cargo_target_args("", "", true);
        println!("==> cargo test --workspace automatic library targets --no-run");
        let selected_targets: Vec<Target> = automatic_groups
            .iter()
            .flat_map(|group| *group)
            .cloned()
            .collect();
        cargo_test_artifacts(root, &args, &selected_targets, runtime, &extra_env)?
    };

    for group in groups {
        let artifacts = if uses_automatic_workspace_selection(&group, targets) {
            &automatic_artifacts
        } else {
            &explicit_artifacts
        };
        for target in &group {
            let executable = artifacts.executable(target).with_context(|| {
                format!(
                    "Cargo did not produce a test executable for {} {}:{}",
                    target.package, target.kind, target.name
                )
            })?;
            let package_root = package_roots
                .get(&target.package)
                .with_context(|| format!("missing package root for {}", target.package))?;
            let package_environment = package_environments
                .get(&target.package)
                .with_context(|| format!("missing package environment for {}", target.package))?;
            let runtime_environment = artifacts.runtime_environment(target, executable)?;
            let package_environment =
                harness_environment(package_environment, &runtime_environment.values);
            let context = HarnessContext {
                package_root,
                package_environment: &package_environment,
                rust_runtime_library: &rust_runtime_library,
                runner: runtime.runner.as_ref(),
                preserves_dynamic_library_path: runtime_environment.preserves_dynamic_library_path,
            };
            run_harness(target, executable, &context, list_only, false)?;
        }
    }
    Ok(())
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

fn create_fixture(mut next_root: impl FnMut() -> PathBuf) -> Result<Fixture> {
    loop {
        let root = next_root();
        match fs::create_dir(&root) {
            Ok(()) => return Ok(Fixture { root }),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("creating smoke fixture {}", root.display()));
            }
        }
    }
}

impl Fixture {
    fn new() -> Result<Self> {
        create_fixture(|| {
            let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            env::temp_dir().join(format!(
                "tcl-lsp-smoke-targets-{}-{serial}",
                std::process::id()
            ))
        })
    }

    fn write(&self, relative: &str, contents: &str) -> Result<()> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))
    }

    #[cfg(unix)]
    fn write_executable(&self, relative: &str, contents: &str) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        self.write(relative, contents)?;
        let path = self.root.join(relative);
        let mut permissions = fs::metadata(&path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions)
            .with_context(|| format!("making {} executable", path.display()))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(unix)]
fn rustc_probe_config(fixture: &Fixture) -> Result<String> {
    let probe = fixture.root.join("rustc-probe.sh");
    let marker = fixture.root.join("rustc-sysroot-queried");
    let wrapper = fixture.root.join("rustc-wrapper.sh");
    let wrapper_marker = fixture.root.join("rustc-wrapper-queried");
    let workspace_wrapper = fixture.root.join("rustc-workspace-wrapper.sh");
    let workspace_wrapper_marker = fixture.root.join("rustc-workspace-wrapper-queried");
    fixture.write_executable(
        "rustc-probe.sh",
        &format!(
            "#!/bin/sh\nif [ \"$1\" = --print ] && [ \"$2\" = sysroot ]; then : > \"{}\"; fi\nexec rustc \"$@\"\n",
            marker.display()
        ),
    )?;
    fixture.write_executable(
        "rustc-wrapper.sh",
        &format!(
            "#!/bin/sh\n: > \"{}\"\nexec \"$@\"\n",
            wrapper_marker.display()
        ),
    )?;
    fixture.write_executable(
        "rustc-workspace-wrapper.sh",
        &format!(
            "#!/bin/sh\n: > \"{}\"\nexec \"$@\"\n",
            workspace_wrapper_marker.display()
        ),
    )?;
    Ok(format!(
        "rustc = {}\nrustc-wrapper = {}\nrustc-workspace-wrapper = {}\n",
        serde_json::to_string(&probe)?,
        serde_json::to_string(&wrapper)?,
        serde_json::to_string(&workspace_wrapper)?,
    ))
}

#[cfg(not(unix))]
fn rustc_probe_config(_fixture: &Fixture) -> Result<String> {
    Ok(String::new())
}

fn reset_rustc_probe(fixture: &Fixture) -> Result<()> {
    for name in [
        "rustc-sysroot-queried",
        "rustc-wrapper-queried",
        "rustc-workspace-wrapper-queried",
    ] {
        let marker = fixture.root.join(name);
        if marker.exists() {
            fs::remove_file(&marker).with_context(|| format!("removing {}", marker.display()))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn verify_rustc_probe(fixture: &Fixture) -> Result<()> {
    for (name, context) in [
        ("rustc-sysroot-queried", "configured Cargo rustc"),
        ("rustc-wrapper-queried", "configured Cargo rustc wrapper"),
        (
            "rustc-workspace-wrapper-queried",
            "configured Cargo workspace rustc wrapper",
        ),
    ] {
        if !fixture.root.join(name).is_file() {
            bail!("{context} was not used for the sysroot query");
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_rustc_probe(_fixture: &Fixture) -> Result<()> {
    Ok(())
}

fn fixture_target<'a>(targets: &'a [Target], package: &str, name: &str) -> Result<&'a Target> {
    targets
        .iter()
        .find(|target| target.package == package && target.name == name)
        .with_context(|| format!("fixture target {package}:{name} is missing"))
}

const CARGO_FIXTURE_FILES: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        r#"[workspace]
members = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r"]
resolver = "2"
"#,
    ),
    (
        "a/Cargo.toml",
        r#"[package]
name = "a"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["rlib"]

[features]
build_only = []
normal = []

[[bin]]
name = "build_smoke"
path = "src/build.rs"
required-features = ["build_only"]

[[bin]]
name = "normal_smoke"
path = "src/normal.rs"
required-features = ["normal"]
"#,
    ),
    (
        "a/src/lib.rs",
        "pub fn value() -> bool { true }\npub fn normal_is_enabled() -> bool { cfg!(feature = \"normal\") }\n",
    ),
    ("a/src/build.rs", "fn main() {}\n"),
    ("a/src/normal.rs", "fn main() {}\n"),
    (
        "b/Cargo.toml",
        r#"[package]
name = "b"
version = "0.1.0"
edition = "2024"

[build-dependencies]
a = { path = "../a", features = ["build_only"] }
e = { path = "../e", features = ["host_only"] }
"#,
    ),
    ("b/src/lib.rs", "pub fn value() -> bool { true }\n"),
    ("b/build.rs", "fn main() { assert!(a::value()); }\n"),
    (
        "c/Cargo.toml",
        r#"[package]
name = "c"
version = "0.1.0"
edition = "2024"

[dependencies]
a = { path = "../a", features = ["normal"] }
"#,
    ),
    ("c/src/lib.rs", "pub fn value() -> bool { a::value() }\n"),
    (
        "c/build.rs",
        "fn main() { std::thread::sleep(std::time::Duration::from_millis(250)); let native = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()).join(\"native\"); std::fs::create_dir_all(&native).unwrap(); println!(\"cargo::rustc-link-search=native={}\", native.display()); }\n",
    ),
    (
        "d/Cargo.toml",
        r#"[package]
name = "d"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true
"#,
    ),
    ("d/src/lib.rs", ""),
    (
        "e/Cargo.toml",
        r#"[package]
name = "e"
version = "0.1.0"
edition = "2024"
links = "fixture-e"

[dependencies]
c = { path = "../c" }
f = { path = "../f" }

[build-dependencies]
r = { path = "../r" }

[features]
host_only = []
"#,
    ),
    (
        "e/src/lib.rs",
        concat!(
            "#[test]\nfn ",
            "smoke_dependency() { assert!(f::normal_is_enabled()); assert_eq!(std::env::var(\"CARGO_PKG_NAME\").as_deref(), Ok(\"e\")); assert_eq!(std::env::var(\"CARGO_PKG_VERSION\").as_deref(), Ok(\"0.1.0\")); assert_eq!(std::env::var(\"CARGO_PKG_DESCRIPTION\").as_deref(), Ok(\"\")); assert_eq!(std::env::var(\"CARGO_MANIFEST_LINKS\").as_deref(), Ok(\"inherited\")); assert!(std::path::Path::new(&std::env::var(\"CARGO_MANIFEST_DIR\").unwrap()).ends_with(\"e\")); assert!(std::path::Path::new(&std::env::var(\"CARGO_MANIFEST_PATH\").unwrap()).ends_with(std::path::Path::new(\"e/Cargo.toml\"))); assert_eq!(std::env::var(\"TCL_LSP_BUILD_ENV\").as_deref(), Ok(\"owned\")); let out_dir = std::env::var(\"OUT_DIR\").unwrap(); assert!(out_dir.replace(char::from(92), \"/\").contains(\"/build/e-\")); let variable = if cfg!(windows) { \"PATH\" } else if cfg!(target_os = \"macos\") { \"DYLD_FALLBACK_LIBRARY_PATH\" } else { \"LD_LIBRARY_PATH\" }; let paths = std::env::split_paths(&std::env::var_os(variable).unwrap()).collect::<Vec<_>>(); assert!(paths.iter().any(|path| path == &std::path::Path::new(&out_dir).join(\"native\"))); assert!(paths.iter().any(|path| path.ends_with(\"shared-native\"))); assert!(!paths.iter().any(|path| path.ends_with(\"outside-native\"))); assert!(paths.iter().any(|path| { let path = path.to_string_lossy().replace(char::from(92), \"/\"); path.contains(\"/build/f-\") && path.ends_with(\"/out/native\") })); assert!(paths.iter().any(|path| { let path = path.to_string_lossy().replace(char::from(92), \"/\"); path.contains(\"/build/r-\") && path.ends_with(\"/out/build-only-native\") })); assert!(paths.iter().any(|path| path == std::path::Path::new(&std::env::var(\"TCL_LSP_EXPECTED_SYSROOT_LIB\").unwrap()))); if let Some(record) = std::env::var_os(\"TCL_LSP_RECORD_CARGO_PATH\") { let text = paths.iter().map(|path| path.to_string_lossy()).collect::<Vec<_>>().join(\"\\n\"); std::fs::write(record, text).unwrap(); } if !cfg!(windows) { assert_eq!(std::env::var(\"TCL_LSP_SMOKE_RUNNER\").as_deref(), Ok(\"used\")); } }\n\n",
            "#[test]\nfn long_smoke_dependency() { panic!(\"deep test must not run\"); }\n\n",
            "#[test]\nfn deep() { panic!(\"deep test must not run\"); }\n",
        ),
    ),
    (
        "e/build.rs",
        "fn main() { let out = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()); let native = out.join(\"native\"); let later = out.join(\"aaa-native\"); let shared = out.parent().unwrap().parent().unwrap().parent().unwrap().join(\"shared-native\"); let escaped = out.ancestors().nth(5).unwrap().join(\"../outside-native\"); std::fs::create_dir_all(&native).unwrap(); std::fs::create_dir_all(&later).unwrap(); std::fs::create_dir_all(&shared).unwrap(); std::fs::create_dir_all(&escaped).unwrap(); let context = if std::env::var_os(\"CARGO_FEATURE_HOST_ONLY\").is_some() { \"host\" } else { \"owned\" }; println!(\"cargo::rustc-env=TCL_LSP_BUILD_ENV={context}\"); println!(\"cargo::rustc-env=CARGO_PKG_NAME=overridden\"); println!(\"cargo::rustc-env=CARGO_PKG_VERSION=9.9.9\"); println!(\"cargo::rustc-env=CARGO_MANIFEST_DIR=overridden\"); println!(\"cargo::rustc-env=CARGO_MANIFEST_PATH=overridden\"); println!(\"cargo::rustc-link-search=native={}\", native.display()); println!(\"cargo::rustc-link-search=native={}\", later.display()); println!(\"cargo::rustc-link-search=native={}\", shared.display()); println!(\"cargo::rustc-link-search=native={}\", escaped.display()); }\n",
    ),
    (
        "f/Cargo.toml",
        r#"[package]
name = "f"
version = "0.1.0"
edition = "2024"

[dependencies]
a = { path = "../a" }
"#,
    ),
    (
        "f/src/lib.rs",
        "pub fn normal_is_enabled() -> bool { a::normal_is_enabled() }\n",
    ),
    (
        "f/build.rs",
        "fn main() { let native = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()).join(\"native\"); std::fs::create_dir_all(&native).unwrap(); println!(\"cargo::rustc-link-search=native={}\", native.display()); }\n",
    ),
    (
        "g/Cargo.toml",
        r#"[package]
name = "g"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]
"#,
    ),
    ("g/src/lib.rs", "pub fn value() -> bool { true }\n"),
    (
        "h/Cargo.toml",
        r#"[package]
name = "h"
version = "0.1.0"
edition = "2024"

[lib]
name = "library_smoke"

[[bin]]
name = "library-helper"
path = "src/main.rs"
"#,
    ),
    (
        "h/src/lib.rs",
        r#"#[test]
fn ordinary_library_test() {
    assert!(std::env::current_dir().unwrap().ends_with("h"));
    assert!(std::env::var_os("CARGO_BIN_EXE_library-helper").is_none());
}
"#,
    ),
    ("h/src/main.rs", "fn main() {}\n"),
    ("h/tests/deep.rs", "#[test]\nfn ordinary() {}\n"),
    (
        "i/Cargo.toml",
        r#"[package]
name = "i"
version = "0.1.0"
edition = "2024"

[[example]]
name = "demo_smoke"
path = "examples/demo.rs"
test = true
"#,
    ),
    ("i/src/lib.rs", ""),
    (
        "i/examples/demo.rs",
        r#"#[test]
fn ordinary_example_test() {
    assert!(std::env::current_dir().unwrap().ends_with("i"));
}
fn main() {}
"#,
    ),
    (
        "j/Cargo.toml",
        r#"[package]
name = "j"
version = "0.1.0"
edition = "2024"

[[example]]
name = "demo_smoke"
path = "examples/demo.rs"
test = false

[[bench]]
name = "bench_smoke"
path = "benches/bench.rs"
test = false
"#,
    ),
    ("j/src/lib.rs", ""),
    (
        "j/examples/demo.rs",
        "#[cfg(test)]\ncompile_error!(\"test=false example must not compile as a test\");\nfn main() {}\n",
    ),
    (
        "j/benches/bench.rs",
        "#[cfg(test)]\ncompile_error!(\"test=false benchmark must not compile as a test\");\nfn main() {}\n",
    ),
    (
        "k/Cargo.toml",
        r#"[package]
name = "k"
version = "0.1.0"
edition = "2024"

[[bench]]
name = "bench_smoke"
path = "benches/bench.rs"
"#,
    ),
    ("k/src/lib.rs", ""),
    (
        "k/benches/bench.rs",
        "#[test]\nfn ordinary_bench_test() {}\n",
    ),
    (
        "l/Cargo.toml",
        r#"[package]
name = "l"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "helper-tool"
path = "src/main.rs"
"#,
    ),
    ("l/src/main.rs", "fn main() {}\n"),
    (
        "l/tests/smoke.rs",
        "#[test]\nfn smoke_binary_path() { assert!(std::path::Path::new(&std::env::var(\"CARGO_BIN_EXE_helper-tool\").unwrap()).is_file()); assert_eq!(std::env::var(\"CARGO_MANIFEST_LINKS\").as_deref(), Ok(\"inherited\")); }\n",
    ),
    (
        "m/Cargo.toml",
        r#"[package]
name = "m"
version = "0.1.0"
edition = "2024"

[[bench]]
name = "unique_smoke"
path = "benches/bench.rs"
"#,
    ),
    ("m/src/lib.rs", ""),
    (
        "m/benches/bench.rs",
        "#[test]\nfn ordinary_bench_test() {}\n",
    ),
    (
        "n/Cargo.toml",
        r#"[package]
name = "n"
version = "0.1.0"
edition = "2024"

[lib]
test = false
"#,
    ),
    (
        "n/src/lib.rs",
        "#[cfg(test)]\ncompile_error!(\"test=false library must not compile as a test\");\n",
    ),
    (
        "o/Cargo.toml",
        r#"[package]
name = "o"
version = "0.1.0"
edition = "2024"

[lib]
name = "common"
path = "../shared.rs"
"#,
    ),
    (
        "p/Cargo.toml",
        r#"[package]
name = "p"
version = "0.1.0"
edition = "2024"

[lib]
name = "common"
path = "../shared.rs"
"#,
    ),
    ("shared.rs", "#[test]\nfn smoke_shared_source() {}\n"),
    (
        "q/Cargo.toml",
        r#"[package]
name = "q"
version = "0.1.0"
edition = "2024"
"#,
    ),
    (
        "q/src/lib.rs",
        "#[test]\nfn smoke_dynamic_override() { assert_eq!(std::env::var(\"CARGO_MANIFEST_LINKS\").as_deref(), Ok(\"emitted\")); let out = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()); let variable = if cfg!(windows) { \"PATH\" } else if cfg!(target_os = \"macos\") { \"DYLD_FALLBACK_LIBRARY_PATH\" } else { \"LD_LIBRARY_PATH\" }; let paths = std::env::split_paths(&std::env::var_os(variable).unwrap()).collect::<Vec<_>>(); assert_eq!(paths.first(), Some(&out.join(\"custom\"))); assert!(!paths.contains(&out.join(\"linked\"))); if let Some(record) = std::env::var_os(\"TCL_LSP_RECORD_OVERRIDE_PATH\") { let text = paths.iter().map(|path| path.to_string_lossy()).collect::<Vec<_>>().join(\"\\n\"); std::fs::write(record, text).unwrap(); } }\n",
    ),
    (
        "q/build.rs",
        "fn main() { let out = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()); let custom = out.join(\"custom\"); let linked = out.join(\"linked\"); std::fs::create_dir_all(&custom).unwrap(); std::fs::create_dir_all(&linked).unwrap(); let target = std::env::var(\"CARGO_CFG_TARGET_OS\").unwrap(); let variable = if target == \"windows\" { \"PATH\" } else if target == \"macos\" { \"DYLD_FALLBACK_LIBRARY_PATH\" } else { \"LD_LIBRARY_PATH\" }; let mut paths = vec![custom]; if let Some(existing) = std::env::var_os(variable) { paths.extend(std::env::split_paths(&existing)); } let override_value = std::env::join_paths(paths).unwrap(); println!(\"cargo::rustc-link-search=native={}\", linked.display()); println!(\"cargo::rustc-env={variable}={}\", override_value.to_string_lossy()); println!(\"cargo::rustc-env=CARGO_MANIFEST_LINKS=emitted\"); }\n",
    ),
    (
        "r/Cargo.toml",
        r#"[package]
name = "r"
version = "0.1.0"
edition = "2024"
"#,
    ),
    ("r/src/lib.rs", "pub fn value() -> bool { true }\n"),
    (
        "r/build.rs",
        "fn main() { let native = std::path::PathBuf::from(std::env::var_os(\"OUT_DIR\").unwrap()).join(\"build-only-native\"); std::fs::create_dir_all(&native).unwrap(); println!(\"cargo::rustc-link-search=native={}\", native.display()); }\n",
    ),
];

fn verify_fixture_metadata(fixture: &Fixture, targets: &[Target]) -> Result<()> {
    let build_smoke = fixture_target(targets, "a", "build_smoke")?;
    let normal_smoke = fixture_target(targets, "a", "normal_smoke")?;
    if build_smoke.available || !normal_smoke.available {
        bail!("resolver-v2 target feature-context self-test failed");
    }
    if fixture_target(targets, "d", "d")?.kind != "lib"
        || fixture_target(targets, "g", "g")?.kind != "lib"
    {
        bail!("Cargo library-kind canonicalisation self-test failed");
    }
    let mut inventory = SmokeInventory::default();
    scan_target_smoke_sources(&fixture.root, build_smoke, &mut inventory)?;
    let build_source = "a/src/build.rs".to_owned();
    if !inventory.sources.contains(&build_source)
        || !inventory
            .owners
            .get(&build_source)
            .is_some_and(|owners| owners.contains(&target_identity(build_smoke)))
    {
        bail!("unavailable target inventory self-test failed");
    }
    let unavailable_row = ManifestRow {
        source_text: build_source,
        package: "a".to_owned(),
        kind: "bin".to_owned(),
        target_name: "build_smoke".to_owned(),
    };
    if !target_groups(&[unavailable_row], targets)?.is_empty() {
        bail!("unavailable target execution self-test failed");
    }
    Ok(())
}

fn verify_fixture_runtime_environment(
    environment: &BTreeMap<OsString, OsString>,
    executable: &Path,
    context: &HarnessContext<'_>,
) -> Result<Vec<&'static str>> {
    let variable = dynamic_library_variable();
    let build_paths: Vec<PathBuf> = environment
        .get(OsStr::new(variable))
        .map(|value| env::split_paths(value).collect())
        .context("fixture package e has no build-script runtime paths")?;
    let own_first = build_paths.iter().position(|path| {
        let path = path.to_string_lossy().replace('\\', "/");
        path.contains("/build/e-") && path.ends_with("/out/native")
    });
    let own_second = build_paths.iter().position(|path| {
        let path = path.to_string_lossy().replace('\\', "/");
        path.contains("/build/e-") && path.ends_with("/out/aaa-native")
    });
    if !matches!((own_first, own_second), (Some(first), Some(second)) if second < first) {
        bail!("build-script linked-path ordering self-test failed");
    }
    let linked_path_order = fixture_linked_path_order(&build_paths);
    if linked_path_order
        != [
            "c-native",
            "e-aaa-native",
            "e-native",
            "f-native",
            "r-build-only-native",
            "e-shared-native",
        ]
    {
        bail!("Cargo-compatible linked-path ordering self-test failed");
    }
    let command = harness_command(executable, context)?;
    let command_paths: Vec<PathBuf> = command
        .get_envs()
        .find_map(|(name, value)| (name == variable).then_some(value).flatten())
        .map(env::split_paths)
        .context("fixture harness has no dynamic-library environment")?
        .collect();
    if !command_paths.starts_with(&build_paths) {
        bail!("build-script runtime paths were not prepended to harness paths");
    }
    let (profile, dependencies) = cargo_profile_library_paths(executable)?;
    if command_paths.get(build_paths.len()) != Some(&profile)
        || command_paths.get(build_paths.len() + 1) != Some(&dependencies)
    {
        bail!("Cargo profile runtime path ordering self-test failed");
    }
    let mut expected_paths = build_paths;
    expected_paths.extend([
        profile,
        dependencies,
        context.rust_runtime_library.to_path_buf(),
    ]);
    if let Some(existing) = env::var_os(variable) {
        expected_paths.extend(env::split_paths(&existing));
    }
    if command_paths != expected_paths {
        bail!(
            "direct harness runtime paths {command_paths:?} differ from Cargo's synthesized paths {expected_paths:?}"
        );
    }
    let manifest_links = command
        .get_envs()
        .find_map(|(name, value)| (name == "CARGO_MANIFEST_LINKS").then_some(value));
    if manifest_links.is_some() {
        bail!("inherited package links environment self-test failed");
    }
    Ok(linked_path_order)
}

fn fixture_linked_path_order(paths: &[PathBuf]) -> Vec<&'static str> {
    paths
        .iter()
        .filter_map(|path| {
            let path = path.to_string_lossy().replace('\\', "/");
            if path.contains("/build/e-") && path.ends_with("/out/native") {
                Some("e-native")
            } else if path.contains("/build/e-") && path.ends_with("/out/aaa-native") {
                Some("e-aaa-native")
            } else if path.contains("/build/c-") && path.ends_with("/out/native") {
                Some("c-native")
            } else if path.contains("/build/f-") && path.ends_with("/out/native") {
                Some("f-native")
            } else if path.contains("/build/r-") && path.ends_with("/out/build-only-native") {
                Some("r-build-only-native")
            } else if path.ends_with("/shared-native") {
                Some("e-shared-native")
            } else {
                None
            }
        })
        .collect()
}

fn verify_cargo_runtime_path_order(
    fixture: &Fixture,
    runtime: &CargoRuntime,
    extra_env: &BTreeMap<OsString, OsString>,
    runtime_library: &Path,
    direct_path_order: &[&str],
) -> Result<()> {
    let cargo_path_record = fixture.root.join("cargo-runtime-paths");
    let mut cargo_args = cargo_target_args("", "", true);
    cargo_args.extend([
        "--target".to_owned(),
        runtime.target.clone(),
        "--exclude".to_owned(),
        "n".to_owned(),
        "smoke_dependency".to_owned(),
        "--".to_owned(),
        "--exact".to_owned(),
    ]);
    let mut cargo_harness = Command::new("cargo");
    cargo_harness
        .args(cargo_args)
        .current_dir(&fixture.root)
        .envs(extra_env)
        .env("TCL_LSP_EXPECTED_SYSROOT_LIB", runtime_library)
        .env("TCL_LSP_RECORD_CARGO_PATH", &cargo_path_record);
    command_output(&mut cargo_harness)?;
    let cargo_paths: Vec<PathBuf> = fs::read_to_string(&cargo_path_record)
        .context("reading Cargo fixture runtime paths")?
        .lines()
        .map(PathBuf::from)
        .collect();
    let cargo_path_order = fixture_linked_path_order(&cargo_paths);
    if cargo_path_order != direct_path_order {
        bail!(
            "direct harness build-script path order {direct_path_order:?} differs from Cargo {cargo_path_order:?}"
        );
    }
    Ok(())
}

fn fixture_override_path_order(paths: &[PathBuf]) -> Vec<&'static str> {
    paths
        .iter()
        .filter_map(|path| {
            let path = path.to_string_lossy().replace('\\', "/");
            if path.contains("/build/q-") && path.ends_with("/out/custom") {
                Some("q-custom")
            } else if path.contains("/build/q-") && path.ends_with("/out/linked") {
                Some("q-linked")
            } else {
                None
            }
        })
        .collect()
}

fn read_fixture_runtime_paths(path: &Path) -> Result<Vec<PathBuf>> {
    Ok(fs::read_to_string(path)
        .with_context(|| format!("reading fixture runtime paths from {}", path.display()))?
        .lines()
        .map(PathBuf::from)
        .collect())
}

fn verify_fixture_dynamic_override(
    fixture: &Fixture,
    package_roots: &HashMap<String, PathBuf>,
    package_environments: &PackageEnvironments,
    targets: &[Target],
    runtime: &CargoRuntime,
    extra_env: &BTreeMap<OsString, OsString>,
    runtime_library: &Path,
) -> Result<()> {
    let target = fixture_target(targets, "q", "q")?;
    let artifacts = cargo_test_artifacts(
        &fixture.root,
        &cargo_target_args("", "", true),
        std::slice::from_ref(target),
        runtime,
        extra_env,
    )?;
    let executable = artifacts
        .executable(target)
        .context("fixture package q has no library executable")?;
    let direct_record = fixture.root.join("direct-override-paths");
    let runtime_environment = artifacts.runtime_environment(target, executable)?;
    let mut environment =
        harness_environment(&package_environments["q"], &runtime_environment.values);
    environment.insert(
        OsString::from("TCL_LSP_RECORD_OVERRIDE_PATH"),
        direct_record.as_os_str().to_os_string(),
    );
    let context = HarnessContext {
        package_root: &package_roots["q"],
        package_environment: &environment,
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
        preserves_dynamic_library_path: runtime_environment.preserves_dynamic_library_path,
    };
    run_harness(target, executable, &context, false, true)?;

    let cargo_record = fixture.root.join("cargo-override-paths");
    let mut cargo_args = cargo_target_args("", "", true);
    cargo_args.extend([
        "--exclude".to_owned(),
        "n".to_owned(),
        "smoke_dynamic_override".to_owned(),
        "--".to_owned(),
        "--exact".to_owned(),
    ]);
    let mut cargo_harness = Command::new("cargo");
    cargo_harness
        .args(cargo_args)
        .current_dir(&fixture.root)
        .envs(extra_env)
        .env("TCL_LSP_RECORD_OVERRIDE_PATH", &cargo_record);
    command_output(&mut cargo_harness)?;

    let direct_paths = read_fixture_runtime_paths(&direct_record)?;
    let cargo_paths = read_fixture_runtime_paths(&cargo_record)?;
    let direct = fixture_override_path_order(&direct_paths);
    let cargo = fixture_override_path_order(&cargo_paths);
    if direct != ["q-custom"] || cargo != direct || cargo_paths != direct_paths {
        bail!(
            "direct harness dynamic-path override {direct:?} differs from Cargo {cargo:?}: direct {direct_paths:?}, Cargo {cargo_paths:?}"
        );
    }
    Ok(())
}

fn verify_package_narrowing_loses_features(
    fixture: &Fixture,
    extra_env: &BTreeMap<OsString, OsString>,
) -> Result<()> {
    let mut targeted = Command::new("cargo");
    targeted
        .args([
            "test",
            "--locked",
            "-p",
            "e",
            "--lib",
            "smoke_dependency",
            "--",
            "--exact",
        ])
        .current_dir(&fixture.root)
        .envs(extra_env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if targeted.output()?.status.success() {
        bail!("package-narrowed feature-loss self-test unexpectedly passed");
    }
    Ok(())
}

fn verify_fixture_libraries(
    fixture: &Fixture,
    package_roots: &HashMap<String, PathBuf>,
    package_environments: &PackageEnvironments,
    targets: &[Target],
    runtime: &CargoRuntime,
    extra_env: &BTreeMap<OsString, OsString>,
    runtime_library: &Path,
) -> Result<()> {
    verify_package_narrowing_loses_features(fixture, extra_env)?;

    let library_args = cargo_target_args("", "", true);
    let e_target = fixture_target(targets, "e", "e")?;
    let h_target = fixture_target(targets, "h", "library_smoke")?;
    let o_target = fixture_target(targets, "o", "common")?;
    let p_target = fixture_target(targets, "p", "common")?;
    if !has_ineligible_collision(std::slice::from_ref(h_target), targets)
        || reject_ineligible_collisions(&[vec![h_target.clone()]], targets).is_err()
    {
        bail!("disabled-library collision self-test failed");
    }
    let library_artifacts = cargo_test_artifacts(
        &fixture.root,
        &library_args,
        &[
            e_target.clone(),
            h_target.clone(),
            o_target.clone(),
            p_target.clone(),
        ],
        runtime,
        extra_env,
    )?;
    if !library_artifacts.explicit_target_context {
        bail!("overlapping host/target build-script context self-test failed");
    }
    let o_executable = library_artifacts
        .executable(o_target)
        .context("fixture package o has no shared-source library executable")?;
    let p_executable = library_artifacts
        .executable(p_target)
        .context("fixture package p has no shared-source library executable")?;
    if o_executable == p_executable {
        bail!("package-specific compiler-artifact identity self-test failed");
    }
    let e_executable = library_artifacts
        .executable(e_target)
        .context("fixture package e has no library executable")?;
    let e_runtime_environment = library_artifacts.runtime_environment(e_target, e_executable)?;
    let mut e_environment =
        harness_environment(&package_environments["e"], &e_runtime_environment.values);
    e_environment.insert(
        OsString::from("TCL_LSP_EXPECTED_SYSROOT_LIB"),
        runtime_library.as_os_str().to_os_string(),
    );
    let e_context = HarnessContext {
        package_root: &package_roots["e"],
        package_environment: &e_environment,
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
        preserves_dynamic_library_path: e_runtime_environment.preserves_dynamic_library_path,
    };
    let direct_path_order =
        verify_fixture_runtime_environment(&e_environment, e_executable, &e_context)?;
    verify_cargo_runtime_path_order(
        fixture,
        runtime,
        extra_env,
        runtime_library,
        &direct_path_order,
    )?;
    verify_fixture_dynamic_override(
        fixture,
        package_roots,
        package_environments,
        targets,
        runtime,
        extra_env,
        runtime_library,
    )?;
    let e_selected = run_harness(e_target, e_executable, &e_context, false, true)?;
    if e_selected != [("smoke_dependency".to_owned(), "test".to_owned())] {
        bail!("exact nextest-name selection self-test failed");
    }
    let h_executable = library_artifacts
        .executable(h_target)
        .context("fixture package h has no library executable")?;
    let h_context = HarnessContext {
        package_root: &package_roots["h"],
        package_environment: &package_environments["h"],
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
        preserves_dynamic_library_path: false,
    };
    let h_selected = run_harness(h_target, h_executable, &h_context, false, true)?;
    if h_selected != [("ordinary_library_test".to_owned(), "test".to_owned())] {
        bail!("smoke-named library artifact self-test failed");
    }
    Ok(())
}

fn verify_fixture_collision_and_bench(
    fixture: &Fixture,
    package_roots: &HashMap<String, PathBuf>,
    package_environments: &PackageEnvironments,
    targets: &[Target],
    runtime: &CargoRuntime,
    extra_env: &BTreeMap<OsString, OsString>,
    runtime_library: &Path,
) -> Result<()> {
    let demo_target = fixture_target(targets, "i", "demo_smoke")?;
    if !has_ineligible_collision(std::slice::from_ref(demo_target), targets) {
        bail!("ineligible same-named target collision self-test failed");
    }
    let bench_target = fixture_target(targets, "k", "bench_smoke")?;
    if !has_ineligible_collision(std::slice::from_ref(bench_target), targets) {
        bail!("ineligible same-named benchmark collision self-test failed");
    }
    if reject_ineligible_collisions(
        &[vec![demo_target.clone()], vec![bench_target.clone()]],
        targets,
    )
    .is_ok()
    {
        bail!("ineligible selector collision contract self-test failed");
    }
    let unique_bench = fixture_target(targets, "m", "unique_smoke")?;
    let bench_args = cargo_target_args("bench", "unique_smoke", false);
    let bench_artifacts = cargo_test_artifacts(
        &fixture.root,
        &bench_args,
        std::slice::from_ref(unique_bench),
        runtime,
        extra_env,
    )?;
    let bench_context = HarnessContext {
        package_root: &package_roots["m"],
        package_environment: &package_environments["m"],
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
        preserves_dynamic_library_path: false,
    };
    run_harness(
        unique_bench,
        bench_artifacts
            .executable(unique_bench)
            .context("fixture package m has no bench executable")?,
        &bench_context,
        false,
        true,
    )?;
    Ok(())
}

fn verify_fixture_binary_environment(
    fixture: &Fixture,
    package_roots: &HashMap<String, PathBuf>,
    package_environments: &PackageEnvironments,
    targets: &[Target],
    runtime: &CargoRuntime,
    extra_env: &BTreeMap<OsString, OsString>,
    runtime_library: &Path,
) -> Result<()> {
    let smoke_target = fixture_target(targets, "l", "smoke")?;
    let args = cargo_target_args("test", "smoke", false);
    let artifacts = cargo_test_artifacts(
        &fixture.root,
        &args,
        std::slice::from_ref(smoke_target),
        runtime,
        extra_env,
    )?;
    let executable = artifacts
        .executable(smoke_target)
        .context("fixture package l has no integration-test executable")?;
    let runtime_environment = artifacts.runtime_environment(smoke_target, executable)?;
    let environment = harness_environment(&package_environments["l"], &runtime_environment.values);
    if !environment.contains_key(OsStr::new("CARGO_BIN_EXE_helper-tool")) {
        bail!("Cargo binary executable environment self-test failed");
    }
    let context = HarnessContext {
        package_root: &package_roots["l"],
        package_environment: &environment,
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
        preserves_dynamic_library_path: runtime_environment.preserves_dynamic_library_path,
    };
    run_harness(smoke_target, executable, &context, false, true)?;
    Ok(())
}

fn cargo_fixture_self_test_inner() -> Result<()> {
    let fixture = Fixture::new()?;
    for &(relative, contents) in CARGO_FIXTURE_FILES {
        fixture.write(relative, contents)?;
    }
    let host = rustc_host(&fixture.root)?;
    let runner = if cfg!(windows) {
        vec!["cmd", "/C"]
    } else {
        vec!["env", "TCL_LSP_SMOKE_RUNNER=used"]
    };
    let rustc_config = rustc_probe_config(&fixture)?;
    let config = format!(
        "[build]\n{rustc_config}\n[target.{}]\nrunner = {}\n",
        serde_json::to_string(&host)?,
        serde_json::to_string(&runner)?,
    );
    fixture.write(".cargo/config.toml", &config)?;
    let mut lock = Command::new("cargo");
    lock.args(["generate-lockfile", "--offline"])
        .current_dir(&fixture.root);
    command_output(&mut lock)?;
    let runtime = cargo_runtime(&fixture.root)?;
    if runtime.target != host || runtime.runner.is_none() {
        bail!("Cargo target-runner resolution self-test failed");
    }
    let (package_roots, package_environments, targets) =
        load_targets(&fixture.root, &runtime.target)?;
    verify_fixture_metadata(&fixture, &targets)?;
    let extra_env = BTreeMap::from([
        (
            OsString::from("CARGO_TARGET_DIR"),
            fixture.root.join("target").into_os_string(),
        ),
        (
            OsString::from("RUSTFLAGS"),
            OsString::from("-C prefer-dynamic"),
        ),
    ]);
    reset_rustc_probe(&fixture)?;
    let runtime_library = rust_runtime_library(&fixture.root, &runtime.target, &runtime.rustc)?;
    verify_rustc_probe(&fixture)?;
    verify_fixture_libraries(
        &fixture,
        &package_roots,
        &package_environments,
        &targets,
        &runtime,
        &extra_env,
        &runtime_library,
    )?;
    verify_fixture_collision_and_bench(
        &fixture,
        &package_roots,
        &package_environments,
        &targets,
        &runtime,
        &extra_env,
        &runtime_library,
    )?;
    verify_fixture_binary_environment(
        &fixture,
        &package_roots,
        &package_environments,
        &targets,
        &runtime,
        &extra_env,
        &runtime_library,
    )
}

fn smoke_module_scanner_self_test() -> Result<()> {
    let module_fixture = Fixture::new()?;
    module_fixture.write("src/lib.rs", "pub mod smoke;\n")?;
    module_fixture.write("src/smoke.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &module_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([module_fixture.root.join("src/smoke.rs")]) {
        bail!("out-of-line smoke module scanner self-test failed");
    }
    let conditional_fixture = Fixture::new()?;
    conditional_fixture.write(
        "src/lib.rs",
        "#[cfg_attr(unix, path = \"platform_tests.rs\")]\n#[cfg_attr(windows, path = \"windows_tests.rs\")]\npub mod smoke;\n",
    )?;
    conditional_fixture.write("src/platform_tests.rs", "#[test]\nfn ordinary() {}\n")?;
    conditional_fixture.write("src/windows_tests.rs", "#[test]\nfn ordinary() {}\n")?;
    conditional_fixture.write("src/smoke.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &conditional_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found
        != BTreeSet::from([
            conditional_fixture.root.join("src/platform_tests.rs"),
            conditional_fixture.root.join("src/smoke.rs"),
            conditional_fixture.root.join("src/windows_tests.rs"),
        ])
    {
        bail!("conditionally attributed module-path scanner self-test failed");
    }
    let relative_fixture = Fixture::new()?;
    relative_fixture.write("src/lib.rs", "#[path = \"../smoke_tests.rs\"] mod smoke;\n")?;
    relative_fixture.write("smoke_tests.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &relative_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([relative_fixture.root.join("smoke_tests.rs")]) {
        bail!("path-attributed source outside crate directory self-test failed");
    }
    let logical_fixture = Fixture::new()?;
    logical_fixture.write("src/lib.rs", "#[path = \"alt.rs\"]\nmod smoke;\n")?;
    logical_fixture.write("src/alt.rs", "mod inner;\n")?;
    logical_fixture.write("src/inner.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &logical_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([logical_fixture.root.join("src/inner.rs")]) {
        bail!("path-attributed module logical-directory self-test failed");
    }
    nested_path_module_scanner_self_test()?;
    literal_include_scanner_self_test()?;
    Ok(())
}

fn nested_path_module_scanner_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.write("src/lib.rs", "#[path = \"sub/alt.rs\"]\nmod smoke;\n")?;
    fixture.write("src/sub/alt.rs", "mod inner;\n")?;
    fixture.write("src/sub/inner.rs", "#[test]\nfn ordinary() {}\n")?;
    fixture.write(
        "src/inner.rs",
        "#[test]\nfn wrong_directory_must_not_be_selected() {}\n",
    )?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([fixture.root.join("src/sub/inner.rs")]) {
        bail!("nested path-attributed module directory self-test failed");
    }
    let inline_fixture = Fixture::new()?;
    inline_fixture.write(
        "src/lib.rs",
        "#[path = \"alt\"]\nmod smoke { mod inner; }\n",
    )?;
    inline_fixture.write("src/alt/inner.rs", "#[test]\nfn ordinary() {}\n")?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &inline_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([inline_fixture.root.join("src/alt/inner.rs")]) {
        bail!("path-attributed inline-module directory self-test failed");
    }
    Ok(())
}

fn literal_include_scanner_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.write("src/lib.rs", "include!(\"sub/generated_tests.rs\");\n")?;
    fixture.write("src/main.rs", "fn main() {}\n")?;
    fixture.write(
        "src/sub/generated_tests.rs",
        "// tcl-lsp-smoke-target\n#[test]\nfn smoke_generated() {}\nmod smoke;\n",
    )?;
    fixture.write(
        "src/sub/smoke.rs",
        "#[test]\nfn ordinary_in_real_module() {}\n",
    )?;
    fixture.write(
        "src/smoke.rs",
        "#[test]\nfn ordinary_in_wrong_module() {}\n",
    )?;
    let targets = [
        Target {
            package_id: "included".to_owned(),
            package: "included".to_owned(),
            kind: "lib".to_owned(),
            name: "included".to_owned(),
            source: fixture.root.join("src/lib.rs"),
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        },
        Target {
            package_id: "included".to_owned(),
            package: "included".to_owned(),
            kind: "bin".to_owned(),
            name: "included".to_owned(),
            source: fixture.root.join("src/main.rs"),
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        },
    ];
    let mut inventory = SmokeInventory::default();
    inventory
        .sources
        .insert("src/sub/generated_tests.rs".to_owned());
    for target in &targets {
        scan_target_smoke_sources(&fixture.root, target, &mut inventory)?;
    }
    let included = inventory_entries(&fixture.root, &inventory, &targets);
    let expected = BTreeSet::from([
        (
            "src/sub/generated_tests.rs".to_owned(),
            target_identity(&targets[0]),
        ),
        ("src/sub/smoke.rs".to_owned(), target_identity(&targets[0])),
    ]);
    if included != expected {
        bail!("literal include target ownership self-test failed: {included:?}");
    }

    let macro_fixture = Fixture::new()?;
    macro_fixture.write(
        "src/lib.rs",
        "macro_rules! generated { () => { include!(\"sub/generated_tests.rs\"); }; }\ngenerated!();\n",
    )?;
    macro_fixture.write(
        "src/sub/generated_tests.rs",
        "#[test]\nfn smoke_generated_in_macro() {}\n",
    )?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(
        &macro_fixture.root.join("src/lib.rs"),
        false,
        &mut HashSet::new(),
        &mut found,
    )?;
    if found != BTreeSet::from([macro_fixture.root.join("src/sub/generated_tests.rs")]) {
        bail!("macro-body literal include self-test failed: {found:?}");
    }
    Ok(())
}

fn cargo_fixture_self_test_subprocess() -> Result<()> {
    let cargo_home = Fixture::new()?;
    let mut command = Command::new(env::current_exe().context("locating xtask executable")?);
    command
        .args(["smoke-targets", "fixture-self-test"])
        .env("CARGO_HOME", cargo_home.root.join("cargo-home"))
        .env("CARGO_MANIFEST_LINKS", "inherited");
    for (name, _) in env::vars_os() {
        let text = name.to_string_lossy();
        if text.starts_with("CARGO_BUILD_")
            || text.starts_with("CARGO_TARGET_")
            || matches!(
                text.as_ref(),
                "RUSTC"
                    | "RUSTC_WRAPPER"
                    | "RUSTC_WORKSPACE_WRAPPER"
                    | "RUSTFLAGS"
                    | "CARGO_ENCODED_RUSTFLAGS"
            )
        {
            command.env_remove(name);
        }
    }
    command_output(&mut command)?;
    Ok(())
}

#[cfg(not(test))]
fn cargo_fixture_self_test() -> Result<()> {
    cargo_fixture_self_test_subprocess()
}

#[cfg(test)]
fn cargo_fixture_self_test() -> Result<()> {
    // A binary unit test's current executable is libtest, not the xtask CLI.
    // Ask Cargo to launch the CLI, whose isolated bridge then starts the
    // fixture under exactly the same scrubbed environment as production.
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .args([
            "run",
            "--quiet",
            "--locked",
            "-p",
            "xtask",
            "--",
            "smoke-targets",
            "fixture-self-test-isolated",
        ])
        .current_dir(repo_root());
    command_output(&mut command)?;
    Ok(())
}

fn smoke_scanner_self_test() -> Result<()> {
    for source in [
        "mod smoke { #[test] fn ordinary() {} }",
        "pub(crate) mod smoke_tests { #[test] fn ordinary() {} }",
        "#[test] extern \"C\" fn smoke_probe() {}",
        "#[test] pub unsafe extern \"C-unwind\" fn smoke_hook() {}",
        "#[test] pub(crate) const unsafe extern fn smoke_qualified() {}",
        "#[test]\npub(crate)\nunsafe\nextern \"C\"\nfn smoke_split() {}",
        "#[test]\nfn r#smoke_raw() {}",
        "#[cfg_attr(unix, test)]\nfn smoke_conditional() {}",
        "#[cfg_attr(unix, cfg_attr(unix, test))]\nfn smoke_nested_conditional() {}",
    ] {
        let parsed = syn::parse_file(source).context("parsing smoke scanner self-test")?;
        let fake_source = Path::new("/repo/src/lib.rs");
        let mut found = BTreeSet::new();
        collect_smoke_test_sources(
            fake_source,
            Path::new("/repo/src"),
            &parsed.items,
            false,
            &mut HashSet::new(),
            &mut found,
        )?;
        if found != BTreeSet::from([fake_source.to_path_buf()]) {
            bail!("smoke declaration scanner missed: {source}");
        }
    }
    for source in [
        "fn smoke_detector() {}",
        "mod smoke { fn ordinary() {} }",
        "#[test] fn ordinary() {}",
    ] {
        let parsed = syn::parse_file(source).context("parsing smoke scanner negative self-test")?;
        let mut found = BTreeSet::new();
        collect_smoke_test_sources(
            Path::new("/repo/src/lib.rs"),
            Path::new("/repo/src"),
            &parsed.items,
            false,
            &mut HashSet::new(),
            &mut found,
        )?;
        if !found.is_empty() {
            bail!("smoke declaration scanner false positive: {source}");
        }
    }
    smoke_module_scanner_self_test()?;
    if !source_smoke_markers("// tcl-lsp-smoke-target")?.target
        || source_smoke_markers("fn tcl_lsp_smoke() {}")?.target
        || source_smoke_markers("const TEXT: &str = r#\"\n// tcl-lsp-smoke-target\n\"#;\n")?.target
        || source_smoke_markers("/*\n// tcl-lsp-smoke-target\n*/\n")?.target
        || source_smoke_markers("// tcl-lsp-smoke-target-extra").is_ok()
        || source_smoke_markers("let value = 1; // tcl-lsp-smoke-target").is_ok()
        || source_smoke_markers("// tcl-lsp-smoke-target\n// tcl-lsp-no-smoke-include\n").is_ok()
    {
        bail!("smoke marker scanner self-test failed");
    }
    include_macro_scanner_self_test()
}

fn include_macro_scanner_self_test() -> Result<()> {
    let ignored = source_include_macros(
        "const TEXT: &str = r#\"include!(concat!(env!(\\\"OUT_DIR\\\"), \\\"/tests.rs\\\"))\"#;\n/* include!(concat!(env!(\"OUT_DIR\"), \"/tests.rs\")); */\n",
    )?;
    if !ignored.literal_paths.is_empty() || ignored.non_literal_count != 0 {
        bail!("include scanner inspected a string or block comment");
    }
    let fixture = Fixture::new()?;
    let source = fixture.root.join("src/lib.rs");
    fixture.write(
        "src/lib.rs",
        "include!(concat!(env!(\"OUT_DIR\"), \"/tests.rs\"));\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("non-literal include without a smoke marker was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "macro_rules! generated { () => { include!(concat!(env!(\"OUT_DIR\"), \"/tests.rs\")); }; }\ngenerated!();\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("macro-body non-literal include without a marker was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "const DATA: &str = include!(concat!(env!(\"OUT_DIR\"), \"/data.rs\"));\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("expression-position non-literal include without a marker was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-no-smoke-include\ninclude!(concat!(env!(\"OUT_DIR\"), \"/data.rs\"));\n",
    )?;
    let mut found = BTreeSet::new();
    collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut found)?;
    if !found.is_empty() {
        bail!("non-smoke generated include marker selected a smoke source");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-no-smoke-include\ninclude!(concat!(env!(\"OUT_DIR\"), \"/one.rs\"));\ninclude!(concat!(env!(\"OUT_DIR\"), \"/two.rs\"));\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("one no-smoke marker classified multiple non-literal includes");
    }
    fixture.write("src/lib.rs", "// tcl-lsp-no-smoke-include\n")?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("stale no-smoke marker without an include was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-smoke-target\ninclude!(concat!(env!(\"OUT_DIR\"), \"/tests.rs\"));\ninclude!(concat!(env!(\"OUT_DIR\"), \"/more_tests.rs\"));\n",
    )?;
    found.clear();
    collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut found)?;
    if found != BTreeSet::from([source.clone()]) {
        bail!("marked non-literal include source was not selected");
    }
    fixture.write("src/lib.rs", "include!(\"generated_tests.rs\");\n")?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("missing literal include without a smoke marker was accepted");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-no-smoke-include\ninclude!(\"generated_tests.rs\");\n",
    )?;
    found.clear();
    collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut found)?;
    if !found.is_empty() {
        bail!("non-smoke missing literal include selected a smoke source");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-smoke-target\ninclude!(\"generated_tests.rs\");\n",
    )?;
    found.clear();
    collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut found)?;
    if found != BTreeSet::from([source.clone()]) {
        bail!("marked missing literal include source was not selected");
    }
    fixture.write(
        "src/lib.rs",
        "// tcl-lsp-no-smoke-include\ninclude!(\"one.rs\");\ninclude!(\"two.rs\");\n",
    )?;
    if collect_source_smoke_tests(&source, false, &mut HashSet::new(), &mut BTreeSet::new()).is_ok()
    {
        bail!("one no-smoke marker classified multiple missing literal includes");
    }
    Ok(())
}

fn multi_owner_inventory_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    let root = &fixture.root;
    fixture.write("shared.rs", "#[test]\nfn smoke_shared_source() {}\n")?;
    fixture.write("o/Cargo.toml", "")?;
    fixture.write("p/Cargo.toml", "")?;
    fixture.write(
        "smoke-targets.tsv",
        "shared.rs\to\tlib\tcommon\nshared.rs\tp\tlib\tcommon\n",
    )?;
    let source = root.join("shared.rs");
    let targets: Vec<Target> = ["o", "p"]
        .into_iter()
        .map(|package| Target {
            package_id: package.to_owned(),
            package: package.to_owned(),
            kind: "lib".to_owned(),
            name: "common".to_owned(),
            source: source.clone(),
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        })
        .collect();
    let mut inventory = SmokeInventory::default();
    record_target_source(&mut inventory, "shared.rs".to_owned(), &targets[0]);
    record_target_owner(&mut inventory, "shared.rs".to_owned(), &targets[1]);
    let rows: Vec<ManifestRow> = targets
        .iter()
        .map(|target| ManifestRow {
            source_text: "shared.rs".to_owned(),
            package: target.package.clone(),
            kind: target.kind.clone(),
            target_name: target.name.clone(),
        })
        .collect();
    if check_source_inventory(root, &rows[..1], &inventory, &targets).is_ok() {
        bail!("multi-owner source accepted a missing target row");
    }
    check_source_inventory(root, &rows, &inventory, &targets)
        .context("multi-owner source rejected complete target rows")?;
    let package_roots = HashMap::from([
        ("o".to_owned(), root.join("o")),
        ("p".to_owned(), root.join("p")),
    ]);
    let validated = validate_manifest(
        root,
        &root.join("smoke-targets.tsv"),
        &package_roots,
        &targets,
        &inventory,
    )?;
    if validated.len() != 2 {
        bail!("Cargo-owned source outside package directories was rejected");
    }

    fixture.write(
        "smoke-targets.tsv",
        "shared.rs\to\tlib\tcommon\nshared.rs\to\tbin\tcommon-bin\n",
    )?;
    let same_package_targets = vec![
        targets[0].clone(),
        Target {
            package_id: "o".to_owned(),
            package: "o".to_owned(),
            kind: "bin".to_owned(),
            name: "common-bin".to_owned(),
            source,
            available: true,
            testable: true,
            link_path_packages: BTreeSet::new(),
        },
    ];
    let mut same_package_inventory = SmokeInventory::default();
    for target in &same_package_targets {
        record_target_source(&mut same_package_inventory, "shared.rs".to_owned(), target);
    }
    let validated = validate_manifest(
        root,
        &root.join("smoke-targets.tsv"),
        &package_roots,
        &same_package_targets,
        &same_package_inventory,
    )?;
    if validated.len() != 2 {
        bail!("same-package shared Cargo source rejected exact target rows");
    }
    Ok(())
}

fn duplicate_manifest_row_self_test() -> Result<()> {
    let fixture = Fixture::new()?;
    let manifest = fixture.root.join("smoke-targets.tsv");
    fixture.write(
        "smoke-targets.tsv",
        "shared.rs\texample\tlib\texample\nshared.rs\texample\tlib\texample\n",
    )?;
    if read_manifest(&manifest).is_ok() {
        bail!("duplicate smoke target manifest row was accepted");
    }
    Ok(())
}

fn cargo_profile_library_paths_self_test() -> Result<()> {
    let profile = Path::new("/repo/target/debug");
    for executable in [
        profile.join("deps/test-harness"),
        profile.join("examples/example-harness"),
        profile.join("direct-harness"),
    ] {
        if cargo_profile_library_paths(&executable)?
            != (profile.to_path_buf(), profile.join("deps"))
        {
            bail!("Cargo profile library-path self-test failed");
        }
    }
    Ok(())
}

fn fixture_collision_self_test() -> Result<()> {
    let parent = Fixture::new()?;
    let collision = parent.root.join("collision");
    let unique = parent.root.join("unique");
    fs::create_dir(&collision)
        .with_context(|| format!("creating collision fixture {}", collision.display()))?;
    let mut candidates = VecDeque::from([collision, unique.clone()]);
    let fixture = create_fixture(|| candidates.pop_front().expect("fixture candidate"))?;
    if fixture.root != unique {
        bail!("fixture collision retry selected the wrong directory");
    }
    Ok(())
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
mod tests {
    use super::*;

    #[test]
    fn target_helpers_cover_static_edge_cases() {
        self_test().expect("smoke target helper self-test");
    }
}
