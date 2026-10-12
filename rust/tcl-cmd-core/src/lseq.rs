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

//! `lseq` (Tcl 8.7/9.0) — the arithmetic-sequence generator, shared over
//! [`ValueOps`](tcl_syntax::value::ValueOps).
//!
//! ```text
//! lseq start ?(..|to)? end ??by? step?
//! lseq start count count ??by? step?
//! lseq count ?by step?
//! ```
//!
//! Follows C's `Tcl_LseqObjCmd` (`tclCmdIL.c`) and `TclNewArithSeriesObj`
//! (`tclArithSeries.c`): the same argument-decode key, the same `..`/`to`/
//! `count`/`by` keywords, the same int-vs-double selection and length formula
//! (`ArithSeriesLenInt`/`ArithSeriesLenDbl`), and the same double-precision
//! matching (`maxObjPrecision`/`ArithRound`) so e.g. `lseq 0 0.5 by 0.1` →
//! `0.0 0.1 0.2 0.3 0.4 0.5`. These backends materialise a concrete list,
//! whereas [`prepare_series`] validates a constant-space native arithmetic
//! payload. Capacity refusals apply only when an element table or string is
//! actually materialized, outside the guest completion channel.
//!
//! [`decode`] accepts native numeric arguments and unique keyword prefixes;
//! it never evaluates argument bytes as Tcl expressions. [`generate`] then
//! builds the ordinary result list over the selected runtime's `ValueOps`.
//!
//! `lseq` is `i64`-based even on the bignum runtime (C's `assignNumber` rejects
//! `TCL_NUMBER_BIG`), so [`Num`] carries a fixed `i64`/`f64` pair — sound for both
//! the bignum runtime and the `i64`+`double` VM.
//!
//! Semantics follow tclsh 9.0.

// `lseq` is faithful to C's mixed `Tcl_WideInt`/`double` arithmetic and scaled
// length formula: i64↔f64 conversions and length-to-index casts are pervasive
// and intentional (each value is range-checked before the narrowing cast, and the
// `MAX_MATERIALIZE` cap bounds every length used as a capacity/index).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    // The float comparisons here are exact-value predicates matching C
    // (`floor(d) != d` to detect a non-integer count; `step == 0.0`), not
    // approximate equalities.
    clippy::float_cmp
)]

use crate::CmdError;
use tcl_syntax::number::{self, Number};
use tcl_syntax::raw_string::{NativeMaterializationLimitError, NativeValueAccessRefusal};
use tcl_syntax::value::ValueOps;

/// Capacity of this eager backend, independent of native Tcl's lazy list size.
/// Exceeding it produces a host-only materialization refusal, never Tcl's
/// catchable list-length error.
pub const MAX_MATERIALIZE: i64 = 100_000_000;

/// A range/operation keyword.
#[derive(Clone, Copy, PartialEq)]
enum Op {
    Dots,  // ".."
    To,    // "to"
    Count, // "count"
    By,    // "by"
}

/// A decoded numeric argument: its int and double views, whether it is a double,
/// and the fractional-digit precision of its source text (for double sequences).
///
/// Opaque to the adapters; only the shared decoder inspects its numeric parts.
#[derive(Clone, Copy)]
pub struct Num {
    is_double: bool,
    i: i64,
    d: f64,
    prec: u32,
    position: Option<usize>,
}

/// One decoded argument.
enum Arg {
    Num(Num),
    Kw(Op),
}

/// The resolved sequence parameters produced by [`decode`] and consumed by
/// [`generate`]. Opaque to the adapters (they only pass it between the two calls).
pub struct Plan {
    start: Num,
    end: Option<Num>,
    step: Option<Num>,
    count: Option<Num>,
    use_doubles: bool,
}

impl Plan {
    /// Reach only selected original Double precision enquiries, in the native
    /// step/start/end order. Count objects are excluded and stay unmaterialized.
    pub fn prepare_double_precision(
        &mut self,
        mut original: impl FnMut(usize) -> Result<Vec<u8>, CmdError>,
    ) -> Result<(), CmdError> {
        if !self.use_doubles {
            return Ok(());
        }
        for argument in [self.step.as_mut(), Some(&mut self.start), self.end.as_mut()]
            .into_iter()
            .flatten()
        {
            if argument.is_double
                && let Some(index) = argument.position
            {
                argument.prec = frac_digits(&original(index)?);
            }
        }
        Ok(())
    }
}

