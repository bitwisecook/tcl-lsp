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

//! The self-test over the Cargo fixture: metadata, runtime environments, library path order, collisions and the subprocess it runs in.

use super::cargo_fixture::{
    CARGO_FIXTURE_FILES, create_fixture, fixture_target, reset_rustc_probe,
};
use super::cargo_metadata::{cargo_runtime, load_targets, rustc_host};
use super::execution::{
    cargo_target_args, harness_command, harness_environment, has_ineligible_collision,
    reject_ineligible_collisions, run_harness, target_groups,
};
use super::manifest::target_identity;
use super::source_scan::scan_target_smoke_sources;
use super::test_artifacts::{
    cargo_profile_dir, cargo_profile_library_paths, cargo_test_artifacts, cargo_test_executables,
    dynamic_library_variable, rust_runtime_library,
};
use super::{
    CargoRuntime, Fixture, HarnessContext, ManifestRow, PackageEnvironments, SmokeInventory,
    Target, command_output, rustc_probe_config, verify_rustc_probe,
};
use anyhow::{Context, Result, bail};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

fn verify_fixture_metadata(fixture: &Fixture, targets: &[Target]) -> Result<()> {
    let build_smoke = fixture_target(targets, "a", "build_smoke")?;
    let normal_smoke = fixture_target(targets, "a", "normal_smoke")?;
    let proc_dependency_smoke = fixture_target(targets, "u", "proc_dependency_smoke")?;
    let host_dependency_smoke = fixture_target(targets, "v", "host_dependency_smoke")?;
    let proc_macro_host_smoke = fixture_target(targets, "d", "proc_macro_host_smoke")?;
    let proc_macro_root_smoke = fixture_target(targets, "d", "proc_macro_root_smoke")?;
    if build_smoke.available
        || !normal_smoke.available
        || !proc_dependency_smoke.available
        || host_dependency_smoke.available
        || proc_macro_host_smoke.available
        || !proc_macro_root_smoke.available
    {
        bail!(
            "resolver-v2 target feature-context self-test failed: build={}, normal={}, proc-dependency={}, host-dependency={}, proc-host={}, proc-root={}",
            build_smoke.available,
            normal_smoke.available,
            proc_dependency_smoke.available,
            host_dependency_smoke.available,
            proc_macro_host_smoke.available,
            proc_macro_root_smoke.available
        );
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
        path.contains("/build/e") && path.ends_with("/out/native")
    });
    let own_second = build_paths.iter().position(|path| {
        let path = path.to_string_lossy().replace('\\', "/");
        path.contains("/build/e") && path.ends_with("/out/aaa-native")
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
    let profile_paths = cargo_profile_library_paths(executable)?;
    if !command_paths[build_paths.len()..].starts_with(&profile_paths) {
        bail!("Cargo profile runtime path ordering self-test failed");
    }
    let mut expected_paths = build_paths;
    expected_paths.extend(profile_paths);
    expected_paths.push(context.rust_runtime_library.to_path_buf());
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
            if path.contains("/build/e") && path.ends_with("/out/native") {
                Some("e-native")
            } else if path.contains("/build/e") && path.ends_with("/out/aaa-native") {
                Some("e-aaa-native")
            } else if path.contains("/build/c") && path.ends_with("/out/native") {
                Some("c-native")
            } else if path.contains("/build/d") && path.ends_with("/out/proc-macro-native") {
                Some("d-proc-macro-native")
            } else if path.contains("/build/f") && path.ends_with("/out/native") {
                Some("f-native")
            } else if path.contains("/build/r") && path.ends_with("/out/build-only-native") {
                Some("r-build-only-native")
            } else if path.ends_with("/shared-native") {
                Some("e-shared-native")
            } else if path.contains("/build/s")
                && path.ends_with("/out/unrelated-proc-macro-native")
            {
                Some("s-unrelated-proc-macro-native")
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
    let cargo_path_order: Vec<_> = fixture_linked_path_order(&cargo_paths)
        .into_iter()
        .filter(|path| *path != "d-proc-macro-native")
        .collect();
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
            if path.contains("/build/q") && path.ends_with("/out/custom") {
                Some("q-custom")
            } else if path.contains("/build/q") && path.ends_with("/out/linked") {
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

fn verify_fixture_proc_macro_link_path(
    fixture: &Fixture,
    package_roots: &HashMap<String, PathBuf>,
    package_environments: &PackageEnvironments,
    targets: &[Target],
    runtime: &CargoRuntime,
    extra_env: &BTreeMap<OsString, OsString>,
    runtime_library: &Path,
) -> Result<()> {
    let target = fixture_target(targets, "t", "t")?;
    let args = cargo_target_args("", "", true);
    let artifacts = cargo_test_executables(&fixture.root, &args, None, extra_env)?;
    let executable = artifacts
        .executable(target)
        .context("fixture package t has no library executable")?;
    let runtime_environment = artifacts.runtime_environment(target, executable)?;
    let direct_record = fixture.root.join("direct-proc-macro-paths");
    let mut environment =
        harness_environment(&package_environments["t"], &runtime_environment.values);
    environment.insert(
        OsString::from("TCL_LSP_RECORD_PROC_MACRO_PATH"),
        direct_record.as_os_str().to_os_string(),
    );
    let context = HarnessContext {
        package_root: &package_roots["t"],
        package_environment: &environment,
        rust_runtime_library: runtime_library,
        runner: runtime.runner.as_ref(),
        preserves_dynamic_library_path: runtime_environment.preserves_dynamic_library_path,
    };
    run_harness(target, executable, &context, false, true)?;

    let cargo_record = fixture.root.join("cargo-proc-macro-paths");
    let mut cargo_args = args;
    cargo_args.extend([
        "--exclude".to_owned(),
        "n".to_owned(),
        "smoke_proc_macro_link_path".to_owned(),
        "--".to_owned(),
        "--exact".to_owned(),
    ]);
    let mut cargo_harness = Command::new("cargo");
    cargo_harness
        .args(cargo_args)
        .current_dir(&fixture.root)
        .envs(extra_env)
        .env("TCL_LSP_RECORD_PROC_MACRO_PATH", &cargo_record);
    command_output(&mut cargo_harness)?;

    let direct_paths = read_fixture_runtime_paths(&direct_record)?;
    let cargo_paths = read_fixture_runtime_paths(&cargo_record)?;
    let direct = fixture_linked_path_order(&direct_paths);
    let cargo = fixture_linked_path_order(&cargo_paths);
    if direct.is_empty()
        || direct.iter().any(|path| *path != "d-proc-macro-native")
        || !cargo.contains(&"d-proc-macro-native")
    {
        bail!(
            "direct harness procedural-macro paths {direct:?} do not preserve Cargo's dependency path {cargo:?}: direct {direct_paths:?}, Cargo {cargo_paths:?}"
        );
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
    let mut cargo_args = args.clone();
    cargo_args.extend([
        "smoke_binary_path".to_owned(),
        "--".to_owned(),
        "--exact".to_owned(),
    ]);
    let mut cargo_harness = Command::new("cargo");
    cargo_harness
        .args(cargo_args)
        .current_dir(&fixture.root)
        .envs(extra_env);
    command_output(&mut cargo_harness)?;
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

pub(super) fn cargo_fixture_self_test_inner() -> Result<()> {
    let fixture = Fixture::new()?;
    for &(relative, contents) in CARGO_FIXTURE_FILES {
        fixture.write(relative, contents)?;
    }
    let host = rustc_host(&fixture.root)?;
    // macOS purges every DYLD_* variable when it execs an Apple-signed binary,
    // and a Cargo runner is always one, so the harness could never hand the
    // dynamic library path through a runner to the test on this platform. Run
    // the fixture without one there and cover the runner leg on the platforms
    // where it is observable; the runner itself is Cargo's, not ours.
    let runner = if cfg!(target_os = "macos") {
        None
    } else if cfg!(windows) {
        Some(vec!["cmd", "/C"])
    } else {
        Some(vec!["env", "TCL_LSP_SMOKE_RUNNER=used"])
    };
    let rustc_config = rustc_probe_config(&fixture)?;
    let runner_config = match &runner {
        Some(runner) => format!("runner = {}\n", serde_json::to_string(runner)?),
        None => String::new(),
    };
    let config = format!(
        "[build]\n{rustc_config}\n[target.{}]\n{runner_config}",
        serde_json::to_string(&host)?,
    );
    fixture.write(".cargo/config.toml", &config)?;
    let mut lock = Command::new("cargo");
    lock.args(["generate-lockfile", "--offline"])
        .current_dir(&fixture.root);
    command_output(&mut lock)?;
    let runtime = cargo_runtime(&fixture.root)?;
    if runtime.target != host || runtime.runner.is_none() != runner.is_none() {
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
    verify_fixture_proc_macro_link_path(
        &fixture,
        &package_roots,
        &package_environments,
        &targets,
        &runtime,
        &extra_env,
        &runtime_library,
    )?;
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

fn configure_cargo_fixture_subprocess(command: &mut Command, cargo_home: &Fixture) {
    command
        .env("CARGO_HOME", cargo_home.root.join("cargo-home"))
        .env("CARGO_MANIFEST_LINKS", "inherited")
        .env("CARGO_BIN_EXE_unrelated", "inherited")
        .env("CARGO_BIN_EXE_helper-tool", "bogus");
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
}

pub(super) fn cargo_fixture_self_test_subprocess() -> Result<()> {
    let cargo_home = Fixture::new()?;
    let mut command = Command::new(env::current_exe().context("locating xtask executable")?);
    command.args(["smoke-targets", "fixture-self-test"]);
    configure_cargo_fixture_subprocess(&mut command, &cargo_home);
    command_output(&mut command)?;
    Ok(())
}

#[cfg(not(test))]
pub(super) fn cargo_fixture_self_test() -> Result<()> {
    cargo_fixture_self_test_subprocess()
}

#[cfg(test)]
pub(super) fn cargo_fixture_self_test() -> Result<()> {
    let cargo_home = Fixture::new()?;
    let mut command = Command::new(env::current_exe().context("locating xtask test executable")?);
    command
        .args([
            "--exact",
            "cargo_smoke::tests::cargo_fixture_environment_bridge",
            "--nocapture",
        ])
        .env("TCL_LSP_CARGO_FIXTURE_TEST_BRIDGE", "1");
    configure_cargo_fixture_subprocess(&mut command, &cargo_home);
    command_output(&mut command)?;
    Ok(())
}

pub(super) fn cargo_profile_library_paths_self_test() -> Result<()> {
    let profile = Path::new("/repo/target/debug");
    for executable in [
        profile.join("deps/test-harness"),
        profile.join("examples/example-harness"),
        profile.join("direct-harness"),
    ] {
        if cargo_profile_library_paths(&executable)?
            != [profile.to_path_buf(), profile.join("deps")]
        {
            bail!("Cargo profile library-path self-test failed");
        }
    }
    let build_dir_executable = profile.join("build/t/0123456789abcdef/out/t-0123456789abcdef");
    if cargo_profile_library_paths(&build_dir_executable)? != [profile.to_path_buf()]
        || cargo_profile_dir(&build_dir_executable)? != profile
    {
        bail!("Cargo build-dir layout library-path self-test failed");
    }
    Ok(())
}

pub(super) fn fixture_collision_self_test() -> Result<()> {
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
