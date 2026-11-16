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

//! One index word as each release reads it: 8.4, 8.5 and 8.6, 9.0 on, and Jim, with their integer ranges.

use super::IndexReading;
use tcl_dialect::TclVersion;
use tcl_syntax::number::{Number, NumberSyntax, ParseFlags, Radix};

/// Whose integer range reads an index's integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Range {
    Tcl84,
    Tcl85,
    Tcl86,
    /// 8.5 or 8.6, the shared numeral grammar naming neither.
    Tcl85Or86,
    /// 9.0 on: a wide, a bignum truncated to one.
    Wide,
    Jim,
}

impl Range {
    pub(super) const fn of_release(release: TclVersion) -> Self {
        match release {
            TclVersion::V8_4 => Self::Tcl84,
            TclVersion::V8_5 => Self::Tcl85,
            TclVersion::V8_6 => Self::Tcl86,
            TclVersion::V9_0 | TclVersion::V9_1 => Self::Wide,
        }
    }

    pub(super) const fn of_numbers(numbers: NumberSyntax) -> Self {
        match numbers {
            NumberSyntax::Tcl84 => Self::Tcl84,
            NumberSyntax::Tcl85 => Self::Tcl85Or86,
            NumberSyntax::Tcl90 => Self::Wide,
            NumberSyntax::Jim | NumberSyntax::Jim080 => Self::Jim,
        }
    }
}

/// One index as a release reads it (`TclGetIntForIndex` up to 8.6,
/// `GetWideForIndex` from 9.0), measured with `string index` on tclsh
/// 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0:
///
/// - an integer, whitespace allowed either side (`Tcl_GetInt`);
/// - `end` from its first byte, which 8.4 to 8.6 also read abbreviated (`e`,
///   `en`), followed by an offset: `-N` in 8.4, whose `N` may start with
///   whitespace (`end- 1`), and `+N` or `-N` from 8.5, whose `N` starts at once;
/// - from 8.5, `M+N` or `M-N` after any leading whitespace, the operator right
///   after `M` and `N` right after the operator.
///
/// Nothing else is an index: not `end+1`, `1+1` or `1-1` in 8.4, not ` end`,
/// `end ` or `end -1` in any release. A Jim grammar reads the forms of Tcl 9.0
/// with whitespace around `end` and each operand.
///
/// Each integer is read in its release's range (measured on each
/// release's `tclsh`):
///
/// - 8.4 to 8.6 read one through `Tcl_GetInt`: a value within ±`UINT_MAX`
///   (4,294,967,295) wraps to the 32-bit `int` — on every platform, the 64-bit
///   `long` casting it and a 32-bit one taking it through
///   `Tcl_GetLongFromObj`'s ±`ULONG_MAX` — so `2147483648` is
///   −2147483648 and `-4294967295` is 1, and the sums and `end` offsets wrap
///   alike (8.6's `1+2147483647` is −2147483648, `end-4294967295` is
///   `end+1`); any other value is `bad index`, but for a magnitude from
///   2^64 − 2^32 + 1 to 2^64 − 1, which 8.4 (`strtoul`) and 8.5 (the bignum
///   branch) wrap into that range where `long` is 64 bits and raise on where it
///   is 32 bits, and 8.6 raises on everywhere;
/// - 9.0 and 9.1 read a wide, a bignum alone being the wide nearest it, and
///   encode `end+1` as the widest wide: a sum that reaches it and a bignum
///   offset after `end` name `end+1` (`lset` appends there), a larger `end+N`
///   any index past it. This is the 64-bit `Tcl_Size` build every oracle is;
///   a 32-bit one bounds an index at 2^31 − 1.
pub(super) fn read(spec: &str, len: usize, numbers: NumberSyntax, range: Range) -> IndexReading {
    match range {
        Range::Tcl84 => read_84(spec, len, numbers, range),
        Range::Tcl85 | Range::Tcl86 | Range::Tcl85Or86 => read_85(spec, len, numbers, range),
        Range::Wide => read_90(spec, len, numbers),
        Range::Jim => {
            let end = i64::try_from(len).unwrap_or(i64::MAX) - 1;
            jim(spec, end, numbers).map_or(IndexReading::Bad, IndexReading::At)
        }
    }
}

