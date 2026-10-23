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
//! so the lexer's `LexerConfig` and the `DialectProfile` catalogue share one
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
    /// turn it into this grammar fact.
    #[must_use]
    pub fn of_dialect_name(name: Option<&str>) -> Self {
        grammar_of_dialect_name(name).braced_var
    }

    /// The rule of an already-resolved `profile`, or the 9.x default when the
    /// caller has none.
    ///
    /// The compiler layers that carry an `Option<&DialectProfile>` — the
    /// optimiser's `PassContext`, GVN's per-function `dialect`, the taint
    /// context, the W313 scan — each need exactly this mapping, and growing
    /// separate copies of it would let them drift. One copy lives here,
    /// beside [`Self::of_dialect_name`], so "no dialect means the default
    /// rule" is stated once: a layer that answered differently would read the
    /// same bytes under a rule the document was not lexed with.
    #[must_use]
    pub fn of_profile(profile: Option<&crate::DialectProfile>) -> Self {
        profile.map_or(Self::default(), |p| p.grammar.braced_var)
    }
}

/// The [`LexerGrammar`] a dialect *name* selects — the one place a name
/// becomes a grammar.
///
/// Every `of_dialect_name` axis constructor delegates here, so a layer that
/// holds only a name (codegen from `IrModule::dialect`, the optimiser's
/// `PassContext`, taint) reads the same grammar the document was lexed with
/// rather than a per-axis approximation of it.
///
/// It resolves through [`crate::model::DialectPoint`], which holds
/// the truth: a grammar is a function of `(family, release, build)`, so an
/// environment names its family and ladder and needs no resolved-grammar row.
/// That is why `jim` works at all — it is an environment with no catalogue
/// profile, so an axis constructor that skipped `DialectPoint` and fell
/// straight to the catalogue would answer `Tcl90` for a Jim document
/// instead. `tk` has the same shape.
///
/// A name carries only the environment's *default* release, so this cannot
/// tell jim 0.79 from 0.84 or tcl8.5 from 8.6. A caller that knows the
/// document's release should resolve a [`crate::model::DialectPoint`] with
/// [`crate::model::DialectPoint::of_name_and_release`] and take its grammar.
#[must_use]
pub fn grammar_of_dialect_name(name: Option<&str>) -> LexerGrammar {
    if let Some(point) = crate::model::DialectPoint::of_dialect_name(name) {
        return point.grammar();
    }
    // No Tcl ladder: either no name at all, or the one shipped dialect that
    // is not Tcl (`f5-bigip`, the BIG-IP config surface), whose grammar the
    // legacy catalogue still states.
    name.and_then(crate::DialectProfile::find)
        .map_or_else(LexerGrammar::default, |p| p.grammar)
}

/// Which literal source bytes may appear in a `$name(index)` variable read.
///
/// Tcl 9 changed the mask passed from `Tcl_ParseVarName` to `ParseTokens`.
/// Tcl 8.x stops only at the closing `)`, so an opening parenthesis, quote,
/// or brace is ordinary index text. Tcl 9.x stops on those bytes as well and
/// raises `invalid character in array index`. The rule applies only to raw
/// bytes at the index token level: a backslash escape, `${…}` / `$name(…)`,
/// or `[…]` substitution shields its own source, and the substituted value
/// may contain any of the same characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ArrayIndexSyntax {
    /// Tcl 9.x (and the unversioned default): reject the complete
    /// `TYPE_BAD_ARRAY_INDEX` source mask apart from `)`, which remains the
    /// successful index terminator.
    #[default]
    Tcl9,
    /// Tcl 8.4–8.6 and runtimes based on them: every raw byte apart from the
    /// closing `)` may be index text.
    Tcl8,
}

impl ArrayIndexSyntax {
    /// Whether a raw byte at array-index token level is invalid.
    ///
    /// Callers must first skip escapes and complete variable/command
    /// substitutions. This method owns the release-varying byte set so the
    /// main lexer, expression lexer, and runtimes cannot drift.
    #[must_use]
    pub const fn rejects_literal(self, byte: u8) -> bool {
        matches!(self, Self::Tcl9) && matches!(byte, b'(' | b'"' | b'{' | b'}')
    }
}

