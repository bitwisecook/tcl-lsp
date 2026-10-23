// SPDX-License-Identifier: AGPL-3.0-or-later
//! Portable selected `strtod` and Jim unsigned-conversion stages.

use crate::number::{Number, Radix};

pub(super) struct ParsedDouble {
    pub value: f64,
    pub range_error: bool,
}

fn whitespace(byte: u8) -> bool {
    byte == b' ' || (b'\t'..=b'\r').contains(&byte)
}
fn trim(input: &[u8]) -> &[u8] {
    let start = input
        .iter()
        .position(|&byte| !whitespace(byte))
        .unwrap_or(input.len());
    let end = input
        .iter()
        .rposition(|&byte| !whitespace(byte))
        .map_or(start, |index| index + 1);
    &input[start..end]
}
fn digit(byte: u8) -> Option<u32> {
    (byte as char).to_digit(16)
}

pub(super) fn jim_unsigned_stage(input: &[u8]) -> super::NativeJimUnsignedStage {
    let mut offset = input
        .iter()
        .position(|&byte| !whitespace(byte))
        .unwrap_or(input.len());
    let negate = input.get(offset) == Some(&b'-');
    if matches!(input.get(offset), Some(b'-' | b'+')) {
        offset += 1;
    }
    let base = if input.get(offset) == Some(&b'0') {
        match input.get(offset + 1) {
            Some(b'x' | b'X') => Some(16),
            Some(b'o' | b'O') => Some(8),
            Some(b'b' | b'B') => Some(2),
            Some(b'd' | b'D') => Some(10),
            _ => None,
        }
    } else {
        None
    };
    if let Some(base) = base {
        offset += 2;
        if !input
            .get(offset)
            .is_some_and(|&b| matches!(b, b'-' | b'+') || whitespace(b))
        {
            return super::NativeJimUnsignedStage {
                offset,
                base,
                negate,
            };
        }
    }
    super::NativeJimUnsignedStage {
        offset: 0,
        base: 10,
        negate: false,
    }
}

/// Pinned Jim's strtoull stage: decimal for `GetDouble`, detected prefixes for Wide.
pub(super) fn jim_unsigned_integer(input: &[u8], force_base: Option<u32>) -> Option<i64> {
    let mut input = trim(input);
    let negative = input.first() == Some(&b'-');
    if matches!(input.first(), Some(b'-' | b'+')) {
        input = &input[1..];
    }
    let mut base = force_base.unwrap_or(10);
    let mut manual_sign = false;
    if force_base.is_none() && input.first() == Some(&b'0') && input.len() > 2 {
        let selected = match input[1] {
            b'x' | b'X' => Some(16),
            b'o' | b'O' => Some(8),
            b'b' | b'B' => Some(2),
            b'd' | b'D' => Some(10),
            _ => None,
        };
        if let Some(selected) = selected {
            base = selected;
            input = &input[2..];
            manual_sign = true;
        }
    }
    if input.is_empty() {
        return None;
    }
    let mut value = 0_u64;
    let mut overflow = false;
    for &byte in input {
        let digit = digit(byte).filter(|&digit| digit < base)?;
        if !overflow {
            if let Some(next) = value
                .checked_mul(u64::from(base))
                .and_then(|value| value.checked_add(u64::from(digit)))
            {
                value = next;
            } else {
                value = u64::MAX;
                overflow = true;
            }
        }
    }
    if negative && (!overflow || manual_sign) {
        value = value.wrapping_neg();
    }
    Some(value.cast_signed())
}

