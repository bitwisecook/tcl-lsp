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

//! Expression-numeral lexeme boundaries.
//!
//! This is the lower half of C Tcl's `ParseLexeme`: it determines where a
//! numeric-looking expression lexeme ends, but deliberately does **not** turn
//! that spelling into a value. The latter is `tcl_syntax::number`'s
//! `TclParseNumber` port. Keeping this below both `tcl-lexer` and `tcl-syntax`
//! prevents either layer from growing a second boundary scanner.
//!
//! Tcl draws an unusual boundary after a partially parsed number. A trailing
//! bareword run normally makes the entire thing one bareword (`12x`, `1_eq`),
//! except when the numeric run contains `.` / an exponent sign, or the suffix
//! starts a release-available word operator (`1eq 2`). The source is
//! `ParseLexeme` in `tcl8.5.19/generic/tclCompExpr.c:1923-1980`,
//! `tcl8.6.16/generic/tclCompExpr.c:2016-2073`, and
//! `tcl9.0.4/generic/tclCompExpr.c:2078-2125`; Tcl 8.4 has the predecessor in
//! `tcl8.4.20/generic/tclParseExpr.c`. The scanner intentionally mirrors that
//! *boundary* only. Invalid radix digits and unavailable prefixes remain one
//! candidate lexeme for the parser to diagnose as barewords.

use crate::{NumberSyntax, TclVersion, is_expr_word_operator};

/// One numeric-looking expression lexeme's exclusive ending byte offset.
///
/// This is a byte offset into the source passed to [`scan_expr_number`]. Tcl's
/// expression grammar is byte-oriented here (`TclIsBareword` is ASCII), so an
/// offset is the correct boundary even beside non-ASCII source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprNumberLexeme {
    end: usize,
    jim_kind: Option<JimExpressionNumberKind>,
}

/// The selected Jim numeric token constructor, independent of object getters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JimExpressionNumberKind {
    /// The original scanner selected the native integer constructor.
    Integer,
    /// The original scanner selected the native double constructor.
    Double,
}

/// The complete, decoded payload of a `NaN(...)` spelling.
///
/// Tcl's `TclParseNumber` accepts ASCII whitespace between its one through
/// thirteen hexadecimal digits. It deliberately stops accepting after the
/// thirteenth digit: a fourteenth digit makes the parenthesised form invalid,
/// rather than truncating the payload. This owner is shared with
/// `tcl_syntax::number` so lexing and value parsing cannot disagree about that
/// boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NanPayloadLexeme {
    end: usize,
    value: u64,
}

impl NanPayloadLexeme {
    /// The exclusive end offset, immediately after the closing `)`.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// The hexadecimal payload value, with whitespace omitted.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.value
    }
}

impl ExprNumberLexeme {
    /// Jim token kind selected while scanning the original numeral.
    /// Other grammars cannot issue this constructor receipt.
    #[must_use]
    pub const fn jim_kind(self) -> Option<JimExpressionNumberKind> {
        self.jim_kind
    }

    /// The exclusive end offset of this lexeme in its source byte slice.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }
}

/// Scan a numeric-looking expression lexeme beginning at `start`.
///
/// `None` means the source begins a regular bareword, not a numeric lexeme.
/// A successful result is deliberately not a validity verdict: `0o8`, `0x`,
/// and `0d99` under Tcl 8.6 still produce one candidate lexeme, and
/// `tcl_syntax::number` decides that they are invalid. `numbers` controls the
/// fractional/exponent `_` boundary; `expr_grammar_base` controls the sole
/// word-operator exception to bareword joining. Pass a profile's
/// `grammar.numbers` and `expr_grammar_base` together.
///
/// The special floating spellings are case-insensitive, as Tcl's
/// `TclParseNumber` is: `Inf`, `Infinity`, `NaN`, and 8.5+'s
/// `NaN(hex-payload)` (one through thirteen digits, with whitespace allowed).
#[must_use]
pub fn scan_expr_number(
    source: &[u8],
    start: usize,
    numbers: NumberSyntax,
    expr_grammar_base: Option<TclVersion>,
) -> Option<ExprNumberLexeme> {
    let first = *source.get(start)?;
    let end = if numbers == NumberSyntax::Tcl84 {
        scan_tcl84_number(source, start, first)?
    } else if first.is_ascii_digit()
        || (first == b'.' && source.get(start + 1).is_some_and(u8::is_ascii_digit))
    {
        scan_digit_number(source, start, numbers, expr_grammar_base)
    } else if matches!(numbers, NumberSyntax::Jim | NumberSyntax::Jim080) {
        scan_jim_special_float(source, start, expr_grammar_base)?
    } else {
        scan_special_float(source, start, expr_grammar_base, true, true)?
    };
    let jim_kind = matches!(numbers, NumberSyntax::Jim | NumberSyntax::Jim080)
        .then(|| jim_number_kind(source, start, end, numbers));
    Some(ExprNumberLexeme { end, jim_kind })
}

