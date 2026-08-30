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

/// The word-shaped `expr` operators, each paired with the oldest release whose
/// `expr` **lexeme table** contains it.
///
/// | operator | since | evidence |
/// |---|---|---|
/// | `eq`, `ne` | 8.4 | `tcl8.4.20/generic/tclParseExpr.c:141`, `tclCompExpr.c:133` |
/// | `in`, `ni` | 8.5 | TIP 201; `tcl8.5.19/generic/tclCompExpr.c:315` |
/// | `lt`, `le`, `gt`, `ge` | 9.0 | TIP 461; `tcl9.0.4/generic/tclCompExpr.c:306` |
///
/// This lives below the lexer because it is a *lexeme-boundary* fact, not only
/// a parser one. `ParseLexeme` recognises a word operator only in a release
/// that has it; before that the same spelling is swept up by the bareword scan
/// (`[A-Za-z0-9_]*`), which moves the token boundary rather than merely
/// rejecting the operator later. tclsh-verified:
///
/// | input | 8.6 | 9.0 |
/// |---|---|---|
/// | `1 lt_ 2` | `invalid bareword "lt_"` | `invalid character "_"` |
/// | `1 eq_ 2` | `invalid character "_"` | `invalid character "_"` |
///
/// `eq` exists in both, so the `_` is its own lexeme in both; `lt` does not
/// exist before 9.0, so `lt_` is one bareword there. A lexer that hard-codes
/// the operator set cannot tell these apart — hence
/// [`is_expr_word_operator`], the single source both the lexer and
/// `tcl-syntax`'s `OperatorSpec::expr_grammar_min_version` resolve through.
///
/// The iRules word operators (`contains`, `starts_with`, …) are gated by
/// dialect *identity* rather than a version threshold, so they are not here —
/// see `DialectProfile::is_irules`.
pub const EXPR_WORD_OPERATORS: &[(&str, crate::TclVersion)] = &[
    ("eq", crate::TclVersion::V8_4),
    ("ne", crate::TclVersion::V8_4),
    ("in", crate::TclVersion::V8_5),
    ("ni", crate::TclVersion::V8_5),
    ("lt", crate::TclVersion::V9_0),
    ("le", crate::TclVersion::V9_0),
    ("gt", crate::TclVersion::V9_0),
    ("ge", crate::TclVersion::V9_0),
];

/// The oldest release whose `expr` grammar has the word-shaped operator
/// `spelling`, or `None` when `spelling` is not a word operator at all.
///
/// See [`EXPR_WORD_OPERATORS`] for the table and its evidence.
#[must_use]
pub fn expr_word_operator_since(spelling: &str) -> Option<crate::TclVersion> {
    EXPR_WORD_OPERATORS
        .iter()
        .find_map(|&(s, since)| (s == spelling).then_some(since))
}

/// Whether `spelling` is a word-shaped `expr` operator under the
/// `expr_grammar_base` of some dialect — `DialectProfile::expr_grammar_base`,
/// the field that already exists to gate exactly these two TIPs.
///
/// A `None` base (an unversioned dialect: the lenient `tcl` sink, the `tk`
/// ingress profile, and `f5-bigip`, which is not Tcl at all) takes the
/// **newest** grammar, so every word operator is present. That is the same
/// default the rest of [`LexerGrammar`] uses — [`BracedVarStyle::Tcl9Nesting`],
/// [`NumberSyntax::Tcl90`] and [`ExprCommentStyle::Hash`] are all the newest
/// release's rule — and it is the answer a lexeme boundary needs: the operator
/// set only ever *grows*, so assuming the newest keeps `lt` splittable, where
/// assuming the oldest would fuse `lt_` into a bareword for a dialect that has
/// no reason to.
#[must_use]
pub fn is_expr_word_operator(spelling: &str, expr_grammar_base: Option<crate::TclVersion>) -> bool {
    expr_word_operator_since(spelling)
        .is_some_and(|since| expr_grammar_base.is_none_or(|base| base >= since))
}

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

    /// The rule of the dialect `name` resolves to, or the 9.x default when
    /// `name` is [`None`] — the sibling of
    /// [`NumberSyntax::of_dialect_name`] and
    /// [`EscapeSyntax::of_dialect_name`], for the same reason: the compiler
    /// threads a dialect *name* from `IrModule::dialect` and needs one way to
    /// turn it into this grammar fact (issue #1568).
    #[must_use]
    pub fn of_dialect_name(name: Option<&str>) -> Self {
        name.map_or(Self::default(), |n| {
            crate::DialectProfile::find(n)
                .unwrap_or_else(crate::DialectProfile::plain_tcl)
                .grammar
                .braced_var
        })
    }

    /// The rule of an already-resolved `profile`, or the 9.x default when the
    /// caller has none.
    ///
    /// The compiler layers that carry an `Option<&DialectProfile>` — the
    /// optimiser's `PassContext`, GVN's per-function `dialect`, the taint
    /// context, the W313 scan — each need exactly this mapping, and each grew
    /// its own copy of it during the issue-#1604 sweep. One copy lives here,
    /// beside [`Self::of_dialect_name`], so "no dialect means the default
    /// rule" is stated once: a layer that answered differently would read the
    /// same bytes under a rule the document was not lexed with.
    #[must_use]
    pub fn of_profile(profile: Option<&crate::DialectProfile>) -> Self {
        profile.map_or(Self::default(), |p| p.grammar.braced_var)
    }
}

