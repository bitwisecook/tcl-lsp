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

//! Why an `expr` body does not parse, in C Tcl's own words.
//!
//! [`parse_expr`](super::parser::parse_expr) deliberately collapses every
//! failure into [`ExprNode::Raw`](super::ast::ExprNode::Raw) so the analysis and
//! LSP pipelines never crash on malformed input. That resilience costs the
//! *reason*, which a runtime evaluator needs: C Tcl's `Tcl_ExprObj` raises a
//! syntax error naming the failure mode, quoting the expression around the
//! offending position, and setting a `TCL PARSE EXPR …` error code.
//!
//! This module recovers the reason with a second, diagnosis-only pass over the
//! same token stream — `parse_expr`'s contract is untouched. The pass mirrors
//! `ParseExpr` in `tclCompExpr.c` (Tcl 9.0.4): C classifies each lexeme, then
//! checks it against the operator/operand alternation it is in, and the first
//! check that trips names the error. [`ExprSyntaxError::message`] reproduces
//! `ParseExpr`'s `error:` label (`tclCompExpr.c:1397-1471`) byte for byte,
//! including the 25-byte elision window and the `_@_` insert mark.
//!
//! The expected strings are pinned by C's own suite: `tests/parseExpr.test`
//! cases `parseExpr-21.1` … `parseExpr-21.31`.

use tcl_dialect::{DialectProfile, TclVersion};
use tcl_lexer::{ExprToken, ExprTokenType};

/// C's `limit` (`tclCompExpr.c:615`): the longest substring of the expression
/// any part of the message quotes before eliding with `...`.
const LIMIT: usize = 25;

/// C's `mark` (`tclCompExpr.c:607`): dropped into the quoted expression to
/// pinpoint a position that has no offending text of its own.
const MARK: &str = "_@_";

/// C's radix hint for a bareword that looks like a botched numeric literal
/// (`tclCompExpr.c:779-809`): the message stays `invalid bareword`, but the
/// postscript gains a parenthesised guess and the error code becomes
/// `BADNUMBER` with a radix subcode.
///
/// C has arms for `b` and `o` and a `default` that fires only when the second
/// character is a digit (legacy leading-zero octal). There is deliberately **no
/// `x` arm**, so a bad hex numeral such as `0xg` keeps the plain `BAREWORD`
/// code and gets no hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumberHint {
    /// `0b…` with a non-binary digit.
    Binary,
    /// `0o…` with a non-octal digit, or a leading-zero numeral with one.
    Octal,
}

impl NumberHint {
    /// C's parenthesised postscript suffix, leading space included.
    fn postscript_suffix(self) -> &'static str {
        match self {
            Self::Binary => " (invalid binary number?)",
            Self::Octal => " (invalid octal number?)",
        }
    }

    /// The `subErrCode` word C pairs with `BADNUMBER`.
    fn sub_code(self) -> &'static str {
        match self {
            Self::Binary => "BINARY",
            Self::Octal => "OCTAL",
        }
    }

    /// C's test at `tclCompExpr.c:779-787`: only for a word starting `0`, and
    /// only when `TclParseNumber` stopped *at a digit* or after consuming just
    /// the leading `0` — i.e. the digits, not the prefix letter, are what went
    /// wrong. `syntax` decides where the parse stops, so the hint follows the
    /// release (`0o8` earns one from 8.5, where `0o` exists at all).
    fn for_word(word: &str, syntax: crate::number::NumberSyntax) -> Option<Self> {
        let b = word.as_bytes();
        if b.first() != Some(&b'0') || b.len() < 2 {
            return None;
        }
        let stop = crate::number::parse(word, crate::number::ParseFlags::for_syntax(syntax))
            .map_or(0, |p| p.end);
        let stopped_at_digit = b.get(stop).is_some_and(u8::is_ascii_digit);
        if !stopped_at_digit && stop != 1 {
            return None;
        }
        match b[1] {
            b'b' | b'B' => Some(Self::Binary),
            b'o' | b'O' => Some(Self::Octal),
            c if c.is_ascii_digit() => Some(Self::Octal),
            _ => None,
        }
    }
}

/// A `ParseExpr` failure mode, with the message and `-errorcode` detail word C
/// pairs with it.
///
/// Only the modes this crate's lexer and token stream can actually reach are
/// modelled; the number-specific `BADNUMBER` / `NOMEM` codes and the
/// `Tcl_ParseCommand`-owned quoting failures (`missing "`, `missing
/// close-brace`) are raised before or below this layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprSyntaxErrorKind {
    /// `tclCompExpr.c:1135` — nothing but whitespace.
    EmptyExpression,
    /// `tclCompExpr.c:707` — a byte that begins no lexeme.
    InvalidCharacter,
    /// `tclCompExpr.c:766` — a word that is neither operator, boolean, nor
    /// function call. Carries a postscript listing the spellings C suggests.
    InvalidBareword,
    /// `tclCompExpr.c:854`/`1054` — two operands, or an operand then a unary
    /// operator, with no operator between them.
    MissingOperator,
    /// `tclCompExpr.c:1151` — an operator with nothing to its right.
    MissingOperand,
    /// `tclCompExpr.c:1237` — a `?` whose right operand is not a `:` arm.
    MissingTernaryColon,
    /// `tclCompExpr.c:1250` — a `:` with no `?` before it.
    UnexpectedColon,
    /// `tclCompExpr.c:1229` — a `(` that is never closed.
    UnbalancedOpenParen,
    /// `tclCompExpr.c:1316` — a `)` that closes nothing.
    UnbalancedCloseParen,
    /// `tclCompExpr.c:1116` — `()` outside a function's argument list.
    EmptySubexpression,
    /// `tclCompExpr.c:1130`/`1144` — a hole in a function's argument list.
    MissingFunctionArgument,
    /// `tclCompExpr.c:1327` — a `,` outside a function's argument list.
    UnexpectedComma,
}

