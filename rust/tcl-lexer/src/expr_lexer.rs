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
            Self::Eof => "EOF",
        }
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
#[must_use]
pub fn math_functions() -> HashSet<&'static str> {
    [
        "abs", "acos", "asin", "atan", "atan2", "bool", "ceil", "cos", "cosh", "double", "entier",
        "exp", "floor", "fmod", "hypot", "int", "isinf", "isnan", "isqrt", "log", "log10", "max",
        "min", "pow", "rand", "round", "sin", "sinh", "sqrt", "srand", "tan", "tanh", "wide",
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

fn irules_ops() -> HashSet<&'static str> {
    [
        "and",
        "or",
        "not",
        "contains",
        "starts_with",
        "ends_with",
        "equals",
        "matches_glob",
        "matches_regex",
    ]
    .into_iter()
    .collect()
}

/// Tokenise a Tcl expression string.
#[must_use]
pub fn tokenise_expr(source: &str, dialect: Option<&str>) -> Vec<ExprToken> {
    let mut lex = Inner::new(source, dialect);
    lex.run()
}

/// Tokenise and report whether unknown characters were skipped.
#[must_use]
pub fn tokenise_expr_checked(source: &str, dialect: Option<&str>) -> (Vec<ExprToken>, bool) {
    let mut lex = Inner::new(source, dialect);
    let tokens = lex.run();
    (tokens, lex.unknown)
}

struct Inner<'s> {
    b: &'s [u8],
    s: &'s str,
    i: usize,
    /// Whether the dialect is iRules — resolved once through the profile
    /// catalog (so the `irules` / `tcl-irule` alias spellings behave like
    /// the canonical name), gating the iRules-only word operators.
    irules_operators: bool,
    unknown: bool,
}

