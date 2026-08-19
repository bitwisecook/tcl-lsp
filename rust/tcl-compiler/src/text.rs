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
//!
//! Also hosts [`fold_interpolation_set`] — a CONSTSET-aware fold
//! used by the W123 emitter to suppress diagnostics on
//! command names like ``foo$suffix`` whose interpolated parts
//! statically resolve to a finite set of known commands.

use std::collections::HashSet;

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
    // Operate on chars so multi-byte UTF-8 sequences count as
    // one edit.
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    if a_chars.len() < b_chars.len() {
        return edit_distance_inner(&b_chars, &a_chars);
    }
    edit_distance_inner(&a_chars, &b_chars)
}

fn edit_distance_inner(longer: &[char], shorter: &[char]) -> usize {
    if shorter.is_empty() {
        return longer.len();
    }
    // Rolling three-row DP: `prev2` (i-2), `prev` (i-1), `curr` (i).
    // The transposition case reads `prev2[j-1]`.
    let mut prev2: Vec<usize> = Vec::new();
    let mut prev: Vec<usize> = (0..=shorter.len()).collect();
    for (i, &ca) in longer.iter().enumerate() {
        let mut curr: Vec<usize> = Vec::with_capacity(shorter.len() + 1);
        curr.push(i + 1);
        for (j, &cb) in shorter.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let mut v = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
            if i > 0 && j > 0 && ca == shorter[j - 1] && longer[i - 1] == cb {
                v = v.min(prev2[j - 1] + 1);
            }
            curr.push(v);
        }
        prev2 = std::mem::replace(&mut prev, curr);
    }
    prev[shorter.len()]
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

/// Maximum size for the Cartesian product when folding an
/// interpolated word.  The analyser also uses
/// [`crate::analyses::MAX_CONSTSET_SIZE`].
const MAX_FOLD_PRODUCT: usize = 32;

/// One piece of a word split by [`split_interpolation_segments`]: either
/// literal text or a `$name`/`${name}` variable reference (bare name, no
/// leading `$`).
enum Segment<'a> {
    Literal(&'a str),
    Var(&'a str),
}

/// Split a Tcl word into alternating literal and `$var`/`${var}`
/// segments, in order, without resolving the variables — the shared
/// front end for [`fold_interpolation_set`] (multi-value SCCP `ConstSet`
/// resolution) and [`fold_interpolation_single`] (scalar constant-string
/// resolution).
///
/// Returns `None` when:
///
/// - the word is empty or contains a command substitution ``[…]`` (the
///   side-effect surface defeats static folding);
/// - a ``${…}`` form is left unterminated;
/// - a ``$`` is not followed by an identifier character;
/// - a variable reference is array-indexed (``$name(…)``).
///
/// **Recognised variable forms.**  Rather than a full `TclLexer` walk
/// over every variable form (`$x`, `${x}`, `$x(idx)`), this scans for the
/// two leading-form variants (`$x`, `${x}`) and rejects array indexing —
/// array-indexed reads aren't expected in command-head positions, and
/// rejecting them errs on the safe side (returns `None`, leaving the
/// caller's fallback — e.g. a W123 diagnostic — in place).
fn split_interpolation_segments(
    word: &str,
    braced_var: tcl_dialect::BracedVarStyle,
) -> Option<Vec<Segment<'_>>> {
    if word.is_empty() {
        return None;
    }
    if word.contains('[') {
        return None;
    }

    let bytes = word.as_bytes();
    let mut segments: Vec<Segment<'_>> = Vec::new();
    let mut i = 0;
    let mut literal_start = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            if literal_start < i {
                segments.push(Segment::Literal(&word[literal_start..i]));
            }
            i += 1;
            // ``${var}`` form — the name starts just past the `${`, and its
            // closer comes from the shared owner under this document's release
            // rule. Reading it with a fixed first-`}` scan resolved a
            // *different* variable than the one the lexer spanned, and the
            // folded string is written back into a diagnostic (issue #1604).
            if i < bytes.len() && bytes[i] == b'{' {
                i += 1;
                let tcl_lexer::BracedVarEnd::Closed(close) =
                    tcl_lexer::braced_var_name_end(bytes, i, braced_var)
                else {
                    return None;
                };
                segments.push(Segment::Var(&word[i..close]));
                i = close + 1;
                literal_start = i;
                continue;
            }
            // Bare ``$name`` — accumulate identifier characters.
            let name_start = i;
            while i < bytes.len() {
                let c = bytes[i];
                if c.is_ascii_alphanumeric() || c == b'_' || c == b':' {
                    i += 1;
                } else {
                    break;
                }
            }
            if i == name_start {
                // ``$`` not followed by an identifier — reject.
                return None;
            }
            // Reject array-indexed reads — this keeps the matcher simple.
            if i < bytes.len() && bytes[i] == b'(' {
                return None;
            }
            segments.push(Segment::Var(&word[name_start..i]));
            literal_start = i;
            continue;
        }
        i += 1;
    }
    if literal_start < bytes.len() {
        segments.push(Segment::Literal(&word[literal_start..]));
    }

    if segments.is_empty() {
        return None;
    }
    Some(segments)
}