/// An `lseq` argument-decode failure, without evaluating argument scripts.
pub enum LseqError {
    /// Native usage error, rendered using the actual invocation header.
    WrongArguments,
    /// Ready-to-publish native guest error with byte-exact payload and code.
    Command(CmdError),
}

fn argument_error(expected: &[u8], bytes: &[u8], code: &[u8]) -> LseqError {
    let mut message = b"expected ".to_vec();
    message.extend_from_slice(expected);
    message.extend_from_slice(b" but got \"");
    message.extend_from_slice(bytes);
    message.push(b'"');
    LseqError::Command(CmdError::with_byte_error_details(
        message,
        code.to_vec(),
        None,
        None,
    ))
}

fn number_error(bytes: &[u8]) -> LseqError {
    if core::str::from_utf8(bytes)
        .ok()
        .and_then(number::parse_whole)
        .is_some_and(|number| matches!(number, Number::Big { .. }))
    {
        overflow_error()
    } else {
        argument_error(b"number", bytes, b"TCL VALUE NUMBER")
    }
}

fn overflow_error() -> LseqError {
    LseqError::Command(CmdError::with_error_code(
        "integer value too large to represent",
        "ARITH IOVERFLOW {integer value too large to represent}",
    ))
}

/// Fractional-digit count of a number's source text (`ObjPrecision`): the digits
/// after `.`, or 0 for an integer or `e`-notation.
fn frac_digits(s: &[u8]) -> u32 {
    if s.iter().any(|&c| c == b'e' || c == b'E') {
        return 0;
    }
    match s.iter().position(|&c| c == b'.') {
        Some(dot) => (s.len() - dot - 1) as u32,
        None => 0,
    }
}

/// Classify `bytes` as a plain numeric literal (the `Tcl_GetNumberFromObj` step),
/// or `None` for a keyword, invalid numeric bytes, or a non-wide integer.
#[must_use]
pub fn as_number(bytes: &[u8]) -> Option<Num> {
    let s = core::str::from_utf8(bytes).ok()?;
    match number::parse_whole(s)? {
        Number::Int(v) => Some(Num {
            is_double: false,
            i: v,
            d: v as f64,
            prec: 0,
            position: None,
        }),
        Number::Double(f) => Some(Num {
            is_double: true,
            i: f as i64,
            d: f,
            prec: frac_digits(bytes),
            position: None,
        }),
        Number::Nan { .. } => Some(Num {
            is_double: true,
            i: 0,
            d: f64::NAN,
            prec: 0,
            position: None,
        }),
        // A bignum literal: C's `assignNumber` rejects `TCL_NUMBER_BIG`.
        Number::Big { .. } => None,
    }
}

/// Construct the decoder's numeric view from an actual reached Number getter.
/// Original spelling is used only for the native Double precision enquiry.
#[must_use]
pub fn number_argument(number: &Number, original: &[u8]) -> Option<Num> {
    match number {
        Number::Int(value) => Some(Num {
            is_double: false,
            i: *value,
            d: *value as f64,
            prec: 0,
            position: None,
        }),
        Number::Double(value) => Some(Num {
            is_double: true,
            i: *value as i64,
            d: *value,
            prec: frac_digits(original),
            position: None,
        }),
        Number::Nan { .. } => Some(Num {
            is_double: true,
            i: 0,
            d: f64::NAN,
            prec: 0,
            position: None,
        }),
        Number::Big { .. } => None,
    }
}

/// Match a range keyword.
fn as_keyword(bytes: &[u8]) -> Option<Op> {
    if bytes.is_empty() {
        return None;
    }
    [
        (b"..".as_slice(), Op::Dots),
        (b"to".as_slice(), Op::To),
        (b"count".as_slice(), Op::Count),
        (b"by".as_slice(), Op::By),
    ]
    .into_iter()
    .find_map(|(word, operation)| word.starts_with(bytes).then_some(operation))
}

fn syntax() -> LseqError {
    LseqError::WrongArguments
}

