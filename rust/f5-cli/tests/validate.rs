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

//! Tests for `f5 validate` (alias `lint`).
//!
//! These are direct CLI-behaviour checks (no external golden/fixture
//! comparison): the `lint` alias must produce output identical to
//! `validate` for the same input, and a missing input file must fail with
//! the documented exit code and stderr message.
//!
//! The lint engine is a *sibling* of the query engine — it walks the BIG-IP
//! model directly (reusing `tcl-bigip`'s model, the `tcl-irules` object-ref
//! walker, and the `tcl-registry` event set) rather than the query DSL.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Run `f5-query`, asserting the exit code and returning stdout/stderr.
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

// Alias: `lint` == `validate`.

#[test]
fn lint_alias_matches_validate() {
    let dir = fixtures_dir();
    let conf = dir.join("validate-rules.conf");
    let conf = conf.to_str().unwrap();
    let (vc, vout, _) = run(&["validate", conf, "--format", "json"]);
    let (lc, lout, _) = run(&["lint", conf, "--format", "json"]);
    assert_eq!(vout, lout, "lint alias output differs from validate");
    assert_eq!(vc, lc, "lint alias exit code differs from validate");
}

// Input error: missing file → `error: not a file: <path>` on stderr,
//    exit 2 (the OS-error path).

#[test]
fn missing_file_errors() {
    let (code, stdout, stderr) = run(&["validate", "/nonexistent/does-not-exist.conf"]);
    assert_eq!(code, 2, "expected exit 2 on missing input");
    assert_eq!(stdout, "", "no stdout expected on input error");
    assert_eq!(
        stderr, "error: not a file: /nonexistent/does-not-exist.conf\n",
        "stderr mismatch on input error"
    );
}
