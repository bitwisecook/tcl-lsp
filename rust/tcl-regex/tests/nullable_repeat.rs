//! Regression tests for nullable repeated subpatterns. Two distinct bugs:
//!
//! 1. **Counting empty iterations toward `min`** — `()+` must match `""`,
//!    `(a?){2}` must match `"a"` (the second iteration is empty).
//! 2. **Capturing the final (possibly empty) iteration** — Tcl records the
//!    *last* iteration's submatch, which for `x{m,n}` with `m>=1` over a
//!    nullable operand is the empty match at the end (e.g. `(a*)+` on "aaa"
//!    captures the inner group as the empty `[3,3)`), while `x*` (m==0) records
//!    the last *non-empty* chunk (`(a*)*` on "aaa" captures "aaa").
//!
//! Every expectation is the half-open form of what `testregexp -indices`
//! reports on real tclsh 9.0.3.

use tcl_regex::Regex;
use tcl_regex::defs::REG_ADVANCED;

/// Compile `re` (ARE) and match `subject`, returning each group as a half-open
/// `(start, end)` pair (`None` = non-participating). Index 0 is the whole match.
fn groups(re: &str, subject: &str) -> Option<Vec<Option<(usize, usize)>>> {
    let cps: Vec<u32> = subject.chars().map(|c| c as u32).collect();
    let rx = Regex::compile_str(re, REG_ADVANCED).expect("compiles");
    rx.exec(&cps, 0, 0)
        .map(|gs| gs.iter().map(|g| g.map(|s| (s.start, s.end))).collect())
}

fn g(re: &str, subject: &str) -> Vec<Option<(usize, usize)>> {
    groups(re, subject).unwrap_or_else(|| panic!("{re:?} should match {subject:?}"))
}

#[test]
fn empty_iterations_count_toward_min() {
    // Bug 1: a positive `min` over a nullable operand must be satisfiable by
    // empty iterations — these patterns match (they used to report no match).
    assert_eq!(g("()+", ""), vec![Some((0, 0)), Some((0, 0))]);
    assert_eq!(g("(a?)+", ""), vec![Some((0, 0)), Some((0, 0))]);
    assert_eq!(g("(){3}", ""), vec![Some((0, 0)), Some((0, 0))]);
    assert_eq!(g("(a?){2}", "a"), vec![Some((0, 1)), Some((1, 1))]);
    assert_eq!(g("(a*){2}", "a"), vec![Some((0, 1)), Some((1, 1))]);
    assert_eq!(g("(a?){3}", "a"), vec![Some((0, 1)), Some((1, 1))]);
}

#[test]
fn final_iteration_capture_plus() {
    // Bug 2, m>=1: the final mandatory iteration is the empty match at the end
    // for a nullable operand.
    assert_eq!(g("(a*)+", "aaa"), vec![Some((0, 3)), Some((3, 3))]);
    assert_eq!(g("(a|b*)+", "aaa"), vec![Some((0, 3)), Some((3, 3))]);
    assert_eq!(g("x(a*)+y", "xy"), vec![Some((0, 2)), Some((1, 1))]);
    assert_eq!(g("x(a*)+y", "xaaay"), vec![Some((0, 5)), Some((4, 4))]);
    assert_eq!(
        g("(a*)(b*)+", "aaabbb"),
        vec![Some((0, 6)), Some((0, 3)), Some((6, 6))]
    );
}

#[test]
fn final_iteration_capture_star_has_no_trailing_empty() {
    // Bug 2, m==0: `x*` records the last non-empty chunk, not a trailing empty.
    assert_eq!(g("(a*)*", "aaa"), vec![Some((0, 3)), Some((0, 3))]);
    // A single-char operand still ends on the last char.
    assert_eq!(g("(a)*", "aaa"), vec![Some((0, 3)), Some((2, 3))]);
    assert_eq!(g("(a)+", "aaa"), vec![Some((0, 3)), Some((2, 3))]);
}
