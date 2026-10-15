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

//! Tcl 8.4's expression parser (`tclParseExpr.c`), for its error reports.
//!
//! 8.4 parses an expression by recursive descent over its own lexemes and
//! words its failures differently from the operator-precedence parser 8.5
//! introduced: `syntax error in expression "1 +": premature end of
//! expression`, with the expression quoted whole (cut at 60 bytes with
//! `...`) and errorCode `NONE`. It also rejects what 8.4 lacks (`**`, `in`,
//! `0o17`) and reads its numeric literals itself, so an integer past 64
//! bits or a double out of range is an error before evaluation starts.
//!
//! [`check`] runs that parser and returns the first failure, or `None` for
//! an expression 8.4 accepts.

/// A failure of 8.4's expression parse: the interpreter result and the
/// errorCode C leaves (`NONE` unless a literal's conversion set one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassicExprError {
    /// The interpreter result.
    pub message: String,
    /// The `-errorcode`.
    pub error_code: String,
}

const NONE: &str = "NONE";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lexeme {
    Literal,
    FuncName,
    OpenBracket,
    OpenBrace,
    OpenParen,
    CloseParen,
    Dollar,
    Quote,
    Comma,
    End,
    Unknown,
    UnknownChar,
    Mult,
    Divide,
    Mod,
    Plus,
    Minus,
    LeftShift,
    RightShift,
    Less,
    Greater,
    Leq,
    Geq,
    Equal,
    Neq,
    BitAnd,
    BitXor,
    BitOr,
    And,
    Or,
    Questy,
    Colon,
    Not,
    BitNot,
    StrEq,
    StrNeq,
}

impl Lexeme {
    /// `lexemeStrings[]`, as `unexpected operator %s` prints it.
    fn spelling(self) -> &'static str {
        match self {
            Self::Literal => "LITERAL",
            Self::FuncName => "FUNCNAME",
            Self::OpenBracket => "[",
            Self::OpenBrace => "{",
            Self::OpenParen => "(",
            Self::CloseParen => ")",
            Self::Dollar => "$",
            Self::Quote => "\"",
            Self::Comma => ",",
            Self::End => "END",
            Self::Unknown => "UNKNOWN",
            Self::UnknownChar => "UNKNOWN_CHAR",
            Self::Mult => "*",
            Self::Divide => "/",
            Self::Mod => "%",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::Less => "<",
            Self::Greater => ">",
            Self::Leq => "<=",
            Self::Geq => ">=",
            Self::Equal => "==",
            Self::Neq => "!=",
            Self::BitAnd => "&",
            Self::BitXor => "^",
            Self::BitOr => "|",
            Self::And => "&&",
            Self::Or => "||",
            Self::Questy => "?",
            Self::Colon => ":",
            Self::Not => "!",
            Self::BitNot => "~",
            Self::StrEq => "eq",
            Self::StrNeq => "ne",
        }
    }
}

type Step = Result<(), ClassicExprError>;

struct Parser<'a> {
    src: &'a [u8],
    text: &'a str,
    lexeme: Lexeme,
    start: usize,
    size: usize,
    next: usize,
}

/// Parse `expr` as Tcl 8.4 does and report its first failure.
#[must_use]
pub fn check(expr: &str) -> Option<ClassicExprError> {
    let mut parser = Parser {
        src: expr.as_bytes(),
        text: expr,
        lexeme: Lexeme::Unknown,
        start: 0,
        size: 0,
        next: 0,
    };
    parser.parse().err()
}