impl ExprSyntaxErrorKind {
    /// The `-errorcode` detail word C pairs with this mode — the fourth element
    /// of `TCL PARSE EXPR <detail>`.
    const fn detail(self) -> &'static str {
        match self {
            Self::EmptyExpression | Self::EmptySubexpression => "EMPTY",
            Self::InvalidCharacter => "BADCHAR",
            Self::InvalidBareword => "BAREWORD",
            Self::MissingOperator
            | Self::MissingOperand
            | Self::MissingTernaryColon
            | Self::MissingFunctionArgument => "MISSING",
            Self::UnbalancedOpenParen | Self::UnbalancedCloseParen => "UNBALANCED",
            Self::UnexpectedColon | Self::UnexpectedComma => "SURPRISE",
        }
    }

    /// Whether C inserts [`MARK`] at the failure position for this mode.
    const fn marks_position(self) -> bool {
        matches!(
            self,
            Self::MissingOperator
                | Self::MissingOperand
                | Self::MissingTernaryColon
                | Self::EmptySubexpression
                | Self::MissingFunctionArgument
        )
    }
}

/// A diagnosed `expr` syntax error, positioned as C positions it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprSyntaxError {
    kind: ExprSyntaxErrorKind,
    /// C's `start` at the `error:` label: the byte offset the failure is
    /// reported at.
    at: usize,
    /// C's `scanned`: how many bytes of the offending lexeme the message quotes
    /// before the mark. Zero for every marked mode, as C sets it.
    scanned: usize,
    /// The offending text, for the two modes whose message names it.
    word: String,
    /// A botched-numeral hint, when the bareword looks like one — see
    /// [`NumberHint`].
    number_hint: Option<NumberHint>,
}

impl ExprSyntaxError {
    /// Diagnose from a compatibility dialect-name boundary.
    ///
    /// Always returns an error: call it only for a source
    /// [`parse_expr`](super::parser::parse_expr) rejected, and pass the same
    /// dialect it was given so the two agree on which words are operators.
    /// Typed callers use [`Self::diagnose_for_profile`].
    #[must_use]
    pub fn diagnose(source: &str, dialect: Option<&str>) -> Self {
        Self::diagnose_for_profile(
            source,
            dialect
                .map(|name| DialectProfile::find(name).unwrap_or_else(DialectProfile::plain_tcl)),
        )
    }

    /// Diagnose under an already-resolved dialect profile.
    #[must_use]
    pub fn diagnose_for_profile(source: &str, profile: Option<&DialectProfile>) -> Self {
        let resolved = profile.map_or_else(DialectProfile::plain_tcl, |profile| {
            DialectProfile::find(profile.name).unwrap_or_else(DialectProfile::plain_tcl)
        });
        let (raw, has_unknown) = tcl_lexer::tokenise_expr_checked_for_profile(source, resolved);
        if has_unknown && let Some(error) = Self::first_invalid_character(source, &raw) {
            return error;
        }
        let tokens: Vec<ExprToken> = raw.into_iter().filter(|t| !t.kind.is_skipped()).collect();
        Scan::new(
            source,
            &tokens,
            super::parser::numbers_for(profile, resolved),
            resolved.expr_grammar_base,
        )
        .run()
    }

    /// The simple message plus C's quoted context (and postscript, where C adds
    /// one) — `Tcl_SetObjResult`'s value at `ParseExpr`'s `error:` label.
    #[must_use]
    pub fn message(&self, source: &str) -> String {
        let mut out = self.simple_message();
        out.push_str(&self.quoted_context(source));
        if let Some(post) = self.postscript() {
            out.push_str(";\n");
            out.push_str(&post);
        }
        out
    }

    /// The full Tcl `-errorcode`, `TCL PARSE EXPR <detail>`.
    #[must_use]
    pub fn error_code(&self) -> String {
        // A hinted bareword is C's `BADNUMBER <radix>` rather than `BAREWORD`
        // (`tclCompExpr.c:788-807` reassigns `errCode` and sets `subErrCode`).
        if let Some(hint) = self.number_hint {
            return format!("TCL PARSE EXPR BADNUMBER {}", hint.sub_code());
        }
        format!("TCL PARSE EXPR {}", self.kind.detail())
    }

