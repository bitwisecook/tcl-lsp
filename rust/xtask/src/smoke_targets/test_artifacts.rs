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

//! The test executables Cargo builds and the runtime environment each needs: build-script output, linked paths and the profile's library paths.

use super::cargo_metadata::canonical_target_kinds;
use super::source_scan::lexically_normalise;
use super::{
    BuildEnvironment, CargoRuntime, CargoTestArtifacts, RuntimeEnvironment, Target, TargetKey,
    command_output,
};
use anyhow::{Context, Result, bail};
use cargo_config2::PathAndArgs;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

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

pub(super) fn cargo_test_executables(
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
    pub(super) fn executable(&self, target: &Target) -> Option<&PathBuf> {
        self.executables.get(&target_key(target))
    }

    pub(super) fn runtime_environment(
        &self,
        target: &Target,
        executable: &Path,
    ) -> Result<RuntimeEnvironment> {
        let mut environment = BTreeMap::new();
        let profile_dir = &cargo_profile_dir(executable)?;
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
            let profile_dir = &cargo_profile_dir(executable)?;
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

pub(super) fn cargo_test_artifacts(
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

pub(super) fn rust_runtime_library(
    root: &Path,
    target: &str,
    rustc: &PathAndArgs,
) -> Result<PathBuf> {
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

pub(super) fn dynamic_library_variable() -> &'static str {
    if cfg!(target_os = "windows") {
        "PATH"
    } else if cfg!(target_os = "macos") {
        "DYLD_FALLBACK_LIBRARY_PATH"
    } else {
        "LD_LIBRARY_PATH"
    }
}

/// Cargo before 1.100 links test executables into `<profile>/deps` (or
/// `examples`); 1.100's build-dir layout leaves them in
/// `<profile>/build/<package>/<hash>/out`.
fn cargo_profile_layout(executable: &Path) -> Result<(PathBuf, bool)> {
    let artifact_directory = executable
        .parent()
        .context("test executable has no parent")?;
    let build_dir = artifact_directory.ancestors().nth(3);
    if artifact_directory
        .file_name()
        .is_some_and(|name| name == "out")
        && build_dir
            .and_then(Path::file_name)
            .is_some_and(|name| name == "build")
    {
        let profile = build_dir
            .and_then(Path::parent)
            .context("Cargo build directory has no profile parent")?;
        return Ok((profile.to_path_buf(), true));
    }
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
    Ok((profile.to_path_buf(), false))
}

pub(super) fn cargo_profile_dir(executable: &Path) -> Result<PathBuf> {
    Ok(cargo_profile_layout(executable)?.0)
}

/// The profile paths Cargo appends to a test's dynamic-library search path:
/// the profile and its `deps` in the flat layout, the profile alone in the
/// build-dir layout.
pub(super) fn cargo_profile_library_paths(executable: &Path) -> Result<Vec<PathBuf>> {
    let (profile, build_dir_layout) = cargo_profile_layout(executable)?;
    let mut paths = vec![profile.clone()];
    if !build_dir_layout {
        paths.push(profile.join("deps"));
    }
    Ok(paths)
}
