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
//! Runs the built `f5-query` binary and asserts stdout matches the captured
//! golden output for `registry-dump …`. Self-contained: no external tool runs
//! at test time.
//!
//! The `profiles`, `objects`, and `events` sections are asserted against a
//! golden. The `commands` / `all` sections are not implemented (they
//! embed the per-command traits/scalars dicts and hover prose catalogue), so
//! they are asserted to fail cleanly with exit code 2.

use std::path::PathBuf;
use std::process::Command;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn run_f5(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(args)
        .output()
        .expect("failed to spawn f5-query binary")
}

fn assert_section_matches(section: &str, golden: &str) {
    let expected = std::fs::read(golden_dir().join(golden)).expect("read golden");
    let output = run_f5(&["registry-dump", "--section", section]);
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "registry-dump --section {section} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&expected),
        "registry-dump --section {section} stdout did not match golden {golden}"
    );
}

#[test]
fn profiles_section_matches_golden() {
    assert_section_matches("profiles", "registry_dump_profiles.golden");
}

#[test]
fn objects_section_matches_golden() {
    assert_section_matches("objects", "registry_dump_objects.golden");
}

#[test]
fn events_section_matches_golden() {
    // The iRules event graph: per-event props (with the `transport`
    // string/list/null remapping), firing order, flow chains (incl. `notes`),
    // and the content-addressed valid-command digests.
    assert_section_matches("events", "registry_dump_events.golden");
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
fn profiles_section_to_file_matches_golden() {
    // `--output FILE` writes the same canonical JSON plus a trailing newline.
    let expected =
        std::fs::read(golden_dir().join("registry_dump_profiles.golden")).expect("read golden");
    let tmp = std::env::temp_dir().join(format!("registry_dump_{}.json", std::process::id()));
    let output = run_f5(&[
        "registry-dump",
        "--section",
        "profiles",
        "-o",
        tmp.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let written = std::fs::read(&tmp).expect("read written file");
    let _ = std::fs::remove_file(&tmp);
    assert_eq!(
        String::from_utf8_lossy(&written),
        String::from_utf8_lossy(&expected),
    );
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
