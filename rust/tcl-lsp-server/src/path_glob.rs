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

//! Path-glob matching for `tclLsp.diagnostics.exclude` (issue #1556).
//!
//! A file whose workspace-relative path matches any configured pattern
//! produces no diagnostics. The workspace carries no glob dependency —
//! [`crate::path_glob`] is hand-rolled the same way `tcl_syntax::glob` is,
//! and deliberately speaks the gitignore-flavoured dialect editors' exclude
//! settings use rather than Tcl's `string match`.
//!
//! # Compilation
//!
//! Each source pattern is put through, in order:
//!
//! 1. whitespace trimming; a blank result is skipped.
//! 2. brace alternation — `{a,b}` expands into one pattern per alternative
//!    before anything else is compiled, nesting included (`a{b,c{d,e}}f`).
//!    A `{` with no matching `}` (or the reverse) is an ordinary literal
//!    character, not an error, and `\{`, `\}`, `\,` escape. Expansion is
//!    capped at 1024 alternatives per source pattern; a pattern past the cap
//!    is dropped, so it matches nothing. A config file is not a
//!    denial-of-service surface, but the bound costs nothing.
//! 3. a leading `./` and a leading `/` are stripped.
//! 4. a trailing `/` is directory sugar: `dir/` is `dir/**`.
//! 5. a pattern containing `/` is a *path pattern*, matched segment-wise
//!    against the whole relative path. One with no `/` is a *name pattern*,
//!    matched against the base name only, so it hits at any depth.
//!
//! A malformed pattern — an unterminated `[` class — is inert: it matches
//! nothing, and never panics.
//!
//! # Matching
//!
//! Matching is case-sensitive and no wildcard ever matches `/`, which is
//! always a separator and cannot be escaped into a literal.
//!
//! - a segment that is exactly `**` is a globstar, matching zero or more
//!   whole path segments (so `docs/**` covers `docs/a.tcl` and
//!   `docs/x/y.tcl`, and `**/gen/*.tcl` covers `gen/a.tcl`).
//! - within a segment, `*` matches any run — possibly empty — of non-`/`
//!   characters, and a `**` inside a longer segment (`a**b`) is just `*`.
//! - `?` matches exactly one character.
//! - `[...]` is a character class, with `a-z` ranges, a leading `!` or `^`
//!   negating, a `]` first in the class taken as a literal `]`, and a `-`
//!   first or last taken as a literal `-`.
//! - `\x` escapes any character `x`, inside a class as well as outside.
//! - everything else is a literal.
//!
//! Both the globstar walk and the in-segment `*` walk are the standard
//! two-pointer backtracking loop: `O(n·m)` worst case, no recursion, no
//! regex engine.

use std::path::{Component, Path};

/// The most alternatives one source pattern may expand to before it is
/// dropped as unreasonable.
const MAX_ALTERNATIVES: usize = 1024;

/// A compiled set of path-glob exclusion patterns.
#[derive(Debug, Clone, Default)]
pub struct PathGlobSet {
    patterns: Vec<Compiled>,
}

impl PathGlobSet {
    /// Compile `patterns`, one glob each.
    ///
    /// Never fails: a blank pattern is skipped, and a malformed one — an
    /// unterminated `[` class, or a brace expansion past the alternative
    /// cap — is dropped, which is to say it matches nothing.
    #[must_use]
    pub fn new(patterns: &[String]) -> Self {
        let mut compiled = Vec::new();
        for raw in patterns {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(alternatives) = expand_braces(trimmed) {
                compiled.extend(
                    alternatives
                        .iter()
                        .map(String::as_str)
                        .filter_map(compile_alternative),
                );
            }
        }
        Self { patterns: compiled }
    }