/// The brace-line continuation axis — the F5 N-rules
/// (`docs/design/f5/embedded-tcl-contract.md` §2), a second
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
    /// R-rules (`docs/design/f5/embedded-tcl-contract.md` §1, §3):
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
    /// Which literal source bytes an array-index read accepts — see
    /// [`ArrayIndexSyntax`].
    pub array_index: ArrayIndexSyntax,
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
    /// Which characters end a command word — see [`WordSeparators`].
    pub word_separators: WordSeparators,
    /// What a backslash-newline pair inside a `{braced}` word becomes — see
    /// [`BraceBackslashNewline`]. Distinct from
    /// [`Self::brace_line_continuation`], which is the F5 fork's
    /// next-line-`{` rule: this one is about a word's *value*, that one
    /// about where a command ends.
    pub brace_backslash_newline: BraceBackslashNewline,
    /// Whether text may follow a word's closing `"` — see
    /// [`QuoteTermination`].
    pub quote_termination: QuoteTermination,
    /// How a `$` substitution's name, index and sugar forms are parsed —
    /// see [`VarSyntax`].
    pub var_syntax: VarSyntax,
    /// Whether malformed list text is an error — see [`ListParse`].
    pub list_parse: ListParse,
    /// How the dialect's `source` reads a script file's line endings — see
    /// [`SourceLineEndings`]. Like [`Self::script_skips_leading_bom`], this
    /// describes the file reader, so it applies only to Document input
    /// (`tcl_lexer::SourceChannel::Document`), never to a native value.
    pub source_line_endings: SourceLineEndings,
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
        grammar_of_dialect_name(name).escapes
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

    /// Whether `\u{…}` — a brace-delimited numeric UTF8 unit — is a form
    /// this grammar decodes. Jim accepts one through six hex digits up to
    /// `0x1fffff`; its byte encoder also retains surrogate units.
    ///
    /// `JimTcl` alone accepts it (`\u{1F600}` is one character on jimsh 0.76
    /// and 0.84); no build of the Tcl core does, and tclsh 8.6 / 9.0 both read
    /// the same text as a literal `u` followed by `{1F600}`. Spelled as an
    /// exhaustive match so a new grammar has to state its answer rather than
    /// inherit Jim's.
    #[must_use]
    pub fn has_braced_unicode(self) -> bool {
        match self {
            Self::Tcl84 | Self::Tcl86 | Self::Tcl90 => false,
            Self::Jim => true,
        }
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
    pub const ALL: &'static [Self] = &[
        Self::Tcl84,
        Self::Tcl85,
        Self::Tcl90,
        Self::Jim,
        Self::Jim080,
    ];

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
        grammar_of_dialect_name(name).numbers
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

    /// Whether `string is double` refuses a decimal spelling that overflows or
    /// underflows the double range (`1e400`, `1e-400`, `4.9e-324`).
    ///
    /// Tcl 8.4 reads a double with C `strtod` and treats `ERANGE` as "not a
    /// double"; `JimTcl` does the same in its `string is double`. From 8.5
    /// `TclParseNumber` takes the value `strtod` would have set `ERANGE` for
    /// (`Inf`, `0.0`, a subnormal), so those spellings are doubles. A
    /// spelling whose mantissa is zero (`0e999`) never underflows, and the
    /// alphabetic `Inf`/`NaN` forms are not range errors in any release.
    #[must_use]
    pub fn string_is_double_rejects_range_error(self) -> bool {
        match self {
            Self::Tcl84 | Self::Jim | Self::Jim080 => true,
            Self::Tcl85 | Self::Tcl90 => false,
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

    /// How this release spells a double it computed, while the script leaves
    /// the precision alone.
    ///
    /// Tcl 8.4's `Tcl_PrintDouble` (`tclUtil.c`) is `sprintf` under
    /// `"%.<tcl_precision>g"`, and `tcl_precision` starts at 12; Jim's
    /// `UpdateStringOfDouble` (`jim.c`) is a fixed `"%.12g"`. 8.5 onward print
    /// the shortest digits through `TclDoubleDigits` (`tclStrToD.c`).
    /// Measured, tclsh 8.4.20 and jimsh 0.84 alike: `expr {1.0/3}` is
    /// `0.333333333333`, `expr {1e15 + 0.3}` is `1e+15`, `expr {1e-5}` is
    /// `1e-05`; tclsh 8.5.19 to 9.1.0 print `0.3333333333333333`,
    /// `1000000000000000.2` and `1e-5`.
    #[must_use]
    pub fn double_spelling(self) -> DoubleSpelling {
        match self {
            Self::Tcl84 | Self::Jim | Self::Jim080 => DoubleSpelling::Precision12,
            Self::Tcl85 | Self::Tcl90 => DoubleSpelling::Shortest,
        }
    }

    /// The variable whose value changes [`Self::double_spelling`] at run
    /// time, where the release has one.
    ///
    /// `tcl_precision` through 8.6, linked by a write trace
    /// (`TclPrecTraceProc`, `tclUtil.c`; 8.4 accepts 1 to 17, 8.5 and 8.6
    /// 0 to 17). Tcl 9.0 removed the link (9.0.4's `generic/` never names
    /// it), and Jim never had one. Measured: after `set tcl_precision 6`,
    /// `expr {$a/3}` with `a` 1.0 is `0.333333` on tclsh 8.4.20, 8.5.19 and
    /// 8.6.18, and `0.3333333333333333` on 9.0.4 and 9.1.0; jimsh 0.84 prints
    /// `0.333333333333` whatever the variable holds.
    #[must_use]
    pub fn precision_variable(self) -> Option<&'static str> {
        match self {
            Self::Tcl84 | Self::Tcl85 => Some("tcl_precision"),
            Self::Tcl90 | Self::Jim | Self::Jim080 => None,
        }
    }

    /// Whether an integer past 64 bits wraps modulo 2^64 as long as its
    /// magnitude fits `u64`, as 8.4's `strtoull`-based reads do on an LP64
    /// host, rather than becoming a bignum. Measured: `expr
    /// {18446744073709551615 + 0}` is -1 and `incr x` from
    /// 9223372036854775807 gives -9223372036854775808 on tclsh 8.4.20 (64-bit).
    /// The width is the host's, so only a running interpreter applies it.
    #[must_use]
    pub fn wraps_wide_integers(self) -> bool {
        matches!(self, Self::Tcl84)
    }

    /// How `Tcl_PrintDouble` lays a double out while the precision is
    /// `precision`.
    ///
    /// 8.4 formats with `sprintf("%.<precision>g")`; 8.5 onward take the
    /// shortest digits at precision 0 and otherwise `precision` correctly
    /// rounded significant digits from `TclDoubleDigits`, laid out with the
    /// shortest form's cutover (`tclUtil.c`). Jim is always `%.12g`.
    /// Measured: at precision 17 `expr {1e-7}` is `9.9999999999999995e-08`
    /// on tclsh 8.4.20 to 8.6.18; at 12 `expr {1e15}` is `1e+15` on 8.4.20
    /// and `1000000000000000.0` on 8.5.19 and 8.6.18.
    ///
    /// The answer is the shared [`crate::DoubleFormat`] the native lazy
    /// double-string owner selects (`crate::DoubleStringPolicy::format`):
    /// `General` is the `%g` layout, `TclSignificant` 8.5/8.6's.
    #[must_use]
    pub fn double_format(self, precision: u8) -> crate::DoubleFormat {
        match self {
            Self::Tcl84 => crate::DoubleFormat::General(precision),
            Self::Jim | Self::Jim080 => crate::DoubleFormat::General(12),
            Self::Tcl85 | Self::Tcl90 if precision == 0 => crate::DoubleFormat::Shortest,
            Self::Tcl85 | Self::Tcl90 => crate::DoubleFormat::TclSignificant(precision),
        }
    }

    /// The precision a fresh interpreter prints doubles at: 12 on 8.4 and
    /// Jim, 0 (shortest) from 8.5.
    #[must_use]
    pub fn default_precision(self) -> u8 {
        match self {
            Self::Tcl84 | Self::Jim | Self::Jim080 => 12,
            Self::Tcl85 | Self::Tcl90 => 0,
        }
    }

    /// The values [`Self::precision_variable`] accepts. `TclPrecTraceProc`
    /// (`tclUtil.c`) rejects anything outside 1 to 17 on 8.4 and 0 to 17
    /// from 8.5 with `improper value for precision`.
    #[must_use]
    pub fn precision_range(self) -> core::ops::RangeInclusive<u8> {
        match self {
            Self::Tcl84 | Self::Jim | Self::Jim080 => 1..=17,
            Self::Tcl85 | Self::Tcl90 => 0..=17,
        }
    }

    /// Whether [`Self::precision_variable`] is read with `strtoul` in base
    /// 10, as 8.4's `TclPrecTraceProc` reads it, rather than as a Tcl
    /// integer (`Tcl_GetIntFromObj`, 8.5 and 8.6). Measured: `set
    /// tcl_precision 0x10` is refused by tclsh 8.4.20 and sets 16 on 8.5.19.
    #[must_use]
    pub fn precision_reads_decimal(self) -> bool {
        matches!(self, Self::Tcl84 | Self::Jim | Self::Jim080)
    }

    /// Whether reading a decimal spelling outside the double range as a
    /// number fails, as Tcl 8.4's `Tcl_GetDouble` does on `strtod`'s
    /// `ERANGE` — unlike [`Self::string_is_double_rejects_range_error`], in
    /// every operand position, not only `string is double`.
    ///
    /// Measured: `set x 1e-310; expr {$x * 1}` raises `can't use
    /// floating-point value as operand of "*"` on tclsh 8.4.20 and is
    /// `1e-310` on 8.5.19 to 9.1.0 and on jimsh 0.84, though 8.4 computes
    /// and prints that same subnormal (`expr {1e-300 * 1e-10}` is `1e-310`).
    #[must_use]
    pub fn double_read_rejects_range_error(self) -> bool {
        matches!(self, Self::Tcl84)
    }

    /// Whether the `pow()` math function of two integers is an integer power
    /// rather than a double: Jim implements `pow` as its `**` operator
    /// (`JimExprOpBin` in `jim.c`), while every Tcl release calls C `pow`.
    /// Measured: `expr {pow(2,3)}` is `8` and `expr {pow(2,-1)}` is `0` on
    /// jimsh 0.84, `8.0` and `0.5` on tclsh 8.4.20 to 9.1.0.
    #[must_use]
    pub fn pow_of_integers_is_integer(self) -> bool {
        matches!(self, Self::Jim | Self::Jim080)
    }
}

