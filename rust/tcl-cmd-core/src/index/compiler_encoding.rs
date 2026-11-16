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

//! Literal index words as each release's compiler encodes them ahead of the length they index.

use super::reading::{
    Range, end_32, exact_sum, int_32, parse_int_prefix, parse_int_whole, signed_operand, sum_head,
    whole_number, wrap_32,
};
use super::{IndexReading, narrow_tcl_integer, read_under, resolve_opt_in};
use tcl_dialect::TclVersion;
use tcl_syntax::number::{Number, NumberSyntax};
use tcl_syntax::value::ValueError;

/// Whether `release` reads `spec` apart when it is a literal its compiler
/// encodes: 8.6 compiles a literal index of `string index`, `string range`,
/// `lindex` and `lrange` through `TclIndexEncode`, which places an `end`
/// offset whose sum with `end` passes the 32-bit `int` after the end, where
/// the same index read at run time wraps before the first element (measured,
/// tclsh 8.6.18: `string range abcdefghijkl 0 end+2147483647` is the whole
/// string, and the empty one with the index in a variable).
#[must_use]
pub fn compiles_apart(spec: &str, len: usize, release: TclVersion) -> bool {
    release == TclVersion::V8_6
        && spec.starts_with('e')
        && spec
            .strip_prefix("end")
            .and_then(|rest| signed_operand(rest, release.number_syntax()))
            .is_some_and(|(negative, number)| {
                let offset = match int_32(&number, Range::Tcl86) {
                    IndexReading::At(value) if negative => wrap_32(-value),
                    IndexReading::At(value) => value,
                    _ => return false,
                };
                offset > 0 && end_32(len) + offset > i64::from(i32::MAX)
            })
}

/// A literal index word fixed ahead of the length it indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoded {
    /// This index, zero or more.
    Index(i64),
    /// `end` plus this offset.
    FromEnd(i64),
    /// Before the first element, whatever the length.
    Before,
    /// After the last element, whatever the length.
    After,
}

/// The literal index word `spec` as `release` reads it whatever the length,
/// `None` for a word it reads apart by length or by host, or not at all.
///
/// `compiled` says whether the release compiles the word with its command, as
/// it does a literal in any body it compiles, rather than reading it as the
/// command runs: 8.6 compiles an index of `string range`, `lindex` and `lrange`
/// through `TclIndexEncode`, which places an `end` offset that is positive
/// after the 32-bit wrap after the end, and one below `INT_MIN + 2` before the
/// first element, whatever the length. Read as the command runs, 8.4 to 8.6
/// sum `end` and its offset in an `int`, so a positive offset passes
/// `INT_MAX` and wraps for a long enough value, and that word has no fixed
/// reading. Measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0: see
/// `a_literal_index_is_fixed_as_each_release_compiles_or_reads_it`.
#[must_use]
pub fn encode(spec: &str, release: TclVersion, compiled: bool) -> Option<Encoded> {
    let at = |len| match read_under(spec, len, release) {
        IndexReading::At(index) => Some(index),
        IndexReading::Bad | IndexReading::HostLong(_) | IndexReading::Unsure => None,
    };
    let (one, two) = (at(1)?, at(2)?);
    if one == two {
        return Some(if one < 0 {
            Encoded::Before
        } else {
            Encoded::Index(one)
        });
    }
    // An `end` word: against one element `end` is 0, so the reading is the
    // offset, as the release reads it.
    let offset = one;
    let int_min = i64::from(i32::MIN);
    if compiled && release == TclVersion::V8_6 {
        return Some(if offset > 0 {
            Encoded::After
        } else if offset < int_min + 2 {
            Encoded::Before
        } else {
            Encoded::FromEnd(offset)
        });
    }
    let sums_in_int = matches!(
        release,
        TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6
    );
    // An `int` sum holds for every length only from `INT_MIN + 1` to zero: a
    // positive offset wraps for a long enough value, and `INT_MIN` for an
    // empty one.
    (!sums_in_int || (int_min < offset && offset <= 0)).then_some(Encoded::FromEnd(offset))
}

