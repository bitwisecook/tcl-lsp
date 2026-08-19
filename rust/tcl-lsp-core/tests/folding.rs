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

//! Tests for the LSP folding-range provider.
//! Verifies `folding_ranges` produces Region folds for multi-line braced
//! bodies (proc/namespace/if/while/else) and Comment folds for comment blocks,
//! and that sibling folds are pairwise disjoint-or-nested (VS Code's tree
//! builder rejects partial overlaps).
//!
//! C-Tcl proof: the fold structure mirrors Tcl's parse — proc/namespace/if/
//! while are real commands whose braced bodies span the folded lines (verified
//! via tclsh: each snippet below is a complete, runnable Tcl script).

use tcl_lsp_core::folding::{FoldKind, FoldingRange, folding_ranges};
use tcl_registry::registry_for_dialect;

fn folds(source: &str) -> Vec<FoldingRange> {
    let registry = registry_for_dialect("tcl8.6");
    folding_ranges(
        source,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        registry,
    )
}

fn regions(source: &str) -> Vec<FoldingRange> {
    folds(source)
        .into_iter()
        .filter(|r| r.kind == FoldKind::Region)
        .collect()
}

fn comments(source: &str) -> Vec<FoldingRange> {
    folds(source)
        .into_iter()
        .filter(|r| r.kind == FoldKind::Comment)
        .collect()
}

#[test]
fn proc_body_folds_from_line_zero() {
    let src = "proc greet {name} {\n    puts \"Hello\"\n    puts \"$name\"\n}\n";
    let r = regions(src);
    assert!(!r.is_empty());
    let body = &r[0];
    assert_eq!(body.start_line, 0);
    assert!(body.end_line >= 2);
}

#[test]
fn namespace_body_folds() {
    let src = "namespace eval myns {\n    proc helper {} { return }\n}\n";
    let starts: std::collections::HashSet<u32> =
        regions(src).iter().map(|r| r.start_line).collect();
    assert!(starts.contains(&0), "namespace body fold starts at line 0");
}

#[test]
fn comment_block_folds_as_comment() {
    let src = "# This is a comment block\n# that spans multiple lines\n# explaining something\nproc foo {} { return }\n";
    let c = comments(src);
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].start_line, 0);
    assert_eq!(c[0].end_line, 2);
}

#[test]
fn if_and_while_bodies_fold_zero_to_two() {
    let if_src = "if {1} {\n    puts \"yes\"\n    puts \"really\"\n}\n";
    assert!(
        regions(if_src)
            .iter()
            .any(|r| r.start_line == 0 && r.end_line == 2)
    );
    let while_src = "while {1} {\n    puts \"loop\"\n    puts \"again\"\n}\n";
    assert!(
        regions(while_src)
            .iter()
            .any(|r| r.start_line == 0 && r.end_line == 2)
    );
}

#[test]
fn single_line_body_has_no_region_fold() {
    assert!(regions("proc foo {} { return 1 }\n").is_empty());
}

#[test]
fn empty_file_and_single_comment_have_no_folds() {
    assert!(folds("").is_empty());
    assert!(comments("# Just one comment\n").is_empty());
}

#[test]
fn if_else_bodies_are_disjoint() {
    // `} else {` must not put body1 and body2 on the same fold line (issue #182).
    let src = "if {1} {\n    puts \"yes\"\n    puts \"really\"\n} else {\n    puts \"no\"\n    puts \"nope\"\n}\n";
    let mut r: Vec<FoldingRange> = regions(src)
        .into_iter()
        .filter(|x| x.start_line == 0 || x.start_line == 3)
        .collect();
    r.sort_by_key(|x| (x.start_line, x.end_line));
    assert_eq!(r.len(), 2, "two sibling branch folds");
    assert!(
        r[0].end_line < r[1].start_line,
        "branch folds must not share a line"
    );
}

#[test]
fn elseif_chain_yields_four_disjoint_folds() {
    let src = "if {1} {\n    puts a\n} elseif {2} {\n    puts b\n} elseif {3} {\n    puts c\n} else {\n    puts d\n}\n";
    let mut r = regions(src);
    r.sort_by_key(|x| (x.start_line, x.end_line));
    assert_eq!(r.len(), 4, "one fold per branch: {r:?}");
    for w in r.windows(2) {
        assert!(
            w[0].end_line < w[1].start_line,
            "sibling folds share a line: {r:?}"
        );
    }
}

#[test]
fn nested_if_else_has_no_partial_overlaps() {
    // The whole fold set must be pairwise disjoint-or-nested.
    let src = "proc demo {x} {\n    if {$x} {\n        if {$x > 1} {\n            puts \"big\"\n            puts \"really big\"\n        } else {\n            puts \"small\"\n            puts \"really small\"\n        }\n    } else {\n        puts \"zero\"\n        puts \"none\"\n    }\n}\n";
    let r = folds(src);
    let contains = |o: &FoldingRange, i: &FoldingRange| {
        o.start_line <= i.start_line && i.end_line <= o.end_line
    };
    for (i, a) in r.iter().enumerate() {
        for b in &r[i + 1..] {
            if a.start_line == b.start_line && a.end_line == b.end_line {
                continue;
            }
            if contains(a, b) || contains(b, a) {
                continue;
            }
            let overlaps = a.end_line >= b.start_line && a.start_line <= b.end_line;
            assert!(
                !overlaps,
                "non-nested overlap {}..{} and {}..{}",
                a.start_line, a.end_line, b.start_line, b.end_line
            );
        }
    }
}

