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

//! Exit-code tests for the **mutating** `f5 query` verb — the no-op /
//! strict-mode paths where a query resolves to no textual change (or queues
//! no edit at all) and the verb must exit with the documented code and
//! write nothing to stdout.
//!
//! Runs the built `f5-query` binary against the committed `bigip.conf`
//! fixture. Self-contained: every assertion is against a literal expected
//! value in this file, not an external golden file.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn conf() -> String {
    std::fs::canonicalize(fixtures_dir().join("bigip.conf"))
        .expect("canonicalize bigip.conf")
        .to_string_lossy()
        .into_owned()
}

/// Run `f5-query query ...`, asserting the exit code is in `ok_codes`, and
/// return stdout.
fn run_query(args: &[&str], ok_codes: &[i32]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("query")
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    assert!(
        ok_codes.contains(&code),
        "f5-query query {args:?} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Assert the verb exits with `code` and writes nothing to stdout (no-op /
/// strict cases).
fn assert_exit_empty(code: i32, args: &[&str]) {
    let actual = run_query(args, &[code]);
    assert_eq!(actual, "", "expected empty stdout for {args:?}");
}

// no-op / strict exit codes

#[test]
fn noop_matches_object_but_same_text_exits_1() {
    // Assigns the existing value: matches an object, queues an edit, but the
    // spliced text is identical → exit 1 (tolerant no-op).
    assert_exit_empty(
        1,
        &[
            r#".ltm.pool["/Common/web_pool"] | .monitor = "/Common/my_http_monitor""#,
            &conf(),
        ],
    );
}

#[test]
fn noop_strict_exits_2() {
    assert_exit_empty(
        2,
        &[
            "--strict",
            r#".ltm.pool["/Common/web_pool"] | .monitor = "/Common/my_http_monitor""#,
            &conf(),
        ],
    );
}

#[test]
fn select_nothing_queues_no_edit_exits_0() {
    // The predicate drops every object, so no edit op is queued: the query is
    // not a mutation and flows through the read path → exit 0, empty output.
    assert_exit_empty(
        0,
        &[
            r#".ltm.virtual[] | select(.name == "nope") | .pool = "x""#,
            &conf(),
        ],
    );
}