/// The spelling a release gives a double it computed — its string rep —
/// while the script leaves the precision alone
/// ([`NumberSyntax::double_spelling`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DoubleSpelling {
    /// The shortest digits that re-read as the double, laid out `%g`-style
    /// with a `.0` on an integral value: `Tcl_PrintDouble` at `tcl_precision`
    /// 0, the 8.5+ default. Tcl's digit loop misprints a power of two
    /// (`tcl_syntax::number::format_double`).
    Shortest,
    /// `%.12g`, with `.0` appended when that has neither a point nor an
    /// exponent: Tcl 8.4 at its default `tcl_precision` of 12, and Jim.
    Precision12,
}

/// How a target spells the numbers it computes, read back under the grammar
/// it parses them with: the release facts a constant folder needs before it
/// writes a computed value as source text.
///
/// A folded literal is re-read by the program, so it is sound only when it
/// is the target's own spelling of the value **and** that spelling reads
/// back as the value — `tcl_syntax::number::spell_double` and `spell_wide`
/// answer both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NumberSpelling {
    numbers: NumberSyntax,
    doubles: Option<DoubleSpelling>,
    garbles_wide_min: bool,
}

impl Default for NumberSpelling {
    fn default() -> Self {
        Self::of(NumberSyntax::default(), None)
    }
}