/// Whether `release` compiles the literal index word `spec` to a reading apart
/// from the one its command makes as it runs ([`encode`]), for some length: 8.6
/// places an `end` offset that passes `INT_MAX` after the end, where its
/// command wraps it before the first element.
#[must_use]
pub fn encodes_apart(spec: &str, release: TclVersion) -> bool {
    encode(spec, release, true) != encode(spec, release, false)
}

/// Whether `release`'s compiler reads some literal index word apart from its
/// command ([`encodes_apart`]), so that a runtime evaluating a command it
/// compiles needs its literal words: 8.6's `TclIndexEncode` alone.
#[must_use]
pub const fn compiles_literals_apart(release: TclVersion) -> bool {
    matches!(release, TclVersion::V8_6)
}

/// A command whose literal index words 8.6's compiler encodes
/// (`TclGetIndexFromToken`), each by the rule of its compile procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexCompiler {
    /// `lindex list index`, three words only (`TclCompileLindexCmd`).
    Lindex,
    /// `lrange list first last`, both literal (`TclCompileLrangeCmd`).
    Lrange,
    /// `linsert list index ?element ...?` (`TclCompileLinsertCmd`).
    Linsert,
    /// `lreplace list first last ?element ...?`, both literal and in an
    /// order the compiler can settle (`TclCompileLreplaceCmd`).
    Lreplace,
    /// `string range string first last` (`TclCompileStringRangeCmd`): a
    /// `first` after the end settles the command alone, else both literal.
    StringRange,
    /// `string replace string first last ?newString?`, both literal and as
    /// [`string_replace_compiles_encoded`] says (`TclCompileStringReplaceCmd`).
    StringReplace {
        /// Whether the command has a replacement string.
        replacing: bool,
    },
}

impl IndexCompiler {
    /// How many leading words select the compile procedure: the command word,
    /// and an ensemble's subcommand word (`TclCompileEnsemble`). The release
    /// compiles the command only where each is one literal word; a
    /// substituted one leaves it to the generic invocation, which reads every
    /// index word as the command runs.
    #[must_use]
    pub const fn selecting_words(self) -> usize {
        match self {
            Self::StringRange | Self::StringReplace { .. } => 2,
            Self::Lindex | Self::Lrange | Self::Linsert | Self::Lreplace => 1,
        }
    }
}

/// The index words of a command `compiler` compiles, each respelt where
/// `release` compiles it to a reading apart from the one its command makes as
/// it runs ([`encodes_apart`]): `2147483647` after the end and `-1` before
/// the first element, which every release's command reads so whatever the
/// length. `words` holds each index word's text where it is a literal and
/// `None` where it is substituted; the answer is aligned with it, `None`
/// where the word is passed as written.
#[must_use]
pub fn compiled_index_words(
    compiler: IndexCompiler,
    words: &[Option<&str>],
    release: TclVersion,
) -> Vec<Option<&'static str>> {
    let encoded: Vec<Option<Encoded>> = words
        .iter()
        .map(|word| word.and_then(|word| encode(word, release, true)))
        .collect();
    let all = encoded.iter().all(Option::is_some);
    let settled: Vec<bool> = match (compiler, encoded.as_slice()) {
        (IndexCompiler::StringRange, [Some(Encoded::After), _]) => vec![true, false],
        (IndexCompiler::Lreplace, [Some(first), Some(last)]) => {
            vec![lreplace_compiles_encoded(*first, *last); 2]
        }
        (IndexCompiler::StringReplace { replacing }, [Some(first), Some(last)]) => {
            vec![string_replace_compiles_encoded(*first, *last, replacing); 2]
        }
        (IndexCompiler::Lreplace | IndexCompiler::StringReplace { .. }, _) => {
            vec![false; words.len()]
        }
        _ => vec![all; words.len()],
    };
    words
        .iter()
        .zip(encoded)
        .zip(settled)
        .map(|((word, encoded), settled)| {
            if !settled || !word.is_some_and(|word| encodes_apart(word, release)) {
                return None;
            }
            match encoded? {
                Encoded::After => Some("2147483647"),
                Encoded::Before => Some("-1"),
                Encoded::Index(_) | Encoded::FromEnd(_) => None,
            }
        })
        .collect()
}

