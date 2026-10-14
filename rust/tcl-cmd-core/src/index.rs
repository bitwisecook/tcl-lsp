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

//! Shared Tcl index parsing (`Tcl_GetIntForIndex` / `GetEndOffsetFromObj`).
//!
//! One parser for every indexed command — `string index/range`, `lindex`,
//! `lrange`, `linsert`, `lreplace`, … — so the accepted forms and the error
//! message live once. The grammar is the release's: an integer, `end` with an
//! offset (`end-2`, `end--1` = `end - (-1)`), and from 8.5 `end+N` and the
//! sums `1+1` and `0-1`, each spelt and each integer read as that release
//! reads it (`read`).

use tcl_dialect::TclVersion;
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;
use tcl_syntax::number::{Number, NumberSyntax, ParseFlags, Radix};

/// One index word as a release reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexReading {
    /// The index the word names, negative or past the end as it falls.
    At(i64),
    /// No index: the release raises `bad index`.
    Bad,
    /// The index where the host's C `long` is 64 bits and `bad index` where
    /// it is 32 bits — 64-bit Windows and every 32-bit build.
    HostLong(i64),
    /// A word 8.5 reads as [`Self::HostLong`] and 8.6 raises on, read under
    /// the numeral grammar the two releases share with neither named.
    Unsure,
}

impl IndexReading {
    /// The index a runtime on this host reads: C Tcl's on the same host.
    #[must_use]
    pub fn on_this_host(self) -> Option<i64> {
        match self {
            Self::At(index) => Some(index),
            Self::HostLong(index) if tcl_syntax::value::host_long_is_wide() => Some(index),
            Self::HostLong(_) | Self::Bad | Self::Unsure => None,
        }
    }

    fn map(self, f: impl FnOnce(i64) -> i64) -> Self {
        match self {
            Self::At(value) => Self::At(f(value)),
            Self::HostLong(value) => Self::HostLong(f(value)),
            other => other,
        }
    }

    fn zip(self, other: Self, f: impl FnOnce(i64, i64) -> i64) -> Self {
        match (self, other) {
            (Self::Bad, _) | (_, Self::Bad) => Self::Bad,
            (Self::Unsure, _) | (_, Self::Unsure) => Self::Unsure,
            (Self::At(a), Self::At(b)) => Self::At(f(a, b)),
            (Self::At(a) | Self::HostLong(a), Self::At(b) | Self::HostLong(b)) => {
                Self::HostLong(f(a, b))
            }
        }
    }
}

/// Resolve an index `spec` against a container length `len` (so `end` is
/// `len - 1`) under the numeral grammar `numbers`, as a runtime on this host
/// reads it. The result may be negative or `>= len`; callers clamp per their
/// command's rules. Errors with the canonical message on an unparseable spec.
/// The numeral grammar 8.5 and 8.6 share reads as 8.6 here.
///
/// # Errors
/// `bad index` for a spec `numbers` does not read as an index on this host.
pub fn resolve_with(spec: &str, len: usize, numbers: NumberSyntax) -> Result<i64, CmdError> {
    read_with(spec, len, numbers)
        .on_this_host()
        .ok_or_else(|| bad_index(spec.trim()))
}

/// [`resolve_with`] under the grammar this thread's interpreter installed
/// (`tcl_syntax::number::runtime_syntax`). Runtime-only: a caller under a
/// stated profile — a registry folder, the compiler, a test running two
/// releases in one process — names its grammar through [`resolve_with`] or
/// [`resolve_opt_with`] instead, since the ambient is whatever the thread
/// last installed.
///
/// # Errors
/// As [`resolve_with`].
pub fn resolve(spec: &str, len: usize) -> Result<i64, CmdError> {
    resolve_with(spec, len, tcl_syntax::number::runtime_syntax())
}

/// Resolve an index `spec` against `len`, returning `None` (rather than an
/// error) on an unparseable spec — the `lsearch`/`lsort -index` driving needs the
/// raw option to classify "bad index" vs "out of range" itself. Runtime-only,
/// as [`resolve`] is.
#[must_use]
pub fn resolve_opt(spec: &str, len: usize) -> Option<i64> {
    read_with(spec, len, tcl_syntax::number::runtime_syntax()).on_this_host()
}

/// [`resolve_opt`] reading the index under an explicitly named numeral
/// grammar, for a compile-time caller whose target need not be the release
/// this process was built for: `None` also for a spec whose reading turns on
/// the host or on which release the grammar stands for, which
/// [`read_with`] tells apart.
#[must_use]
pub fn resolve_opt_with(spec: &str, len: usize, numbers: NumberSyntax) -> Option<i64> {
    match read_with(spec, len, numbers) {
        IndexReading::At(index) => Some(index),
        IndexReading::Bad | IndexReading::HostLong(_) | IndexReading::Unsure => None,
    }
}