impl<'s> Inner<'s> {
    fn new(s: &'s str, dialect: Option<&'s str>) -> Self {
        Self {
            b: s.as_bytes(),
            s,
            i: 0,
            irules_operators: tcl_dialect::DialectProfile::by_opt_name(dialect).is_irules(),
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
    fn is_backslash_nl(&self, i: usize) -> bool {
        self.b.get(i) == Some(&b'\\') && self.b.get(i + 1) == Some(&b'\n')
    }

    fn run(&mut self) -> Vec<ExprToken> {
        let irops = irules_ops();
        let mut out = Vec::new();
        while self.i < self.b.len() {
            let ch = self.b[self.i];
            if matches!(ch, b' ' | b'\t' | b'\n' | b'\r') || self.is_backslash_nl(self.i) {
                let start = self.i;
                while self.i < self.b.len() {
                    if matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
                        self.i += 1;
                    } else if self.is_backslash_nl(self.i) {
                        self.i += 2;
                    } else {
                        break;
                    }
                }
                out.push(self.tok(ExprTokenType::Whitespace, start));
            } else if ch.is_ascii_digit()
                || (ch == b'.' && self.i + 1 < self.b.len() && self.b[self.i + 1].is_ascii_digit())
            {
                out.push(self.number());
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
            } else if let Some(t) = self.multi_op() {
                out.push(t);
            } else if is_single_op(ch) {
                out.push(ExprToken {
                    kind: ExprTokenType::Operator,
                    text: self.s[self.i..=self.i].to_owned(),
                    start: p(self.i),
                    end: p(self.i),
                });
                self.i += 1;
            } else if ch.is_ascii_alphabetic() || ch == b'_' {
                out.push(self.ident(&irops));
            } else if ch == b'{' {
                out.push(self.braced());
            } else {
                self.unknown = true;
                self.i += 1;
            }
        }
        out
    }

    fn number(&mut self) -> ExprToken {
        let start = self.i;
        if self.b[self.i] == b'0'
            && self.i + 1 < self.b.len()
            && matches!(self.b[self.i + 1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B')
        {
            self.i += 2;
            while self.i < self.b.len()
                && (self.b[self.i].is_ascii_alphanumeric() || self.b[self.i] == b'_')
            {
                self.i += 1;
            }
            return self.tok(ExprTokenType::Number, start);
        }
        while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
            self.i += 1;
        }
        if self.i < self.b.len() && self.b[self.i] == b'.' {
            self.i += 1;
            while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                self.i += 1;
            }
        }
        if self.i < self.b.len() && matches!(self.b[self.i], b'e' | b'E') {
            let save = self.i;
            self.i += 1;
            if self.i < self.b.len() && matches!(self.b[self.i], b'+' | b'-') {
                self.i += 1;
            }
            if self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
                    self.i += 1;
                }
            } else {
                self.i = save;
            }
        }
        self.tok(ExprTokenType::Number, start)
    }

    fn variable(&mut self) -> ExprToken {
        let start = self.i;
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
            // A bare `$name` consumes alphanumerics / `_`, and `:` only as part
            // of a `::` namespace-separator pair — a *single* colon ends the
            // name, exactly like the main lexer's `parse_var`. Accepting a lone
            // `:` made `expr {$x>0?$y:$z}` lex `$y:` as one variable, swallowing
            // the ternary separator so the whole expr degraded to Raw
            // (RUST_ISSUE_086).
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
                self.scan_array_index();
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
    /// escaped `)` (`RUST_ISSUE_085`).
    fn scan_array_index(&mut self) {
        while self.i < self.b.len() {
            match self.b[self.i] {
                b')' => {
                    self.i += 1;
                    return;
                }
                b'\\' => self.i = (self.i + 2).min(self.b.len()),
                b'[' => {
                    self.i += 1;
                    let mut depth: u32 = 1;
                    while self.i < self.b.len() && depth > 0 {
                        match self.b[self.i] {
                            b'\\' => self.i = (self.i + 2).min(self.b.len()),
                            b'[' => {
                                depth += 1;
                                self.i += 1;
                            }
                            b']' => {
                                depth -= 1;
                                self.i += 1;
                            }
                            _ => self.i += 1,
                        }
                    }
                }
                b'$' => {
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
                            self.scan_array_index(); // nested index
                        }
                    }
                }
                _ => self.i += 1,
            }
        }
    }

    fn command(&mut self) -> ExprToken {
        let start = self.i;
        self.i += 1;
        let mut lvl = 1u32;
        while self.i < self.b.len() && lvl > 0 {
            match self.b[self.i] {
                b'[' => lvl += 1,
                b']' => lvl -= 1,
                // Skip the escaped byte, but only when one exists: a trailing
                // `\` at end-of-input must not combine with the unconditional
                // `self.i += 1` below to push `i` to `len + 1`, which then
                // panics the out-of-bounds `self.s[start..self.i]` slice in
                // `tok` (RUST_ISSUE_026).
                b'\\' if self.i + 1 < self.b.len() => {
                    self.i += 1;
                }
                _ => {}
            }
            self.i += 1;
        }
        self.tok(ExprTokenType::Command, start)
    }

    fn quoted(&mut self) -> ExprToken {
        let start = self.i;
        self.i += 1;
        while self.i < self.b.len() && self.b[self.i] != b'"' {
            // Skip the escaped byte only when it exists (a trailing `\` must not
            // push `i` past the end and panic `tok`'s slice — RUST_ISSUE_026).
            if self.b[self.i] == b'\\' && self.i + 1 < self.b.len() {
                self.i += 1;
            }
            self.i += 1;
        }
        if self.i < self.b.len() {
            self.i += 1;
        }
        self.tok(ExprTokenType::String, start)
    }

    fn ident(&mut self, irops: &HashSet<&str>) -> ExprToken {
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
        } else if self.irules_operators && irops.contains(text) {
            ExprTokenType::Operator
        } else if matches!(
            text,
            "Inf" | "inf" | "Infinity" | "infinity" | "NaN" | "nan"
        ) {
            // IEEE 754 special float literals — Tcl 9.0 recognises
            // these as numeric literals in expressions, not function
            // calls.
            ExprTokenType::Number
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

    fn multi_op(&mut self) -> Option<ExprToken> {
        for &op in MULTI_OPS {
            // Compare on bytes, not `self.s[self.i..]`: `self.i` is a byte index
            // that the byte-wise scanner can leave *inside* a multi-byte UTF-8
            // char (e.g. a non-ASCII variable name like `$itemés`), and string
            // slicing at a non-char-boundary panics. Every `MULTI_OPS` entry is
            // ASCII, so a byte-prefix check is exactly equivalent and total.
            if self.b[self.i..].starts_with(op.as_bytes()) {
                if matches!(op, "eq" | "ne" | "in" | "ni" | "lt" | "le" | "gt" | "ge") {
                    if self.i > 0 {
                        let prev = self.b[self.i - 1];
                        if prev.is_ascii_alphabetic() || prev == b'_' {
                            continue;
                        }
                    }
                    let after = self.i + op.len();
                    if after < self.b.len()
                        && (self.b[after].is_ascii_alphabetic() || self.b[after] == b'_')
                    {
                        continue;
                    }
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

    #[test]
    fn integer() {
        assert_eq!(texts("42"), vec!["42"]);
    }

    #[test]
    fn trailing_backslash_in_command_does_not_panic() {
        // RUST_ISSUE_026: a `[...` command substitution ending in a bare `\`
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
        // RUST_ISSUE_026: a `"...` string ending in a bare `\`.
        let toks = tokenise_expr(r#""a\"#, None);
        assert!(toks.iter().any(|t| t.kind == ExprTokenType::String));
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

    #[test]
    fn variables() {
        assert_eq!(texts("$x"), vec!["$x"]);
        assert_eq!(texts("${name}"), vec!["${name}"]);
        assert_eq!(texts("$arr(idx)"), vec!["$arr(idx)"]);
    }

    #[test]
    fn array_index_first_paren_terminates() {
        // RUST_ISSUE_085: no paren nesting — the index ends at the first `)`.
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

    #[test]
    fn single_colon_ends_variable_name() {
        // RUST_ISSUE_086: a lone `:` is not a name char — the ternary separator
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
        // Tcl 9.0 treats Inf/NaN as numeric literals, not function
        // names.
        for word in ["Inf", "inf", "Infinity", "infinity", "NaN", "nan"] {
            assert_eq!(
                types(word),
                vec![ExprTokenType::Number],
                "word `{word}` should tokenise as NUMBER",
            );
        }
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
}