/// `TclIndexEncode`'s spaces: `TCL_INDEX_END` is -2 with `end-N` below it,
/// `TCL_INDEX_BEFORE` -1, `TCL_INDEX_START` 0 and `TCL_INDEX_AFTER` `INT_MAX`.
const ENCODED_BEFORE: i64 = -1;

const ENCODED_END: i64 = -2;

const ENCODED_START: i64 = 0;

const ENCODED_AFTER: i64 = i32::MAX as i64;

/// `encoded` in `TclIndexEncode`'s space, with what `before` and `after` the
/// first element and the end stand for.
fn encoded_value(encoded: Encoded, before: i64, after: i64) -> i64 {
    match encoded {
        Encoded::Index(index) if index >= ENCODED_AFTER => after,
        Encoded::Index(index) => index,
        Encoded::FromEnd(offset) => ENCODED_END + offset,
        Encoded::Before => before,
        Encoded::After => after,
    }
}

/// Whether 8.6's `TclCompileLreplaceCmd` compiles `lreplace` on its encoded
/// indices: where it can tell which of `first` and `last + 1` the kept suffix
/// starts at.
#[must_use]
pub fn lreplace_compiles_encoded(first: Encoded, last: Encoded) -> bool {
    let first = encoded_value(first, ENCODED_START, ENCODED_AFTER);
    let last = encoded_value(last, ENCODED_BEFORE, ENCODED_END);
    first == ENCODED_AFTER
        || last == ENCODED_BEFORE
        || last == ENCODED_END
        || (last < ENCODED_END && first <= ENCODED_END)
        || (last >= ENCODED_START && first >= ENCODED_START)
}

/// Whether 8.6's `TclCompileStringReplaceCmd` compiles `string replace` on
/// its encoded indices, rather than falling through to `INST_STR_REPLACE`
/// over the words as written: always without a replacement string, and with
/// one only for a no-op it can prove, `first` before the first element, or
/// `last` after the end with `first` relative to `end`.
#[must_use]
pub fn string_replace_compiles_encoded(first: Encoded, last: Encoded, replacing: bool) -> bool {
    let first = encoded_value(first, ENCODED_BEFORE, ENCODED_AFTER);
    let last = encoded_value(last, ENCODED_BEFORE, ENCODED_AFTER);
    let no_op = last == ENCODED_BEFORE
        || first == ENCODED_AFTER
        || (first <= ENCODED_END && last <= ENCODED_END && last < first)
        || (first >= ENCODED_START && last >= ENCODED_START && last < first);
    no_op
        || !replacing
        || (first == ENCODED_BEFORE && last >= ENCODED_START)
        || (last == ENCODED_AFTER && first <= ENCODED_END)
}

/// How a release's `TclCompileStringReplaceCmd` settles a `string replace`
/// on its literal indices, so what the compiled command reads and returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringReplaceSettlement {
    /// A replacement it proves changes nothing: the string itself, unread.
    Unchanged,
    /// The whole string replaced: the replacement itself, unread.
    Replacement,
    /// The whole string removed: the empty string.
    Empty,
    /// The kept part read by character (`INST_STR_RANGE_IMM`) and joined.
    Ranges,
    /// Left to `INST_STR_REPLACE` over the words as written.
    Runtime,
}

/// [`StringReplaceSettlement`] for the indices `first` and `last` as the
/// release encodes them (`None` for a word it does not settle), with a
/// replacement string where `replacing` says; it settles exactly where
/// [`string_replace_compiles_encoded`] says it compiles.
#[must_use]
pub fn settle_string_replace(
    first: Option<Encoded>,
    last: Option<Encoded>,
    replacing: bool,
) -> StringReplaceSettlement {
    let (Some(first), Some(last)) = (first, last) else {
        return StringReplaceSettlement::Runtime;
    };
    if !string_replace_compiles_encoded(first, last, replacing) {
        return StringReplaceSettlement::Runtime;
    }
    let first = encoded_value(first, ENCODED_BEFORE, ENCODED_AFTER);
    let last = encoded_value(last, ENCODED_BEFORE, ENCODED_AFTER);
    let no_op = last == ENCODED_BEFORE
        || first == ENCODED_AFTER
        || (first <= ENCODED_END && last <= ENCODED_END && last < first)
        || (first >= ENCODED_START && last >= ENCODED_START && last < first);
    if no_op {
        StringReplaceSettlement::Unchanged
    } else if replacing && first == ENCODED_BEFORE && last == ENCODED_AFTER {
        StringReplaceSettlement::Replacement
    } else if !replacing
        && (first == ENCODED_BEFORE || first == ENCODED_START)
        && (last == ENCODED_END || last == ENCODED_AFTER)
    {
        StringReplaceSettlement::Empty
    } else {
        StringReplaceSettlement::Ranges
    }
}