/// One index `spec` against `len` as the numeral grammar `numbers` reads it;
/// the grammar 8.5 and 8.6 share names neither release.
#[must_use]
pub fn read_with(spec: &str, len: usize, numbers: NumberSyntax) -> IndexReading {
    read(spec, len, numbers, Range::of_numbers(numbers))
}

/// One index `spec` against `len` as `release` reads it.
#[must_use]
pub fn read_under(spec: &str, len: usize, release: TclVersion) -> IndexReading {
    read(
        spec,
        len,
        release.number_syntax(),
        Range::of_release(release),
    )
}

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

/// A failed `lsearch`/`lsort -index` path step: C's message and, where C sets
/// one, its `-errorcode`.
pub struct IndexPathError {
    pub message: Vec<u8>,
    pub code: Option<&'static [u8]>,
}

/// Why `TclIndexEncode` refuses an `-index` path element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    /// Not index syntax at all.
    Bad,
    /// An index it cannot encode.
    OutOfRange,
}

/// An `-index` path element as `TclIndexEncode` encodes it: an absolute index,
/// or one `end` plus a non-positive offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathEncoding {
    At(i64),
    FromEnd(i64),
}

impl PathEncoding {
    /// Encode `spec`, or refuse it: out of range is an absolute index outside
    /// `0..=INT_MAX`, `end+N`, or `end-N` with `N` at or past `INT_MAX`
    /// (measured on 64-bit tclsh 9.0.4). End-relativity is told apart by
    /// resolving against two different lengths.
    fn of(spec: &str) -> Result<Self, Refusal> {
        const BIG: usize = 1 << 20;
        let r_big = resolve_opt(spec, BIG + 1).ok_or(Refusal::Bad)?;
        let r_small = resolve_opt(spec, 1).ok_or(Refusal::Bad)?;
        let int_max = i64::from(i32::MAX);
        if r_big == r_small {
            return if (0..=int_max).contains(&r_big) {
                Ok(Self::At(r_big))
            } else {
                Err(Refusal::OutOfRange)
            };
        }
        let offset = r_big - i64::try_from(BIG).unwrap_or(i64::MAX);
        if (1 - int_max..=0).contains(&offset) {
            Ok(Self::FromEnd(offset))
        } else {
            Err(Refusal::OutOfRange)
        }
    }

    /// The index into a level of `len` elements.
    fn decode(self, len: usize) -> i64 {
        match self {
            Self::At(index) => index,
            Self::FromEnd(offset) => i64::try_from(len).unwrap_or(i64::MAX) - 1 + offset,
        }
    }
}

/// Check every `-index` path element encodes (`TclIndexEncode` at option
/// parse): a syntactically bad spec is `bad index`, an unencodable one `index
/// "…" out of range`.
///
/// # Errors
/// The first element that does not encode.
pub fn check_index_path(path: &[Vec<u8>]) -> Result<(), IndexPathError> {
    for spec in path {
        PathEncoding::of(&String::from_utf8_lossy(spec)).map_err(|refusal| path_error(spec, refusal))?;
    }
    Ok(())
}

fn path_error(spec: &[u8], refusal: Refusal) -> IndexPathError {
    match refusal {
        Refusal::Bad => {
            let mut message = b"bad index \"".to_vec();
            message.extend_from_slice(spec);
            message.extend_from_slice(b"\": must be integer?[+-]integer? or end?[+-]integer?");
            IndexPathError {
                message,
                code: Some(b"TCL VALUE INDEX"),
            }
        }
        Refusal::OutOfRange => {
            let mut message = b"index \"".to_vec();
            message.extend_from_slice(spec);
            message.extend_from_slice(b"\" out of range");
            IndexPathError {
                message,
                code: Some(b"TCL VALUE INDEX OUTOFRANGE"),
            }
        }
    }
}

