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

//! The destructuring writers: `scan`, `binary scan`, `lassign` and `array
//! set`, each one value taken apart into several places over the shared
//! cores (`docs/design/compiler/value-transfers.md` § *Storage-writing
//! commands*).
//!
//! A converted field writes its target with the type it was built as; a
//! target the conversion did not reach is preserved — the command leaves it
//! untouched and never creates it — and `array set` writes one element per
//! pair. Each route answers only where every release its target names
//! reads the words alike; the rest decline on the axis that differs.

use tcl_cmd_core::scan::{Scanned, scan_match, validate_format};
use tcl_dialect::TclVersion;
use tcl_dialect::model::SpecSurface;
use tcl_syntax::value::ValueOps as _;

use crate::types::TclType;

use super::CommandSemantics;
use super::answers::EvalAnswer;
use super::const_ops::{ConstOps, ConstValue, Needs, Representation};
use super::context::Budget;
use super::decline::{Axis, DeclineReason};
use super::inputs::{AnalysisInputs, OperandId, TargetId};
use super::publication::{PendingStore, Publication, open_words, targets_are};
use super::route::{EvalRoute, NativeEvalId};

/// The revision of the four destructuring routes.
const DESTRUCTURE_REVISION: u64 = 1;

/// Whether the target names a release at or after `floor`.
fn at_least(ops: &ConstOps<'_>, floor: TclVersion) -> bool {
    ops.target().release.is_some_and(|release| release >= floor)
}

/// The decline for a form only the releases from `surface`'s floor have,
/// under a target that does not name one of them.
fn unavailable(surface: &[SpecSurface]) -> DeclineReason {
    surface
        .first()
        .map_or(DeclineReason::Unsupported, |surface| {
            DeclineReason::ReleaseAmbiguous(Axis::Availability(*surface))
        })
}

/// `scan string format ?varName …?` on the direct route: the shared matcher
/// over the format the shared validator accepted. With variables, the
/// count of the leading conversions that succeeded as the result, a `Write`
/// per converted variable typed as its conversion built it, and a
/// `Preserve` for the rest — `-1` and every variable preserved when the
/// input ended before any conversion; without, the conversions as a list,
/// a failed one the empty string. Only the formats every release the
/// target names reads alike are evaluated: no positional `%n$` or size
/// modifier (the matcher assigns in order and ignores sizes); `%b` from 8.6;
/// a float conversion from 8.5, whose double spelling 8.4 does not share,
/// and never over an input spelling an infinity (from 8.5 `scan -inf %f` is
/// `-Inf`, where the matcher converts nothing) or to a negative zero (`scan
/// -0 %f` is `0.0` from 8.5, which reads the integer spelling as an integer
/// first); no `%u`, which the matcher reads as signed (`scan -1 %u` is
/// `18446744073709551615` on every release); no `0x` input to a radix
/// conversion under 8.4, whose sign handling differs; and every integer
/// within the 32-bit range, past which the releases
/// disagree (measured: `scan 2147483648 %d` is `-2147483648` on 8.4, 9.0
/// and 9.1 and `2147483648` on 8.5 and 8.6; `scan 4294967296 %d` is
/// `4294967296` up to 8.6 and `2147483647` from 9.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanSemantics;

/// `scan`.
pub static SCAN: ScanSemantics = ScanSemantics;

impl ScanSemantics {
    /// The axes the route reads: the conversions' numerals, the characters
    /// `%c`, `%n` and a width count, the source decoding of a non-ASCII
    /// operand, and the inline form's list.
    pub const NEEDS: Needs = Needs::NUMERAL_GRAMMAR
        .union(Needs::CHAR_INDEXING)
        .union(Needs::SOURCE_ENCODING)
        .union(Needs::LIST_RENDERING);

