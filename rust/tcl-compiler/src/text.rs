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
    let mut scored: Vec<(usize, &'a str)> = candidates
        .into_iter()
        .map(|name| (edit_distance(attempted, name), name))
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    scored
        .into_iter()
        .take(max_suggestions)
        .filter(|(d, _)| *d <= max_distance)
        .map(|(_, n)| n)
        .collect()
}

/// Maximum size for the Cartesian product when folding an
/// interpolated word.  The analyser also uses
/// [`crate::analyses::MAX_CONSTSET_SIZE`].
const MAX_FOLD_PRODUCT: usize = 32;

/// Resolve a Tcl word with `$var` interpolations to the set of
/// possible literal strings, given a per-variable resolved-value
/// map.
///
/// Returns `None` when:
///
/// - the word contains a command substitution ``[…]`` (the
///   side-effect surface defeats static folding);
/// - any variable's resolved set is missing from `var_values`
///   (treated as overdefined / unknown);
/// - the Cartesian product would exceed [`MAX_FOLD_PRODUCT`]
///   (a widening cutoff).
///
/// `var_values` is keyed by bare variable name (no leading
/// ``$``); each value is the flat set of constant strings
/// the variable may take.  Callers materialise this from the
/// SCCP `LatticeValue::Const` / `LatticeValue::ConstSet` maps
/// — same shape as the W307 emitter's `all_constsets` aggregator.
///
/// **Recognised variable forms.**  Rather than a full `TclLexer`
/// walk over every variable form (`$x`, `${x}`, `$x(idx)`), this
/// uses a regex that matches the two leading-form variants (`$x`,
/// `${x}`) and rejects array indexing — array-indexed reads
/// aren't expected in command-head positions, and rejecting them
/// errs on the safe side (returns `None`, leaving the W123 in place).
#[must_use]
pub(crate) fn fold_interpolation_set(
    word: &str,
    var_values: &std::collections::HashMap<String, HashSet<String>>,
) -> Option<HashSet<String>> {
    if word.is_empty() {
        return None;
    }
    if word.contains('[') {
        return None;
    }

    // Walk the word, alternating literal segments and
    // ``$var`` / ``${var}`` substitutions.  Each segment becomes
    // a `Vec<String>` of possible expansions; the final result is
    // the Cartesian product across all segments.
    let bytes = word.as_bytes();
    let mut segments: Vec<Vec<String>> = Vec::new();
    let mut i = 0;
    let mut literal_start = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            if literal_start < i {
                segments.push(vec![word[literal_start..i].to_string()]);
            }
            i += 1;
            // ``${var}`` form.
            if i < bytes.len() && bytes[i] == b'{' {
                i += 1;
                let close = bytes[i..].iter().position(|&b| b == b'}')?;
                let name = &word[i..i + close];
                let resolved = var_values.get(name)?.clone();
                if resolved.is_empty() {
                    return None;
                }
                segments.push(resolved.into_iter().collect());
                i += close + 1;
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
            let name = &word[name_start..i];
            // Reject array-indexed reads — this keeps the matcher simple.
            if i < bytes.len() && bytes[i] == b'(' {
                return None;
            }
            let resolved = var_values.get(name)?.clone();
            if resolved.is_empty() {
                return None;
            }
            segments.push(resolved.into_iter().collect());
            literal_start = i;
            continue;
        }
        i += 1;
    }
    if literal_start < bytes.len() {
        segments.push(vec![word[literal_start..].to_string()]);
    }

    if segments.is_empty() {
        return None;
    }

    // Cartesian product, bounded by `MAX_FOLD_PRODUCT`.
    let mut current: Vec<String> = vec![String::new()];
    for seg in segments {
        let mut next: Vec<String> = Vec::with_capacity(current.len() * seg.len().max(1));
        for prefix in &current {
            for piece in &seg {
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
        let set = fold_interpolation_set("foo$x", &vars).expect("resolved");
        assert!(set.contains("fooa"));
        assert!(set.contains("foob"));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn fold_interpolation_set_resolves_braced_form() {
        let vars = vmap(&[("x", &["a"])]);
        let set = fold_interpolation_set("foo${x}bar", &vars).expect("resolved");
        assert!(set.contains("fooabar"));
    }

    #[test]
    fn fold_interpolation_set_returns_none_for_unknown_var() {
        let vars: HashMap<String, HashSet<String>> = HashMap::new();
        assert!(fold_interpolation_set("foo$x", &vars).is_none());
    }

    #[test]
    fn fold_interpolation_set_returns_none_for_command_substitution() {
        let vars = vmap(&[("x", &["a"])]);
        assert!(fold_interpolation_set("foo[bar]", &vars).is_none());
    }

    #[test]
    fn fold_interpolation_set_returns_none_for_array_indexed_var() {
        // ``$arr(idx)`` is rejected.
        let vars = vmap(&[("arr", &["a"])]);
        assert!(fold_interpolation_set("foo$arr(idx)", &vars).is_none());
    }

    #[test]
    fn fold_interpolation_set_widens_oversized_product() {
        // 6 vars × 6 alternatives each = 7776 — way over
        // ``MAX_FOLD_PRODUCT``.
        let vars = vmap(&[("x", &["a", "b", "c", "d", "e", "f"])]);
        // 4 × 6 = 24 still fits, but 7 × 6 = 42 overflows.
        let big = "$x$x$x$x$x$x$x";
        assert!(fold_interpolation_set(big, &vars).is_none());
    }

    #[test]
    fn fold_interpolation_set_preserves_pure_literal() {
        // No ``$`` — just a single literal segment.  The helper
        // still accepts this and resolves it to a one-element
        // set containing the literal unchanged.
        let vars: HashMap<String, HashSet<String>> = HashMap::new();
        let set = fold_interpolation_set("foo", &vars).expect("resolved");
        assert!(set.contains("foo"));
        assert_eq!(set.len(), 1);
    }
}
