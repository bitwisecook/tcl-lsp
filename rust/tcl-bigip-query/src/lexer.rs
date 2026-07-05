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

//! Tokeniser for the query DSL.
//!
//! The grammar is small enough that a hand-rolled scanner is shorter than the
//! regex it would replace. Tokens carry their offset so parse errors can
//! underline the offending character.
//!
//! Offsets are *code-point* indices (the source is scanned as a
//! `Vec<char>`), matching `source[i]` indexing exactly so a
//! non-ASCII string literal does not skew the reported positions.

use crate::ast::LitValue;
use crate::errors::QueryError;

/// The lexical category of a token.
///
/// Variant names match `TokenKind` enum members one-for-one so
/// the differential fixtures line up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Dot,
    LBracket,
    RBracket,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Semicolon,
    Pipe,
    PipeEq,
    Plus,
    PlusEq,
    Minus,
    MinusEq,
    Star,
    Slash,
    Eq,
    EqEq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    String,
    Number,
    Ident,
    And,
    Or,
    Not,
    True,
    False,
    Null,
    As,
    If,
    Then,
    Elif,
    Else,
    End,
    DollarIdent,
    Question,
    Eof,
}

impl TokenKind {
    /// The `TokenKind` member name, used by the differential test.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            TokenKind::Dot => "DOT",
            TokenKind::LBracket => "LBRACKET",
            TokenKind::RBracket => "RBRACKET",
            TokenKind::LParen => "LPAREN",
            TokenKind::RParen => "RPAREN",
            TokenKind::LBrace => "LBRACE",
            TokenKind::RBrace => "RBRACE",
            TokenKind::Comma => "COMMA",
            TokenKind::Colon => "COLON",
            TokenKind::Semicolon => "SEMICOLON",
            TokenKind::Pipe => "PIPE",
            TokenKind::PipeEq => "PIPE_EQ",
            TokenKind::Plus => "PLUS",
            TokenKind::PlusEq => "PLUS_EQ",
            TokenKind::Minus => "MINUS",
            TokenKind::MinusEq => "MINUS_EQ",
            TokenKind::Star => "STAR",
            TokenKind::Slash => "SLASH",
            TokenKind::Eq => "EQ",
            TokenKind::EqEq => "EQEQ",
            TokenKind::Neq => "NEQ",
            TokenKind::Lt => "LT",
            TokenKind::Le => "LE",
            TokenKind::Gt => "GT",
            TokenKind::Ge => "GE",
            TokenKind::String => "STRING",
            TokenKind::Number => "NUMBER",
            TokenKind::Ident => "IDENT",
            TokenKind::And => "AND",
            TokenKind::Or => "OR",
            TokenKind::Not => "NOT",
            TokenKind::True => "TRUE",
            TokenKind::False => "FALSE",
            TokenKind::Null => "NULL",
            TokenKind::As => "AS",
            TokenKind::If => "IF",
            TokenKind::Then => "THEN",
            TokenKind::Elif => "ELIF",
            TokenKind::Else => "ELSE",
            TokenKind::End => "END",
            TokenKind::DollarIdent => "DOLLAR_IDENT",
            TokenKind::Question => "QUESTION",
            TokenKind::Eof => "EOF",
        }
    }
}

/// A lexed token: its kind, source text, code-point offset, and — for
/// string / number / keyword / identifier tokens — the parsed value.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub offset: usize,
    /// The parsed value (`LitValue::Null` when the token carries none).
    pub value: LitValue,
}

