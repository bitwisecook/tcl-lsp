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
//! `0.0 0.1 0.2 0.3 0.4 0.5`. Both runtimes materialise a concrete list (the C
//! lazy abstract-list object is representation-only and incompatible-by-design).
//!
//! The split that lets this be shared: [`decode`] runs the argument state
//! machine and the reads C makes of each value, and [`generate`] then builds the
//! element list over `ValueOps`. Every word is a number or a keyword, never an
//! expression: tclsh 9.0.4 and 9.1.0 refuse `lseq 1+2` as `expected number`,
//! whatever the manpage's `{[llength $l]-1}` example suggests.
//!
//! `lseq` is `i64`-based even on the bignum runtime (C's `assignNumber` refuses
//! `TCL_NUMBER_BIG` as the overflow), so [`Num`] carries a fixed `i64`/`f64` pair — sound for both
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

use tcl_syntax::number::{self, Number};
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;
use crate::prefix::OptionTable;

/// We materialise a concrete list (the C lazy *abstract* series is
/// representation-only, incompatible-by-design — `lseq 10 2147483647` builds a
/// 2-billion-element series lazily in C). Beyond this cap we raise C's
/// `TclNewArithSeriesObj` "max length" error rather than OOM-aborting; no
/// legitimate behavioural test asks for a list anywhere near this size.
pub const MAX_MATERIALIZE: i64 = 100_000_000;

/// A range/operation keyword.
#[derive(Clone, Copy, PartialEq)]
enum Op {
    Dots,  // ".."
    To,    // "to"
    Count, // "count"
    By,    // "by"
}

/// `seq_operations[]` (`tclCmdIL.c`), looked up as `Tcl_GetIndexFromObj` does
/// with flags `0`, so a unique prefix (`co`, `t`) names its keyword.
const OPERATIONS: OptionTable<'static> =
    OptionTable::abbreviating("operation", &["..", "to", "count", "by"]);

