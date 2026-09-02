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

/// Tcl `string match` over byte-valued strings.
///
/// Valid UTF-8 retains the ordinary Unicode-scalar semantics. An embedder can
/// also supply an invalid-UTF-8 plain string; that is outside Tcl's normal
/// script-created string representation, so the runtime's established policy
/// is deliberately narrow and collision-free: such a pattern/text pair matches
/// only when its bytes are identical. In particular, a valid `*` does not
/// reinterpret an invalid byte string through replacement characters.
#[must_use]
pub fn string_match_bytes(pattern: &[u8], text: &[u8]) -> bool {
    match (core::str::from_utf8(pattern), core::str::from_utf8(text)) {
        (Ok(pattern), Ok(text)) => string_match(pattern, text),
        _ => pattern == text,
    }
}

/// Whether `pattern` contains no glob metacharacter, so
/// [`string_match(pattern, text)`](string_match) is exactly `pattern == text`.
///
/// The metacharacters are the four [`match_one`] and the star loop act on —
/// `*`, `?`, `[` and `\` — named here rather than at each caller so a change
/// to the matcher's dialect cannot leave a "this is a plain name" test behind.
/// (An unterminated `[` is still special: it makes the match depend on where
/// both sides end, which is not string equality.)
///
/// Lets a caller that matches one pattern set against many names put the
/// literal patterns in a hash set and glob-match only the rest — the shape
/// [`crate::glob`]'s `namespace export` consumers need (issue #1297).
#[must_use]
pub fn is_literal(pattern: &str) -> bool {
    !pattern.contains(['*', '?', '[', '\\'])
}

/// Tcl `string match ?-nocase? pattern text`.
///
/// Two allocation-free fast paths stand in front of [`do_match`], which needs
/// random access with backtracking and so has to collect both sides into
/// `Vec<char>` first. Both are answers `do_match` would reach anyway, proven
/// equivalent over a pattern × text matrix by
/// `fast_paths_agree_with_the_general_matcher`:
///
/// - `*` matches every string, including the empty one.
/// - a pattern with no metacharacter ([`is_literal`]) is consumed one
///   folded char per folded char and must end where the text does, which is
///   exactly (folded) string equality.
///
/// They are here rather than at one caller because they are properties of the
/// glob dialect, and because those two shapes dominate every consumer: `*` and
/// a plain name are what `namespace export` / `namespace import` patterns
/// almost always are. The workspace import walk alone called this ~5 M times
/// to answer one `textDocument/references`, and the two `Vec<char>`
/// allocations per call were ~1 s of it (issue #1297).
#[must_use]
pub fn string_case_match(pattern: &str, text: &str, nocase: bool) -> bool {
    if pattern == "*" {
        return true;
    }
    if is_literal(pattern) {
        return if nocase {
            pattern
                .chars()
                .map(|c| fold(c, true))
                .eq(text.chars().map(|c| fold(c, true)))
        } else {
            pattern == text
        };
    }
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

    /// Issue #1297: [`string_case_match`]'s two allocation-free fast paths
    /// (`*`, and a metacharacter-free pattern) must answer exactly what the
    /// general matcher would. Checked over a full pattern × text matrix, in
    /// both case modes, against [`do_match`] called directly — so a future
    /// change to either side that makes them disagree fails here rather than
    /// silently changing what `namespace export` covers.
    ///
    /// The matrix deliberately mixes the shapes the fast paths claim (`*`, a
    /// plain name, the empty pattern, non-ASCII, case pairs) with ones they
    /// must decline to (`*x`, `?`, `[ab]`, an escaped literal, an
    /// unterminated class), so the test also pins that [`is_literal`] is not
    /// over-claiming.
    #[test]
    fn fast_paths_agree_with_the_general_matcher() {
        let patterns = [
            "*", "", "Resistor", "resistor", "R", "*x", "x*", "a*b", "?", "a?", "[ab]", "[abc",
            "\\*", "\\a", "p[ab]", "Ω", "ΩΩ", "a\\", "**", "*R*",
        ];
        let texts = [
            "", "*", "R", "Resistor", "resistor", "RESISTOR", "x", "ax", "ab", "a", "p a", "pa",
            "Ω", "ω", "a\\", "\\", "aXb",
        ];
        for pattern in patterns {
            for text in texts {
                for nocase in [false, true] {
                    let p: Vec<char> = pattern.chars().collect();
                    let s: Vec<char> = text.chars().collect();
                    let general = do_match(&p, 0, &s, 0, nocase);
                    assert_eq!(
                        string_case_match(pattern, text, nocase),
                        general,
                        "fast path disagreed: pattern={pattern:?} text={text:?} nocase={nocase}",
                    );
                }
            }
        }
    }

    /// [`is_literal`] must claim a pattern only when `string_match` against it
    /// really is string equality — the property the fast path above rests on.
    #[test]
    fn is_literal_claims_exactly_the_patterns_that_are_string_equality() {
        for pattern in ["", "R", "Resistor", "a b", "Ω", "-", "]", "{p}"] {
            assert!(is_literal(pattern), "should be literal: {pattern:?}");
            assert!(string_match(pattern, pattern), "{pattern:?}");
        }
        // Every metacharacter the matcher acts on disqualifies a pattern, and
        // each of these really does match something other than itself.
        for (pattern, other) in [
            ("*", "anything"),
            ("?", "z"),
            ("[ab]", "a"),
            ("\\*", "*"),
            ("a*", "abc"),
        ] {
            assert!(!is_literal(pattern), "should not be literal: {pattern:?}");
            assert!(string_match(pattern, other), "{pattern:?} vs {other:?}");
            assert_ne!(pattern, other);
        }
    }

    #[test]
    fn byte_match_preserves_invalid_utf8_identity() {
        assert!(string_match_bytes(b"plain*", b"plaintext"));
        assert!(string_match_bytes(b"raw\xff", b"raw\xff"));
        assert!(!string_match_bytes(b"raw\xff", b"raw\xfe"));
        assert!(!string_match_bytes(b"*", b"raw\xff"));
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
