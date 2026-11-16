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

use crate::backslash_continuation_end;
use crate::ranges::{close_quote_offset_in, command_substitution_end_bytes_in};

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
pub struct ExprToken<Text = std::string::String> {
    /// Token kind.
    pub kind: ExprTokenType,
    /// The token's text (owned).
    pub text: Text,
    /// Byte offset of the first character.
    pub start: u32,
    /// Byte offset of the last character (inclusive).
    pub end: u32,
}

/// An original expression operand's lexical purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExprTermKind {
    /// A numeric candidate; native construction selects its numeric type.
    Number,
    /// A Boolean word, independently of numeric object construction.
    Boolean,
    /// A literal string operand.
    String,
    /// A quoted operand containing native substitution syntax.
    EscapedString,
    /// An original variable name.
    Variable,
    /// An original variable name and unsubstituted index.
    IndexedVariable,
    /// An original command body.
    Command,
}

/// Source-owned operand geometry, without constructing native objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprTerm {
    /// Original token extent, with an exclusive end.
    pub source: std::ops::Range<u32>,
    /// Original native operand spelling, without its source wrapper.
    pub value: std::ops::Range<u32>,
    /// Purpose selected by the shared token and variable scanners.
    pub kind: ExprTermKind,
    /// Raw LF count preceding the token, added to the native signed base line.
    pub line_delta: u32,
    /// Original Jim numeric token constructor; absent for other term kinds.
    pub jim_numeric_kind: Option<tcl_dialect::JimExpressionNumberKind>,
}

/// Project a complete token into its original native operand geometry.
/// Operators and recovery tokens cannot issue an operand receipt. Empty
/// operand bodies retain an empty range at their actual source position.
#[must_use]
pub fn expression_term(
    source: &[u8],
    token: &ExprToken<Vec<u8>>,
    config: crate::LexerConfig,
) -> Option<ExprTerm> {
    let line_delta =
        u32::try_from(bytecount::count(source.get(..token.start as usize)?, b'\n')).ok()?;
    expression_term_at_line(source, token, config, line_delta)
}

/// Project an ordered token stream's operands with one raw line scan.
/// All token geometry must still refer to the same original source.
#[must_use]
pub fn expression_terms(
    source: &[u8],
    tokens: &[ExprToken<Vec<u8>>],
    config: crate::LexerConfig,
) -> Option<Vec<ExprTerm>> {
    let mut line_delta = 0_u32;
    let mut previous = 0;
    let mut terms = Vec::new();
    for token in tokens {
        let start = token.start as usize;
        let skipped = source.get(previous..start)?;
        line_delta =
            line_delta.checked_add(u32::try_from(bytecount::count(skipped, b'\n')).ok()?)?;
        previous = start;
        if matches!(
            token.kind,
            ExprTokenType::Number
                | ExprTokenType::Bool
                | ExprTokenType::String
                | ExprTokenType::Variable
                | ExprTokenType::Command
        ) {
            terms.push(expression_term_at_line(source, token, config, line_delta)?);
        }
    }
    Some(terms)
}

fn expression_term_at_line(
    source: &[u8],
    token: &ExprToken<Vec<u8>>,
    config: crate::LexerConfig,
    line_delta: u32,
) -> Option<ExprTerm> {
    let start = token.start as usize;
    let end = token.end.checked_add(1)? as usize;
    let spelling = source.get(start..end)?;
    if spelling != token.text {
        return None;
    }
    let (kind, value) = expression_term_value(source, token.kind, start..end, config)?;
    Some(ExprTerm {
        source: token.start..token.end.checked_add(1)?,
        value: u32::try_from(value.start).ok()?..u32::try_from(value.end).ok()?,
        kind,
        line_delta,
        jim_numeric_kind: if kind == ExprTermKind::Number && config.var_syntax.has_expr_sugar() {
            let scanned = tcl_dialect::scan_jim_expression_number(
                source,
                token.start as usize,
                tcl_dialect::NumberSyntax::Jim,
            )?;
            if scanned.end() != end {
                return None;
            }
            scanned.jim_kind()
        } else {
            None
        },
    })
}

