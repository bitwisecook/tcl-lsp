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

//! Differential tests for additional `f5 query` surfaces: the
//! `--format tmsh` / `tmsh-delta` / `--transaction` rendering of **mutating**
//! queries, the `--in-place` + tmsh incompatibility guard, the `--partition`
//! loader binding, and `-f/--from-file`.
//!
//! Each test asserts byte-equality against the captured golden output for
//! `query …`; the test runs the built `f5-query` binary against the committed
//! fixtures. Self-contained: no external tool runs at test time.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> String {
    fixtures_dir().join(name).to_string_lossy().into_owned()
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("read golden {name}: {e}"))
}

/// Run `f5-query query …` and return `(stdout, stderr, exit_code)`.
fn run(args: &[&str]) -> (String, String, i32) {
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .arg("query")
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// Assert stdout matches `golden` at exit `code` (stderr ignored).
fn assert_stdout(golden_name: &str, code: i32, args: &[&str]) {
    let (out, err, got) = run(args);
    assert_eq!(got, code, "exit code for {args:?}\nstderr: {err}");
    assert_eq!(
        out,
        golden(golden_name),
        "stdout does not match the golden ({golden_name})\nargs: {args:?}"
    );
}

// mutating queries rendered through --format

#[test]
fn mutation_write_tmsh() {
    // `--write --format tmsh` re-renders the rewritten SCF as a `tmsh modify`
    // script (the accepted format must not be silently ignored).
    assert_stdout(
        "query-mut-monitor-tmsh-write.golden",
        0,
        &[
            r#".ltm.pool[] | .monitor = "/Common/tcp""#,
            &fixture("bigip.conf"),
            "--write",
            "--format",
            "tmsh",
        ],
    );
}

#[test]
fn mutation_write_tmsh_delta() {
    assert_stdout(
        "query-mut-monitor-tmsh-delta-write.golden",
        0,
        &[
            r#".ltm.pool[] | .monitor = "/Common/tcp""#,
            &fixture("bigip.conf"),
            "--write",
            "--format",
            "tmsh-delta",
        ],
    );
}

#[test]
fn mutation_write_tmsh_transaction() {
    assert_stdout(
        "query-mut-monitor-tmsh-txn-write.golden",
        0,
        &[
            r#".ltm.pool[] | .monitor = "/Common/tcp""#,
            &fixture("bigip.conf"),
            "--write",
            "--format",
            "tmsh",
            "--transaction",
        ],
    );
}

#[test]
fn in_place_tmsh_refused() {
    // `--in-place` must preserve the on-disk SCF format, so tmsh output is
    // rejected before any file is touched (stderr + exit 2).
    let (out, err, code) = run(&[
        r#".ltm.pool[] | .monitor = "/Common/tcp""#,
        &fixture("bigip.conf"),
        "--in-place",
        "--format",
        "tmsh",
    ]);
    assert_eq!(code, 2, "in-place + tmsh must exit 2");
    assert_eq!(out, "", "no stdout on the refusal path");
    assert_eq!(err, golden("query-inplace-tmsh-refused.err.golden"));
}

// --partition

#[test]
fn partition_bare_qualifies_short_names() {
    // A bare `--partition NAME` qualifies partition-relative object names with
    // `/<NAME>/` instead of the implicit `/Common/`.
    assert_stdout(
        "query-partition-bare.golden",
        0,
        &[
            ".ltm.pool[]",
            &fixture("tenant-relative.conf"),
            "--partition",
            "Tenant_A",
            "--paths-only",
        ],
    );
}

#[test]
fn partition_default_is_common() {
    assert_stdout(
        "query-partition-default-common.golden",
        0,
        &[
            ".ltm.pool[]",
            &fixture("tenant-relative.conf"),
            "--paths-only",
        ],
    );
}

#[test]
fn partition_path_form() {
    // `--partition PATH=PARTITION` binds a single source by path.
    let path = fixture("tenant-relative.conf");
    assert_stdout(
        "query-partition-pathform.golden",
        0,
        &[
            ".ltm.pool[]",
            &path,
            "--partition",
            &format!("{path}=Tenant_B"),
            "--paths-only",
        ],
    );
}

// -f/--from-file

#[test]
fn from_file_reads_expression() {
    // `-f FILE` reads the query expression from FILE; the positional that would
    // otherwise be the expression becomes the first input.
    assert_stdout(
        "query-fromfile-names.golden",
        0,
        &[
            "-f",
            &fixture("query-pool-names.f5q"),
            &fixture("bigip.conf"),
            "--raw",
        ],
    );
}
