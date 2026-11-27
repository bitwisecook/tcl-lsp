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

//! Structural / literal-assertion tests for the native `tcl` CLI.
//!
//! Each test runs the built `tcl` binary and asserts its output against
//! expectations written directly in this file — exit codes, output
//! substrings, or structural checks on parsed JSON — rather than against
//! committed golden/snapshot fixtures.

use std::path::PathBuf;
use std::process::{Command, Output};

#[path = "cli/cr_terminated_documents.rs"]
mod cr_terminated_documents;
#[path = "cli/diag_findings.rs"]
mod diag_findings;
#[path = "cli/dialect_flag.rs"]
mod dialect_flag;
#[path = "cli/inspection_verbs.rs"]
mod inspection_verbs;
#[path = "cli/opt_folds.rs"]
mod opt_folds;
#[path = "cli/policy_layers.rs"]
mod policy_layers;
#[path = "cli/policy_truth_table.rs"]
mod policy_truth_table;
#[path = "cli/project_spec_packs.rs"]
mod project_spec_packs;
#[path = "cli/show_suppressed.rs"]
mod show_suppressed;
#[path = "cli/source_transforms.rs"]
mod source_transforms;

/// `Command::output` through a reused `tcl` process (`tcl_cli_support::worker`):
/// a fresh process spends seconds loading packs before it reads its argv, and
/// this file asserts on many invocations.
trait Reused {
    fn reused_output(&mut self) -> std::io::Result<std::process::Output>;
}

impl Reused for Command {
    fn reused_output(&mut self) -> std::io::Result<std::process::Output> {
        tcl_cli_support::worker::output(self, env!("CARGO_TARGET_TMPDIR"))
    }
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn spec_pack_project() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/spec-packs/tiny-project")
}

/// An `XDG_CONFIG_HOME` that names no directory — never created — so no
/// `tcl-lsp/config.ini` can be found under it and nothing is left behind: the
/// global layer every policy-reading verb resolves (`diag`, `lint`,
/// `validate`, `opt`) is empty, and a developer's own `config.ini` cannot fail
/// a test CI passes. A missing file is an absent layer, read silently.
fn absent_config_home() -> PathBuf {
    // One path for every spawn, so the invocations can share reused `tcl`
    // processes (a worker is keyed by its environment); it is never created.
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("tcl-cli-tests-no-config-absent-config")
}

/// The built `tcl` binary, isolated from the machine's global configuration
/// (`XDG_CONFIG_HOME` is consulted first on every platform). Every spawn in
/// this file starts here; a test that wants a global layer of its own sets
/// `XDG_CONFIG_HOME` again, which overrides this one.
fn tcl() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
    command.env("XDG_CONFIG_HOME", absent_config_home());
    command
}

/// Run the built `tcl` binary with `args`, returning captured stdout bytes.
fn run_tcl(args: &[&str]) -> Vec<u8> {
    let output = tcl()
        .args(args)
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn run_tcl_in(current_dir: &std::path::Path, args: &[&str]) -> Output {
    tcl()
        .current_dir(current_dir)
        .args(args)
        .reused_output()
        .expect("failed to spawn tcl binary")
}

/// Like [`run_tcl`] but tolerates a non-zero exit — `diag` returns 1 whenever
/// it reports a problem-severity finding, which is not a harness failure.
fn run_tcl_allow_failure(args: &[&str]) -> Vec<u8> {
    tcl()
        .args(args)
        .reused_output()
        .expect("failed to spawn tcl binary")
        .stdout
}

/// Run `tcl diag --json` over one `.tcl` document written to a scratch file and
/// return every row as a `(code, line)` pair in report order.
fn tcl_diag_rows(tag: &str, text: &str) -> Vec<(String, u64)> {
    let scratch = Scratch::new(&format!("cr-{tag}"));
    let path = scratch.write("doc.tcl", text);

    let out = run_tcl_allow_failure(&["diag", path.to_str().expect("utf-8 path"), "--json"]);
    let report: serde_json::Value = serde_json::from_slice(&out).expect("diag JSON");
    report[0]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|d| {
            (
                d["code"].as_str().expect("code").to_owned(),
                d["line"].as_u64().expect("line"),
            )
        })
        .collect()
}

/// Run `tcl diag --json` over one `.sslictcl` document written to a scratch
/// file (so the dialect routes by extension, not by content signature), and
/// return its `SSLIC*` rows as `(code, line)` pairs in report order.
fn sslictcl_diag_rows(tag: &str, text: &str) -> Vec<(String, u64)> {
    let scratch = Scratch::new(&format!("sslictcl-{tag}"));
    let path = scratch.write("doc.sslictcl", text);

    let out = run_tcl_allow_failure(&["diag", path.to_str().expect("utf-8 path"), "--json"]);
    let report: serde_json::Value = serde_json::from_slice(&out).expect("diag JSON");
    report[0]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter_map(|d| {
            let code = d["code"].as_str()?;
            code.starts_with("SSLIC")
                .then(|| (code.to_owned(), d["line"].as_u64().expect("line")))
        })
        .collect()
}

/// A scratch directory for one test, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tcl-cli-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Self(dir)
    }

    /// Write `text` at `rel` (directories created) and return its path.
    fn write(&self, rel: &str, text: &str) -> PathBuf {
        self.write_bytes(rel, text.as_bytes())
    }

    /// Write `bytes` at `rel` (directories created) and return its path.
    fn write_bytes(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        std::fs::write(&path, bytes).expect("write file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// Run the built `tcl` binary with `args` and `env`, tolerating a non-zero
/// exit, and return its stdout. `env` applies over [`tcl`]'s isolation, so
/// an `XDG_CONFIG_HOME` there names the global layer the run reads.
fn run_tcl_env(args: &[&str], env: &[(&str, &std::ffi::OsStr)]) -> Vec<u8> {
    let mut command = tcl();
    command.args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    command
        .reused_output()
        .expect("failed to spawn tcl binary")
        .stdout
}

/// `(file label, code)` pairs of a `diag --json` report.
fn diag_codes_by_file(out: &[u8]) -> Vec<(String, String)> {
    let report: serde_json::Value = serde_json::from_slice(out).expect("diag JSON");
    report
        .as_array()
        .expect("report array")
        .iter()
        .flat_map(|file| {
            let label = file["file"].as_str().expect("file").to_owned();
            file["diagnostics"]
                .as_array()
                .expect("diagnostics")
                .iter()
                .map(move |d| (label.clone(), d["code"].as_str().expect("code").to_owned()))
        })
        .collect()
}

tcl_registry::testing::install_shipped!();
