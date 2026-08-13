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

//! Dialect-derived lexing grammar knobs.
//!
//! [`BracedVarStyle`] and [`LexerGrammar`] live here — below `tcl-lexer` —
//! so the lexer's `LexerConfig` and the `DialectProfile` catalog share one
//! definition of the per-dialect grammar surface instead of keeping parallel
//! string-keyed tables.

/// The dialect's `${…}` variable-name delimiting rule — Tcl 9.0 changed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BracedVarStyle {
    /// Tcl 9.x (and the unversioned default): `Tcl_ParseVarName` tracks
    /// nested `{…}` pairs and treats `\X` as an inert two-character unit
    /// inside the braces, so `${a{b}c}` names the variable `a{b}c`
    /// (tcl9.0.1 `tclParse.c`, the `braceCount` loop).
    #[default]
    Tcl9Nesting,
    /// The 8.x family (8.4–8.6, iRules/iApps, EDA): the name runs to the
    /// FIRST literal `}` — no nesting, no backslash processing — so
    /// `${a{b}c}` names `a{b` and `c}` is ordinary word text
    /// (8.6.14 `tclParse.c:1466`, tclsh-verified).
    FirstClose,
}

impl BracedVarStyle {
    /// Whether this style tracks nested `{…}` / `\X` pairs (the Tcl 9 rule).
    #[must_use]
    pub fn nests(self) -> bool {
        matches!(self, Self::Tcl9Nesting)
    }
}

/// Whether `#` starts a comment inside an `[expr]` body — Tcl 9.0 added it
/// (TIP 582).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ExprCommentStyle {
    /// Tcl 9.x (and the unversioned default): at any point in the expression
    /// except within double quotes or braces, `#` begins a comment that lasts
    /// to the end of the line or the end of the expression, whichever comes
    /// first (`doc/expr.n`'s `.VS TIP582` block). `COMMENT` is a first-class
    /// lexeme (9.0.4 `tclCompExpr.c:168`) that `ParseExpr` skips like a
    /// whitespace run (`:701`), so it may appear anywhere whitespace may.
    #[default]
    Hash,
    /// The 8.x family (8.4-8.6, iRules/iApps, EDA): `#` begins no lexeme, so it
    /// is an ordinary invalid expression character and C reports
    /// `invalid character "#"`. Neither the `COMMENT` lexeme nor
    /// `ParseLexeme`'s `case '#':` exists in `tclCompExpr.c` at `core-8-5-19`
    /// or `core-8-6-16`, and 8.4 — which parsed `expr` in `tclParseExpr.c`
    /// entirely — has no `#` lexeme either.
    None,
}

impl ExprCommentStyle {
    /// Whether `#` begins a comment (the Tcl 9 rule).
    #[must_use]
    pub fn comments(self) -> bool {
        matches!(self, Self::Hash)
    }
}

/// The dialect-derived slice of the lexer configuration — exactly the fields
/// of `tcl_lexer::LexerConfig` that vary *by dialect* (as opposed to the
/// call-site knobs: strict quoting, sub-lexing base offsets).
///
/// A `DialectProfile` carries one of these; `LexerConfig` is built from it.
/// The profile's copy is the single source of these values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LexerGrammar {
    /// When true, `{*}` at a word boundary followed by a non-separator is
    /// argument expansion (TIP 157). True for Tcl 8.5+ and dialects built on
    /// 8.5+; false for Tcl 8.4 and iRules.
    pub expand_syntax: bool,
    /// When true, `}{` at a brace-string boundary separates two words
    /// (a zero-width ghost separator). iRules-only.
    pub irules_brace_separator: bool,
    /// How a `${…}` variable name is delimited — see [`BracedVarStyle`].
    pub braced_var: BracedVarStyle,
    /// When true, the script reader skips a leading UTF-8 byte-order mark
    /// (U+FEFF) at offset 0 of a *file* before evaluating it.
    ///
    /// Tcl 9.0's `source` does (`Tcl_FSEvalFileEx` strips the BOM after
    /// decoding); Tcl 8.x's does not, so an 8.x script whose first byte is a
    /// BOM really does fail with `invalid command name "﻿set"` and the
    /// unresolved-command diagnostic on it is correct.  Only the *file* entry
    /// point skips it — a BOM inside a string, or partway through a script,
    /// is ordinary data in every version, which is why this is a grammar
    /// property consulted once at the top of a file analysis rather than a
    /// lexer rule that fires wherever U+FEFF appears.
    pub script_skips_leading_bom: bool,
    /// Whether `#` inside an `[expr]` body begins a comment — see
    /// [`ExprCommentStyle`].
    ///
    /// Derived: `runtime_base >= V9_0`. TIP 582 landed in the 8.7 / 9.0
    /// development cycle, and since 8.7 is not a version this crate models,
    /// `>= V9_0` is the exact gate for the supported set.
    pub expr_comments: ExprCommentStyle,
    /// Which numeric literal forms the release accepts — see [`NumberSyntax`].
    pub numbers: NumberSyntax,
}