/// Decode name-stripped native arguments without executing Tcl expression text.
///
/// # Errors
/// Returns the native numeric, keyword, count or usage validation failure.
pub fn decode(args: &[&[u8]]) -> Result<Plan, LseqError> {
    decode_original(args.len(), |index, numeric_allowed| {
        let bytes = args[index];
        Ok((
            numeric_allowed.then(|| as_number(bytes)).flatten(),
            bytes.to_vec(),
        ))
    })
}

/// Decode original arguments in native positional order. The backend supplies
/// the reached Number primitive payload and bytes only where the actual
/// keyword, diagnostic, or Double precision stage requires them.
pub fn decode_original(
    nargs: usize,
    mut original: impl FnMut(usize, bool) -> Result<(Option<Num>, Vec<u8>), LseqError>,
) -> Result<Plan, LseqError> {
    if nargs == 0 || nargs > 5 {
        return Err(syntax());
    }

    // Decode each argument (the `SequenceIdentifyArgument` state machine).
    // `allowed_num`/`allowed_kw` gate what each position may be; after a keyword
    // only a number is allowed; a number after the first restricts when the next
    // may be a keyword (mirrors C's `remNums`/`allowedArgs`).
    let mut decoded: Vec<Arg> = Vec::with_capacity(nargs);
    let mut use_doubles = 0u32;
    let mut allowed_num = true;
    let mut allowed_kw = false;
    let mut rem_nums = 3i32;
    let mut sources = Vec::with_capacity(nargs);
    for idx in 0..nargs {
        let (num, bytes) = original(idx, allowed_num)?;
        sources.push(bytes);
        let bytes = sources
            .last()
            .expect("original reached argument")
            .as_slice();
        let is_last = idx == nargs - 1;
        // 1) a plain number (when numbers are allowed here);
        let num = if allowed_num { num } else { None };
        if let Some(mut n) = num {
            n.position = Some(idx);
            if n.is_double {
                use_doubles += 1;
            }
            decoded.push(Arg::Num(n));
            rem_nums -= 1;
            // After a number: a keyword is allowed next; a further number too,
            // unless this is the last number with exactly two args remaining.
            allowed_kw = true;
            allowed_num = !(rem_nums == 1 && (nargs - 1 - idx) == 2);
            continue;
        }
        // 2) a range keyword (when allowed);
        if allowed_kw && let Some(op) = as_keyword(bytes) {
            if is_last {
                let mut m = b"missing \"".to_vec();
                m.extend_from_slice(bytes);
                m.extend_from_slice(b"\" value.");
                return Err(LseqError::Command(CmdError::new_bytes(m)));
            }
            decoded.push(Arg::Kw(op));
            allowed_num = true;
            allowed_kw = false;
            continue;
        }
        // Native C only performs number conversion here; Tcl expression
        // evaluation is not part of the lseq argument protocol.
        if allowed_num {
            return Err(number_error(bytes));
        }
        return Err(syntax());
    }

    let sources = sources.iter().map(Vec::as_slice).collect::<Vec<_>>();
    plan_from(&decoded, &sources, use_doubles, |index| {
        original(index, false).map(|(_, bytes)| bytes)
    })
}

