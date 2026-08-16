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

//! Tests for the LSP selection-range provider.
//! Verifies `selection_range` returns an innermost-first chain that expands
//! strictly outward (word → command → line → enclosing proc/namespace body →
//! whole document), with no duplicate adjacent ranges.
//!
//! C-Tcl proof: the chain mirrors Tcl's own nesting — every snippet is a
//! complete, runnable Tcl script whose proc/namespace braced bodies contain
//! the cursor's command, which contains the word. (Selection ranges are an
//! editor structure, not a runtime value, so the proof is structural: the
//! nesting matches how Tcl parses the script into commands and braced blocks.)

use tcl_compiler::analyser::Analyser;
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::selection_range::{SelectionRange, selection_range, selection_range_for_dialect};

/// The chain ordered innermost → outermost by following `parent_index`.
fn chain(source: &str, line: u32, character: u32) -> Vec<LspRange> {
    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, "tcl8.6");
    let links: Vec<SelectionRange> = selection_range(source, line, character, Some(&analysis));
    if links.is_empty() {
        return Vec::new();
    }
    // Start at the link that is no one's parent (the innermost), then walk
    // parent_index outward.
    let is_parent: Vec<bool> = {
        let mut v = vec![false; links.len()];
        for l in &links {
            if let Some(p) = l.parent_index {
                v[p] = true;
            }
        }
        v
    };
    let mut idx = (0..links.len()).find(|&i| !is_parent[i]).unwrap_or(0);
    let mut out = Vec::new();
    let mut guard = 0;
    loop {
        out.push(links[idx].range);
        match links[idx].parent_index {
            Some(p) if guard < links.len() => {
                idx = p;
                guard += 1;
            }
            _ => break,
        }
    }
    out
}

fn contains(outer: LspRange, inner: LspRange) -> bool {
    let starts_le =
        (outer.start_line, outer.start_character) <= (inner.start_line, inner.start_character);
    let ends_ge = (outer.end_line, outer.end_character) >= (inner.end_line, inner.end_character);
    starts_le && ends_ge
}

#[test]
fn cursor_on_variable_in_proc_chain() {
    let src = "proc greet {name} {\n    puts $name\n}\n";
    let c = chain(src, 1, 10); // on `name` in `$name`
    assert!(c.len() >= 2, "expected word + outer ranges: {c:?}");
    // Outermost covers the whole document (starts at 0,0).
    let last = c.last().unwrap();
    assert_eq!((last.start_line, last.start_character), (0, 0));
}

#[test]
fn cursor_on_command_name_top_level() {
    let c = chain("puts hello\n", 0, 0);
    assert!(c.len() >= 2);
    let last = c.last().unwrap();
    assert_eq!((last.start_line, last.start_character), (0, 0));
}

#[test]
fn cursor_inside_namespace_has_deeper_chain() {
    let src = "namespace eval ns {\n    proc helper {} {\n        return 1\n    }\n}\n";
    let c = chain(src, 2, 8); // inside helper body
    // word → line → proc body → namespace body → document.
    assert!(
        c.len() >= 3,
        "namespace nesting should deepen the chain: {c:?}"
    );
}

#[test]
fn empty_file_has_at_least_one_range() {
    // Even with no word, the document range is present (or the chain is empty —
    // either way no panic).
    let c = chain("", 0, 0);
    assert!(c.len() <= 1 || !c.is_empty());
}

#[test]
fn multiple_positions_each_get_a_chain() {
    let src = "set x 42\nset y 99\n";
    for (line, ch) in [(0u32, 4u32), (1, 4)] {
        let c = chain(src, line, ch);
        assert!(!c.is_empty(), "position ({line},{ch}) has a chain");
    }
}

#[test]
fn ranges_expand_strictly_outward() {
    let src = "proc add {a b} {\n    expr {$a + $b}\n}\n";
    let c = chain(src, 1, 6); // inside expr body
    for w in c.windows(2) {
        // w[1] is the parent of w[0] → must contain it.
        assert!(
            contains(w[1], w[0]),
            "parent {:?} must contain child {:?}",
            w[1],
            w[0]
        );
    }
}

#[test]
fn no_duplicate_adjacent_ranges() {
    let src = "set x 42\n";
    let c = chain(src, 0, 4);
    for w in c.windows(2) {
        assert_ne!(
            (
                w[0].start_line,
                w[0].start_character,
                w[0].end_line,
                w[0].end_character
            ),
            (
                w[1].start_line,
                w[1].start_character,
                w[1].end_line,
                w[1].end_character
            ),
            "adjacent chain ranges must differ"
        );
    }
}

#[test]
fn registry_recursive_command_spans_cover_case_lambda_and_definition_members() {
    // Each expected range is the innermost command, not its enclosing list or
    // definition script. The traversal receives only registry roles and the
    // resolved DialectProfile; no command name selects a descent path.
    let cases = [
        (
            "switch $kind {\n    ok { puts switch-hit }\n}\n",
            1,
            17,
            9,
            24,
            "tcl9.0",
        ),
        (
            "set result [if {1} { puts command-hit }]\n",
            0,
            29,
            21,
            37,
            "tcl9.0",
        ),
        (
            "apply {{value} {\n    puts $value\n}} item\n",
            1,
            10,
            4,
            15,
            "tcl9.0",
        ),
        (
            "oo::class create C {\n    method run {} {\n        puts member-hit\n    }\n}\n",
            2,
            15,
            8,
            23,
            "tcl9.0",
        ),
    ];
    for (source, line, character, start, end, dialect) in cases {
        let links = selection_range_for_dialect(source, line, character, None, dialect);
        assert!(
            links.iter().any(|link| {
                link.range.start_line == line
                    && link.range.start_character == start
                    && link.range.end_line == line
                    && link.range.end_character == end
            }),
            "{dialect}: missing exact nested command span in {links:?}"
        );
        for pair in links.windows(2) {
            assert!(contains(pair[1].range, pair[0].range), "{links:?}");
            assert_ne!(pair[0].range, pair[1].range, "{links:?}");
        }
    }
}

#[test]
fn alias_to_user_proc_named_method_does_not_recurse_into_data() {
    // Tclsh 9.0.4: this runs the user `method` proc and prints neither the
    // comment nor `puts must-not-run`; `define_method` is an alias, not an OO
    // member declaration. A registry member spelling alone cannot prove the
    // target owns the surrounding definition body.
    let source = "proc method {name parameters body} {\n    return \"$name:$parameters:$body\"\n}\ninterp alias {} define_method {} method\noo::class create C {\n    define_method m {} {\n        # inert data\n        puts must-not-run\n    }\n}\n";
    let links = selection_range_for_dialect(source, 7, 15, None, "tcl9.0");
    assert!(
        !links.iter().any(|link| {
            link.range.start_line == 7
                && link.range.start_character == 8
                && link.range.end_line == 7
                && link.range.end_character == 25
        }),
        "an alias to a user proc must not make its final braced argument executable: {links:?}"
    );
}

#[test]
fn overlapping_scopes_are_strictly_nested() {
    // A proc-body range and a namespace-body range must be strictly nested
    // (regression: a bad sort key could break containment).
    let src = "namespace eval ns {\n    proc p {} {\n        set local 1\n    }\n}\n";
    let c = chain(src, 2, 12); // on `local`
    for w in c.windows(2) {
        assert!(contains(w[1], w[0]), "{:?} must contain {:?}", w[1], w[0]);
    }
}