    /// The decline for a format the target's releases do not all read as
    /// the matcher does, when it is one.
    fn format_decline(ops: &ConstOps<'_>, fmt: &[char], input: &str) -> Option<DeclineReason> {
        let mut radix = false;
        let mut float = false;
        let mut at = 0;
        while at < fmt.len() {
            if fmt[at] != '%' {
                at += 1;
                continue;
            }
            at += 1;
            let Ok(conversion) = tcl_syntax::scan::parse_conversion(fmt, &mut at) else {
                return Some(DeclineReason::WrongRepresentation);
            };
            if conversion.verb == '%' {
                continue;
            }
            if conversion.xpg_index.is_some() || conversion.size.is_some() {
                return Some(DeclineReason::Unsupported);
            }
            match conversion.verb {
                'b' if !at_least(ops, TclVersion::V8_6) => {
                    return Some(unavailable(SpecSurface::TCL86_PLUS));
                }
                'e' | 'E' | 'f' | 'g' | 'G' if !at_least(ops, TclVersion::V8_5) => {
                    return Some(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar));
                }
                'e' | 'E' | 'f' | 'g' | 'G' => float = true,
                'u' => return Some(DeclineReason::Unsupported),
                'x' | 'X' | 'i' => radix = true,
                _ => {}
            }
        }
        if float && input.to_ascii_lowercase().contains("inf") {
            return Some(DeclineReason::Unsupported);
        }
        (radix
            && !at_least(ops, TclVersion::V8_5)
            && (input.contains("0x") || input.contains("0X")))
        .then_some(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar))
    }

    /// One conversion's value, built as the command builds it.
    fn value(ops: &mut ConstOps<'_>, scanned: &Scanned) -> Result<ConstValue, DeclineReason> {
        match scanned {
            Scanned::Int(n) if i32::try_from(*n).is_ok() => Ok(ConstValue::int(*n)),
            Scanned::Int(_) => Err(DeclineReason::ReleaseAmbiguous(Axis::IntTower)),
            Scanned::Double(d) if *d == 0.0 && d.is_sign_negative() => {
                Err(DeclineReason::Unsupported)
            }
            Scanned::Double(d) => Ok(ops.new_double(*d)),
            Scanned::Str(s) => Ok(ConstValue::text(s)),
        }
    }

    fn evaluate_scan(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let (mut ops, texts, targets) = match open_words(input, budget, Self::NEEDS) {
            Ok(opened) => opened,
            Err(answer) => return answer,
        };
        let [string, format, vars @ ..] = texts.as_slice() else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        let fmt: Vec<char> = format.chars().collect();
        // The command's own validation first: a malformed format, or one
        // whose conversions and variables disagree, raises.
        let Ok(fields) = validate_format(&fmt, vars.len()) else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        if let Some(reason) = Self::format_decline(&ops, &fmt, string) {
            return EvalAnswer::Declined(reason);
        }
        // The matcher's work: one unit per subject byte.
        if let Err(reason) = ops.charge(u64::try_from(string.len()).unwrap_or(u64::MAX)) {
            return EvalAnswer::Declined(reason);
        }
        let chars: Vec<char> = string.chars().collect();
        let outcome = scan_match(&chars, &fmt);
        let mut values = Vec::with_capacity(outcome.values.len());
        for scanned in &outcome.values {
            match scanned
                .as_ref()
                .map(|scanned| Self::value(&mut ops, scanned))
            {
                Some(Ok(value)) => values.push(Some(value)),
                Some(Err(reason)) => return EvalAnswer::Declined(reason),
                None => values.push(None),
            }
        }
        let ended_first = outcome.nconv == 0 && outcome.eof_before_conv;
        let publication = if vars.is_empty() {
            if values.is_empty() || ended_first {
                Publication::preserving(ConstValue::text(""), TclType::String, &targets)
            } else {
                // A field the input never reached is the empty string too.
                let mut items: Vec<ConstValue> = values
                    .into_iter()
                    .map(|value| value.unwrap_or_else(|| ConstValue::text("")))
                    .collect();
                items.resize(items.len().max(fields), ConstValue::text(""));
                let list = ops.new_list(items);
                Publication::preserving(list, TclType::List, &targets)
            }
        } else {
            if !targets_are(&targets, 2, vars.len()) {
                return EvalAnswer::Declined(DeclineReason::Unsupported);
            }
            let converted = if ended_first {
                0
            } else {
                values.iter().take_while(|value| value.is_some()).count()
            };
            let mut values = values.into_iter().flatten();
            let stores = targets
                .iter()
                .enumerate()
                .map(|(at, id)| match values.next() {
                    Some(value) if at < converted => PendingStore::Write(TargetId(*id), value),
                    _ => PendingStore::Preserve(TargetId(*id)),
                })
                .collect();
            Publication {
                result: ConstValue::int(if ended_first {
                    -1
                } else {
                    i64::try_from(converted).unwrap_or(i64::MAX)
                }),
                result_type: TclType::Int,
                stores,
            }
        };
        publication.publish(ops, NativeEvalId::ScanFormat, DESTRUCTURE_REVISION)
    }
}

