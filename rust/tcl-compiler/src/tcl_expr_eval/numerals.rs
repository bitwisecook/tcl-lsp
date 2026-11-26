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

//! Reading and rendering numbers: literal parsing under a release grammar, strict numbers and Tcl source spellings.

use super::TclValue;
use super::fold_ops::{FoldValue, is_bare_leading_zero, parse_octal_literal};
use super::fold_policy::FoldPolicy;
use tcl_dialect::{DoubleFormat, NumberSyntax};

/// Render a `TclValue` as a Tcl source literal. Matches Tcl's
/// `Tcl_GetStringFromObj` output for numbers.
#[must_use]
pub fn format_tcl_value(v: &TclValue) -> String {
    match v {
        TclValue::Int(i) => i.to_string(),
        // The shared canonical double formatter (also the runtime's `double`
        // string rep): integer-valued floats get `.0`, plus `Inf`/`NaN`.
        TclValue::Float(f) => tcl_syntax::number::format_double(*f),
        // A bignum's canonical string rep is its decimal spelling.
        TclValue::Big(b) => b.to_string(),
    }
}

/// Render a folded numeric value using its exact native string policy.
/// Mutable or unknown floating precision remains a runtime operation.
#[must_use]
pub fn format_tcl_value_with_policy(value: &TclValue, policy: FoldPolicy) -> Option<String> {
    format_with_double_format(value, policy.double_format())
}

pub(super) fn format_with_double_format(
    value: &TclValue,
    format: Option<DoubleFormat>,
) -> Option<String> {
    match value {
        TclValue::Float(value) => Some(tcl_syntax::number::format_double_selected(*value, format?)),
        _ => Some(format_tcl_value(value)),
    }
}

// Literals and variables

/// Parse a numeric/boolean literal. Supports `0x`/`0o`/`0b` prefixes,
/// Tcl-style leading-zero decimals (e.g. `0005`), floats, and the
/// Tcl boolean spellings.
#[must_use]
pub fn parse_literal(text: &str) -> Option<TclValue> {
    parse_literal_in(text, NumberSyntax::default())
}

/// [`parse_literal`] under an explicit release grammar — the form the folder
/// uses, so an operand is read for the dialect being compiled for (`0o17` is
/// nothing under 8.4, and `0755` is 493 up to 8.6).
#[must_use]
pub fn parse_literal_in(text: &str, numbers: NumberSyntax) -> Option<TclValue> {
    // Boolean keywords (`Tcl_GetBoolean`, not part of the number grammar).
    // Accepts unique-prefix spellings (`tr`, `ye`, `of`) like real Tcl.
    if let Some(b) = tcl_syntax::boolean::parse_boolean_word(text) {
        return Some(TclValue::Int(i64::from(b)));
    }
    parse_number_in(text, numbers)
}

fn parse_number_in(text: &str, numbers: NumberSyntax) -> Option<TclValue> {
    use tcl_syntax::number::Number;

    // The numeric grammar is the shared `tcl_syntax::number` (the same
    // `TclParseNumber` port the runtime const-folds with): `0x`/`0o`/`0b`,
    // leading-zero decimal, `_` separators, `Inf`/`NaN`. A magnitude past a
    // wide builds the exact bignum (P4 of type-tracking.md), matching C
    // Tcl's seamless promotion.
    match tcl_syntax::number::parse_whole_with(
        text,
        tcl_syntax::number::ParseFlags::for_syntax(numbers),
    )? {
        Number::Int(v) => Some(TclValue::Int(v)),
        Number::Double(d) => Some(TclValue::Float(d)),
        Number::Nan { .. } => Some(TclValue::Float(f64::NAN)),
        Number::Big {
            negative,
            radix,
            digits,
        } => big_from_parts(negative, radix, &digits),
    }
}

/// Build the exact [`TclValue`] of a beyond-wide integer literal from the
/// shared grammar's `Number::Big` parts (mirrors the VM's `value_as_bigint`).
fn big_from_parts(
    negative: bool,
    radix: tcl_syntax::number::Radix,
    digits: &str,
) -> Option<TclValue> {
    let b = num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix as u32)?;
    Some(TclValue::from_big(if negative { -b } else { b }))
}

/// Parse a *strict* Tcl number — the number grammar only, **without** the
/// boolean-word coercion [`parse_literal`] adds. Used for the polymorphic
/// comparison operators, whose numeric-vs-string decision follows Tcl's number
/// rules: `true`/`yes`/`off`/… are strings, not numbers.
///
/// Reads the numeral under `numbers`, so every release-dependent spelling is
/// judged against the fold's own target: `1_0` and `0d1` are numbers only from
/// 9.0, `0o17`/`0b101` only from 8.5. The bare-leading-zero case is decided one
/// level up by [`strict_number_for_dialect`], which can also abstain when the
/// two readings disagree.
pub(super) fn strict_number(value: &FoldValue, numbers: NumberSyntax) -> Option<TclValue> {
    let FoldValue::Str(text) = value else {
        return value.to_number(numbers);
    };
    parse_number_in(&text.bytes, numbers)
}

/// Like [`strict_number`] but for the arithmetic/unary/math-function
/// operand path, which — unlike [`strict_number`]'s comparison callers —
/// must honour the active dialect's leading-zero rule: `Some(true)` = tcl8.x
/// octal (`010` → 8; `08`/`09` are invalid octal, and Tcl raises an error
/// for them in arithmetic context, so this declines rather than guess),
/// `Some(false)` = tcl9.0 decimal (`010` → 10, matching the shared grammar
/// [`strict_number`] already applies).
///
/// `None` (dialect unknown) folds only when the octal and decimal readings
/// *agree* — e.g. `07` is `7` under both (`7` is a valid octal digit), so
/// it isn't actually ambiguous — and declines when they disagree (`010`:
/// octal 8 vs decimal 10) or either reading is invalid (`08`/`09`: invalid
/// octal, valid decimal). This is more precise than blanket-declining every
/// bare leading-zero operand under an unknown dialect while staying sound:
/// the two candidate dialects are the only readings a real Tcl interpreter
/// could apply.
///
/// Unlike [`classify_operand`], a leading-zero operand that fails to parse
/// under the active dialect always declines (`None`) here rather than
/// falling back to a string classification — arithmetic has no string
/// fallback in Tcl (`expr {08 + 1}` is a runtime error under the 8.x octal
/// rule, not a string operation), so "can't fold" is the only sound outcome.
pub(super) fn strict_number_for_dialect(
    value: &FoldValue,
    octal: Option<bool>,
    numbers: NumberSyntax,
) -> Option<TclValue> {
    let FoldValue::Str(s) = value else {
        return strict_number(value, numbers);
    };
    let s = s.bytes.as_str();
    if is_bare_leading_zero(s) {
        return match octal {
            Some(true) => parse_octal_literal(s),
            Some(false) => strict_number(value, numbers),
            None => match (parse_octal_literal(s), strict_number(value, numbers)) {
                (Some(o), Some(d)) if o == d => Some(o),
                _ => None,
            },
        };
    }
    strict_number(value, numbers)
}