/// Scan an original Jim numeric token using its native integer-first frontier.
/// The native scanner can stop before a following bareword or invalid radix
/// suffix. The returned constructor kind belongs to this exact token extent.
/// This is distinct from C-style whole-candidate numeric/bareword joining.
#[must_use]
pub fn scan_jim_expression_number(
    source: &[u8],
    start: usize,
    numbers: NumberSyntax,
) -> Option<ExprNumberLexeme> {
    if !matches!(numbers, NumberSyntax::Jim | NumberSyntax::Jim080) {
        return None;
    }
    let first = *source.get(start)?;
    if !first.is_ascii_digit() && first != b'.' {
        let end = jim_irrational_end(source, start)?;
        return Some(ExprNumberLexeme {
            end,
            jim_kind: Some(JimExpressionNumberKind::Double),
        });
    }
    let integer_end = if first == b'0' {
        scan_explicit_radix_number(source, start, numbers)
            .unwrap_or_else(|| decimal_integer_end(source, start))
    } else {
        decimal_integer_end(source, start)
    };
    let double = integer_end == start
        || source
            .get(integer_end)
            .is_some_and(|byte| matches!(byte, b'e' | b'E' | b'N' | b'n' | b'I' | b'i' | b'.'));
    if double
        && let Some(end) = jim_double_end(source, start)
        && end > integer_end
    {
        return Some(ExprNumberLexeme {
            end,
            jim_kind: Some(JimExpressionNumberKind::Double),
        });
    }
    (integer_end != start).then_some(ExprNumberLexeme {
        end: integer_end,
        jim_kind: Some(JimExpressionNumberKind::Integer),
    })
}

fn decimal_integer_end(source: &[u8], start: usize) -> usize {
    let mut end = start;
    while source.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    end
}

fn jim_double_end(source: &[u8], start: usize) -> Option<usize> {
    if source
        .get(start..start + 2)
        .is_some_and(|bytes| matches!(bytes, b"0x" | b"0X"))
    {
        let mut end = start + 2;
        let mut digits = 0_usize;
        while source.get(end).is_some_and(u8::is_ascii_hexdigit) {
            end += 1;
            digits += 1;
        }
        if source.get(end) == Some(&b'.') {
            end += 1;
            while source.get(end).is_some_and(u8::is_ascii_hexdigit) {
                end += 1;
                digits += 1;
            }
        }
        if digits != 0 {
            return Some(float_exponent_end(source, end, b'p', b'P'));
        }
    }
    let end = scan_tcl84_decimal(source, start);
    source
        .get(start..end)
        .filter(|bytes| bytes.iter().any(u8::is_ascii_digit))
        .map(|_| end)
}

fn float_exponent_end(source: &[u8], at: usize, lower: u8, upper: u8) -> usize {
    if !source
        .get(at)
        .is_some_and(|&byte| byte == lower || byte == upper)
    {
        return at;
    }
    let mut end = at + 1;
    if source
        .get(end)
        .is_some_and(|byte| matches!(byte, b'+' | b'-'))
    {
        end += 1;
    }
    let digits = end;
    end = decimal_integer_end(source, end);
    if end == digits { at } else { end }
}

fn jim_irrational_end(source: &[u8], start: usize) -> Option<usize> {
    const SPELLINGS: [&[u8]; 6] = [b"Inf", b"inf", b"INF", b"NaN", b"nan", b"NAN"];
    SPELLINGS.into_iter().find_map(|spelling| {
        source
            .get(start..start + spelling.len())
            .is_some_and(|bytes| bytes == spelling)
            .then_some(start + spelling.len())
    })
}

fn jim_number_kind(
    source: &[u8],
    start: usize,
    end: usize,
    numbers: NumberSyntax,
) -> JimExpressionNumberKind {
    let integer_end = if source.get(start) == Some(&b'0')
        && source
            .get(start + 1)
            .and_then(|byte| numbers.explicit_radix(*byte))
            .is_some()
    {
        scan_explicit_radix_number(source, start, numbers).unwrap_or(start)
    } else {
        let mut cursor = start;
        while source.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        cursor
    };
    if integer_end < end
        && (integer_end == start
            || source
                .get(integer_end)
                .is_some_and(|byte| matches!(byte, b'e' | b'E' | b'N' | b'n' | b'I' | b'i' | b'.')))
    {
        JimExpressionNumberKind::Double
    } else {
        JimExpressionNumberKind::Integer
    }
}

