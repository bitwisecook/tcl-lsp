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

//! Differential tests for `f5 grep` (alias `related`).
//!
//! Runs the built `f5-query` binary on the committed `bigip.conf` fixture and
//! asserts stdout matches the captured golden output for `grep …`. The fixture
//! is the same drift-free config the other BIG-IP verbs use, so the grep
//! reference graph inherits their stability.

use std::path::PathBuf;
use std::process::Command;

use regex::Regex;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Collapse the fixture's absolute `file://` URI to a stable token.
///
/// `f5 grep` echoes the absolute path of the source config (as the captured
/// golden does), so the `# Sources:` line and every JSON `source` field embed a
/// directory prefix that varies by checkout location — the CI runner
/// (`/home/runner/work/...`), a dev machine (`/Users/...`), or the agent
/// sandbox the goldens were captured in (`/home/user/...`).  Normalising both
/// the actual output and the golden makes the comparison portable without
/// regenerating the goldens, mirroring the `__FIXTURES__` normalisation in
/// `tcl-cli`'s CLI tests.
fn normalise_fixture_uri(s: &str) -> String {
    let re = Regex::new(r#"file://[^\s"]*?/rust/f5-cli/tests/fixtures/"#).unwrap();
    re.replace_all(s, "file://__FIXTURES__/").into_owned()
}

/// Run `f5-query grep <args> <fixture>` and assert stdout equals the golden.
/// `grep` exits 0 on a match and 1 on no match, so both are accepted.
fn assert_grep_matches(args: &[&str], golden: &str) {
    let conf = fixtures_dir().join("bigip.conf");
    let mut full: Vec<String> = vec!["grep".to_owned()];
    full.extend(args.iter().map(|s| (*s).to_owned()));
    full.push(conf.to_string_lossy().into_owned());

    let output = Command::new(env!("CARGO_BIN_EXE_f5-query"))
        .args(&full)
        .output()
        .expect("failed to spawn f5-query binary");
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 1,
        "f5-query {full:?} exited {code}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = std::fs::read(fixtures_dir().join(golden)).expect("read golden");
    assert_eq!(
        normalise_fixture_uri(&String::from_utf8_lossy(&output.stdout)),
        normalise_fixture_uri(&String::from_utf8_lossy(&expected)),
        "f5 grep output does not match the golden ({golden})"
    );
}

#[test]
fn grep_literal() {
    assert_grep_matches(&["web_pool"], "grep-literal.text.golden");
}

#[test]
fn grep_regex() {
    assert_grep_matches(&["-e", "/Common/.*_vs"], "grep-regex.text.golden");
}

#[test]
fn grep_cidr() {
    assert_grep_matches(&["--cidr", "10.0.0.0/8"], "grep-cidr.text.golden");
}

#[test]
fn grep_reverse() {
    assert_grep_matches(
        &["/Common/www_vs", "--direction", "reverse"],
        "grep-reverse.text.golden",
    );
}

#[test]
fn grep_forward() {
    assert_grep_matches(
        &["/Common/www_vs", "--direction", "forward"],
        "grep-forward.text.golden",
    );
}

#[test]
fn grep_both() {
    assert_grep_matches(
        &["/Common/www_vs", "--direction", "both"],
        "grep-both.text.golden",
    );
}

#[test]
fn grep_max_depth() {
    assert_grep_matches(
        &["web_pool", "--max-depth", "1"],
        "grep-maxdepth.text.golden",
    );
}

#[test]
fn grep_include_body() {
    assert_grep_matches(&["web_pool", "--full"], "grep-full.text.golden");
}

#[test]
fn grep_json() {
    assert_grep_matches(&["web_pool", "--json"], "grep-json.json.golden");
}

#[test]
fn grep_no_match() {
    assert_grep_matches(&["no_such_object_xyz"], "grep-nomatch.text.golden");
}
