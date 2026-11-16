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

//! Running the manifest: the Cargo target selection for each group and the harness run of every smoke test.

use super::manifest::is_smoke_named_target;
use super::test_artifacts::{
    cargo_profile_library_paths, cargo_test_artifacts, dynamic_library_variable,
    rust_runtime_library,
};
use super::{
    CargoRuntime, CargoTestArtifacts, HarnessContext, ManifestRow, PackageEnvironments, Target,
    command_output,
};
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(super) fn harness_environment(
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

fn target_selector(target: &Target) -> (String, String) {
    if target.kind == "lib" {
        (target.kind.clone(), String::new())
    } else {
        (target.kind.clone(), target.name.clone())
    }
}

pub(super) fn target_groups(rows: &[ManifestRow], targets: &[Target]) -> Result<Vec<Vec<Target>>> {
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

pub(super) fn has_ineligible_collision(group: &[Target], targets: &[Target]) -> bool {
    let selector = target_selector(&group[0]);
    targets.iter().any(|target| {
        target_selector(target) == selector && (!target.available || !target.testable)
    })
}

pub(super) fn reject_ineligible_collisions(
    groups: &[Vec<Target>],
    targets: &[Target],
) -> Result<()> {
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

pub(super) fn cargo_target_args(
    kind: &str,
    target_name: &str,
    full_workspace: bool,
) -> Vec<String> {
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

pub(super) fn harness_command(executable: &Path, context: &HarnessContext<'_>) -> Result<Command> {
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
        paths.extend(cargo_profile_library_paths(executable)?);
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

pub(super) fn substring_skips(
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

pub(super) fn run_harness(
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

pub(super) fn execute_manifest(
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
