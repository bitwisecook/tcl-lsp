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
//! pinned against real tclsh) — including the `namespace path`-carrying
//! vectors, which exercise the analyser's static `namespace path`
//! tracking (`handle_namespace_path_command` → the settlement pass).

use tcl_compiler::analyser::Analyser;
use tcl_syntax::naming::conformance::vectors;

/// Render a vector as analysable source: the definitions as procs, any
/// `namespace path` as a literal declaration in the call namespace, and
/// the call inside `namespace eval` — the same shapes `vector_script`
/// executes, minus the runtime plumbing.
///
/// Namespace / definition fields are **constructed keys**: a key may hold a
/// colon-named segment (issue #934: `":::"` is the proc — or namespace —
/// named `:`) with no absolute written spelling, so definitions descend the
/// holder chain one `namespace eval {tail}` at a time with brace-quoted
/// relative tails, exactly as `vector_script` does for real tclsh.
fn vector_source(ns: &str, path: &[String], defs: &[String], call: &str) -> String {
    use std::fmt::Write as _;
    use tcl_syntax::naming::key_holder_and_tail;
    fn run(ns_key: &str, body: &str) -> String {
        if ns_key.is_empty() || ns_key == "::" {
            return body.to_owned();
        }
        let (holder, tail) = key_holder_and_tail(ns_key);
        run(holder, &format!("namespace eval {{{tail}}} {{ {body} }}"))
    }
    let mut src = String::new();
    for def in defs {
        let (holder, tail) = key_holder_and_tail(def);
        let _ = writeln!(
            src,
            "{}",
            run(
                holder,
                &format!("proc {{{tail}}} {{}} {{ return {{{def}}} }}"),
            ),
        );
    }
    if !path.is_empty() {
        let entries = path.join(" ");
        let _ = writeln!(src, "{}", run(ns, &format!("namespace path {{{entries}}}")));
    }
    let _ = writeln!(src, "{}", run(ns, call));
    src
}

#[test]
fn analyser_settlement_matches_every_vector() {
    for v in vectors() {
        let src = vector_source(&v.ns, &v.path, &v.defs, &v.call);
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
            tcl_syntax::naming::command_resolution_candidates(&v.ns, &v.path, &v.call)
                .into_iter()
                .next()
                .expect("at least one candidate")
        });
        assert_eq!(
            resolved,
            expected.as_str(),
            "vector line {} (ns={} path={:?} defs={:?} call={}): analyser settlement disagrees \
             with C Tcl (or, for an unresolvable call, with the pinned local-first guess)\
             \nsource:\n{src}",
            v.line,
            v.ns,
            v.path,
            v.defs,
            v.call,
        );
    }
}