    /// Whether the set holds no usable pattern, so [`Self::matches`] is
    /// always `false` and callers can skip the check entirely.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Whether the file matches any pattern.
    ///
    /// `rel_path` is the workspace-folder-relative path, `/`-separated, and
    /// `None` when the file lies outside every workspace folder; `file_name`
    /// is the base name. Path patterns are only consulted when `rel_path` is
    /// `Some`, name patterns always are.
    #[must_use]
    pub fn matches(&self, rel_path: Option<&str>, file_name: &str) -> bool {
        let segments: Option<Vec<&str>> =
            rel_path.map(|p| p.split('/').filter(|s| !s.is_empty()).collect());
        self.patterns.iter().any(|pattern| match pattern {
            Compiled::Name(tokens) => match_tokens(tokens, file_name),
            Compiled::Path(pattern_segments) => segments
                .as_ref()
                .is_some_and(|segs| match_segments(pattern_segments, segs)),
        })
    }
}

/// Render `path` as a `/`-separated string for matching.
///
/// Only [`Component::Normal`] components survive, so a root, a drive prefix
/// and any `.` are dropped; non-UTF-8 components go through lossily.
#[must_use]
pub fn slash_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// One compiled pattern, in the shape its source decided.
#[derive(Debug, Clone)]
enum Compiled {
    /// No `/` in the source: matched against the base name at any depth.
    Name(Vec<Token>),
    /// Matched segment-wise against the whole relative path.
    Path(Vec<Segment>),
}

/// One `/`-delimited piece of a path pattern.
#[derive(Debug, Clone)]
enum Segment {
    /// A bare `**`: zero or more whole segments.
    Globstar,
    /// An ordinary segment matcher.
    Tokens(Vec<Token>),
}

impl Segment {
    /// Whether this segment matches one path segment.
    fn matches(&self, segment: &str) -> bool {
        match self {
            Self::Globstar => true,
            Self::Tokens(tokens) => match_tokens(tokens, segment),
        }
    }
}

/// One matcher atom within a segment.
#[derive(Debug, Clone)]
enum Token {
    /// A literal character, which an escape may have produced.
    Literal(char),
    /// `*`: any run, possibly empty, of non-`/` characters.
    Any,
    /// `?`: exactly one character.
    One,
    /// `[...]`.
    Class(CharClass),
}

impl Token {
    /// Whether this token accepts the single character `c`.
    fn matches(&self, c: char) -> bool {
        match self {
            Self::Literal(x) => *x == c,
            Self::One | Self::Any => true,
            Self::Class(class) => class.matches(c),
        }
    }
}

/// A `[...]` character class.
#[derive(Debug, Clone)]
struct CharClass {
    negated: bool,
    items: Vec<ClassItem>,
}

impl CharClass {
    /// Whether the class accepts `c`.
    fn matches(&self, c: char) -> bool {
        let hit = self.items.iter().any(|item| match item {
            ClassItem::Char(x) => *x == c,
            ClassItem::Range(lo, hi) => *lo <= c && c <= *hi,
        });
        hit != self.negated
    }
}

/// One member of a character class.
#[derive(Debug, Clone)]
enum ClassItem {
    /// A single character.
    Char(char),
    /// An inclusive `a-z` range.
    Range(char, char),
}

/// Compile one brace-expanded alternative, or `None` if it is inert.
fn compile_alternative(pattern: &str) -> Option<Compiled> {
    let mut rest = pattern;
    while let Some(stripped) = rest.strip_prefix("./") {
        rest = stripped;
    }
    while let Some(stripped) = rest.strip_prefix('/') {
        rest = stripped;
    }
    if rest.is_empty() {
        return None;
    }
    let body = if rest.ends_with('/') {
        format!("{rest}**")
    } else {
        rest.to_string()
    };
    if body.contains('/') {
        let mut segments = Vec::new();
        for piece in body.split('/').filter(|piece| !piece.is_empty()) {
            segments.push(compile_segment(piece)?);
        }
        if segments.is_empty() {
            return None;
        }
        Some(Compiled::Path(segments))
    } else {
        Some(Compiled::Name(compile_tokens(&body)?))
    }
}

