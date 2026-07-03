//! Edit-distance and suggestion utilities.
//!
//! The feature under test lives in `tcl-compiler/src/text.rs`
//! and is re-exported via `pub mod text;` in `lib.rs`, so it is
//! reachable from an integration test as
//! `tcl_compiler::text::{edit_distance, suggest_similar}`.
//!
//! Edit distance is a pure algorithm (no Tcl semantics), so the
//! values are asserted directly — no `tclsh` round-trip needed.
//!
//! Signature note: `suggest_similar(attempted, candidates,
//! max_suggestions, max_distance)` takes all four positionally, with
//! no defaults, and `max_suggestions` precedes `max_distance`. Tests
//! that want the default behaviour pass the explicit values
//! `max_suggestions = 3, max_distance = 3`.

use tcl_compiler::text::{edit_distance, suggest_similar};

// ---------------------------------------------------------------------------
// Edit distance
// ---------------------------------------------------------------------------
//
// `edit_distance` is plain
// Levenshtein (substitution / insertion / deletion, no
// transposition). A swap of adjacent chars therefore costs 2, not 1
// (no Damerau transposition), and the asserted values reflect that.

#[test]
fn test_identical() {
    assert_eq!(edit_distance("puts", "puts"), 0);
}

#[test]
fn test_single_substitution() {
    assert_eq!(edit_distance("puts", "putz"), 1);
}

#[test]
fn test_transposition() {
    // Levenshtein counts a transposition as 2 operations (one
    // delete + one insert, or two substitutions). Damerau would
    // score this as 1 — but both impls here are plain Levenshtein.
    assert_eq!(edit_distance("puts", "pust"), 2);
}

#[test]
fn test_single_insertion() {
    assert_eq!(edit_distance("set", "sett"), 1);
}

#[test]
fn test_single_deletion() {
    assert_eq!(edit_distance("string", "strig"), 1);
}

#[test]
fn test_empty() {
    assert_eq!(edit_distance("", ""), 0);
    assert_eq!(edit_distance("abc", ""), 3);
    assert_eq!(edit_distance("", "abc"), 3);
}

#[test]
fn test_completely_different() {
    assert_eq!(edit_distance("abc", "xyz"), 3);
}

// ---------------------------------------------------------------------------
// suggest_similar
// ---------------------------------------------------------------------------
//
// `suggest_similar` borrows `&'a str` from the candidate
// iterator and returns `Vec<&'a str>`, so the candidate slices are
// bound to `&str` literals here. The default arguments
// (`max_suggestions=3`, `max_distance=3`) are supplied
// explicitly.

#[test]
fn test_exact_match_first() {
    // Exact match has distance 0, so it sorts first.
    let result = suggest_similar("puts", ["puts", "set", "string"], 3, 3);
    assert_eq!(result[0], "puts");
}

#[test]
fn test_close_match() {
    // "pust" -> "puts" is a transposition (distance 2 under
    // Levenshtein), which is within the default max_distance of 3,
    // so "puts" must appear in the suggestions.
    let result = suggest_similar("pust", ["puts", "set", "string"], 3, 3);
    assert!(result.contains(&"puts"));
}

#[test]
fn test_no_match_beyond_max_distance() {
    // With max_distance = 2, "xyzzy" is too far from every
    // candidate -> empty result.
    let result = suggest_similar("xyzzy", ["puts", "set", "string"], 3, 2);
    assert_eq!(result, Vec::<&str>::new());
}

#[test]
fn test_max_suggestions() {
    // Cap the number of returned suggestions at 2.
    let candidates = ["aa", "ab", "ac", "ad"];
    let result = suggest_similar("aa", candidates, 2, 3);
    assert!(result.len() <= 2);
}

#[test]
fn test_empty_candidates() {
    // No candidates -> no suggestions. Empty iterator typed as
    // `&str` so the return type's lifetime resolves.
    let result = suggest_similar("foo", Vec::<&str>::new(), 3, 3);
    assert_eq!(result, Vec::<&str>::new());
}
