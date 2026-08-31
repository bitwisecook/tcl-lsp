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

//! Expression sub-lexer for Tcl `[expr]` bodies.
//!
//! A flat single-pass tokeniser
//! for the infix expression sub-language. Unlike the main `Lexer`,
//! the expression lexer does not use `Span` + `SourceMap`; it
//! produces simple `ExprToken` values with inline start/end offsets
//! because expression bodies are always short strings extracted from
//! a parent token, not full source documents.

use std::collections::HashSet;

use tcl_core_types::RecursionLimit;

use crate::{backslash_continuation_end, close_quote_offset, command_substitution_end};

/// Cap on `$name(…)` array-index nesting depth for `Inner::scan_array_index`
/// — mirrors the main lexer's `MAX_ARRAY_INDEX_DEPTH` (`crate::lexer`) and
/// exists for the same reason: unbounded self-recursion on
/// `$a($b($c(...)))` could otherwise abort the process with an
/// uncatchable native-stack overflow on pathologically deep input. See
/// `docs/design/compiler/recursive-descent-depth-limits.md`.
const MAX_EXPR_ARRAY_INDEX_DEPTH: RecursionLimit = RecursionLimit(64);

/// Token types specific to Tcl expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExprTokenType {
    /// Integer or float literal.
    Number,
    /// `"quoted string"`.
    String,
    /// `$var`, `$ns::var`, `$arr(idx)`.
    Variable,
    /// `[cmd ...]` command substitution.
    Command,
    /// Operator (`+`, `==`, `&&`, `eq`, etc.).
    Operator,
    /// `(`.
    ParenOpen,
    /// `)`.
    ParenClose,
    /// `,` — function argument separator.
    Comma,
    /// Math function name.
    Function,
    /// Boolean literal (`true`, `false`, `yes`, `no`, `on`, `off`).
    Bool,
    /// `?` — ternary.
    TernaryQ,
    /// `:` — ternary colon.
    TernaryC,
    /// Whitespace run.
    Whitespace,
    /// `# …` comment, running to the end of the line or the end of the
    /// expression — TIP 582, Tcl 9.0+ only. Skipped exactly like
    /// [`Whitespace`](Self::Whitespace) by every consumer that cares about the
    /// grammar; emitted rather than dropped so the token stream still covers
    /// every byte of the source.
    Comment,
    /// End of input.
    Eof,
}

impl ExprTokenType {
    /// Symbolic name for the token type.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Number => "NUMBER",
            Self::String => "STRING",
            Self::Variable => "VARIABLE",
            Self::Command => "COMMAND",
            Self::Operator => "OPERATOR",
            Self::ParenOpen => "PAREN_OPEN",
            Self::ParenClose => "PAREN_CLOSE",
            Self::Comma => "COMMA",
            Self::Function => "FUNCTION",
            Self::Bool => "BOOL",
            Self::TernaryQ => "TERNARY_Q",
            Self::TernaryC => "TERNARY_C",
            Self::Whitespace => "WHITESPACE",
            Self::Comment => "COMMENT",
            Self::Eof => "EOF",
        }
    }

    /// Whether the expression grammar skips this token outright, so it never
    /// reaches a parse or a syntax check.
    ///
    /// C's `ParseExpr` advances past a `COMMENT` lexeme and `continue`s its
    /// main loop exactly as it does for a whitespace run
    /// (`tclCompExpr.c:701-704`), and re-skips whitespace on the way round, so
    /// a comment may appear anywhere whitespace may — including between a
    /// function name and its `(` (`tclCompExpr.c:743-758`). Consumers filter on
    /// this rather than on [`Whitespace`](Self::Whitespace) alone so the two
    /// can never drift apart.
    #[must_use]
    pub const fn is_skipped(self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment)
    }
}

/// A token in a Tcl expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprToken {
    /// Token kind.
    pub kind: ExprTokenType,
    /// The token's text (owned).
    pub text: std::string::String,
    /// Byte offset of the first character.
    pub start: u32,
    /// Byte offset of the last character (inclusive).
    pub end: u32,
}

#[inline]
fn p(n: usize) -> u32 {
    u32::try_from(n).expect("expression offset fits u32")
}

/// Known Tcl math functions. Exported so upstream consumers (like
/// the compiler) can check for shadowed functions. In the lexer
/// itself, any identifier not in the `Bool` or `Operator` sets
/// becomes `Function` regardless (the default fallback).
///
/// `tcl-lexer` sits below `tcl-syntax` in the dependency graph (see
/// `tcl-syntax`'s own architecture doc comment), so this can't derive from
/// `tcl_syntax::expr::mathfunc::ALL_NAMES` directly — the two lists are kept
/// in sync by `tcl-syntax`'s own drift-guard test
/// (`expr::operators::tests::tcl_lexer_recognises_every_mathfunc_name`), which
/// fails the moment this list and `mathfunc::all()` disagree. Was missing
/// TIP 521 (9.0) and TIP 745 (9.1)'s additions until that guard caught it —
/// a real gap (`tcl-lsp-core::semantic_tokens` reads this set to decide
/// which `Function`-kind expr tokens get the "known math function"
/// modifier, so `expr {gamma(2.5)}`/`expr {isfinite($x)}` were previously
/// under-classified in a 9.1-dialect document).
#[must_use]
pub fn math_functions() -> HashSet<&'static str> {
    [
        "abs",
        "acos",
        "asin",
        "atan",
        "atan2",
        "bool",
        "ceil",
        "cos",
        "cosh",
        "double",
        "entier",
        "exp",
        "floor",
        "fmod",
        "hypot",
        "int",
        "isinf",
        "isnan",
        "isqrt",
        "log",
        "log10",
        "max",
        "min",
        "pow",
        "rand",
        "round",
        "sin",
        "sinh",
        "sqrt",
        "srand",
        "tan",
        "tanh",
        "wide",
        // TIP 521 (Tcl 9.0): floating-point classification.
        "isfinite",
        "isnormal",
        "issubnormal",
        "isunordered",
        // TIP 745 (Tcl 9.1): the C99 math function batch.
        "acosh",
        "asinh",
        "atanh",
        "cbrt",
        "copysign",
        "dim",
        "erf",
        "erfc",
        "exp2",
        "expm1",
        "fma",
        "gamma",
        "ldexp",
        "lgamma",
        "log1p",
        "log2",
        "logb",
        "nextafter",
        "remainder",
        "signbit",
        "trunc",
    ]
    .into_iter()
    .collect()
}

const MULTI_OPS: &[&str] = &[
    "**", "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "ne", "eq", "in", "ni", "lt", "le", "gt",
    "ge",
];

fn is_single_op(ch: u8) -> bool {
    matches!(
        ch,
        b'+' | b'-' | b'*' | b'/' | b'%' | b'<' | b'>' | b'&' | b'|' | b'^' | b'~' | b'!'
    )
}

/// Tokenise a Tcl expression string from a compatibility name boundary.
///
/// Internal typed callers should use [`tokenise_expr_for_profile`].
#[must_use]
pub fn tokenise_expr(source: &str, dialect: Option<&str>) -> Vec<ExprToken> {
    let mut lex = Inner::new(
        source,
        dialect
            .and_then(tcl_dialect::DialectProfile::find)
            .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl),
    );
    lex.run()
}

/// Tokenise under an already-resolved dialect profile.
#[must_use]
pub fn tokenise_expr_for_profile(
    source: &str,
    profile: &tcl_dialect::DialectProfile,
) -> Vec<ExprToken> {
    let mut lex = Inner::new(source, profile);
    lex.run()
}

/// Tokenise from a compatibility name boundary and report skipped characters.
///
/// Internal typed callers should use [`tokenise_expr_checked_for_profile`].
#[must_use]
pub fn tokenise_expr_checked(source: &str, dialect: Option<&str>) -> (Vec<ExprToken>, bool) {
    tokenise_expr_checked_for_profile(
        source,
        dialect
            .and_then(tcl_dialect::DialectProfile::find)
            .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl),
    )
}