/// Whether a literal C Tcl index fits the native compiler's immediate encoding.
/// The wide releases can depend on the target's container-size width; those
/// ranges retain an unknown result rather than assuming the host compiling us.
#[must_use]
pub fn compiler_encodable_in(spec: &str, syntax: tcl_dialect::IndexSyntax) -> Option<bool> {
    use tcl_dialect::{IndexGrammar, IndexIntegerWidth};
    if syntax.grammar == IndexGrammar::Jim {
        return None;
    }
    let value = resolve_opt_in(spec, 1, syntax);
    if syntax.width == IndexIntegerWidth::Tcl32 {
        return Some(value.is_some());
    }
    let value = value?;
    let end_relative = spec.starts_with("end");
    if end_relative {
        return (value > -i64::from(i32::MAX)).then_some(true);
    }
    if parse_int_whole(spec, syntax.numbers).is_none()
        && parse_int_prefix(spec.trim_start(), syntax.numbers)?.0 < 0
    {
        // The native end-offset internal representation also distinguishes
        // arithmetic whose original base is negative.
        return None;
    }
    (value <= i64::from(i32::MAX) || value >= i64::MAX - 1).then_some(true)
}

/// Compile-known list-index encoding after native token-shape validation.
/// `Ok(None)` means the native compiler evaluates the original index object;
/// `Err` retains target-width uncertainty rather than choosing that path.
///
/// # Errors
/// Reports unavailable native container-size width for an immediate operand.
pub fn compiled_list_index_in(
    spec: &str,
    version: tcl_dialect::TclVersion,
) -> Result<Option<tcl_syntax::native_compiled_index::NativeCompiledListIndex>, ValueError> {
    use tcl_syntax::native_compiled_index::NativeCompiledListIndex as Index;
    let syntax = tcl_dialect::IndexSyntax::for_version(version);
    if version == tcl_dialect::TclVersion::V8_4 {
        return Ok(None);
    }
    if version == tcl_dialect::TclVersion::V8_5 {
        return Ok(parse_int_whole(spec, syntax.numbers)
            .and_then(|value| narrow_tcl_integer(value, syntax))
            .and_then(|value| i32::try_from(value).ok())
            .filter(|value| *value >= 0)
            .map(Index::from_encoded));
    }
    compiled_list_bound_in(spec, version, -1, -1)
}

/// Encode a compile-known list coordinate with its native boundary policy.
/// The native range and insertion compilers clamp before/after coordinates
/// differently from a member lookup. This does not construct an index object.
///
/// # Errors
/// Reports native container-width uncertainty rather than choosing an opcode.
pub fn compiled_list_bound_in(
    spec: &str,
    version: tcl_dialect::TclVersion,
    before: i32,
    after: i32,
) -> Result<Option<tcl_syntax::native_compiled_index::NativeCompiledListIndex>, ValueError> {
    use tcl_syntax::native_compiled_index::NativeCompiledListIndex as Index;
    let syntax = tcl_dialect::IndexSyntax::for_version(version);
    if version < tcl_dialect::TclVersion::V8_6 {
        return Ok(None);
    }
    if syntax.width == tcl_dialect::IndexIntegerWidth::Tcl64 {
        return Ok(c9_index_encode(spec, syntax.numbers, before, after).map(Index::from_encoded));
    }
    let Some(value) = resolve_opt_in(spec, 1, syntax) else {
        return Ok(None);
    };
    if compiler_encodable_in(spec, syntax).is_none() {
        return Err(ValueError::CommandProtocolUnavailable(
            "compiled index target width",
        ));
    }
    let encoded = if spec.starts_with("end") {
        if value > 0 {
            after
        } else if value < i64::from(i32::MIN) + 2 {
            before
        } else {
            i32::try_from(value - 2).expect("native encoded end offset")
        }
    } else if value < 0 {
        before
    } else if (version < tcl_dialect::TclVersion::V9_0 && value == i64::from(i32::MAX))
        || value > i64::from(i32::MAX)
    {
        after
    } else {
        i32::try_from(value).expect("native encoded absolute index")
    };
    Ok(Some(Index::from_encoded(encoded)))
}