/// The brace-line continuation axis — the F5 N-rules
/// (`docs/design/bigip-irule-parser-measurements.md` §2), a second
/// divergence independent of the implicit word break and likewise an
/// `f5-tcl` trunk fact (measurements §4a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BraceLineContinuation {
    /// Stock Tcl (and the unversioned default): a newline always
    /// terminates the command (barring backslash-newline continuation).
    #[default]
    Terminates,
    /// The F5 fork: a newline does **not** terminate a command when the
    /// next line's first non-whitespace character is `{` — the line is
    /// absorbed as further arguments (N1). Unconditional — independent of
    /// the command's identity, arity, or completeness (N2) — and applies
    /// at any nesting depth (N3). A blank, whitespace-only, or comment
    /// line still terminates the command (N4); backslash-newline rules
    /// are unchanged. The `else`/`elseif` single-newline lookahead is
    /// `if`'s own consumer-side rule (N5), not part of this axis.
    Continues,
}

impl BraceLineContinuation {
    /// Whether a next-line-`{` newline continues the command (the F5
    /// rule).
    #[must_use]
    pub fn continues(self) -> bool {
        matches!(self, Self::Continues)
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
    /// When true, the word break implied by a close delimiter — the F5
    /// R-rules (`docs/design/bigip-irule-parser-measurements.md` §1, §3):
    /// a word that *started* with `{` or `"` ends at its matching close
    /// delimiter, and any character other than whitespace or a command
    /// terminator immediately after it begins a **new word** (a
    /// zero-width ghost separator) instead of raising stock Tcl's "extra
    /// characters after close-brace/close-quote" error. Applies
    /// repeatedly and in every word position including the command name
    /// (R4); never fires after a bare word, `$var`, `${name}`, or
    /// `[cmd]` (R5); emits no diagnostic (R6). An axis of the `f5-tcl`
    /// trunk — measured byte-identical in TMM iRules, tmsh cli scripts,
    /// and iApp implementations (measurements §4a) — not iRules-only.
    ///
    /// The field keeps its historical name: it predates the measured
    /// `f5-tcl` trunk and is threaded by name through crates outside this
    /// change's blast radius.
    pub irules_brace_separator: bool,
    /// The brace-line continuation axis — see [`BraceLineContinuation`].
    pub brace_line_continuation: BraceLineContinuation,
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
    /// Which backslash-escape grammar the release decodes — see
    /// [`EscapeSyntax`].
    pub escapes: EscapeSyntax,
}

/// The backslash-escape grammar of a Tcl release: how many hex digits `\x`
/// swallows, whether `\U` exists at all, how far an octal escape runs, and
/// whether a decoded scalar above the basic multilingual plane survives.
///
/// `TclParseBackslash` changed twice across the supported range, and every
/// change moves both the *value* and the *width* of an escape — so a
/// version-blind decoder mis-reads real scripts *and* mis-measures them.
/// `\x4142` is `B` under 8.5 (one character, six bytes consumed) and `A42`
/// under 8.6 (three characters, four bytes consumed); `\U0001F600` is the
/// literal `U0001F600` under 8.5, U+FFFD under 8.6, and an emoji under 9.0.
///
/// Each variant below is evidenced from the release's own
/// `generic/tclParse.c`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum EscapeSyntax {
    /// Tcl 8.4 and 8.5: `\x` runs unbounded, `\U` does not exist, and an octal
    /// escape always takes up to three digits.
    ///
    /// `TclParseBackslash`'s `case 'x'` passes `numBytes-2` — the whole
    /// remaining input — as `TclParseHex`'s digit budget and then keeps
    /// `(unsigned char) result` (`tcl8.4.20/generic/tclParse.c:616-625`,
    /// `tcl8.5.19/generic/tclParse.c:903-914`), so `\x4142` consumes four
    /// digits and yields 0x42. There is no `case 'U'` at all, so `\U…` falls
    /// through to the default arm and is the literal character `U`. The octal
    /// arm takes a third digit whenever one is present, without 8.6's
    /// `result >= 0x20` guard (`tcl8.4.20:648-670`, `tcl8.5.19:938-957`), so
    /// `\400` is one NUL byte where 8.6 reads `\40` plus a literal `0`.
    Tcl84,
    /// Tcl 8.6: TIP 388 caps `\x` at two digits and adds `\U`, but the stock
    /// build is UTF-16-internal, so any scalar above U+FFFF degrades to U+FFFD.
    ///
    /// `case 'x'` passes `(numBytes > 3) ? 2 : numBytes-2`
    /// (`tcl8.6.16/generic/tclParse.c:900`) and `case 'U'` appears at
    /// `:934-946`. `tcl.h:2199` leaves `TCL_UTF_MAX` at 3, which makes
    /// `Tcl_UniCharToUtf` fall through its `ch <= 0x10FFFF` arm to
    /// `ch = 0xFFFD` for anything past the basic multilingual plane
    /// (`tcl8.6.16/generic/tclUtf.c`) — tclsh 8.6.14 confirms
    /// `string length [subst \\U0001F600]` is 1 and the character is U+FFFD.
    /// The *width* is 9.0's: the escape still consumes its digits.
    Tcl86,
    /// Tcl 9.0 and 9.1 (and the unversioned default): 8.6's grammar with a
    /// UCS-4 internal representation, so `\U0001F600` really is U+1F600.
    ///
    /// `TclParseBackslash` is byte-identical at 9.0.4 (`tclParse.c:846-870`)
    /// and 9.1b0; `tcl.h:2094` raises `TCL_UTF_MAX` to 4.
    #[default]
    Tcl90,
    /// `JimTcl` 0.76-0.84: a combination no C Tcl release has — 9.0's `\x`
    /// cap and wide `\U`, but 8.4's greedy octal escape.
    ///
    /// `JimEscape` in `jim.c` caps `\x` at two hex digits and implements
    /// `\u`, `\u{...}` and `\U` over a UCS-4 internal string, so
    /// `\U0001F600` really is U+1F600 (measured: `scan \U0001F600 %c` is
    /// 128512 in every modelled release, where 8.6 would give U+FFFD).
    /// Its octal arm, though, always takes a third digit and truncates to a
    /// byte, with none of 8.6's `result >= 0x20` guard: `subst \400` is a
    /// single NUL under Jim (`string length` 1, `scan %c` 0) where Tcl 8.6
    /// and 9.0 both read `\40` plus a literal `0` (`string length` 2,
    /// `scan %c` 32) — measured on jimsh 0.84 against tclsh 8.6/9.0.
    ///
    /// So Jim can borrow neither [`Self::Tcl86`] (which is BMP-only and
    /// would fold the emoji to U+FFFD) nor [`Self::Tcl90`] (which would cut
    /// the octal escape one digit short).
    Jim,
}