fn keyword(text: &str) -> Option<TokenKind> {
    match text {
        "and" => Some(TokenKind::And),
        "or" => Some(TokenKind::Or),
        "not" => Some(TokenKind::Not),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        "null" => Some(TokenKind::Null),
        "as" => Some(TokenKind::As),
        "if" => Some(TokenKind::If),
        "then" => Some(TokenKind::Then),
        "elif" => Some(TokenKind::Elif),
        "else" => Some(TokenKind::Else),
        "end" => Some(TokenKind::End),
        _ => None,
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_ident_cont(ch: char) -> bool {
    // Hyphens are valid mid-identifier so TMSH-spelt names like
    // `data-group` lex as a single token. A leading hyphen is *not* an
    // identifier — it is the minus operator.
    ch.is_alphanumeric() || ch == '_' || ch == '-'
}

fn simple_punct(ch: char) -> Option<TokenKind> {
    match ch {
        '.' => Some(TokenKind::Dot),
        '[' => Some(TokenKind::LBracket),
        ']' => Some(TokenKind::RBracket),
        '(' => Some(TokenKind::LParen),
        ')' => Some(TokenKind::RParen),
        '{' => Some(TokenKind::LBrace),
        '}' => Some(TokenKind::RBrace),
        ',' => Some(TokenKind::Comma),
        ':' => Some(TokenKind::Colon),
        ';' => Some(TokenKind::Semicolon),
        '*' => Some(TokenKind::Star),
        '/' => Some(TokenKind::Slash),
        '?' => Some(TokenKind::Question),
        _ => None,
    }
}

fn escape(esc: char) -> char {
    match esc {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        '"' => '"',
        '\\' => '\\',
        other => other,
    }
}

/// Tokenise *source* into a token stream ending in an `Eof` token.
///
/// # Errors
///
/// Returns [`QueryError::Lex`] on an unterminated string, a stray `!` /
/// `$`, or any character the scanner does not understand.
#[allow(clippy::too_many_lines)]
pub fn tokenise(source: &str) -> Result<Vec<Token>, QueryError> {
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();
    let mut out: Vec<Token> = Vec::new();
    let mut i = 0usize;

    let slice = |a: usize, b: usize| -> String { chars[a..b].iter().collect() };

    while i < n {
        let ch = chars[i];

        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        // Comments: `# ...` to end of line.
        if ch == '#' {
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        let start = i;

        // Single-character punctuation that has no two-char form.
        if let Some(kind) = simple_punct(ch) {
            out.push(Token {
                kind,
                text: ch.to_string(),
                offset: start,
                value: LitValue::Null,
            });
            i += 1;
            continue;
        }

        // Two-char-or-one operators.
        if ch == '|' {
            if i + 1 < n && chars[i + 1] == '=' {
                out.push(Token {
                    kind: TokenKind::PipeEq,
                    text: "|=".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 2;
            } else {
                out.push(Token {
                    kind: TokenKind::Pipe,
                    text: "|".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 1;
            }
            continue;
        }

        if ch == '+' {
            if i + 1 < n && chars[i + 1] == '=' {
                out.push(Token {
                    kind: TokenKind::PlusEq,
                    text: "+=".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 2;
            } else {
                out.push(Token {
                    kind: TokenKind::Plus,
                    text: "+".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 1;
            }
            continue;
        }

        if ch == '-' {
            if i + 1 < n && chars[i + 1] == '=' {
                out.push(Token {
                    kind: TokenKind::MinusEq,
                    text: "-=".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 2;
            } else {
                out.push(Token {
                    kind: TokenKind::Minus,
                    text: "-".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 1;
            }
            continue;
        }

        if ch == '=' {
            if i + 1 < n && chars[i + 1] == '=' {
                out.push(Token {
                    kind: TokenKind::EqEq,
                    text: "==".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 2;
            } else {
                out.push(Token {
                    kind: TokenKind::Eq,
                    text: "=".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 1;
            }
            continue;
        }

        if ch == '!' {
            if i + 1 < n && chars[i + 1] == '=' {
                out.push(Token {
                    kind: TokenKind::Neq,
                    text: "!=".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 2;
                continue;
            }
            return Err(QueryError::Lex {
                message: "unexpected '!' (did you mean '!='?)".to_string(),
                offset: start,
            });
        }

        if ch == '<' {
            if i + 1 < n && chars[i + 1] == '=' {
                out.push(Token {
                    kind: TokenKind::Le,
                    text: "<=".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 2;
            } else {
                out.push(Token {
                    kind: TokenKind::Lt,
                    text: "<".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 1;
            }
            continue;
        }

        if ch == '>' {
            if i + 1 < n && chars[i + 1] == '=' {
                out.push(Token {
                    kind: TokenKind::Ge,
                    text: ">=".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 2;
            } else {
                out.push(Token {
                    kind: TokenKind::Gt,
                    text: ">".to_string(),
                    offset: start,
                    value: LitValue::Null,
                });
                i += 1;
            }
            continue;
        }

        // Variable references: `$name`.
        if ch == '$' {
            let mut j = i + 1;
            if j >= n || !is_ident_start(chars[j]) {
                return Err(QueryError::Lex {
                    message: "expected an identifier after '$' (e.g. $ltm)".to_string(),
                    offset: start,
                });
            }
            j += 1;
            while j < n && is_ident_cont(chars[j]) {
                j += 1;
            }
            let name = slice(i + 1, j);
            out.push(Token {
                kind: TokenKind::DollarIdent,
                text: slice(i, j),
                offset: start,
                value: LitValue::Str(name),
            });
            i = j;
            continue;
        }

        // Strings: double-quoted, with the usual backslash escapes.
        if ch == '"' {
            let mut j = i + 1;
            let mut buf = String::new();
            while j < n && chars[j] != '"' {
                if chars[j] == '\\' && j + 1 < n {
                    buf.push(escape(chars[j + 1]));
                    j += 2;
                    continue;
                }
                buf.push(chars[j]);
                j += 1;
            }
            if j >= n {
                return Err(QueryError::Lex {
                    message: "unterminated string literal".to_string(),
                    offset: start,
                });
            }
            j += 1; // consume closing quote
            out.push(Token {
                kind: TokenKind::String,
                text: slice(start, j),
                offset: start,
                value: LitValue::Str(buf),
            });
            i = j;
            continue;
        }

        // Numbers. Integer or float (no scientific notation in v1).
        if ch.is_ascii_digit() {
            let mut j = i;
            while j < n && chars[j].is_ascii_digit() {
                j += 1;
            }
            let value: LitValue =
                if j < n && chars[j] == '.' && j + 1 < n && chars[j + 1].is_ascii_digit() {
                    j += 1;
                    while j < n && chars[j].is_ascii_digit() {
                        j += 1;
                    }
                    let text = slice(i, j);
                    // A run of digits with a single dot is always a valid
                    // `f64` lexeme; out-of-range magnitudes saturate to
                    // ±inf rather than failing, so `parse` cannot error here.
                    // Guard it anyway so a future grammar change can never
                    // turn a malformed number into a panic.
                    match text.parse::<f64>() {
                        Ok(f) => LitValue::Float(f),
                        Err(_) => {
                            return Err(QueryError::Lex {
                                message: format!("invalid number literal {}", py_repr_str(&text)),
                                offset: start,
                            });
                        }
                    }
                } else {
                    let text = slice(i, j);
                    // The integer literal is untrusted user input: a digit run
                    // longer than `i64::MAX` (`99999999999999999999`) overflows
                    // the parse. Report a clean lex error instead of panicking
                    // — this DSL has no big-int model, so the value is simply
                    // out of range.
                    match text.parse::<i64>() {
                        Ok(int) => LitValue::Int(int),
                        Err(_) => {
                            return Err(QueryError::Lex {
                                message: format!(
                                    "integer literal {} is out of range for a 64-bit integer",
                                    py_repr_str(&text)
                                ),
                                offset: start,
                            });
                        }
                    }
                };
            out.push(Token {
                kind: TokenKind::Number,
                text: slice(i, j),
                offset: start,
                value,
            });
            i = j;
            continue;
        }

        // Identifiers / keywords.
        if is_ident_start(ch) {
            let mut j = i + 1;
            while j < n && is_ident_cont(chars[j]) {
                j += 1;
            }
            let text = slice(i, j);
            if let Some(kw) = keyword(&text) {
                let value = match kw {
                    TokenKind::True => LitValue::Bool(true),
                    TokenKind::False => LitValue::Bool(false),
                    _ => LitValue::Null,
                };
                out.push(Token {
                    kind: kw,
                    text,
                    offset: start,
                    value,
                });
            } else {
                out.push(Token {
                    kind: TokenKind::Ident,
                    text: text.clone(),
                    offset: start,
                    value: LitValue::Str(text),
                });
            }
            i = j;
            continue;
        }

        return Err(QueryError::Lex {
            message: format!("unexpected character {}", py_repr_str(&ch.to_string())),
            offset: start,
        });
    }

    out.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        offset: n,
        value: LitValue::Null,
    });
    Ok(out)
}

/// Render *s* the way `repr()` would for a `str` — the spelling
/// the DSL's error messages embed via `{value!r}`. Single-quoted by
/// default, switching to double quotes only when the string contains a
/// single quote but no double quote (the single-quote choice), with
/// backslash / newline / tab / carriage-return escaping.
///
/// Shared by the lexer (`unexpected character {ch!r}`) and the parser
/// (`unexpected token {tok.text!r}`, the `as`-form diagnostics, …).
pub(crate) fn py_repr_str(s: &str) -> String {
    let has_single = s.contains('\'');
    let has_double = s.contains('"');
    let quote = if has_single && !has_double { '"' } else { '\'' };
    let mut out = String::new();
    out.push(quote);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}
