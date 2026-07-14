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

//! Command-resolution conformance: the canonical resolver
//! (`naming::resolve_command_with`) must agree with every vector in
//! `tests/data/command_resolution_vectors.txt`, and the vectors themselves
//! must agree with real tclsh — so the table can never drift from C Tcl,
//! and no consumer can drift from the table.

use tcl_syntax::naming::conformance::{ResolutionVector, vector_script, vectors};
use tcl_syntax::naming::resolve_command_with;

/// The pure resolver must reproduce every vector's winner.
#[test]
fn canonical_resolver_matches_every_vector() {
    for v in vectors() {
        let got = resolve_command_with(&v.ns, &v.path, &v.call, |candidate| {
            v.defs.iter().any(|d| d == candidate)
        });
        assert_eq!(
            got, v.want,
            "vector line {} (ns={} path={:?} defs={:?} call={}): resolver disagrees",
            v.line, v.ns, v.path, v.defs, v.call,
        );
    }
}

/// Find a tclsh to pin against: `TCL_LSP_TCLSH`, then common PATH names.
fn find_tclsh() -> Option<std::path::PathBuf> {
    if let Ok(explicit) = std::env::var("TCL_LSP_TCLSH") {
        let p = std::path::PathBuf::from(explicit);
        if p.exists() {
            return Some(p);
        }
    }
    for name in ["tclsh9.0", "tclsh8.6", "tclsh"] {
        if let Ok(out) = std::process::Command::new(name).arg("--version").output()
            && (out.status.success() || !out.stderr.is_empty() || !out.stdout.is_empty())
        {
            return Some(std::path::PathBuf::from(name));
        }
    }
    None
}

fn run_vector_under(tclsh: &std::path::Path, v: &ResolutionVector) -> String {
    use std::io::Write;
    let script = vector_script(v);
    let mut child = std::process::Command::new(tclsh)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn tclsh");
    child
        .stdin
        .as_mut()
        .expect("tclsh stdin")
        .write_all(script.as_bytes())
        .expect("write script");
    let out = child.wait_with_output().expect("tclsh run");
    assert!(
        out.status.success(),
        "vector line {}: tclsh failed on script:\n{script}\nstderr: {}",
        v.line,
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Every vector's `want` must match what a real tclsh dispatches — this is
/// what keeps the table (and through it every conforming implementation)
/// pinned to C Tcl rather than to our own beliefs.  Skips silently when no
/// tclsh is installed (CI's heavy suite and dev machines have one; set
/// `TCL_LSP_TCLSH` to pin a specific build, e.g. a fresh 9.0 tree).
#[test]
fn vectors_match_real_tclsh() {
    let Some(tclsh) = find_tclsh() else {
        eprintln!("skipping: no tclsh on PATH (set TCL_LSP_TCLSH to enable)");
        return;
    };
    for v in vectors() {
        let got = run_vector_under(&tclsh, &v);
        let want = v.want.clone().unwrap_or_else(|| "-".to_string());
        assert_eq!(
            got,
            want,
            "vector line {} (ns={} path={:?} defs={:?} call={}): tclsh disagrees with the table\nscript:\n{}",
            v.line,
            v.ns,
            v.path,
            v.defs,
            v.call,
            vector_script(&v),
        );
    }
}