/// Resolve a Tcl word with `$var` interpolations to the set of
/// possible literal strings, given a per-variable resolved-value
/// map.
///
/// Returns `None` when [`split_interpolation_segments`] does (see its
/// doc), when any variable's resolved set is missing from `var_values`
/// (treated as overdefined / unknown), or when the Cartesian product
/// would exceed [`MAX_FOLD_PRODUCT`] (a widening cutoff).
///
/// `var_values` is keyed by bare variable name (no leading
/// ``$``); each value is the flat set of constant strings
/// the variable may take.  Callers materialise this from the
/// SCCP `LatticeValue::Const` / `LatticeValue::ConstSet` maps
/// — same shape as the W307 emitter's `all_constsets` aggregator.
#[must_use]
pub(crate) fn fold_interpolation_set(
    word: &str,
    var_values: &std::collections::HashMap<String, HashSet<String>>,
    braced_var: tcl_dialect::BracedVarStyle,
) -> Option<HashSet<String>> {
    let segments = split_interpolation_segments(word, braced_var)?;

    // Cartesian product, bounded by `MAX_FOLD_PRODUCT`.
    let mut current: Vec<String> = vec![String::new()];
    for seg in segments {
        let piece: Vec<String> = match seg {
            Segment::Literal(text) => vec![text.to_string()],
            Segment::Var(name) => {
                let resolved = var_values.get(name)?;
                if resolved.is_empty() {
                    return None;
                }
                resolved.iter().cloned().collect()
            }
        };
        let mut next: Vec<String> = Vec::with_capacity(current.len() * piece.len().max(1));
        for prefix in &current {
            for piece in &piece {
                next.push(format!("{prefix}{piece}"));
                if next.len() > MAX_FOLD_PRODUCT {
                    return None;
                }
            }
        }
        current = next;
    }
    if current.is_empty() {
        return None;
    }
    Some(current.into_iter().collect())
}

