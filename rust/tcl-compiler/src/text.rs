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

//! Shared text-similarity utilities.
//!
//! Used by the analyser's W123 (unknown command) emitter to
//! produce "did you mean…?" suggestions.  When the compiler
//! grows a W001 (unknown subcommand) emitter the same
//! [`suggest_similar`] helper applies.
//!
//! The LSP layer consumes the same machinery: `tcl-lsp-core`'s
//! completion fuzzy fallback ranks typo'd fragments with
//! [`edit_distance`] / [`scaled_max_distance`] /
//! [`rank_suggestions`], and its `Add 'package require …'` code
//! action ranks package names with
//! [`rank_containment_suggestions`].  Every suggestion surface
//! shares one ordering rule — ascending `(score, name)` — via
//! [`rank_suggestions`].

/// Edit distance between two strings — optimal string alignment
/// (restricted Damerau–Levenshtein): insertions, deletions,
/// substitutions, and **adjacent transpositions** each count as one
/// edit.  A transposed pair (`ste` → `set`) is the single most common
/// real-world typo, so counting it as one edit instead of two keeps
/// such names inside the "did you mean…?" distance budget.
///
/// O(len(a) × len(b)) time, O(min(len(a), len(b))) space.
#[must_use]
pub fn edit_distance(a: &str, b: &str) -> usize {
    // strsim sizes its working rows from the second string.
    // grammar-bypass-ok: encodings — Sizes strsim work rows by Rust scalar count.
    if a.chars().count() < b.chars().count() {
        strsim::osa_distance(b, a)
    } else {
        strsim::osa_distance(a, b)
    }
}

/// The "did you mean…?" distance budget for a name of `chars` characters:
/// one edit per three whole characters, at least 1, at most 3.
///
/// A fixed budget over-suggests on short names (with a budget of 2, a
/// 3-character typo like `ua2` "matches" the entirely unrelated `cat`;
/// with 3, the 7-character `require` "matches" `re_quote`) and
/// under-suggests on long ones.  Scaling by length keeps suggestions
/// plausible at both ends: 1–5 chars → 1 edit, 6–8 → 2, 9+ → 3.
/// Transpositions count as one edit ([`edit_distance`] is OSA), so the
/// common swapped-pair typo stays within budget even on short names.
#[must_use]
pub fn scaled_max_distance(name: &str) -> usize {
    (name.chars().count() / 3).clamp(1, 3)
}

/// [`scaled_max_distance`] additionally capped *below* the name's own
/// character count, so an edit budget can never rewrite the whole name.
/// Used by the undefined-variable "did you mean…?" family (W210 / W212 /
/// W215), where one-character names (`$i`, `$u`) are common and every
/// other one-character variable sits at distance 1 — a full-replacement
/// "suggestion" there is noise, not a typo correction.
#[must_use]
pub fn scaled_max_distance_strict(name: &str) -> usize {
    scaled_max_distance(name).min(name.chars().count().saturating_sub(1))
}

/// Order scored candidates ascending by `(score, name)` and cap the
/// list at `max_suggestions` — the one ranking rule every suggestion
/// surface shares: [`suggest_similar`] (edit-distance scores),
/// [`rank_containment_suggestions`] (containment scores), and the
/// completion fuzzy fallback in `tcl-lsp-core`.  Lower scores rank
/// first; ties break on the candidate name via `name_of`.
#[must_use]
pub fn rank_suggestions<T>(
    mut scored: Vec<(usize, T)>,
    max_suggestions: usize,
    name_of: impl Fn(&T) -> &str,
) -> Vec<T> {
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| name_of(&a.1).cmp(name_of(&b.1))));
    scored.truncate(max_suggestions);
    scored.into_iter().map(|(_, candidate)| candidate).collect()
}

/// Suggest up to `max_suggestions` candidates from `candidates`
/// ranked by edit distance to `attempted`, dropping any whose
/// distance exceeds `max_distance`.
///
/// Returns the suggestions in ascending-distance order; ties
/// are broken by lexicographic order of the candidate name
/// (i.e. ordering on the `(dist, name)` tuple).
#[must_use]
pub fn suggest_similar<'a, I>(
    attempted: &str,
    candidates: I,
    max_suggestions: usize,
    max_distance: usize,
) -> Vec<&'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    let scored: Vec<(usize, &'a str)> = candidates
        .into_iter()
        .filter_map(|name| {
            let distance = edit_distance(attempted, name);
            (distance <= max_distance).then_some((distance, name))
        })
        .collect();
    rank_suggestions(scored, max_suggestions, |name| name)
}