    /// The `errorInfo` frame C appends for a failed expression parse
    /// (`tclCompExpr.c:1461-1466`), leading newline included.
    #[must_use]
    pub fn error_info_frame(source: &str) -> String {
        let (head, ellipsis) = elide(source.as_bytes(), 0, source.len());
        format!("\n    (parsing expression \"{head}{ellipsis}\")")
    }

    /// The diagnosed failure mode.
    #[must_use]
    pub const fn kind(&self) -> ExprSyntaxErrorKind {
        self.kind
    }

    fn at(kind: ExprSyntaxErrorKind, at: usize, scanned: usize) -> Self {
        Self {
            kind,
            at,
            // C zeroes `scanned` wherever it sets `insertMark`, so the mark lands
            // on the position rather than after the offending lexeme.
            scanned: if kind.marks_position() { 0 } else { scanned },
            word: String::new(),
            number_hint: None,
        }
    }

    fn naming(kind: ExprSyntaxErrorKind, at: usize, word: &str) -> Self {
        Self {
            kind,
            at,
            scanned: word.len(),
            word: word.to_owned(),
            number_hint: None,
        }
    }

    /// C's bareword error for a word that looks like a botched numeral: the
    /// same message and postscript, plus [`NumberHint`]'s parenthesised guess
    /// and `BADNUMBER` code where C adds them.
    fn bad_numeral(at: usize, word: &str, syntax: crate::number::NumberSyntax) -> Self {
        Self {
            number_hint: NumberHint::for_word(word, syntax),
            ..Self::naming(ExprSyntaxErrorKind::InvalidBareword, at, word)
        }
    }

    /// The first source character the lexer skipped — the gap its
    /// `has_unknown` flag reports without naming.
    fn first_invalid_character(source: &str, tokens: &[ExprToken]) -> Option<Self> {
        // The lexer emits no token for a byte it rejects — it just advances past
        // it — so an uncovered offset *is* the offending character. Token spans
        // are inclusive of `end`.
        let covered = |offset: usize| {
            tokens
                .iter()
                .any(|t| (t.start as usize..=t.end as usize).contains(&offset))
        };
        source
            .char_indices()
            .find(|(offset, _)| !covered(*offset))
            .map(|(offset, ch)| {
                Self::naming(
                    ExprSyntaxErrorKind::InvalidCharacter,
                    offset,
                    &source[offset..offset + ch.len_utf8()],
                )
            })
    }

    fn simple_message(&self) -> String {
        match self.kind {
            ExprSyntaxErrorKind::EmptyExpression => "empty expression".to_owned(),
            ExprSyntaxErrorKind::InvalidCharacter => {
                format!("invalid character \"{}\"", self.word)
            }
            ExprSyntaxErrorKind::InvalidBareword => {
                let (word, ellipsis) = elide(self.word.as_bytes(), 0, self.word.len());
                format!("invalid bareword \"{word}{ellipsis}\"")
            }
            ExprSyntaxErrorKind::MissingOperator => format!("missing operator at {MARK}"),
            ExprSyntaxErrorKind::MissingOperand => format!("missing operand at {MARK}"),
            ExprSyntaxErrorKind::MissingTernaryColon => {
                format!("missing operator \":\" at {MARK}")
            }
            ExprSyntaxErrorKind::UnexpectedColon => {
                "unexpected operator \":\" without preceding \"?\"".to_owned()
            }
            ExprSyntaxErrorKind::UnbalancedOpenParen => "unbalanced open paren".to_owned(),
            ExprSyntaxErrorKind::UnbalancedCloseParen => "unbalanced close paren".to_owned(),
            ExprSyntaxErrorKind::EmptySubexpression => format!("empty subexpression at {MARK}"),
            ExprSyntaxErrorKind::MissingFunctionArgument => {
                format!("missing function argument at {MARK}")
            }
            ExprSyntaxErrorKind::UnexpectedComma => {
                "unexpected \",\" outside function argument list".to_owned()
            }
        }
    }

    /// C's postscript (`tclCompExpr.c:769-779`): only the bareword mode has one.
    fn postscript(&self) -> Option<String> {
        if self.kind != ExprSyntaxErrorKind::InvalidBareword {
            return None;
        }
        let (word, ellipsis) = elide(self.word.as_bytes(), 0, self.word.len());
        let mut post = format!(
            "should be \"${word}{ellipsis}\" or \"{{{word}{ellipsis}}}\" or \"{word}{ellipsis}(...)\" or ..."
        );
        if let Some(hint) = self.number_hint {
            post.push_str(hint.postscript_suffix());
        }
        Some(post)
    }

    /// `\nin expression "<head><lexeme><mark><tail>"`, with each of the three
    /// slices independently elided at [`LIMIT`] exactly as
    /// `Tcl_AppendPrintfToObj` at `tclCompExpr.c:1434-1445` does.
    fn quoted_context(&self, source: &str) -> String {
        let bytes = source.as_bytes();
        let at = self.at.min(bytes.len());
        let scanned = self.scanned.min(bytes.len() - at);

        // Head: the expression up to the failure, elided from the *left* so the
        // failure position stays visible.
        let (head, head_ellipsis) = if at < LIMIT {
            (lossy(bytes, 0, at), "")
        } else {
            (lossy(bytes, at + 3 - LIMIT, at), "...")
        };
        let (lexeme, lexeme_ellipsis) = elide(bytes, at, scanned);
        let mark = if self.kind.marks_position() { MARK } else { "" };
        let (tail, tail_ellipsis) = elide(bytes, at + scanned, bytes.len() - at - scanned);

        format!(
            "\nin expression \"{head_ellipsis}{head}{lexeme}{lexeme_ellipsis}{mark}{tail}{tail_ellipsis}\""
        )
    }
}