/// C's `int` index of `end` for a container of `len` elements.
pub(super) fn end_32(len: usize) -> i64 {
    wrap_32(i64::try_from(len).unwrap_or(i64::MAX) - 1)
}

/// `value` cast to C's 32-bit `int`: its low 32 bits, sign-extended.
pub(super) const fn wrap_32(value: i64) -> i64 {
    (value << 32) >> 32
}

/// 8.4: `Tcl_GetInt` on the word; then `e`, `en`, `end`, or `end-` and a
/// `Tcl_GetInt` integer, wherever `end` is the `int` before the first element.
fn read_84(spec: &str, len: usize, numbers: NumberSyntax, range: Range) -> IndexReading {
    if let Some(number) = whole_number(spec, numbers) {
        return int_32(&number, range);
    }
    if !spec.starts_with('e') {
        return IndexReading::Bad;
    }
    let end = end_32(len);
    if spec.len() <= 3 {
        return if "end".starts_with(spec) {
            IndexReading::At(end)
        } else {
            IndexReading::Bad
        };
    }
    let Some(number) = spec
        .strip_prefix("end-")
        .and_then(|operand| whole_number(operand, numbers))
    else {
        return IndexReading::Bad;
    };
    int_32(&number, range).map(|offset| wrap_32(end + wrap_32(-offset)))
}

/// 8.5 and 8.6: `Tcl_GetInt` on the word; then `end` with an offset, or a sum.
fn read_85(spec: &str, len: usize, numbers: NumberSyntax, range: Range) -> IndexReading {
    if let Some(number) = whole_number(spec, numbers) {
        return int_32(&number, range);
    }
    if spec.starts_with('e') {
        let end = end_32(len);
        if spec.len() <= 3 && "end".starts_with(spec) {
            return IndexReading::At(end);
        }
        return spec.strip_prefix("end").map_or(IndexReading::Bad, |rest| {
            signed_32(rest, numbers, range).map(|offset| wrap_32(end + offset))
        });
    }
    let Some((first, rest)) = sum_head(spec, numbers) else {
        return IndexReading::Bad;
    };
    int_32(&first, range).zip(signed_32(rest, numbers, range), |a, b| wrap_32(a + b))
}

/// 9.0 on: `GetWideForIndex` on the word, then `GetEndOffsetFromObj`.
fn read_90(spec: &str, len: usize, numbers: NumberSyntax) -> IndexReading {
    let end = i64::try_from(len).unwrap_or(i64::MAX) - 1;
    if let Some(number) = whole_number(spec, numbers) {
        return match number {
            Number::Int(value) => IndexReading::At(value),
            Number::Big { negative, .. } => {
                IndexReading::At(if negative { i64::MIN } else { i64::MAX })
            }
            _ => IndexReading::Bad,
        };
    }
    if spec.starts_with('e') {
        if spec == "end" {
            return IndexReading::At(end);
        }
        return spec
            .strip_prefix("end")
            .and_then(|rest| signed_operand(rest, numbers))
            .map_or(IndexReading::Bad, |(negative, number)| {
                IndexReading::At(end_offset_90(end, negative, &number))
            });
    }
    let Some((first, rest)) = sum_head(spec, numbers) else {
        return IndexReading::Bad;
    };
    let Some((negative, second)) = signed_operand(rest, numbers) else {
        return IndexReading::Bad;
    };
    match exact_sum(&first, negative, &second) {
        // The sum's encoding when it reaches the widest wide is `end+1`.
        Some(sum) if sum >= i128::from(i64::MAX) => IndexReading::At(end + 1),
        Some(sum) => IndexReading::At(i64::try_from(sum).unwrap_or(i64::MIN)),
        None => IndexReading::Unsure,
    }
}

/// 9.0's `end+N` or `end-N`: a bignum offset is `end+1` or before the first
/// element, `end+1` itself `end+1`, a larger offset any index past it.
fn end_offset_90(end: i64, negative: bool, number: &Number) -> i64 {
    let offset = match number {
        Number::Big {
            negative: below, ..
        } => return if *below == negative { end + 1 } else { -1 },
        Number::Int(value) if negative => value.checked_neg().unwrap_or(i64::MAX),
        Number::Int(value) => *value,
        _ => return -1,
    };
    end.saturating_add(offset)
}

