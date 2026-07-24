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

//! Behavioural tests for the `f5 irule` verb group.
//!
//! Runs the built `f5-query` binary against committed `.irule` / `.conf`
//! fixtures and asserts exit codes and error/usage text for cases that don't
//! depend on comparing full stdout against a captured golden file (input
//! validation, the unimplemented `pgo` sub, and `--help` for every sub).
//! Self-contained: no external tool runs at test time.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> String {
    fixtures_dir().join(name).to_string_lossy().into_owned()
}

/// Run `f5-query <args…>`; return `(code, stdout, stderr)`.
fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn extract_rejects_standalone_irule() {
    // Contract: extract only consumes configs.
    let (code, _out, stderr) = run(&[
        "irule",
        "extract",
        &fixture("irule-sample.irule"),
        "/tmp/_x",
    ]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("extract only accepts bigip.conf / SCF / UCS"),
        "stderr: {stderr}"
    );
}

#[test]
fn no_input_errors_exit_2() {
    let (code, _out, stderr) = run(&["irule", "format"]);
    assert_eq!(code, 2);
    assert_eq!(
        stderr,
        "error: no input provided; pass files, --source, or `-` for stdin\n"
    );
}

// Unimplemented subs — clean exit-2 error naming the missing engine.

fn assert_deferred(args: &[&str], expect_sub: &str) {
    let (code, _out, stderr) = run(args);
    assert_eq!(code, 2, "{args:?} should exit 2; stderr: {stderr}");
    let expected = format!("error: f5 irule {expect_sub} is not yet implemented (requires the ");
    assert!(
        stderr.starts_with(&expected) && stderr.ends_with(" engine)\n"),
        "{args:?} stderr: {stderr}"
    );
}

#[test]
fn pgo_is_deferred() {
    assert_deferred(
        &[
            "irule",
            "pgo",
            "--profile",
            "/dev/null",
            &fixture("irule-sample.irule"),
        ],
        "pgo",
    );
}

#[test]
fn lint_clean_input_no_findings() {
    // A config / standalone iRule with no issues prints the no-findings line
    // and exits 0.
    let (code, out, _) = run(&["irule", "lint", &fixture("irule-sample.irule")]);
    assert_eq!(code, 0);
    assert_eq!(out, "validate: no findings\n");
}

#[test]
fn context_no_irules_found_exits_1() {
    // A `--rule` filter matching nothing yields no bundles → exit 1.
    let (code, _out, stderr) = run(&[
        "irule",
        "context",
        &fixture("bigip.conf"),
        "--rule",
        "/Common/does-not-exist",
    ]);
    assert_eq!(code, 1);
    assert_eq!(stderr, "error: no iRules found in input\n");
}

// `--help` must work for every sub (including the unimplemented ones), since
// they parse their args before the handler runs.

#[test]
fn help_works_for_all_subs() {
    for sub in [
        "event-order",
        "event-info",
        "lint",
        "trace",
        "pgo",
        "extract",
        "format",
        "minify",
        "context",
    ] {
        let (code, out, _) = run(&["irule", sub, "--help"]);
        assert_eq!(code, 0, "irule {sub} --help should exit 0");
        assert!(!out.is_empty(), "irule {sub} --help should print usage");
    }
}