/// Resolve the decoded arguments into a [`Plan`] via the decode key (number→1,
/// keyword→2 per position) — the pure `SequenceIdentifyArgument` dispatch. The
/// length is the C argument table written out as one match, not decomposable
/// without obscuring the 1:1 correspondence.
// `too_many_lines`: one match arm per decode-key shape — the C argument table, 1:1.
#[allow(clippy::too_many_lines)]
fn plan_from(
    decoded: &[Arg],
    _args: &[&[u8]],
    mut use_doubles: u32,
    mut diagnostic: impl FnMut(usize) -> Result<Vec<u8>, LseqError>,
) -> Result<Plan, LseqError> {
    let key: u32 = decoded.iter().fold(0, |k, a| {
        k * 10
            + match a {
                Arg::Num(_) => 1,
                Arg::Kw(_) => 2,
            }
    });
    let n = |i: usize| match &decoded[i] {
        Arg::Num(x) => *x,
        Arg::Kw(_) => unreachable!(),
    };
    let op = |i: usize| match &decoded[i] {
        Arg::Kw(o) => *o,
        Arg::Num(_) => unreachable!(),
    };

    let zero = Num {
        is_double: false,
        i: 0,
        d: 0.0,
        prec: 0,
        position: None,
    };
    let one = Num {
        is_double: false,
        i: 1,
        d: 1.0,
        prec: 0,
        position: None,
    };
    let (mut start, mut end, mut step, mut count): (Num, Option<Num>, Option<Num>, Option<Num>) =
        (zero, None, None, None);

    let mut count_at = None;
    match key {
        // lseq n
        1 => {
            count = Some(n(0));
            count_at = Some(0);
            step = Some(one);
            use_doubles = 0; // count-only is integer-valued
        }
        // lseq n n
        11 => {
            start = n(0);
            end = Some(n(1));
        }
        // lseq n n n
        111 => {
            start = n(0);
            end = Some(n(1));
            step = Some(n(2));
        }
        // lseq n (to|..|count|by) n
        121 => match op(1) {
            Op::Dots | Op::To => {
                start = n(0);
                end = Some(n(2));
            }
            Op::By => {
                count = Some(n(0));
                count_at = Some(0);
                step = Some(n(2));
            }
            Op::Count => {
                start = n(0);
                count = Some(n(2));
                count_at = Some(2);
                step = Some(one);
            }
        },
        // lseq n (to|count) n n
        1211 => match op(1) {
            Op::Dots | Op::To => {
                start = n(0);
                end = Some(n(2));
                step = Some(n(3));
            }
            Op::Count => {
                start = n(0);
                count = Some(n(2));
                count_at = Some(2);
                step = Some(n(3));
            }
            Op::By => return Err(syntax()),
        },
        // lseq n n by n
        1121 => {
            start = n(0);
            end = Some(n(1));
            match op(2) {
                Op::By => step = Some(n(3)),
                _ => return Err(syntax()),
            }
        }
        // lseq n (to|count) n by n
        12121 => {
            match op(3) {
                Op::By => step = Some(n(4)),
                _ => return Err(syntax()),
            }
            match op(1) {
                Op::Dots | Op::To => {
                    start = n(0);
                    end = Some(n(2));
                }
                Op::Count => {
                    start = n(0);
                    count = Some(n(2));
                    count_at = Some(2);
                }
                Op::By => return Err(syntax()),
            }
        }
        _ => return Err(syntax()),
    }

    // A double-valued count is converted to an integer and does not, by itself,
    // make the sequence use doubles (C: "Don't consider Count type ...").
    if let Some(c) = count
        && c.is_double
    {
        use_doubles = use_doubles.saturating_sub(1);
        if !c.d.is_finite() || c.d.floor() != c.d {
            return Err(argument_error(
                b"integer",
                &diagnostic(count_at.expect("selected count retains its original operand"))?,
                b"TCL VALUE INTEGER",
            ));
        }
        if c.d < i64::MIN as f64 || c.d >= -(i64::MIN as f64) {
            return Err(overflow_error());
        }
    }

    Ok(Plan {
        start,
        end,
        step,
        count,
        use_doubles: use_doubles > 0,
    })
}

/// The validated constant-space arithmetic-series payload. This owner does
/// not allocate an element table or grant a backend's native type identity.
#[derive(Clone, Copy, Debug)]
pub struct Series {
    length: usize,
    values: SeriesValues,
}
#[derive(Clone, Copy, Debug)]
enum SeriesValues {
    Integer {
        start: i64,
        step: i64,
    },
    Double {
        start: f64,
        step: f64,
        precision: u32,
    },
}
impl Series {
    /// Captured native Length; no element or string materialization.
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }
    /// A fresh numeric payload for Index, never a cached element handle.
    #[must_use]
    pub fn number_at(&self, index: usize) -> Option<Number> {
        if index >= self.length {
            return None;
        }
        Some(match self.values {
            SeriesValues::Integer { start, step } => {
                Number::Int((i128::from(start) + index as i128 * i128::from(step)) as i64)
            }
            SeriesValues::Double {
                start,
                step,
                precision,
            } => {
                let value = if index == 0 {
                    start
                } else {
                    start + index as f64 * step
                };
                Number::Double(arith_round(value, precision))
            }
        })
    }
}

