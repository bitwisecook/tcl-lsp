//! Expression sub-lexer for Tcl `[expr]` bodies.
//!
//! Ports `core/parsing/expr_lexer.py` — a flat single-pass tokeniser
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
    /// Symbolic name matching the Python enum member names.
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
/// becomes `Function` regardless (matching the Python fallback).
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
    dialect: Option<&'s str>,
    unknown: bool,
}

impl<'s> Inner<'s> {
    fn new(s: &'s str, dialect: Option<&'s str>) -> Self {
        Self {
            b: s.as_bytes(),
            s,
            i: 0,
            dialect,
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

    fn run(&mut self) -> Vec<ExprToken> {
        let irops = irules_ops();
        let mut out = Vec::new();
        while self.i < self.b.len() {
            let ch = self.b[self.i];
            if matches!(ch, b' ' | b'\t' | b'\n' | b'\r') {
                let start = self.i;
                while self.i < self.b.len()
                    && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r')
                {
                    self.i += 1;
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
            while self.i < self.b.len()
                && (self.b[self.i].is_ascii_alphanumeric()
                    || self.b[self.i] == b'_'
                    || self.b[self.i] == b':')
            {
                self.i += 1;
            }
            if self.i < self.b.len() && self.b[self.i] == b'(' {
                self.i += 1;
                let mut lvl = 1u32;
                while self.i < self.b.len() && lvl > 0 {
                    if self.b[self.i] == b'(' {
                        lvl += 1;
                    } else if self.b[self.i] == b')' {
                        lvl -= 1;
                    }
                    self.i += 1;
                }
            }
        }
        self.tok(ExprTokenType::Variable, start)
    }

    fn command(&mut self) -> ExprToken {
        let start = self.i;
        self.i += 1;
        let mut lvl = 1u32;
        while self.i < self.b.len() && lvl > 0 {
            match self.b[self.i] {
                b'[' => lvl += 1,
                b']' => lvl -= 1,
                b'\\' => {
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
            if self.b[self.i] == b'\\' {
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
        let kind = if matches!(text, "true" | "false" | "yes" | "no" | "on" | "off") {
            ExprTokenType::Bool
        } else if self.dialect == Some("f5-irules") && irops.contains(text) {
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
            if self.b[self.i] == b'{' {
                lvl += 1;
            } else if self.b[self.i] == b'}' {
                lvl -= 1;
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
            if self.s[self.i..].starts_with(op) {
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
    }

    #[test]
    fn ieee_754_special_literals() {
        // Tcl 9.0 treats Inf/NaN as numeric literals, not function
        // names. See main commit ad81e67b for the parity fix.
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
    fn checked_no_unknown() {
        let (_, u) = tokenise_expr_checked("1 + 2", None);
        assert!(!u);
    }
}
