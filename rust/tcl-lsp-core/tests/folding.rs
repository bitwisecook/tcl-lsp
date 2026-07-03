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
    folding_ranges(source, "tcl8.6", registry)
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