/// Drill into a (nested) list `value` by an index `path` (`lsearch`/`lsort
/// -index`): each spec steps one level. A step past its level is `element <n>
/// missing from sublist "<list>"`, naming the decoded index as C's
/// `SelectObjFromSublists` does (`0x10` is `16`, an `end` step that falls
/// before the first element is `end-<n>`); a non-list step stops, returning the
/// current value (C's `TclLindexFlat` fallthrough).
///
/// # Errors
/// `bad index` / `out of range`, the list-parse error, or `element … missing
/// from sublist …`.
pub fn drill<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    path: &[Vec<u8>],
) -> Result<O::Value, IndexPathError> {
    let mut cur = value.clone();
    for spec in path {
        let len = ops.list_len(&cur).map_err(|e| IndexPathError {
            message: e.message().into_bytes(),
            code: e.error_code().map(str::as_bytes),
        })?;
        let encoded = PathEncoding::of(&String::from_utf8_lossy(spec))
            .map_err(|refusal| path_error(spec, refusal))?;
        let idx = encoded.decode(len);
        if idx < 0 || usize::try_from(idx).unwrap_or(usize::MAX) >= len {
            let named = match encoded {
                PathEncoding::FromEnd(offset) if idx < 0 => format!("end-{}", -offset),
                _ => idx.to_string(),
            };
            let mut message = format!("element {named} missing from sublist \"").into_bytes();
            message.extend_from_slice(&ops.as_bytes(&cur));
            message.push(b'"');
            return Err(IndexPathError {
                message,
                code: Some(b"TCL OPERATION LSORT INDEXFAILED"),
            });
        }
        match ops.list_index(&cur, usize::try_from(idx).unwrap_or(0)) {
            Ok(Some(e)) => cur = e,
            _ => return Ok(cur),
        }
    }
    Ok(cur)
}

/// Whether an index `spec` is *encodable* the way `TclIndexEncode` requires for
/// `lsearch`/`lsort -index`: `Some(true)` for a normal index (an integer in
/// `0..=INT_MAX`, or `end`/`end-N`), `Some(false)` for one it refuses as out of
/// range, and `None` for a syntactically bad spec.
#[must_use]
pub fn encodable(spec: &str) -> Option<bool> {
    match PathEncoding::of(spec) {
        Ok(_) => Some(true),
        Err(Refusal::OutOfRange) => Some(false),
        Err(Refusal::Bad) => None,
    }
}