/// Rank `candidates` against `needle` by containment quality — exact
/// match, then prefix, then substring — dropping candidates that do
/// not contain `needle` at all, and capping the list at
/// `max_suggestions`.  Ties break on the candidate name.  Matching
/// is case-insensitive; the returned names keep their original
/// spelling.  An empty `needle` matches nothing (rather than
/// prefix-matching everything).
///
/// This is the ranking behind the LSP `Add 'package require …'`
/// suggestions (`tcl-lsp-core`'s code actions): a namespace prefix
/// (`http`) is expected to *appear in* the package name (`http`,
/// `httpd`), so containment — not edit distance — is the metric, and
/// an exact package-name hit always outranks a longer name that
/// merely starts with it.
#[must_use]
pub fn rank_containment_suggestions<'a, I>(
    needle: &str,
    candidates: I,
    max_suggestions: usize,
) -> Vec<&'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    let needle = needle.to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let scored: Vec<(usize, &'a str)> = candidates
        .into_iter()
        .filter_map(|candidate| {
            let lower = candidate.to_lowercase();
            let score = if lower == needle {
                0
            } else if lower.starts_with(&needle) {
                1
            } else if lower.contains(&needle) {
                2
            } else {
                return None;
            };
            Some((score, candidate))
        })
        .collect();
    rank_suggestions(scored, max_suggestions, |name| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_zero_for_equal() {
        assert_eq!(edit_distance("foo", "foo"), 0);
    }

    #[test]
    fn edit_distance_one_for_single_substitution() {
        assert_eq!(edit_distance("foo", "fou"), 1);
    }

    #[test]
    fn edit_distance_one_for_single_insertion() {
        assert_eq!(edit_distance("foo", "fooo"), 1);
    }

    #[test]
    fn edit_distance_handles_empty() {
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("", ""), 0);
    }

    #[test]
    fn edit_distance_unicode_counts_chars_not_bytes() {
        // ``é`` is 2 bytes in UTF-8 but should count as one
        // edit when substituted for ``e``.
        assert_eq!(edit_distance("café", "cafe"), 1);
    }

    #[test]
    fn scaled_max_distance_strict_caps_below_name_length() {
        // A 1-char name gets a zero budget — every other 1-char name is
        // a full replacement, never a typo correction.
        assert_eq!(scaled_max_distance_strict("u"), 0);
        // 2+ chars keep the ordinary scaled budget.
        assert_eq!(scaled_max_distance_strict("ab"), 1);
        assert_eq!(scaled_max_distance_strict("countr"), 2);
    }

    #[test]
    fn suggest_similar_returns_closest_within_max_distance() {
        let candidates = ["set", "puts", "lappend", "foreach"];
        let suggestions = suggest_similar("sett", candidates, 1, 2);
        assert_eq!(suggestions, vec!["set"]);
    }

    #[test]
    fn suggest_similar_drops_too_far_candidates() {
        let candidates = ["set", "puts"];
        // ``unknown`` is too far from both — no suggestions.
        let suggestions = suggest_similar("unknown", candidates, 3, 2);
        assert_eq!(suggestions, [] as [&str; 0]);
    }

    #[test]
    fn suggest_similar_limits_to_max_suggestions() {
        let candidates = ["set", "sex", "see"];
        let suggestions = suggest_similar("sea", candidates, 2, 3);
        assert_eq!(suggestions.len(), 2);
    }

    #[test]
    fn suggest_similar_breaks_ties_lexicographically() {
        // ``aaa`` and ``aab`` both have distance 1 from
        // ``aax``.  Tie broken by lexicographic order.
        let candidates = ["aab", "aaa"];
        let suggestions = suggest_similar("aax", candidates, 2, 2);
        assert_eq!(suggestions, vec!["aaa", "aab"]);
    }

    #[test]
    fn rank_suggestions_orders_by_score_then_name_and_caps() {
        let scored = vec![(2, "zeta"), (1, "beta"), (1, "alpha"), (0, "omega")];
        assert_eq!(
            rank_suggestions(scored.clone(), 4, |n| n),
            vec!["omega", "alpha", "beta", "zeta"]
        );
        // The cap trims from the tail of the ranked order.
        assert_eq!(rank_suggestions(scored, 2, |n| n), vec!["omega", "alpha"]);
    }

    #[test]
    fn rank_containment_orders_exact_before_prefix_before_substring() {
        let candidates = ["shttp", "httpd", "http", "json"];
        let ranked = rank_containment_suggestions("http", candidates, 5);
        assert_eq!(ranked, vec!["http", "httpd", "shttp"]);
    }

    #[test]
    fn rank_containment_is_case_insensitive_and_keeps_spelling() {
        let candidates = ["Thread", "tdom"];
        let ranked = rank_containment_suggestions("thread", candidates, 5);
        assert_eq!(ranked, vec!["Thread"]);
    }

    #[test]
    fn rank_containment_caps_and_breaks_ties_by_name() {
        let candidates = ["abz", "aby", "abx"];
        let ranked = rank_containment_suggestions("ab", candidates, 2);
        // All three are prefix matches (score 1); ties break by name
        // and the cap keeps the first two.
        assert_eq!(ranked, vec!["abx", "aby"]);
    }

    #[test]
    fn rank_containment_rejects_empty_needle() {
        let candidates = ["http", "json"];
        assert_eq!(
            rank_containment_suggestions("", candidates, 5),
            [] as [&str; 0]
        );
    }
}