/// Compile one path segment.
fn compile_segment(piece: &str) -> Option<Segment> {
    if piece == "**" {
        Some(Segment::Globstar)
    } else {
        Some(Segment::Tokens(compile_tokens(piece)?))
    }
}

/// Compile a segment's characters into matcher tokens, or `None` when a
/// `[` class is left unterminated.
fn compile_tokens(piece: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = piece.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                let escaped = chars.get(i + 1).copied().unwrap_or('\\');
                tokens.push(Token::Literal(escaped));
                i += 2;
            }
            '*' => {
                if !matches!(tokens.last(), Some(Token::Any)) {
                    tokens.push(Token::Any);
                }
                i += 1;
            }
            '?' => {
                tokens.push(Token::One);
                i += 1;
            }
            '[' => {
                let (class, next) = parse_class(&chars, i + 1)?;
                tokens.push(Token::Class(class));
                i = next;
            }
            c => {
                tokens.push(Token::Literal(c));
                i += 1;
            }
        }
    }
    Some(tokens)
}

/// Parse a `[...]` class whose body starts at `start`, returning it with the
/// index just past the closing `]`. `None` means unterminated.
fn parse_class(chars: &[char], start: usize) -> Option<(CharClass, usize)> {
    let mut i = start;
    let negated = matches!(chars.get(i), Some('!' | '^'));
    if negated {
        i += 1;
    }
    let mut items = Vec::new();
    if matches!(chars.get(i), Some(']')) {
        items.push(ClassItem::Char(']'));
        i += 1;
    }
    loop {
        if *chars.get(i)? == ']' {
            return Some((CharClass { negated, items }, i + 1));
        }
        let (lo, after_lo) = read_class_char(chars, i)?;
        let ranged = matches!(chars.get(after_lo), Some('-'))
            && !matches!(chars.get(after_lo + 1), Some(']') | None);
        if ranged {
            let (hi, after_hi) = read_class_char(chars, after_lo + 1)?;
            items.push(ClassItem::Range(lo, hi));
            i = after_hi;
        } else {
            items.push(ClassItem::Char(lo));
            i = after_lo;
        }
    }
}

/// Read one class character, honouring `\x`, returning it with the index
/// just past it.
fn read_class_char(chars: &[char], i: usize) -> Option<(char, usize)> {
    let c = *chars.get(i)?;
    if c == '\\' {
        Some((*chars.get(i + 1)?, i + 2))
    } else {
        Some((c, i + 1))
    }
}

/// Two-pointer backtracking match of a token list against one segment.
fn match_tokens(tokens: &[Token], text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let mut ti = 0;
    let mut ci = 0;
    let mut star: Option<(usize, usize)> = None;
    while ci < chars.len() {
        let live = ti < tokens.len();
        if live && matches!(tokens[ti], Token::Any) {
            star = Some((ti, ci));
            ti += 1;
        } else if live && tokens[ti].matches(chars[ci]) {
            ti += 1;
            ci += 1;
        } else if let Some((star_ti, star_ci)) = star {
            star = Some((star_ti, star_ci + 1));
            ti = star_ti + 1;
            ci = star_ci + 1;
        } else {
            return false;
        }
    }
    tokens[ti..].iter().all(|t| matches!(t, Token::Any))
}

/// Two-pointer backtracking match of a segment list against a split path.
fn match_segments(pattern: &[Segment], segments: &[&str]) -> bool {
    let mut pi = 0;
    let mut si = 0;
    let mut star: Option<(usize, usize)> = None;
    while si < segments.len() {
        let live = pi < pattern.len();
        if live && matches!(pattern[pi], Segment::Globstar) {
            star = Some((pi, si));
            pi += 1;
        } else if live && pattern[pi].matches(segments[si]) {
            pi += 1;
            si += 1;
        } else if let Some((star_pi, star_si)) = star {
            star = Some((star_pi, star_si + 1));
            pi = star_pi + 1;
            si = star_si + 1;
        } else {
            return false;
        }
    }
    pattern[pi..].iter().all(|s| matches!(s, Segment::Globstar))
}