impl Parser<'_> {
    fn parse(&mut self) -> Step {
        self.get_lexeme()?;
        self.cond()?;
        if self.lexeme != Lexeme::End {
            return Err(self.syntax("extra tokens at end of expression"));
        }
        Ok(())
    }

    /// `LogSyntaxError`: the whole expression, cut at 60 bytes.
    fn syntax(&self, info: &str) -> ClassicExprError {
        let quoted = if self.src.len() > 60 {
            format!("{}...", String::from_utf8_lossy(&self.src[..60]))
        } else {
            self.text.to_owned()
        };
        ClassicExprError {
            message: format!("syntax error in expression \"{quoted}\": {info}"),
            error_code: NONE.to_owned(),
        }
    }

    fn cond(&mut self) -> Step {
        self.binary(0)?;
        if self.lexeme == Lexeme::Questy {
            self.get_lexeme()?;
            self.cond()?;
            if self.lexeme != Lexeme::Colon {
                return Err(self.syntax("missing colon from ternary conditional"));
            }
            self.get_lexeme()?;
            self.cond()?;
        }
        Ok(())
    }

    /// The binary precedence levels, `ParseLorExpr` down to
    /// `ParseMultiplyExpr`.
    fn binary(&mut self, level: usize) -> Step {
        const LEVELS: &[&[Lexeme]] = &[
            &[Lexeme::Or],
            &[Lexeme::And],
            &[Lexeme::BitOr],
            &[Lexeme::BitXor],
            &[Lexeme::BitAnd],
            &[Lexeme::Equal, Lexeme::Neq, Lexeme::StrEq, Lexeme::StrNeq],
            &[Lexeme::Less, Lexeme::Greater, Lexeme::Leq, Lexeme::Geq],
            &[Lexeme::LeftShift, Lexeme::RightShift],
            &[Lexeme::Plus, Lexeme::Minus],
            &[Lexeme::Mult, Lexeme::Divide, Lexeme::Mod],
        ];
        let Some(operators) = LEVELS.get(level) else {
            return self.unary();
        };
        self.binary(level + 1)?;
        while operators.contains(&self.lexeme) {
            self.get_lexeme()?;
            self.binary(level + 1)?;
        }
        Ok(())
    }

    fn unary(&mut self) -> Step {
        if matches!(
            self.lexeme,
            Lexeme::Plus | Lexeme::Minus | Lexeme::BitNot | Lexeme::Not
        ) {
            self.get_lexeme()?;
            self.unary()
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Step {
        match self.lexeme {
            Lexeme::OpenParen => {
                self.get_lexeme()?;
                self.cond()?;
                if self.lexeme != Lexeme::CloseParen {
                    return Err(self.syntax("looking for close parenthesis"));
                }
                return self.get_lexeme();
            }
            Lexeme::Literal => {}
            Lexeme::Dollar => self.next = skip_var_name(self.src, self.next - 1)?,
            Lexeme::Quote => self.next = skip_quoted(self.text, self.start)?,
            Lexeme::OpenBracket => self.next = skip_command(self.src, self.start)?,
            Lexeme::OpenBrace => self.next = skip_braces(self.src, self.start)?,
            Lexeme::FuncName => {
                let (name_start, name_size) = (self.start, self.size);
                let saved = (self.lexeme, self.start, self.size, self.next);
                self.get_lexeme()?;
                if self.lexeme != Lexeme::OpenParen {
                    let name = &self.text[name_start..name_start + name_size];
                    if crate::boolean::parse_boolean_word(name).is_some() {
                        (self.lexeme, self.start, self.size, self.next) = saved;
                        return self.get_lexeme();
                    }
                    // 8.4 words the error by whether the name is in its
                    // fixed math-function table.
                    let in_table = super::mathfunc::added_in(name)
                        == Some(super::mathfunc::MathFuncSince::Tcl84);
                    return Err(self.syntax(if in_table {
                        "expected parenthesis enclosing function arguments"
                    } else {
                        "variable references require preceding $"
                    }));
                }
                self.get_lexeme()?;
                while self.lexeme != Lexeme::CloseParen {
                    self.cond()?;
                    if self.lexeme == Lexeme::Comma {
                        self.get_lexeme()?;
                    } else if self.lexeme != Lexeme::CloseParen {
                        return Err(
                            self.syntax("missing close parenthesis at end of function call")
                        );
                    }
                }
            }
            Lexeme::Comma => return Err(self.syntax("commas can only separate function arguments")),
            Lexeme::End => return Err(self.syntax("premature end of expression")),
            Lexeme::Unknown => {
                return Err(self.syntax("single equality character not legal in expressions"));
            }
            Lexeme::UnknownChar => return Err(self.syntax("character not legal in expressions")),
            Lexeme::Questy => return Err(self.syntax("unexpected ternary 'then' separator")),
            Lexeme::Colon => return Err(self.syntax("unexpected ternary 'else' separator")),
            Lexeme::CloseParen => return Err(self.syntax("unexpected close parenthesis")),
            other => {
                return Err(self.syntax(&format!("unexpected operator {}", other.spelling())));
            }
        }
        self.get_lexeme()
    }

    fn set(&mut self, lexeme: Lexeme, start: usize, size: usize) {
        self.lexeme = lexeme;
        self.start = start;
        self.size = size;
        self.next = start + size;
    }

    fn get_lexeme(&mut self) -> Step {
        let src = self.src;
        let mut at = self.next;
        while at < src.len() && is_space(src[at]) {
            at += 1;
        }
        if at >= src.len() {
            self.set(Lexeme::End, at, 0);
            return Ok(());
        }
        let c = src[at];
        if c != b'+' && c != b'-' {
            let length = parse_integer(&src[at..]);
            if length > 0 {
                read_integer(&self.text[at..at + length])?;
                self.set(Lexeme::Literal, at, length);
                return Ok(());
            }
            let length = max_double_length(&src[at..]);
            if length > 0
                && let Some(size) = read_double(&self.text[at..at + length])?
            {
                self.set(Lexeme::Literal, at, size);
                return Ok(());
            }
        }
        let two = |next: u8| src.get(at + 1) == Some(&next);
        let (lexeme, size) = match c {
            b'[' => (Lexeme::OpenBracket, 1),
            b'{' => (Lexeme::OpenBrace, 1),
            b'(' => (Lexeme::OpenParen, 1),
            b')' => (Lexeme::CloseParen, 1),
            b'$' => (Lexeme::Dollar, 1),
            b'"' => (Lexeme::Quote, 1),
            b',' => (Lexeme::Comma, 1),
            b'*' => (Lexeme::Mult, 1),
            b'/' => (Lexeme::Divide, 1),
            b'%' => (Lexeme::Mod, 1),
            b'+' => (Lexeme::Plus, 1),
            b'-' => (Lexeme::Minus, 1),
            b'?' => (Lexeme::Questy, 1),
            b':' => (Lexeme::Colon, 1),
            b'<' if two(b'<') => (Lexeme::LeftShift, 2),
            b'<' if two(b'=') => (Lexeme::Leq, 2),
            b'<' => (Lexeme::Less, 1),
            b'>' if two(b'>') => (Lexeme::RightShift, 2),
            b'>' if two(b'=') => (Lexeme::Geq, 2),
            b'>' => (Lexeme::Greater, 1),
            b'=' if two(b'=') => (Lexeme::Equal, 2),
            b'=' => (Lexeme::Unknown, 1),
            b'!' if two(b'=') => (Lexeme::Neq, 2),
            b'!' => (Lexeme::Not, 1),
            b'&' if two(b'&') => (Lexeme::And, 2),
            b'&' => (Lexeme::BitAnd, 1),
            b'^' => (Lexeme::BitXor, 1),
            b'|' if two(b'|') => (Lexeme::Or, 2),
            b'|' => (Lexeme::BitOr, 1),
            b'~' => (Lexeme::BitNot, 1),
            b'e' if two(b'q') => (Lexeme::StrEq, 2),
            b'n' if two(b'e') => (Lexeme::StrNeq, 2),
            c if c.is_ascii_alphabetic() => {
                let end = src[at..]
                    .iter()
                    .position(|&b| !(b.is_ascii_alphanumeric() || b == b'_'))
                    .map_or(src.len(), |n| at + n);
                (Lexeme::FuncName, end - at)
            }
            _ => (Lexeme::UnknownChar, 1),
        };
        self.set(lexeme, at, size);
        Ok(())
    }
}

/// `TclParseWhiteSpace` plus the newlines `GetLexeme` also skips.
fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// `TclParseInteger`: the length of a leading integer, `0x` hex included,
/// or 0 when the digits run into `.`, `e` or `E` (a double).
fn parse_integer(s: &[u8]) -> usize {
    if s.len() > 1 && s[0] == b'0' && matches!(s[1], b'x' | b'X') {
        let hex = s[2..].iter().take_while(|c| c.is_ascii_hexdigit()).count();
        return if hex > 0 { hex + 2 } else { 1 };
    }
    let digits = s.iter().take_while(|c| c.is_ascii_digit()).count();
    if matches!(s.get(digits), Some(b'.' | b'e' | b'E')) {
        0
    } else {
        digits
    }
}

/// `Tcl_GetWideIntFromObj` on an integer lexeme: octal with a leading 0,
/// wrapping at 64 bits as `strtoull` does, and an error past them.
fn read_integer(text: &str) -> Step {
    let flags = crate::number::ParseFlags {
        integer_only: true,
        ..crate::number::ParseFlags::for_syntax(crate::number::NumberSyntax::Tcl84)
    };
    match crate::number::parse_whole_with(text, flags) {
        Some(crate::number::Number::Int(_)) => Ok(()),
        Some(_) => Err(ClassicExprError {
            message: crate::expr::errors::IOVERFLOW_MESSAGE.to_owned(),
            error_code: crate::expr::errors::IOVERFLOW_CODE.to_owned(),
        }),
        // The lexeme is all digits, so a refusal is a leading-zero octal
        // with an 8 or 9 in it.
        None => Err(ClassicExprError {
            message: format!(
                "expected integer but got \"{text}\" (looks like invalid octal number)"
            ),
            error_code: NONE.to_owned(),
        }),
    }
}

/// `ParseMaxDoubleLength`: how many leading bytes could belong to a double.
fn max_double_length(s: &[u8]) -> usize {
    s.iter()
        .take_while(|c| b"0123456789ABCDEFINPXabcdefinpx.+-".contains(c))
        .count()
}

/// `strtod` over at most `text`: the length it consumes, or `None` when it
/// reads nothing; an out-of-range value is `TclExprFloatError`'s error.
fn read_double(text: &str) -> Result<Option<usize>, ClassicExprError> {
    let Some((value, consumed)) = strtod_prefix(text) else {
        return Ok(None);
    };
    if let Some(error) = float_range_error(value, &text[..consumed]) {
        return Err(error);
    }
    Ok(Some(consumed))
}

/// The `ERANGE` `strtod` raises for the decimal `spelling` it read as
/// `value`, as `TclExprFloatError` reports it: an overflow, or a subnormal
/// result, is "too large"; an underflow to zero is "too small".
#[must_use]
pub fn float_range_error(value: f64, spelling: &str) -> Option<ClassicExprError> {
    let lower = spelling.to_ascii_lowercase();
    let named = lower.contains("inf") || lower.contains("nan");
    let mantissa = lower.split('e').next().unwrap_or(&lower);
    let nonzero = mantissa.bytes().any(|b| (b'1'..=b'9').contains(&b));
    let erange = !named
        && (value.is_infinite()
            || (value != 0.0 && value.abs() < f64::MIN_POSITIVE)
            || (value == 0.0 && nonzero));
    erange.then(|| float_error(value))
}

/// `strtod`'s longest prefix of `text`: a decimal with an optional
/// exponent, or `inf`, `infinity` or `nan`; `None` when it reads nothing.
fn strtod_prefix(text: &str) -> Option<(f64, usize)> {
    let lower = text.to_ascii_lowercase();
    for (word, value) in [
        ("infinity", f64::INFINITY),
        ("inf", f64::INFINITY),
        ("nan", f64::NAN),
    ] {
        if lower.starts_with(word) {
            return Some((value, word.len()));
        }
    }
    let b = text.as_bytes();
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let int = digits(0);
    let mut end = int;
    let mut frac = 0;
    if b.get(end) == Some(&b'.') {
        frac = digits(end + 1);
        end += 1 + frac;
    }
    if int + frac == 0 {
        return None;
    }
    if matches!(b.get(end), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(b.get(end + 1), Some(b'+' | b'-')));
        let exp = digits(end + 1 + sign);
        if exp > 0 {
            end += 1 + sign + exp;
        }
    }
    text[..end].parse::<f64>().ok().map(|value| (value, end))
}