/// Parse C Tcl's `NaN(hexdigits)` payload beginning at its opening `(`.
///
/// This mirrors the `sNANPAREN`/`sNANHEX` states in
/// `tcl8.5.19/generic/tclStrToD.c:1032-1060`,
/// `tcl8.6.17/generic/tclStrToD.c:1117-1145`, and
/// `tcl9.0.4/generic/tclStrToD.c:1178-1206`. It is intentionally unavailable
/// to the 8.4 `GetLexeme` path, which delegates special floats to the platform
/// `strtod` and has no parenthesised payload grammar.
#[must_use]
pub fn scan_nan_payload(source: &[u8], open_paren: usize) -> Option<NanPayloadLexeme> {
    if source.get(open_paren) != Some(&b'(') {
        return None;
    }
    let mut cursor = open_paren + 1;
    let mut digits = 0usize;
    let mut value = 0u64;
    while let Some(&byte) = source.get(cursor) {
        if byte.is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if byte == b')' {
            return (digits != 0).then_some(NanPayloadLexeme {
                end: cursor + 1,
                value,
            });
        }
        let digit = byte.to_ascii_lowercase();
        let value_digit = match digit {
            b'0'..=b'9' => u64::from(digit - b'0'),
            b'a'..=b'f' => u64::from(digit - b'a' + 10),
            _ => return None,
        };
        if digits == 13 {
            return None;
        }
        value = (value << 4) | value_digit;
        digits += 1;
        cursor += 1;
    }
    None
}

/// C's `TclIsBareword` for the ASCII bytes that matter at a numeric junction.
#[must_use]
pub const fn is_expr_bareword_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Whether `op` has C Tcl's word-operator lexical boundary at `at`.
///
/// `ParseLexeme` protects the right side with `!isalpha`, rather than with
/// `!TclIsBareword`: `eq2` is consequently `eq` plus `2`, while `eqx` is one
/// bareword. The preceding guard falls out of C's preceding bareword scan and
/// is explicit here so callers can ask about an arbitrary offset.
#[must_use]
pub fn expr_word_operator_boundary_ok(source: &[u8], op: &str, at: usize) -> bool {
    if !expr_word_operator_right_boundary_ok(source, op, at) {
        return false;
    }
    if source
        .get(at.wrapping_sub(1))
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
    {
        return false;
    }
    true
}

/// Whether `op` has C Tcl's trailing word-operator boundary at `at`.
///
/// `ParseLexeme(end, …)` recursively probes the suffix after a number. In
/// that call `end` is the *start* of a new lexeme, so the byte before it must
/// not participate: `Infinityeq` is `Infinity eq`, even though `y` precedes
/// `eq` in the enclosing source. Call [`expr_word_operator_boundary_ok`] when
/// asking about a word operator in the outer source stream instead.
#[must_use]
pub fn expr_word_operator_right_boundary_ok(source: &[u8], op: &str, at: usize) -> bool {
    if at > source.len()
        || !source
            .get(at..)
            .is_some_and(|tail| tail.starts_with(op.as_bytes()))
    {
        return false;
    }
    !source
        .get(at + op.len())
        .is_some_and(u8::is_ascii_alphabetic)
}

/// The release-available binary word operator beginning at `at`, if any.
///
/// This is the `ParseLexeme(end, …)` probe the numeral scanner needs before it
/// decides whether `1eq` is `1` followed by `eq`, or one bareword. Only the
/// word-shaped binary operators participate; punctuation operators cannot be
/// swallowed by a bareword run in the first place.
#[must_use]
pub fn expr_binary_word_operator_at(
    source: &[u8],
    at: usize,
    expr_grammar_base: Option<TclVersion>,
) -> Option<&'static str> {
    crate::EXPR_WORD_OPERATORS.iter().find_map(|&(op, _)| {
        (is_expr_word_operator(op, expr_grammar_base)
            && expr_word_operator_right_boundary_ok(source, op, at))
        .then_some(op)
    })
}