fn expression_term_value(
    source: &[u8],
    kind: ExprTokenType,
    range: std::ops::Range<usize>,
    config: crate::LexerConfig,
) -> Option<(ExprTermKind, std::ops::Range<usize>)> {
    match kind {
        ExprTokenType::Number => Some((ExprTermKind::Number, range)),
        ExprTokenType::Bool => Some((ExprTermKind::Boolean, range)),
        ExprTokenType::Variable => {
            let reference = crate::word_parts::scan_var_ref(source, range.start, config).ok()??;
            if reference.next != range.end {
                return None;
            }
            let start = (reference.name.as_ptr() as usize).checked_sub(source.as_ptr() as usize)?;
            let (kind, end) = if reference.index.is_some() {
                (ExprTermKind::IndexedVariable, reference.next)
            } else {
                (
                    ExprTermKind::Variable,
                    start.checked_add(reference.name.len())?,
                )
            };
            Some((kind, start..end))
        }
        ExprTokenType::Command | ExprTokenType::String => {
            expression_wrapped_term(source, kind, range)
        }
        _ => None,
    }
}

fn expression_wrapped_term(
    source: &[u8],
    kind: ExprTokenType,
    range: std::ops::Range<usize>,
) -> Option<(ExprTermKind, std::ops::Range<usize>)> {
    let spelling = source.get(range.clone())?;
    let (open, close) = match spelling.first()? {
        b'[' if kind == ExprTokenType::Command => (b'[', b']'),
        b'"' if kind == ExprTokenType::String => (b'"', b'"'),
        b'{' if kind == ExprTokenType::String => (b'{', b'}'),
        _ => return None,
    };
    if spelling.len() < 2 || spelling.last() != Some(&close) {
        return None;
    }
    let value = range.start + 1..range.end - 1;
    let kind = if open == b'[' {
        ExprTermKind::Command
    } else if open == b'"'
        && source[value.clone()]
            .iter()
            .any(|b| matches!(b, b'\\' | b'$' | b'['))
    {
        ExprTermKind::EscapedString
    } else {
        ExprTermKind::String
    };
    Some((kind, value))
}

#[inline]
fn p(n: usize) -> u32 {
    u32::try_from(n).expect("expression offset fits u32")
}

/// Whether an expression of the grammar `expr_grammar_base` names reads a
/// `$` with no variable name after it as the text `$`, as 8.4's
/// `Tcl_ParseExpr` does; from 8.5 it is an invalid character. A grammar that
/// names no base takes the newest.
#[must_use]
pub fn lone_dollar_is_text(expr_grammar_base: Option<tcl_dialect::TclVersion>) -> bool {
    expr_grammar_base.is_some_and(|base| base < tcl_dialect::TclVersion::V8_5)
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
    tokenise_expr_checked(source, dialect).0
}

/// Tokenise under an already-resolved dialect profile.
#[must_use]
pub fn tokenise_expr_for_profile(
    source: &str,
    profile: &tcl_dialect::DialectProfile,
) -> Vec<ExprToken> {
    let mut lex = Inner::new(source, profile);
    unicode_tokens(lex.run())
}

/// Tokenise from a compatibility name boundary and report skipped characters.
///
/// Internal typed callers should use [`tokenise_expr_checked_for_profile`].
#[must_use]
pub fn tokenise_expr_checked(source: &str, dialect: Option<&str>) -> (Vec<ExprToken>, bool) {
    match dialect.map(|name| (name, tcl_dialect::DialectProfile::find(name))) {
        None => tokenise_expr_checked_for_profile(source, tcl_dialect::DialectProfile::plain_tcl()),
        Some((_, Some(profile))) => tokenise_expr_checked_for_profile(source, profile),
        // A name with no catalogue row still names a grammar — `jim` — and
        // the central resolution is where it is found.
        Some((name, None)) => tokenise_expr_checked_with_grammar(
            source,
            &tcl_dialect::grammar_of_dialect_name(Some(name)),
        ),
    }
}

