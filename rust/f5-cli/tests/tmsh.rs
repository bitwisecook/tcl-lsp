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

//! CLI-level tests for the `f5 tmsh` verb (SCF → tmsh emit engine).
//!
//! The golden/snapshot-comparison tests that used to live here (checking
//! Rust output against goldens captured from the original Python
//! implementation) have been removed now that the Rust implementation is
//! the source of truth. What remains is argument-validation behaviour,
//! asserted against literal expected values rather than external fixtures.

use std::path::PathBuf;
use std::process::Command;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn conf() -> String {
    fixtures_dir()
        .join("bigip.conf")
        .to_string_lossy()
        .into_owned()
}

#[test]
fn unknown_include_kind_errors() {
    let conf = conf();
    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(["tmsh", &conf, "--include", "bogus"])
        .output()
        .expect("failed to spawn f5-query binary");
    assert_eq!(output.status.code(), Some(2));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("unknown kind 'bogus'"),
        "stderr should flag the unknown kind: {err}"
    );
}