fn scan_digit_number(
    source: &[u8],
    start: usize,
    numbers: NumberSyntax,
    expr_grammar_base: Option<TclVersion>,
) -> usize {
    let mut end = start;

    // C first lets TclParseNumber inspect a radix spelling. A valid prefix plus
    // at least one valid digit has a real numeric end, which must be offered to
    // the recursive word-operator probe (`0b1ne`, `0d9lt`). An unavailable
    // prefix or invalid first digit has no numeric end, so it stays one greedy
    // bareword candidate (`0o8`, `0d99` before 9.0, `0xg`). Prefix availability
    // is the NumberSyntax grammar owner, never a lexer-local table.
    if source.get(start) == Some(&b'0')
        && source.get(start + 1).is_some_and(|byte| {
            matches!(byte, b'x' | b'X' | b'o' | b'O' | b'b' | b'B' | b'd' | b'D')
        })
    {
        if let Some(end) = scan_explicit_radix_number(source, start, numbers) {
            return join_trailing_bareword(source, start, end, expr_grammar_base, true);
        }
        end = start + 2;
        while source
            .get(end)
            .is_some_and(|byte| is_expr_bareword_byte(*byte))
        {
            end += 1;
        }
        return end;
    }

    while source
        .get(end)
        .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'_')
    {
        end += 1;
    }
    if source.get(end) == Some(&b'.') {
        end += 1;
        while source.get(end).is_some_and(|byte| {
            byte.is_ascii_digit() || (numbers.allows_digit_separators() && *byte == b'_')
        }) {
            end += 1;
        }
    }
    if source
        .get(end)
        .is_some_and(|byte| matches!(byte, b'e' | b'E'))
    {
        let exponent = end;
        end += 1;
        if source
            .get(end)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
        {
            end += 1;
        }
        if source.get(end).is_some_and(u8::is_ascii_digit) {
            while source.get(end).is_some_and(|byte| {
                byte.is_ascii_digit() || (numbers.allows_digit_separators() && *byte == b'_')
            }) {
                end += 1;
            }
        } else {
            end = exponent;
        }
    }
    join_trailing_bareword(source, start, end, expr_grammar_base, false)
}

/// Scan an available explicit-radix numeral through its last valid digit.
///
/// `None` deliberately means the candidate has no valid numeral at all, not
/// merely that it has trailing junk. This preserves C Tcl's whole-bareword
/// diagnostic for `0o8`/`0xg`; a successful prefix can instead let
/// `join_trailing_bareword` distinguish `0b1ne` from `0b1x`.
fn scan_explicit_radix_number(source: &[u8], start: usize, numbers: NumberSyntax) -> Option<usize> {
    let radix = numbers.explicit_radix(*source.get(start + 1)?)?;
    let mut end = start + 2;
    let mut digits = 0usize;
    while let Some(&byte) = source.get(end) {
        if let Some(value) = ascii_digit_value(byte) {
            if value >= radix {
                break;
            }
            digits += 1;
            end += 1;
            continue;
        }
        if numbers.allows_digit_separators()
            && byte == b'_'
            && digits != 0
            && source
                .get(end + 1)
                .and_then(|next| ascii_digit_value(*next))
                .is_some_and(|value| value < radix)
        {
            end += 1;
            continue;
        }
        break;
    }
    (digits != 0).then_some(end)
}

fn ascii_digit_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Tcl 8.4's pre-`TclParseNumber` expression scanner.
///
/// `GetLexeme` first uses `TclParseInteger`, then falls back to `strtod`; it
/// never performs the 8.5+ `ParseLexeme` bareword rescan. Consequently `1_eq`
/// is a literal `1` followed by invalid `_`, `12x` is literal `12` then `x`,
/// and a `0x` prefix stops after its valid hex digits (`0x1p2` is `0x1`, then
/// `p2`). See `tcl8.4.20/generic/tclParseExpr.c:1592-1645` and
/// `:1898-1928`.
fn scan_tcl84_number(source: &[u8], start: usize, first: u8) -> Option<usize> {
    if first.is_ascii_digit() {
        if source
            .get(start + 1)
            .is_some_and(|byte| matches!(byte, b'x' | b'X'))
        {
            let mut end = start + 2;
            while source.get(end).is_some_and(u8::is_ascii_hexdigit) {
                end += 1;
            }
            return Some(if end == start + 2 { start + 1 } else { end });
        }
        return Some(scan_tcl84_decimal(source, start));
    }
    if first == b'.' && source.get(start + 1).is_some_and(u8::is_ascii_digit) {
        return Some(scan_tcl84_decimal(source, start));
    }
    scan_special_float(source, start, None, false, false)
}

