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

//! Behaviour-driven coverage of the `tcl-lsp-core` references and rename
//! providers (`references::references`, `rename::rename`,
//! `rename::prepare_rename`, plus `is_safe_symbol_name`).
//!
//! These tests are derived from the providers' public API (see
//! `src/references.rs` / `src/rename.rs`) and from Tcl name-resolution
//! semantics, with the Tcl-semantic facts pinned to real C-Tcl
//! (tclsh8.6 / tclsh9.0 via `scripts/dev/tclsh_check.sh`).
//!
//! C-Tcl proof model
//! A "reference" is a genuine call/use site in runnable Tcl, and a rename is
//! correct exactly when the renamed script still resolves the same way Tcl
//! itself would. Every snippet below is a COMPLETE runnable Tcl script in
//! which the proc/method is actually invoked, or the variable actually read,
//! at the sites asserted as references. The reference/rename SET (which
//! positions) is an editor-structural fact about the parse, so it is asserted
//! structurally; the proof that the set is *right* is that those positions are
//! exactly the call/use sites Tcl's own name resolution binds together — and
//! the sites it keeps APART (a same-named local in another proc, a same-named
//! proc in another namespace) are left untouched. Each test cites the
//! corresponding `tclsh:` observation.
//!
//! Verified C-Tcl facts (tclsh8.6 + tclsh9.0, identical output):
//!   * `proc greet {} {return hi}; puts [greet]; puts [greet]`  -> hi / hi
//!     (the two `greet` words are real invocations of the one proc).
//!   * `set x 1; puts $x; puts $x`  -> 1 / 1  (two real reads of `x`).
//!   * `proc a {} {set v 10; return $v}; proc b {} {set v 20; return $v}`
//!     -> a=10, b=20  (each `v` is a distinct local; renaming a's `v` must
//!     not touch b's `v`).
//!   * namespace: `::myns::greet` and a short `greet` inside
//!     `namespace eval ::myns` both invoke the one proc -> ns-hi / ns-hi;
//!     and `::a::helper` vs `::b::helper` are two independent procs
//!     -> a / b  (renaming one must not touch the other).
//!   * renamed forms still run: `salute`/`salute`, `$y`/`$y`,
//!     `::myns::hello` + short `hello` -> identical output to the original.

// Test column math indexes tiny in-memory sources; a `find`/`len` result
// always fits u32, so the pedantic truncation the lint warns of can't occur.
#![allow(clippy::cast_possible_truncation)]

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::references::references;
use tcl_lsp_core::rename::{TextEdit, is_safe_symbol_name, prepare_rename, rename};
use tcl_registry::CommandRegistry;

mod brace_literal_names;
mod command_dispatch;
mod namespaces;
mod reference_sets;
mod rename_edits;
mod variable_cells;

/// Build an analysis exactly the way the existing port files do
/// (`call_hierarchy.rs`, `selection_range.rs`).
fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