/// Take `len` bytes from `offset`, capped at [`LIMIT`] minus the `...` that
/// then stands in for the remainder — C's `(n < limit) ? n : limit - 3` idiom.
fn elide(bytes: &[u8], offset: usize, len: usize) -> (String, &'static str) {
    if len < LIMIT {
        (lossy(bytes, offset, offset + len), "")
    } else {
        (lossy(bytes, offset, offset + LIMIT - 3), "...")
    }
}

/// A byte slice as display text. C quotes raw bytes; a mid-character offset
/// (the expr lexer advances one byte past an unknown one) must not panic here.
fn lossy(bytes: &[u8], start: usize, end: usize) -> String {
    let start = start.min(bytes.len());
    let end = end.clamp(start, bytes.len());
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

/// What the alternation expects next: C's `lastParsed`, reduced to the one bit
/// its syntax checks read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expecting {
    /// C's `IsOperator(lastParsed)` — `START`, or an operator awaiting its right
    /// operand.
    Operand,
    /// C's `NotOperator(lastParsed)` — a complete subexpression is in hand.
    Operator,
}

/// One open grouping, so a `,` / `)` / end-of-input can be judged against the
/// construct it is inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Group {
    /// `(` opening a parenthesised subexpression.
    Paren,
    /// `(` opening a function call's argument list.
    ArgList,
    /// `?` awaiting its `:`.
    Ternary,
}

/// C's `ParseExpr` main loop, reduced to the syntax checks that decide *which*
/// error to report.
struct Scan<'a> {
    source: &'a str,
    tokens: &'a [ExprToken],
    pos: usize,
    expecting: Expecting,
    open: Vec<Group>,
    /// The release's numeric grammar, so a `Number` token the lexer only
    /// delimited can be re-tested here the way C's `ParseLexeme` does.
    numbers: crate::number::NumberSyntax,
    /// The paired expression word-operator grammar for the shared numeral
    /// boundary check.
    expr_grammar_base: Option<TclVersion>,
}

impl<'a> Scan<'a> {
    fn new(
        source: &'a str,
        tokens: &'a [ExprToken],
        numbers: crate::number::NumberSyntax,
        expr_grammar_base: Option<TclVersion>,
    ) -> Self {
        Self {
            source,
            tokens,
            numbers,
            expr_grammar_base,
            pos: 0,
            // C seeds the tree with a `START` node, which `IsOperator` accepts.
            expecting: Expecting::Operand,
            open: Vec::new(),
        }
    }

    /// Walk the tokens left to right, returning the first check that trips.
    fn run(mut self) -> ExprSyntaxError {
        if self.tokens.is_empty() {
            // C reaches `END` with `lastParsed == START` only after its main
            // loop has skipped every whitespace run and comment, so its `start`
            // — the offset the quote is centred on (`tclCompExpr.c:1435-1444`)
            // — is the end of the input, not its beginning. The distinction is
            // invisible for a short expression, where head and tail together
            // reproduce the source either way, but decides which end gets
            // elided once the skipped run passes `LIMIT`: C keeps the *last* 22
            // bytes behind a leading `...`. Reachable in practice only now that
            // a comment can be the whole expression body.
            return ExprSyntaxError::at(ExprSyntaxErrorKind::EmptyExpression, self.source.len(), 0);
        }
        while self.pos < self.tokens.len() {
            if let Some(error) = self.step() {
                return error;
            }
        }
        self.at_end()
    }