impl NumberSpelling {
    /// The spelling of a target reading numerals under `numbers`, running
    /// `release` when it names one.
    ///
    /// With no release named, a computed `INT64_MIN` is taken to be garbled
    /// wherever the grammar leaves 8.4 or 8.5 possible
    /// ([`crate::TclVersion::garbles_wide_min`]), so a consumer declines
    /// rather than guess.
    #[must_use]
    pub fn of(numbers: NumberSyntax, release: Option<crate::TclVersion>) -> Self {
        let garbles_wide_min = match numbers {
            NumberSyntax::Tcl90 | NumberSyntax::Jim | NumberSyntax::Jim080 => false,
            NumberSyntax::Tcl84 | NumberSyntax::Tcl85 => {
                release.is_none_or(crate::TclVersion::garbles_wide_min)
            }
        };
        Self {
            numbers,
            doubles: Some(numbers.double_spelling()),
            garbles_wide_min,
        }
    }

    /// The spelling of `profile`'s runtime, or of the 9.x default when no
    /// profile is loaded.
    #[must_use]
    pub fn of_profile(profile: Option<&crate::DialectProfile>) -> Self {
        Self::of(
            NumberSyntax::of_profile(profile),
            profile.and_then(crate::DialectProfile::runtime_version),
        )
    }

    /// `self` for a program that may set [`NumberSyntax::precision_variable`]:
    /// a computed double then has no spelling known at compile time. A
    /// release with no such variable is unaffected.
    #[must_use]
    pub fn with_precision_written(self, written: bool) -> Self {
        if written && self.numbers.precision_variable().is_some() {
            Self {
                doubles: None,
                ..self
            }
        } else {
            self
        }
    }

