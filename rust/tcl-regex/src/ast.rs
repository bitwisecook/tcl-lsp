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

//! The compiled-regex abstract syntax tree — an idiomatic Rust analogue of the
//! C engine's `struct subre` tree, plus the character-set / class model that
//! replaces the colour map. The matcher in [`crate::exec`] walks this tree.
//!
//! We do not mirror the C colour machinery: a [`Node::Set`] carries its own
//! character predicate. Newline semantics (`-linestop` for `.`/`[^…]`) and
//! case-folding (`-nocase`) are resolved at compile time and baked into the
//! set, so the matcher only asks "does codepoint `c` belong to this set?".

use crate::defs::Chr;

/// A POSIX character class (`[:alpha:]`, …) plus the ARE shorthands. Membership
/// is decided by [`ClassKind::matches`], following Tcl 9 semantics (ASCII-exact;
/// Unicode via Rust's codepoint predicates, which agree with Tcl's tables for
/// the cases `reg.test` exercises).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassKind {
    Alnum,
    Alpha,
    Ascii,
    Blank,
    Cntrl,
    Digit,
    Graph,
    Lower,
    Print,
    Punct,
    Space,
    Upper,
    Xdigit,
}

impl ClassKind {
    /// Map a `[:name:]` to its kind, or `None` if unknown (`REG_ECTYPE`).
    #[must_use]
    pub fn from_name(name: &str) -> Option<ClassKind> {
        Some(match name {
            "alnum" => ClassKind::Alnum,
            "alpha" => ClassKind::Alpha,
            "ascii" => ClassKind::Ascii,
            "blank" => ClassKind::Blank,
            "cntrl" => ClassKind::Cntrl,
            "digit" => ClassKind::Digit,
            "graph" => ClassKind::Graph,
            "lower" => ClassKind::Lower,
            "print" => ClassKind::Print,
            "punct" => ClassKind::Punct,
            "space" => ClassKind::Space,
            "upper" => ClassKind::Upper,
            "xdigit" => ClassKind::Xdigit,
            _ => return None,
        })
    }

    /// Whether codepoint `c` is a member of this class.
    #[must_use]
    pub fn matches(self, c: Chr) -> bool {
        let ch = char::from_u32(c);
        match self {
            ClassKind::Alnum => ch.is_some_and(is_alpha_char) || is_digit(c),
            ClassKind::Alpha => ch.is_some_and(is_alpha_char),
            ClassKind::Ascii => c <= 0x7F,
            ClassKind::Blank => c == u32::from(b'\t') || c == u32::from(b' '),
            ClassKind::Cntrl => ch.is_some_and(char::is_control),
            ClassKind::Digit => is_digit(c),
            ClassKind::Graph => ch.is_some_and(is_graph),
            ClassKind::Lower => ch.is_some_and(char::is_lowercase),
            ClassKind::Print => ch.is_some_and(|c| is_graph(c) || c == ' '),
            ClassKind::Punct => ch.is_some_and(is_punct),
            ClassKind::Space => ch.is_some_and(is_space),
            ClassKind::Upper => ch.is_some_and(char::is_uppercase),
            ClassKind::Xdigit => ch.is_some_and(|c| c.is_ascii_hexdigit()),
        }
    }
}

fn is_alpha_char(c: char) -> bool {
    c.is_alphabetic()
}

fn is_digit(c: Chr) -> bool {
    // Tcl's [:digit:] / `\d`: decimal digits. `char::to_digit(c, 10)` succeeds
    // only for the ASCII `0`-`9`, so this is exactly the ASCII decimal-digit set
    // (Unicode "decimal digit" codepoints all fold to those under radix 10).
    char::from_u32(c).is_some_and(|c| c.is_ascii_digit())
}

fn is_space(c: char) -> bool {
    // Tcl's [:space:]: the ASCII whitespace set plus Unicode whitespace.
    c.is_whitespace()
}

fn is_graph(c: char) -> bool {
    // Visible: not a control, not whitespace.
    !c.is_control() && !c.is_whitespace()
}

fn is_punct(c: char) -> bool {
    // Tcl [:punct:]: graphic, non-alphanumeric. (ASCII-exact; close enough on
    // Unicode for the cases the suite covers.)
    is_graph(c) && !c.is_alphanumeric()
}

/// A set of codepoints that one [`Node::Set`] matches. Built at compile time
/// with `-nocase` and `-linestop` already folded in.
#[derive(Clone, Debug, Default)]
pub struct CharSet {
    /// `true` for `[^…]` / `.` — membership is complemented.
    pub negated: bool,
    /// Inclusive codepoint ranges (single chars are `(c, c)`).
    pub ranges: Vec<(Chr, Chr)>,
    /// POSIX classes referenced (`[:alpha:]`, `\d`-expansions, …).
    pub classes: Vec<ClassKind>,
    /// `-nocase`: test the case-folded variants of `c` as well.
    pub nocase: bool,
    /// `-linestop`: never match a newline (applies to `.` and `[^…]`).
    pub exclude_newline: bool,
}

impl CharSet {
    /// A `.`-style set: matches everything (minus newline under `-linestop`).
    #[must_use]
    pub fn any(exclude_newline: bool) -> CharSet {
        CharSet {
            negated: true,
            exclude_newline,
            ..CharSet::default()
        }
    }