/// Tokenise under a bare grammar and report skipped characters — for a
/// caller that holds the document's grammar but no profile.
#[must_use]
pub fn tokenise_expr_checked_with_grammar(
    source: &str,
    grammar: &tcl_dialect::LexerGrammar,
) -> (Vec<ExprToken>, bool) {
    let mut lex = Inner::from_grammar(source.as_bytes(), grammar, None, None);
    let tokens = lex.run();
    (unicode_tokens(tokens), lex.unknown)
}

/// Tokenise with independent lexical, operator-release and host word-operator axes.
#[must_use]
pub fn tokenise_expr_checked_with_expression_grammar(
    source: &str,
    grammar: &tcl_dialect::LexerGrammar,
    expr_grammar_base: Option<tcl_dialect::TclVersion>,
    f5_word_grammar: Option<&'static tcl_dialect::model::ExprGrammar>,
) -> (Vec<ExprToken>, bool) {
    let mut lex = Inner::from_grammar(
        source.as_bytes(),
        grammar,
        expr_grammar_base,
        f5_word_grammar,
    );
    let tokens = lex.run();
    (unicode_tokens(tokens), lex.unknown)
}

/// Tokenise under an already-resolved profile and report skipped characters.
#[must_use]
pub fn tokenise_expr_checked_for_profile(
    source: &str,
    profile: &tcl_dialect::DialectProfile,
) -> (Vec<ExprToken>, bool) {
    let mut lex = Inner::new(source, profile);
    let tokens = lex.run();
    (unicode_tokens(tokens), lex.unknown)
}

/// A rejected original expression component from the shared scanner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprLexicalFailure {
    /// Original source byte offset.
    pub at: usize,
    /// Native lexical operation that could not consume the component.
    pub kind: ExprLexicalFailureKind,
}

/// The scanner's failure, independently of native error presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExprLexicalFailureKind {
    /// A byte does not begin an expression lexeme.
    InvalidCharacter,
    /// A dollar sign is not followed by a variable name.
    MissingVariableName,
    /// The shared variable scanner rejected a delimiter or index.
    Variable(&'static str),
    /// A bracketed script has no closing delimiter.
    MissingBracket,
    /// A quoted operand has no closing delimiter.
    MissingQuote,
    /// A braced operand has no closing delimiter.
    MissingBrace,
}

/// Complete lexical and expression axes retained by an authoring cursor.
/// This selects source grammar only, never a Native expression policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressionCursorGrammar {
    /// Full original word configuration, including caller overrides and bases.
    pub config: crate::LexerConfig,
    /// Independently selected expression comments and numeral grammar.
    pub grammar: tcl_dialect::LexerGrammar,
    /// Actual source release for expression operators.
    pub expr_grammar_base: Option<tcl_dialect::TclVersion>,
    /// Independently selected host expression word operators, when present.
    pub f5_word_grammar: Option<&'static tcl_dialect::model::ExprGrammar>,
}

/// Original expression-prefix geometry at a cursor. Incomplete source stays
/// incomplete; this supplies no parsed AST, result, evaluation or edit grant.
#[derive(Debug, Clone)]
pub struct OriginalExpressionCursor {
    image: crate::SourceImage,
    grammar: ExpressionCursorGrammar,
    region: crate::Span,
    prefix: crate::Span,
    expression_position: bool,
    failures: Vec<ExprLexicalFailure>,
}