    /// The grammar a spelled number is read back under.
    #[must_use]
    pub const fn numbers(self) -> NumberSyntax {
        self.numbers
    }

    /// How a computed double is spelled, or `None` when the program may
    /// have changed the precision.
    #[must_use]
    pub const fn doubles(self) -> Option<DoubleSpelling> {
        self.doubles
    }

    /// Whether a computed `INT64_MIN` is spelled `-8` rather than its digits
    /// ([`crate::TclVersion::garbles_wide_min`]).
    #[must_use]
    pub const fn garbles_wide_min(self) -> bool {
        self.garbles_wide_min
    }
}

/// Which characters separate one command word from the next.
///
/// Every Tcl release treats space, tab, carriage return, vertical tab and
/// form feed as horizontal whitespace inside a command
/// (`TclParseWhiteSpace`'s `TYPE_SPACE` class in `generic/tclParse.c`, whose
/// `charTypeTable` marks all five). `JimTcl`'s script parser spells the set
/// out as a `switch` and simply has no `case '\v'` (`jim.c:1338-1341` —
/// `' '`, `'\t'`, `'\r'`, `'\f'`), so a vertical tab is an ordinary word
/// character there.
///
/// Measured: `eval "f a\vb"` passes **one** argument under every modelled Jim
/// release and **two** under tclsh 8.6.
///
/// This axis governs *command* parsing only. Jim's **list** parser is a
/// separate routine that tests `isspace()` (`JimParseList`, `jim.c:2121`),
/// and `isspace` does include `\v` — so `llength "a\vb"` is 2 in Jim even
/// though the same bytes are one word in a command.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum WordSeparators {
    /// ` `, `\t`, `\r`, `\v`, `\f` — every build of the Tcl core.
    #[default]
    Tcl,
    /// ` `, `\t`, `\r`, `\f` — `JimTcl`. `\v` is a word character.
    Jim,
}

impl WordSeparators {
    /// Whether `byte` separates two words in a command under this grammar.
    ///
    /// Newline and `;` are *terminators*, not separators, and are not
    /// reported here by either variant.
    #[must_use]
    pub const fn is_separator(self, byte: u8) -> bool {
        match byte {
            b' ' | b'\t' | b'\r' | 0x0C => true,
            // The one byte the two variants disagree on.
            0x0B => matches!(self, Self::Tcl),
            _ => false,
        }
    }
}

/// What a backslash-newline pair inside a `{braced}` word becomes.
///
/// A braced word is otherwise substitution-free in both implementations;
/// this is the single exception, and they resolve it opposite ways.
///
/// C Tcl folds the pair — `TclCopyAndCollapse` rewrites backslash-newline
/// plus any following leading whitespace to one space, so `{a\<newline>b}` is
/// the three-character string `a b`. `JimTcl` deliberately does not:
/// `JimParseSubBrace` (`jim.c:1444-1485`) advances past the escaped
/// character and bumps `linenr`, but the token keeps its original bytes, so
/// the same word is the four-character `a\<newline>b` — upstream keeps it
/// that way to preserve line numbers through a braced body.
///
/// Measured: `string length {a\<newline>b}` is 3 under tclsh 8.6 and 9.0, and
/// 4 under every modelled Jim release.
///
/// Both variants agree on *boundaries* — a backslash always hides the next
/// character from the brace counter, so `{a\}b}` is one word either way.
/// This axis moves only the resulting **value** and its length, which is what
/// separates it from [`BraceLineContinuation`]: that one decides where a
/// command ends, this one what a word contains.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum BraceBackslashNewline {
    /// Collapse backslash-newline (and the whitespace run after it) to a
    /// single space — every build of the Tcl core.
    #[default]
    Folds,
    /// Keep every byte, so the word spans lines verbatim — `JimTcl`.
    Literal,
}

impl BraceBackslashNewline {
    /// Whether the pair collapses to a space.
    #[must_use]
    pub const fn folds(self) -> bool {
        matches!(self, Self::Folds)
    }
}

/// Whether ordinary text may follow the `"` that closes a quoted word.
///
/// C Tcl requires a separator after the closing quote: `ParseTokens` checks
/// the next character and raises `extra characters after close-quote`
/// otherwise, so `puts "abc"def` is a parse error. `JimTcl` has no such
/// check anywhere — `JimParseStr` simply clears `inquote` and keeps
/// scanning, so the quoted and unquoted runs concatenate into one word and
/// `puts "abc"def` prints `abcdef`.
///
/// The mirror-image rule for braces (`extra characters after close-brace`)
/// is **not** on this axis: Jim rejects `{abc}def` the same way C Tcl does,
/// so that diagnostic stays unconditional.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum QuoteTermination {
    /// A close-quote must be followed by a separator — every build of the
    /// Tcl core.
    #[default]
    Strict,
    /// Text after a close-quote concatenates onto the same word — `JimTcl`.
    Concatenating,
}