fn scan_tcl84_decimal(source: &[u8], start: usize) -> usize {
    let mut end = start;
    while source.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let needs_double = source
        .get(end)
        .is_some_and(|byte| matches!(byte, b'.' | b'e' | b'E'))
        || source.get(start) == Some(&b'.');
    if !needs_double {
        return end;
    }
    if source.get(end) == Some(&b'.') {
        end += 1;
        while source.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
    }
    if source
        .get(end)
        .is_some_and(|byte| matches!(byte, b'e' | b'E'))
    {
        let exponent = end;
        end += 1;
        if source
            .get(end)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
        {
            end += 1;
        }
        let digits = end;
        while source.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end == digits {
            return exponent;
        }
    }
    end
}

/// `JimTcl`'s special-float spellings, which are a strict and differently
/// cased subset of C Tcl's.
///
/// Jim's `JimParseExprNumber` recognises exactly `Inf`, `inf`, `INF`, `NaN`,
/// `nan` and `NAN` — six literal spellings, not a case-insensitive match —
/// and has no `Infinity` form and no `NaN(payload)` grammar at all.
///
/// | input | Jim | C Tcl 8.5+ |
/// |---|---|---|
/// | `Inf` | infinity | infinity |
/// | `InF` | bareword | infinity |
/// | `Infinity` | **syntax error** | infinity |
/// | `NaN(1)` | **syntax error** | NaN with payload |
///
/// Measured on jimsh 0.84: `expr {Infinity}` is
/// `syntax error in expression: "Infinity"` where tclsh 9.0 answers `Inf`.
fn scan_jim_special_float(
    source: &[u8],
    start: usize,
    expr_grammar_base: Option<TclVersion>,
) -> Option<usize> {
    let end = jim_irrational_end(source, start)?;
    if source
        .get(end)
        .is_some_and(|byte| is_expr_bareword_byte(*byte))
    {
        expr_binary_word_operator_at(source, end, expr_grammar_base)?;
    }
    Some(end)
}

fn scan_special_float(
    source: &[u8],
    start: usize,
    expr_grammar_base: Option<TclVersion>,
    nan_payload: bool,
    modern_bareword_junction: bool,
) -> Option<usize> {
    let (end, completed_nan_payload) = if starts_with_ignore_ascii_case(source, start, b"inf") {
        let after_inf = start + 3;
        if starts_with_ignore_ascii_case(source, after_inf, b"inity") {
            (after_inf + 5, false)
        } else {
            (after_inf, false)
        }
    } else if starts_with_ignore_ascii_case(source, start, b"nan") {
        if nan_payload {
            if let Some(payload) = scan_nan_payload(source, start + 3) {
                (payload.end(), true)
            } else {
                (start + 3, false)
            }
        } else {
            (start + 3, false)
        }
    } else {
        return None;
    };

    if modern_bareword_junction
        && !completed_nan_payload
        && source
            .get(end)
            .is_some_and(|byte| is_expr_bareword_byte(*byte))
    {
        // A special float is a double, but it has no `.` / exponent sign to
        // force C down the number path. It therefore follows exactly the same
        // bareword-vs-following-operator rule as an integer.
        expr_binary_word_operator_at(source, end, expr_grammar_base)?;
    }
    Some(end)
}

fn join_trailing_bareword(
    source: &[u8],
    start: usize,
    end: usize,
    expr_grammar_base: Option<TclVersion>,
    restarted_at_suffix: bool,
) -> usize {
    if !source
        .get(end)
        .is_some_and(|byte| is_expr_bareword_byte(*byte))
        || !source[start..end]
            .iter()
            .copied()
            .all(is_expr_bareword_byte)
        || expr_binary_word_operator_at(source, end, expr_grammar_base).is_some_and(|op| {
            // C restarts ParseLexeme after a successful explicit-radix parse.
            // That probe sees only the suffix's right boundary: a preceding
            // hexadecimal `f` belongs to the finished number, so `0xfne` is
            // `0xf ne`. Other numeric scans still inspect the enclosing
            // bareword boundary (`1_eq` remains one candidate).
            restarted_at_suffix || expr_word_operator_boundary_ok(source, op, end)
        })
    {
        return end;
    }
    let mut end = end;
    while source
        .get(end)
        .is_some_and(|byte| is_expr_bareword_byte(*byte))
    {
        end += 1;
    }
    end
}

