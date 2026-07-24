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

//! Differential tests for the `f5 registry-dump` verb.
//!
//! Runs the built `f5-query` binary and asserts stdout against structural
//! expectations (exit code, canonical JSON shape, expected keys/prefixes).
//! Self-contained: no external tool runs at test time, and no golden fixtures
//! are read from disk.

use std::process::Command;

fn run_f5(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary")
}

#[test]
fn commands_section_produces_snapshot() {
    // The `f5-irules` command-registry snapshot (core Tcl + iRules commands).
    // Its correctness is gated by tcl-registry's and tcl-cli's snapshot tests
    // (the same `command_registry_snapshot`), so here we only assert the verb is
    // wired and emits the canonical shape —
    // a full golden would duplicate that ~140k-line snapshot.
    let out = run_f5(&["registry-dump", "--section", "commands"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "commands exit: {:?}",
        out.status
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.starts_with("{\n  \"commandCount\": "),
        "canonical header"
    );
    assert!(text.contains("\"f5-irules\""), "f5-irules dialect");
    assert!(text.contains("\"summary\":"), "hover prose catalogue");
}

#[test]
fn all_section_bundles_every_subsection() {
    let out = run_f5(&["registry-dump", "--section", "all"]);
    assert_eq!(out.status.code(), Some(0), "all exit: {:?}", out.status);
    let text = String::from_utf8_lossy(&out.stdout);
    for key in [
        "\"commands\":",
        "\"events\":",
        "\"objects\":",
        "\"profiles\":",
    ] {
        assert!(text.contains(key), "all is missing {key}");
    }
}

#[test]
fn unknown_section_fails_cleanly() {
    // A genuinely unknown section still exits 2 with no stdout; `commands` and
    // `all` are now implemented (see the tests above).
    let output = run_f5(&["registry-dump", "--section", "bogus"]);
    assert_eq!(output.status.code(), Some(2), "unknown section exits 2");
    assert!(output.stdout.is_empty(), "unknown section emits no stdout");
}

#[test]
fn default_section_is_all_and_serialises() {
    // The default `--section all` now emits the full bundle (commands + the
    // graph snapshots), so the bare verb succeeds.
    let output = run_f5(&["registry-dump"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"commands\":"));
}