/// Jim's `strtol` stage for an absolute frame suffix. Unlike Wide, this
/// requires the end pointer to reach NUL without trailing whitespace and uses
/// signed-long saturation before a manually stripped prefix sign.
pub(super) fn jim_frame_long(input: &[u8]) -> Option<i64> {
    if input.is_empty() || input.last().is_some_and(|byte| whitespace(*byte)) {
        return None;
    }
    let mut input = trim(input);
    let negative = input.first() == Some(&b'-');
    if matches!(input.first(), Some(b'-' | b'+')) {
        input = &input[1..];
    }
    let mut base = 10;
    let mut manual_sign = false;
    if input.len() > 2 && input.first() == Some(&b'0') {
        base = match input[1] {
            b'x' | b'X' => 16,
            b'o' | b'O' => 8,
            b'b' | b'B' => 2,
            _ => 10,
        };
        if matches!(
            input[1],
            b'x' | b'X' | b'o' | b'O' | b'b' | b'B' | b'd' | b'D'
        ) {
            input = &input[2..];
            manual_sign = true;
        }
    }
    if input.is_empty() {
        return None;
    }
    let bound = if negative && !manual_sign {
        i64::MIN.unsigned_abs()
    } else {
        i64::MAX.cast_unsigned()
    };
    let mut magnitude = 0_u64;
    for &byte in input {
        let digit = digit(byte).filter(|digit| *digit < base)?;
        magnitude = magnitude
            .saturating_mul(u64::from(base))
            .saturating_add(u64::from(digit))
            .min(bound);
    }
    Some(if negative {
        magnitude.wrapping_neg().cast_signed()
    } else {
        magnitude.cast_signed()
    })
}

pub(super) fn parse_c_double(input: &[u8]) -> Option<ParsedDouble> {
    let input = trim(input);
    let (negative, unsigned) = if matches!(input.first(), Some(b'-' | b'+')) {
        (input[0] == b'-', &input[1..])
    } else {
        (false, input)
    };
    if unsigned.eq_ignore_ascii_case(b"inf") || unsigned.eq_ignore_ascii_case(b"infinity") {
        return Some(ParsedDouble {
            value: if negative {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            },
            range_error: false,
        });
    }
    if let Some(value) = nan(unsigned, negative) {
        return Some(ParsedDouble {
            value,
            range_error: false,
        });
    }
    if unsigned.len() >= 2 && unsigned[..2].eq_ignore_ascii_case(b"0x") {
        let (value, inexact) = hex_double(&unsigned[2..], negative)?;
        return Some(ParsedDouble {
            value,
            range_error: value.is_infinite() || (value.abs() < f64::MIN_POSITIVE && inexact),
        });
    }
    let text = std::str::from_utf8(input).ok()?;
    if !decimal_grammar(unsigned) {
        return None;
    }
    let value: f64 = text.parse().ok()?;
    let range_error = value.is_infinite()
        || (value.abs() < f64::MIN_POSITIVE && !decimal_exact(unsigned, value.abs()));
    Some(ParsedDouble { value, range_error })
}

fn nan(input: &[u8], negative: bool) -> Option<f64> {
    if input.len() < 3 || !input[..3].eq_ignore_ascii_case(b"nan") {
        return None;
    }
    let payload = if input.len() == 3 {
        0
    } else {
        if input.get(3) != Some(&b'(') || input.last() != Some(&b')') {
            return None;
        }
        let payload = &input[4..input.len() - 1];
        if !payload
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            return None;
        }
        nan_payload(payload)
    };
    Some(f64::from_bits(
        (u64::from(negative) << 63) | 0x7ff8_0000_0000_0000 | (payload & 0x0007_ffff_ffff_ffff),
    ))
}
fn nan_payload(input: &[u8]) -> u64 {
    let (base, input) = if input.len() > 2 && input[..2].eq_ignore_ascii_case(b"0x") {
        (16, &input[2..])
    } else if input.first() == Some(&b'0') {
        (8, input)
    } else {
        (10, input)
    };
    if input.is_empty() {
        return 0;
    }
    let mut value = 0_u64;
    for &byte in input {
        let Some(digit) = digit(byte).filter(|&digit| digit < base) else {
            return 0;
        };
        value = value
            .saturating_mul(u64::from(base))
            .saturating_add(u64::from(digit));
    }
    value
}

