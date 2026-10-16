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

//! Tcl 8.4's expression parser (`tclParseExpr.c`, `Tcl_ParseExpr`), for what
//! it refuses and with which message: 8.4 parses an expression with a
//! recursive-descent parser of its own whose syntax errors read `syntax error
//! in expression "…": <reason>`, which the 8.5 parser replaced. Its compiler
//! parses a literal expression this way as it compiles the command, so a
//! syntax error there fails the whole compile; the command substitutions the
//! expression holds are the scripts it compiles next.

/// Why 8.4's parser refuses an expression: the message `Tcl_ParseExpr`
/// leaves in the interpreter's result, and the error code a numeral it cannot
/// read sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tcl84ExprError {
    /// The error message.
    pub message: String,
    /// The `-errorcode`, where the parser sets one.
    pub error_code: Option<&'static str>,
}

/// What 8.4's parser makes of an expression: the command substitutions it
/// holds, in order (the scripts between `[` and `]`, as written), or why it
/// refuses the expression.
///
/// # Errors
///
/// The error `Tcl_ParseExpr` raises.
pub fn parse_tcl84(expr: &str) -> Result<Vec<String>, Tcl84ExprError> {
    let mut parser = Parser {
        src: expr.as_bytes(),
        next: 0,
        lexeme: Lexeme::Unknown,
        start: 0,
        commands: Vec::new(),
        error_code: None,
    };
    let parsed = parser.get_lexeme().and_then(|()| parser.cond_expr());
    let parsed = parsed.and_then(|()| {
        if parser.lexeme == Lexeme::End {
            Ok(())
        } else {
            Err(parser.syntax_error("extra tokens at end of expression"))
        }
    });
    match parsed {
        Ok(()) => Ok(parser.commands),
        Err(message) => Err(Tcl84ExprError {
            message,
            error_code: parser.error_code,
        }),
    }
}

/// `GetLexeme`'s token kinds.
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
    /// `lexemeStrings`, for `unexpected operator %s`.
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

struct Parser<'s> {
    src: &'s [u8],
    next: usize,
    lexeme: Lexeme,
    start: usize,
    commands: Vec<String>,
    /// The error code of a numeral the lexer could not read.
    error_code: Option<&'static str>,
}