/// Validate the native C9 arithmetic-series constructor before any header is
/// manufactured. None selects `Tcl_NewObj`, distinct from an integer series of
/// length zero, which retains its actual arithmetic primary.
pub fn prepare_series(plan: &Plan) -> Result<Option<Series>, CmdError> {
    if plan.use_doubles {
        prepare_double_series(plan)
    } else {
        prepare_integer_series(plan)
    }
}

fn series_domain_error() -> CmdError {
    CmdError::with_error_code(
        "invalid arithmetic series parameter values",
        "ARITH DOMAIN {invalid arithmetic series parameter values}",
    )
}

fn prepare_double_series(plan: &Plan) -> Result<Option<Series>, CmdError> {
    let start = plan.start.d;
    let step = plan.step.map_or_else(
        || {
            if plan.end.is_some_and(|end| start > end.d) {
                -1.0
            } else {
                1.0
            }
        },
        |step| step.d,
    );
    let precision = plan
        .start
        .prec
        .max(plan.step.map_or(0, |step| step.prec))
        .max(plan.end.map_or(0, |end| end.prec));
    let length = if let Some(count) = plan.count {
        count.i
    } else {
        let end = plan.end.expect("selected endpoint").d;
        if start.is_infinite() || end.is_infinite() {
            return Err(CmdError::with_error_code(
                "max length of a Tcl list exceeded",
                "TCL MEMORY",
            ));
        }
        if start.is_nan() || end.is_nan() {
            return Err(CmdError::with_error_code(
                "cannot use non-numeric floating-point value \"NaN\" to estimate length of arith-series",
                "ARITH DOMAIN {non-numeric floating-point value}",
            ));
        }
        if step == 0.0 {
            1
        } else {
            native_double_length(start, end, step, precision)?
        }
    };
    if (start + length.saturating_sub(1) as f64 * step).is_nan() {
        return Err(CmdError::with_error_code(
            "domain error: argument not in valid range",
            "ARITH DOMAIN {domain error: argument not in valid range}",
        ));
    }
    if length <= 0 {
        return Ok(None);
    }
    let length = usize::try_from(length).map_err(|_| series_domain_error())?;
    Ok(Some(Series {
        length,
        values: SeriesValues::Double {
            start,
            step,
            precision,
        },
    }))
}

fn prepare_integer_series(plan: &Plan) -> Result<Option<Series>, CmdError> {
    let start = plan.start.i;
    let step = plan.step.map_or_else(
        || {
            if plan.end.is_some_and(|end| start > end.i) {
                -1
            } else {
                1
            }
        },
        |step| step.i,
    );
    let length = if let Some(count) = plan.count {
        i128::from(count.i)
    } else if step == 0 {
        1
    } else {
        let end = plan.end.expect("selected endpoint").i;
        let distance = end.checked_sub(start).ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native arithmetic-series signed endpoint distance",
            ),
        )?;
        if (step < 0 && distance > 0) || (step > 0 && distance < 0) {
            0
        } else {
            (i128::from(distance) / i128::from(step) + 1).max(0)
        }
    };
    if length < 0 {
        return Ok(None);
    }
    if length > i128::from(i64::MAX) {
        return Err(series_domain_error());
    }
    if length >= 1 {
        let intervals = length - 1;
        let magnitude = i128::from(step).abs() * intervals;
        if magnitude > i128::from(u64::MAX)
            || (step == i64::MIN && (intervals > 0 || start < 0))
            || !(i128::from(i64::MIN)..=i128::from(i64::MAX))
                .contains(&(i128::from(start) + i128::from(step) * intervals))
        {
            return Err(series_domain_error());
        }
    }
    let length = usize::try_from(length).map_err(|_| series_domain_error())?;
    Ok(Some(Series {
        length,
        values: SeriesValues::Integer { start, step },
    }))
}

/// Build the sequence's element list over `ops` from a decoded [`Plan`].
///
/// # Errors
/// Native generation errors retain guest completion. Exceeding this eager
/// backend's [`MAX_MATERIALIZE`] capacity retains a host-only refusal tag.
pub fn generate<O: ValueOps>(ops: &mut O, plan: &Plan) -> Result<O::Value, CmdError> {
    let elems = if plan.use_doubles {
        build_double(ops, plan)?
    } else {
        build_int(ops, plan)?
    };
    Ok(ops.new_list(elems))
}