/// Resolve a Tcl word with `$var` interpolations to a single literal
/// string, given a variable resolver.  The single-value analogue of
/// [`fold_interpolation_set`], for a caller that already has a scalar
/// constant-string lattice (e.g. `Analyser::lookup_const_string`) rather
/// than the SCCP `ConstSet` map — short-circuits to `None` on the first
/// unresolvable variable, same rejection rules as
/// [`split_interpolation_segments`] (issue #923 idx 3).
#[must_use]
pub(crate) fn fold_interpolation_single(
    word: &str,
    braced_var: tcl_dialect::BracedVarStyle,
    mut resolve_var: impl FnMut(&str) -> Option<String>,
) -> Option<String> {
    let segments = split_interpolation_segments(word, braced_var)?;
    let mut result = String::new();
    for seg in segments {
        match seg {
            Segment::Literal(text) => result.push_str(text),
            Segment::Var(name) => result.push_str(&resolve_var(name)?),
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
        assert!(suggestions.is_empty());
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
        assert!(rank_containment_suggestions("", candidates, 5).is_empty());
    }

    fn vmap(entries: &[(&str, &[&str])]) -> HashMap<String, HashSet<String>> {
        entries
            .iter()
            .map(|(k, vs)| {
                (
                    (*k).to_string(),
                    vs.iter().map(|s| (*s).to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn fold_interpolation_set_resolves_simple_dollar_var() {
        // ``foo$x`` with ``x ∈ {a, b}`` → ``{fooa, foob}``.
        let vars = vmap(&[("x", &["a", "b"])]);
        let set = fold_interpolation_set("foo$x", &vars, tcl_dialect::BracedVarStyle::default())
            .expect("resolved");
        assert!(set.contains("fooa"));
        assert!(set.contains("foob"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn fold_interpolation_set_resolves_braced_form() {
        let vars = vmap(&[("x", &["a"])]);
        let set =
            fold_interpolation_set("foo${x}bar", &vars, tcl_dialect::BracedVarStyle::default())
                .expect("resolved");
        assert!(set.contains("fooabar"));
    }

    #[test]
    fn fold_interpolation_set_returns_none_for_unknown_var() {
        let vars: HashMap<String, HashSet<String>> = HashMap::new();
        assert!(
            fold_interpolation_set("foo$x", &vars, tcl_dialect::BracedVarStyle::default())
                .is_none()
        );
    }

    #[test]
    fn fold_interpolation_set_returns_none_for_command_substitution() {
        let vars = vmap(&[("x", &["a"])]);
        assert!(
            fold_interpolation_set("foo[bar]", &vars, tcl_dialect::BracedVarStyle::default())
                .is_none()
        );
    }

    #[test]
    fn fold_interpolation_set_returns_none_for_array_indexed_var() {
        // ``$arr(idx)`` is rejected.
        let vars = vmap(&[("arr", &["a"])]);
        assert!(
            fold_interpolation_set(
                "foo$arr(idx)",
                &vars,
                tcl_dialect::BracedVarStyle::default()
            )
            .is_none()
        );
    }

    #[test]
    fn fold_interpolation_set_widens_oversized_product() {
        // 6 vars × 6 alternatives each = 7776 — way over
        // ``MAX_FOLD_PRODUCT``.
        let vars = vmap(&[("x", &["a", "b", "c", "d", "e", "f"])]);
        // 4 × 6 = 24 still fits, but 7 × 6 = 42 overflows.
        let big = "$x$x$x$x$x$x$x";
        assert!(
            fold_interpolation_set(big, &vars, tcl_dialect::BracedVarStyle::default()).is_none()
        );
    }

    #[test]
    fn fold_interpolation_set_preserves_pure_literal() {
        // No ``$`` — just a single literal segment.  The helper
        // still accepts this and resolves it to a one-element
        // set containing the literal unchanged.
        let vars: HashMap<String, HashSet<String>> = HashMap::new();
        let set = fold_interpolation_set("foo", &vars, tcl_dialect::BracedVarStyle::default())
            .expect("resolved");
        assert!(set.contains("foo"));
        assert_eq!(set.len(), 1);
    }

    fn resolver(pairs: &[(&str, &str)]) -> impl FnMut(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name: &str| map.get(name).cloned()
    }

    #[test]
    fn fold_interpolation_single_resolves_simple_dollar_var() {
        assert_eq!(
            fold_interpolation_single(
                "foo_$x",
                tcl_dialect::BracedVarStyle::default(),
                resolver(&[("x", "impl")])
            ),
            Some("foo_impl".to_string())
        );
    }

    #[test]
    fn fold_interpolation_single_resolves_braced_form() {
        assert_eq!(
            fold_interpolation_single(
                "foo${x}bar",
                tcl_dialect::BracedVarStyle::default(),
                resolver(&[("x", "a")])
            ),
            Some("fooabar".to_string())
        );
    }

    #[test]
    fn fold_interpolation_single_resolves_two_vars() {
        assert_eq!(
            fold_interpolation_single(
                "::mypkg::${c}_$key",
                tcl_dialect::BracedVarStyle::default(),
                resolver(&[("c", "greet"), ("key", "tcl")])
            ),
            Some("::mypkg::greet_tcl".to_string())
        );
    }

    #[test]
    fn fold_interpolation_single_returns_none_for_unknown_var() {
        assert_eq!(
            fold_interpolation_single(
                "foo_$x",
                tcl_dialect::BracedVarStyle::default(),
                resolver(&[])
            ),
            None
        );
    }

    #[test]
    fn fold_interpolation_single_returns_none_for_command_substitution() {
        assert_eq!(
            fold_interpolation_single(
                "foo[bar]",
                tcl_dialect::BracedVarStyle::default(),
                resolver(&[])
            ),
            None
        );
    }

    #[test]
    fn fold_interpolation_single_returns_none_for_array_indexed_var() {
        assert_eq!(
            fold_interpolation_single(
                "foo$arr(idx)",
                tcl_dialect::BracedVarStyle::default(),
                resolver(&[("arr", "a")])
            ),
            None
        );
    }

    #[test]
    fn fold_interpolation_single_preserves_pure_literal() {
        assert_eq!(
            fold_interpolation_single("foo", tcl_dialect::BracedVarStyle::default(), resolver(&[])),
            Some("foo".to_string())
        );
    }

    #[test]
    fn fold_interpolation_single_returns_none_for_empty_word() {
        assert_eq!(
            fold_interpolation_single("", tcl_dialect::BracedVarStyle::default(), resolver(&[])),
            None
        );
    }
}