impl Parser<'_> {
    /// `LogSyntaxError`: the whole expression, cut at 60 bytes, and the reason.
    fn syntax_error(&self, reason: &str) -> String {
        let text = if self.src.len() > 60 {
            format!("{}...", String::from_utf8_lossy(&self.src[..60]))
        } else {
            String::from_utf8_lossy(self.src).into_owned()
        };
        format!("syntax error in expression \"{text}\": {reason}")
    }

    fn byte(&self, at: usize) -> Option<u8> {
        self.src.get(at).copied()
    }

    fn get_lexeme(&mut self) -> Result<(), String> {
        let mut src = self.next;
        loop {
            src = skip_white_space(self.src, src);
            if self.byte(src) == Some(b'\n') {
                src += 1;
            } else {
                break;
            }
        }
        if src >= self.src.len() {
            self.lexeme = Lexeme::End;
            self.next = src;
            return Ok(());
        }
        let c = self.src[src];
        if c != b'+'
            && c != b'-'
            && let Some(lexeme) = tcl_dialect::scan_expr_number(
                self.src,
                src,
                tcl_dialect::NumberSyntax::Tcl84,
                Some(tcl_dialect::TclVersion::V8_4),
            )
            && lexeme.end() > src
        {
            if let Err((message, code)) =
                check_number(&String::from_utf8_lossy(&self.src[src..lexeme.end()]))
            {
                self.error_code = code;
                return Err(message);
            }
            self.lexeme = Lexeme::Literal;
            self.start = src;
            self.next = lexeme.end();
            return Ok(());
        }
        self.start = src;
        self.next = src + 1;
        let second = self.byte(src + 1);
        let pair = |lexeme, ok: bool, single| if ok { (lexeme, 2) } else { (single, 1) };
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
            b'<' => match second {
                Some(b'<') => (Lexeme::LeftShift, 2),
                Some(b'=') => (Lexeme::Leq, 2),
                _ => (Lexeme::Less, 1),
            },
            b'>' => match second {
                Some(b'>') => (Lexeme::RightShift, 2),
                Some(b'=') => (Lexeme::Geq, 2),
                _ => (Lexeme::Greater, 1),
            },
            b'=' => pair(Lexeme::Equal, second == Some(b'='), Lexeme::Unknown),
            b'!' => pair(Lexeme::Neq, second == Some(b'='), Lexeme::Not),
            b'&' => pair(Lexeme::And, second == Some(b'&'), Lexeme::BitAnd),
            b'^' => (Lexeme::BitXor, 1),
            b'|' => pair(Lexeme::Or, second == Some(b'|'), Lexeme::BitOr),
            b'~' => (Lexeme::BitNot, 1),
            b'e' if second == Some(b'q') => (Lexeme::StrEq, 2),
            b'n' if second == Some(b'e') => (Lexeme::StrNeq, 2),
            _ => {
                let rest = &self.src[src..];
                let first = utf8_char(rest);
                if first.is_some_and(|(ch, _)| (ch as u8).is_ascii_alphabetic()) {
                    let mut at = 0;
                    while let Some((ch, width)) = utf8_char(&rest[at..]) {
                        let low = ch as u8;
                        if !(low.is_ascii_alphanumeric() || low == b'_') {
                            break;
                        }
                        at += width;
                    }
                    (Lexeme::FuncName, at)
                } else {
                    (Lexeme::UnknownChar, 1)
                }
            }
        };
        self.lexeme = lexeme;
        self.next = src + size;
        Ok(())
    }

    fn cond_expr(&mut self) -> Result<(), String> {
        self.lor_expr()?;
        if self.lexeme == Lexeme::Questy {
            self.get_lexeme()?;
            self.cond_expr()?;
            if self.lexeme != Lexeme::Colon {
                return Err(self.syntax_error("missing colon from ternary conditional"));
            }
            self.get_lexeme()?;
            self.cond_expr()?;
        }
        Ok(())
    }

    /// One level of left-associative binary operators: `operand (op
    /// operand)*` for each lexeme in `ops`.
    fn binary(
        &mut self,
        ops: &[Lexeme],
        operand: fn(&mut Self) -> Result<(), String>,
    ) -> Result<(), String> {
        operand(self)?;
        while ops.contains(&self.lexeme) {
            self.get_lexeme()?;
            operand(self)?;
        }
        Ok(())
    }

    fn lor_expr(&mut self) -> Result<(), String> {
        self.binary(&[Lexeme::Or], Self::land_expr)
    }

    fn land_expr(&mut self) -> Result<(), String> {
        self.binary(&[Lexeme::And], Self::bit_or_expr)
    }

    fn bit_or_expr(&mut self) -> Result<(), String> {
        self.binary(&[Lexeme::BitOr], Self::bit_xor_expr)
    }

    fn bit_xor_expr(&mut self) -> Result<(), String> {
        self.binary(&[Lexeme::BitXor], Self::bit_and_expr)
    }

    fn bit_and_expr(&mut self) -> Result<(), String> {
        self.binary(&[Lexeme::BitAnd], Self::equality_expr)
    }

    fn equality_expr(&mut self) -> Result<(), String> {
        self.binary(
            &[Lexeme::Equal, Lexeme::Neq, Lexeme::StrEq, Lexeme::StrNeq],
            Self::relational_expr,
        )
    }

    fn relational_expr(&mut self) -> Result<(), String> {
        self.binary(
            &[Lexeme::Less, Lexeme::Greater, Lexeme::Leq, Lexeme::Geq],
            Self::shift_expr,
        )
    }

    fn shift_expr(&mut self) -> Result<(), String> {
        self.binary(&[Lexeme::LeftShift, Lexeme::RightShift], Self::add_expr)
    }

    fn add_expr(&mut self) -> Result<(), String> {
        self.binary(&[Lexeme::Plus, Lexeme::Minus], Self::multiply_expr)
    }

    fn multiply_expr(&mut self) -> Result<(), String> {
        self.binary(
            &[Lexeme::Mult, Lexeme::Divide, Lexeme::Mod],
            Self::unary_expr,
        )
    }

    fn unary_expr(&mut self) -> Result<(), String> {
        if matches!(
            self.lexeme,
            Lexeme::Plus | Lexeme::Minus | Lexeme::BitNot | Lexeme::Not
        ) {
            self.get_lexeme()?;
            return self.unary_expr();
        }
        self.primary_expr()
    }

    fn primary_expr(&mut self) -> Result<(), String> {
        match self.lexeme {
            Lexeme::OpenParen => {
                self.get_lexeme()?;
                self.cond_expr()?;
                if self.lexeme != Lexeme::CloseParen {
                    return Err(self.syntax_error("looking for close parenthesis"));
                }
                return self.get_lexeme();
            }
            Lexeme::Literal => {}
            Lexeme::Dollar => {
                self.next = parse_var_name(self.src, self.next - 1, &mut self.commands)?;
            }
            Lexeme::Quote => {
                self.next = parse_quoted(self.src, self.next, &mut self.commands)?;
            }
            Lexeme::OpenBracket => {
                self.next = parse_bracketed(self.src, self.next, &mut self.commands)?;
            }
            Lexeme::OpenBrace => {
                self.next = parse_braces(self.src, self.start)?;
            }
            // 8.4.20 reads no function named `eq` or `ne`: those lexemes are
            // operators, also at the start of a longer name.
            Lexeme::FuncName => {
                let name = String::from_utf8_lossy(&self.src[self.start..self.next]).into_owned();
                let (saved_next, saved_start, saved_lexeme) = (self.next, self.start, self.lexeme);
                self.get_lexeme()?;
                if self.lexeme == Lexeme::OpenParen {
                    self.get_lexeme()?;
                    while self.lexeme != Lexeme::CloseParen {
                        self.cond_expr()?;
                        if self.lexeme == Lexeme::Comma {
                            self.get_lexeme()?;
                        } else if self.lexeme != Lexeme::CloseParen {
                            return Err(self.syntax_error(
                                "missing close parenthesis at end of function call",
                            ));
                        }
                    }
                } else if crate::boolean::parse_boolean_word(&name).is_some() {
                    self.next = saved_next;
                    self.start = saved_start;
                    self.lexeme = saved_lexeme;
                } else if super::mathfunc::added_in(&name)
                    == Some(super::mathfunc::MathFuncSince::Tcl84)
                {
                    return Err(
                        self.syntax_error("expected parenthesis enclosing function arguments")
                    );
                } else {
                    return Err(self.syntax_error("variable references require preceding $"));
                }
            }
            Lexeme::Comma => {
                return Err(self.syntax_error("commas can only separate function arguments"));
            }
            Lexeme::End => return Err(self.syntax_error("premature end of expression")),
            Lexeme::Unknown => {
                return Err(self.syntax_error("single equality character not legal in expressions"));
            }
            Lexeme::UnknownChar => {
                return Err(self.syntax_error("character not legal in expressions"));
            }
            Lexeme::Questy => {
                return Err(self.syntax_error("unexpected ternary 'then' separator"));
            }
            Lexeme::Colon => {
                return Err(self.syntax_error("unexpected ternary 'else' separator"));
            }
            Lexeme::CloseParen => return Err(self.syntax_error("unexpected close parenthesis")),
            other => {
                return Err(self.syntax_error(&format!("unexpected operator {}", other.spelling())));
            }
        }
        self.get_lexeme()
    }
}