fn native_double_length(start: f64, end: f64, step: f64, precision: u32) -> Result<i64, CmdError> {
    if step == 0.0 {
        return Ok(1);
    }
    let scale = power10(precision);
    let (start, end, step) = if precision == 0 {
        (start, end, step)
    } else {
        (start * scale, end * scale, step * scale)
    };
    let distance = end - start;
    if (i64::MIN as f64..=i64::MAX as f64).contains(&distance)
        && (i64::MIN as f64..=i64::MAX as f64).contains(&step)
    {
        let rounded_distance = if distance < 0.0 {
            distance - 0.5
        } else {
            distance + 0.5
        };
        let rounded_step = if step < 0.0 { step - 0.5 } else { step + 0.5 };
        if rounded_distance >= -(i64::MIN as f64) || rounded_step >= -(i64::MIN as f64) {
            return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native arithmetic-series floating-to-wide frontier",
            )
            .into());
        }
        let distance = rounded_distance as i64;
        let step = rounded_step as i64;
        if step != 0 {
            let length = distance
                .checked_div(step)
                .and_then(|length| length.checked_add(1))
                .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native arithmetic-series signed length frontier",
                ))?;
            return Ok(length.max(0));
        }
    }
    let length = distance / step + 1.0;
    if length.is_nan() {
        return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native arithmetic-series NaN length frontier",
        )
        .into());
    }
    Ok(if length >= i64::MAX as f64 {
        i64::MAX
    } else if length <= 0.0 {
        1
    } else {
        length as i64
    })
}

/// `power10` for the precision scaling.
fn power10(n: u32) -> f64 {
    10f64.powi(n as i32)
}

/// `ArithRound` — round `d` to `n` fractional digits (identity for `n == 0`).
fn arith_round(d: f64, n: u32) -> f64 {
    if n == 0 {
        return d;
    }
    let s = power10(n);
    (d * s).round() / s
}

/// Integer arithmetic series → element values (`Tcl_WideInt` path).
fn build_int<O: ValueOps>(ops: &mut O, plan: &Plan) -> Result<Vec<O::Value>, CmdError> {
    let s = plan.start.i;
    // Length is computed in i128 so an extreme `end - start` (e.g.
    // `lseq 10 9223372036854775000`) cannot overflow i64 before the cap check.
    let (len, st): (i128, i64) = if let Some(c) = plan.count {
        // count given: len = count, step defaults to 1.
        let st = plan.step.map_or(1, |x| x.i);
        (i128::from(c.i).max(0), st)
    } else {
        let e = plan
            .end
            .expect("int series without end or count has a count")
            .i;
        // step defaults to ±1 by direction when omitted.
        let st = match plan.step {
            Some(x) => x.i,
            None => {
                if s <= e {
                    1
                } else {
                    -1
                }
            }
        };
        let len = if st == 0 {
            1
        } else {
            (i128::from(e) - i128::from(s)) / i128::from(st) + 1
        };
        (len.max(0), st)
    };
    let len = materialization_length(len as u128)?;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        // s + i*st, computed in i128 (each value is in range, so the cast is
        // lossless) — avoids accumulation overflow at the i64 boundary.
        let v = (i128::from(s) + (i as i128) * i128::from(st)) as i64;
        out.push(ops.new_int(v));
    }
    Ok(out)
}

/// Double arithmetic series → element values, with C's precision matching.
fn build_double<O: ValueOps>(ops: &mut O, plan: &Plan) -> Result<Vec<O::Value>, CmdError> {
    let ds = plan.start.d;
    let prec = {
        // maxObjPrecision(start, end, step) — count is excluded.
        let mut p = plan.step.map_or(0, |x| x.prec);
        p = p.max(plan.start.prec);
        if let Some(e) = plan.end {
            p = p.max(e.prec);
        }
        p
    };
    let (len, dstep) = if let Some(c) = plan.count {
        let dstep = plan.step.map_or(1.0, |x| x.d);
        (c.i.max(0), dstep)
    } else {
        let de = plan.end.expect("double series without end or count").d;
        let dstep = match plan.step {
            Some(x) => x.d,
            None => {
                if ds <= de {
                    1.0
                } else {
                    -1.0
                }
            }
        };
        if !ds.is_finite() || !de.is_finite() {
            if ds.is_nan() || de.is_nan() {
                return Err(CmdError::new_bytes(
                    b"cannot use non-numeric floating-point value to estimate length of arith-series",
                ));
            }
            return Err(CmdError::with_error_code(
                "max length of a Tcl list exceeded",
                "TCL MEMORY",
            ));
        }
        (
            if dstep == 0.0 {
                1
            } else {
                arith_series_len_dbl(ds, de, dstep, prec)
            },
            dstep,
        )
    };
    let len = materialization_length(len.max(0) as u128)?;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let d = arith_round(ds + (i as f64) * dstep, prec);
        out.push(ops.new_double(d));
    }
    Ok(out)
}