impl EscapeSyntax {
    /// Every escape grammar, oldest first — the counterpart of
    /// [`NumberSyntax::ALL`], for callers that must answer a question across
    /// all of them.
    pub const ALL: &'static [Self] = &[Self::Tcl84, Self::Tcl86, Self::Tcl90, Self::Jim];

    /// The grammar of `profile`, or the permissive 9.x default when no profile
    /// is loaded.
    #[must_use]
    pub fn of_profile(profile: Option<&crate::DialectProfile>) -> Self {
        profile.map_or(Self::default(), |p| p.grammar.escapes)
    }

    /// The grammar of the dialect `name` resolves to, or the 9.x default when
    /// `name` is [`None`].
    #[must_use]
    pub fn of_dialect_name(name: Option<&str>) -> Self {
        name.map_or(Self::default(), |n| {
            crate::DialectProfile::find(n)
                .unwrap_or_else(crate::DialectProfile::plain_tcl)
                .grammar
                .escapes
        })
    }

    /// How many hex digits `\x` may consume. [`None`] means unbounded — the
    /// 8.4/8.5 rule, where only the low byte of the accumulated value survives.
    #[must_use]
    pub fn hex_escape_digits(self) -> Option<usize> {
        match self {
            Self::Tcl84 => None,
            Self::Tcl86 | Self::Tcl90 | Self::Jim => Some(2),
        }
    }

