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
//!
//! The resolver is release-agnostic, so it asserts the newest column; the
//! tclsh leg is a matrix, asserting each release's own column against that
//! release's own interpreter.

mod support;

use tcl_syntax::naming::conformance::{vector_script, vectors};
use tcl_syntax::naming::resolve_command_with;

/// The sentinel for a row whose scenario the release cannot even set up
/// (a `namespace path` row before 8.5).
const UNSUPPORTED: &str = "!ERROR";

/// The pure resolver must reproduce every vector's winner on the newest
/// modelled release.
#[test]
fn canonical_resolver_matches_every_vector() {
    for v in vectors() {
        let got = resolve_command_with(&v.ns, &v.path, &v.call, |candidate| {
            v.defs.iter().any(|d| d == candidate)
        });
        assert_eq!(
            got,
            v.want(),
            "vector line {} (ns={} path={:?} defs={:?} call={}): resolver disagrees",
            v.line,
            v.ns,
            v.path,
            v.defs,
            v.call,
        );
    }
}

/// A row with no release-tagged column must mean the same thing on every
/// release — otherwise the ladder is silently asserting a value nobody
/// checked.
#[test]
fn untagged_rows_are_uniform_across_the_ladder() {
    for v in vectors() {
        assert!(
            v.wants.is_release_tagged() || v.wants.is_uniform(),
            "vector line {}: an untagged row must be uniform",
            v.line,
        );
    }
}

/// Every vector's winner must match what a real tclsh dispatches, per
/// release — this is what keeps the table (and through it every conforming
/// implementation) pinned to C Tcl rather than to our own beliefs.
#[test]
fn vectors_match_real_tclsh() {
    let releases = support::available_releases();
    assert!(
        !releases.is_empty(),
        "no tclsh on this machine: set TCL_LSP_TCLSH84 … TCL_LSP_TCLSH91, \
         or put tclsh8.4 … tclsh9.1 on PATH",
    );
    for (release, tclsh) in releases {
        for v in vectors() {
            let script = vector_script(&v);
            let want = v.wants.get(release);
            let got = match support::run_script(&tclsh, &script) {
                Ok(stdout) => stdout,
                Err(stderr) => {
                    assert_eq!(
                        want,
                        UNSUPPORTED,
                        "vector line {} on Tcl {}: the script failed but the table \
                         expects {want:?}\nscript:\n{script}\nstderr: {stderr}",
                        v.line,
                        release.version_string(),
                    );
                    continue;
                }
            };
            assert_eq!(
                got,
                want,
                "vector line {} on Tcl {} (ns={} path={:?} defs={:?} call={}): \
                 tclsh disagrees with the table\nscript:\n{script}",
                v.line,
                release.version_string(),
                v.ns,
                v.path,
                v.defs,
                v.call,
            );
        }
    }
}
