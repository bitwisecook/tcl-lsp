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

//! Variable-resolution conformance: every row of
//! `tests/data/variable_resolution_vectors.txt` must reproduce, under each
//! real tclsh on the machine, exactly the observable its column for that
//! release names.
//!
//! This is the data-and-pin phase of the variable domain: no analyser or VM
//! consumer is wired to these rows yet, so the interpreter matrix is the
//! only gate that can catch a wrong transcription — which is precisely why
//! it runs every row against every available release.

mod support;

use tcl_syntax::var_conformance::{vector_script, vectors};

/// Every row must answer, on every available release, exactly its column.
#[test]
fn vectors_match_real_tclsh() {
    let releases = support::available_releases();
    assert!(
        !releases.is_empty(),
        "no tclsh on this machine: set TCL_LSP_TCLSH84 … TCL_LSP_TCLSH91, \
         or put tclsh8.4 … tclsh9.1 on PATH",
    );
    for (release, tclsh) in releases {
        for vector in vectors() {
            let script = vector_script(&vector);
            let got = support::run_script(&tclsh, &script).unwrap_or_else(|stderr| {
                panic!(
                    "vector line {} on Tcl {}: the rendered script is not runnable \
                     (every row catches its own errors, so this is a renderer bug)\
                     \nscript:\n{script}\nstderr: {stderr}",
                    vector.line,
                    release.version_string(),
                )
            });
            assert_eq!(
                got,
                vector.wants.get(release),
                "vector line {} on Tcl {} (ns={} frame={:?} probe={}): tclsh \
                 disagrees with the table\nscript:\n{script}",
                vector.line,
                release.version_string(),
                vector.ns,
                vector.frame,
                vector.probe.to_tcl(),
            );
        }
    }
}

/// A row with no release-tagged column must mean the same thing on every
/// release, and a tagged one must actually differ — a tagged row that is
/// uniform is a row someone tagged by mistake.
#[test]
fn tagging_matches_whether_the_releases_disagree() {
    for vector in vectors() {
        if vector.wants.is_release_tagged() {
            assert!(
                !vector.wants.is_uniform(),
                "vector line {}: release-tagged but every column is the same",
                vector.line,
            );
        }
    }
}
