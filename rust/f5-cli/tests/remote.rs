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

//! Differential tests for the `f5 fetch` / `push` / `pull` remote verbs.
//!
//! These verbs talk to a *live* BIG-IP, so the transport itself cannot be
//! golden-tested offline. The byte-testable surfaces are exercised here
//! against the built `f5-query` binary:
//!
//! - `push --dry-run`: the request summary on stderr + the JSON payload body
//!   on stdout, asserted for the `fullPath`-vs-`name` target-selection
//!   fallback.
//! - credential resolution errors (no host / user / password) for all three
//!   verbs, including the env-supplies-host/user precedence.
//! - payload errors: missing file (`[Errno 2] …`) and invalid-JSON position
//!   (`Expecting value: line L column C (char N)`).
//! - the SSH-transport deferral and clap arg validation (unknown kind /
//!   transport).
//!
//! Self-contained: no live connection and no external tool at test time. Every child
//! process has the ambient `F5_*` environment scrubbed; each case sets exactly
//! the variables it needs, so the host's env can never leak in.

use std::path::PathBuf;
use std::process::{Command, Output};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn payload() -> String {
    fixtures_dir()
        .join("remote-payload.json")
        .to_string_lossy()
        .into_owned()
}

/// Run `f5-query <args…>` with the `F5_*` env scrubbed and `env_extra` applied,
/// returning `(stdout, stderr, exit_code)`.
fn run(args: &[&str], env_extra: &[(&str, &str)]) -> (String, String, i32) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_f5-query"));
    cmd.args(args);
    // Scrub every credential env var so the host environment cannot leak in.
    for var in [
        "F5_HOST",
        "F5_USER",
        "F5_PASSWORD",
        "F5_PORT",
        "F5_SSH_PORT",
        "XDG_CONFIG_HOME",
    ] {
        cmd.env_remove(var);
    }
    for (k, v) in env_extra {
        cmd.env(k, v);
    }
    let Output {
        status,
        stdout,
        stderr,
    } = cmd.output().expect("failed to spawn f5-query binary");
    (
        String::from_utf8_lossy(&stdout).into_owned(),
        String::from_utf8_lossy(&stderr).into_owned(),
        status.code().unwrap_or(-1),
    )
}

