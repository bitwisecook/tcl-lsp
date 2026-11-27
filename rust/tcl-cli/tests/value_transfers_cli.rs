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

//! The value-transfer design's CLI witnesses: the `tcl explore` and
//! `tcl opt` evidence for its programs, driven through the built `tcl`
//! binary rather than the library directly. Separate from
//! `rust/tcl-cli/tests/cli.rs`, which the diagnostic policy's work edits.

use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "value_transfers_cli/decided_branches.rs"]
mod decided_branches;
#[path = "value_transfers_cli/ordered_state.rs"]
mod ordered_state;
#[path = "value_transfers_cli/pack_completion_surfaces.rs"]
mod pack_completion_surfaces;
#[path = "value_transfers_cli/procedure_summaries.rs"]
mod procedure_summaries;
#[path = "value_transfers_cli/release_spellings.rs"]
mod release_spellings;
#[path = "value_transfers_cli/transfer_routes.rs"]
mod transfer_routes;

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

/// An `XDG_CONFIG_HOME` that names no directory — never created — so no
/// `tcl-lsp/config.ini` can be found under it: the global policy layer every verb resolves is empty, and a
/// developer's own `config.ini` cannot change what these witnesses see.
fn absent_config_home() -> PathBuf {
    // One path for every spawn, so the invocations can share reused `tcl`
    // processes (a worker is keyed by its environment); it is never created.
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("value-transfers-cli-tests-absent-config")
}

/// The built `tcl` binary, isolated from the machine's global configuration.
fn tcl() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
    command.env("XDG_CONFIG_HOME", absent_config_home());
    command
}

/// Run the built `tcl` binary with `args`, returning captured stdout text.
fn run_tcl(args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

/// The releases the oracle runs, oldest first.
const RELEASES: [&str; 5] = ["8.4", "8.5", "8.6", "9.0", "9.1"];

/// Run `script` from a file under `tclsh`: whether it exited cleanly and
/// what it printed. A script run from a file exits non-zero on an error,
/// where one read from stdin does not.
fn run_tclsh(tclsh: &str, script: &str) -> Option<(bool, String)> {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "value-transfers-cli-{}-{}.tcl",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::write(&path, script).ok()?;
    let output = Command::new(tclsh).arg(&path).output();
    let _ = std::fs::remove_file(&path);
    let output = output.ok()?;
    Some((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

/// Every release from `first` on with its reference interpreter, oldest
/// first, as the shared oracle lookup finds it
/// ([`tcl_test_support::witness_tclsh`]): a release with none fails the test
/// where `TCL_LSP_REQUIRE_TCLSH` requires it, and is reported as skipped
/// otherwise.
fn tclshs_from(first: &str) -> Vec<(&'static str, String)> {
    RELEASES
        .iter()
        .filter(|&&series| series >= first)
        .filter_map(|&series| {
            let version = tcl_dialect::TclVersion::from_version_string(series)?;
            let tclsh = tcl_test_support::witness_tclsh(version)?;
            Some((series, tclsh.path.to_string_lossy().into_owned()))
        })
        .collect()
}

/// `(line, column, code)` of every diagnostic `tcl diag` reports for
/// `source` under `dialect`, with `enable` turned on.
fn diagnostics_at(source: &str, dialect: &str, enable: &[&str]) -> Vec<(u64, u64, String)> {
    let mut args = vec!["diag", "--json", "--source", source, "--dialect", dialect];
    for code in enable {
        args.extend(["--enable", code]);
    }
    let output = tcl()
        .args(&args)
        .reused_output()
        .expect("failed to spawn tcl binary");
    // `tcl diag` exits 1 when it reports a warning or an error, 0 otherwise.
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("diag JSON");
    report
        .as_array()
        .expect("a report per input")
        .iter()
        .flat_map(|file| file["diagnostics"].as_array().expect("diagnostics").iter())
        .map(|diagnostic| {
            (
                diagnostic["line"].as_u64().expect("a line"),
                diagnostic["column"].as_u64().expect("a column"),
                diagnostic["code"].as_str().expect("a code").to_owned(),
            )
        })
        .collect()
}

/// `tcl opt --profile full` over `source` under `series`' dialect: the
/// rewritten program, with the report the verb appends as comments.
fn opt_under(source: &str, series: &str) -> String {
    run_tcl(&[
        "opt",
        "--source",
        source,
        "--profile",
        "full",
        "--dialect",
        tcl_dialect::TclVersion::profile_name_for(series).expect("modelled release"),
    ])
}

/// `opt`'s program without its trailing report.
fn statements_of(optimised: &str) -> String {
    optimised
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

tcl_registry::testing::install_shipped!();