/// Tokenise under an already-resolved profile and report skipped characters.
#[must_use]
pub fn tokenise_expr_checked_for_profile(
    source: &str,
    profile: &tcl_dialect::DialectProfile,
) -> (Vec<ExprToken>, bool) {
    let mut lex = Inner::new(source, profile);
    let tokens = lex.run();
    (tokens, lex.unknown)
}

// Dialect-derived grammar knobs plus one output flag, not a state machine.
#[allow(clippy::struct_excessive_bools)]
struct Inner<'s> {
    b: &'s [u8],
    s: &'s str,
    i: usize,
    /// The F5-family core `expr` grammar when the dialect's runtime core is
    /// on the F5 tree, else `None` — resolved once through the profile
    /// catalog (so the `irules` / `tcl-irule` alias spellings behave like
    /// the canonical name). Gates the word-form operators
    /// (`starts_with`, `and`, …), which are an `f5-tcl` **trunk** fact —
    /// measured valid in tmsh and iApp `expr` too, not iRules-only
    /// (`docs/design/bigip-irule-parser-measurements.md` §4a) — so the
    /// acceptance reads `tcl_dialect`'s `ExprGrammar` word table directly
    /// instead of a lexer-local iRules list.
    f5_word_grammar: Option<&'static tcl_dialect::model::ExprGrammar>,
    /// Whether `#` starts a comment in this dialect — TIP 582, resolved
    /// through the profile's `LexerGrammar` (9.0+ only; see
    /// `tcl_dialect::LexerGrammar::expr_comments`). When false, `#` stays an
    /// unknown character, which is what C 8.4-8.6 does with it.
    expr_comments: bool,
    /// The release's number grammar, forwarded intact to the shared
    /// expression-numeral boundary scanner.
    numbers: tcl_dialect::NumberSyntax,
    /// The release's `${…}` close rule, forwarded intact to the shared owner
    /// [`crate::ranges::braced_var_name_end`]. `expr` bodies are ordinary Tcl
    /// words, so `expr {${a{b}c} + 1}` must resolve the reference exactly as
    /// `if {${a{b}c} > 3}` does; re-deriving it here with a first-`}` scan made
    /// the VM answer one expression grammar two ways depending on the carrying
    /// command (issue #1601).
    braced_var: tcl_dialect::BracedVarStyle,
    /// The release whose `expr` lexeme table this dialect uses, for the
    /// word-shaped operators (`eq`, `in`, `lt`, …) — the profile's
    /// `expr_grammar_base`, resolved through
    /// [`tcl_dialect::is_expr_word_operator`] rather than a list held here,
    /// because *which* of them exist moves the lexeme boundary and so cannot
    /// be settled above the lexer. `None` — the lenient `tcl` sink, the `tk`
    /// ingress profile, and `f5-bigip` — takes the newest grammar, like every
    /// other knob on `LexerGrammar`. Every catalogue dialect names a base:
    /// `f5-irules`, `f5-tmsh` and `f5-iapps` are all `Some(V8_4)`, the
    /// measured F5 fork point.
    expr_grammar_base: Option<tcl_dialect::TclVersion>,
    /// The previous emitted lexeme was a number with no intervening byte.
    /// C's successful explicit-radix path recursively starts a fresh lexeme at
    /// this point, so a word operator then has only a right-side boundary.
    numeric_suffix_probe: bool,
    unknown: bool,
}

impl<'s> Inner<'s> {
    fn new(s: &'s str, profile: &tcl_dialect::DialectProfile) -> Self {
        Self {
            b: s.as_bytes(),
            s,
            i: 0,
            f5_word_grammar: profile.f5_core_expr_grammar(),
            expr_comments: profile.grammar.expr_comments.comments(),
            numbers: profile.grammar.numbers,
            braced_var: profile.grammar.braced_var,
            expr_grammar_base: profile.expr_grammar_base,
            numeric_suffix_probe: false,
            unknown: false,
        }
    }

    fn tok(&self, kind: ExprTokenType, start: usize) -> ExprToken {
        ExprToken {
            kind,
            text: self.s[start..self.i].to_owned(),
            start: p(start),
            end: if self.i > start {
                p(self.i - 1)
            } else {
                p(start)
            },
        }
    }

    fn single(&mut self, kind: ExprTokenType, text: &str) -> ExprToken {
        let start = self.i;
        self.i += 1;
        ExprToken {
            kind,
            text: text.to_owned(),
            start: p(start),
            end: p(start),
        }
    }

    /// True when a backslash-newline line continuation (`\<LF>`)
    /// starts at byte `i`.  Tcl 9 collapses it to a single space, so
    /// the expr lexer treats it as whitespace — otherwise a
    /// `\`-continued multi-line braced condition (common in tcltest:
    /// `if {… \<NL><tabs> || …}`) would fall through, set `unknown`,
    /// and degrade the whole expression to `ExprNode::Raw`.  The
    /// whitespace scan is LF only — a `\`
    /// before `\r\n` is not a continuation.
    fn backslash_nl_end(&self, i: usize) -> Option<usize> {
        backslash_continuation_end(self.b, i)
    }