    /// Whether `\UHHHHHHHH` is an escape at all (TIP 388, 8.6 onward). When
    /// false, `\U` is the literal character `U` and the digits are word text.
    #[must_use]
    pub fn has_wide_unicode(self) -> bool {
        !matches!(self, Self::Tcl84)
    }

    /// Whether an octal escape takes a **third** digit given the value its
    /// first two produced.
    ///
    /// 8.6 onward refuses one once the two-digit value has reached 0x20, so
    /// `\400` is `\40` plus a literal `0`; 8.4/8.5 always take it and truncate
    /// the result to a byte, so `\400` is a single NUL.
    #[must_use]
    pub fn octal_takes_third_digit(self, two_digit_value: u32) -> bool {
        match self {
            // Jim's octal arm has no `result >= 0x20` guard at all.
            Self::Tcl84 | Self::Jim => true,
            Self::Tcl86 | Self::Tcl90 => two_digit_value < 0x20,
        }
    }

    /// Whether a decoded scalar above U+FFFF degrades to U+FFFD because the
    /// release's internal string representation is UTF-16 — true for 8.6's
    /// stock `TCL_UTF_MAX 3` build.
    ///
    /// 8.4/8.5 are UTF-16 internally too, but no escape they *have* can reach
    /// past U+FFFF (`\u` stops at four digits and `\x` keeps one byte), so the
    /// flag is answered on its own terms rather than folded into the release
    /// order.
    #[must_use]
    pub fn is_bmp_only(self) -> bool {
        matches!(self, Self::Tcl84 | Self::Tcl86)
    }
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
    /// `JimTcl` 0.76-0.79: `0x` / `0o` / `0b` prefixes, leading zero is
    /// **decimal**, and neither the `0d` prefix nor `_` separators exist.
    ///
    /// Jim never adopted leading-zero octal, so `010` is 10 in every
    /// modelled release — the Tcl 9.0 answer, a decade before Tcl 9.0
    /// (measured; `jim_strtoull` in `jim.c` takes an explicit base).
    /// `expr 0d5` is a syntax error through 0.79 and `expr 1_000` is one in
    /// every release, so this is neither [`Self::Tcl85`] (which would read
    /// `010` as 8) nor [`Self::Tcl90`] (which would accept both).
    Jim,
    /// `JimTcl` 0.80 and later: [`Self::Jim`] plus the `0d` decimal prefix.
    ///
    /// 0.80 added `0d` alongside the `lt`/`le`/`gt`/`ge` operators —
    /// measured: `expr 0d10` is a syntax error on jimsh 0.79 and 10 on 0.80.
    /// `_` digit separators are still rejected, which is what keeps this
    /// distinct from [`Self::Tcl90`].
    Jim080,
}

impl NumberSyntax {
    /// Every numeral grammar, oldest first.
    ///
    /// For callers that must answer a question across all of them — e.g.
    /// "do the releases agree how to read this word?" — so adding a future
    /// grammar reaches them without each keeping its own list.
    pub const ALL: &'static [Self] =
        &[Self::Tcl84, Self::Tcl85, Self::Tcl90, Self::Jim, Self::Jim080];

    /// The grammar of `profile`, or the permissive 9.x default when no profile
    /// is loaded.
    ///
    /// The one way to get from a dialect profile to a numeral grammar. Written
    /// out by hand it is `profile.map_or(Tcl90, |p| p.grammar.numbers)`, which
    /// had accumulated eleven copies — each free to pick a different default and
    /// so to drift from the rest.
    #[must_use]
    pub fn of_profile(profile: Option<&crate::DialectProfile>) -> Self {
        profile.map_or(Self::default(), |p| p.grammar.numbers)
    }

    /// The grammar of the dialect `name` resolves to, or the 9.x default when
    /// `name` is [`None`].
    #[must_use]
    pub fn of_dialect_name(name: Option<&str>) -> Self {
        name.map_or(Self::default(), |n| {
            crate::DialectProfile::find(n)
                .unwrap_or_else(crate::DialectProfile::plain_tcl)
                .grammar
                .numbers
        })
    }