impl CommandSemantics for ScanSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ScanFormat.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ScanFormat,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        Self::evaluate_scan(input, budget)
    }
}

/// `binary scan string formatString ?varName …?` on the direct route: the
/// shared unpacker over the string's bytes. The count of the fields it
/// scanned is the result; each scanned field writes its variable — a byte
/// array for `a` and `A`, a digit string for `b`, `B`, `h` and `H`, a
/// number for a countless numeric field and a list of them for a counted
/// one — and a variable whose field the data did not reach is preserved.
/// A format with more value fields than variables is not evaluated: the
/// command raises when it reaches such a field with data left, and answers
/// when the data runs out first (`binary scan \x01 ccc a b` is 1), and the
/// route does not follow where the data runs out. A field letter or the `u`
/// suffix a named release lacks, and a float field under 8.4, whose double
/// spelling differs, are not evaluated; nor is a character above `U+00FF`,
/// which 8.x truncates to its low byte and 9.x refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryScanSemantics;

/// `binary scan`.
pub static BINARY_SCAN: BinaryScanSemantics = BinaryScanSemantics;

impl BinaryScanSemantics {
    /// The axes the route reads: the field set, the crossing of characters
    /// to bytes, and the source decoding of a non-ASCII operand.
    pub const NEEDS: Needs = Needs::BINARY_FIELDS
        .union(Needs::BYTE_STRINGS)
        .union(Needs::SOURCE_ENCODING);

    /// Whether `letter` is a float field, whose values are doubles.
    const fn is_float(letter: u8) -> bool {
        matches!(letter, b'f' | b'r' | b'R' | b'd' | b'q' | b'Q')
    }

    /// Whether `letter` stores a value: every field but the cursor moves.
    const fn stores(letter: u8) -> bool {
        !matches!(letter, b'x' | b'X' | b'@')
    }