/// `TclExprFloatError` for a value C flagged: NaN is a domain error, zero
/// an underflow, anything else an overflow.
#[must_use]
pub fn float_error(value: f64) -> ClassicExprError {
    let (message, code) = if value.is_nan() {
        (
            crate::expr::errors::DOMAIN_MESSAGE,
            crate::expr::errors::DOMAIN_CODE.to_owned(),
        )
    } else if value == 0.0 {
        (
            "floating-point value too small to represent",
            "ARITH UNDERFLOW {floating-point value too small to represent}".to_owned(),
        )
    } else {
        (
            "floating-point value too large to represent",
            "ARITH OVERFLOW {floating-point value too large to represent}".to_owned(),
        )
    };
    ClassicExprError {
        message: message.to_owned(),
        error_code: code,
    }
}

fn simple_error(message: &str) -> ClassicExprError {
    ClassicExprError {
        message: message.to_owned(),
        error_code: NONE.to_owned(),
    }
}

/// 8.4's word grammar, which the lexer's scans read `${…}` and array
/// indices by.
fn lexer_config() -> tcl_lexer::LexerConfig {
    tcl_lexer::LexerConfig::for_dialect(tcl_dialect::TclVersion::V8_4.dialect_name())
}

/// `Tcl_ParseVarName` from the `$` at `at`: the end of the reference. A
/// `$` naming nothing is a literal `$`.
fn skip_var_name(src: &[u8], at: usize) -> Result<usize, ClassicExprError> {
    match tcl_lexer::scan_var_ref(src, at, lexer_config()) {
        Ok(Some(var)) => Ok(var.next),
        Ok(None) => Ok(at + 1),
        Err(message) => Err(simple_error(message)),
    }
}