/// One brace group found in a pattern: the byte offsets of its `{` and `}`
/// and the alternatives between them.
struct Group {
    open: usize,
    close: usize,
    alternatives: Vec<String>,
}

/// Expand every brace group, or `None` once the alternative cap is passed.
///
/// A worklist rather than recursion: nesting depth is attacker-chosen and
/// only the alternative count is capped.
fn expand_braces(pattern: &str) -> Option<Vec<String>> {
    let mut done = Vec::new();
    let mut work = vec![pattern.to_string()];
    while let Some(current) = work.pop() {
        if let Some(group) = find_group(&current) {
            for alternative in &group.alternatives {
                let mut next = String::with_capacity(current.len());
                next.push_str(&current[..group.open]);
                next.push_str(alternative);
                next.push_str(&current[group.close + 1..]);
                work.push(next);
            }
        } else {
            done.push(current);
        }
        if done.len() + work.len() > MAX_ALTERNATIVES {
            return None;
        }
    }
    Some(done)
}

/// Find the first brace group that actually closes. An unmatched `{` is a
/// literal, so the scan simply moves past it.
fn find_group(pattern: &str) -> Option<Group> {
    let chars: Vec<(usize, char)> = pattern.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (_, c) = chars[i];
        if c == '\\' {
            i += 2;
        } else {
            let found = if c == '{' {
                scan_group(&chars, i, pattern)
            } else {
                None
            };
            if let Some(group) = found {
                return Some(group);
            }
            i += 1;
        }
    }
    None
}