    /// The fields that store a value, in order, with whether each carries a
    /// count — or the decline for a field the target's releases do not all
    /// read alike.
    fn fields(ops: &ConstOps<'_>, fmt: &str) -> Result<Vec<(u8, bool)>, DeclineReason> {
        let unsigned = at_least(ops, TclVersion::V8_5);
        let mut fields = Vec::new();
        for field in tcl_cmd_core::binary::specifiers(fmt.as_bytes(), unsigned) {
            if let Some(floor) = tcl_cmd_core::binary::specifier_min_version(field.letter)
                && !at_least(ops, floor)
            {
                return Err(unavailable(SpecSurface::TCL85_PLUS));
            }
            if Self::is_float(field.letter) && !at_least(ops, TclVersion::V8_5) {
                return Err(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar));
            }
            if Self::stores(field.letter) {
                fields.push((field.letter, field.star || field.count.is_some()));
            }
        }
        // The `u` suffix is ordinary text before 8.5: `binary scan … su v`
        // is `bad field specifier "u"` on 8.4. The specifier scan treats it
        // as text there, and the unpacker would read it as a suffix.
        if !unsigned && fmt.contains('u') {
            return Err(unavailable(SpecSurface::TCL85_PLUS));
        }
        Ok(fields)
    }

    /// One scanned field's value as the command builds it: the unpacker's
    /// bytes, a byte array's crossed back to the characters `U+0000` to
    /// `U+00FF` its string spells.
    fn value(letter: u8, counted: bool, bytes: &[u8]) -> ConstValue {
        match letter {
            b'a' | b'A' => {
                let text: String = bytes.iter().copied().map(char::from).collect();
                ConstValue::bytes(text.as_bytes(), Representation::ByteArray)
            }
            b'b' | b'B' | b'h' | b'H' => ConstValue::bytes(bytes, Representation::String),
            _ if counted => ConstValue::bytes(bytes, Representation::List),
            _ if Self::is_float(letter) => ConstValue::bytes(bytes, Representation::Double),
            _ => ConstValue::bytes(bytes, Representation::Int),
        }
    }

    fn evaluate_binary_scan(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let (mut ops, texts, targets) = match open_words(input, budget, Self::NEEDS) {
            Ok(opened) => opened,
            Err(answer) => return answer,
        };
        // Operand 0 is the subcommand word.
        let [_, string, format, vars @ ..] = texts.as_slice() else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        let Some(data) = string
            .chars()
            .map(|c| u8::try_from(u32::from(c)).ok())
            .collect::<Option<Vec<u8>>>()
        else {
            return EvalAnswer::Declined(DeclineReason::ReleaseAmbiguous(Axis::ByteStrings));
        };
        let fields = match Self::fields(&ops, format) {
            Ok(fields) => fields,
            Err(reason) => return EvalAnswer::Declined(reason),
        };
        // A value field without a variable raises once the scan reaches it
        // with data left; where the data runs out is not followed, so any
        // such format declines.
        if fields.len() > vars.len() {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        }
        if !targets_are(&targets, 3, vars.len()) {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        // The unpacker's work: one unit per data byte.
        if let Err(reason) = ops.charge(u64::try_from(data.len()).unwrap_or(u64::MAX)) {
            return EvalAnswer::Declined(reason);
        }
        let Ok(scanned) = tcl_cmd_core::binary::scan(&data, format.as_bytes()) else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        if scanned.len() > fields.len() {
            return EvalAnswer::Declined(DeclineReason::MalformedAnswer);
        }
        let mut values = scanned
            .iter()
            .zip(&fields)
            .map(|(bytes, &(letter, counted))| Self::value(letter, counted, bytes));
        let stores = targets
            .iter()
            .map(|id| match values.next() {
                Some(value) => PendingStore::Write(TargetId(*id), value),
                None => PendingStore::Preserve(TargetId(*id)),
            })
            .collect();
        Publication {
            result: ConstValue::int(i64::try_from(scanned.len()).unwrap_or(i64::MAX)),
            result_type: TclType::Int,
            stores,
        }
        .publish(ops, NativeEvalId::BinaryScan, DESTRUCTURE_REVISION)
    }
}

impl CommandSemantics for BinaryScanSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::BinaryScan.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::BinaryScan,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        Self::evaluate_binary_scan(input, budget)
    }
}

/// `lassign list ?varName …?` on the direct route: the list's elements
/// written to the variables in order, the empty string past its end, and
/// the elements left over as the result, rendered as one list. Two
/// variables spelling one place compose in order, so the last wins. The
/// command arrives in 8.5, and its variables become optional in 8.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LassignSemantics;

/// `lassign`.
pub static LASSIGN: LassignSemantics = LassignSemantics;

impl LassignSemantics {
    /// The axes the route reads: the list's parse and the result's
    /// rendering, and the source decoding of a non-ASCII operand.
    pub const NEEDS: Needs = Needs::LIST_RENDERING.union(Needs::SOURCE_ENCODING);