/// The numeric-literal grammar of a Tcl release: which radix prefixes exist,
/// whether a bare leading `0` means octal, and whether `_` digit separators are
/// allowed.
///
/// These rules changed twice across the supported range, so a version-blind
/// number parser silently mis-reads real scripts: `0755` is 493 under 8.6 and
/// 755 under 9.0, and `0o17` is a *bareword* under 8.4 where it is 15 under 8.5.
/// Each variant below is evidenced from the release's own `generic/` sources.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum NumberSyntax {
    /// Tcl 8.4: `0x` hex and octal-by-leading-zero only.
    ///
    /// 8.4 predates `TclParseNumber` — it has no `tclStrToD.c` at all and
    /// parses integers with `strtoul`-style base-0 scanning, which knows `0x`
    /// and leading-zero octal and nothing else. So `0b101` / `0o17` / `0d5`
    /// are not numbers here; they lex as barewords.
    Tcl84,
    /// Tcl 8.5 and 8.6: adds the `0b` / `0o` prefixes; leading zero is still
    /// octal.
    ///
    /// `tclStrToD.c` at `core-8-5-19` and `core-8-6-16` has the `ZERO_B` and
    /// `ZERO_O` states but **no** `ZERO_D`, and carries `#undef KILL_OCTAL`
    /// (whose comment reads "Define `KILL_OCTAL` to suppress interpretation of
    /// numbers with leading zero as octal") — so leading-zero octal is active.
    /// Neither release mentions `'_'` anywhere in that file.
    Tcl85,
    /// Tcl 9.0 and later: adds the `0d` prefix and `_` digit separators, and
    /// leading zero is **decimal**.
    ///
    /// `tclStrToD.c` at 9.0.4 gains the `ZERO_D` state and seven `'_'` sites,
    /// and `changes.md` records "`0NNN` format is no longer octal
    /// interpretation. Use `0oNNN`." (`doc/expr.n` lists all four prefixes.)
    #[default]
    Tcl90,
}

impl NumberSyntax {
    /// Every numeral grammar, oldest first.
    ///
    /// For callers that must answer a question across all of them — e.g.
    /// "do the releases agree how to read this word?" — so adding a future
    /// grammar reaches them without each keeping its own list.
    pub const ALL: &'static [Self] = &[Self::Tcl84, Self::Tcl85, Self::Tcl90];

    /// Whether a bare leading `0` introduces an octal integer (`0755` == 493),
    /// as it does up to 8.6. False from 9.0, where it is plain decimal.
    #[must_use]
    pub fn leading_zero_is_octal(self) -> bool {
        matches!(self, Self::Tcl84 | Self::Tcl85)
    }

    /// Whether the `0b` (binary) and `0o` (octal) prefixes exist — 8.5 onward.
    #[must_use]
    pub fn has_binary_octal_prefix(self) -> bool {
        !matches!(self, Self::Tcl84)
    }

    /// Whether the explicit `0d` (decimal) prefix exists — 9.0 onward.
    #[must_use]
    pub fn has_decimal_prefix(self) -> bool {
        matches!(self, Self::Tcl90)
    }

    /// Whether `_` may separate digits (`1__000`, `0xff__ff`) — 9.0 onward.
    #[must_use]
    pub fn allows_digit_separators(self) -> bool {
        matches!(self, Self::Tcl90)
    }
}

impl Default for LexerGrammar {
    /// The modern-Tcl (9.x / unversioned) grammar — matches
    /// `LexerConfig::default()`.
    fn default() -> Self {
        Self {
            expand_syntax: true,
            irules_brace_separator: false,
            braced_var: BracedVarStyle::Tcl9Nesting,
            script_skips_leading_bom: true,
            expr_comments: ExprCommentStyle::Hash,
            numbers: NumberSyntax::Tcl90,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BracedVarStyle, ExprCommentStyle, LexerGrammar};

    #[test]
    fn nesting_rule_is_tcl9_only() {
        assert!(BracedVarStyle::Tcl9Nesting.nests());
        assert!(!BracedVarStyle::FirstClose.nests());
        assert_eq!(BracedVarStyle::default(), BracedVarStyle::Tcl9Nesting);
    }

    #[test]
    fn expr_comments_are_tcl9_only() {
        assert!(ExprCommentStyle::Hash.comments());
        assert!(!ExprCommentStyle::None.comments());
        assert_eq!(ExprCommentStyle::default(), ExprCommentStyle::Hash);
    }

    #[test]
    fn default_grammar_is_modern_tcl() {
        let g = LexerGrammar::default();
        assert!(g.expand_syntax);
        assert!(!g.irules_brace_separator);
        assert_eq!(g.braced_var, BracedVarStyle::Tcl9Nesting);
        assert!(g.script_skips_leading_bom);
        assert_eq!(g.expr_comments, ExprCommentStyle::Hash);
    }
}