/// The set of start lines touched by a list of reference ranges, sorted and
/// de-duplicated — the load-bearing structural fact for most assertions.
fn ref_lines(ranges: &[LspRange]) -> Vec<u32> {
    let mut v: Vec<u32> = ranges.iter().map(|r| r.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The set of start lines touched by a list of rename edits.
fn edit_lines(edits: &[TextEdit]) -> Vec<u32> {
    let mut v: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Select a validated Tcl 9.0 or 8.6 oracle for transformed rename output.
/// `None` skips the execution leg when neither optional release is available.
fn find_tclsh() -> Option<tcl_test_support::Tclsh> {
    // Ask the shared owner about both preferred releases before choosing, so
    // a configured required lane cannot be hidden by the other interpreter.
    let tcl_9_0 = tcl_test_support::witness_tclsh(tcl_dialect::TclVersion::V9_0);
    let tcl_8_6 = tcl_test_support::witness_tclsh(tcl_dialect::TclVersion::V8_6);
    tcl_9_0.or(tcl_8_6)
}

/// Apply line-based [`lsp_core::rename::TextEdit`]s to `src`.
fn apply_edits(src: &str, edits: &[tcl_lsp_core::rename::TextEdit]) -> String {
    let mut lines: Vec<String> = src.lines().map(str::to_owned).collect();
    let mut sorted: Vec<_> = edits.iter().collect();
    sorted.sort_by_key(|e| {
        (
            std::cmp::Reverse(e.range.start_line),
            std::cmp::Reverse(e.range.start_character),
        )
    });
    for e in sorted {
        assert_eq!(
            e.range.start_line, e.range.end_line,
            "single-line edits expected in this fixture"
        );
        let line = &mut lines[e.range.start_line as usize];
        let (s, t) = (
            e.range.start_character as usize,
            e.range.end_character as usize,
        );
        line.replace_range(s..t, &e.new_text);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

fn hover_text(src: &str, line: u32, character: u32, analysis: &AnalysisResult) -> Option<String> {
    tcl_lsp_core::hover::hover(
        src,
        line,
        character,
        analysis,
        Some(tcl_registry::model::ingress::static_context_for("tcl8.6").commands()),
    )
    .map(|h| h.value)
}

// Review N05 (naming-migration-review-20261022.md): rename and find-references
// settle every occurrence once. A rename may refuse, but it never edits an
// occurrence find-references does not report for the same binding, and an
// accepted rename keeps the program's observable output (tclsh 8.6.18 /
// 9.0.4 outputs recorded per fixture).

/// `(fixture, declaration line, declaration column, tclsh output)`.
const N05_FIXTURES: &[(&str, u32, u32, &str)] = &[
    // The forced import replaces `b::foo`; the call runs `a::foo`: "A".
    (
        "namespace eval a {namespace export *; proc foo {} {return A}}\nnamespace eval b {\n    proc foo {} {return B}\n    namespace import -force ::a::*\n    puts [foo]\n}\n",
        2,
        10,
        "A\n",
    ),
    // Before the forced import the call is the local; after it, the import: "B A".
    (
        "namespace eval a {namespace export *; proc foo {} {return A}}\nnamespace eval b {\n    proc foo {} {return B}\n    puts [foo]\n    namespace import -force ::a::*\n    puts [foo]\n}\n",
        2,
        10,
        "B\nA\n",
    ),
    // A forgotten import leaves room for the local definition: "B".
    (
        "namespace eval a {namespace export *; proc foo {} {return A}}\nnamespace eval b {\n    namespace import ::a::*\n    namespace forget ::a::foo\n    proc foo {} {return B}\n    puts [foo]\n}\n",
        4,
        10,
        "B\n",
    ),
    // A redefinition after the forced import is the local again: "C".
    (
        "namespace eval a {namespace export *; proc foo {} {return A}}\nnamespace eval b {\n    proc foo {} {return B}\n    namespace import -force ::a::*\n    proc foo {} {return C}\n    puts [foo]\n}\n",
        4,
        10,
        "C\n",
    ),
];

#[test]
fn n05_rename_never_edits_an_occurrence_references_does_not_report() {
    for (src, line, character, _) in N05_FIXTURES {
        let analysis = analyse(src);
        let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let refs = ref_lines(&references(
            src, profile, *line, *character, &analysis, true,
        ));
        let edits = rename(src, profile, *line, *character, "baz", &analysis, None);
        for edited in edit_lines(&edits) {
            assert!(
                refs.contains(&edited),
                "rename edited line {edited}, which references ({refs:?}) does not report:\n{src}"
            );
        }
    }
}

#[test]
fn n05_an_accepted_rename_keeps_the_program_output() {
    let Some(tclsh) = find_tclsh() else {
        eprintln!("skipping: no pinned Tcl 9.0/8.6 oracle is available");
        return;
    };
    for (src, line, character, expected) in N05_FIXTURES {
        let original = tcl_test_support::run_script(&tclsh.path, src.as_bytes())
            .unwrap_or_else(|error| panic!("running Tcl oracle: {error}"));
        assert_eq!(
            String::from_utf8_lossy(&original.stdout),
            *expected,
            "{src}"
        );
        let analysis = analyse(src);
        let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
        let edits = rename(src, profile, *line, *character, "baz", &analysis, None);
        if edits.is_empty() {
            continue;
        }
        let renamed = apply_edits(src, &edits);
        let outcome = tcl_test_support::run_script(&tclsh.path, renamed.as_bytes())
            .unwrap_or_else(|error| panic!("running Tcl oracle: {error}"));
        assert!(outcome.success(), "renamed script failed:\n{renamed}");
        assert_eq!(
            String::from_utf8_lossy(&outcome.stdout),
            *expected,
            "the rename changed the program's output:\n{renamed}"
        );
    }
}