/// A decoded numeric argument: its int and double views, what kind of number
/// `Tcl_GetNumberFromObj` read, the fractional-digit precision of its source
/// text (for double sequences), and which argument it was.
#[derive(Clone, Copy)]
pub struct Num {
    is_double: bool,
    /// An integer past the wide range: a number to the decode, refused as C's
    /// overflow once the series reads it.
    big: bool,
    i: i64,
    d: f64,
    prec: u32,
    arg: usize,
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

/// Read `bytes` as `Tcl_GetNumberFromObj` reads a number — an integer of any
/// size, a double or a NaN — or `None` for anything else.
fn as_number(bytes: &[u8], arg: usize) -> Option<Num> {
    let s = core::str::from_utf8(bytes).ok()?;
    let num = |is_double, big, i, d, prec| Num {
        is_double,
        big,
        i,
        d,
        prec,
        arg,
    };
    Some(match number::parse_whole(s)? {
        Number::Int(v) => num(false, false, v, v as f64, 0),
        Number::Double(f) => num(true, false, f as i64, f, frac_digits(bytes)),
        Number::Nan { .. } => num(true, false, 0, f64::NAN, 0),
        Number::Big { .. } => num(false, true, 0, 0.0, 0),
    })
}

/// The `wrong # args` error.
fn syntax() -> CmdError {
    CmdError::wrong_args("lseq n ??op? n ??by? n??")
}

/// `Tcl_GetNumberFromObj`'s refusal of a word that is no number.
fn not_a_number(bytes: &[u8]) -> CmdError {
    let text = String::from_utf8_lossy(bytes);
    CmdError::with_error_code(
        format!(
            "expected number but got {}",
            tcl_syntax::list::describe_bad_value(&text)
        ),
        "TCL VALUE NUMBER",
    )
}

fn overflow() -> CmdError {
    CmdError::with_error_code(
        tcl_syntax::expr::errors::IOVERFLOW_MESSAGE,
        tcl_syntax::expr::errors::IOVERFLOW_CODE,
    )
}

/// An `ARITH DOMAIN` failure, its description as the code's third word.
fn domain(message: &str, description: &str) -> CmdError {
    CmdError::with_error_code(
        message,
        tcl_syntax::list::join_list(["ARITH", "DOMAIN", description]),
    )
}

fn max_length() -> CmdError {
    CmdError::with_error_code("max length of a Tcl list exceeded", "TCL MEMORY")
}

/// Decode the (already name-stripped) `lseq` arguments into a [`Plan`] —
/// `SequenceIdentifyArgument` and `Tcl_LseqObjCmd`'s argument table, then the
/// reads `TclNewArithSeriesObj` makes of each value. A word is a number or a
/// keyword, never an expression (tclsh 9.0.4/9.1.0 refuse `lseq 1+2` as
/// `expected number`, the manpage's `{[llength $l]-1}` example included).
///
/// # Errors
/// C's message and `-errorcode`: `expected number` for a word that is
/// neither, `bad operation` where only a keyword may stand, `missing "…"
/// value.` for a trailing keyword, `wrong # args` for any other shape, then
/// a count that is no integer (`TCL VALUE INTEGER`) or a value past the wide
/// range (`ARITH IOVERFLOW`).
pub fn decode(args: &[&[u8]]) -> Result<Plan, CmdError> {
    let nargs = args.len();
    if nargs > 5 {
        return Err(syntax());
    }

    // `allow_num`/`allow_kw` gate what each position may be: after a keyword
    // only a number; after a number a keyword, and a further number unless
    // this is the last of three with exactly two words left (C's
    // `remNums`/`allowedArgs`).
    let mut decoded: Vec<Arg> = Vec::with_capacity(nargs);
    let mut use_doubles = 0u32;
    let mut allow_num = true;
    let mut allow_kw = false;
    let mut rem_nums = 3i32;
    for (idx, &bytes) in args.iter().enumerate() {
        if allow_num && let Some(n) = as_number(bytes, idx) {
            if n.is_double {
                use_doubles += 1;
            }
            decoded.push(Arg::Num(n));
            rem_nums -= 1;
            allow_kw = true;
            allow_num = !(rem_nums == 1 && nargs - 1 - idx == 2);
            continue;
        }
        if allow_kw && let Ok(op) = OPERATIONS.index_of(bytes) {
            if idx + 1 == nargs {
                let word = String::from_utf8_lossy(bytes);
                return Err(CmdError::new(format!("missing \"{word}\" value.")));
            }
            decoded.push(Arg::Kw(match op {
                0 => Op::Dots,
                1 => Op::To,
                2 => Op::Count,
                _ => Op::By,
            }));
            allow_num = true;
            allow_kw = false;
            continue;
        }
        if !allow_num {
            return Err(OPERATIONS.index_of_cmd(bytes).err().unwrap_or_else(syntax));
        }
        return Err(not_a_number(bytes));
    }

    let plan = plan_from(&decoded, use_doubles)?;
    check_reads(&plan, args)?;
    Ok(plan)
}

/// `TclNewArithSeriesObj`'s reads, in its order: the count as a wide integer
/// (a fractional double is refused as the double it holds), then the step,
/// start and end, any of which past the wide range is C's overflow.
fn check_reads(plan: &Plan, args: &[&[u8]]) -> Result<(), CmdError> {
    if let Some(count) = plan.count {
        if count.big {
            return Err(overflow());
        }
        if count.is_double {
            let word = String::from_utf8_lossy(args[count.arg]);
            return Err(CmdError::with_error_code(
                format!("expected integer but got \"{word}\""),
                "TCL VALUE INTEGER",
            ));
        }
    }
    for value in [plan.step, Some(plan.start), plan.end]
        .into_iter()
        .flatten()
    {
        if value.big {
            return Err(overflow());
        }
    }
    Ok(())
}

/// Resolve the decoded arguments into a [`Plan`] via the decode key (number→1,
/// keyword→2 per position) — the pure `SequenceIdentifyArgument` dispatch. The
/// length is the C argument table written out as one match, not decomposable
/// without obscuring the 1:1 correspondence.
// `too_many_lines`: one match arm per decode-key shape — the C argument table, 1:1.
#[allow(clippy::too_many_lines)]
fn plan_from(decoded: &[Arg], mut use_doubles: u32) -> Result<Plan, CmdError> {
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

    let constant = |i: i64| Num {
        is_double: false,
        big: false,
        i,
        d: i as f64,
        prec: 0,
        arg: 0,
    };
    let (zero, one) = (constant(0), constant(1));
    let (mut start, mut end, mut step, mut count): (Num, Option<Num>, Option<Num>, Option<Num>) =
        (zero, None, None, None);

    match key {
        // lseq n
        1 => {
            count = Some(n(0));
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
                step = Some(n(2));
            }
            Op::Count => {
                start = n(0);
                count = Some(n(2));
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
                }
                Op::By => return Err(syntax()),
            }
        }
        _ => return Err(syntax()),
    }

    // A double-valued count does not, by itself, make the sequence use
    // doubles (C: "Don't consider Count type ..."); a whole one reads as the
    // integer it is, past the wide range as a bignum, and a fractional or
    // non-finite one is left to fail the count read.
    if let Some(c) = count.as_mut()
        && c.is_double
    {
        use_doubles = use_doubles.saturating_sub(1);
        if c.d.is_finite() && c.d.floor() == c.d {
            c.is_double = false;
            if c.d >= i64::MAX as f64 || c.d <= i64::MIN as f64 {
                c.big = true;
            } else {
                c.i = c.d as i64;
            }
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

/// Build the sequence's element list over `ops` from a decoded [`Plan`].
///
/// # Errors
/// C's `TclNewArithSeriesObj` failures: the series would exceed
/// [`MAX_MATERIALIZE`] (`TCL MEMORY`), a NaN bound or element (`ARITH
/// DOMAIN`), or an integer series whose last element overflows.
pub fn generate<O: ValueOps>(ops: &mut O, plan: &Plan) -> Result<O::Value, CmdError> {
    let elems = if plan.use_doubles {
        build_double(ops, plan)?
    } else {
        build_int(ops, plan)?
    };
    Ok(ops.new_list(elems))
}

/// Charge a series of `len` elements against the value-size limit before it is
/// built: each element renders to at least one byte, with a separator between.
fn charge_series<O: ValueOps>(ops: &mut O, len: i128) -> Result<(), CmdError> {
    let bytes = u64::try_from(len.saturating_mul(2).saturating_sub(1)).unwrap_or(0);
    crate::value_size::charge(ops, |_, _| bytes)
        .map_err(|_| CmdError::new(crate::value_size::VALUE_SIZE_EXCEEDED))
}

/// The bytes a series renders to so far, charged against the value-size
/// limit as it is built: [`charge_series`] can only count a byte per element
/// before the elements exist, so a series of long numerals is stopped here
/// once it crosses the limit, not after it is whole.
struct SeriesMeter {
    limit: Option<u64>,
    bytes: u64,
}

impl SeriesMeter {
    fn new<O: ValueOps>(ops: &O) -> Self {
        Self {
            limit: ops.value_size_limit(),
            bytes: 0,
        }
    }

    fn add<O: ValueOps>(&mut self, ops: &mut O, element: &O::Value) -> Result<(), CmdError> {
        let Some(limit) = self.limit else {
            return Ok(());
        };
        let separator = u64::from(self.bytes > 0);
        self.bytes = self
            .bytes
            .saturating_add(separator)
            .saturating_add(ops.measure_value(element, limit));
        if self.bytes > limit {
            let bytes = self.bytes;
            return crate::value_size::charge(ops, |_, _| bytes)
                .map_err(|_| CmdError::new(crate::value_size::VALUE_SIZE_EXCEEDED));
        }
        Ok(())
    }
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

/// `ArithSeriesLenInt`: no elements when the step runs against the range, a
/// single one for a zero step.
fn arith_series_len_int(start: i64, end: i64, step: i64) -> i128 {
    let distance = i128::from(end) - i128::from(start);
    if (step < 0 && distance > 0) || (step > 0 && distance < 0) {
        return 0;
    }
    if step == 0 {
        return 1;
    }
    (distance / i128::from(step) + 1).max(0)
}

/// Integer arithmetic series → element values (`Tcl_WideInt` path).
fn build_int<O: ValueOps>(ops: &mut O, plan: &Plan) -> Result<Vec<O::Value>, CmdError> {
    let s = plan.start.i;
    // Length is computed in i128 so an extreme `end - start` (e.g.
    // `lseq 10 9223372036854775000`) cannot overflow i64 before the cap check.
    let (len, st): (i128, i64) = if let Some(c) = plan.count {
        (i128::from(c.i), plan.step.map_or(1, |x| x.i))
    } else {
        let e = plan
            .end
            .expect("int series without end or count has a count")
            .i;
        // step defaults to ±1 by direction when omitted.
        let st = plan.step.map_or(if s <= e { 1 } else { -1 }, |x| x.i);
        (arith_series_len_int(s, e, st), st)
    };
    if len <= 0 {
        return Ok(Vec::new());
    }
    // `NewArithSeriesInt`: the last element must stay in the wide range.
    let last = i128::from(s) + (len - 1) * i128::from(st);
    if i64::try_from(last).is_err() {
        let description = "invalid arithmetic series parameter values";
        return Err(domain(description, description));
    }
    if len > i128::from(MAX_MATERIALIZE) {
        return Err(max_length());
    }
    charge_series(ops, len)?;
    let len = len as i64;
    let mut meter = SeriesMeter::new(ops);
    let mut out = Vec::with_capacity(len as usize);
    for i in 0..len {
        // s + i*st, computed in i128 (each value is in range, so the cast is
        // lossless) — avoids accumulation overflow at the i64 boundary.
        let v = (i128::from(s) + i128::from(i) * i128::from(st)) as i64;
        let element = ops.new_int(v);
        meter.add(ops, &element)?;
        out.push(element);
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
        (c.i, plan.step.map_or(1.0, |x| x.d))
    } else {
        let de = plan.end.expect("double series without end or count").d;
        let dstep = plan.step.map_or(if ds > de { -1.0 } else { 1.0 }, |x| x.d);
        if ds.is_infinite() || de.is_infinite() {
            return Err(max_length());
        }
        if ds.is_nan() || de.is_nan() {
            let description = "non-numeric floating-point value";
            return Err(domain(
                &format!("cannot use {description} \"NaN\" to estimate length of arith-series"),
                description,
            ));
        }
        (arith_series_len_dbl(ds, de, dstep, prec), dstep)
    };
    if len > MAX_MATERIALIZE {
        return Err(max_length());
    }
    if len <= 0 {
        return Ok(Vec::new());
    }
    // No NaN anywhere in the series: C checks its last element, as
    // `expr {Inf - Inf}` would fail.
    if (ds + (len - 1) as f64 * dstep).is_nan() {
        let message = "domain error: argument not in valid range";
        return Err(domain(message, message));
    }
    charge_series(ops, i128::from(len))?;
    let mut meter = SeriesMeter::new(ops);
    let mut out = Vec::with_capacity(len as usize);
    for i in 0..len {
        let d = arith_round(ds + (i as f64) * dstep, prec);
        let element = ops.new_double(d);
        meter.add(ops, &element)?;
        out.push(element);
    }
    Ok(out)
}

/// `ArithSeriesLenDbl` — element count of a double series, computed in scaled
/// wide arithmetic for stability (mirrors the C function); a zero step is a
/// single element.
fn arith_series_len_dbl(start: f64, end: f64, step: f64, precision: u32) -> i64 {
    if step == 0.0 {
        return 1;
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
    #[allow(clippy::approx_constant)] // `3.14` is parsed test data, not π
    fn as_number_parses_ints_doubles_and_precision() {
        // `lseq` number parsing: ints stay ints, doubles record their
        // fractional precision, hex is an int, a bignum is a number marked
        // big, and non-numbers are rejected.
        let i = as_number(b"42", 0).unwrap();
        assert!(!i.is_double);
        assert_eq!(i.i, 42);

        let d = as_number(b"3.14", 0).unwrap();
        assert!(d.is_double);
        assert_eq!(d.prec, 2);
        assert!((d.d - 3.14).abs() < 1e-9);

        // Trailing zeros count toward the recorded precision.
        assert_eq!(as_number(b"1.500", 0).unwrap().prec, 3);

        // Hex integer literal.
        let h = as_number(b"0x10", 0).unwrap();
        assert!(!h.is_double);
        assert_eq!(h.i, 16);

        assert!(as_number(b"99999999999999999999", 0).unwrap().big);
        // Non-numbers are rejected.
        assert!(as_number(b"abc", 0).is_none());
        assert!(as_number(b"", 0).is_none());
    }

    #[test]
    fn frac_digits_counts_fractional_part() {
        assert_eq!(frac_digits(b"1.25"), 2);
        assert_eq!(frac_digits(b"5"), 0);
        assert_eq!(frac_digits(b"1.500"), 3);
    }
}