impl QuoteTermination {
    /// Whether a close-quote must be followed by a separator.
    #[must_use]
    pub const fn is_strict(self) -> bool {
        matches!(self, Self::Strict)
    }
}

/// How a `$` substitution's name, index and sugar forms are parsed.
///
/// `JimTcl`'s `JimParseVar` (`jim.c:1641-1746`) diverges from
/// `TclParseVarName` on three points at once, and all three are present in
/// every modelled release (0.76-0.84), so they ride one axis:
///
/// * **`$(...)` is expression substitution.** With an empty variable name,
///   the parenthesised text is an `expr` body, not a dict index —
///   `JIM_TT_EXPRSUGAR`. `set a 5; puts $($a * 2)` prints `10`. C Tcl reads
///   `$(` as a variable named `(`, so ordinary Jim code draws false
///   diagnostics without this. (The `$[...]` spelling sits behind
///   `EXPRSUGAR_BRACKET`, which no shipped build defines — not modelled.)
/// * **Index parens nest.** The scanner counts `(` and `)` and honours a
///   backslash escape, so `$d(a(b))` indexes by the whole `a(b)`. C Tcl
///   stops at the first `)`.
/// * **A name may contain any byte >= 0x80.** Jim's name loop accepts
///   `isalnum`, `_`, and every non-ASCII byte — its comment reads "we
///   consider all unicode points outside of ASCII as letters" — so `$café`
///   is one variable where C Tcl names `caf`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum VarSyntax {
    /// `TclParseVarName`: `$(` starts an empty-name array read; an index ends at the
    /// first `)`, and a bare name is ASCII alphanumerics plus `_`.
    #[default]
    Tcl,
    /// `JimParseVar`: `$(...)` is expression sugar, index parens nest, and a
    /// name may contain any byte >= 0x80.
    Jim,
}

impl VarSyntax {
    /// Whether `$(...)` at a word start is an expression substitution.
    #[must_use]
    pub const fn has_expr_sugar(self) -> bool {
        matches!(self, Self::Jim)
    }

    /// Whether a `$name(index)` index counts nested parens.
    #[must_use]
    pub const fn index_parens_nest(self) -> bool {
        matches!(self, Self::Jim)
    }

    /// Whether a bare `$name` may contain bytes >= 0x80.
    #[must_use]
    pub const fn name_allows_high_bytes(self) -> bool {
        matches!(self, Self::Jim)
    }
}

/// Whether text that cannot be split as a list is an error.
///
/// C Tcl's `TclFindElement` reports `unmatched open brace in list`,
/// `unmatched open quote in list` and `list element in braces followed by
/// "x" instead of space`, so `llength "a {b"` fails. `JimTcl`'s list parser
/// has no error path at all: `JimParseBrace` records a `missing.ch` for the
/// script parser's benefit but `SetListFromAny` never consults it, so the
/// unterminated element is taken to end at the end of the string and
/// `llength "a {b"` is 2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ListParse {
    /// Malformed list text raises an error — every build of the Tcl core.
    #[default]
    Strict,
    /// Malformed list text never errors; an unterminated element runs to the
    /// end of the string — `JimTcl`.
    Lenient,
}

/// How a dialect's `source` command reads a script file's line endings
/// before its parser sees the text.
///
/// C Tcl opens the file with `-translation auto`, and its `TranslateInputEOL`
/// turns CR and CRLF into LF. A lone CR in a sourced file therefore ends a
/// command or a comment, exactly as LF does, while `eval` of the same bytes
/// keeps the CR as a word separator. This was measured on tclsh 8.4.20,
/// 8.5.19, 8.6.18, 9.0.4 and 9.1.0, with identical results.
/// `JimTcl`'s `Jim_EvalFile` reads the file with `fopen(…, "rt")` and
/// `fread` and translates nothing (`jim.c:12130` at 0.84). On jimsh 0.84,
/// `proc t {a b} {…}\rt x value\r` is one `proc` command ("wrong # args"),
/// and `#\r{\nputs reached\n` prints `reached`, because the comment runs
/// to the LF.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SourceLineEndings {
    /// `-translation auto`: a lone CR reads as LF. The CR of a CRLF pair
    /// stays a word separator before the pair's LF, which gives the same
    /// command structure as one translated LF. Every C Tcl release.
    #[default]
    TranslateAuto,
    /// The file's bytes reach the parser unchanged, so a lone CR stays a
    /// word separator — `JimTcl`.
    Preserve,
}

