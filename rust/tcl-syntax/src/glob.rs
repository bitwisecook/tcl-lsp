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

fn do_match(p: &[char], mut pi: usize, s: &[char], mut si: usize, nocase: bool) -> bool {
    loop {
        // End of both ⇒ match; end of pattern but not string ⇒ no match.
        if pi >= p.len() {
            return si >= s.len();
        }
        let pc = p[pi];
        // End of string with a non-`*` pattern char left ⇒ no match.
        if si >= s.len() && pc != '*' {
            return false;
        }

        // `*` — collapse a run, then try the tail at each remaining position.
        if pc == '*' {
            while pi < p.len() && p[pi] == '*' {
                pi += 1;
            }
            if pi >= p.len() {
                return true; // trailing `*` matches the rest
            }
            loop {
                if do_match(p, pi, s, si, nocase) {
                    return true;
                }
                if si >= s.len() {
                    return false;
                }
                si += 1;
            }
        }

        // `?` — any single char.
        if pc == '?' {
            pi += 1;
            si += 1;
            continue;
        }

        // `[...]` — a set, possibly with ranges.
        if pc == '[' {
            pi += 1;
            let ch1 = fold(s[si], nocase);
            si += 1;
            loop {
                if pi >= p.len() || p[pi] == ']' {
                    return false; // exhausted the set with no member match
                }
                let start = fold(p[pi], nocase);
                pi += 1;
                if pi < p.len() && p[pi] == '-' {
                    pi += 1;
                    if pi >= p.len() {
                        return false;
                    }
                    let end = fold(p[pi], nocase);
                    pi += 1;
                    if (start <= ch1 && ch1 <= end) || (end <= ch1 && ch1 <= start) {
                        break; // matched a range ([a-z] or reversed [z-a])
                    }
                } else if start == ch1 {
                    break; // matched a literal set member
                }
            }
            // Skip to past the closing `]` (C's unclosed-bracket handling: if we
            // run out, it's a match only if the string also ended).
            while pi < p.len() && p[pi] != ']' {
                pi += 1;
            }
            if pi >= p.len() {
                return si >= s.len();
            }
            pi += 1;
            continue;
        }

        // `\` — strip it and match the next char literally.
        if pc == '\\' {
            pi += 1;
            if pi >= p.len() {
                return false;
            }
        }

        // Ordinary character: fold-compare one char from each side.
        if fold(p[pi], nocase) != fold(s[si], nocase) {
            return false;
        }
        pi += 1;
        si += 1;
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
    }
}