fn starts_with_ignore_ascii_case(source: &[u8], at: usize, word: &[u8]) -> bool {
    source
        .get(at..at.saturating_add(word.len()))
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(source: &str, numbers: NumberSyntax, base: Option<TclVersion>) -> Option<&str> {
        scan_expr_number(source.as_bytes(), 0, numbers, base).map(|lexeme| &source[..lexeme.end()])
    }

    /// Boundary rows are source-derived from Tcl's `ParseLexeme`: 8.4 uses
    /// `tclParseExpr.c`; 8.5/8.6 use `tclCompExpr.c`; 9.0/9.1 use the same
    /// routine with `TclParseNumber`. Oracle runs on tclsh 8.6.17 and 9.0.3:
    /// `expr {1_eq}` reports one bareword, `expr {1eq 2}` returns 0,
    /// `expr {1.0_2}` is BADCHAR on 8.6 and returns 1.02 on 9.0.
    #[test]
    fn number_bareword_junctions_match_tcl_parselexeme() {
        for (numbers, base) in [
            (NumberSyntax::Tcl85, Some(TclVersion::V8_5)),
            (NumberSyntax::Tcl90, Some(TclVersion::V9_0)),
        ] {
            for source in ["1_eq", "12x", "1e_0", "1_2_eq"] {
                assert_eq!(scan(source, numbers, base), Some(source), "{source}");
            }
            for (source, start) in [("0 + 1_eq", 4), ("0 + 12x", 4)] {
                assert_eq!(
                    scan_expr_number(source.as_bytes(), start, numbers, base)
                        .map(|lexeme| &source[start..lexeme.end()]),
                    Some(&source[start..]),
                    "{source}"
                );
            }
            assert_eq!(scan("1.5abc", numbers, base), Some("1.5"));
            assert_eq!(scan("1eq 2", numbers, base), Some("1"));
        }
    }

    /// Tcl 8.4's `GetLexeme` predates the 8.5 `ParseLexeme` bareword rescan.
    /// `tcl8.4.20/generic/tclParseExpr.c:1592-1645` first accepts the prefix
    /// returned by `TclParseInteger` (`:1898-1928`), so each of these is two
    /// lexemes rather than one modern bareword. Mutating the 8.4 branch to use
    /// `join_trailing_bareword` reverses every row.
    #[test]
    fn tcl84_getlexeme_keeps_integer_prefixes_separate() {
        let syntax = NumberSyntax::Tcl84;
        let base = Some(TclVersion::V8_4);
        assert_eq!(scan("1_eq", syntax, base), Some("1"));
        assert_eq!(scan("12x", syntax, base), Some("12"));
        assert_eq!(scan("0x1p2", syntax, base), Some("0x1"));
        assert_eq!(scan("0xg", syntax, base), Some("0"));
        assert_eq!(scan("1.5abc", syntax, base), Some("1.5"));
    }

    /// `_` after a decimal point or in an exponent is a release-sensitive
    /// boundary, whereas the all-bareword integer run is deliberately whole
    /// on every release. Mutating either branch makes a 8.x/9.x row fail.
    #[test]
    fn digit_separator_boundaries_follow_the_number_syntax() {
        let old = (NumberSyntax::Tcl85, Some(TclVersion::V8_6));
        let new = (NumberSyntax::Tcl90, Some(TclVersion::V9_0));
        assert_eq!(scan("1.0_2", old.0, old.1), Some("1.0"));
        assert_eq!(scan("1.0_2", new.0, new.1), Some("1.0_2"));
        assert_eq!(scan("1e1_0", old.0, old.1), Some("1e1_0"));
        assert_eq!(scan("1e1_0", new.0, new.1), Some("1e1_0"));
        assert_eq!(scan("0xff_ff", old.0, old.1), Some("0xff_ff"));
        assert_eq!(scan("0xff_ff", new.0, new.1), Some("0xff_ff"));
    }

    /// A word operator only splits a preceding numeral in the release that
    /// owns that operator. The `lt_` row is the mutation-sensitive boundary
    /// oracle: changing either release gate or right-boundary test reverses it.
    #[test]
    fn word_operator_availability_controls_numeric_joining() {
        assert_eq!(
            scan("1lt 2", NumberSyntax::Tcl85, Some(TclVersion::V8_6)),
            Some("1lt")
        );
        assert_eq!(
            scan("1lt 2", NumberSyntax::Tcl90, Some(TclVersion::V9_0)),
            Some("1")
        );
        assert_eq!(
            scan("1eq_ 2", NumberSyntax::Tcl85, Some(TclVersion::V8_6)),
            Some("1")
        );
        assert_eq!(
            scan("1 lt_ 2", NumberSyntax::Tcl85, Some(TclVersion::V8_6)),
            Some("1")
        );
    }

    /// A successful `TclParseNumber` radix prefix has a distinct numeric end,
    /// so the usual recursive operator probe applies there too. A missing,
    /// invalid, or release-unavailable prefix has no such end and remains one
    /// bareword candidate. These are tclsh 8.6.17 / 9.0.3 oracle rows.
    #[test]
    fn explicit_radix_numerals_split_only_before_available_word_operators() {
        let old = (NumberSyntax::Tcl85, Some(TclVersion::V8_6));
        for source in [
            "0x1ne 1",
            "0xfne 1",
            "0xffin {255}",
            "0b1ne 1",
            "0o7in {7}",
            "0b1ni {1}",
        ] {
            let expected = match source {
                "0x1ne 1" => "0x1",
                "0xfne 1" => "0xf",
                "0xffin {255}" => "0xff",
                "0b1ne 1" | "0b1ni {1}" => "0b1",
                "0o7in {7}" => "0o7",
                _ => unreachable!(),
            };
            assert_eq!(scan(source, old.0, old.1), Some(expected), "{source}");
        }
        let new = (NumberSyntax::Tcl90, Some(TclVersion::V9_0));
        for source in ["0d9lt 10", "0d9le 9", "0d9gt 8", "0d9ge 9"] {
            assert_eq!(scan(source, new.0, new.1), Some("0d9"), "{source}");
        }
        assert_eq!(scan("0xfge 15", new.0, new.1), Some("0xf"));
        assert_eq!(scan("0o8ne 1", old.0, old.1), Some("0o8ne"));
        assert_eq!(scan("0d9ne 1", old.0, old.1), Some("0d9ne"));
        assert_eq!(scan("0b2ne 1", new.0, new.1), Some("0b2ne"));
    }

    /// `JimTcl` accepts exactly six special-float spellings and has neither
    /// `Infinity` nor a `NaN(payload)` grammar. Measured on interpreters
    /// built from every tag 0.76-0.84.
    #[test]
    fn jim_special_floats_are_the_measured_subset() {
        for (numbers, base) in [
            (NumberSyntax::Jim, Some(TclVersion::V8_5)),
            (NumberSyntax::Jim080, Some(TclVersion::V9_0)),
        ] {
            for accepted in ["Inf", "inf", "INF", "NaN", "nan", "NAN"] {
                assert_eq!(scan(accepted, numbers, base), Some(accepted), "{accepted}");
            }
            // Arbitrary case is NOT accepted, where C Tcl's scan is fully
            // case-insensitive.
            for rejected in ["iNf", "nAn", "InF"] {
                assert_eq!(scan(rejected, numbers, base), None, "{rejected}");
            }
            // No `Infinity` spelling: the scan stops after `Inf`, and the
            // trailing `inity` is a bareword byte run with no word operator
            // at its start, so there is no lexeme at all.
            assert_eq!(scan("Infinity", numbers, base), None);
            // No `NaN(payload)` grammar: the `(` simply ends the lexeme.
            assert_eq!(scan("NaN(1)", numbers, base), Some("NaN"));
            // The shared bareword-junction rule still applies.
            assert_eq!(scan("Inf eq 1", numbers, base), Some("Inf"));
            assert_eq!(scan("NaNx", numbers, base), None);
        }
        // `lt` is a word operator only from 0.80, so the junction after a
        // special float follows the same release boundary as everywhere else.
        assert_eq!(
            scan("Inflt 1", NumberSyntax::Jim080, Some(TclVersion::V9_0)),
            Some("Inf")
        );
        assert_eq!(
            scan("Inflt 1", NumberSyntax::Jim, Some(TclVersion::V8_5)),
            None
        );
    }

    /// Tcl's special floats are parsed case-insensitively. `Infinityeq` is
    /// `Infinity eq` (tclsh 8.6.17 / 9.0.3 return 0), while `Infinityx` is a
    /// bareword/function name rather than a numeric prefix. `TclParseNumber`'s
    /// NaN state permits thirteen hexadecimal digits and whitespace only; its
    /// fourteenth digit leaves the scanner at bare `NaN`.
    #[test]
    fn special_floats_follow_numeric_junction_rules() {
        for syntax in NumberSyntax::ALL {
            // JimTcl's special-float grammar is a different shape, not a
            // point on this line — it is covered by
            // `jim_special_floats_are_the_measured_subset` below.
            if matches!(syntax, NumberSyntax::Jim | NumberSyntax::Jim080) {
                continue;
            }
            let base = match syntax {
                NumberSyntax::Tcl84 => Some(TclVersion::V8_4),
                NumberSyntax::Tcl85 => Some(TclVersion::V8_5),
                NumberSyntax::Tcl90 => Some(TclVersion::V9_0),
                NumberSyntax::Jim | NumberSyntax::Jim080 => unreachable!("skipped above"),
            };
            assert_eq!(scan("iNf", *syntax, base), Some("iNf"));
            let nan = if *syntax == NumberSyntax::Tcl84 {
                Some("NaN")
            } else {
                Some("NaN(1)")
            };
            assert_eq!(scan("NaN(1)", *syntax, base), nan);
            assert_eq!(scan("Infinityeq 1", *syntax, base), Some("Infinity"));
            let infinity_x = if *syntax == NumberSyntax::Tcl84 {
                Some("Infinity")
            } else {
                None
            };
            assert_eq!(scan("Infinityx", *syntax, base), infinity_x);
        }
        let modern = (NumberSyntax::Tcl85, Some(TclVersion::V8_6));
        assert_eq!(
            scan("NaN(123456789abcd)", modern.0, modern.1),
            Some("NaN(123456789abcd)")
        );
        assert_eq!(
            scan("NaN( 1 2 3 )", modern.0, modern.1),
            Some("NaN( 1 2 3 )")
        );
        assert_eq!(scan("NaN(123456789abcde)", modern.0, modern.1), Some("NaN"));
        assert_eq!(scan("NaN(1)x", modern.0, modern.1), Some("NaN(1)"));
        assert_eq!(scan("NaNx", modern.0, modern.1), None);
    }

    #[test]
    fn nan_payload_owner_matches_tcl_parse_number_limits() {
        assert_eq!(
            scan_nan_payload(b"NaN( 1 2 3 )", 3),
            Some(NanPayloadLexeme {
                end: 12,
                value: 0x123
            })
        );
        assert_eq!(scan_nan_payload(b"NaN(  )", 3), None);
        assert_eq!(scan_nan_payload(b"NaN(123456789abcde)", 3), None);
    }
}