    /// Answer `f` under **every** grammar, returning `Some` only when they all
    /// agree — the sound reading for a consumer that has no release in hand.
    ///
    /// Which direction "no answer" points is the caller's decision and it is not
    /// always the same one: a predicate claiming a value *is* a number wants
    /// unanimity, while one refusing an optimisation because a value *could* be
    /// a number wants [`Self::any`]. Both were hand-rolled at eight sites before
    /// this; see `docs/design/contracts/numeric-tower-and-expr-semantics.md`.
    #[must_use]
    pub fn unanimous<T: PartialEq>(f: impl Fn(Self) -> T) -> Option<T> {
        let mut answers = Self::ALL.iter().map(|&n| f(n));
        let first = answers.next()?;
        answers.all(|a| a == first).then_some(first)
    }

    /// Whether `f` holds under **any** grammar — the permissive counterpart to
    /// [`Self::unanimous`], for a gate whose safe direction is to abstain from
    /// an optimisation rather than to claim a fact.
    #[must_use]
    pub fn any(f: impl Fn(Self) -> bool) -> bool {
        Self::ALL.iter().any(|&n| f(n))
    }

    /// Whether `f` holds under **every** grammar.
    #[must_use]
    pub fn every(f: impl Fn(Self) -> bool) -> bool {
        Self::ALL.iter().all(|&n| f(n))
    }

    /// Whether this grammar is Tcl 9.0's or later.
    ///
    /// For the handful of *non-grammar* rules that changed in the same release
    /// and have no numeral-syntax question of their own — `string is integer`
    /// becoming unbounded, for instance. Written out as
    /// `matches!(n, NumberSyntax::Tcl90)` it silently answers `false` for any
    /// grammar variant added later, which is the failure mode
    /// [`Self::ALL`] exists to prevent elsewhere.
    #[must_use]
    pub fn is_tcl9_or_later(self) -> bool {
        !matches!(self, Self::Tcl84 | Self::Tcl85)
    }

    /// Whether a bare leading `0` introduces an octal integer (`0755` == 493),
    /// as it does up to 8.6. False from 9.0, where it is plain decimal.
    #[must_use]
    pub fn leading_zero_is_octal(self) -> bool {
        match self {
            Self::Tcl84 | Self::Tcl85 => true,
            // Jim never adopted the rule — `expr 010` is 10 in every
            // modelled release, the Tcl 9.0 answer a decade early.
            Self::Tcl90 | Self::Jim | Self::Jim080 => false,
        }
    }

    /// Whether the `0b` (binary) and `0o` (octal) prefixes exist — 8.5 onward.
    #[must_use]
    pub fn has_binary_octal_prefix(self) -> bool {
        match self {
            Self::Tcl84 => false,
            Self::Tcl85 | Self::Tcl90 | Self::Jim | Self::Jim080 => true,
        }
    }

    /// Whether the explicit `0d` (decimal) prefix exists — 9.0 onward.
    #[must_use]
    pub fn has_decimal_prefix(self) -> bool {
        match self {
            Self::Tcl84 | Self::Tcl85 | Self::Jim => false,
            // Jim added `0d` in 0.80, alongside the `lt`/`ge` operators:
            // `expr 0d10` is a syntax error on jimsh 0.79 and 10 on 0.80.
            Self::Tcl90 | Self::Jim080 => true,
        }
    }

    /// The radix selected by an explicit numeral-prefix marker in this release.
    ///
    /// This is the grammar owner for the release availability of `0x`, `0b`,
    /// `0o`, and `0d`. Both the value parser and expression-boundary scanner
    /// must ask it instead of maintaining independent prefix tables.
    #[must_use]
    pub fn explicit_radix(self, marker: u8) -> Option<u8> {
        match marker {
            b'x' | b'X' => Some(16),
            b'b' | b'B' if self.has_binary_octal_prefix() => Some(2),
            b'o' | b'O' if self.has_binary_octal_prefix() => Some(8),
            b'd' | b'D' if self.has_decimal_prefix() => Some(10),
            _ => None,
        }
    }

    /// Whether `_` may separate digits (`1__000`, `0xff__ff`) — 9.0 onward.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn allows_digit_separators(self) -> bool {
        match self {
            Self::Tcl84 | Self::Tcl85 => false,
            Self::Tcl90 => true,
            // Pre-9.0 Tcl and Jim both reject `_`, but not for the same
            // reason, and the arms are the place that is recorded. This is
            // what keeps `Jim080` distinct from `Tcl90`.
            Self::Jim | Self::Jim080 => false,
        }
    }
}

