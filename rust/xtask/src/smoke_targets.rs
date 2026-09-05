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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, anyhow, bail};
use cargo_config2::{Config as CargoConfig, PathAndArgs};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

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
}

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
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
    links: Option<String>,
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

#[derive(Clone, Debug)]
struct CargoRuntime {
    target: String,
    runner: Option<PathAndArgs>,
    rustc: PathAndArgs,
}

#[derive(Debug)]
struct BuildEnvironment {
    out_dir: PathBuf,
    values: BTreeMap<OsString, OsString>,
}

#[derive(Debug, Default)]
struct CargoTestArtifacts {
    executables: HashMap<TargetKey, PathBuf>,
    build_environments: HashMap<String, Vec<BuildEnvironment>>,
}

struct HarnessContext<'a> {
    package_root: &'a Path,
    package_environment: &'a BTreeMap<OsString, OsString>,
    rust_runtime_library: &'a Path,
    runner: Option<&'a PathAndArgs>,
}

type TargetKey = (String, String, PathBuf);
type PackageEnvironments = HashMap<String, BTreeMap<OsString, OsString>>;
type Contract = (
    HashMap<String, PathBuf>,
    PackageEnvironments,
    Vec<Target>,
    Vec<ManifestRow>,
    CargoRuntime,
);

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
            "CARGO_MANIFEST_LINKS",
            OsString::from(package.links.as_deref().unwrap_or_default()),
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
    Ok(values
        .into_iter()
        .map(|(name, value)| (OsString::from(name), value))
        .collect())
}