pub fn materialization_length(requested: u128) -> Result<usize, CmdError> {
    let limit = MAX_MATERIALIZE as u128;
    if requested > limit {
        return Err(
            NativeValueAccessRefusal::from(NativeMaterializationLimitError::new(
                requested,
                MAX_MATERIALIZE as u64,
            ))
            .into(),
        );
    }
    Ok(usize::try_from(requested).expect("bounded sequence length fits host usize"))
}

/// `ArithSeriesLenDbl` — element count of a double series, computed in scaled
/// wide arithmetic for stability (mirrors the C function).
fn arith_series_len_dbl(start: f64, end: f64, step: f64, precision: u32) -> i64 {
    if step == 0.0 {
        return 0;
    }
    let (mut s, mut e, mut st) = (start, end, step);
    if precision > 0 {
        let sf = power10(precision);
        s *= sf;
        e *= sf;
        st *= sf;
    }
    let dist = e - s;
    let wide_min = i64::MIN as f64;
    let wide_max = i64::MAX as f64;
    if (wide_min..=wide_max).contains(&dist) && (wide_min..=wide_max).contains(&st) {
        let iend = if dist < 0.0 { dist - 0.5 } else { dist + 0.5 } as i64;
        let istep = if st < 0.0 { st - 0.5 } else { st + 0.5 } as i64;
        if istep != 0 {
            return (iend / istep + 1).max(0);
        }
    }
    let len = dist / st + 1.0;
    if len < 0.0 { 0 } else { len as i64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materialization_capacity_is_a_host_refusal_without_guest_error_details() {
        assert_eq!(
            NativeMaterializationLimitError::new(u128::MAX, 100_000_000).requested(),
            u128::MAX
        );
        assert_eq!(materialization_length(0).unwrap(), 0);
        assert_eq!(
            materialization_length(MAX_MATERIALIZE as u128).unwrap(),
            MAX_MATERIALIZE as usize
        );
        let error = materialization_length(MAX_MATERIALIZE as u128 + 1).unwrap_err();
        assert_eq!(
            error.native_access_refusal(),
            Some(NativeValueAccessRefusal::Materialization(
                NativeMaterializationLimitError::new(100_000_001, 100_000_000)
            ))
        );
        assert_eq!(error.message_bytes(), []);
        assert!(error.error_code_bytes().is_none());
    }

    #[test]
    #[allow(clippy::approx_constant)] // `3.14` is parsed test data, not π
    fn as_number_parses_ints_doubles_and_precision() {
        // `lseq` number parsing: ints stay ints, doubles record their
        // fractional precision, hex is an int, and non-numbers / bignums are
        // rejected.
        let i = as_number(b"42").unwrap();
        assert!(!i.is_double);
        assert_eq!(i.i, 42);

        let d = as_number(b"3.14").unwrap();
        assert!(d.is_double);
        assert_eq!(d.prec, 2);
        assert!((d.d - 3.14).abs() < 1e-9);

        // Trailing zeros count toward the recorded precision.
        assert_eq!(as_number(b"1.500").unwrap().prec, 3);

        // Hex integer literal.
        let h = as_number(b"0x10").unwrap();
        assert!(!h.is_double);
        assert_eq!(h.i, 16);

        // Non-numbers are rejected.
        assert!(as_number(b"abc").is_none());
        assert!(as_number(b"").is_none());
    }

    #[test]
    fn frac_digits_counts_fractional_part() {
        assert_eq!(frac_digits(b"1.25"), 2);
        assert_eq!(frac_digits(b"5"), 0);
        assert_eq!(frac_digits(b"1.500"), 3);
    }
}