/// The exact value of `first ± second` within `i128`, saturated where an
/// operand is past it, or `None` where two such operands of opposite signs
/// leave the sum unknown.
pub(super) fn exact_sum(first: &Number, negative: bool, second: &Number) -> Option<i128> {
    let integer = |number: &Number| matches!(number, Number::Int(_) | Number::Big { .. });
    if !integer(first) || !integer(second) {
        return None;
    }
    let a = wide_128(first);
    let b = wide_128(second).map(|b| if negative { b.wrapping_neg() } else { b });
    let sign = |number: &Number, flip: bool| -> i128 {
        let below = matches!(number, Number::Int(v) if *v < 0)
            || matches!(number, Number::Big { negative: true, .. });
        if below == flip { i128::MAX } else { i128::MIN }
    };
    match (a, b) {
        (Some(a), Some(b)) => Some(a.saturating_add(b)),
        (Some(_), None) => Some(sign(second, negative)),
        (None, Some(_)) => Some(sign(first, false)),
        (None, None) => {
            let (x, y) = (sign(first, false), sign(second, negative));
            (x == y).then_some(x)
        }
    }
}

/// An integer as an `i128`, `None` past it.
fn wide_128(number: &Number) -> Option<i128> {
    match number {
        Number::Int(value) => Some(i128::from(*value)),
        Number::Big {
            negative,
            radix,
            digits,
        } => magnitude(*radix, digits)
            .and_then(|m| i128::try_from(m).ok())
            .map(|m| if *negative { -m } else { m }),
        _ => None,
    }
}

/// An integer as `Tcl_GetInt` reads it before 9.0: see
/// [`read`].
pub(super) fn int_32(number: &Number, range: Range) -> IndexReading {
    const UINT_MAX: u64 = 0xffff_ffff;
    match number {
        Number::Int(value) if value.unsigned_abs() <= UINT_MAX => IndexReading::At(wrap_32(*value)),
        Number::Big {
            negative,
            radix,
            digits,
        } => {
            let Some(unsigned) = magnitude(*radix, digits).and_then(|m| u64::try_from(m).ok())
            else {
                return IndexReading::Bad;
            };
            let long = if *negative {
                unsigned.cast_signed().wrapping_neg()
            } else {
                unsigned.cast_signed()
            };
            if long.unsigned_abs() > UINT_MAX {
                return IndexReading::Bad;
            }
            match range {
                Range::Tcl84 | Range::Tcl85 => IndexReading::HostLong(wrap_32(long)),
                Range::Tcl85Or86 => IndexReading::Unsure,
                Range::Tcl86 | Range::Wide | Range::Jim => IndexReading::Bad,
            }
        }
        _ => IndexReading::Bad,
    }
}

/// A bignum's magnitude, `None` past `u128`.
pub(super) fn magnitude(radix: Radix, digits: &str) -> Option<u128> {
    let base = match radix {
        Radix::Bin => 2,
        Radix::Oct => 8,
        Radix::Dec => 10,
        Radix::Hex => 16,
    };
    digits.chars().try_fold(0_u128, |value, digit| {
        value
            .checked_mul(u128::from(base))?
            .checked_add(u128::from(digit.to_digit(base)?))
    })
}

/// Tcl's whitespace (`TclIsSpaceProc`).
const fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r')
}

/// The first integer of a sum `M+N` or `M-N` from 8.5, after any leading
/// whitespace and with its operator right after it, and the rest from that
/// operator.
pub(super) fn sum_head(spec: &str, numbers: NumberSyntax) -> Option<(Number, &str)> {
    let body = spec.trim_start_matches(is_space);
    let flags = ParseFlags {
        no_whitespace: true,
        ..index_int_flags(numbers)
    };
    let parsed = tcl_syntax::number::parse(body, flags)?;
    Some((parsed.number, &body[parsed.end..]))
}

/// `+N` or `-N` with `N` a whole integer starting right after the operator:
/// whether it subtracts, and `N`.
pub(super) fn signed_operand(rest: &str, numbers: NumberSyntax) -> Option<(bool, Number)> {
    let negative = match rest.as_bytes().first()? {
        b'-' => true,
        b'+' => false,
        _ => return None,
    };
    let operand = &rest[1..];
    if operand.is_empty() || operand.starts_with(is_space) {
        return None;
    }
    Some((negative, whole_number(operand, numbers)?))
}

