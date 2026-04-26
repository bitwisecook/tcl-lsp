//! Shared text-similarity utilities — Rust port of
//! `core/common/text.py`.
//!
//! Used by the analyser's W123 (unknown command) emitter to
//! produce "did you mean…?" suggestions.  When the compiler
//! grows a W001 (unknown subcommand) emitter the same
//! [`suggest_similar`] helper applies.

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
}
