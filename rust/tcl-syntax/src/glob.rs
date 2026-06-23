//! `string match` glob matching — the shared, byte-exact mirror of
//! `Tcl_StringCaseMatch` (`tmp/tcl9.0.3/generic/tclUtil.c:2138`).
//!
//! One implementation for every consumer: the compiler's `matches_glob`
//! constant-folding, `string match`, `lsearch -glob`, `switch -glob`,
//! `array names pattern`, and the runtime's `namespace export`/`import`/`forget`
//! pattern matching all funnel here so the glob dialect never drifts.
//!
//! Operates on Unicode scalar values (Tcl matches code points and case-folds
//! per code point), consistent with the crate's UTF-8-internal invariant.
//! Special characters: `*` (any run), `?` (one char), `[...]` (set, with `a-z`
//! ranges — reversed `z-a` too — and no negation, matching Tcl), and `\` (escape
//! the next char to a literal). An unterminated `[` matches only if both pattern
//! and string end together, exactly as the C does.

/// Tcl `string match pattern text` (case-sensitive).
#[must_use]
pub fn string_match(pattern: &str, text: &str) -> bool {
    string_case_match(pattern, text, false)
}

/// Tcl `string match ?-nocase? pattern text`.
#[must_use]
pub fn string_case_match(pattern: &str, text: &str, nocase: bool) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = text.chars().collect();
    do_match(&p, 0, &s, 0, nocase)
}

/// Single-code-point case fold (mirrors `Tcl_UniCharToLower`'s 1:1 mapping).
fn fold(c: char, nocase: bool) -> char {
    if nocase {
        c.to_lowercase().next().unwrap_or(c)
    } else {
        c
    }
}

/// Match one non-`*` pattern atom at `p[pi]` against the single string char
/// `sc`, returning the pattern index just past the atom if it matched, else
/// `None`. Handles `?`, a `[...]` set/range, a `\`-escaped literal, and a plain
/// folded char — every glob atom except `*` (whose backtracking the caller
/// drives). `pi` is assumed `< p.len()`; the returned index always advances.
///
/// Factored out so the iterative star loop below can call it from both the
/// "try to extend the current match" path and the "resume after a `*`" path
/// without duplicating the atom semantics (which must stay byte-identical to
/// `Tcl_StringCaseMatch`).
fn match_one(p: &[char], pi: usize, sc: char, nocase: bool) -> Option<usize> {
    let pc = p[pi];

    // `?` — any single char.
    if pc == '?' {
        return Some(pi + 1);
    }

    // `[...]` — a set, possibly with `a-z`/reversed `z-a` ranges.
    if pc == '[' {
        let mut pj = pi + 1;
        let ch1 = fold(sc, nocase);
        let mut matched = false;
        // Scan members until the closing `]`. A member match doesn't stop the
        // scan early — we still walk to `]` so the returned index is correct.
        while pj < p.len() && p[pj] != ']' {
            let start = fold(p[pj], nocase);
            pj += 1;
            if pj < p.len() && p[pj] == '-' {
                pj += 1;
                if pj >= p.len() {
                    // Dangling `-` with the pattern exhausted (an unterminated
                    // class ending in `-`): the C just runs off the end skipping
                    // it, so any *earlier* member match still stands. Stop the
                    // scan and keep whatever `matched` we already have.
                    break;
                }
                let end = fold(p[pj], nocase);
                pj += 1;
                if (start <= ch1 && ch1 <= end) || (end <= ch1 && ch1 <= start) {
                    matched = true;
                }
            } else if start == ch1 {
                matched = true;
            }
        }
        if !matched {
            return None;
        }
        // Skip past the closing `]`. If the pattern ran out before `]` (an
        // unterminated class that still matched a member), the C treats the rest
        // of the pattern as consumed — leave `pj` at `p.len()` so the caller's
        // end-of-pattern check decides the overall match on the string's end.
        while pj < p.len() && p[pj] != ']' {
            pj += 1;
        }
        if pj < p.len() {
            pj += 1; // step over the `]`
        }
        return Some(pj);
    }

    // `\` — strip it and match the next char literally (the escaped char folds
    // like any other). A trailing `\` (nothing to escape) cannot match.
    let mut pj = pi;
    if pc == '\\' {
        pj += 1;
        if pj >= p.len() {
            return None;
        }
    }

    // Ordinary (or escaped) character: fold-compare one char from each side.
    if fold(p[pj], nocase) == fold(sc, nocase) {
        Some(pj + 1)
    } else {
        None
    }
}