impl OriginalExpressionCursor {
    /// Exact complete image/channel and all original configured grammar axes.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &crate::SourceImage,
        grammar: ExpressionCursorGrammar,
    ) -> bool {
        self.image == *image && self.grammar == grammar
    }

    /// Original selected expression operand content, including its real suffix.
    #[must_use]
    pub const fn region_span(&self) -> crate::Span {
        self.region
    }

    /// Actual source through the cursor; no closing delimiter is inserted.
    #[must_use]
    pub const fn prefix_span(&self) -> crate::Span {
        self.prefix
    }

    /// Whether this lexical prefix ends in expression space rather than inside
    /// a quoted/braced operand, script substitution, comment or rejected token.
    /// This proves no complete expression or actual function-call production.
    #[must_use]
    pub const fn is_expression_position(&self) -> bool {
        self.expression_position
    }

    /// Actual incomplete lexical issues, with offsets relative to the region.
    /// They are source observations, never a proved Native syntax failure.
    #[must_use]
    pub fn lexical_failures(&self) -> &[ExprLexicalFailure] {
        &self.failures
    }
}

/// Inspect one genuine expression region's original prefix with the same
/// configured lexer that owns complete expression tokens. Quotes, braces,
/// expression comments, nested scripts and variable syntax use its existing
/// scanners. Unknown characters and malformed variables withhold a receipt;
/// unfinished quote/brace/script tokens retain their real failure coordinates.
/// The caller independently owns the outer Expr role and original word.
#[must_use]
pub fn original_expression_cursor_in(
    image: &crate::SourceImage,
    region: crate::Span,
    cursor: u32,
    grammar: ExpressionCursorGrammar,
) -> Option<OriginalExpressionCursor> {
    if region.start() > cursor || cursor > region.end() {
        return None;
    }
    image.bytes().get(region.as_range())?;
    let prefix = crate::Span::new(region.start(), cursor);
    let source = image.bytes().get(prefix.as_range())?;
    let lexical = grammar.config.grammar_over(grammar.grammar);
    let mut lexer = Inner::from_grammar(
        source,
        &lexical,
        grammar.expr_grammar_base,
        grammar.f5_word_grammar,
    );
    // Preserve caller configuration for the existing variable, quote and
    // bracket scanners instead of reconstructing it from a profile label.
    lexer.variable_config = grammar.config;
    let tokens = lexer.run();
    if lexer.failures.iter().any(|failure| {
        !matches!(
            failure.kind,
            ExprLexicalFailureKind::MissingBracket
                | ExprLexicalFailureKind::MissingQuote
                | ExprLexicalFailureKind::MissingBrace
        )
    }) {
        return None;
    }
    let expression_position = lexer.failures.is_empty()
        && tokens
            .last()
            .is_none_or(|token| token.kind != ExprTokenType::Comment);
    Some(OriginalExpressionCursor {
        image: image.clone(),
        grammar,
        region,
        prefix,
        expression_position,
        failures: lexer.failures,
    })
}

/// Scan original byte source and retain every rejected component in source order.
#[must_use]
pub fn tokenise_expr_bytes_with_failures(
    source: &[u8],
    grammar: &tcl_dialect::LexerGrammar,
    expr_grammar_base: Option<tcl_dialect::TclVersion>,
    f5_word_grammar: Option<&'static tcl_dialect::model::ExprGrammar>,
) -> (Vec<ExprToken<Vec<u8>>>, Vec<ExprLexicalFailure>) {
    let mut lexer = Inner::from_grammar(source, grammar, expr_grammar_base, f5_word_grammar);
    let tokens = lexer.run();
    (tokens, lexer.failures)
}

/// Scan original expression bytes with the complete retained script configuration.
/// Comment/numeral/operator axes remain selected by the supplied expression
/// grammar; nested variables, quotes and scripts receive every caller knob.
#[must_use]
pub fn tokenise_expr_bytes_with_config_and_failures(
    source: &[u8],
    grammar: &tcl_dialect::LexerGrammar,
    expr_grammar_base: Option<tcl_dialect::TclVersion>,
    f5_word_grammar: Option<&'static tcl_dialect::model::ExprGrammar>,
    config: crate::LexerConfig,
) -> (Vec<ExprToken<Vec<u8>>>, Vec<ExprLexicalFailure>) {
    let mut lexer = Inner::from_grammar(
        source,
        &config.grammar_over(*grammar),
        expr_grammar_base,
        f5_word_grammar,
    );
    lexer.variable_config = config;
    let tokens = lexer.run();
    (tokens, lexer.failures)
}