#[test]
fn incomplete_body_still_folds() {
    // An unterminated braced body (mid-edit) must still produce a fold so the
    // editor's folds don't flicker off on every trailing-brace delete.
    let src = "proc foo {} {\n    puts hi\n    puts there\n";
    let r = regions(src);
    assert!(!r.is_empty(), "unterminated proc body should still fold");
    assert!(r.iter().any(|x| x.start_line == 0));
}

#[test]
fn snit_method_bodies_fold() {
    // The folding walk consumes the registry definition-body grammar, so a
    // snit method's multi-line body folds like a TclOO method's.
    let src = "snit::type Dog {\n\
               \x20   method bark {volume} {\n\
               \x20       set n 0\n\
               \x20       return $n\n\
               \x20   }\n\
               }\n";
    let r = regions(src);
    // The outer type body folds, and the inner method body folds too.
    assert!(
        r.iter().any(|f| f.start_line == 0),
        "the snit type body must fold: {r:?}",
    );
    assert!(
        r.iter().any(|f| f.start_line == 1 && f.end_line >= 3),
        "the snit method body must fold: {r:?}",
    );
}

#[test]
fn itcl_method_bodies_fold() {
    // itcl bodies fold via the same grammar walk, including a body nested under
    // a `public` access-modifier wrapper.
    let src = "itcl::class Dog {\n\
               \x20   public method bark {volume} {\n\
               \x20       set n 0\n\
               \x20       return $n\n\
               \x20   }\n\
               }\n";
    let r = regions(src);
    assert!(
        r.iter().any(|f| f.start_line == 0),
        "the itcl class body must fold: {r:?}",
    );
    assert!(
        r.iter().any(|f| f.start_line == 1 && f.end_line >= 3),
        "the itcl `public method` body must fold: {r:?}",
    );
}

#[test]
fn alias_to_user_proc_named_method_does_not_fold_data_as_a_member_body() {
    // Tclsh executes `method`, a user proc, through this alias. Its third
    // argument is data, even though the target spelling matches an OO member.
    let src = "proc method {name parameters body} {\n    return \"$name:$parameters:$body\"\n}\ninterp alias {} define_method {} method\noo::class create C {\n    define_method m {} {\n        # inert data\n        puts must-not-run\n    }\n}\n";
    let r = regions(src);
    assert!(
        !r.iter()
            .any(|fold| fold.start_line == 5 && fold.end_line >= 7),
        "the user proc's final braced data must not acquire a member-body fold: {r:?}"
    );
}

// Issue #1243 — a leading UTF-8 byte-order mark is a *file* prologue under
// Tcl 9 (`source` strips it), but ordinary data at the head of a nested body
// slice. Every provider that re-segments the raw document must draw that split
// at its own top level.
//
// tclsh-proof: tclsh8.6.14 rejects a BOM'd first command
// (`invalid command name "<BOM>proc"`), which is why the skip is Tcl 9 only.

/// The first command of a BOM'd Tcl 9 file still gets its body fold: the mark
/// must not lex into the command name (which would resolve to no registry
/// command, so no `ArgRole::Body` and no fold).
#[test]
fn tcl9_leading_bom_does_not_suppress_the_first_body_fold() {
    let registry = registry_for_dialect("tcl9.0");
    let src = "\u{FEFF}proc greet {} {\n    return hi\n}\n";
    let with_bom: Vec<FoldingRange> = folding_ranges(
        src,
        tcl_dialect::DialectProfile::by_name("tcl9.0"),
        registry,
    )
    .into_iter()
    .filter(|r| r.kind == FoldKind::Region)
    .collect();
    let plain: Vec<FoldingRange> = folding_ranges(
        &src["\u{FEFF}".len()..],
        tcl_dialect::DialectProfile::by_name("tcl9.0"),
        registry,
    )
    .into_iter()
    .filter(|r| r.kind == FoldKind::Region)
    .collect();
    assert_eq!(
        with_bom, plain,
        "a BOM'd Tcl 9 file folds exactly like the same file without the mark",
    );
    assert!(!with_bom.is_empty(), "the proc body is a region fold");
}

/// TN control — a mark at the head of a **nested** body is data, not a
/// prologue, so the inner command keeps the mark in its name and draws no
/// inner fold. The outer proc's own fold is unaffected.
#[test]
fn a_bom_inside_a_nested_body_is_data() {
    let registry = registry_for_dialect("tcl9.0");
    let src = "proc outer {} {\n    \u{FEFF}proc inner {} {\n        return 1\n    }\n}\n";
    let regions: Vec<FoldingRange> = folding_ranges(
        src,
        tcl_dialect::DialectProfile::by_name("tcl9.0"),
        registry,
    )
    .into_iter()
    .filter(|r| r.kind == FoldKind::Region)
    .collect();
    assert_eq!(
        regions.len(),
        1,
        "only the outer proc folds — the marked inner head is not `proc`; got {regions:?}",
    );
}