impl Default for LexerGrammar {
    /// The modern-Tcl (9.x / unversioned) grammar — matches
    /// `LexerConfig::default()`.
    fn default() -> Self {
        Self {
            expand_syntax: true,
            irules_brace_separator: false,
            brace_line_continuation: BraceLineContinuation::Terminates,
            braced_var: BracedVarStyle::Tcl9Nesting,
            script_skips_leading_bom: true,
            expr_comments: ExprCommentStyle::Hash,
            numbers: NumberSyntax::Tcl90,
            escapes: EscapeSyntax::Tcl90,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BracedVarStyle, EscapeSyntax, ExprCommentStyle, LexerGrammar, NumberSyntax};

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
        assert!(!g.brace_line_continuation.continues());
        assert_eq!(g.braced_var, BracedVarStyle::Tcl9Nesting);
        assert!(g.script_skips_leading_bom);
        assert_eq!(g.expr_comments, ExprCommentStyle::Hash);
        assert_eq!(g.escapes, EscapeSyntax::Tcl90);
    }

    #[test]
    fn explicit_radix_prefixes_follow_the_number_syntax() {
        assert_eq!(NumberSyntax::Tcl84.explicit_radix(b'x'), Some(16));
        assert_eq!(NumberSyntax::Tcl84.explicit_radix(b'b'), None);
        assert_eq!(NumberSyntax::Tcl85.explicit_radix(b'b'), Some(2));
        assert_eq!(NumberSyntax::Tcl85.explicit_radix(b'o'), Some(8));
        assert_eq!(NumberSyntax::Tcl85.explicit_radix(b'd'), None);
        assert_eq!(NumberSyntax::Tcl90.explicit_radix(b'd'), Some(10));
    }

    #[test]
    fn escape_rules_follow_the_release() {
        // `\x` is unbounded up to 8.5 and two digits from 8.6 (TIP 388).
        assert_eq!(EscapeSyntax::Tcl84.hex_escape_digits(), None);
        assert_eq!(EscapeSyntax::Tcl86.hex_escape_digits(), Some(2));
        assert_eq!(EscapeSyntax::Tcl90.hex_escape_digits(), Some(2));
        // `\U` arrives with the same TIP.
        assert!(!EscapeSyntax::Tcl84.has_wide_unicode());
        assert!(EscapeSyntax::Tcl86.has_wide_unicode());
        assert!(EscapeSyntax::Tcl90.has_wide_unicode());
        // The octal third-digit guard is 8.6's; 8.4/8.5 always take it.
        assert!(EscapeSyntax::Tcl84.octal_takes_third_digit(0x20));
        assert!(!EscapeSyntax::Tcl86.octal_takes_third_digit(0x20));
        assert!(EscapeSyntax::Tcl86.octal_takes_third_digit(0x1F));
        // Only 9.0 has a UCS-4 internal representation.
        assert!(EscapeSyntax::Tcl84.is_bmp_only());
        assert!(EscapeSyntax::Tcl86.is_bmp_only());
        assert!(!EscapeSyntax::Tcl90.is_bmp_only());
        // Jim: 9.0's `\x` cap and wide `\U`, but 8.4's greedy octal — the
        // combination no C Tcl release has. Measured on jimsh 0.84.
        assert_eq!(EscapeSyntax::Jim.hex_escape_digits(), Some(2));
        assert!(EscapeSyntax::Jim.has_wide_unicode());
        assert!(!EscapeSyntax::Jim.is_bmp_only());
        assert!(EscapeSyntax::Jim.octal_takes_third_digit(0x20));
        assert_eq!(EscapeSyntax::default(), EscapeSyntax::Tcl90);
    }

    #[test]
    fn every_escape_grammar_is_listed() {
        // `ALL` must stay exhaustive: a variant missing from it silently
        // narrows every cross-release sweep built on it.
        for syntax in EscapeSyntax::ALL {
            match syntax {
                EscapeSyntax::Tcl84
                | EscapeSyntax::Tcl86
                | EscapeSyntax::Tcl90
                | EscapeSyntax::Jim => {}
            }
        }
        assert_eq!(EscapeSyntax::ALL.len(), 4);
    }
}