/// Scan the group opening at `start`, splitting on its top-level commas.
/// `None` means it never closes.
fn scan_group(chars: &[(usize, char)], start: usize, pattern: &str) -> Option<Group> {
    let open = chars[start].0;
    let mut depth = 0usize;
    let mut alternatives = Vec::new();
    let mut part_start = open + 1;
    let mut i = start;
    while i < chars.len() {
        let (idx, c) = chars[i];
        if c == '\\' {
            i += 2;
        } else {
            if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    alternatives.push(pattern[part_start..idx].to_string());
                    return Some(Group {
                        open,
                        close: idx,
                        alternatives,
                    });
                }
            } else if c == ',' && depth == 1 {
                alternatives.push(pattern[part_start..idx].to_string());
                part_start = idx + 1;
            }
            i += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{PathGlobSet, slash_path};
    use std::path::PathBuf;

    fn set(patterns: &[&str]) -> PathGlobSet {
        let owned: Vec<String> = patterns.iter().map(|p| (*p).to_string()).collect();
        PathGlobSet::new(&owned)
    }

    fn hits(patterns: &[&str], rel: &str) -> bool {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        set(patterns).matches(Some(rel), name)
    }

    #[test]
    fn name_pattern_matches_at_any_depth() {
        assert!(hits(&["*.ruff"], "foo.ruff"));
        assert!(hits(&["*.ruff"], "a/b/c/foo.ruff"));
        assert!(!hits(&["*.ruff"], "a/foo.ruffx"));
        assert!(!hits(&["*.ruff"], "a/ruff"));
    }

    #[test]
    fn name_pattern_ignores_the_relative_path() {
        let globs = set(&["*.ruff"]);
        assert!(globs.matches(None, "foo.ruff"));
        assert!(globs.matches(Some("deep/deeper/foo.ruff"), "foo.ruff"));
    }

    #[test]
    fn question_mark_is_exactly_one_character() {
        assert!(hits(&["a?c.tcl"], "x/abc.tcl"));
        assert!(!hits(&["a?c.tcl"], "x/ac.tcl"));
        assert!(!hits(&["a?c.tcl"], "x/abbc.tcl"));
    }

    #[test]
    fn character_classes_and_ranges() {
        assert!(hits(&["[a-c]*.tcl"], "src/blah.tcl"));
        assert!(!hits(&["[a-c]*.tcl"], "src/zebra.tcl"));
        assert!(hits(&["[!a-c]*.tcl"], "src/zebra.tcl"));
        assert!(!hits(&["[!a-c]*.tcl"], "src/blah.tcl"));
        assert!(hits(&["[^a-c]*.tcl"], "src/zebra.tcl"));
    }

    #[test]
    fn class_with_leading_bracket_or_edge_dash_is_literal() {
        assert!(hits(&["[]]*.tcl"], "x/].tcl"));
        assert!(!hits(&["[]]*.tcl"], "x/a.tcl"));
        assert!(hits(&["[a-]*.tcl"], "x/-.tcl"));
        assert!(hits(&["[a-]*.tcl"], "x/a.tcl"));
        assert!(!hits(&["[a-]*.tcl"], "x/b.tcl"));
        assert!(hits(&["[-a]*.tcl"], "x/-x.tcl"));
    }

    #[test]
    fn unterminated_class_is_inert() {
        let globs = set(&["[abc*.tcl"]);
        assert!(!globs.matches(Some("x/a.tcl"), "a.tcl"));
        assert!(!globs.matches(Some("x/[abc.tcl"), "[abc.tcl"));
    }

    #[test]
    fn path_pattern_globstar_covers_a_whole_tree() {
        assert!(hits(&["docs/**"], "docs/a.tcl"));
        assert!(hits(&["docs/**"], "docs/x/y.tcl"));
        assert!(!hits(&["docs/**"], "src/docs.tcl"));
        assert!(!hits(&["docs/**"], "src/docs/a.tcl"));
    }

    #[test]
    fn leading_globstar_matches_zero_segments() {
        assert!(hits(&["**/gen/*.tcl"], "gen/a.tcl"));
        assert!(hits(&["**/gen/*.tcl"], "a/b/gen/c.tcl"));
        assert!(!hits(&["**/gen/*.tcl"], "gen/sub/c.tcl"));
    }

    #[test]
    fn star_never_crosses_a_separator() {
        assert!(hits(&["src/*/mod.tcl"], "src/a/mod.tcl"));
        assert!(!hits(&["src/*/mod.tcl"], "src/a/b/mod.tcl"));
        assert!(!hits(&["src/*/mod.tcl"], "src/mod.tcl"));
    }

    #[test]
    fn several_globstars_in_one_pattern() {
        assert!(hits(&["a/**/b/**/c"], "a/b/c"));
        assert!(hits(&["a/**/b/**/c"], "a/x/y/b/z/c"));
        assert!(hits(&["a/**/b/**/c"], "a/x/b/x/b/c"));
        assert!(!hits(&["a/**/b/**/c"], "a/x/y/c"));
        assert!(!hits(&["a/**/b/**/c"], "z/b/c"));
    }

    #[test]
    fn double_star_inside_a_segment_is_a_plain_star() {
        assert!(hits(&["src/a**b.tcl"], "src/axxb.tcl"));
        assert!(!hits(&["src/a**b.tcl"], "src/a/x/b.tcl"));
    }

    #[test]
    fn trailing_slash_is_directory_sugar() {
        assert!(hits(&["vendor/"], "vendor/x.tcl"));
        assert!(hits(&["vendor/"], "vendor/a/b.tcl"));
        assert!(!hits(&["vendor/"], "src/vendor.tcl"));
    }

    #[test]
    fn brace_alternation() {
        assert!(hits(&["{docs,samples}/**"], "docs/a.tcl"));
        assert!(hits(&["{docs,samples}/**"], "samples/deep/a.tcl"));
        assert!(!hits(&["{docs,samples}/**"], "src/a.tcl"));
    }

    #[test]
    fn nested_brace_alternation() {
        let pattern = ["src/{a,b{1,2}}/*.tcl"];
        assert!(hits(&pattern, "src/a/x.tcl"));
        assert!(hits(&pattern, "src/b1/x.tcl"));
        assert!(hits(&pattern, "src/b2/x.tcl"));
        assert!(!hits(&pattern, "src/b/x.tcl"));
        assert!(!hits(&pattern, "src/b3/x.tcl"));
    }

    #[test]
    fn unmatched_brace_is_a_literal() {
        assert!(hits(&["a{b.tcl"], "x/a{b.tcl"));
        assert!(!hits(&["a{b.tcl"], "x/ab.tcl"));
        assert!(hits(&["a}b.tcl"], "x/a}b.tcl"));
        assert!(hits(&["\\{a\\}.tcl"], "x/{a}.tcl"));
    }

    #[test]
    fn escapes_make_wildcards_literal() {
        let globs = set(&["\\*.tcl"]);
        assert!(globs.matches(Some("x/*.tcl"), "*.tcl"));
        assert!(!globs.matches(Some("x/foo.tcl"), "foo.tcl"));
        assert!(hits(&["\\[x].tcl"], "x/[x].tcl"));
        assert!(!hits(&["\\[x].tcl"], "x/x.tcl"));
    }

    #[test]
    fn leading_dot_slash_and_slash_are_stripped() {
        assert!(hits(&["./docs/**"], "docs/a.tcl"));
        assert!(hits(&["/docs/**"], "docs/a.tcl"));
        assert!(hits(&["/*.ruff"], "deep/a.ruff"));
    }

    #[test]
    fn matching_is_case_sensitive() {
        assert!(!hits(&["*.tcl"], "x/foo.TCL"));
        assert!(!hits(&["Docs/**"], "docs/a.tcl"));
        assert!(hits(&["Docs/**"], "Docs/a.tcl"));
    }

    #[test]
    fn blank_patterns_are_skipped() {
        let globs = set(&["", "   ", "\t"]);
        assert!(globs.is_empty());
        assert!(!globs.matches(Some("a/b.tcl"), "b.tcl"));
    }

    #[test]
    fn patterns_are_trimmed() {
        assert!(hits(&["  docs/**  "], "docs/a.tcl"));
    }

    #[test]
    fn empty_set_matches_nothing() {
        let globs = PathGlobSet::new(&[]);
        assert!(globs.is_empty());
        assert!(!globs.matches(Some("a/b.tcl"), "b.tcl"));
        assert!(!globs.matches(None, "b.tcl"));
    }

    #[test]
    fn path_patterns_need_a_relative_path() {
        let globs = set(&["docs/**", "*.ruff"]);
        assert!(!globs.matches(None, "a.tcl"));
        assert!(globs.matches(None, "a.ruff"));
        assert!(globs.matches(Some("docs/a.tcl"), "a.tcl"));
    }

    #[test]
    fn any_pattern_in_the_set_can_hit() {
        let globs = set(&["docs/**", "*.ruff", "vendor/"]);
        assert!(!globs.is_empty());
        assert!(!globs.matches(Some("src/a.tcl"), "a.tcl"));
        assert!(globs.matches(Some("vendor/deep/a.tcl"), "a.tcl"));
    }

    #[test]
    fn expansion_cap_drops_the_pattern() {
        let over = "{a,b}".repeat(11);
        assert!(set(&[over.as_str()]).is_empty());
        let under = "{a,b}".repeat(10);
        assert!(!set(&[under.as_str()]).is_empty());
    }

    #[test]
    fn slash_path_joins_normal_components() {
        assert_eq!(slash_path(&PathBuf::from("a/b/c.tcl")), "a/b/c.tcl");
        assert_eq!(slash_path(&PathBuf::from("/a/b/c.tcl")), "a/b/c.tcl");
        assert_eq!(slash_path(&PathBuf::from("./a/./b.tcl")), "a/b.tcl");
        assert_eq!(slash_path(&PathBuf::from("solo.tcl")), "solo.tcl");
    }
}