/// The next character of `bytes` and its width, as `Tcl_UtfToUniChar` reads
/// it; `None` at the end.
fn utf8_char(bytes: &[u8]) -> Option<(char, usize)> {
    let first = *bytes.first()?;
    let width = match first {
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    };
    std::str::from_utf8(bytes.get(..width)?)
        .ok()
        .and_then(|text| text.chars().next())
        .map_or(Some((char::from(first), 1)), |ch| Some((ch, width)))
}

/// `TclParseWhiteSpace`: spaces, tabs, vertical tabs, form feeds, carriage
/// returns and backslash-newlines, but not a newline itself.
fn skip_white_space(src: &[u8], mut at: usize) -> usize {
    loop {
        match src.get(at) {
            Some(b' ' | b'\t' | b'\x0b' | b'\x0c' | b'\r') => at += 1,
            Some(b'\\') if src.get(at + 1) == Some(&b'\n') => {
                at += 2;
                while matches!(src.get(at), Some(b' ' | b'\t')) {
                    at += 1;
                }
            }
            _ => return at,
        }
    }
}

/// What 8.4 makes of a numeral it reads in an expression: an integer it
/// cannot convert (`Tcl_GetWideIntFromObj`), a bad octal one or one past the
/// wide range, is an error there, as is a double past the double range or
/// one that rounds to zero from non-zero digits (`TclExprFloatError`), each
/// with its message and error code.
fn check_number(text: &str) -> Result<(), (String, Option<&'static str>)> {
    use super::errors;
    let flags = crate::number::ParseFlags::for_syntax(tcl_dialect::NumberSyntax::Tcl84);
    let coded = |message: &str, code: &'static str| Err((message.to_owned(), Some(code)));
    match crate::number::parse_whole_with(text, flags) {
        Some(crate::number::Number::Big { .. }) => {
            coded(errors::IOVERFLOW_MESSAGE, errors::IOVERFLOW_CODE)
        }
        None if errors::check_bad_octal(text, false) => Err((
            format!("expected integer but got \"{text}\" (looks like invalid octal number)"),
            None,
        )),
        // `strtod`'s `ERANGE`: an overflow, or a non-zero numeral that reads
        // as zero or a subnormal (`inf` reads without it).
        Some(crate::number::Number::Double(value))
            if crate::number::is_out_of_range_double(text, tcl_dialect::NumberSyntax::Tcl84) =>
        {
            let (message, code) = errors::float_range_error(value);
            coded(message, code)
        }
        _ => Ok(()),
    }
}