/// Tokenise original native expression bytes without Unicode projection.
/// Offsets and token payloads refer to the unmodified input; quoted operands,
/// variable names and nested scripts retain arbitrary byte sequences.
#[must_use]
pub fn tokenise_expr_bytes_checked_with_expression_grammar(
    source: &[u8],
    grammar: &tcl_dialect::LexerGrammar,
    expr_grammar_base: Option<tcl_dialect::TclVersion>,
    f5_word_grammar: Option<&'static tcl_dialect::model::ExprGrammar>,
) -> (Vec<ExprToken<Vec<u8>>>, bool) {
    let mut lexer = Inner::from_grammar(source, grammar, expr_grammar_base, f5_word_grammar);
    let tokens = lexer.run();
    (tokens, lexer.unknown)
}

fn unicode_tokens(tokens: Vec<ExprToken<Vec<u8>>>) -> Vec<ExprToken> {
    tokens
        .into_iter()
        .map(|token| ExprToken {
            kind: token.kind,
            text: String::from_utf8(token.text).expect("Unicode source token boundary"),
            start: token.start,
            end: token.end,
        })
        .collect()
}

// Dialect-derived grammar knobs plus one output flag, not a state machine.
#[allow(clippy::struct_excessive_bools)]
struct Inner<'s> {
    b: &'s [u8],
    i: usize,
    /// The F5-family core `expr` grammar when the dialect's runtime core is
    /// on the F5 tree, else `None` — resolved once through the profile
    /// catalogue (so the `irules` / `tcl-irule` alias spellings behave like
    /// the canonical name). Gates the word-form operators
    /// (`starts_with`, `and`, …), which are an `f5-tcl` **trunk** fact —
    /// measured valid in tmsh and iApp `expr` too, not iRules-only
    /// (`docs/design/f5/bigip-irule-parser-measurements.md` §4a) — so the
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
    variable_config: crate::LexerConfig,
    /// The release's source-level array-index grammar. Kept with the other
    /// profile grammar facts so `expr` cannot accept an index the containing
    /// script lexer rejects (issue #1732).
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
    failures: Vec<ExprLexicalFailure>,
}

impl<'s> Inner<'s> {
    fn new(s: &'s str, profile: &tcl_dialect::DialectProfile) -> Self {
        Self::from_grammar(
            s.as_bytes(),
            &profile.grammar,
            profile.expr_grammar_base,
            profile.f5_core_expr_grammar(),
        )
    }

    /// The grammar-keyed constructor every other one reduces to: the four
    /// lexical axes the expression lexer reads all live on the
    /// [`tcl_dialect::LexerGrammar`], so a caller holding only a grammar
    /// (an environment with no catalogue profile) loses nothing.
    fn from_grammar(
        source: &'s [u8],
        grammar: &tcl_dialect::LexerGrammar,
        expr_grammar_base: Option<tcl_dialect::TclVersion>,
        f5_word_grammar: Option<&'static tcl_dialect::model::ExprGrammar>,
    ) -> Self {
        Self {
            b: source,
            i: 0,
            f5_word_grammar,
            expr_comments: grammar.expr_comments.comments(),
            numbers: grammar.numbers,
            variable_config: crate::LexerConfig::default().with_grammar(*grammar),
            expr_grammar_base,
            numeric_suffix_probe: false,
            unknown: false,
            failures: Vec::new(),
        }
    }

    fn reject(&mut self, at: usize, kind: ExprLexicalFailureKind) {
        self.unknown = true;
        self.failures.push(ExprLexicalFailure { at, kind });
    }

    fn tok(&self, kind: ExprTokenType, start: usize) -> ExprToken<Vec<u8>> {
        ExprToken {
            kind,
            text: self.b[start..self.i].to_vec(),
            start: p(start),
            end: if self.i > start {
                p(self.i - 1)
            } else {
                p(start)
            },
        }
    }

