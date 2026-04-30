#![allow(clippy::implicit_hasher)]

//! Shared text-similarity utilities — Rust port of
//! `core/common/text.py`.
//!
//! Used by the analyser's W123 (unknown command) emitter to
//! produce "did you mean…?" suggestions.  When the compiler
//! grows a W001 (unknown subcommand) emitter the same
//! [`suggest_similar`] helper applies.
//!
//! Also hosts [`fold_interpolation_set`] — the CONSTSET-aware
//! analogue of `core/compiler/core_analyses.py::_fold_interpolation_set`,
//! used by the W123 emitter to suppress diagnostics on
//! command names like ``foo$suffix`` whose interpolated parts
//! statically resolve to a finite set of known commands.

use std::collections::HashSet;

/// Levenshtein edit distance between two strings.
///
/// Mirrors `edit_distance` in `core/common/text.py:13-26`.
/// O(len(a) × len(b)) time, O(min(len(a), len(b))) space.
#[must_use]
pub fn edit_distance(a: &str, b: &str) -> usize {
    // Operate on chars so multi-byte UTF-8 sequences count as
    // one edit (matches Python's len-of-string-iteration).
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
    let mut prev: Vec<usize> = (0..=shorter.len()).collect();
    for (i, &ca) in longer.iter().enumerate() {
        let mut curr: Vec<usize> = Vec::with_capacity(shorter.len() + 1);
        curr.push(i + 1);
        for (j, &cb) in shorter.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let v = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
            curr.push(v);
        }
        prev = curr;
    }
    prev[shorter.len()]
}

/// Suggest up to `max_suggestions` candidates from `candidates`
/// ranked by edit distance to `attempted`, dropping any whose
/// distance exceeds `max_distance`.
///
/// Mirrors `suggest_similar` in `core/common/text.py:29-42`.
/// Returns the suggestions in ascending-distance order; ties
/// are broken by lexicographic order of the candidate name
/// (matches Python's `heapq.nsmallest` tuple ordering on
/// `(dist, name)`).
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
/// interpolated word.  Mirrors
/// `core/analyses.MAX_CONSTSET_SIZE`; the Rust analyser also
/// uses [`crate::analyses::MAX_CONSTSET_SIZE`].
const MAX_FOLD_PRODUCT: usize = 32;

/// Resolve a Tcl word with `$var` interpolations to the set of
/// possible literal strings, given a per-variable resolved-value
/// map.
///
/// Mirrors `_fold_interpolation_set` in
/// `core/compiler/core_analyses.py:371-416`.  Returns `None`
/// when:
///
/// - the word contains a command substitution ``[…]`` (the
///   side-effect surface defeats static folding);
/// - any variable's resolved set is missing from `var_values`
///   (treated as overdefined / unknown);
/// - the Cartesian product would exceed [`MAX_FOLD_PRODUCT`]
///   (matches Python's widening behaviour).
///
/// `var_values` is keyed by bare variable name (no leading
/// ``$``); each value is the flat set of constant strings
/// the variable may take.  Callers materialise this from the
/// SCCP `LatticeValue::Const` / `LatticeValue::ConstSet` maps
/// — same shape as the W307 emitter's `all_constsets` aggregator.
///
/// **Simplifications vs. Python.**  The Python helper uses a
/// full `TclLexer` to recognise every variable form (`$x`,
/// `${x}`, `$x(idx)`).  The Rust port uses a regex that
/// matches the two leading-form variants and rejects array
/// indexing — array-indexed reads aren't expected in command-
/// head positions, and rejecting them errs on the safe side
/// (returns `None`, leaving the W123 in place).
#[must_use]
pub fn fold_interpolation_set(
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
            // Reject array-indexed reads; Python folds them
            // independently but the Rust port keeps it simple.
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
        // ``$arr(idx)`` is rejected (Python folds it independently).
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