    fn run(&mut self) -> Vec<ExprToken> {
        let mut out = Vec::new();
        while self.i < self.b.len() {
            let follows_numeric_lexeme = self.numeric_suffix_probe;
            self.numeric_suffix_probe = false;
            let ch = self.b[self.i];
            if matches!(ch, b' ' | b'\t' | b'\n' | b'\r') || self.backslash_nl_end(self.i).is_some()
            {
                let start = self.i;
                while self.i < self.b.len() {
                    if matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
                        self.i += 1;
                    } else if let Some(end) = self.backslash_nl_end(self.i) {
                        self.i = end;
                    } else {
                        break;
                    }
                }
                out.push(self.tok(ExprTokenType::Whitespace, start));
            } else if ch == b'#' && self.expr_comments {
                out.push(self.comment());
            } else if let Some(number) =
                tcl_dialect::scan_expr_number(self.b, self.i, self.numbers, self.expr_grammar_base)
            {
                let start = self.i;
                self.i = number.end();
                out.push(self.tok(ExprTokenType::Number, start));
                self.numeric_suffix_probe = true;
            } else if ch == b'$' {
                out.push(self.variable());
            } else if ch == b'[' {
                out.push(self.command());
            } else if ch == b'"' {
                out.push(self.quoted());
            } else if ch == b'(' {
                out.push(self.single(ExprTokenType::ParenOpen, "("));
            } else if ch == b')' {
                out.push(self.single(ExprTokenType::ParenClose, ")"));
            } else if ch == b',' {
                out.push(self.single(ExprTokenType::Comma, ","));
            } else if ch == b'?' {
                out.push(self.single(ExprTokenType::TernaryQ, "?"));
            } else if ch == b':' {
                out.push(self.single(ExprTokenType::TernaryC, ":"));
            } else if let Some(t) = self.multi_op(follows_numeric_lexeme) {
                out.push(t);
            } else if is_single_op(ch) {
                out.push(ExprToken {
                    kind: ExprTokenType::Operator,
                    text: self.s[self.i..=self.i].to_owned(),
                    start: p(self.i),
                    end: p(self.i),
                });
                self.i += 1;
            } else if ch.is_ascii_alphabetic() {
                // A bareword may *contain* `_` but never start with one, on
                // every release — `ParseLexeme`'s last gate is
                // `if (!TclIsBareword(*start) || *start == '_')`, which yields
                // INVALID, not BAREWORD (`tclCompExpr.c:2135` in 9.0.4,
                // `:2068` in 8.6.16, above the comment "We reject leading
                // underscores in bareword. No sensible reason why."). So `_x`
                // is `invalid character "_"`, not `invalid bareword "_x"`,
                // and the invalid lexeme is one character wide — the `x`
                // after it is never joined on.
                out.push(self.ident());
            } else if ch == b'{' {
                out.push(self.braced());
            } else {
                self.unknown = true;
                self.i += 1;
            }
        }
        out
    }

    /// Scan a TIP 582 `#` comment, which lasts to the end of the line or the
    /// end of the expression, whichever comes first.
    ///
    /// Mirrors `ParseLexeme`'s `case '#':` (`tclCompExpr.c:1931-1942` in
    /// 9.0.4), including its two asymmetric terminators: C's
    /// `return size - (byte == '\n')` leaves a terminating newline *outside*
    /// the comment — it belongs to the following whitespace run, which is what
    /// keeps `expr "1 #\n+ 2"` equal to 3 rather than 1 — while an embedded NUL
    /// is consumed as part of the comment.
    ///
    /// A `\` before the newline does not extend the comment: C's scan is plain
    /// byte-wise and stops at the first raw `\n`, so
    /// [`Self::is_backslash_nl`]'s continuation rule deliberately does not
    /// apply here.
    fn comment(&mut self) -> ExprToken {
        let start = self.i;
        self.i = self.b[start..]
            .iter()
            .position(|&c| c == b'\n' || c == 0)
            .map_or(self.b.len(), |k| {
                start + k + usize::from(self.b[start + k] == 0)
            });
        self.tok(ExprTokenType::Comment, start)
    }

    /// Scan a `$…` reference.
    ///
    /// The `${…}` form is delimited by the shared owner
    /// [`crate::ranges::braced_var_name_end`] under this dialect's
    /// [`tcl_dialect::BracedVarStyle`], not by a local scan. An `expr` body is
    /// parsed out of an ordinary Tcl word, so the reference boundary is the
    /// same `Tcl_ParseVarName` rule every other surface uses: at 9.x
    /// `expr {${a{b}c} + 1}` is one `Variable` naming `a{b}c` (tclsh 9.0.4:
    /// `8`), while at 8.x the name ends at the first `}` and the trailing `c}`
    /// is separate lexemes (tclsh 8.6.16: `invalid bareword "c"`). The old
    /// release-blind first-`}` walk gave the 8.x answer at every release, so
    /// the VM disagreed with its own `if` / `while` / `for` conditions on the
    /// identical expression (issue #1601).
    fn variable(&mut self) -> ExprToken {
        let start = self.i;
        self.i += 1;
        if self.i < self.b.len() && self.b[self.i] == b'{' {
            // The name starts just past the `${`.
            match crate::ranges::braced_var_name_end(self.b, self.i + 1, self.braced_var) {
                crate::ranges::BracedVarEnd::Closed(end) => self.i = end + 1,
                crate::ranges::BracedVarEnd::Unterminated => {
                    // Preserve a recovery token for editor callers, but a
                    // variable whose `${…}` closer is absent cannot participate
                    // in an executable expression.
                    self.i = self.b.len();
                    self.unknown = true;
                }
            }
        } else {
            // A bare `$name` consumes alphanumerics / `_`, and `:` only as part
            // of a `::` namespace-separator pair — a *single* colon ends the
            // name, exactly like the main lexer's `parse_var`. Accepting a lone
            // `:` made `expr {$x>0?$y:$z}` lex `$y:` as one variable, swallowing
            // the ternary separator so the whole expr degraded to Raw.
            while self.i < self.b.len() {
                let c = self.b[self.i];
                if c.is_ascii_alphanumeric() || c == b'_' {
                    self.i += 1;
                } else if c == b':' && self.i + 1 < self.b.len() && self.b[self.i + 1] == b':' {
                    self.i += 2;
                } else {
                    break;
                }
            }
            if self.i < self.b.len() && self.b[self.i] == b'(' {
                self.i += 1;
                if !self.scan_array_index(0) {
                    // As with an unterminated command or quote, let recovery
                    // retain the token while making `parse_expr` fail closed.
                    self.unknown = true;
                }
            }
        }
        self.tok(ExprTokenType::Variable, start)
    }

    /// Scan a `$name(…)` array-index body (after the opening `(`) up to and
    /// including the first top-level `)`.
    ///
    /// Mirrors the main lexer's `scan_array_index_body`: C Tcl does NOT nest
    /// parens, so the index ends at the first `)`; a literal `(` is text. A `)`
    /// stays in the index only when escaped (`\)`), inside a `[…]` command
    /// substitution, or inside a nested `${…}` / `$name(…)` reference — whose
    /// tokens are scanned so their inner `)` are not the terminator. The old
    /// paren-counting left `$a((b)` unterminated and ended `$a(x\)y)` at the
    /// escaped `)`.
    ///
    /// `depth` is the nesting level of this call (0 at the top); past
    /// [`MAX_EXPR_ARRAY_INDEX_DEPTH`] a nested `$name(` is scanned as an
    /// ordinary character rather than recursed into, so pathologically
    /// deep `$a($b($c(...)))` input degrades gracefully rather than
    /// overflowing the native stack.
    fn scan_array_index(&mut self, depth: u32) -> bool {
        let past_cap = MAX_EXPR_ARRAY_INDEX_DEPTH.exceeded(depth);
        while self.i < self.b.len() {
            match self.b[self.i] {
                b')' => {
                    self.i += 1;
                    return true;
                }
                b'\\' => self.i = (self.i + 2).min(self.b.len()),
                b'[' => match command_substitution_end(self.s, self.i) {
                    Some(end) => self.i = end,
                    // A `]` in a nested braced/quoted script word cannot
                    // close the substitution. Delegate that grammar to the
                    // shared range owner rather than approximating it here.
                    None => return false,
                },
                b'$' if !past_cap => {
                    self.i += 1;
                    if self.i < self.b.len() && self.b[self.i] == b'{' {
                        self.i += 1;
                        while self.i < self.b.len() && self.b[self.i] != b'}' {
                            self.i += 1;
                        }
                        if self.i < self.b.len() {
                            self.i += 1;
                        }
                    } else {
                        while self.i < self.b.len() {
                            let c = self.b[self.i];
                            if c.is_ascii_alphanumeric() || c == b'_' {
                                self.i += 1;
                            } else if c == b':'
                                && self.i + 1 < self.b.len()
                                && self.b[self.i + 1] == b':'
                            {
                                self.i += 2;
                            } else {
                                break;
                            }
                        }
                        if self.i < self.b.len() && self.b[self.i] == b'(' {
                            self.i += 1;
                            if !self.scan_array_index(depth + 1) {
                                return false;
                            }
                        }
                    }
                }
                _ => self.i += 1,
            }
        }
        false
    }

    fn command(&mut self) -> ExprToken {
        let start = self.i;
        if let Some(end) = command_substitution_end(self.s, start) {
            self.i = end;
        } else {
            // Keep a recovery token for callers that need the incomplete
            // source slice, but make `tokenise_expr_checked`/`parse_expr`
            // reject it. A missing `]` cannot execute a command.
            self.i = self.b.len();
            self.unknown = true;
        }
        self.tok(ExprTokenType::Command, start)
    }

    fn quoted(&mut self) -> ExprToken {
        let start = self.i;
        if let Some(close) = close_quote_offset(self.s, start) {
            self.i = close + 1;
        } else {
            // As with an unterminated command substitution, retain the
            // recovery token but reject the expression as non-executable.
            self.i = self.b.len();
            self.unknown = true;
        }
        self.tok(ExprTokenType::String, start)
    }

    fn ident(&mut self) -> ExprToken {
        let start = self.i;
        while self.i < self.b.len()
            && (self.b[self.i].is_ascii_alphanumeric() || self.b[self.i] == b'_')
        {
            self.i += 1;
        }
        let text = &self.s[start..self.i];
        // Tcl boolean literals are case-insensitive (`True`, `YES`, `Off`,
        // …) per `Tcl_GetBoolean`; compare without allocating.
        let is_bool = ["true", "false", "yes", "no", "on", "off"]
            .iter()
            .any(|w| text.eq_ignore_ascii_case(w));
        let kind = if is_bool {
            ExprTokenType::Bool
        } else if self
            .f5_word_grammar
            .is_some_and(|grammar| grammar.has_word_operator(text))
        {
            // The F5 word-form operators, read straight off the family's
            // `ExprGrammar` word table (measurements §4a: a trunk fact,
            // valid in tmsh/iApp expr too). `eq`/`ne` are also rows there
            // but never reach this scan — the MULTI_OPS path consumed them.
            ExprTokenType::Operator
        } else {
            ExprTokenType::Function
        };
        ExprToken {
            kind,
            text: text.to_owned(),
            start: p(start),
            end: p(self.i - 1),
        }
    }

    fn braced(&mut self) -> ExprToken {
        let start = self.i;
        self.i += 1;
        let saved = self.i;
        let mut lvl = 1u32;
        while self.i < self.b.len() && lvl > 0 {
            match self.b[self.i] {
                b'{' => lvl += 1,
                b'}' => lvl -= 1,
                // Tcl's brace scan (`TclParseBraces`) consumes `\X` as a pair,
                // so `\{` / `\}` are literal and do *not* change the nesting
                // level: `{a\}b}` is the single braced word `a\}b`, not `{a\}`
                // + stray `b}` (issue 165). Bounds-guarded like `command` /
                // `quoted` so a trailing `\` can't push `i` past the end.
                b'\\' if self.i + 1 < self.b.len() => {
                    self.i += 1;
                }
                _ => {}
            }
            self.i += 1;
        }
        if lvl != 0 {
            self.i = saved;
            return ExprToken {
                kind: ExprTokenType::String,
                text: "{".to_owned(),
                start: p(start),
                end: p(start),
            };
        }
        self.tok(ExprTokenType::String, start)
    }

    /// Whether the main scan should emit `op` at `at` as an operator token.
    ///
    /// Deliberately *weaker* than [`tcl_dialect::expr_word_operator_boundary_ok`]: the release gate
    /// is applied only where it actually moves a boundary — when a bareword
    /// byte (a digit or `_`) follows, so the run would otherwise fuse into one
    /// bareword. tclsh 8.6 vs 9.0:
    ///
    /// | input | 8.6 | 9.0 | gate needed? |
    /// |---|---|---|---|
    /// | `1 lt_ 2` | `invalid bareword "lt_"` | `invalid character "_"` | yes |
    /// | `1 lt2 2` | `invalid bareword "lt2"` | `missing operator` | yes |
    /// | `1 lt 2`, `$a lt $b` | `invalid bareword "lt"` | `1` | **no** |
    ///
    /// In the last row the boundary is the same either way, and the release
    /// gate is settled *above* the lexer — the parser rejects a `lt` the
    /// dialect lacks and reports `invalid bareword "lt"`, which is what tclsh
    /// prints. Emitting the token there keeps that path working, and keeps it
    /// visible to the analyser: W003 exists precisely to say "this operator is
    /// Tcl 9.0+ (TIP 461)", and it can only say so about an operator it can
    /// still see. Suppressing the token here made W003 silent on the very
    /// dialects it targets.
    fn word_operator_lexeme_at(&self, op: &str, at: usize, after_numeric_lexeme: bool) -> bool {
        (if after_numeric_lexeme {
            tcl_dialect::expr_word_operator_right_boundary_ok(self.b, op, at)
        } else {
            tcl_dialect::expr_word_operator_boundary_ok(self.b, op, at)
        }) && (!self
            .b
            .get(at + op.len())
            .copied()
            .is_some_and(tcl_dialect::is_expr_bareword_byte)
            || tcl_dialect::is_expr_word_operator(op, self.expr_grammar_base))
    }

    fn multi_op(&mut self, after_numeric_lexeme: bool) -> Option<ExprToken> {
        for &op in MULTI_OPS {
            // Compare on bytes, not `self.s[self.i..]`: `self.i` is a byte index
            // that the byte-wise scanner can leave *inside* a multi-byte UTF-8
            // char (e.g. a non-ASCII variable name like `$itemés`), and string
            // slicing at a non-char-boundary panics. Every `MULTI_OPS` entry is
            // ASCII, so a byte-prefix check is exactly equivalent and total.
            if self.b[self.i..].starts_with(op.as_bytes()) {
                if tcl_dialect::expr_word_operator_since(op).is_some()
                    && !self.word_operator_lexeme_at(op, self.i, after_numeric_lexeme)
                {
                    continue;
                }
                let start = self.i;
                self.i += op.len();
                return Some(ExprToken {
                    kind: ExprTokenType::Operator,
                    text: op.to_owned(),
                    start: p(start),
                    end: p(start + op.len() - 1),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types(source: &str) -> Vec<ExprTokenType> {
        tokenise_expr(source, None)
            .into_iter()
            .filter(|t| t.kind != ExprTokenType::Whitespace)
            .map(|t| t.kind)
            .collect()
    }

    fn texts(source: &str) -> Vec<String> {
        tokenise_expr(source, None)
            .into_iter()
            .filter(|t| t.kind != ExprTokenType::Whitespace)
            .map(|t| t.text)
            .collect()
    }

    fn texts_for(source: &str, dialect: &str) -> Vec<String> {
        tokenise_expr(source, Some(dialect))
            .into_iter()
            .filter(|t| t.kind != ExprTokenType::Whitespace)
            .map(|t| t.text)
            .collect()
    }

    #[test]
    fn integer() {
        assert_eq!(texts("42"), vec!["42"]);
    }

    #[test]
    fn trailing_backslash_in_command_does_not_panic() {
        // A `[...` command substitution ending in a bare `\`
        // must not push the cursor past end-of-input (out-of-bounds slice in
        // `tok`). The whole token, backslash included, is returned.
        let toks = tokenise_expr(r"[a \", None);
        assert!(toks.iter().any(|t| t.kind == ExprTokenType::Command));
        // A realistic reachable form: `if {$x && [foo \` body extracted and
        // re-tokenised must not crash.
        let _ = tokenise_expr(r"$x && [foo \", None);
    }

    #[test]
    fn trailing_backslash_in_quoted_does_not_panic() {
        // A `"...` string ending in a bare `\`.
        let toks = tokenise_expr(r#""a\"#, None);
        assert!(toks.iter().any(|t| t.kind == ExprTokenType::String));
    }

    #[test]
    fn command_and_quote_tokens_use_the_shared_tcl_range_grammar() {
        let source = "[set x 1; # ] remains a script comment\nHTTP::host]";
        let (tokens, unknown) = tokenise_expr_checked(source, Some("tcl9.0"));
        assert!(!unknown, "a complete script substitution is executable");
        assert_eq!(
            tokens
                .iter()
                .find(|token| token.kind == ExprTokenType::Command)
                .map(|token| token.text.as_str()),
            Some(source)
        );

        let quoted = r#""a[format "%s" b]c""#;
        let (tokens, unknown) = tokenise_expr_checked(quoted, Some("tcl9.0"));
        assert!(!unknown, "a quote may contain a quoted nested script word");
        assert_eq!(
            tokens
                .iter()
                .find(|token| token.kind == ExprTokenType::String)
                .map(|token| token.text.as_str()),
            Some(quoted)
        );
    }

    #[test]
    fn unterminated_command_and_quote_are_checked_lex_errors() {
        for source in [r"[a ", r#""a\"#] {
            let (_tokens, unknown) = tokenise_expr_checked(source, None);
            assert!(unknown, "unterminated expression token: {source:?}");
        }
    }

    #[test]
    fn backslash_newline_is_whitespace_continuation() {
        // GAP-B7: `1 \<LF>    + 2` — the backslash-newline is a Tcl 9
        // line continuation and must lex as whitespace, not an Unknown
        // token, so the expression stays structured.
        let src = "1 \\\n    + 2";
        assert_eq!(
            types(src),
            vec![
                ExprTokenType::Number,
                ExprTokenType::Operator,
                ExprTokenType::Number
            ],
        );
        let (_toks, has_unknown) = tokenise_expr_checked(src, None);
        assert!(!has_unknown, "backslash-newline should not be unknown");
    }

    #[test]
    fn lone_backslash_without_newline_stays_unknown() {
        // A bare `\` not before a newline is still not valid expr
        // syntax — only the continuation form is whitespace.
        let (_toks, has_unknown) = tokenise_expr_checked("1 \\ 2", None);
        assert!(has_unknown);
    }

    #[test]
    fn float_and_scientific() {
        assert_eq!(texts("3.14"), vec!["3.14"]);
        assert_eq!(texts("1.5e10"), vec!["1.5e10"]);
    }

    #[test]
    fn hex_octal_binary() {
        assert_eq!(texts("0xFF"), vec!["0xFF"]);
        assert_eq!(texts("0o77"), vec!["0o77"]);
        assert_eq!(texts("0b1010"), vec!["0b1010"]);
    }

    #[test]
    fn operators() {
        assert_eq!(texts("1 + 2"), vec!["1", "+", "2"]);
        assert_eq!(texts("$a == $b"), vec!["$a", "==", "$b"]);
    }

    #[test]
    fn word_operators() {
        assert_eq!(texts("1 eq 2"), vec!["1", "eq", "2"]);
    }

    #[test]
    fn word_operator_boundary() {
        assert_eq!(texts("equal"), vec!["equal"]);
    }

    /// The F5 word-form operators are an `f5-tcl` **trunk** fact — measured
    /// valid in tmsh and iApp `expr` too, not iRules-only
    /// (`docs/design/bigip-irule-parser-measurements.md` §4a) — so every
    /// F5Tcl-cored profile lexes them as one `Operator` token, derived from
    /// the family's `ExprGrammar` word table.
    #[test]
    fn f5_word_operators_are_a_trunk_fact() {
        let f5_words = [
            "and",
            "or",
            "not",
            "contains",
            "starts_with",
            "ends_with",
            "equals",
            "matches",
            "matches_glob",
            "matches_regex",
        ];
        for dialect in ["f5-irules", "f5-tmsh", "f5-iapps"] {
            for op in f5_words {
                let tokens = tokenise_expr(op, Some(dialect));
                assert_eq!(tokens.len(), 1, "{dialect}: {op}");
                assert_eq!(
                    tokens[0].kind,
                    ExprTokenType::Operator,
                    "{dialect}: {op} must lex as an operator"
                );
            }
        }
        // Plain-Tcl parity: outside the F5 tree the same spellings stay
        // barewords (function-shaped lexemes), byte-identical to before.
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            for op in f5_words {
                let tokens = tokenise_expr(op, Some(dialect));
                assert_eq!(tokens.len(), 1, "{dialect}: {op}");
                assert_eq!(
                    tokens[0].kind,
                    ExprTokenType::Function,
                    "{dialect}: {op} must stay a bareword"
                );
            }
        }
    }

    #[test]
    fn variables() {
        assert_eq!(texts("$x"), vec!["$x"]);
        assert_eq!(texts("${name}"), vec!["${name}"]);
        assert_eq!(texts("$arr(idx)"), vec!["$arr(idx)"]);
    }

    #[test]
    fn array_index_first_paren_terminates() {
        // No paren nesting — the index ends at the first `)`.
        assert_eq!(texts("$a((b)").first().map(String::as_str), Some("$a((b)"));
        // An escaped `)` stays in the index; a `[…]`/`${…}` inner `)` too.
        assert_eq!(
            texts("$a(x\\)y)").first().map(String::as_str),
            Some("$a(x\\)y)")
        );
        assert_eq!(
            texts("$a([f(x)])").first().map(String::as_str),
            Some("$a([f(x)])")
        );
        assert_eq!(
            texts("$a(${b(c)})").first().map(String::as_str),
            Some("$a(${b(c)})")
        );
    }

    /// Issue #1601 — the `${…}` closer follows the release's
    /// `Tcl_ParseVarName` rule, the same one the carrying command's condition
    /// already used, not a local first-`}` walk.
    ///
    /// Oracles (`set {a{b}c} 7`, `set {a\}b} 7`, `set {a{b}} 7`):
    ///
    /// | expression | tclsh 9.0.4 | tclsh 8.6.16 |
    /// |---|---|---|
    /// | `expr {${a{b}c} + 1}` | `8` | `invalid bareword "c"` |
    /// | `expr {${a\}b} + 1}`  | `8` | `invalid bareword "b"` |
    /// | `expr {${a{b}} + 1}`  | `8` | `invalid character "}"` |
    #[test]
    fn braced_variable_closer_follows_the_release_rule() {
        // 9.x nests `{…}` and consumes `\X`, so each of these is one Variable
        // spanning the whole reference.
        for source in [r"${a{b}c}", r"${a\}b}", r"${a{b}}"] {
            let t = texts(source);
            assert_eq!(
                t.first().map(String::as_str),
                Some(source),
                "9.x names the whole reference: {source:?} -> {t:?}"
            );
            assert_eq!(t.len(), 1, "no trailing lexemes at 9.x: {t:?}");
        }

        // 8.x ends the name at the first literal `}` — `{` and `\` are name
        // characters — so the remainder lexes as its own (invalid) lexemes.
        let t = texts_for("${a{b}c}", "tcl8.6");
        assert_eq!(t.first().map(String::as_str), Some("${a{b}"), "{t:?}");
        assert!(t.iter().any(|s| s == "c"), "trailing bareword: {t:?}");
        let t = texts_for(r"${a\}b}", "tcl8.6");
        assert_eq!(t.first().map(String::as_str), Some(r"${a\}"), "{t:?}");
        assert!(t.iter().any(|s| s == "b"), "trailing bareword: {t:?}");

        // The nesting rule also *widens* what counts as unterminated: `${a{b}`
        // closes at 8.x but runs off the end at 9.x.
        let (tokens, unknown) = tokenise_expr_checked("${a{b}", Some("tcl8.6"));
        assert!(!unknown, "8.x closes at the first `}}`");
        assert_eq!(tokens.first().map(|t| t.text.as_str()), Some("${a{b}"));
        let (tokens, unknown) = tokenise_expr_checked("${a{b}", Some("tcl9.0"));
        assert!(
            unknown,
            "9.x has no closer left, so the expression is inexecutable"
        );
        assert_eq!(tokens.first().map(|t| t.text.as_str()), Some("${a{b}"));
    }

    #[test]
    fn single_colon_ends_variable_name() {
        // A lone `:` is not a name char — the ternary separator
        // in `$y:$z` must not be swallowed into the variable, or the spaceless
        // ternary `$x>0?$y:$z` degrades to Raw.
        let t = texts("$y:$z");
        assert_eq!(t.first().map(String::as_str), Some("$y"), "{t:?}");
        assert!(
            t.iter().any(|s| s == "$z"),
            "second var must survive: {t:?}"
        );
        // `::` namespace separators are still part of the name.
        assert_eq!(texts("$::g").first().map(String::as_str), Some("$::g"));
        assert_eq!(texts("$a::b").first().map(String::as_str), Some("$a::b"));
        // `$a:b` reports the variable as `a`, not `a:b`.
        assert_eq!(texts("$a:b").first().map(String::as_str), Some("$a"));
    }

    #[test]
    fn command_sub() {
        assert_eq!(texts("[cmd]"), vec!["[cmd]"]);
    }

    #[test]
    fn quoted_string() {
        assert_eq!(texts(r#""hello""#), vec![r#""hello""#]);
    }

    #[test]
    fn function_call() {
        let t = types("sin($x)");
        assert_eq!(t[0], ExprTokenType::Function);
    }

    #[test]
    fn boolean_literals() {
        assert_eq!(types("true"), vec![ExprTokenType::Bool]);
        // Tcl booleans are case-insensitive (`Tcl_GetBoolean`).
        for word in ["True", "FALSE", "Yes", "NO", "On", "Off"] {
            assert_eq!(
                types(word),
                vec![ExprTokenType::Bool],
                "word `{word}` should tokenise as BOOL",
            );
        }
    }

    #[test]
    fn ieee_754_special_literals() {
        // Tcl's `TclParseNumber` treats these case-insensitively. The shared
        // scanner also owns the NaN payload and number/bareword junction:
        // tclsh 8.6.17 and 9.0.3 both accept `iNf`, `NaN(1)`, and
        // `Infinityeq 1`, but keep `Infinityx` a bareword/function name.
        for word in [
            "Inf", "inf", "iNf", "Infinity", "infinity", "NaN", "nan", "NaN(1)",
        ] {
            assert_eq!(
                types(word),
                vec![ExprTokenType::Number],
                "word `{word}` should tokenise as NUMBER",
            );
        }
        assert_eq!(
            texts("Infinityeq 1"),
            vec!["Infinity", "eq", "1"],
            "a word operator after a special float starts a new lexeme",
        );
        assert_eq!(types("Infinityx"), vec![ExprTokenType::Function]);
    }

    #[test]
    fn ternary() {
        assert_eq!(
            types("$a ? 1 : 0"),
            vec![
                ExprTokenType::Variable,
                ExprTokenType::TernaryQ,
                ExprTokenType::Number,
                ExprTokenType::TernaryC,
                ExprTokenType::Number,
            ]
        );
    }

    #[test]
    fn braced() {
        let t = tokenise_expr("{1 + 2}", None);
        let nw: Vec<_> = t
            .into_iter()
            .filter(|t| t.kind != ExprTokenType::Whitespace)
            .collect();
        assert_eq!(nw[0].kind, ExprTokenType::String);
        assert_eq!(nw[0].text, "{1 + 2}");
    }

    #[test]
    fn braced_escaped_brace_does_not_close() {
        // `{a\}b} eq $x` — Tcl's brace scan treats `\}` as a literal pair, so
        // the whole `{a\}b}` is one braced String token, leaving a clean
        // `eq $x` behind rather than ending at the escaped `}` and degrading
        // to Raw (issue 165).
        let t = tokenise_expr(r"{a\}b} eq $x", None);
        let nw: Vec<_> = t
            .into_iter()
            .filter(|t| t.kind != ExprTokenType::Whitespace)
            .collect();
        assert_eq!(nw[0].kind, ExprTokenType::String);
        assert_eq!(nw[0].text, r"{a\}b}");
        assert_eq!(nw[1].kind, ExprTokenType::Operator);
        assert_eq!(nw[1].text, "eq");
        assert_eq!(nw[2].kind, ExprTokenType::Variable);
    }

    #[test]
    fn braced_escaped_open_brace_balances() {
        // `\{` likewise does not open a level: `{a\{b}` is one word.
        let t = tokenise_expr(r"{a\{b}", None);
        let nw: Vec<_> = t
            .into_iter()
            .filter(|t| t.kind != ExprTokenType::Whitespace)
            .collect();
        assert_eq!(nw[0].kind, ExprTokenType::String);
        assert_eq!(nw[0].text, r"{a\{b}");
    }

    #[test]
    fn checked_no_unknown() {
        let (_, u) = tokenise_expr_checked("1 + 2", None);
        assert!(!u);
    }

    #[test]
    fn checked_rejects_unclosed_variable_delimiters() {
        for source in ["${name", "$array(key", "$array([command)"] {
            let (_, unknown) = tokenise_expr_checked(source, Some("f5-irules"));
            assert!(
                unknown,
                "unterminated variable syntax stayed executable: {source:?}"
            );
        }
    }

    #[test]
    fn non_ascii_does_not_panic_at_operator_scan() {
        // Regression: the byte-wise scanner steps over a multi-byte UTF-8 char
        // (e.g. `é`) one byte at a time, so a later `multi_op` check ran at a
        // byte index *inside* the char. The old `self.s[self.i..]` string slice
        // panicked on the non-char-boundary; the byte-prefix check is total.
        // Surfaced via the optimiser running the expr lexer on a `foreach` whose
        // list variable had a non-ASCII name (`foreach item $itemés { … }`).
        for src in [
            "é",
            "$itemés",
            "aé eq b",
            "$x + é",
            " café ne thé",
            "\u{e9}\u{e9}\u{e9}",
        ] {
            // The regression guard is that neither entry point panics; both must
            // also agree on the token stream they produce.
            let toks = tokenise_expr(src, None);
            let (checked, _unknown) = tokenise_expr_checked(src, None);
            assert_eq!(
                toks.len(),
                checked.len(),
                "entry points disagree for {src:?}"
            );
        }
        // The non-ASCII char must not swallow a following operator: the scan has
        // to make progress past it and still recognise the `+`.
        let plus = tokenise_expr("$x + é", None);
        assert!(
            plus.iter()
                .any(|t| t.kind == ExprTokenType::Operator && t.text == "+"),
            "operator after non-ASCII not found: {plus:?}",
        );
    }

    // TIP 582 `#` comments (`tclCompExpr.c:1931-1942`)

    /// The comment token's text, for each comment in `source`.
    fn comments(source: &str) -> Vec<String> {
        tokenise_expr(source, None)
            .into_iter()
            .filter(|t| t.kind == ExprTokenType::Comment)
            .map(|t| t.text)
            .collect()
    }

    /// `#` runs to the end of the line or the end of the expression, wherever
    /// it appears — C skips a `COMMENT` lexeme exactly like a whitespace run
    /// (`tclCompExpr.c:701`), so it may sit anywhere whitespace may.
    #[test]
    fn a_comment_may_appear_wherever_whitespace_may() {
        // Trailing: the case that used to set `unknown` and degrade the whole
        // expression to `ExprNode::Raw`.
        assert_eq!(comments("1 + 2 # note"), vec!["# note"]);
        assert_eq!(
            types("1 + 2 # note"),
            vec![
                ExprTokenType::Number,
                ExprTokenType::Operator,
                ExprTokenType::Number,
                ExprTokenType::Comment,
            ]
        );
        // Mid-expression, and immediately before an operator with no
        // intervening space.
        assert_eq!(comments("1 #c\n+ 2"), vec!["#c"]);
        assert_eq!(comments("1#c\n+2"), vec!["#c"]);
        // A comment swallows the rest of the line, operators included, so
        // expr-62.1's `expr {1 # + 2}` is just `1`.
        assert_eq!(
            types("1 # + 2"),
            vec![ExprTokenType::Number, ExprTokenType::Comment]
        );
        // Inside a function's argument list, on either side of the comma.
        assert_eq!(comments("max(1,# comment\n2)"), vec!["# comment"]);
        assert_eq!(comments("max(1# comment\n,2)"), vec!["# comment"]);
    }

    /// C's `:750` lookahead: a comment between a bareword and its `(` still
    /// leaves a function call ("Actually a function call, but with obscuring
    /// comments"). Pins expr-62.10's token shape.
    #[test]
    fn a_comment_may_obscure_a_function_call_paren() {
        assert_eq!(
            types("max# comment\n(1,2)"),
            vec![
                ExprTokenType::Function,
                ExprTokenType::Comment,
                ExprTokenType::ParenOpen,
                ExprTokenType::Number,
                ExprTokenType::Comma,
                ExprTokenType::Number,
                ExprTokenType::ParenClose,
            ]
        );
    }

    /// The terminating newline is *not* part of the comment: C returns
    /// `size - (byte == '\n')`, leaving the newline to the whitespace scan.
    /// This is what keeps expr-62.2's `expr "1 #\n+ 2"` equal to 3 — if the
    /// comment ate the newline nothing would change here, but if it ate the
    /// rest of the *expression* the `+ 2` would vanish.
    #[test]
    fn a_comment_stops_before_its_newline() {
        let toks = tokenise_expr("1 #\n+ 2", None);
        let comment = toks
            .iter()
            .find(|t| t.kind == ExprTokenType::Comment)
            .expect("comment token");
        assert_eq!(comment.text, "#");
        assert_eq!((comment.start, comment.end), (2, 2));
        // The newline survives as its own whitespace token, and the `+ 2` is
        // still there.
        assert_eq!(
            types("1 #\n+ 2"),
            vec![
                ExprTokenType::Number,
                ExprTokenType::Comment,
                ExprTokenType::Operator,
                ExprTokenType::Number,
            ]
        );
        assert!(
            tokenise_expr("1 #\n+ 2", None)
                .iter()
                .any(|t| t.kind == ExprTokenType::Whitespace && t.text == "\n"),
            "the newline must be a whitespace token of its own"
        );
        // A `\` before the newline does not extend the comment either: C's scan
        // is byte-wise and stops at the first raw `\n`.
        assert_eq!(comments("1 #c\\\n+ 2"), vec!["#c\\"]);
        assert_eq!(
            types("1 #c\\\n+ 2"),
            vec![
                ExprTokenType::Number,
                ExprTokenType::Comment,
                ExprTokenType::Operator,
                ExprTokenType::Number,
            ]
        );
    }

    /// `doc/expr.n`'s one exception: `#` begins a comment "at any point in the
    /// expression **except within double quotes or braces**". Those operands are
    /// scanned as whole tokens before a `#` inside them is ever looked at, so a
    /// hash there stays ordinary text.
    #[test]
    fn a_hash_inside_a_quoted_or_braced_operand_is_not_a_comment() {
        for src in [r#""a#b" eq $x"#, r"{a#b} eq $x"] {
            let toks = tokenise_expr(src, None);
            assert!(
                !toks.iter().any(|t| t.kind == ExprTokenType::Comment),
                "{src:?} must not lex a comment: {toks:?}"
            );
            // The operand keeps its `#`, and the `eq $x` after it still lexes —
            // proof the hash did not swallow the rest of the line.
            assert_eq!(toks[0].kind, ExprTokenType::String);
            assert!(toks[0].text.contains('#'), "{src:?}");
            assert_eq!(
                types(src),
                vec![
                    ExprTokenType::String,
                    ExprTokenType::Operator,
                    ExprTokenType::Variable,
                ],
                "{src:?}"
            );
        }
    }

    /// A `#` inside a comment is ordinary text — the comment does not nest or
    /// re-open, it simply runs to end of line.
    #[test]
    fn a_comment_may_contain_a_hash() {
        assert_eq!(comments("1 + 2 # a # b"), vec!["# a # b"]);
        assert_eq!(
            comments("1 ## still one comment"),
            vec!["## still one comment"]
        );
    }

    /// An unterminated comment — one with no closing newline — runs to the end
    /// of the input and is not an error. C's scan simply stops at `numBytes`.
    #[test]
    fn an_unterminated_comment_runs_to_end_of_input() {
        for src in ["1 + 2 # no newline", "1 + 2 #", "#"] {
            let (toks, unknown) = tokenise_expr_checked(src, None);
            assert!(!unknown, "{src:?} must not report an unknown character");
            let comment = toks
                .iter()
                .find(|t| t.kind == ExprTokenType::Comment)
                .unwrap_or_else(|| panic!("no comment token for {src:?}"));
            // The token covers every remaining byte, so the stream still spans
            // the whole source — which is what keeps the diagnoser from
            // mistaking an uncovered offset for an invalid character.
            assert_eq!(
                comment.end as usize + 1,
                src.len(),
                "comment must reach end of input for {src:?}"
            );
        }
    }

    /// C stops the scan at an embedded NUL *and consumes it*: the
    /// `size - (byte == '\n')` adjustment fires only for a newline, so unlike
    /// a newline the NUL is inside the comment.
    #[test]
    fn an_embedded_nul_ends_the_comment_but_is_consumed() {
        let toks = tokenise_expr("1 #a\0+ 2", None);
        let comment = toks
            .iter()
            .find(|t| t.kind == ExprTokenType::Comment)
            .expect("comment token");
        assert_eq!(comment.text, "#a\0");
        // Scanning resumes after the NUL, so the `+ 2` is still lexed.
        assert_eq!(
            types("1 #a\0+ 2"),
            vec![
                ExprTokenType::Number,
                ExprTokenType::Comment,
                ExprTokenType::Operator,
                ExprTokenType::Number,
            ]
        );
    }

    /// The gate: TIP 582 is 9.0+. Under an 8.x dialect `#` is still an unknown
    /// character, which is what C 8.4-8.6 does with it — neither the `COMMENT`
    /// lexeme nor `ParseLexeme`'s `case '#':` exists in `tclCompExpr.c` at
    /// `core-8-5-19` / `core-8-6-16`, nor in 8.4's `tclParseExpr.c`.
    #[test]
    fn comments_are_tcl9_only() {
        for dialect in ["tcl9.0", "tcl9.1", "tcl"] {
            let (toks, unknown) = tokenise_expr_checked("1 + 2 # note", Some(dialect));
            assert!(!unknown, "{dialect} should accept a comment");
            assert!(
                toks.iter().any(|t| t.kind == ExprTokenType::Comment),
                "{dialect} should lex a comment token"
            );
        }
        // 8.x runtimes, and iRules (a genuine embedded 8.4).
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules"] {
            let (toks, unknown) = tokenise_expr_checked("1 + 2 # note", Some(dialect));
            assert!(unknown, "{dialect} should reject `#` as unknown");
            assert!(
                !toks.iter().any(|t| t.kind == ExprTokenType::Comment),
                "{dialect} must not lex a comment token"
            );
        }
    }
}

#[cfg(test)]
mod separator_boundary_tests {
    use super::*;

    /// Tcl 8.4's `GetLexeme` takes an integer prefix before it reaches the
    /// generic function-name/unknown scanner; it does not have the 8.5+
    /// number-to-bareword rescan. The shared lower scanner must therefore keep
    /// these token boundaries distinct for the last 8.4 profile.
    #[test]
    fn tcl84_keeps_legacy_integer_prefix_boundaries() {
        let (tokens, unknown) = tokenise_expr_checked("1_eq", Some("tcl8.4"));
        assert!(
            unknown,
            "`_` must be Tcl 8.4's invalid next lexeme: {tokens:?}"
        );
        assert_eq!(tokens[0].kind, ExprTokenType::Number);
        assert_eq!(tokens[0].text, "1");

        for (source, first, second) in [("12x", "12", "x"), ("0x1p2", "0x1", "p2")] {
            let tokens = tokenise_expr(source, Some("tcl8.4"));
            assert_eq!(
                tokens[0].kind,
                ExprTokenType::Number,
                "{source}: {tokens:?}"
            );
            assert_eq!(tokens[0].text, first, "{source}: {tokens:?}");
            assert_eq!(tokens[1].text, second, "{source}: {tokens:?}");
        }
    }

    /// Explicit-radix spelling validity comes from the shared `NumberSyntax`
    /// owner. Once a prefix has valid digits, a following release-available
    /// word operator is a separate lexeme; an invalid radix stays whole.
    #[test]
    fn explicit_radix_numbers_stop_before_word_operators() {
        for (dialect, source, number, operator) in [
            ("tcl8.6", "0x1ne 1", "0x1", "ne"),
            ("tcl8.6", "0xfne 1", "0xf", "ne"),
            ("tcl8.6", "0xffin {255}", "0xff", "in"),
            ("tcl9.0", "0xfge 15", "0xf", "ge"),
            ("tcl8.6", "0b1ne 1", "0b1", "ne"),
            ("tcl8.6", "0o7in {7}", "0o7", "in"),
            ("tcl8.6", "0b1ni {1}", "0b1", "ni"),
            ("tcl9.0", "0d9lt 10", "0d9", "lt"),
            ("tcl9.0", "0d9le 9", "0d9", "le"),
            ("tcl9.0", "0d9gt 8", "0d9", "gt"),
            ("tcl9.0", "0d9ge 9", "0d9", "ge"),
        ] {
            let tokens = tokenise_expr(source, Some(dialect));
            assert_eq!(
                tokens[0].kind,
                ExprTokenType::Number,
                "{dialect}: {source}: {tokens:?}"
            );
            assert_eq!(tokens[0].text, number, "{dialect}: {source}: {tokens:?}");
            assert_eq!(tokens[1].text, operator, "{dialect}: {source}: {tokens:?}");
        }
        for (dialect, source) in [
            ("tcl8.6", "0o8ne 1"),
            ("tcl8.6", "0d9ne 1"),
            ("tcl9.0", "0b2ne 1"),
        ] {
            let tokens = tokenise_expr(source, Some(dialect));
            assert_eq!(
                tokens[0].text,
                source.split_whitespace().next().unwrap(),
                "{dialect}: {tokens:?}"
            );
        }
    }

    #[test]
    fn ordinary_number_barewords_stay_whole_after_a_nonzero_offset() {
        for source in ["0 + 1_eq", "0 + 12x"] {
            let expected = source.split(" + ").nth(1).unwrap();
            let tokens = tokenise_expr(source, Some("tcl9.0"));
            assert!(
                tokens.iter().any(|token| token.text == expected),
                "{source}: {tokens:?}"
            );
        }
    }

    #[test]
    fn completed_nan_payload_is_a_number_before_following_bareword() {
        let tokens = tokenise_expr("NaN(1)x", Some("tcl9.0"));
        assert_eq!(tokens[0].kind, ExprTokenType::Number, "{tokens:?}");
        assert_eq!(tokens[0].text, "NaN(1)");
        assert_eq!(tokens[1].text, "x");
        assert_eq!(
            tokenise_expr("NaNx", Some("tcl9.0"))[0].kind,
            ExprTokenType::Function
        );
    }

    /// `_` inside a *fraction* changes the lexeme boundary, so unlike the
    /// integer run it has to follow the release. C's bareword scan takes
    /// `[A-Za-z0-9_]` and stops at `.`, which is why `1_0` is a bareword on
    /// 8.6 while `1.0_2` is an `invalid character "_"` there — pinned on
    /// tclsh 8.6.16 and 9.0.4.
    #[test]
    fn fraction_separators_follow_the_release() {
        // 9.0: one numeral.
        let t = tokenise_expr("1.0_2", Some("tcl9.0"));
        assert_eq!(t.len(), 1, "9.0 lexes `1.0_2` whole: {t:?}");
        assert_eq!(t[0].text, "1.0_2");

        // 8.6: the number ends at `1.0`, and `_` is its own (unknown) lexeme.
        let t = tokenise_expr("1.0_2", Some("tcl8.6"));
        assert!(t.len() > 1, "8.6 must split `1.0_2`: {t:?}");
        assert_eq!(t[0].text, "1.0");

        // The exponent behaves the same way.
        assert_eq!(tokenise_expr("1e1_0", Some("tcl9.0")).len(), 1);

        // The *integer* run absorbs `_` on every release — `1_0` is one
        // bareword candidate on 8.6 too, which is what C reports.
        assert_eq!(tokenise_expr("1_0", Some("tcl8.6"))[0].text, "1_0");
        assert_eq!(tokenise_expr("1_0", Some("tcl9.0"))[0].text, "1_0");
    }

    /// `_` never starts a lexeme, on any release — `ParseLexeme`'s final gate
    /// is `if (!TclIsBareword(*start) || *start == '_')`, which yields INVALID
    /// rather than BAREWORD (9.0.4 `tclCompExpr.c:2136`, 8.6.16 `:2068`,
    /// 8.5.19 `:1980`; 8.4.20 `tclParseExpr.c:1852` requires `isalpha` to
    /// start a lexeme at all). Verified on tclsh 8.4.20, 8.5.19, 8.6.16,
    /// 9.0.4 and 9.1b0: every one reports `_`, `_1`, `_abc`, `1+_2` and
    /// `$x + _y` as an invalid *character*, never a bareword.
    #[test]
    fn underscore_never_starts_a_lexeme() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for src in ["_", "_1", "_abc", "__"] {
                let (toks, unknown) = tokenise_expr_checked(src, Some(dialect));
                assert!(
                    unknown,
                    "{dialect}: `{src}` must be an invalid character, got {toks:?}"
                );
            }
            // …but `_` *inside* a bareword that starts alphanumeric is fine:
            // `abc_1` and `a__b` are barewords on every release.
            assert_eq!(tokenise_expr("abc_1", Some(dialect))[0].text, "abc_1");
            assert_eq!(tokenise_expr("a__b", Some(dialect))[0].text, "a__b");
        }
    }

    /// The word-operator set is release-dependent, and that moves the lexeme
    /// boundary *where a bareword byte follows the operator*. tclsh-verified:
    /// `1 lt_ 2` is `invalid bareword "lt_"` on 8.5/8.6 (no `lt` lexeme before
    /// TIP 461) but `invalid character "_"` on 9.0/9.1, and `1 lt2 2` is
    /// `invalid bareword "lt2"` on 8.6 against `missing operator` on 9.0.
    /// `eq` exists throughout, so `eq_` splits on every release.
    #[test]
    fn word_operator_availability_moves_the_boundary() {
        // `lt` is 9.0+: before it, `lt_` / `lt2` are one bareword.
        for old in ["tcl8.5", "tcl8.6"] {
            for src in ["1 lt_ 2", "1 lt2 2"] {
                let t = tokenise_expr(src, Some(old));
                assert!(
                    t[2].text.starts_with("lt") && t[2].text.len() > 2,
                    "{old} must keep `{src}`'s `lt…` whole: {t:?}"
                );
            }
        }
        for new in ["tcl9.0", "tcl9.1"] {
            let t = tokenise_expr("1 lt_ 2", Some(new));
            assert_eq!(t[2].text, "lt", "{new} must split `lt_`: {t:?}");
        }
        // `eq` is in every modelled release, so `eq_` splits in all of them.
        for d in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            assert_eq!(tokenise_expr("1 eq_ 2", Some(d))[2].text, "eq");
        }
        // C guards the trailing side with `isalpha` only, so a digit after the
        // operator still leaves it an operator: `1 eq2` is `1 eq 2` (tclsh: 0).
        let t = tokenise_expr("1 eq2", Some("tcl9.0"));
        assert_eq!(t[2].text, "eq", "a digit must not glue onto `eq`: {t:?}");
    }

    /// The release gate must NOT fire where it changes nothing about the
    /// boundary — a word operator followed by whitespace is one lexeme in
    /// every release, and the version check belongs above the lexer (the
    /// parser reports `invalid bareword "lt"` under 8.6, which is what tclsh
    /// prints). W003 reads these tokens to say "this operator is Tcl 9.0+
    /// (TIP 461)", so suppressing them here silenced the diagnostic on exactly
    /// the dialects it targets — nine e2e tests caught it.
    #[test]
    fn a_standalone_word_operator_stays_visible_to_the_analyser() {
        for d in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules"] {
            for (src, op) in [("$a lt $b", "lt"), ("$a in $b", "in"), ("1 ge 2", "ge")] {
                let t = tokenise_expr(src, Some(d));
                assert!(
                    t.iter()
                        .any(|k| k.kind == ExprTokenType::Operator && k.text == op),
                    "{d}: `{src}` must still expose `{op}` as an Operator token: {t:?}"
                );
            }
        }
    }

    /// A numeric run butted against bareword characters is ONE bareword —
    /// C rescans `[A-Za-z0-9_]*` from the start rather than emitting a number
    /// (`tclCompExpr.c:2080-2126`). Two exceptions keep it a number: a
    /// non-bareword character in the run (`1.5abc` names only `abc`), and a
    /// following binary word operator (`1eq 2` is `1 eq 2`). All tclsh-checked
    /// on 8.5, 8.6, 9.0 and 9.1.
    #[test]
    fn a_number_against_barewords_is_one_bareword() {
        for d in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            for src in ["1_eq", "1abc", "12x", "1e_0", "9_ne", "1_2_eq", "1_abc"] {
                let t = tokenise_expr(src, Some(d));
                assert_eq!(t[0].text, src, "{d}: `{src}` must stay one lexeme: {t:?}");
            }
            // Exception 1 — the run holds a `.`, which no bareword can.
            let t = tokenise_expr("1.5abc", Some(d));
            assert_eq!(
                t[0].text, "1.5",
                "{d}: `1.5abc` splits at the double: {t:?}"
            );
            // Exception 2 — a binary word operator follows.
            let t = tokenise_expr("1eq 2", Some(d));
            assert_eq!(t[0].text, "1", "{d}: `1eq 2` is `1 eq 2`: {t:?}");
            assert_eq!(t[1].text, "eq");
        }
        // Release-sensitive: `lt` is not an operator before 9.0, so `1lt 2`
        // is one bareword there and a number + operator after.
        assert_eq!(tokenise_expr("1lt 2", Some("tcl8.6"))[0].text, "1lt");
        assert_eq!(tokenise_expr("1lt 2", Some("tcl9.0"))[0].text, "1");
    }
}