    fn evaluate_lassign(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let (mut ops, texts, targets) = match open_words(input, budget, Self::NEEDS) {
            Ok(opened) => opened,
            Err(answer) => return answer,
        };
        let [list, vars @ ..] = texts.as_slice() else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        if !at_least(&ops, TclVersion::V8_5) {
            return EvalAnswer::Declined(unavailable(SpecSurface::TCL85_PLUS));
        }
        if vars.is_empty() && !at_least(&ops, TclVersion::V8_6) {
            return EvalAnswer::Declined(unavailable(SpecSurface::TCL86_PLUS));
        }
        if !targets_are(&targets, 1, vars.len()) {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let Ok(mut elements) = ops.list_elements(&ConstValue::text(list)) else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        let rest = elements.split_off(vars.len().min(elements.len()));
        let mut elements = elements.into_iter();
        let stores = targets
            .iter()
            .map(|id| {
                PendingStore::Write(
                    TargetId(*id),
                    elements.next().unwrap_or_else(|| ConstValue::text("")),
                )
            })
            .collect();
        let result = ops.new_list(rest);
        Publication {
            result,
            result_type: TclType::List,
            stores,
        }
        .publish(ops, NativeEvalId::ListAssign, DESTRUCTURE_REVISION)
    }
}

impl CommandSemantics for LassignSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ListAssign.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ListAssign,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        Self::evaluate_lassign(input, budget)
    }
}

/// `array set arrayName list` on the direct route: one element write per
/// key of the list's pairs, a repeated key holding its last value, and the
/// empty result. The elements no pair names are the array's own and are not
/// stated; an odd-length list raises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArraySetSemantics;

/// `array set`.
pub static ARRAY_SET: ArraySetSemantics = ArraySetSemantics;

impl ArraySetSemantics {
    /// The axes the route reads: the list's parse, and the source decoding
    /// of a non-ASCII operand.
    pub const NEEDS: Needs = Needs::LIST_RENDERING.union(Needs::SOURCE_ENCODING);

    fn evaluate_array_set(input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        let (mut ops, texts, targets) = match open_words(input, budget, Self::NEEDS) {
            Ok(opened) => opened,
            Err(answer) => return answer,
        };
        // Operand 0 is the subcommand word.
        let [_, _, list] = texts.as_slice() else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        if !targets_are(&targets, 1, 1) {
            return EvalAnswer::Declined(DeclineReason::Unsupported);
        }
        let Ok(elements) = ops.list_elements(&ConstValue::text(list)) else {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        };
        if elements.len() % 2 != 0 {
            return EvalAnswer::Declined(DeclineReason::WrongRepresentation);
        }
        let array = TargetId(OperandId(1));
        let mut pairs: Vec<(String, ConstValue)> = Vec::with_capacity(elements.len() / 2);
        for [key, value] in elements.as_chunks::<2>().0 {
            let Some(key) = key.as_utf8() else {
                return EvalAnswer::Declined(DeclineReason::NotText);
            };
            let value = value.clone();
            if let Some(held) = pairs.iter_mut().find(|(held, _)| held == key) {
                held.1 = value;
            } else {
                pairs.push((key.to_owned(), value));
            }
        }
        Publication {
            result: ConstValue::text(""),
            result_type: TclType::String,
            stores: pairs
                .into_iter()
                .map(|(key, value)| PendingStore::WriteElement(array, key, value))
                .collect(),
        }
        .publish(ops, NativeEvalId::ArraySet, DESTRUCTURE_REVISION)
    }
}

impl CommandSemantics for ArraySetSemantics {
    fn identity(&self) -> &'static str {
        NativeEvalId::ArraySet.as_str()
    }

    fn route(&self) -> EvalRoute {
        EvalRoute::Direct {
            id: NativeEvalId::ArraySet,
        }
    }

    fn evaluate(&self, input: &dyn AnalysisInputs, budget: &mut Budget) -> EvalAnswer {
        Self::evaluate_array_set(input, budget)
    }
}