fn load_targets(
    root: &Path,
    target: &str,
) -> Result<(HashMap<String, PathBuf>, PackageEnvironments, Vec<Target>)> {
    let metadata = metadata(root, target)?;
    let features = workspace_target_features(&metadata, target, root)?;
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

fn is_smoke_named_target(target: &Target) -> bool {
    target.name == "smoke" || target.name.ends_with("_smoke")
}

fn read_manifest(manifest: &Path) -> Result<Vec<ManifestRow>> {
    let text =
        fs::read_to_string(manifest).with_context(|| format!("reading {}", manifest.display()))?;
    let mut rows = Vec::new();
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
        let Some(package_root) = package_roots.get(&row.package) else {
            errors.push(format!(
                "unknown Cargo package '{}' for {}",
                row.package, row.source_text
            ));
            continue;
        };
        if !source.starts_with(package_root) {
            errors.push(format!(
                "smoke source {} does not belong to package '{}'",
                row.source_text, row.package
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
        let owners = best_owners(&source, &package_targets);
        let exact = owners
            .iter()
            .filter(|owner| owner.kind == row.kind && owner.name == row.target_name);
        if owners.len() == 1 && exact.count() == 1 {
            if !owners[0].testable {
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
        .filter(|target| target.available && target.testable && is_smoke_named_target(target))
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

fn items_contain_smoke_declaration(items: &[syn::Item]) -> bool {
    items.iter().any(|item| match item {
        syn::Item::Fn(function) => function.sig.ident.to_string().starts_with("smoke"),
        syn::Item::Mod(module) => {
            module.ident.to_string().starts_with("smoke")
                || module
                    .content
                    .as_ref()
                    .is_some_and(|(_, items)| items_contain_smoke_declaration(items))
        }
        _ => false,
    })
}

fn compile_marker_regex() -> Result<Regex> {
    Regex::new(r"^[ \t]*//[ \t]*tcl-lsp-smoke-target[ \t]*$")
        .context("compiling smoke marker regex")
}

fn scan_smoke_sources(root: &Path, targets: &[Target]) -> Result<BTreeSet<String>> {
    let marker = compile_marker_regex()?;
    let integration = Regex::new(r"/tests/(smoke\.rs|[^/]*_smoke\.rs|smoke/[^/]*_smoke\.rs)$")?;
    let mut discovered = BTreeSet::new();
    for relative in tracked_rust_sources(root)? {
        let text = fs::read_to_string(root.join(&relative))
            .with_context(|| format!("reading {relative}"))?;
        let parsed = syn::parse_file(&text).with_context(|| format!("parsing {relative}"))?;
        if integration.is_match(&relative)
            || items_contain_smoke_declaration(&parsed.items)
            || text.lines().any(|line| marker.is_match(line))
        {
            discovered.insert(relative);
        }
    }
    for target in targets
        .iter()
        .filter(|target| target.available && target.testable && is_smoke_named_target(target))
    {
        let relative = target.source.strip_prefix(root).with_context(|| {
            format!(
                "smoke target source is outside workspace: {}",
                target.source.display()
            )
        })?;
        discovered.insert(relative.to_string_lossy().replace('\\', "/"));
    }
    Ok(discovered)
}

fn check_source_inventory(root: &Path, rows: &[ManifestRow], targets: &[Target]) -> Result<()> {
    let expected: BTreeSet<String> = rows.iter().map(|row| row.source_text.clone()).collect();
    let actual = scan_smoke_sources(root, targets)?;
    if expected == actual {
        return Ok(());
    }
    let missing: Vec<_> = actual.difference(&expected).cloned().collect();
    let stale: Vec<_> = expected.difference(&actual).cloned().collect();
    bail!(
        "smoke-targets.tsv inventory drift; missing rows: [{}]; stale rows: [{}]",
        missing.join(", "),
        stale.join(", ")
    )
}

fn target_selector(target: &Target) -> (String, String) {
    if target.kind == "lib" {
        (target.kind.clone(), String::new())
    } else {
        (target.kind.clone(), target.name.clone())
    }
}

fn target_groups(rows: &[ManifestRow], targets: &[Target]) -> Vec<Vec<Target>> {
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
    groups
}

fn has_ineligible_collision(group: &[Target], targets: &[Target]) -> bool {
    let selector = target_selector(&group[0]);
    targets.iter().any(|target| {
        target_selector(target) == selector && (!target.available || !target.testable)
    })
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
        target.kind.clone(),
        target.name.clone(),
        target.source.clone(),
    )
}

fn cargo_test_executables(
    root: &Path,
    args: &[String],
    extra_env: &BTreeMap<OsString, OsString>,
) -> Result<CargoTestArtifacts> {
    let mut command = Command::new("cargo");
    command
        .args(args)
        .args(["--no-run", "--message-format=json"])
        .current_dir(root)
        .envs(extra_env);
    let output = command_output(&mut command)?;
    let stdout = String::from_utf8(output.stdout).context("Cargo JSON output is not UTF-8")?;
    let mut executables = HashMap::new();
    let mut build_environments: HashMap<String, Vec<BuildEnvironment>> = HashMap::new();
    for line in stdout.lines() {
        let Ok(message) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if message.get("reason").and_then(Value::as_str) == Some("build-script-executed") {
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
            build_environments
                .entry(package_id.to_owned())
                .or_default()
                .push(BuildEnvironment {
                    out_dir: PathBuf::from(out_dir),
                    values,
                });
            continue;
        }
        if message.get("reason").and_then(Value::as_str) != Some("compiler-artifact")
            || message.pointer("/profile/test").and_then(Value::as_bool) != Some(true)
        {
            continue;
        }
        let Some(executable) = message.get("executable").and_then(Value::as_str) else {
            continue;
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
        for kind in canonical_target_kinds(&raw_kinds) {
            executables.insert(
                (kind, name.to_owned(), PathBuf::from(source)),
                PathBuf::from(executable),
            );
        }
    }
    Ok(CargoTestArtifacts {
        executables,
        build_environments,
    })
}

impl CargoTestArtifacts {
    fn executable(&self, target: &Target) -> Option<&PathBuf> {
        self.executables.get(&target_key(target))
    }

    fn build_environment(
        &self,
        target: &Target,
        executable: &Path,
    ) -> Result<BTreeMap<OsString, OsString>> {
        let Some(candidates) = self.build_environments.get(&target.package_id) else {
            return Ok(BTreeMap::new());
        };
        let profile_dir = executable
            .parent()
            .and_then(Path::parent)
            .context("test executable has no Cargo profile directory")?;
        let matching: Vec<&BTreeMap<OsString, OsString>> = candidates
            .iter()
            .filter(|candidate| candidate.out_dir.starts_with(profile_dir))
            .map(|candidate| &candidate.values)
            .collect();
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
        Ok((*first).clone())
    }
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

fn harness_command(executable: &Path, context: &HarnessContext<'_>) -> Result<Command> {
    let variable = dynamic_library_variable();
    let mut paths = vec![
        executable
            .parent()
            .context("test executable has no parent")?
            .to_path_buf(),
    ];
    if let Some(profile) = executable.parent().and_then(Path::parent) {
        paths.push(profile.to_path_buf());
        paths.push(profile.join("deps"));
    }
    paths.push(context.rust_runtime_library.to_path_buf());
    if let Some(existing) = env::var_os(variable) {
        paths.extend(env::split_paths(&existing));
    }
    let joined = env::join_paths(paths).context("joining dynamic library search path")?;
    let mut command = if let Some(runner) = context.runner {
        let mut command = Command::new(&runner.path);
        command.args(&runner.args).arg(executable);
        command
    } else {
        Command::new(executable)
    };
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
    let groups = target_groups(rows, targets);
    let rust_runtime_library = rust_runtime_library(root, &runtime.target, &runtime.rustc)?;
    let extra_env = BTreeMap::new();
    let regular_groups: Vec<&[Target]> = groups
        .iter()
        .filter(|group| !has_ineligible_collision(group, targets))
        .map(Vec::as_slice)
        .collect();
    let regular_executables = if regular_groups.is_empty() {
        CargoTestArtifacts::default()
    } else {
        let args = combined_cargo_target_args(&regular_groups);
        println!("==> cargo test --workspace selected smoke targets --no-run");
        cargo_test_executables(root, &args, &extra_env)?
    };
    let automatic_executables = if groups
        .iter()
        .any(|group| has_ineligible_collision(group, targets))
    {
        println!("==> cargo test --workspace automatic targets --no-run");
        Some(cargo_test_executables(
            root,
            &cargo_target_args("", "", true),
            &extra_env,
        )?)
    } else {
        None
    };

    for group in groups {
        let executables = if has_ineligible_collision(&group, targets) {
            automatic_executables
                .as_ref()
                .context("automatic Cargo artifact set is missing")?
        } else {
            &regular_executables
        };
        for target in &group {
            let executable = executables.executable(target).with_context(|| {
                format!(
                    "Cargo did not produce a test executable for {} {}:{}",
                    target.package, target.kind, target.name
                )
            })?;
            let package_root = package_roots
                .get(&target.package)
                .with_context(|| format!("missing package root for {}", target.package))?;
            let mut package_environment = package_environments
                .get(&target.package)
                .with_context(|| format!("missing package environment for {}", target.package))?
                .clone();
            package_environment.extend(executables.build_environment(target, executable)?);
            let context = HarnessContext {
                package_root,
                package_environment: &package_environment,
                rust_runtime_library: &rust_runtime_library,
                runner: runtime.runner.as_ref(),
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
    let rows = validate_manifest(root, &manifest, &package_roots, &targets)?;
    check_source_inventory(root, &rows, &targets)?;
    Ok((package_roots, package_environments, targets, rows, runtime))
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self> {
        let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = env::temp_dir().join(format!(
            "tcl-lsp-smoke-targets-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&root)
            .with_context(|| format!("creating smoke fixture {}", root.display()))?;
        Ok(Self { root })
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
    fixture.write_executable(
        "rustc-probe.sh",
        &format!(
            "#!/bin/sh\nif [ \"$1\" = --print ] && [ \"$2\" = sysroot ]; then : > \"{}\"; fi\nexec rustc \"$@\"\n",
            marker.display()
        ),
    )?;
    Ok(format!("rustc = {}\n", serde_json::to_string(&probe)?))
}

#[cfg(not(unix))]
fn rustc_probe_config(_fixture: &Fixture) -> Result<String> {
    Ok(String::new())
}

fn reset_rustc_probe(fixture: &Fixture) -> Result<()> {
    let marker = fixture.root.join("rustc-sysroot-queried");
    if marker.exists() {
        fs::remove_file(&marker).with_context(|| format!("removing {}", marker.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn verify_rustc_probe(fixture: &Fixture) -> Result<()> {
    if !fixture.root.join("rustc-sysroot-queried").is_file() {
        bail!("configured Cargo rustc was not used for the sysroot query");
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
members = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"]
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

[dependencies]
f = { path = "../f" }
"#,
    ),
    (
        "e/src/lib.rs",
        concat!(
            "#[test]\nfn ",
            "smoke_dependency() { assert!(f::normal_is_enabled()); assert_eq!(std::env::var(\"CARGO_PKG_NAME\").as_deref(), Ok(\"e\")); assert_eq!(std::env::var(\"CARGO_PKG_VERSION\").as_deref(), Ok(\"0.1.0\")); assert_eq!(std::env::var(\"CARGO_PKG_DESCRIPTION\").as_deref(), Ok(\"\")); assert!(std::path::Path::new(&std::env::var(\"CARGO_MANIFEST_DIR\").unwrap()).ends_with(\"e\")); assert!(std::path::Path::new(&std::env::var(\"CARGO_MANIFEST_PATH\").unwrap()).ends_with(std::path::Path::new(\"e/Cargo.toml\"))); assert_eq!(std::env::var(\"TCL_LSP_BUILD_ENV\").as_deref(), Ok(\"owned\")); assert!(std::env::var(\"OUT_DIR\").unwrap().replace(char::from(92), \"/\").contains(\"/build/e-\")); if !cfg!(windows) { assert_eq!(std::env::var(\"TCL_LSP_SMOKE_RUNNER\").as_deref(), Ok(\"used\")); } }\n\n",
            "#[test]\nfn long_smoke_dependency() { panic!(\"deep test must not run\"); }\n\n",
            "#[test]\nfn deep() { panic!(\"deep test must not run\"); }\n",
        ),
    ),
    (
        "e/build.rs",
        "fn main() { println!(\"cargo::rustc-env=TCL_LSP_BUILD_ENV=owned\"); }\n",
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
"#,
    ),
    (
        "h/src/lib.rs",
        r#"#[test]
fn ordinary_library_test() {
    assert!(std::env::current_dir().unwrap().ends_with("h"));
}
"#,
    ),
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
"#,
    ),
    ("j/src/lib.rs", ""),
    (
        "j/examples/demo.rs",
        "#[test]\nfn must_not_run() { panic!(); }\nfn main() {}\n",
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
];

fn verify_fixture_metadata(targets: &[Target]) -> Result<()> {
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

    let library_args = cargo_target_args("lib", "", false);
    let library_artifacts = cargo_test_executables(&fixture.root, &library_args, extra_env)?;
    let e_target = fixture_target(targets, "e", "e")?;
    let e_executable = library_artifacts
        .executable(e_target)
        .context("fixture package e has no library executable")?;
    let mut e_environment = package_environments["e"].clone();
    e_environment.extend(library_artifacts.build_environment(e_target, e_executable)?);
    let e_context = HarnessContext {
        package_root: &package_roots["e"],
        package_environment: &e_environment,
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
    };
    let e_selected = run_harness(e_target, e_executable, &e_context, false, true)?;
    if e_selected != [("smoke_dependency".to_owned(), "test".to_owned())] {
        bail!("exact nextest-name selection self-test failed");
    }
    let h_target = fixture_target(targets, "h", "library_smoke")?;
    let h_executable = library_artifacts
        .executable(h_target)
        .context("fixture package h has no library executable")?;
    let h_context = HarnessContext {
        package_root: &package_roots["h"],
        package_environment: &package_environments["h"],
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
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
    let automatic_args = cargo_target_args("example", "demo_smoke", true);
    let automatic = cargo_test_executables(&fixture.root, &automatic_args, extra_env)?;
    if automatic.executable(demo_target).is_none()
        || automatic
            .executable(fixture_target(targets, "j", "demo_smoke")?)
            .is_some()
    {
        bail!("automatic Cargo target eligibility self-test failed");
    }
    let demo_context = HarnessContext {
        package_root: &package_roots["i"],
        package_environment: &package_environments["i"],
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
    };
    run_harness(
        demo_target,
        automatic
            .executable(demo_target)
            .context("fixture package i has no example executable")?,
        &demo_context,
        false,
        true,
    )?;

    let bench_target = fixture_target(targets, "k", "bench_smoke")?;
    let bench_args = cargo_target_args("bench", "bench_smoke", false);
    let benches = cargo_test_executables(&fixture.root, &bench_args, extra_env)?;
    let bench_context = HarnessContext {
        package_root: &package_roots["k"],
        package_environment: &package_environments["k"],
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
    };
    run_harness(
        bench_target,
        benches
            .executable(bench_target)
            .context("fixture package k has no bench executable")?,
        &bench_context,
        false,
        true,
    )?;
    Ok(())
}

fn cargo_fixture_self_test() -> Result<()> {
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
        "[build]\ntarget = {}\n{rustc_config}\n[target.{}]\nrunner = {}\n",
        serde_json::to_string(&host)?,
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
    verify_fixture_metadata(&targets)?;
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
    )
}

fn self_test() -> Result<()> {
    let root = Path::new("/repo/example");
    let targets = vec![
        Target {
            package_id: "example".to_owned(),
            package: "example".to_owned(),
            kind: "lib".to_owned(),
            name: "example".to_owned(),
            source: root.join("src/lib.rs"),
            available: true,
            testable: true,
        },
        Target {
            package_id: "example".to_owned(),
            package: "example".to_owned(),
            kind: "bin".to_owned(),
            name: "example".to_owned(),
            source: root.join("src/main.rs"),
            available: true,
            testable: true,
        },
        Target {
            package_id: "example".to_owned(),
            package: "example".to_owned(),
            kind: "test".to_owned(),
            name: "new".to_owned(),
            source: root.join("tests/new.rs"),
            available: true,
            testable: true,
        },
    ];
    let owners = best_owners(&root.join("tests/new.rs"), &targets);
    if owners.len() != 1 || owners[0].kind != "test" || owners[0].name != "new" {
        bail!("integration target ownership self-test failed");
    }
    if best_owners(&root.join("src/shared.rs"), &targets).len() != 2 {
        bail!("ambiguous library/binary ownership self-test failed");
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
    for source in [
        "mod smoke {}",
        "pub(crate) mod smoke_tests;",
        "extern \"C\" fn smoke_probe() {}",
        "pub unsafe extern \"C-unwind\" fn smoke_hook() {}",
        "pub(crate) const unsafe extern fn smoke_qualified() {}",
        "#[test]\npub(crate)\nunsafe\nextern \"C\"\nfn smoke_split() {}",
    ] {
        let parsed = syn::parse_file(source).context("parsing smoke scanner self-test")?;
        if !items_contain_smoke_declaration(&parsed.items) {
            bail!("smoke declaration scanner missed: {source}");
        }
    }
    let marker = compile_marker_regex()?;
    if !marker.is_match("// tcl-lsp-smoke-target")
        || marker.is_match("// tcl-lsp-smoke-target-extra")
    {
        bail!("smoke marker scanner self-test failed");
    }
    cargo_fixture_self_test()
}

/// Validate or execute the exact Cargo smoke fallback.
pub fn run(operation: &str) -> Result<ExitCode> {
    let root = repo_root();
    match operation {
        "check" => {
            self_test()?;
            check_contract(&root)?;
            println!("targeted Cargo smoke ownership contract passed");
        }
        "run" | "list" => {
            let (package_roots, package_environments, targets, rows, runtime) =
                check_contract(&root)?;
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