/// Whose integer range reads an index's integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Range {
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
    const fn of_release(release: TclVersion) -> Self {
        match release {
            TclVersion::V8_4 => Self::Tcl84,
            TclVersion::V8_5 => Self::Tcl85,
            TclVersion::V8_6 => Self::Tcl86,
            TclVersion::V9_0 | TclVersion::V9_1 => Self::Wide,
        }
    }

    const fn of_numbers(numbers: NumberSyntax) -> Self {
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
fn read(spec: &str, len: usize, numbers: NumberSyntax, range: Range) -> IndexReading {
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
fn end_32(len: usize) -> i64 {
    wrap_32(i64::try_from(len).unwrap_or(i64::MAX) - 1)
}

/// `value` cast to C's 32-bit `int`: its low 32 bits, sign-extended.
const fn wrap_32(value: i64) -> i64 {
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
fn exact_sum(first: &Number, negative: bool, second: &Number) -> Option<i128> {
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
fn int_32(number: &Number, range: Range) -> IndexReading {
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
fn magnitude(radix: Radix, digits: &str) -> Option<u128> {
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
fn sum_head(spec: &str, numbers: NumberSyntax) -> Option<(Number, &str)> {
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
fn signed_operand(rest: &str, numbers: NumberSyntax) -> Option<(bool, Number)> {
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
fn index_int_flags(numbers: NumberSyntax) -> ParseFlags {
    ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(numbers)
    }
}

/// A whole integer word, whitespace allowed either side (`Tcl_GetInt`): a wide
/// or a bignum.
fn whole_number(s: &str, numbers: NumberSyntax) -> Option<Number> {
    let parsed = tcl_syntax::number::parse(s, index_int_flags(numbers))?;
    (s[parsed.end..].trim_matches(is_space).is_empty()
        && matches!(parsed.number, Number::Int(_) | Number::Big { .. }))
    .then_some(parsed.number)
}

/// Parse a leading Tcl integer, returning its value and the unconsumed tail via
/// the canonical [`tcl_syntax::number`] parser (so every radix the runtime
/// accepts resolves identically). `None` if `s` does not start with an integer,
/// or the value is a non-integer / bignum (`Int` only).
fn parse_int_prefix(s: &str, numbers: NumberSyntax) -> Option<(i64, &str)> {
    let parsed = tcl_syntax::number::parse(s, index_int_flags(numbers))?;
    match parsed.number {
        Number::Int(v) => Some((v, &s[parsed.end..])),
        _ => None,
    }
}

/// Parse `s` as a whole Tcl integer — the [`parse_int_prefix`] value only when
/// nothing but trailing space follows it.
fn parse_int_whole(s: &str, numbers: NumberSyntax) -> Option<i64> {
    let (val, tail) = parse_int_prefix(s, numbers)?;
    tail.trim_matches(is_space).is_empty().then_some(val)
}

/// The canonical `bad index "<spec>": …` error, as Tcl 8.6 and later raise it
/// ([`bad_index_under`]).
#[must_use]
pub fn bad_index(spec: &str) -> CmdError {
    bad_index_under(spec, TclVersion::V8_6)
}

/// The `bad index` error `release` raises for `spec`: 8.4 words the grammar
/// it reads (`integer or end?-integer?`), 8.5 on the sums too, and from 8.6
/// the error carries `TCL VALUE INDEX` (measured, 8.4.20 to 9.1.0).
#[must_use]
pub fn bad_index_under(spec: &str, release: TclVersion) -> CmdError {
    if release < TclVersion::V8_5 {
        return CmdError::new(format!(
            "bad index \"{spec}\": must be integer or end?-integer?"
        ));
    }
    let message = format!("bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?");
    if release < TclVersion::V8_6 {
        CmdError::new(message)
    } else {
        CmdError::with_error_code(message, "TCL VALUE INDEX")
    }
}

#[cfg(test)]
mod tests {
    use super::IndexReading::{At, Bad, HostLong};
    use super::*;

    #[test]
    fn resolve_integer_and_end_forms() {
        // Mirrors Tcl's `TclGetIntForIndex` grammar (end / signed integer /
        // base[+-]integer).
        assert_eq!(resolve("5", 10).unwrap(), 5);
        assert_eq!(resolve("0", 10).unwrap(), 0);
        assert_eq!(resolve("-1", 10).unwrap(), -1);
        assert_eq!(resolve("end", 10).unwrap(), 9);
        assert_eq!(resolve("end-2", 10).unwrap(), 7);
        assert_eq!(resolve("end+1", 10).unwrap(), 10);
        // `end--1` parses as `end - (-1)`.
        assert_eq!(resolve("end--1", 10).unwrap(), 10);
        assert_eq!(resolve("1+1", 10).unwrap(), 2);
        assert_eq!(resolve("0-1", 10).unwrap(), -1);
        // `end` of an empty container is -1.
        assert_eq!(resolve("end", 0).unwrap(), -1);
    }

    #[test]
    fn resolve_rejects_garbage_and_resolve_opt_returns_none() {
        assert!(resolve("bad", 10).is_err());
        assert!(resolve("", 10).is_err());
        assert!(resolve("1.5", 10).is_err());
        assert_eq!(resolve_opt("bad", 10), None);
        assert_eq!(resolve_opt("end-1", 10), Some(8));
    }

    #[test]
    fn resolve_matches_tclsh_oracle() {
        // Ground truth captured from real tclsh 8.6 and 9.0 (identical) via
        // `string index`/`lindex` over a length-5 container (end = 4). Each
        // `resolve_opt` returns the raw resolved index (callers clamp for
        // out-of-range); a `None` is a `bad index` error at runtime.
        let ok: &[(&str, i64)] = &[
            // integers, signs, whitespace
            ("0", 0),
            ("1", 1),
            ("4", 4),
            ("5", 5),
            ("-1", -1),
            ("-2", -2),
            ("-0", 0),
            (" 1", 1),
            ("1 ", 1),
            // end and end offsets (`end--1` = end - (-1) = end + 1)
            ("end", 4),
            ("end-1", 3),
            ("end+1", 5),
            ("end-4", 0),
            ("end-5", -1),
            ("end--1", 5),
            ("end+-1", 3),
            ("end+0", 4),
            // arithmetic operands
            ("1+1", 2),
            ("0-1", -1),
            ("2+2", 4),
            ("3-1", 2),
            ("end-2", 2),
            // radix prefixes (hex / octal / binary / 0d) — oracle accepts these
            ("0x2", 2),
            ("0xa", 10),
            ("0xf", 15),
            ("0o7", 7),
            ("0b101", 5),
            ("0d5", 5),
            ("+0x2", 2),
            ("-0x1", -1),
            // radices inside the arithmetic operands
            ("end-0x2", 2),
            ("end-0o3", 1),
            ("0x2+1", 3),
            ("1+0x1", 2),
        ];
        for &(spec, want) in ok {
            assert_eq!(resolve_opt(spec, 5), Some(want), "resolve_opt({spec:?})");
        }
        // Syntactically bad — a `bad index` error at runtime.
        for spec in ["1.0", "1+", "end-", "foo", "", "1 1", "0x", "0xg", "2e0"] {
            assert_eq!(
                resolve_opt(spec, 5),
                None,
                "resolve_opt({spec:?}) should be None"
            );
        }
    }

    #[test]
    fn encodable_classifies_specs() {
        // `TclIndexEncode` scale (used by `lsearch`/`lsort -index` parse-time
        // validation): Some(true) = encodable, Some(false) = out of range,
        // None = syntactically bad.
        assert_eq!(encodable("5"), Some(true)); // absolute, non-negative
        assert_eq!(encodable("0"), Some(true));
        assert_eq!(encodable("-1"), Some(false)); // absolute negative → out of range
        assert_eq!(encodable("end"), Some(true)); // offset 0 ≤ 0
        assert_eq!(encodable("end-2"), Some(true)); // offset -2 ≤ 0
        assert_eq!(encodable("end+1"), Some(false)); // offset +1 > 0 → out of range
        assert_eq!(encodable("bad"), None); // syntactically bad
    }

    /// An index word is read by `Tcl_GetIntForIndex`, so it inherits every
    /// version difference in the numeral grammar. Pinned on tclsh 8.6.16 /
    /// 9.0.4 against a 12-element list:
    ///
    /// ```text
    /// lindex $l 010      -> i (index 8, octal)  | k (index 10, decimal)
    /// lindex $l end-010  -> d (end-8)           | b (end-10)
    /// lindex $l 1_0      -> bad index           | k
    /// lindex $l 0d1      -> bad index           | b
    /// ```
    #[test]
    fn index_numerals_follow_the_named_release() {
        use tcl_syntax::number::NumberSyntax::{Tcl85, Tcl90};
        // (spec, len, 8.6 answer, 9.0 answer)
        let matrix = [
            ("010", 12, Some(8), Some(10)),
            ("end-010", 12, Some(3), Some(1)),
            ("1_0", 12, None, Some(10)),
            ("0d1", 12, None, Some(1)),
            // Unanimous spellings resolve the same either way.
            ("1", 12, Some(1), Some(1)),
            ("0x1", 12, Some(1), Some(1)),
            ("007", 12, Some(7), Some(7)),
            ("end", 12, Some(11), Some(11)),
            ("end-2", 12, Some(9), Some(9)),
        ];
        for (spec, len, on_86, on_90) in matrix {
            assert_eq!(
                resolve_opt_with(spec, len, Tcl85),
                on_86,
                "index {spec} on 8.6"
            );
            assert_eq!(
                resolve_opt_with(spec, len, Tcl90),
                on_90,
                "index {spec} on 9.0"
            );
        }
    }

    /// The rows past 32 bits of [`each_release_reads_an_index_as_its_tclsh_does`]:
    /// (spec, 8.4, 8.5, 8.6, 9.0 and 9.1).
    const PAST_32_BITS: &[(&str, [IndexReading; 4])] = &[
        (
            "2147483647",
            [
                At(2_147_483_647),
                At(2_147_483_647),
                At(2_147_483_647),
                At(2_147_483_647),
            ],
        ),
        (
            "2147483648",
            [
                At(-2_147_483_648),
                At(-2_147_483_648),
                At(-2_147_483_648),
                At(2_147_483_648),
            ],
        ),
        ("4294967295", [At(-1), At(-1), At(-1), At(4_294_967_295)]),
        ("0xffffffff", [At(-1), At(-1), At(-1), At(4_294_967_295)]),
        ("4294967296", [Bad, Bad, Bad, At(4_294_967_296)]),
        ("0x100000000", [Bad, Bad, Bad, At(4_294_967_296)]),
        (
            "-2147483649",
            [
                At(2_147_483_647),
                At(2_147_483_647),
                At(2_147_483_647),
                At(-2_147_483_649),
            ],
        ),
        ("-4294967295", [At(1), At(1), At(1), At(-4_294_967_295)]),
        ("-4294967296", [Bad, Bad, Bad, At(-4_294_967_296)]),
        (
            "end-2147483648",
            [
                At(-2_147_483_637),
                At(-2_147_483_637),
                At(-2_147_483_637),
                At(-2_147_483_637),
            ],
        ),
        (
            "end-4294967295",
            [At(12), At(12), At(12), At(-4_294_967_284)],
        ),
        ("end-4294967296", [Bad, Bad, Bad, At(-4_294_967_285)]),
        (
            "end+2147483647",
            [
                Bad,
                At(-2_147_483_638),
                At(-2_147_483_638),
                At(2_147_483_658),
            ],
        ),
        ("end+4294967295", [Bad, At(10), At(10), At(4_294_967_306)]),
        (
            "1+2147483647",
            [
                Bad,
                At(-2_147_483_648),
                At(-2_147_483_648),
                At(2_147_483_648),
            ],
        ),
        ("4294967295+1", [Bad, At(0), At(0), At(4_294_967_296)]),
        (
            "-2147483648-1",
            [
                Bad,
                At(2_147_483_647),
                At(2_147_483_647),
                At(-2_147_483_649),
            ],
        ),
        ("9223372036854775807", [Bad, Bad, Bad, At(i64::MAX)]),
        ("9223372036854775808", [Bad, Bad, Bad, At(i64::MAX)]),
        ("-9223372036854775809", [Bad, Bad, Bad, At(i64::MIN)]),
        ("0x8000000000000000", [Bad, Bad, Bad, At(i64::MAX)]),
        ("0x1ffffffffffffffff", [Bad, Bad, Bad, At(i64::MAX)]),
        ("1+9223372036854775807", [Bad, Bad, Bad, At(12)]),
        (
            "9223372036854775807+9223372036854775807",
            [Bad, Bad, Bad, At(12)],
        ),
        ("-9223372036854775808-1", [Bad, Bad, Bad, At(i64::MIN)]),
        ("end+9223372036854775807", [Bad, Bad, Bad, At(i64::MAX)]),
        ("end+9223372036854775808", [Bad, Bad, Bad, At(12)]),
        ("end+99999999999999999999", [Bad, Bad, Bad, At(12)]),
        ("end-9223372036854775808", [Bad, Bad, Bad, At(-1)]),
        (
            "18446744073709551615",
            [HostLong(-1), HostLong(-1), Bad, At(i64::MAX)],
        ),
        (
            "-0xffffffffffffffff",
            [HostLong(1), HostLong(1), Bad, At(i64::MIN)],
        ),
        (
            "18446744069414584321",
            [HostLong(1), HostLong(1), Bad, At(i64::MAX)],
        ),
        (
            "end-18446744073709551615",
            [HostLong(12), HostLong(12), Bad, At(-1)],
        ),
        ("0xffffffffffffffff+1", [Bad, HostLong(0), Bad, At(12)]),
    ];

    /// Each release reads one index as its own `string index` does, measured on
    /// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 over `abcdefghijkl` (8.5
    /// and 8.6 answer alike, as do 9.0 and 9.1, but for the integers the second
    /// table reads): `None` is `bad index`. Past 32 bits each release reads in
    /// its own range, measured with `string index`, `string
    /// range` from each side and `lset`: 8.4 to 8.6 wrap an integer within
    /// ±4294967295 to 32 bits and their sums and `end` offsets alike, 8.4 and
    /// 8.5 read a magnitude from 2^64 − 2^32 + 1 to 2^64 − 1 as a 64-bit `long`
    /// wraps it, which a 32-bit one cannot hold, and 9.0 reads a wide, a bignum
    /// alone as the nearest wide and a sum that reaches the widest, or a bignum
    /// offset after `end`, as `end+1`.
    #[test]
    fn each_release_reads_an_index_as_its_tclsh_does() {
        use tcl_syntax::number::NumberSyntax::{Tcl84, Tcl85, Tcl90};
        // (spec, 8.4, 8.5 and 8.6, 9.0 and 9.1)
        type Row = (&'static str, Option<i64>, Option<i64>, Option<i64>);
        let table: &[Row] = &[
            ("1", Some(1), Some(1), Some(1)),
            (" 1", Some(1), Some(1), Some(1)),
            ("1 ", Some(1), Some(1), Some(1)),
            ("+1", Some(1), Some(1), Some(1)),
            (" +1", Some(1), Some(1), Some(1)),
            ("- 1", None, None, None),
            ("-1", Some(-1), Some(-1), Some(-1)),
            ("0x2", Some(2), Some(2), Some(2)),
            ("010", Some(8), Some(8), Some(10)),
            ("0o10", None, Some(8), Some(8)),
            ("0b10", None, Some(2), Some(2)),
            ("1_0", None, None, Some(10)),
            ("1e0", None, None, None),
            ("end", Some(11), Some(11), Some(11)),
            ("e", Some(11), Some(11), None),
            ("en", Some(11), Some(11), None),
            ("endx", None, None, None),
            (" end", None, None, None),
            ("end ", None, None, None),
            ("end-1", Some(10), Some(10), Some(10)),
            ("end+1", None, Some(12), Some(12)),
            ("end--1", Some(12), Some(12), Some(12)),
            ("end-+1", Some(10), Some(10), Some(10)),
            ("end- 1", Some(10), None, None),
            ("end+ 1", None, None, None),
            ("end-1 ", Some(10), Some(10), Some(10)),
            ("end-0x1", Some(10), Some(10), Some(10)),
            ("end-010", Some(3), Some(3), Some(1)),
            ("end-", None, None, None),
            ("end+", None, None, None),
            ("end-1-1", None, None, None),
            ("end -1", None, None, None),
            ("1+1", None, Some(2), Some(2)),
            ("1-1", None, Some(0), Some(0)),
            ("1+-1", None, Some(0), Some(0)),
            ("1--1", None, Some(2), Some(2)),
            ("1++1", None, Some(2), Some(2)),
            ("+1+1", None, Some(2), Some(2)),
            ("1+ 1", None, None, None),
            ("1 +1", None, None, None),
            (" 1+1", None, Some(2), Some(2)),
            ("1+1 ", None, Some(2), Some(2)),
            ("0x1+1", None, Some(2), Some(2)),
            ("010+0", None, Some(8), Some(10)),
            ("-1+2", None, Some(1), Some(1)),
            ("1+end", None, None, None),
            ("end-1+1", None, None, None),
            ("1 0", None, None, None),
            ("{1}", None, None, None),
            ("", None, None, None),
            (" ", None, None, None),
        ];
        for &(spec, on_84, on_85, on_90) in table {
            assert_eq!(resolve_opt_with(spec, 12, Tcl84), on_84, "{spec:?} on 8.4");
            assert_eq!(resolve_opt_with(spec, 12, Tcl85), on_85, "{spec:?} on 8.5");
            assert_eq!(resolve_opt_with(spec, 12, Tcl90), on_90, "{spec:?} on 9.0");
        }

        for &(spec, readings) in PAST_32_BITS {
            for (release, want) in TclVersion::ALL.into_iter().zip([
                readings[0],
                readings[1],
                readings[2],
                readings[3],
                readings[3],
            ]) {
                assert_eq!(
                    read_under(spec, 12, release),
                    want,
                    "{spec:?} on {release:?}"
                );
            }
        }
        // The grammar 8.5 and 8.6 share names neither, so it cannot say which
        // reading a 64-bit `long` gives one of them.
        assert_eq!(
            read_with("18446744073709551615", 12, Tcl85),
            IndexReading::Unsure
        );
        assert_eq!(
            read_with("0xffffffffffffffff+1", 12, Tcl85),
            IndexReading::Unsure
        );
        assert_eq!(resolve_opt_with("18446744073709551615", 12, Tcl84), None);
    }

    /// 8.6 encodes a literal `end` offset that is positive after the 32-bit
    /// wrap as after the end, where the word read at run time sums with `end`
    /// in an `int`; the two part where that sum passes `INT_MAX`.
    /// Measured on a 12-element string and list, a literal against the same
    /// word in a variable (`programs/s1/litvar.tcl`): only 8.6 parts, and
    /// only on these rows.
    #[test]
    fn a_literal_end_offset_past_int_compiles_apart_in_8_6() {
        for (spec, apart) in [
            ("end+2147483647", true),
            ("end+2147483637", true),
            ("end+2147483636", false),
            ("end-2147483649", true),
            ("end--2147483647", true),
            ("end-2147483648", false),
            ("end-2147483647", false),
            ("end+4294967296", false),
            ("end+1", false),
            ("2147483647", false),
            ("1+2147483647", false),
        ] {
            assert_eq!(compiles_apart(spec, 12, TclVersion::V8_6), apart, "{spec}");
            for release in TclVersion::ALL {
                if release != TclVersion::V8_6 {
                    assert!(!compiles_apart(spec, 12, release), "{spec} on {release:?}");
                }
            }
        }
    }

    /// A literal index word fixed as each release reads it whatever the length,
    /// compiled in a body and read by its command at run time. Measured on
    /// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 with `string index`,
    /// `string range` from each side, `lindex` and `lrange` over a 12-element
    /// string and list, each word a literal inside a proc against the same
    /// word in a variable (`end+2147483647` in a proc ranges the whole string
    /// on 8.6 and nothing through a variable; 8.5 ranges nothing either way,
    /// 9.0 and 9.1 the whole string either way). `None` is a reading that turns
    /// on the length or the host, or `bad index`.
    #[test]
    fn a_literal_index_is_fixed_as_each_release_compiles_or_reads_it() {
        use Encoded::{After, Before, FromEnd, Index};
        use TclVersion::{V8_4, V8_5, V8_6, V9_0, V9_1};
        // (spec, release, compiled, read at run time)
        type Row = (&'static str, TclVersion, Option<Encoded>, Option<Encoded>);
        let table: &[Row] = &[
            ("end+2147483647", V8_6, Some(After), None),
            ("end+2147483646", V8_6, Some(After), None),
            ("end+2147483637", V8_6, Some(After), None),
            ("end+2147483636", V8_6, Some(After), None),
            ("end-2147483649", V8_6, Some(After), None),
            ("end--2147483647", V8_6, Some(After), None),
            ("end+1", V8_6, Some(After), None),
            (
                "end-2147483647",
                V8_6,
                Some(Before),
                Some(FromEnd(-2_147_483_647)),
            ),
            ("end-2147483648", V8_6, Some(Before), None),
            ("end+2147483648", V8_6, Some(Before), None),
            ("end-1", V8_6, Some(FromEnd(-1)), Some(FromEnd(-1))),
            ("end", V8_6, Some(FromEnd(0)), Some(FromEnd(0))),
            ("e", V8_6, Some(FromEnd(0)), Some(FromEnd(0))),
            ("end+4294967295", V8_6, Some(FromEnd(-1)), Some(FromEnd(-1))),
            ("end+4294967296", V8_6, None, None),
            (
                "2147483647",
                V8_6,
                Some(Index(2_147_483_647)),
                Some(Index(2_147_483_647)),
            ),
            ("2147483648", V8_6, Some(Before), Some(Before)),
            (
                "-2147483649",
                V8_6,
                Some(Index(2_147_483_647)),
                Some(Index(2_147_483_647)),
            ),
            ("1+2147483647", V8_6, Some(Before), Some(Before)),
            ("010", V8_6, Some(Index(8)), Some(Index(8))),
            ("-1", V8_6, Some(Before), Some(Before)),
            ("end+2147483647", V8_5, None, None),
            ("end-2147483649", V8_5, None, None),
            (
                "end-2147483647",
                V8_5,
                Some(FromEnd(-2_147_483_647)),
                Some(FromEnd(-2_147_483_647)),
            ),
            ("end-1", V8_5, Some(FromEnd(-1)), Some(FromEnd(-1))),
            ("2147483648", V8_5, Some(Before), Some(Before)),
            ("end-2147483649", V8_4, None, None),
            ("end+1", V8_4, None, None),
            ("end-1", V8_4, Some(FromEnd(-1)), Some(FromEnd(-1))),
            (
                "end+2147483647",
                V9_0,
                Some(FromEnd(2_147_483_647)),
                Some(FromEnd(2_147_483_647)),
            ),
            (
                "end-2147483649",
                V9_0,
                Some(FromEnd(-2_147_483_649)),
                Some(FromEnd(-2_147_483_649)),
            ),
            (
                "2147483648",
                V9_0,
                Some(Index(2_147_483_648)),
                Some(Index(2_147_483_648)),
            ),
            ("end+1", V9_1, Some(FromEnd(1)), Some(FromEnd(1))),
            ("010", V9_1, Some(Index(10)), Some(Index(10))),
        ];
        for &(spec, release, compiled, run_time) in table {
            assert_eq!(
                encode(spec, release, true),
                compiled,
                "{spec:?} compiled on {release:?}"
            );
            assert_eq!(
                encode(spec, release, false),
                run_time,
                "{spec:?} read on {release:?}"
            );
        }
    }

    /// The version-less entry takes the grammar this runtime was built for. The
    /// ambient starts at 9.0, so an unpinned process reads `010` as decimal —
    /// and pinning it to 8.5 flips the same spec to octal.
    #[test]
    fn the_versionless_entry_follows_the_installed_grammar() {
        use tcl_syntax::number::{NumberSyntax, runtime_syntax, set_runtime_syntax};
        let restore = runtime_syntax();
        set_runtime_syntax(NumberSyntax::Tcl90);
        assert_eq!(resolve_opt("010", 12), Some(10));
        set_runtime_syntax(NumberSyntax::Tcl85);
        assert_eq!(resolve_opt("010", 12), Some(8));
        set_runtime_syntax(restore);
    }

    /// The grammar-taking entry ignores the installed grammar. Measured on a
    /// 12-character subject with `string index` (LANG=C.UTF-8): `010` is 8
    /// and `end-010` is 3 on tclsh 8.4.20, 8.5.19 and 8.6.18, 10 and 1 on
    /// 9.0.4 and 9.1.0; `1_0` is `bad index` up to 8.6 and 10 from 9.0.
    #[test]
    fn the_grammar_taking_entry_ignores_the_installed_grammar() {
        use tcl_syntax::number::{NumberSyntax, runtime_syntax, set_runtime_syntax};
        let restore = runtime_syntax();
        for ambient in [NumberSyntax::Tcl85, NumberSyntax::Tcl90] {
            set_runtime_syntax(ambient);
            for numbers in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
                assert_eq!(resolve_with("010", 12, numbers).ok(), Some(8));
                assert_eq!(resolve_with("end-010", 12, numbers).ok(), Some(3));
                assert!(resolve_with("1_0", 12, numbers).is_err());
            }
            assert_eq!(resolve_with("010", 12, NumberSyntax::Tcl90).ok(), Some(10));
            assert_eq!(
                resolve_with("end-010", 12, NumberSyntax::Tcl90).ok(),
                Some(1)
            );
            assert_eq!(resolve_with("1_0", 12, NumberSyntax::Tcl90).ok(), Some(10));
        }
        set_runtime_syntax(restore);
    }
}
