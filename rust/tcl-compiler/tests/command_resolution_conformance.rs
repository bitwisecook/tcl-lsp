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

//! Command-resolution conformance for the static analyser: the settled
//! `resolved_qualified_name` on every recorded call site must agree with
//! the shared vector table (`tcl_syntax::naming::conformance`, itself
//! pinned against real tclsh), for every vector the analyser can express.
//!
//! The analyser does not track `namespace path` declarations, so
//! path-carrying vectors are skipped here (the executing backends — the
//! bytecode VM and the WASM runtime — cover them); if the analyser gains
//! path tracking, delete the filter and this test tightens itself.

use tcl_compiler::analyser::Analyser;
use tcl_syntax::naming::conformance::vectors;

/// Render a vector as analysable source: the definitions as procs, and the
/// call inside `namespace eval {ns}` — the same shapes `vector_script`
/// executes, minus the runtime plumbing.
fn vector_source(ns: &str, defs: &[String], call: &str) -> String {
    use std::fmt::Write as _;
    let mut src = String::new();
    for def in defs {
        if let Some((holder, _)) = def.rsplit_once("::")
            && !holder.is_empty()
            && holder != ":"
        {
            let _ = writeln!(src, "namespace eval {holder} {{}}");
        }
        let _ = writeln!(src, "proc {def} {{}} {{ return {def} }}");
    }
    if ns == "::" {
        let _ = writeln!(src, "{call}");
    } else {
        let _ = writeln!(src, "namespace eval {ns} {{ {call} }}");
    }
    src
}

#[test]
fn analyser_settlement_matches_every_pathless_vector() {
    for v in vectors().iter().filter(|v| v.path.is_empty()) {
        let src = vector_source(&v.ns, &v.defs, &v.call);
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(&src, "tcl8.6");
        let inv = analysis
            .command_invocations
            .iter()
            .find(|i| i.name == v.call)
            .unwrap_or_else(|| {
                panic!(
                    "vector line {}: no `{}` invocation recorded for source:\n{src}",
                    v.line, v.call
                )
            });
        let resolved = inv
            .resolved_qualified_name
            .as_deref()
            .unwrap_or_else(|| panic!("vector line {}: unresolved invocation", v.line));
        // For an unresolvable call (`want` = None) the analyser keeps its
        // local-first guess (the first candidate) for cross-file
        // consumers — pin that exact behaviour so it can't silently
        // change.
        let expected = v.want.clone().unwrap_or_else(|| {
            tcl_syntax::naming::command_resolution_candidates::<&str>(&v.ns, &[], &v.call)
                .into_iter()
                .next()
                .expect("at least one candidate")
        });
        assert_eq!(
            resolved,
            expected.as_str(),
            "vector line {} (ns={} defs={:?} call={}): analyser settlement disagrees with \
             C Tcl (or, for an unresolvable call, with the pinned local-first guess)\
             \nsource:\n{src}",
            v.line,
            v.ns,
            v.defs,
            v.call,
        );
    }
}
