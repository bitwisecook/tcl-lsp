//! Port of the Python pytest suite `tests/test_text_utils.py`
//! ("shared.text — edit distance and suggestion utilities").
//!
//! The Rust feature under test lives in `tcl-compiler/src/text.rs`
//! and is re-exported via `pub mod text;` in `lib.rs`, so it is
//! reachable from an integration test as
//! `tcl_compiler::text::{edit_distance, suggest_similar}`.
//!
//! Edit distance is a pure algorithm (no Tcl semantics), so the
//! values are asserted directly — no `tclsh` round-trip needed.
//!
//! Signature note (Python vs Rust):
//!
//! - Python: `suggest_similar(attempted, candidates, *,
//!   max_suggestions=3, max_distance=3)` — keyword-only with
//!   defaults.
//! - Rust:   `suggest_similar(attempted, candidates,
//!   max_suggestions, max_distance)` — all four positional, no
//!   defaults, and `max_suggestions` precedes `max_distance`.
//!
//! Where the pytest relies on Python's defaults, this port passes
//! the explicit values `max_suggestions = 3, max_distance = 3`.

use tcl_compiler::text::{edit_distance, suggest_similar};

// ---------------------------------------------------------------------------
// TestEditDistance
// ---------------------------------------------------------------------------
//
// Both the Python `edit_distance` and the Rust one are plain
// Levenshtein (substitution / insertion / deletion, no
// transposition). The pytest `test_transposition` even documents
// this: a swap of adjacent chars costs 2, not 1. So there is no
// Damerau divergence between the two implementations — the asserted
// values match exactly. (No `// GAP:` needed.)

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
// TestSuggestSimilar
// ---------------------------------------------------------------------------
//
// The Rust `suggest_similar` borrows `&'a str` from the candidate
// iterator and returns `Vec<&'a str>`, so the candidate slices are
// bound to `&str` literals here. The default arguments the pytest
// leans on (`max_suggestions=3`, `max_distance=3`) are supplied
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
    // candidate -> empty result. (Python: max_distance=2,
    // max_suggestions defaults to 3.)
    let result = suggest_similar("xyzzy", ["puts", "set", "string"], 3, 2);
    assert_eq!(result, Vec::<&str>::new());
}

#[test]
fn test_max_suggestions() {
    // Cap the number of returned suggestions at 2. (Python:
    // max_suggestions=2, max_distance=3.)
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
