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

use tcl_compiler::text::{
    edit_distance, rank_containment_suggestions, rank_suggestions, suggest_similar,
};

// Edit distance.
//
// `edit_distance` is optimal string alignment (restricted
// Damerau–Levenshtein): substitution, insertion, deletion, and
// adjacent transposition each cost one edit, and the asserted values
// reflect that.

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
    // Optimal string alignment (restricted Damerau–Levenshtein): an
    // adjacent transposition is ONE edit, not the two plain Levenshtein
    // charges. The swapped-pair typo is the most common real-world
    // misspelling, so `pust` must stay within a one-edit "did you
    // mean…?" budget of `puts`.
    assert_eq!(edit_distance("puts", "pust"), 1);
    // A non-adjacent swap is not a transposition — still two edits.
    assert_eq!(edit_distance("abcd", "dbca"), 2);
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

// suggest_similar.
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
    // "pust" -> "puts" is an adjacent transposition (one edit under
    // optimal string alignment), well within max_distance of 3, so
    // "puts" must appear in the suggestions.
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

// rank_suggestions / rank_containment_suggestions.
//
// The shared `(score, name)` ordering core and its containment-scored
// variant, consumed by the LSP package-require suggestions and the
// completion fuzzy fallback.

#[test]
fn test_rank_suggestions_score_then_name() {
    let scored = vec![(1, "bb"), (0, "cc"), (1, "aa")];
    assert_eq!(rank_suggestions(scored, 3, |n| n), vec!["cc", "aa", "bb"]);
}

#[test]
fn test_rank_suggestions_cap() {
    let scored = vec![(0, "a"), (0, "b"), (0, "c")];
    assert_eq!(rank_suggestions(scored, 2, |n| n).len(), 2);
}

#[test]
fn test_rank_containment_exact_prefix_substring() {
    let ranked = rank_containment_suggestions("http", ["xhttpx", "httpd", "http"], 5);
    assert_eq!(ranked, vec!["http", "httpd", "xhttpx"]);
}

#[test]
fn test_rank_containment_drops_non_containing() {
    let ranked = rank_containment_suggestions("http", ["json", "tls"], 5);
    assert_eq!(ranked, Vec::<&str>::new());
}
