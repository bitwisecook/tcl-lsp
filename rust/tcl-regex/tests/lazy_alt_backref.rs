//! Regression tests for the second round of Codex review on PR #688:
//!
//! 1. **Lazy quantifier must not discard longer alternatives** — a leading
//!    non-greedy quantifier sets a *shortest* preference only for a lone branch;
//!    a multi-branch alternation still takes the POSIX-longest extent (the C
//!    engine seeds the `'|'` subre with `LONGER`). So `a+?|aa` on "aa" matches
//!    the whole "aa", not the lazy "a".
//! 2. **Backref path must keep zero-width iterations** — the backtracking
//!    matcher used for backreferences must count an empty iteration toward a
//!    positive repeat minimum (and record its capture), so `(a*)+\1` matches ""
//!    and `(a?){2}\1` matches "a".
//!
//! Every expectation is the half-open form of what `testregexp -indices`
//! reports on real tclsh 9.0.3.

use tcl_regex::Regex;
use tcl_regex::defs::REG_ADVANCED;

/// Compile `re` (ARE), match `subject`, return each group as a half-open
/// `(start, end)` pair (`None` = non-participating); index 0 is the whole match.
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
fn lazy_quantifier_keeps_longer_alternative() {
    // A lazy branch loses to a longer alternative at the same start: the overall
    // extent is POSIX-longest because the alternation prefers longer.
    assert_eq!(g("a+?|aa", "aa"), vec![Some((0, 2))]);
    assert_eq!(g("a*?|aa", "aa"), vec![Some((0, 2))]);
    assert_eq!(g("a+?b|a", "aab"), vec![Some((0, 3))]);
    // Lazy still wins when there is no longer alternative at that start.
    assert_eq!(g("a+?", "aa"), vec![Some((0, 1))]);
    assert_eq!(g("(a+?)", "aa"), vec![Some((0, 1)), Some((0, 1))]);
    // A multi-branch alternation nested in a capture keeps the longest extent.
    assert_eq!(g("(a+?|aa)", "aa"), vec![Some((0, 2)), Some((0, 2))]);
}

#[test]
fn backref_counts_empty_iterations() {
    // The repeated nullable operand needs an empty iteration to satisfy `min`,
    // and the backref then matches the (empty) final capture.
    assert_eq!(g("(a*)+\\1", ""), vec![Some((0, 0)), Some((0, 0))]);
    assert_eq!(g("(a?){2}\\1", "a"), vec![Some((0, 1)), Some((1, 1))]);
}

#[test]
fn backref_final_empty_iteration_capture() {
    // `(a*)+` over "aaa": the first iteration eats "aaa", a final empty iteration
    // captures `[3,3)`, and `\1` matches that empty string at the end.
    assert_eq!(g("(a*)+\\1", "aaa"), vec![Some((0, 3)), Some((3, 3))]);
    // `(a+)+` over "aaaa": the last non-empty iteration captures the trailing
    // "a" at `[2,3)`, then `\1` matches the final "a".
    assert_eq!(g("(a+)+\\1", "aaaa"), vec![Some((0, 4)), Some((2, 3))]);
}