/// `Tcl_ParseVarName` from the `$` at `dollar`: the byte after the variable
/// reference, recording the command substitutions an array index holds.
fn parse_var_name(src: &[u8], dollar: usize, commands: &mut Vec<String>) -> Result<usize, String> {
    let mut at = dollar + 1;
    if at >= src.len() {
        return Ok(at);
    }
    if src[at] == b'{' {
        let close = src[at..].iter().position(|&byte| byte == b'}');
        return close.map_or_else(
            || Err("missing close-brace for variable name".to_owned()),
            |close| Ok(at + close + 1),
        );
    }
    while at < src.len() {
        let Some((ch, width)) = utf8_char(&src[at..]) else {
            break;
        };
        let low = ch as u8;
        if low.is_ascii_alphanumeric() || low == b'_' {
            at += width;
        } else if low == b':' && src.get(at + 1) == Some(&b':') {
            at += 2;
            while src.get(at) == Some(&b':') {
                at += 1;
            }
        } else {
            break;
        }
    }
    if src.get(at) == Some(&b'(') {
        let end = parse_tokens(src, at + 1, Some(b')'), commands)?;
        if end >= src.len() || src[end] != b')' {
            return Err("missing )".to_owned());
        }
        return Ok(end + 1);
    }
    Ok(at)
}

/// `ParseTokens` from `at` to the byte `term` (or the end): backslash
/// sequences, variable references and command substitutions, returning
/// where it stopped.
fn parse_tokens(
    src: &[u8],
    mut at: usize,
    term: Option<u8>,
    commands: &mut Vec<String>,
) -> Result<usize, String> {
    while at < src.len() {
        match src[at] {
            byte if Some(byte) == term => return Ok(at),
            b'$' => at = parse_var_name(src, at, commands)?,
            b'[' => at = parse_bracketed(src, at + 1, commands)?,
            b'\\' => at += backslash_length(&src[at..]),
            _ => at += 1,
        }
    }
    Ok(at)
}

/// The bytes a backslash sequence at the start of `src` spans.
fn backslash_length(src: &[u8]) -> usize {
    match src.get(1) {
        None => 1,
        Some(b'\n') => {
            2 + src[2..]
                .iter()
                .take_while(|byte| matches!(byte, b' ' | b'\t'))
                .count()
        }
        Some(b'x') => {
            2 + src[2..]
                .iter()
                .take_while(|byte| byte.is_ascii_hexdigit())
                .count()
        }
        Some(b'u') => {
            2 + src[2..]
                .iter()
                .take(4)
                .take_while(|byte| byte.is_ascii_hexdigit())
                .count()
        }
        Some(b'0'..=b'7') => {
            1 + src[1..]
                .iter()
                .take(3)
                .take_while(|byte| (b'0'..=b'7').contains(byte))
                .count()
        }
        Some(_) => 1 + utf8_char(&src[1..]).map_or(1, |(_, width)| width),
    }
}