#[cfg(test)]
mod jim_constructor_tests {
    use super::*;

    #[test]
    fn native_jim_frontiers_preserve_integer_first_tokens() {
        use JimExpressionNumberKind::{Double, Integer};
        for (source, end, kind) in [
            ("0x", 1, Integer),
            ("0xg", 1, Integer),
            ("0b2", 1, Integer),
            ("0o8", 1, Integer),
            ("0d", 1, Integer),
            ("12x", 2, Integer),
            ("1_", 1, Integer),
            ("1eq2", 1, Integer),
            ("09e", 2, Integer),
            ("1e+", 1, Integer),
            ("0x1p4", 3, Integer),
            ("0x1.8p1", 7, Double),
            ("0x1.", 4, Double),
            ("0x.p4", 1, Integer),
            ("0xFE", 4, Integer),
            ("0o755", 5, Integer),
            ("0d755", 5, Integer),
            ("0755", 4, Integer),
            ("1e0", 3, Double),
            (".5", 2, Double),
            ("NaNfoo", 3, Double),
            ("Infinity", 3, Double),
            ("Inf", 3, Double),
        ] {
            let original = format!("!{source}\0suffix");
            let token =
                scan_jim_expression_number(original.as_bytes(), 1, NumberSyntax::Jim080).unwrap();
            assert_eq!(
                (token.end(), token.jim_kind()),
                (1 + end, Some(kind)),
                "{source}"
            );
        }
        assert_eq!(
            scan_jim_expression_number(b".", 0, NumberSyntax::Jim080),
            None
        );
        assert_eq!(
            scan_jim_expression_number(b"0x1.8p1", 0, NumberSyntax::Tcl90),
            None
        );
    }

    #[test]
    fn original_numeric_tokens_select_integer_and_double_constructors() {
        use JimExpressionNumberKind::{Double, Integer};
        for (source, expected) in [
            (b"0755".as_slice(), Integer),
            (b"0xFE", Integer),
            (b"0b101", Integer),
            (b"0o755", Integer),
            (b"0d755", Integer),
            (b"1e0", Double),
            (b".5", Double),
            (b"1.", Double),
            (b"NaN", Double),
            (b"Inf", Double),
        ] {
            let token = scan_expr_number(source, 0, NumberSyntax::Jim, None).unwrap();
            assert_eq!(token.end(), source.len());
            assert_eq!(token.jim_kind(), Some(expected), "{source:?}");
            assert_eq!(
                scan_expr_number(source, 0, NumberSyntax::Tcl90, None)
                    .unwrap()
                    .jim_kind(),
                None
            );
        }
    }
}