    /// A single literal codepoint.
    #[must_use]
    pub fn one(c: Chr, nocase: bool) -> CharSet {
        CharSet {
            ranges: vec![(c, c)],
            nocase,
            ..CharSet::default()
        }
    }

    fn raw_member(&self, c: Chr) -> bool {
        self.ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi)
            || self.classes.iter().any(|k| k.matches(c))
    }

    /// Whether codepoint `c` is matched by this set.
    #[must_use]
    pub fn matches(&self, c: Chr) -> bool {
        let mut inside = self.raw_member(c);
        if !inside && self.nocase {
            for v in case_variants(c) {
                if v != c && self.raw_member(v) {
                    inside = true;
                    break;
                }
            }
        }
        let mut result = if self.negated { !inside } else { inside };
        if result && self.exclude_newline && c == u32::from(b'\n') {
            result = false;
        }
        result
    }
}

/// The up-to-three case variants of a codepoint (lower, upper, title-ish),
/// used for `-nocase` matching and for expanding case-insensitive literals.
#[must_use]
pub fn case_variants(c: Chr) -> Vec<Chr> {
    let Some(ch) = char::from_u32(c) else {
        return vec![c];
    };
    let mut out = vec![c];
    // `to_lowercase`/`to_uppercase` can yield multi-char expansions (e.g. ß);
    // for single-codepoint folding we only take the 1:1 cases, matching how
    // Tcl's `Tcl_UniCharToLower`/`ToUpper` operate on one codepoint.
    let mut push = |s: char| {
        let v = s as u32;
        if !out.contains(&v) {
            out.push(v);
        }
    };
    if let Some(l) = ch.to_lowercase().next()
        && ch.to_lowercase().count() == 1
    {
        push(l);
    }
    if let Some(u) = ch.to_uppercase().next()
        && ch.to_uppercase().count() == 1
    {
        push(u);
    }
    out
}

/// A zero-width assertion (constraint).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// `^` — beginning of line/string (line-aware per `lineanchor`).
    Bol,
    /// `$` — end of line/string.
    Eol,
    /// `\A` — absolute beginning of string.
    Bos,
    /// `\Z` — absolute end of string.
    Eos,
    /// `\m` / `[[:<:]]` — beginning of a word.
    WordBegin,
    /// `\M` / `[[:>:]]` — end of a word.
    WordEnd,
    /// `\y` — a word boundary.
    WordBoundary,
    /// `\Y` — not a word boundary.
    NotWordBoundary,
}

/// Match-preference for a quantifier (POSIX longest vs. ARE shortest `*?`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pref {
    Longer,
    Shorter,
}

/// The "leading" match preference of a node — the first quantifier with an
/// imposed preference, left to right (mirrors the C `PREF2` propagation). `{m}`
/// exact bounds impose none and pass their operand's preference through.
#[must_use]
pub fn leading_pref(node: &Node) -> Option<Pref> {
    match node {
        Node::Empty | Node::Set(_) | Node::Anchor(_) | Node::Look { .. } | Node::Backref { .. } => {
            None
        }
        Node::Capture { sub, .. } => leading_pref(sub),
        Node::Repeat {
            sub,
            min,
            max,
            pref,
        } => {
            if min == max {
                leading_pref(sub)
            } else {
                Some(*pref)
            }
        }
        Node::Concat(items) => items.iter().find_map(leading_pref),
        // A multi-branch alternation has no leading shortest preference: the C
        // engine seeds the `'|'` subre with `LONGER` (`sub_re(v, '|', LONGER,
        // …)`), and `PREF2` keeps that seed regardless of the branches' own
        // preferences — so the overall extent is the POSIX-longest one across
        // alternatives (`a+?|aa` on "aa" matches "aa", not the lazy "a"). The
        // branch preferences only matter once an alternative is chosen, during
        // dissection. A single-branch alternation is collapsed away by the
        // parser (see `Parser::parse_alt`), so this arm is the ≥2-branch case;
        // we still pass a lone branch through for robustness.
        Node::Alt(branches) => match branches.as_slice() {
            [only] => leading_pref(only),
            _ => Some(Pref::Longer),
        },
    }
}

/// One AST node. Capturing parens become [`Node::Capture`]; the rest mirror
/// the structural `subre` ops `=`/`.`/`|`/`*`/`(`/`b`.
#[derive(Clone, Debug)]
pub enum Node {
    /// Matches the empty string.
    Empty,
    /// Matches exactly one codepoint in the set.
    Set(CharSet),
    /// A zero-width assertion.
    Anchor(Anchor),
    /// Lookahead constraint `(?=…)` (`positive == true`) / `(?!…)`.
    Look { positive: bool, sub: Box<Node> },
    /// Concatenation of sub-nodes (matched left to right).
    Concat(Vec<Node>),
    /// Alternation `a|b|…` (branches in source order).
    Alt(Vec<Node>),
    /// Capturing group with 1-based subexpression number.
    Capture { subno: usize, sub: Box<Node> },
    /// Iteration `sub{min,max}` (`max == DUPINF` for unbounded).
    Repeat {
        sub: Box<Node>,
        min: i32,
        max: i32,
        pref: Pref,
    },
    /// Backreference to capture `subno`, optionally quantified `{min,max}`.
    Backref {
        subno: usize,
        min: i32,
        max: i32,
        pref: Pref,
    },
}