/// `Tcl_ParseQuotedString` from the `"` at `at`.
fn skip_quoted(text: &str, at: usize) -> Result<usize, ClassicExprError> {
    tcl_lexer::quoted_word_close(text, at)
        .map(|close| close + 1)
        .map_err(simple_error)
}

/// `Tcl_ParseBraces` from the `{` at `at`.
fn skip_braces(src: &[u8], at: usize) -> Result<usize, ClassicExprError> {
    let mut depth = 0usize;
    let mut i = at;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err(simple_error("missing close-brace"))
}

/// The nested `Tcl_ParseCommand` loop from the `[` at `at`: the byte after
/// its `]`. The substituted text is a script, so its comments and words
/// decide where it ends.
fn skip_command(src: &[u8], at: usize) -> Result<usize, ClassicExprError> {
    tcl_lexer::command_subst_close(src, at, tcl_lexer::SubstFlags::default(), lexer_config())
        .map_err(simple_error)
}

#[cfg(test)]
mod tests {
    use super::check;

    /// Expressions 8.4 rejects, with the result and errorCode tclsh 8.4.20
    /// reports from `expr`.
    const FAILURES: &[(&str, &str, &str)] = &[
        (
            "1e999",
            "floating-point value too large to represent",
            "ARITH OVERFLOW {floating-point value too large to represent}",
        ),
        (
            "-1e999",
            "floating-point value too large to represent",
            "ARITH OVERFLOW {floating-point value too large to represent}",
        ),
        (
            "0x10000000000000000",
            "integer value too large to represent",
            "ARITH IOVERFLOW {integer value too large to represent}",
        ),
        (
            "18446744073709551616",
            "integer value too large to represent",
            "ARITH IOVERFLOW {integer value too large to represent}",
        ),
        (
            "01 + 08",
            r#"expected integer but got "08" (looks like invalid octal number)"#,
            "NONE",
        ),
        (
            "08",
            r#"expected integer but got "08" (looks like invalid octal number)"#,
            "NONE",
        ),
        (
            "0x",
            r#"syntax error in expression "0x": extra tokens at end of expression"#,
            "NONE",
        ),
        (
            "1e",
            r#"syntax error in expression "1e": extra tokens at end of expression"#,
            "NONE",
        ),
        (
            "1.5e+",
            r#"syntax error in expression "1.5e+": extra tokens at end of expression"#,
            "NONE",
        ),
        (
            "2 ** 3",
            r#"syntax error in expression "2 ** 3": unexpected operator *"#,
            "NONE",
        ),
        (
            r#""a" in {a b}"#,
            r#"syntax error in expression ""a" in {a b}": extra tokens at end of expression"#,
            "NONE",
        ),
        (
            "1 +",
            r#"syntax error in expression "1 +": premature end of expression"#,
            "NONE",
        ),
        (
            "(1",
            r#"syntax error in expression "(1": looking for close parenthesis"#,
            "NONE",
        ),
        (
            "1 2",
            r#"syntax error in expression "1 2": extra tokens at end of expression"#,
            "NONE",
        ),
        (
            "foo",
            r#"syntax error in expression "foo": variable references require preceding $"#,
            "NONE",
        ),
        (
            ",",
            r#"syntax error in expression ",": commas can only separate function arguments"#,
            "NONE",
        ),
        (
            "=",
            r#"syntax error in expression "=": single equality character not legal in expressions"#,
            "NONE",
        ),
        (
            "1 = 2",
            r#"syntax error in expression "1 = 2": extra tokens at end of expression"#,
            "NONE",
        ),
        (
            "#",
            r##"syntax error in expression "#": character not legal in expressions"##,
            "NONE",
        ),
        (
            "1 ? 2",
            r#"syntax error in expression "1 ? 2": missing colon from ternary conditional"#,
            "NONE",
        ),
        (
            ": 1",
            r#"syntax error in expression ": 1": unexpected ternary 'else' separator"#,
            "NONE",
        ),
        (
            ")",
            r#"syntax error in expression ")": unexpected close parenthesis"#,
            "NONE",
        ),
        (
            "1 eq",
            r#"syntax error in expression "1 eq": premature end of expression"#,
            "NONE",
        ),
        (
            "eq 1",
            r#"syntax error in expression "eq 1": unexpected operator eq"#,
            "NONE",
        ),
        ("[", "missing close-bracket", "NONE"),
        (r#""abc"#, r#"missing ""#, "NONE"),
        (
            "sin(1,",
            r#"syntax error in expression "sin(1,": premature end of expression"#,
            "NONE",
        ),
        (
            "sin 1",
            r#"syntax error in expression "sin 1": expected parenthesis enclosing function arguments"#,
            "NONE",
        ),
        (
            "sin(1 2)",
            r#"syntax error in expression "sin(1 2)": missing close parenthesis at end of function call"#,
            "NONE",
        ),
        (
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopq + ",
            r#"syntax error in expression "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefgh...": variable references require preceding $"#,
            "NONE",
        ),
    ];

    /// Each expression's failure as tclsh 8.4.20 reports it from `expr`.
    #[test]
    fn failures_read_as_tclsh_8_4_reports_them() {
        for &(expr, message, code) in FAILURES {
            let error = check(expr).unwrap_or_else(|| panic!("{expr:?} must fail"));
            assert_eq!(error.message, message, "{expr:?}");
            assert_eq!(error.error_code, code, "{expr:?}");
        }
    }

    /// What 8.4 accepts parses clean, including a lone `$`, boolean words
    /// and `eq`/`ne`.
    #[test]
    fn accepted_expressions_pass() {
        for expr in [
            "1 + 2",
            "$a eq {b}",
            "true",
            "sin(1) ne [f]",
            "$",
            "\"a\" == $b(c)",
            ".5",
            "5.",
            "0x1F",
        ] {
            assert_eq!(check(expr), None, "{expr:?}");
        }
    }

    /// A command substitution is a script: a `]` in its comment or inside
    /// one of its words does not close it (tclsh 8.4.20 evaluates each).
    #[test]
    fn substitutions_end_where_their_script_does() {
        for expr in [
            "[# ] comment\nexpr 1]",
            "[set a \"x]y\"] eq \"x]y\"",
            "\"a[# ]\nset x b]\" eq \"b\"",
            "$b([# ]\nset k 1]) == 1",
        ] {
            assert_eq!(check(expr), None, "{expr:?}");
        }
        for (expr, message) in [
            ("[set a", "missing close-bracket"),
            ("\"abc", "missing \""),
            ("$b(1", "missing )"),
            ("${a + 1", "missing close-brace for variable name"),
        ] {
            assert_eq!(
                check(expr).map(|e| e.message).as_deref(),
                Some(message),
                "{expr:?}"
            );
        }
    }
}
