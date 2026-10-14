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

//! `tcl pkg build` runs the project's build script deprivileged, and still
//! builds: the script writes into the project and finds its interpreter (here
//! possibly a wrapper that execs one elsewhere), while a network it did not
//! declare is refused wherever the host enforces that — which the audit log
//! records.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The first `tclsh` on `PATH`, when there is one.
fn tclsh_on_path() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join("tclsh"))
        .find(|candidate| candidate.is_file())
}

/// A fresh project directory of its own.
fn project(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after the epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcl-pkg-build-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("creates the project");
    dir
}

/// `tcl pkg build` in `dir`, with every per-user directory inside it.
fn build(dir: &Path) -> (String, String, i32) {
    let home = dir.join("home");
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["pkg", "build"])
        .current_dir(dir)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env_remove("TCL_VENV")
        .output()
        .expect("runs tcl");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

#[test]
fn a_build_script_builds_in_the_project_without_an_undeclared_network() {
    if tclsh_on_path().is_none() {
        eprintln!("skipped: no tclsh on PATH");
        return;
    }
    let dir = project("confined");
    std::fs::write(
        dir.join("tclpkg.tcl"),
        "package demo\nversion 1.0.0\nbuild build.tcl\n",
    )
    .expect("writes the manifest");
    std::fs::write(
        dir.join("tclpkg.toml"),
        "[build]\nallow-build-scripts = true\ntrusted = [\"demo\"]\n",
    )
    .expect("writes the policy");
    // A listener the script could reach if it had the network.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listens");
    let port = listener.local_addr().expect("bound").port();
    std::fs::write(
        dir.join("build.tcl"),
        format!(
            "set f [open built.txt w]; puts -nonewline $f built; close $f\n\
             catch {{socket 127.0.0.1 {port}}} net\n\
             set f [open net.txt w]; puts -nonewline $f $net; close $f\n"
        ),
    )
    .expect("writes the build script");

    let (stdout, stderr, code) = build(&dir);
    assert_eq!(code, 0, "stdout: {stdout}\nstderr: {stderr}");
    assert_eq!(
        std::fs::read_to_string(dir.join("built.txt")).expect("the script built"),
        "built"
    );
    let audit = std::fs::read_to_string(dir.join("home/state/tcl-lsp/pkg-audit.log"))
        .expect("the build was audited");
    let record = audit
        .lines()
        .find(|line| line.contains("\"label\":\"pkg-build\""))
        .expect("a pkg-build record");
    let net = std::fs::read_to_string(dir.join("net.txt")).expect("the script reported");
    assert_eq!(
        net.starts_with("sock"),
        !record.contains("\"network_enforced\":true"),
        "{net}\n{record}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