    fn single(&mut self, kind: ExprTokenType, text: &str) -> ExprToken<Vec<u8>> {
        let start = self.i;
        self.i += 1;
        ExprToken {
            kind,
            text: text.as_bytes().to_vec(),
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

    fn number(&self) -> Option<tcl_dialect::ExprNumberLexeme> {
        if matches!(
            self.numbers,
            tcl_dialect::NumberSyntax::Jim | tcl_dialect::NumberSyntax::Jim080
        ) {
            tcl_dialect::scan_jim_expression_number(self.b, self.i, self.numbers)
        } else {
            tcl_dialect::scan_expr_number(self.b, self.i, self.numbers, self.expr_grammar_base)
        }
    }

    fn run(&mut self) -> Vec<ExprToken<Vec<u8>>> {
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
            } else if let Some(number) = self.number() {
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
                    text: vec![self.b[self.i]],
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
                self.reject(self.i, ExprLexicalFailureKind::InvalidCharacter);
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
    fn comment(&mut self) -> ExprToken<Vec<u8>> {
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
    fn variable(&mut self) -> ExprToken<Vec<u8>> {
        let start = self.i;
        match crate::word_parts::scan_var_ref(self.b, start, self.variable_config) {
            Ok(Some(reference)) => self.i = reference.next,
            Ok(None) => {
                self.i += 1;
                // `Tcl_ParseVarName` reads a `$` with no name as text: 8.4's
                // expression keeps that text, while from 8.5 it is an invalid
                // character.
                if !lone_dollar_is_text(self.expr_grammar_base) {
                    self.reject(start, ExprLexicalFailureKind::MissingVariableName);
                }
            }
            Err(message) => {
                self.i = self.b.len();
                self.reject(start, ExprLexicalFailureKind::Variable(message));
            }
        }
        self.tok(ExprTokenType::Variable, start)
    }

    fn command(&mut self) -> ExprToken<Vec<u8>> {
        let start = self.i;
        if let Some(end) = command_substitution_end_bytes_in(self.b, start, self.variable_config) {
            self.i = end;
        } else {
            // Keep a recovery token for callers that need the incomplete
            // source slice, but make `tokenise_expr_checked`/`parse_expr`
            // reject it. A missing `]` cannot execute a command.
            self.i = self.b.len();
            self.reject(start, ExprLexicalFailureKind::MissingBracket);
        }
        self.tok(ExprTokenType::Command, start)
    }

    fn quoted(&mut self) -> ExprToken<Vec<u8>> {
        let start = self.i;
        if let Some(close) = close_quote_offset_in(self.b, start, self.variable_config) {
            self.i = close + 1;
        } else {
            // As with an unterminated command substitution, retain the
            // recovery token but reject the expression as non-executable.
            self.i = self.b.len();
            self.reject(start, ExprLexicalFailureKind::MissingQuote);
        }
        self.tok(ExprTokenType::String, start)
    }

    fn ident(&mut self) -> ExprToken<Vec<u8>> {
        let start = self.i;
        while self.i < self.b.len()
            && (self.b[self.i].is_ascii_alphanumeric() || self.b[self.i] == b'_')
        {
            self.i += 1;
        }
        let text =
            std::str::from_utf8(&self.b[start..self.i]).expect("identifier scanner consumes ASCII");
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
            text: text.as_bytes().to_vec(),
            start: p(start),
            end: p(self.i - 1),
        }
    }

    fn braced(&mut self) -> ExprToken<Vec<u8>> {
        let start = self.i;
        self.i += 1;
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
            self.reject(start, ExprLexicalFailureKind::MissingBrace);
            self.i = self.b.len();
            return ExprToken {
                kind: ExprTokenType::String,
                text: b"{".to_vec(),
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

    fn multi_op(&mut self, after_numeric_lexeme: bool) -> Option<ExprToken<Vec<u8>>> {
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
                    text: op.as_bytes().to_vec(),
                    start: p(start),
                    end: p(start + op.len() - 1),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod separator_boundary_tests;

#[cfg(test)]
mod original_expression_cursor_tests;