fn decimal_grammar(input: &[u8]) -> bool {
    let mut index = 0;
    let mut digits = 0;
    while input.get(index).is_some_and(u8::is_ascii_digit) {
        index += 1;
        digits += 1;
    }
    if input.get(index) == Some(&b'.') {
        index += 1;
        while input.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return false;
    }
    if matches!(input.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(input.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let start = index;
        while input.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == start {
            return false;
        }
    }
    index == input.len()
}

// Exact decimal subnormal detection: remove decimal powers of five, then
// compare the remaining binary rational to the f64's exact significand.
fn decimal_exact(input: &[u8], value: f64) -> bool {
    let exponent_at = input
        .iter()
        .position(|byte| *byte == b'e' || *byte == b'E')
        .unwrap_or(input.len());
    let significand = &input[..exponent_at];
    let exponent = if exponent_at < input.len() {
        decimal_exponent(&input[exponent_at + 1..])
    } else {
        0
    };
    let fraction = significand
        .iter()
        .position(|byte| *byte == b'.')
        .map_or(0, |dot| significand.len() - dot - 1);
    let mut power = exponent.saturating_sub(i64::try_from(fraction).unwrap_or(i64::MAX));
    let mut digits: Vec<u8> = significand
        .iter()
        .filter(|byte| byte.is_ascii_digit())
        .map(|byte| byte - b'0')
        .collect();
    if digits.iter().all(|&digit| digit == 0) {
        return value == 0.0;
    }
    if value == 0.0 || power >= 0 {
        return false;
    }
    // Each successful division shrinks the coefficient; no loop proportional
    // to an attacker-controlled exponent is needed once divisibility fails.
    while power < 0 {
        if !decimal_divide_five(&mut digits) {
            return false;
        }
        power += 1;
    }
    let mut coefficient = 0_u64;
    for digit in digits {
        let Some(next) = coefficient
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(digit)))
        else {
            return false;
        };
        coefficient = next;
    }
    // A subnormal has value = fraction * 2^-1074.
    let mantissa = value.to_bits() & 0x000f_ffff_ffff_ffff;
    let mut coefficient_power =
        exponent.saturating_sub(i64::try_from(fraction).unwrap_or(i64::MAX));
    while coefficient != 0 && coefficient & 1 == 0 {
        coefficient >>= 1;
        coefficient_power += 1;
    }
    let mut mantissa = mantissa;
    let mut native_power = -1074;
    while mantissa != 0 && mantissa & 1 == 0 {
        mantissa >>= 1;
        native_power += 1;
    }
    coefficient == mantissa && coefficient_power == native_power
}
fn decimal_divide_five(digits: &mut [u8]) -> bool {
    let mut carry = 0;
    for digit in digits {
        let value = carry * 10 + *digit;
        *digit = value / 5;
        carry = value % 5;
    }
    carry == 0
}
fn decimal_exponent(input: &[u8]) -> i64 {
    let negative = input.first() == Some(&b'-');
    let input = if matches!(input.first(), Some(b'+' | b'-')) {
        &input[1..]
    } else {
        input
    };
    let value = input.iter().fold(0_i64, |value, byte| {
        value
            .saturating_mul(10)
            .saturating_add(i64::from(byte - b'0'))
    });
    if negative {
        value.saturating_neg()
    } else {
        value
    }
}