    fn token(&self, index: usize) -> Option<&'a ExprToken> {
        self.tokens.get(index)
    }

    fn start_of(&self, index: usize) -> usize {
        self.token(index)
            .map_or(self.source.len(), |t| t.start as usize)
    }

    /// The lexeme before the one [`Self::step`] is handling — C's `nodePtr[-1]`,
    /// which several of its checks branch on. `None` stands for C's `START`.
    fn previous_kind(&self) -> Option<ExprTokenType> {
        self.pos
            .checked_sub(2)
            .and_then(|i| self.token(i))
            .map(|t| t.kind)
    }

    /// Classify one token and check it against the alternation.
    fn step(&mut self) -> Option<ExprSyntaxError> {
        let token = self.token(self.pos)?;
        let at = token.start as usize;
        self.pos += 1;
        match token.kind {
            // A bareword is a function call only when `(` follows; our lexer
            // cannot tell the two apart, so the check lands here rather than in
            // the lexer (C decides it at `tclCompExpr.c:716-780`).
            ExprTokenType::Function => self.function_or_bareword(token, at),
            // The lexer delimits a numeric-looking run without validating it,
            // so a radix-invalid numeral (`0o8`), a bare prefix (`0x`), or a
            // prefix this release lacks (`0d99` before 9.0) arrives as a
            // `Number`. C classifies exactly those as barewords, with a radix
            // hint where it can guess (`tclCompExpr.c:716-809`).
            ExprTokenType::Number
                if !crate::number::is_expr_number(
                    &token.text,
                    self.numbers,
                    self.expr_grammar_base,
                ) =>
            {
                Some(ExprSyntaxError::bad_numeral(at, &token.text, self.numbers))
            }
            ExprTokenType::Number
            | ExprTokenType::String
            | ExprTokenType::Variable
            | ExprTokenType::Command
            | ExprTokenType::Bool => self.push_operand(at),
            ExprTokenType::Operator => self.operator(token, at),
            ExprTokenType::ParenOpen => self.paren_open(at),
            ExprTokenType::ParenClose => self.paren_close(at),
            ExprTokenType::Comma => self.comma(at),
            ExprTokenType::TernaryQ => self.ternary_question(at),
            ExprTokenType::TernaryC => self.ternary_colon(at),
            // Filtered out before the scan (whitespace and comments) or never
            // emitted by the lexer at all (`Eof`).
            ExprTokenType::Whitespace | ExprTokenType::Comment | ExprTokenType::Eof => None,
        }
    }

    /// A complete operand: legal only where an operand is expected.
    fn push_operand(&mut self, at: usize) -> Option<ExprSyntaxError> {
        if self.expecting == Expecting::Operator {
            return Some(ExprSyntaxError::at(
                ExprSyntaxErrorKind::MissingOperator,
                at,
                0,
            ));
        }
        self.expecting = Expecting::Operator;
        None
    }

    /// An identifier: a function call when `(` follows, else C's bareword error.
    fn function_or_bareword(&mut self, token: &ExprToken, at: usize) -> Option<ExprSyntaxError> {
        if self.token(self.pos).map(|t| t.kind) != Some(ExprTokenType::ParenOpen) {
            return Some(ExprSyntaxError::naming(
                ExprSyntaxErrorKind::InvalidBareword,
                at,
                &token.text,
            ));
        }
        if self.expecting == Expecting::Operator {
            return Some(ExprSyntaxError::at(
                ExprSyntaxErrorKind::MissingOperator,
                at,
                0,
            ));
        }
        // Consume the `(` here so it opens an argument list, not a
        // parenthesised subexpression: `f()` is legal where `()` is not.
        self.pos += 1;
        self.open.push(Group::ArgList);
        self.expecting = Expecting::Operand;
        None
    }

    /// `+`, `-`, `!`, `eq`, a dialect word operator … — unary where an operand
    /// is expected, binary where an operator is.
    fn operator(&mut self, token: &ExprToken, at: usize) -> Option<ExprSyntaxError> {
        if self.expecting == Expecting::Operand {
            if super::parser::is_unary_operator(&token.text) {
                return None;
            }
            // A binary operator with no left operand: C reports the hole rather
            // than the operator (`tclCompExpr.c:1151`).
            return Some(ExprSyntaxError::at(
                ExprSyntaxErrorKind::MissingOperand,
                at,
                0,
            ));
        }
        if !super::parser::is_binary_operator(&token.text) {
            // A prefix-only operator (`~`, `!`, `not`) after a complete operand:
            // C reads it as the operand of an operator that never appeared
            // (`tclCompExpr.c:1053` — the `UNARY` case's `NotOperator` check).
            return Some(ExprSyntaxError::at(
                ExprSyntaxErrorKind::MissingOperator,
                at,
                0,
            ));
        }
        self.expecting = Expecting::Operand;
        None
    }

    fn paren_open(&mut self, at: usize) -> Option<ExprSyntaxError> {
        if self.expecting == Expecting::Operator {
            return Some(ExprSyntaxError::at(
                ExprSyntaxErrorKind::MissingOperator,
                at,
                0,
            ));
        }
        if self.token(self.pos).map(|t| t.kind) == Some(ExprTokenType::ParenClose) {
            // `()` is an argument list, never a subexpression — and the
            // argument-list form was consumed by `function_or_bareword`.
            return Some(ExprSyntaxError::at(
                ExprSyntaxErrorKind::EmptySubexpression,
                self.start_of(self.pos),
                0,
            ));
        }
        self.open.push(Group::Paren);
        None
    }

    fn paren_close(&mut self, at: usize) -> Option<ExprSyntaxError> {
        // A `?` still awaiting its `:` binds tighter than the paren it sits in.
        if self.open.last() == Some(&Group::Ternary) {
            return Some(ExprSyntaxError::at(
                ExprSyntaxErrorKind::MissingTernaryColon,
                at,
                0,
            ));
        }
        if self.expecting == Expecting::Operand {
            return Some(match self.previous_kind() {
                // A leading `)` closes nothing: C's `START` node does not
                // outrank `CLOSE_PAREN`, so the balance check reports it
                // (`tclCompExpr.c:1140` — parseExpr-21.20).
                None => ExprSyntaxError::naming(ExprSyntaxErrorKind::UnbalancedCloseParen, at, ")"),
                // `f(1,)` — the hole is a missing argument (parseExpr-21.18).
                Some(ExprTokenType::Comma) => {
                    ExprSyntaxError::at(ExprSyntaxErrorKind::MissingFunctionArgument, at, 0)
                }
                // `(1+)` / `f(1+)` — a dangling operator, wherever it sits.
                Some(_) => ExprSyntaxError::at(ExprSyntaxErrorKind::MissingOperand, at, 0),
            });
        }
        if self.open.pop().is_none() {
            return Some(ExprSyntaxError::naming(
                ExprSyntaxErrorKind::UnbalancedCloseParen,
                at,
                ")",
            ));
        }
        self.expecting = Expecting::Operator;
        None
    }

    fn comma(&mut self, at: usize) -> Option<ExprSyntaxError> {
        if self.open.last() != Some(&Group::ArgList) {
            return Some(ExprSyntaxError::naming(
                ExprSyntaxErrorKind::UnexpectedComma,
                at,
                ",",
            ));
        }
        if self.expecting == Expecting::Operand {
            // `a(,0)` — nothing at all before the comma is a missing argument
            // (parseExpr-21.21); `a(1+,0)` is a dangling operator instead, which
            // C reports as a missing operand (parseExpr-21.25).
            let kind = if self.previous_kind() == Some(ExprTokenType::ParenOpen) {
                ExprSyntaxErrorKind::MissingFunctionArgument
            } else {
                ExprSyntaxErrorKind::MissingOperand
            };
            return Some(ExprSyntaxError::at(kind, at, 0));
        }
        self.expecting = Expecting::Operand;
        None
    }

    fn ternary_question(&mut self, at: usize) -> Option<ExprSyntaxError> {
        if self.expecting == Expecting::Operand {
            return Some(ExprSyntaxError::at(
                ExprSyntaxErrorKind::MissingOperand,
                at,
                0,
            ));
        }
        self.open.push(Group::Ternary);
        self.expecting = Expecting::Operand;
        None
    }

    fn ternary_colon(&mut self, at: usize) -> Option<ExprSyntaxError> {
        if self.open.last() != Some(&Group::Ternary) {
            return Some(ExprSyntaxError::naming(
                ExprSyntaxErrorKind::UnexpectedColon,
                at,
                ":",
            ));
        }
        if self.expecting == Expecting::Operand {
            return Some(ExprSyntaxError::at(
                ExprSyntaxErrorKind::MissingOperand,
                at,
                0,
            ));
        }
        self.open.pop();
        self.expecting = Expecting::Operand;
        None
    }

    /// End of input: whatever is still open, or still expected, is the error.
    fn at_end(self) -> ExprSyntaxError {
        let end = self.source.len();
        let kind = match self.open.last() {
            Some(Group::Paren | Group::ArgList) => ExprSyntaxErrorKind::UnbalancedOpenParen,
            Some(Group::Ternary) => ExprSyntaxErrorKind::MissingTernaryColon,
            // A dangling operator, and the residual case: our parser rejects
            // sources this scan finds well-formed only when a *later* stage
            // (nesting depth, an operator spelling the AST has no node for)
            // gave up, and C reports every truncated expression this way.
            None => ExprSyntaxErrorKind::MissingOperand,
        };
        ExprSyntaxError::at(kind, end, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Diagnose and render in one step, as a runtime evaluator does.
    fn diagnose(source: &str) -> (String, String) {
        let error = ExprSyntaxError::diagnose(source, None);
        (error.message(source), error.error_code())
    }

    /// C's own expectations, transcribed from `tests/parseExpr.test` (Tcl
    /// 9.0.4). Every entry is a string `tclsh` prints verbatim, paired with the
    /// fourth word of the `-errorcode` it sets.
    fn c_suite_vectors() -> &'static [(&'static str, &'static str, &'static str)] {
        &[
            // parseExpr-21.19
            ("", "empty expression\nin expression \"\"", "EMPTY"),
            // parseExpr-21.1
            (
                "@",
                "invalid character \"@\"\nin expression \"@\"",
                "BADCHAR",
            ),
            // parseExpr-21.3
            (
                "x",
                "invalid bareword \"x\"\nin expression \"x\";\nshould be \"$x\" or \"{x}\" or \"x(...)\" or ...",
                "BAREWORD",
            ),
            // parseExpr-21.6
            (
                "0 0",
                "missing operator at _@_\nin expression \"0 _@_0\"",
                "MISSING",
            ),
            // parseExpr-21.15
            (
                "0~0",
                "missing operator at _@_\nin expression \"0_@_~0\"",
                "MISSING",
            ),
            // parseExpr-21.22
            (
                "0&|0",
                "missing operand at _@_\nin expression \"0&_@_|0\"",
                "MISSING",
            ),
            // parseExpr-21.16
            (
                "()",
                "empty subexpression at _@_\nin expression \"(_@_)\"",
                "EMPTY",
            ),
            // parseExpr-21.17
            (
                "(",
                "unbalanced open paren\nin expression \"(\"",
                "UNBALANCED",
            ),
            // parseExpr-21.26
            (
                "(0",
                "unbalanced open paren\nin expression \"(0\"",
                "UNBALANCED",
            ),
            // parseExpr-21.20
            (
                ")",
                "unbalanced close paren\nin expression \")\"",
                "UNBALANCED",
            ),
            // parseExpr-21.18
            (
                "a(0,)",
                "missing function argument at _@_\nin expression \"a(0,_@_)\"",
                "MISSING",
            ),
            // parseExpr-21.21
            (
                "a(,0)",
                "missing function argument at _@_\nin expression \"a(_@_,0)\"",
                "MISSING",
            ),
            // parseExpr-21.25
            (
                "a(1+,0)",
                "missing operand at _@_\nin expression \"a(1+_@_,0)\"",
                "MISSING",
            ),
            // parseExpr-21.27
            (
                "0?0",
                "missing operator \":\" at _@_\nin expression \"0?0_@_\"",
                "MISSING",
            ),
            // parseExpr-21.28
            (
                "0:0",
                "unexpected operator \":\" without preceding \"?\"\nin expression \"0:0\"",
                "SURPRISE",
            ),
            // parseExpr-21.29
            (
                "0)",
                "unbalanced close paren\nin expression \"0)\"",
                "UNBALANCED",
            ),
            // parseExpr-21.30
            (
                "0,",
                "unexpected \",\" outside function argument list\nin expression \"0,\"",
                "SURPRISE",
            ),
        ]
    }

    #[test]
    fn matches_the_c_suite_messages() {
        for (source, message, code) in c_suite_vectors() {
            let (got_message, got_code) = diagnose(source);
            assert_eq!(&got_message, message, "message for expr {{{source}}}");
            assert_eq!(
                got_code,
                format!("TCL PARSE EXPR {code}"),
                "code for {source}"
            );
        }
    }

    /// parseExpr-21.4: a bareword longer than the 25-byte window is elided at 22
    /// bytes in the message, the postscript, *and* the quoted context.
    #[test]
    fn a_long_bareword_elides_at_the_limit() {
        let (message, _) = diagnose("abcdefghijklmnopqrstuvwxyz");
        assert_eq!(
            message,
            "invalid bareword \"abcdefghijklmnopqrstuv...\"\n\
             in expression \"abcdefghijklmnopqrstuv...\";\n\
             should be \"$abcdefghijklmnopqrstuv...\" or \"{abcdefghijklmnopqrstuv...}\" \
             or \"abcdefghijklmnopqrstuv...(...)\" or ..."
        );
    }

    /// The head of the quoted context elides from the left, so a failure deep in
    /// a long expression is still shown in context.
    #[test]
    fn a_late_failure_elides_the_head() {
        let source = "1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 2";
        let (message, _) = diagnose(source);
        // 22 bytes of head (the 25-byte limit less the three the `...` costs),
        // taken from immediately before the failure position.
        assert_eq!(
            message,
            "missing operator at _@_\nin expression \"...1 + 1 + 1 + 1 + 1 + 1 _@_2\""
        );
    }

    /// The `errorInfo` frame quotes the whole expression, elided at the same
    /// limit (`tclCompExpr.c:1461-1466`).
    #[test]
    fn the_error_info_frame_quotes_the_expression() {
        assert_eq!(
            ExprSyntaxError::error_info_frame("1+"),
            "\n    (parsing expression \"1+\")"
        );
        assert_eq!(
            ExprSyntaxError::error_info_frame("abcdefghijklmnopqrstuvwxyz"),
            "\n    (parsing expression \"abcdefghijklmnopqrstuv...\")"
        );
    }

    /// A truncated expression — the shape the runtime meets most often.
    #[test]
    fn a_dangling_operator_is_a_missing_operand() {
        assert_eq!(
            diagnose("1+"),
            (
                "missing operand at _@_\nin expression \"1+_@_\"".to_owned(),
                "TCL PARSE EXPR MISSING".to_owned()
            )
        );
    }

    /// An iRules word operator is a plain bareword in a plain-Tcl grammar, which
    /// is exactly what C Tcl reports for it — the same diagnosis the dialect
    /// profile gives when the word *is* an operator but the release is too old
    /// (`tcl_registry::expr_surface`).
    #[test]
    fn a_word_operator_outside_its_dialect_is_a_bareword() {
        let (message, code) = diagnose("$x contains \"y\"");
        assert_eq!(
            message,
            "invalid bareword \"contains\"\nin expression \"$x contains \"y\"\";\n\
             should be \"$contains\" or \"{contains}\" or \"contains(...)\" or ..."
        );
        assert_eq!(code, "TCL PARSE EXPR BAREWORD");
        // Under the iRules dialect the same word *is* an operator, so the whole
        // expression parses and there is nothing to diagnose.
        assert!(!matches!(
            crate::expr::parse_expr("$x contains \"y\"", Some("f5-irules")),
            crate::expr::ExprNode::Raw { .. }
        ));
    }

    /// Whitespace-only input is C's empty expression, not a bareword.
    #[test]
    fn whitespace_only_is_an_empty_expression() {
        let (message, code) = diagnose("   ");
        assert_eq!(message, "empty expression\nin expression \"   \"");
        assert_eq!(code, "TCL PARSE EXPR EMPTY");
    }

    /// A multi-word bareword run names the *first* word, as C's left-to-right
    /// lexeme classification does.
    #[test]
    fn the_first_bareword_wins() {
        let (message, _) = diagnose("foo bar baz");
        assert!(
            message.starts_with("invalid bareword \"foo\"\nin expression \"foo bar baz\";"),
            "got: {message}"
        );
    }

    /// A non-ASCII byte the lexer skips is named without panicking on the
    /// mid-character offset its one-byte advance leaves behind.
    #[test]
    fn a_non_ascii_character_is_named_safely() {
        let (message, code) = diagnose("1 + \u{43f}");
        assert_eq!(code, "TCL PARSE EXPR BADCHAR");
        assert!(
            message.starts_with("invalid character \""),
            "got: {message}"
        );
    }

    // TIP 582 `#` comments

    /// A comment is never the reported fault: it is skipped like whitespace, so
    /// the diagnosis names whatever *else* is wrong — and `#` must never come
    /// back as `invalid character`.
    #[test]
    fn a_comment_is_never_the_reported_fault() {
        // A dangling operator followed by a comment: the fault is the missing
        // operand, at the end of the input, not the `#`.
        let (message, code) = diagnose("1 +#c");
        assert_eq!(
            message,
            "missing operand at _@_\nin expression \"1 +#c_@_\""
        );
        assert_eq!(code, "TCL PARSE EXPR MISSING");
        // Two operands with a comment between them: still `missing operator`,
        // marked at the second operand rather than at the comment.
        let (message, code) = diagnose("1 # c\n2");
        assert_eq!(
            message,
            "missing operator at _@_\nin expression \"1 # c\n_@_2\""
        );
        assert_eq!(code, "TCL PARSE EXPR MISSING");
        // A bareword is still a bareword when a comment trails it.
        let (message, code) = diagnose("foo # c");
        assert!(
            message.starts_with("invalid bareword \"foo\"\nin expression \"foo # c\";"),
            "got: {message}"
        );
        assert_eq!(code, "TCL PARSE EXPR BAREWORD");
        // No diagnosis anywhere should blame the `#` itself.
        for src in ["1 +#c", "1 # c\n2", "foo # c", "# c", "1 + 2 # ok"] {
            let (message, code) = diagnose(src);
            assert!(
                !message.contains("invalid character"),
                "{src:?} blamed a character: {message}"
            );
            assert_ne!(code, "TCL PARSE EXPR BADCHAR", "{src:?}");
        }
    }

    /// A comment-only body is C's empty expression: the comment is skipped, so
    /// `ParseExpr` reaches `END` with `lastParsed == START`
    /// (`tclCompExpr.c:1135`). There is no operand for it to be.
    #[test]
    fn a_comment_only_expression_is_the_empty_expression() {
        for src in ["# c", "#", "  # c", "# a\n# b"] {
            let (message, code) = diagnose(src);
            assert_eq!(
                message,
                format!("empty expression\nin expression \"{src}\""),
                "{src:?}"
            );
            assert_eq!(code, "TCL PARSE EXPR EMPTY", "{src:?}");
        }
    }

    /// C centres the quote on `start`, which for an all-skipped body sits at the
    /// *end* of the input — so once the comment passes the 25-byte window it is
    /// the head that elides, keeping the last 22 bytes behind a leading `...`
    /// (`tclCompExpr.c:1435-1444`). A comment is the first body long enough to
    /// make this observable.
    #[test]
    fn a_long_comment_only_expression_elides_from_the_left() {
        // 27 bytes: longer than `LIMIT`, so the head cannot be quoted whole.
        let src = "# 0123456789abcdefghijklmno";
        assert_eq!(src.len(), 27);
        let (message, code) = diagnose(src);
        assert_eq!(code, "TCL PARSE EXPR EMPTY");
        // Head elides from the left: `...` then the final `LIMIT - 3` == 22
        // bytes, i.e. from offset `27 + 3 - 25` == 5. C computes the same slice
        // as `limit - 3` bytes at `start - limit + 3`.
        assert_eq!(
            message,
            "empty expression\nin expression \"...3456789abcdefghijklmno\""
        );
        assert_eq!(src[5..].len(), LIMIT - 3);
        // The tail is empty — nothing follows the failure position, which sits
        // at the end of the input.
        assert!(message.ends_with("o\""), "got: {message}");
    }

    /// The gate: before 9.0 `#` really is an invalid expression character, and
    /// C 8.4-8.6 reports exactly that. The diagnoser must keep saying so for an
    /// 8.x dialect.
    #[test]
    fn before_tcl9_a_hash_is_still_an_invalid_character() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules"] {
            let error = ExprSyntaxError::diagnose("1 + 2 # note", Some(dialect));
            assert_eq!(
                error.message("1 + 2 # note"),
                "invalid character \"#\"\nin expression \"1 + 2 # note\"",
                "{dialect}"
            );
            assert_eq!(error.error_code(), "TCL PARSE EXPR BADCHAR", "{dialect}");
        }
    }
}