impl SourceLineEndings {
    /// Whether a lone CR in a sourced file reads as LF.
    #[must_use]
    pub const fn translates(self) -> bool {
        matches!(self, Self::TranslateAuto)
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
            array_index: ArrayIndexSyntax::Tcl9,
            script_skips_leading_bom: true,
            expr_comments: ExprCommentStyle::Hash,
            numbers: NumberSyntax::Tcl90,
            escapes: EscapeSyntax::Tcl90,
            word_separators: WordSeparators::Tcl,
            brace_backslash_newline: BraceBackslashNewline::Folds,
            quote_termination: QuoteTermination::Strict,
            var_syntax: VarSyntax::Tcl,
            list_parse: ListParse::Strict,
            source_line_endings: SourceLineEndings::TranslateAuto,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ArrayIndexSyntax, BracedVarStyle, DoubleSpelling, EscapeSyntax, ExprCommentStyle,
        LexerGrammar, NumberSpelling, NumberSyntax,
    };

    /// Each Tcl profile's number spelling, measured on its reference shell:
    /// the `%.12g` double of 8.4, the shortest one from 8.5, `tcl_precision`
    /// through 8.6, and the `-8` that 8.4 and 8.5 print for a computed
    /// `INT64_MIN`. iRules has no shell here and follows its 8.4 base.
    #[test]
    fn number_spelling_follows_the_release() {
        for (dialect, doubles, precision, garbles) in [
            ("tcl8.4", DoubleSpelling::Precision12, true, true),
            ("f5-irules", DoubleSpelling::Precision12, true, true),
            ("tcl8.5", DoubleSpelling::Shortest, true, true),
            ("tcl8.6", DoubleSpelling::Shortest, true, false),
            ("tcl9.0", DoubleSpelling::Shortest, false, false),
            ("tcl9.1", DoubleSpelling::Shortest, false, false),
        ] {
            let spelling = NumberSpelling::of_profile(crate::DialectProfile::find(dialect));
            assert_eq!(spelling.doubles(), Some(doubles), "{dialect}");
            assert_eq!(spelling.garbles_wide_min(), garbles, "{dialect}");
            assert_eq!(
                spelling.with_precision_written(true).doubles().is_none(),
                precision,
                "{dialect}"
            );
        }
        for jim in [NumberSyntax::Jim, NumberSyntax::Jim080] {
            let spelling = NumberSpelling::of(jim, None);
            assert_eq!(spelling.doubles(), Some(DoubleSpelling::Precision12));
            assert!(!spelling.garbles_wide_min());
            assert_eq!(jim.precision_variable(), None);
        }
        // With no release named, 8.4 or 8.5 may be running.
        assert!(NumberSpelling::of(NumberSyntax::Tcl85, None).garbles_wide_min());
        assert!(!NumberSpelling::default().garbles_wide_min());
        assert!(NumberSyntax::Tcl84.double_read_rejects_range_error());
        assert!(!NumberSyntax::Jim080.double_read_rejects_range_error());
    }

    #[test]
    fn nesting_rule_is_tcl9_only() {
        assert!(BracedVarStyle::Tcl9Nesting.nests());
        assert!(!BracedVarStyle::FirstClose.nests());
        assert_eq!(BracedVarStyle::default(), BracedVarStyle::Tcl9Nesting);
    }