/// C9's `TclIndexEncode` on the supported 64-bit host, where `Tcl_Size` is
/// wider than `int`. `None` is `TCL_ERROR`: a word that does not parse, or a
/// range error, which leaves the compiler to evaluate the index object.
fn c9_index_encode(spec: &str, numbers: NumberSyntax, before: i32, after: i32) -> Option<i32> {
    // `GetWideForIndex`'s end value and `LIST_MAX` for 8-byte slots behind a
    // 40-byte `ListStore` header.
    const END_VALUE: i128 = 2 * i32::MAX as i128;
    const WIDE_MAX: i128 = i64::MAX as i128;
    const WIDE_MIN: i128 = i64::MIN as i128;
    const LIST_MAX: i128 = (WIDE_MAX - 40) / 8;
    const INT_MAX: i128 = i32::MAX as i128;
    let clamp = |value: i128| value.clamp(WIDE_MIN, WIDE_MAX);
    // The value and whether an end-offset rep with a negative offset remains.
    let (wide, end_relative) = if let Some(number) = whole_number(spec, numbers) {
        let value = match number {
            Number::Int(value) if value < 0 => -1,
            Number::Int(value) => i128::from(value),
            Number::Big { negative: true, .. } => WIDE_MIN,
            Number::Big {
                negative: false, ..
            } => WIDE_MAX,
            _ => return None,
        };
        (value, false)
    } else {
        let offset = if spec == "end" {
            -1
        } else if let Some(rest) = spec.strip_prefix("end") {
            let (subtract, number) = signed_operand(rest, numbers)?;
            match number {
                Number::Big { negative, .. } => {
                    if negative == subtract {
                        WIDE_MAX
                    } else {
                        WIDE_MIN
                    }
                }
                Number::Int(value) => {
                    let value = i128::from(value);
                    let offset = match (subtract, value == WIDE_MIN) {
                        (true, true) => WIDE_MAX,
                        (true, false) => -value,
                        (false, _) => value,
                    };
                    match offset {
                        1 => WIDE_MAX,
                        offset if offset > 1 => WIDE_MAX - 1,
                        WIDE_MIN => WIDE_MIN,
                        offset => offset - 1,
                    }
                }
                _ => return None,
            }
        } else {
            let (first, rest) = sum_head(spec, numbers)?;
            let (subtract, second) = signed_operand(rest, numbers)?;
            match clamp(exact_sum(&first, subtract, &second)?) {
                -1 => WIDE_MIN,
                sum if sum < 0 => WIDE_MIN + 1,
                sum => sum,
            }
        };
        let wide = match offset {
            WIDE_MAX => END_VALUE + 1,
            WIDE_MIN => -1,
            offset if offset < 0 => END_VALUE + offset + 1,
            offset => offset,
        };
        (wide, offset < 0)
    };
    if end_relative {
        if wide > END_VALUE - LIST_MAX && wide <= INT_MAX {
            return None;
        }
        return Some(if wide > END_VALUE {
            after
        } else if wide <= INT_MAX {
            before
        } else {
            // `(int)wide`: the end-relative encoding below `INT_MIN`'s wrap.
            i32::try_from(wide - (1_i128 << 32)).ok()?
        });
    }
    if wide > INT_MAX && wide < WIDE_MAX - 1 {
        return None;
    }
    Some(if wide > INT_MAX {
        after
    } else if wide < 0 {
        before
    } else {
        i32::try_from(wide).ok()?
    })
}