/// `Tcl_ParseQuotedString` from just after the opening `"`: the byte after
/// the closing one.
fn parse_quoted(src: &[u8], at: usize, commands: &mut Vec<String>) -> Result<usize, String> {
    let end = parse_tokens(src, at, Some(b'"'), commands)?;
    if end >= src.len() {
        return Err("missing \"".to_owned());
    }
    Ok(end + 1)
}

/// `Tcl_ParseBraces` from the opening `{` at `open`: the byte after its
/// matching close, braces escaped by a backslash not counting.
fn parse_braces(src: &[u8], open: usize) -> Result<usize, String> {
    let mut depth = 1;
    let mut at = open + 1;
    while at < src.len() {
        match src[at] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(at + 1);
                }
            }
            b'\\' => {
                at += backslash_length(&src[at..]);
                continue;
            }
            _ => {}
        }
        at += 1;
    }
    Err("missing close-brace".to_owned())
}

/// The nested commands of a command substitution, from just after the `[`:
/// the byte after its closing `]`, recording the script.
fn parse_bracketed(src: &[u8], at: usize, commands: &mut Vec<String>) -> Result<usize, String> {
    let after = std::str::from_utf8(src)
        .ok()
        .and_then(|text| tcl_lexer::command_substitution_end(text, at - 1))
        .ok_or_else(|| "missing close-bracket".to_owned())?;
    commands.push(String::from_utf8_lossy(&src[at..after - 1]).into_owned());
    Ok(after)
}

#[cfg(test)]
mod tests {
    use super::parse_tcl84;

    /// The messages tclsh 8.4.20 gives `expr` of each expression.
    #[test]
    fn refuses_as_tcl_8_4_does() {
        for (expr, message) in [
            (
                "1 +",
                "syntax error in expression \"1 +\": premature end of expression",
            ),
            (
                "(1",
                "syntax error in expression \"(1\": looking for close parenthesis",
            ),
            (
                "1 2",
                "syntax error in expression \"1 2\": extra tokens at end of expression",
            ),
            (
                "foo",
                "syntax error in expression \"foo\": variable references require preceding $",
            ),
            (
                "sin 1",
                "syntax error in expression \"sin 1\": expected parenthesis enclosing function arguments",
            ),
            (
                "1 = 2",
                "syntax error in expression \"1 = 2\": extra tokens at end of expression",
            ),
            (
                "1 ? 2",
                "syntax error in expression \"1 ? 2\": missing colon from ternary conditional",
            ),
            (
                "1 * * 2",
                "syntax error in expression \"1 * * 2\": unexpected operator *",
            ),
            (
                ")",
                "syntax error in expression \")\": unexpected close parenthesis",
            ),
        ] {
            let error = parse_tcl84(expr).unwrap_err();
            assert_eq!(error.message, message, "{expr}");
            assert_eq!(error.error_code, None, "{expr}");
        }
        assert_eq!(parse_tcl84("true").unwrap(), Vec::<String>::new());
        assert_eq!(parse_tcl84("$a(x) + [set y] + {z}").unwrap(), ["set y"]);
        assert_eq!(parse_tcl84("sin(1) + 0x1f").unwrap(), Vec::<String>::new());
        assert_eq!(parse_tcl84("$").unwrap(), Vec::<String>::new());
    }

    /// The numerals tclsh 8.4.20 cannot read in `expr`, with the message and
    /// error code of each.
    #[test]
    fn refuses_a_numeral_it_cannot_read() {
        for (expr, message, code) in [
            (
                "08",
                "expected integer but got \"08\" (looks like invalid octal number)",
                None,
            ),
            (
                "1 + 99999999999999999999",
                "integer value too large to represent",
                Some("ARITH IOVERFLOW {integer value too large to represent}"),
            ),
            (
                "1e400",
                "floating-point value too large to represent",
                Some("ARITH OVERFLOW {floating-point value too large to represent}"),
            ),
            (
                "1e-400",
                "floating-point value too small to represent",
                Some("ARITH UNDERFLOW {floating-point value too small to represent}"),
            ),
        ] {
            let error = parse_tcl84(expr).unwrap_err();
            assert_eq!(
                (error.message.as_str(), error.error_code),
                (message, code),
                "{expr}"
            );
        }
    }
}