struct BinaryMagnitude {
    prefix: u64,
    bits: i64,
    tail_nonzero: bool,
}
impl BinaryMagnitude {
    fn new() -> Self {
        Self {
            prefix: 0,
            bits: 0,
            tail_nonzero: false,
        }
    }
    fn push(&mut self, bit: bool) {
        if self.bits == 0 && !bit {
            return;
        }
        if self.bits < 54 {
            self.prefix = (self.prefix << 1) | u64::from(bit);
        } else {
            self.tail_nonzero |= bit;
        }
        self.bits = self.bits.saturating_add(1);
    }
    fn round(self, exponent: i64, negative: bool) -> (f64, bool) {
        let sign = u64::from(negative) << 63;
        if self.bits == 0 {
            return (f64::from_bits(sign), false);
        }
        let top = self.bits.saturating_sub(1).saturating_add(exponent);
        if top > 1023 {
            return (f64::from_bits(sign | 0x7ff0_0000_0000_0000), true);
        }
        let quantum = top.saturating_sub(52).max(-1074);
        let keep = self.bits.saturating_sub(quantum.saturating_sub(exponent));
        let prefix_bits = self.bits.min(54);
        let (mut mantissa, guard, sticky) = if keep < 0 {
            (0, false, true)
        } else if keep == 0 {
            (
                0,
                true,
                self.prefix & ((1_u64 << (prefix_bits - 1)) - 1) != 0 || self.tail_nonzero,
            )
        } else if keep >= self.bits {
            (self.prefix << (keep - prefix_bits), false, false)
        } else {
            let shift = prefix_bits - keep;
            let mantissa = self.prefix >> shift;
            let guard = self.prefix & (1_u64 << (shift - 1)) != 0;
            let sticky = self.prefix & ((1_u64 << (shift - 1)) - 1) != 0 || self.tail_nonzero;
            (mantissa, guard, sticky)
        };
        if guard && (sticky || mantissa & 1 != 0) {
            mantissa += 1;
        }
        let result = if mantissa == 0 {
            0
        } else if quantum == -1074 && mantissa < (1_u64 << 52) {
            mantissa
        } else {
            let mut top = quantum + 52;
            if mantissa == (1_u64 << 53) {
                mantissa >>= 1;
                top += 1;
            }
            if top > 1023 {
                0x7ff0_0000_0000_0000
            } else {
                (u64::try_from(top + 1023).expect("normal binary exponent") << 52)
                    | (mantissa & 0x000f_ffff_ffff_ffff)
            }
        };
        (f64::from_bits(sign | result), guard || sticky)
    }
}

fn hex_double(input: &[u8], negative: bool) -> Option<(f64, bool)> {
    let mut magnitude = BinaryMagnitude::new();
    let mut index = 0;
    let mut digits = 0;
    let mut fraction = 0_i64;
    let mut after_dot = false;
    while let Some(&byte) = input.get(index) {
        if byte == b'.' && !after_dot {
            after_dot = true;
            index += 1;
            continue;
        }
        let Some(digit) = digit(byte) else { break };
        for bit in (0..4).rev() {
            magnitude.push(digit & (1 << bit) != 0);
        }
        digits += 1;
        fraction += i64::from(after_dot);
        index += 1;
    }
    if digits == 0 {
        return None;
    }
    let exponent = if matches!(input.get(index), Some(b'p' | b'P')) {
        index += 1;
        let start = index;
        if matches!(input.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        let first_digit = index;
        while input.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == first_digit {
            return None;
        }
        decimal_exponent(&input[start..index])
    } else {
        0
    };
    if index != input.len() {
        return None;
    }
    Some(magnitude.round(
        exponent.saturating_sub(fraction.saturating_mul(4)),
        negative,
    ))
}

pub(super) fn number_double(number: &Number) -> f64 {
    match number {
        Number::Int(value) => integer_double(*value),
        Number::Double(value) => *value,
        Number::Nan { negative, payload } => f64::from_bits(
            (u64::from(*negative) << 63)
                | 0x7ff8_0000_0000_0000
                | (payload.unwrap_or(0) & 0x0007_ffff_ffff_ffff),
        ),
        Number::Big {
            negative,
            radix: Radix::Dec,
            digits,
        } => {
            let value = digits.parse::<f64>().unwrap_or(f64::INFINITY);
            if *negative { -value } else { value }
        }
        Number::Big {
            negative,
            radix,
            digits,
        } => {
            let width = match radix {
                Radix::Bin => 1,
                Radix::Oct => 3,
                Radix::Hex => 4,
                Radix::Dec => unreachable!(),
            };
            let mut magnitude = BinaryMagnitude::new();
            for byte in digits.bytes() {
                let digit = digit(byte).expect("native parser supplied radix digits");
                for bit in (0..width).rev() {
                    magnitude.push(digit & (1 << bit) != 0);
                }
            }
            magnitude.round(0, *negative).0
        }
    }
}

// Each limb is exactly representable. The sum performs one native IEEE round.
pub(super) fn integer_double(value: i64) -> f64 {
    let magnitude = value.unsigned_abs();
    let high = u32::try_from(magnitude >> 32).expect("upper integer limb");
    let low = u32::try_from(magnitude & u64::from(u32::MAX)).expect("lower integer limb");
    let value_double = f64::from(high) * 4_294_967_296.0 + f64::from(low);
    if value < 0 {
        -value_double
    } else {
        value_double
    }
}
