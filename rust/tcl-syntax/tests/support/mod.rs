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

//! The real-tclsh matrix the conformance suites pin their vectors against.
//!
//! One interpreter per modelled release, keyed the way
//! `tcl-vm/tests/cross_version_info_surface_e2e.rs` keys it:
//! `TCL_LSP_TCLSH84` … `TCL_LSP_TCLSH91` name a binary explicitly, and each
//! release otherwise falls back to its conventional PATH name.  A release
//! with no binary is skipped **loudly** — the suite says which column went
//! unchecked rather than passing in silence.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tcl_dialect::TclVersion;

/// Environment variable and PATH names for one release of the matrix.
struct ReleaseBinary {
    release: TclVersion,
    env: &'static str,
    names: &'static [&'static str],
}

const MATRIX: &[ReleaseBinary] = &[
    ReleaseBinary {
        release: TclVersion::V8_4,
        env: "TCL_LSP_TCLSH84",
        names: &["tclsh8.4"],
    },
    ReleaseBinary {
        release: TclVersion::V8_5,
        env: "TCL_LSP_TCLSH85",
        names: &["tclsh8.5"],
    },
    ReleaseBinary {
        release: TclVersion::V8_6,
        env: "TCL_LSP_TCLSH86",
        names: &["tclsh8.6"],
    },
    ReleaseBinary {
        release: TclVersion::V9_0,
        env: "TCL_LSP_TCLSH90",
        names: &["tclsh9.0"],
    },
    ReleaseBinary {
        release: TclVersion::V9_1,
        env: "TCL_LSP_TCLSH91",
        names: &["tclsh9.1"],
    },
];

/// Every release of the matrix that has a usable interpreter on this
/// machine, oldest first.
///
/// Prints one `skipping` line per release it could not find, so a run that
/// only covers part of the ladder says so in its output.
pub fn available_releases() -> Vec<(TclVersion, PathBuf)> {
    let mut found = Vec::new();
    for entry in MATRIX {
        match locate(entry) {
            Some(path) => found.push((entry.release, path)),
            None => eprintln!(
                "skipping Tcl {}: no interpreter (set {} or put {} on PATH)",
                entry.release.version_string(),
                entry.env,
                entry.names.join(" / "),
            ),
        }
    }
    found
}

/// Run `script` on `tclsh`, returning its trimmed standard output.
///
/// # Errors
/// Returns the interpreter's standard error whenever the script exits
/// non-zero **or** writes anything to standard error.  Both halves matter:
/// `tclsh` reading a script from standard input reports a failed command
/// and then carries on with the next one, still exiting 0, so a row whose
/// setup a release rejects would otherwise report whatever the *rest* of
/// the script happened to do.
pub fn run_script(tclsh: &Path, script: &str) -> Result<String, String> {
    let mut child = Command::new(tclsh)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|why| panic!("spawn {}: {why}", tclsh.display()));
    child
        .stdin
        .as_mut()
        .expect("tclsh stdin")
        .write_all(script.as_bytes())
        .expect("write script");
    let output = child.wait_with_output().expect("tclsh run");
    let complaint = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if output.status.success() && complaint.is_empty() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else if complaint.is_empty() {
        Err(format!("exited with {}", output.status))
    } else {
        Err(complaint)
    }
}

/// The interpreter for one release: the environment override first, then
/// the conventional PATH names.
fn locate(entry: &ReleaseBinary) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var(entry.env) {
        let path = PathBuf::from(explicit);
        if path.exists() {
            return Some(path);
        }
    }
    for name in entry.names {
        if runs(name) {
            return Some(PathBuf::from(name));
        }
    }
    None
}

/// Whether `name` names an interpreter that answers `info patchlevel` with
/// the release its name claims — a PATH `tclsh8.6` that is really a 9.0
/// would silently pin the wrong column.
fn runs(name: &str) -> bool {
    let Ok(mut child) = Command::new(name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let _ = child
        .stdin
        .as_mut()
        .expect("tclsh stdin")
        .write_all(b"puts [info tclversion]\n");
    let Ok(output) = child.wait_with_output() else {
        return false;
    };
    let reported = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    output.status.success() && name.ends_with(&reported)
}