/// [`signed_operand`] read through `Tcl_GetInt` before 9.0, as the value it
/// adds.
fn signed_32(rest: &str, numbers: NumberSyntax, range: Range) -> IndexReading {
    signed_operand(rest, numbers).map_or(IndexReading::Bad, |(negative, number)| {
        int_32(&number, range).map(|value| if negative { wrap_32(-value) } else { value })
    })
}

/// A Jim index: a base (`end` or an integer) and an optional `+`/`-` operand,
/// whitespace allowed around each.
fn jim(spec: &str, end: i64, numbers: NumberSyntax) -> Option<i64> {
    let s = spec.trim_matches(is_space);
    let (base, rest) = if let Some(rest) = s.strip_prefix("end") {
        (end, rest)
    } else {
        parse_int_prefix(s, numbers)?
    };
    if rest.is_empty() {
        return Some(base);
    }
    let negative = match rest.as_bytes()[0] {
        b'-' => true,
        b'+' => false,
        _ => return None,
    };
    let operand = parse_int_whole(rest[1..].trim_matches(is_space), numbers)?;
    Some(base.saturating_add(if negative {
        operand.saturating_neg()
    } else {
        operand
    }))
}

/// [`ParseFlags`](tcl_syntax::number::ParseFlags) for an index integer: reject a
/// fractional part / exponent, but otherwise the full Tcl integer syntax of the
/// release in play — so a hex index like `0x2` resolves the way real Tcl's
/// `Tcl_GetIntForIndex` does.
///
/// An index word is read by `Tcl_GetIntForIndex`, so it inherits every version
/// difference in the numeral grammar. Pinned on tclsh 8.6.16 / 9.0.4 against a
/// 12-element list `a`…`l`:
///
/// | spec | 8.6 | 9.0 |
/// |---|---|---|
/// | `lindex $l 010` | `i` (index 8, octal) | `k` (index 10, decimal) |
/// | `lindex $l end-010` | `d` (end−8) | `b` (end−10) |
/// | `lindex $l 1_0` | `bad index` | `k` |
/// | `lindex $l 0d1` | `bad index` | `b` |
///
/// The ordinary runtime path takes the grammar the interpreter was built for —
/// the ambient installed once by `set_runtime_version`, exactly as C settles it
/// at build time; a compile-time caller that must name a different release
/// passes it explicitly through [`read_with`] or [`read_under`].
pub(super) fn index_int_flags(numbers: NumberSyntax) -> ParseFlags {
    ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(numbers)
    }
}

/// A whole integer word, whitespace allowed either side (`Tcl_GetInt`): a wide
/// or a bignum.
pub(super) fn whole_number(s: &str, numbers: NumberSyntax) -> Option<Number> {
    let parsed = tcl_syntax::number::parse(s, index_int_flags(numbers))?;
    (s[parsed.end..].trim_matches(is_space).is_empty()
        && matches!(parsed.number, Number::Int(_) | Number::Big { .. }))
    .then_some(parsed.number)
}

/// Parse a leading Tcl integer, returning its value and the unconsumed tail via
/// the canonical [`tcl_syntax::number`] parser (so every radix the runtime
/// accepts resolves identically). `None` if `s` does not start with an integer,
/// or the value is a non-integer / bignum (`Int` only).
pub(super) fn parse_int_prefix(s: &str, numbers: NumberSyntax) -> Option<(i64, &str)> {
    let parsed = tcl_syntax::number::parse(s, index_int_flags(numbers))?;
    match parsed.number {
        Number::Int(v) => Some((v, &s[parsed.end..])),
        _ => None,
    }
}

/// Parse `s` as a whole Tcl integer — the [`parse_int_prefix`] value only when
/// nothing but trailing space follows it.
pub(super) fn parse_int_whole(s: &str, numbers: NumberSyntax) -> Option<i64> {
    let (val, tail) = parse_int_prefix(s, numbers)?;
    tail.trim_matches(is_space).is_empty().then_some(val)
}