/// Iterative glob matcher with a single `*` resume point, giving `O(n·m)` worst
/// case instead of the old recursion's `O(2^n)` (a pattern like
/// `a*a*…a*b` against a run of `a`s used to blow up exponentially — a denial of
/// service, since glob patterns come from attacker-controllable LSP buffers via
/// `string match`, `lsearch -glob`, `switch -glob`, `array names`, and the
/// compiler's glob folding). This is the classic backtracking wildcard match: on
/// a literal mismatch we rewind only to the most recent `*` and let it swallow
/// one more string char, never re-exploring earlier `*`s. Results are
/// byte-identical to the previous recursion (verified against the existing
/// tests).
fn do_match(p: &[char], mut pi: usize, s: &[char], mut si: usize, nocase: bool) -> bool {
    // The single resume point: `star_pat` is the pattern index just after the
    // last `*` we committed to, and `star_str` is the string index it was first
    // tried at; on a mismatch we backtrack there with `star_str` advanced by one.
    let mut star_pat: Option<usize> = None;
    let mut star_str = 0usize;

    loop {
        let advanced = if pi < p.len() && p[pi] == '*' {
            // Collapse a `*` run and record the single resume point. A trailing
            // `*` matches the remaining string outright.
            while pi < p.len() && p[pi] == '*' {
                pi += 1;
            }
            if pi >= p.len() {
                return true;
            }
            star_pat = Some(pi);
            star_str = si;
            true
        } else if pi < p.len()
            && si < s.len()
            && let Some(next_pi) = match_one(p, pi, s[si], nocase)
        {
            // The atom at `pi` matched `s[si]`: advance both. (`match_one` may
            // push `pi` to `p.len()` for an unterminated `[`; the end-of-pattern
            // check below then settles the match on whether `si` is also at the
            // end, exactly as the C's unclosed-bracket rule does.)
            pi = next_pi;
            si += 1;
            true
        } else {
            false
        };
        if advanced {
            continue;
        }

        // Could not advance the pattern here. If both sides ended together it is
        // a match; otherwise rewind to the last `*` (letting it absorb one more
        // char) if there is one, else fail.
        if pi >= p.len() && si >= s.len() {
            return true;
        }
        let Some(resume) = star_pat else {
            return false;
        };
        if star_str >= s.len() {
            return false; // the `*` has already swallowed the whole string
        }
        star_str += 1;
        si = star_str;
        pi = resume;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_and_empty() {
        assert!(string_match("", ""));
        assert!(string_match("abc", "abc"));
        assert!(!string_match("abc", "abd"));
        assert!(!string_match("abc", "ab"));
        assert!(!string_match("ab", "abc"));
    }

    #[test]
    fn star() {
        assert!(string_match("*", ""));
        assert!(string_match("*", "anything"));
        assert!(string_match("a*", "abc"));
        assert!(string_match("*c", "abc"));
        assert!(string_match("a*c", "abXYZc"));
        assert!(string_match("a**c", "ac")); // collapsed run
        assert!(!string_match("a*c", "ab"));
        assert!(string_match("::foo::*", "::foo::bar"));
        assert!(!string_match("::foo::*", "::baz::bar"));
    }

    #[test]
    fn question() {
        assert!(string_match("a?c", "abc"));
        assert!(!string_match("a?c", "ac"));
        assert!(!string_match("a?", "a"));
    }

    #[test]
    fn classes_and_ranges() {
        assert!(string_match("[abc]", "b"));
        assert!(!string_match("[abc]", "d"));
        assert!(string_match("[a-z]", "m"));
        assert!(string_match("[z-a]", "m")); // reversed range
        assert!(!string_match("[a-z]", "M"));
        assert!(string_match("foo[0-9]", "foo7"));
        assert!(string_match("[A-Za-z]*", "Hello"));
    }

    #[test]
    fn escapes() {
        assert!(string_match(r"a\*c", "a*c"));
        assert!(!string_match(r"a\*c", "abc"));
        assert!(string_match(r"\[x\]", "[x]"));
    }

    #[test]
    fn nocase() {
        assert!(string_case_match("ABC", "abc", true));
        assert!(string_case_match("[a-z]", "M", true));
        assert!(!string_case_match("ABC", "abc", false));
    }

    #[test]
    fn unclosed_class() {
        // Matches a member, then runs out of pattern: match iff string also ends.
        assert!(string_match("[abc", "a"));
        assert!(!string_match("[abc", "ab"));
        // No member match before the (missing) close ⇒ no match.
        assert!(!string_match("[xyz", "a"));
        // An unterminated class ending in a dangling `-` still honours an earlier
        // member match (the old recursion's run-to-`]` skip of the `-`).
        assert!(string_match("[ab-", "a"));
        assert!(!string_match("[ab-", "aa"));
    }

    #[test]
    fn star_backtracking_is_not_exponential() {
        // Regression test: the old recursive `*` handling was O(2^n), so
        // `a*a*…a*b` (16 stars) against a run of 'a's with no trailing 'b' took
        // ~14s for 32 'a's — a DoS reachable from `string match`/`switch -glob`/
        // `array names` on attacker-controlled buffers. The iterative single-
        // resume matcher is O(n·m), so 40 'a's must return (no match) instantly.
        let pattern = "a*a*a*a*a*a*a*a*a*a*a*a*a*a*a*a*b";
        let text = "a".repeat(40);
        let start = std::time::Instant::now();
        assert!(!string_match(pattern, &text));
        // Generous bound: the pathological recursion was seconds; this is µs.
        assert!(
            start.elapsed() < std::time::Duration::from_secs(1),
            "glob match took too long: {:?}",
            start.elapsed()
        );
    }
}