    #[test]
    fn array_index_source_mask_is_tcl9_only() {
        for byte in *b"(\"{}" {
            assert!(ArrayIndexSyntax::Tcl9.rejects_literal(byte));
            assert!(!ArrayIndexSyntax::Tcl8.rejects_literal(byte));
        }
        for byte in *b"a[$\\" {
            assert!(!ArrayIndexSyntax::Tcl9.rejects_literal(byte));
            assert!(!ArrayIndexSyntax::Tcl8.rejects_literal(byte));
        }
        assert_eq!(ArrayIndexSyntax::default(), ArrayIndexSyntax::Tcl9);
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
    fn only_strtod_readers_reject_a_double_range_error() {
        for (syntax, rejects) in [
            (NumberSyntax::Tcl84, true),
            (NumberSyntax::Tcl85, false),
            (NumberSyntax::Tcl90, false),
            (NumberSyntax::Jim, true),
            (NumberSyntax::Jim080, true),
        ] {
            assert_eq!(syntax.string_is_double_rejects_range_error(), rejects);
        }
        assert_eq!(
            NumberSyntax::ALL.len(),
            5,
            "a new grammar needs a row above"
        );
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

/// The single name→grammar resolution, and the agreement it buys.
#[cfg(test)]
mod grammar_of_dialect_name_tests {
    use super::*;
    use crate::model::DialectPoint;

    /// Wherever both catalogues know a name they must answer the same
    /// grammar. This is the property that lets the resolution prefer the
    /// environment model without changing any existing dialect's behaviour —
    /// and it fails loudly if the two ever drift apart.
    #[test]
    fn the_two_catalogues_agree_wherever_both_know_a_name() {
        // `tk` is a catalogue profile that `all()` deliberately excludes, and
        // it is exactly the row that drifted: 9.x here, 8.6 in the
        // environment model, until the two were made to agree.
        let rows = crate::DialectProfile::all()
            .iter()
            .chain(std::iter::once(crate::DialectProfile::tk()));
        for profile in rows {
            let Some(point) = DialectPoint::of_dialect_name(Some(profile.name)) else {
                continue;
            };
            assert_eq!(
                point.grammar(),
                profile.grammar,
                "`{}` resolves to a different grammar in the two catalogues",
                profile.name
            );
        }
    }

    /// An environment with no catalogue profile still gets its own grammar —
    /// the hole this resolution closes. Before it, every `of_dialect_name`
    /// answered with C Tcl's grammar for a Jim document, so codegen compiled
    /// one with 9.0 numerals and escapes.
    #[test]
    fn an_environment_without_a_profile_still_resolves() {
        assert!(
            crate::DialectProfile::find("jim").is_none(),
            "jim is not a catalogue profile"
        );
        let jim = grammar_of_dialect_name(Some("jim"));
        // The default release is 0.84, which sits on the `Jim080` rung —
        // `0d` became available at 0.80.
        assert_eq!(jim.numbers, NumberSyntax::Jim080);
        assert_eq!(jim.escapes, EscapeSyntax::Jim);
        assert_eq!(jim.word_separators, WordSeparators::Jim);
        assert_eq!(jim.list_parse, ListParse::Lenient);
        assert_eq!(jim.brace_backslash_newline, BraceBackslashNewline::Literal);
        assert_eq!(jim.source_line_endings, SourceLineEndings::Preserve);
        assert_ne!(
            jim,
            LexerGrammar::default(),
            "a Jim document is not compiled under the 9.0 grammar"
        );
        for alias in ["jimsh", "jimtcl"] {
            assert_eq!(grammar_of_dialect_name(Some(alias)), jim, "{alias}");
        }
    }

    /// The one shipped dialect with no Tcl ladder still answers from the
    /// legacy catalogue rather than falling to the default.
    #[test]
    fn the_config_dialect_still_answers() {
        assert!(DialectPoint::of_dialect_name(Some("f5-bigip")).is_none());
        let profile = crate::DialectProfile::find("f5-bigip").expect("shipped");
        assert_eq!(grammar_of_dialect_name(Some("f5-bigip")), profile.grammar);
    }

    /// A name neither catalogue knows, and no name at all, are C Tcl.
    #[test]
    fn an_unknown_name_is_the_default() {
        assert_eq!(grammar_of_dialect_name(None), LexerGrammar::default());
        assert_eq!(
            grammar_of_dialect_name(Some("nonsense")),
            LexerGrammar::default()
        );
    }

    /// The per-axis constructors must not disagree with the central one for
    /// any name in either catalogue — the regression this exists to prevent.
    #[test]
    fn every_axis_constructor_agrees_with_the_central_one() {
        let mut names: Vec<String> = crate::DialectProfile::all()
            .iter()
            .map(|p| p.name.to_string())
            .collect();
        names.extend(["jim".to_string(), "tk".to_string()]);
        for name in names {
            let central = grammar_of_dialect_name(Some(&name));
            assert_eq!(
                NumberSyntax::of_dialect_name(Some(&name)),
                central.numbers,
                "numbers disagree for `{name}`"
            );
            assert_eq!(
                EscapeSyntax::of_dialect_name(Some(&name)),
                central.escapes,
                "escapes disagree for `{name}`"
            );
            assert_eq!(
                BracedVarStyle::of_dialect_name(Some(&name)),
                central.braced_var,
                "braced_var disagrees for `{name}`"
            );
        }
    }
}