// The dry-run summary prefers `fullPath`, falling back to `name`.
#[test]
fn push_dry_run_name_only_target() {
    let dir = std::env::temp_dir();
    let p = dir.join("f5-remote-parity-nameonly.json");
    std::fs::write(&p, r#"{"name":"only_name"}"#).unwrap();
    let p = p.to_string_lossy().into_owned();
    let (out, err, code) = run(
        &[
            "push",
            "virtual",
            &p,
            "--host",
            "h",
            "--user",
            "admin",
            "--dry-run",
        ],
        &[("F5_PASSWORD", "x")],
    );
    assert_eq!(code, 0);
    assert_eq!(err, "would PUT (replace) virtual only_name\n");
    assert_eq!(out, "{\n  \"name\": \"only_name\"\n}\n");
    let _ = std::fs::remove_file(&p);
}

// credential resolution errors (no host / user / password) — all three verbs

#[test]
fn missing_host_errors_for_all_verbs() {
    let p = payload();
    for args in [
        vec!["push", "virtual", p.as_str(), "--no-prompt"],
        vec!["pull", "virtual", "/Common/x", "--no-prompt"],
        vec!["fetch", "--no-prompt"],
    ] {
        let (out, err, code) = run(&args, &[]);
        assert_eq!(code, 2, "{args:?} should exit 2");
        assert!(out.is_empty(), "{args:?} stdout empty");
        assert_eq!(
            err, "error: no host configured (set --host or F5_HOST)\n",
            "{args:?} stderr"
        );
    }
}

#[test]
fn missing_user_after_env_host() {
    let p = payload();
    let (_out, err, code) = run(&["push", "virtual", &p, "--no-prompt"], &[("F5_HOST", "h")]);
    assert_eq!(code, 2);
    assert_eq!(err, "error: no user configured (set --user or F5_USER)\n");
}

#[test]
fn missing_password_after_env_host_and_user() {
    let p = payload();
    let (_out, err, code) = run(
        &["push", "virtual", &p, "--no-prompt"],
        &[("F5_HOST", "h"), ("F5_USER", "admin")],
    );
    assert_eq!(code, 2);
    assert_eq!(
        err,
        "error: no password configured (set --password or F5_PASSWORD)\n"
    );
}

// payload errors: missing file + invalid-JSON position

#[test]
fn missing_payload_file_errno_message() {
    let missing = "/tmp/f5-remote-parity-does-not-exist.json";
    let (out, err, code) = run(
        &[
            "push",
            "virtual",
            missing,
            "--host",
            "h",
            "--user",
            "admin",
            "--dry-run",
        ],
        &[("F5_PASSWORD", "x")],
    );
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert_eq!(
        err,
        format!("error: [Errno 2] No such file or directory: '{missing}'\n")
    );
}

#[test]
fn invalid_json_position() {
    // (contents, expected message tail) — the position is recomputed from the
    // first non-whitespace char (or end-of-input) to match `json`
    // error position.
    let cases: &[(&str, &str)] = &[
        ("", "Expecting value: line 1 column 1 (char 0)"),
        ("   not json", "Expecting value: line 1 column 4 (char 3)"),
        ("\n\n   xyz", "Expecting value: line 3 column 4 (char 5)"),
        ("   \n  \n ", "Expecting value: line 3 column 2 (char 8)"),
    ];
    let dir = std::env::temp_dir();
    for (idx, (contents, tail)) in cases.iter().enumerate() {
        let p = dir.join(format!("f5-remote-parity-badjson-{idx}.json"));
        std::fs::write(&p, contents).unwrap();
        let path = p.to_string_lossy().into_owned();
        let (_out, err, code) = run(
            &[
                "push",
                "virtual",
                &path,
                "--host",
                "h",
                "--user",
                "admin",
                "--dry-run",
            ],
            &[("F5_PASSWORD", "x")],
        );
        assert_eq!(code, 2, "case {idx}: {contents:?}");
        assert_eq!(
            err,
            format!("error: invalid JSON payload: {tail}\n"),
            "case {idx}: {contents:?}"
        );
        let _ = std::fs::remove_file(&p);
    }
}

// SSH transport (connection failure) + clap arg validation

// The SSH transport is an in-process `russh` client. Pointed at a closed local
// port it fails to connect, so the fetch exits 2 with a `fetch failed:` error.
// This is the offline-testable slice of the SSH flow (the live tmsh/SFTP
// round-trip needs a real device, exactly like REST).
#[test]
fn ssh_transport_connection_refused() {
    let (out, err, code) = run(
        &[
            "fetch",
            "--host",
            "127.0.0.1",
            "--ssh-port",
            "1",
            "--user",
            "admin",
            "--transport",
            "ssh",
            "--timeout",
            "5",
        ],
        &[("F5_PASSWORD", "x")],
    );
    assert_eq!(code, 2);
    assert!(out.is_empty(), "stdout empty on ssh failure");
    assert!(
        err.starts_with("error: fetch failed: ssh connect to 127.0.0.1"),
        "stderr: {err}"
    );
}

#[test]
fn unknown_kind_rejected_by_clap() {
    let p = payload();
    let (_out, _err, code) = run(&["push", "bogus", &p, "--host", "h"], &[]);
    assert_eq!(code, 2, "clap rejects an unknown object kind");
}

#[test]
fn unknown_transport_rejected_by_clap() {
    let (_out, _err, code) = run(&["fetch", "--host", "h", "--transport", "bogus"], &[]);
    assert_eq!(code, 2, "clap rejects an unknown transport");
}
