//! `lseq` (Tcl 8.7/9.0) — the arithmetic-sequence generator, shared over
//! [`ValueOps`](tcl_syntax::value::ValueOps).
//!
//! ```text
//! lseq start ?(..|to)? end ??by? step?
//! lseq start count count ??by? step?
//! lseq count ?by step?
//! ```
//!
//! Ported from C's `Tcl_LseqObjCmd` (`tclCmdIL.c`) + `TclNewArithSeriesObj`
//! (`tclArithSeries.c`): the same argument-decode key, the same `..`/`to`/
//! `count`/`by` keywords, the same int-vs-double selection and length formula
//! (`ArithSeriesLenInt`/`ArithSeriesLenDbl`), and the same double-precision
//! matching (`maxObjPrecision`/`ArithRound`) so e.g. `lseq 0 0.5 by 0.1` →
//! `0.0 0.1 0.2 0.3 0.4 0.5`. Both runtimes materialise a concrete list (the C
//! lazy abstract-list object is representation-only and incompatible-by-design).
//!
//! The split that lets this be shared: [`decode`] runs the argument state machine
//! — including the **expression-valued-argument** edge (`lseq $n*2 to 10`) — over
//! an injected `eval_expr` callback (so the core never names an interp); [`generate`]
//! then builds the element list over `ValueOps`. The two are **separate calls** so
//! a runtime whose value-ops *is* its interp can run the eval callback first (its
//! interp borrowed by the closure) and the generation second (its interp borrowed
//! as the ops) without a borrow conflict.
//!
//! `lseq` is `i64`-based even on the bignum runtime (C's `assignNumber` rejects
//! `TCL_NUMBER_BIG`), so [`Num`] carries a fixed `i64`/`f64` pair — sound for both
//! the bignum runtime and the `i64`+`double` VM.
//!
//! Semantics verified against tclsh 9.0.

// `lseq` is faithful to C's mixed `Tcl_WideInt`/`double` arithmetic and scaled
// length formula: i64↔f64 conversions and length-to-index casts are pervasive
// and intentional (each value is range-checked before the narrowing cast, and the
// `MAX_MATERIALIZE` cap bounds every length used as a capacity/index).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    // The float comparisons here are exact-value ports of C predicates
    // (`floor(d) != d` to detect a non-integer count; `step == 0.0`), not
    // approximate equalities.
    clippy::float_cmp
)]

use tcl_syntax::number::{self, Number};
use tcl_syntax::value::ValueOps;

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

/// A decoded numeric argument: its int and double views, whether it is a double,
/// and the fractional-digit precision of its source text (for double sequences).
///
/// Opaque to the adapters — the host's `eval_expr` callback obtains one via
/// [`as_number`] and hands it back; nothing outside this module inspects it.
#[derive(Clone, Copy)]
pub struct Num {
    is_double: bool,
    i: i64,
    d: f64,
    prec: u32,
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

/// An `lseq` failure during [`decode`].
pub enum LseqError<E> {
    /// A complete, ready-to-report error message (syntax / missing-value /
    /// non-integer count) — the adapter sets it as the result.
    Message(Vec<u8>),
    /// The host's `eval_expr` callback failed (an expression-valued argument did
    /// not evaluate). The host error is carried through unchanged — on a runtime
    /// that already set its interp result, the adapter just returns it.
    Eval(E),
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
/// or `None` if it is not a number (a keyword or an expression). Public so an
/// adapter's `eval_expr` callback can classify the evaluated result the same way.
#[must_use]
pub fn as_number(bytes: &[u8]) -> Option<Num> {
    let s = core::str::from_utf8(bytes).ok()?;
    match number::parse_whole(s)? {
        Number::Int(v) => Some(Num {
            is_double: false,
            i: v,
            d: v as f64,
            prec: 0,
        }),
        Number::Double(f) => Some(Num {
            is_double: true,
            i: f as i64,
            d: f,
            prec: frac_digits(bytes),
        }),
        Number::Nan { .. } => Some(Num {
            is_double: true,
            i: 0,
            d: f64::NAN,
            prec: 0,
        }),
        // A bignum literal: C's `assignNumber` rejects `TCL_NUMBER_BIG`.
        Number::Big { .. } => None,
    }
}

/// Match a range keyword.
fn as_keyword(bytes: &[u8]) -> Option<Op> {
    match bytes {
        b".." => Some(Op::Dots),
        b"to" => Some(Op::To),
        b"count" => Some(Op::Count),
        b"by" => Some(Op::By),
        _ => None,
    }
}

/// The `wrong # args` error.
fn syntax<E>() -> LseqError<E> {
    LseqError::Message(b"wrong # args: should be \"lseq n ??op? n ??by? n??\"".to_vec())
}

/// Decode the (already name-stripped) `lseq` arguments into a [`Plan`].
///
/// `eval_expr(src)` evaluates an expression-valued argument and classifies its
/// result: `Ok(Some(num))` = a number, `Ok(None)` = evaluated but not a number
/// (→ a syntax error here), `Err(e)` = the evaluation itself failed (propagated
/// as [`LseqError::Eval`]). This is the only edge that needs the interp, so it is
/// the only thing injected.
pub fn decode<E, F>(args: &[&[u8]], mut eval_expr: F) -> Result<Plan, LseqError<E>>
where
    F: FnMut(&[u8]) -> Result<Option<Num>, E>,
{
    let nargs = args.len();
    if nargs == 0 || nargs > 5 {
        return Err(syntax());
    }

    // -- decode each argument (the `SequenceIdentifyArgument` state machine) ---
    // `allowed_num`/`allowed_kw` gate what each position may be; after a keyword
    // only a number is allowed; a number after the first restricts when the next
    // may be a keyword (mirrors C's `remNums`/`allowedArgs`).
    let mut decoded: Vec<Arg> = Vec::with_capacity(nargs);
    let mut use_doubles = 0u32;
    let mut allowed_num = true;
    let mut allowed_kw = false;
    let mut rem_nums = 3i32;
    for (idx, &bytes) in args.iter().enumerate() {
        let is_last = idx == nargs - 1;
        // 1) a plain number (when numbers are allowed here);
        let num = if allowed_num { as_number(bytes) } else { None };
        if let Some(n) = num {
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
                return Err(LseqError::Message(m));
            }
            decoded.push(Arg::Kw(op));
            allowed_num = true;
            allowed_kw = false;
            continue;
        }
        // 3) otherwise, if a number is allowed, evaluate as an expression.
        if allowed_num {
            match eval_expr(bytes).map_err(LseqError::Eval)? {
                Some(n) => {
                    if n.is_double {
                        use_doubles += 1;
                    }
                    decoded.push(Arg::Num(n));
                    rem_nums -= 1;
                    allowed_kw = true;
                    allowed_num = !(rem_nums == 1 && (nargs - 1 - idx) == 2);
                }
                None => return Err(syntax()),
            }
            continue;
        }
        return Err(syntax());
    }

    plan_from(&decoded, use_doubles)
}

/// Resolve the decoded arguments into a [`Plan`] via the decode key (number→1,
/// keyword→2 per position) — the pure `SequenceIdentifyArgument` dispatch. The
/// length is the C argument table written out as one match, not decomposable
/// without obscuring the 1:1 correspondence.
#[allow(clippy::too_many_lines)]
fn plan_from<E>(decoded: &[Arg], mut use_doubles: u32) -> Result<Plan, LseqError<E>> {
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
    };
    let one = Num {
        is_double: false,
        i: 1,
        d: 1.0,
        prec: 0,
    };
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

    // A double-valued count is converted to an integer and does not, by itself,
    // make the sequence use doubles (C: "Don't consider Count type ...").
    if let Some(c) = count
        && c.is_double
    {
        use_doubles = use_doubles.saturating_sub(1);
        if !c.d.is_finite() || c.d.floor() != c.d {
            return Err(LseqError::Message(
                b"expected integer but got non-integer count".to_vec(),
            ));
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
/// Returns the C `TclNewArithSeriesObj` message bytes if the series would exceed
/// [`MAX_MATERIALIZE`] or estimating its length hits a non-numeric float.
pub fn generate<O: ValueOps>(ops: &mut O, plan: &Plan) -> Result<O::Value, &'static [u8]> {
    let elems = if plan.use_doubles {
        build_double(ops, plan)?
    } else {
        build_int(ops, plan)?
    };
    Ok(ops.new_list(elems))
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
fn build_int<O: ValueOps>(ops: &mut O, plan: &Plan) -> Result<Vec<O::Value>, &'static [u8]> {
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
        if st == 0 {
            return Ok(Vec::new());
        }
        let len = (i128::from(e) - i128::from(s)) / i128::from(st) + 1;
        (len.max(0), st)
    };
    if st == 0 {
        return Ok(Vec::new());
    }
    if len > i128::from(MAX_MATERIALIZE) {
        return Err(b"max length of a Tcl list exceeded");
    }
    let len = len as i64;
    let mut out = Vec::with_capacity(len as usize);
    for i in 0..len {
        // s + i*st, computed in i128 (each value is in range, so the cast is
        // lossless) — avoids accumulation overflow at the i64 boundary.
        let v = (i128::from(s) + i128::from(i) * i128::from(st)) as i64;
        out.push(ops.new_int(v));
    }
    Ok(out)
}

/// Double arithmetic series → element values, with C's precision matching.
fn build_double<O: ValueOps>(ops: &mut O, plan: &Plan) -> Result<Vec<O::Value>, &'static [u8]> {
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
        if dstep == 0.0 {
            return Ok(Vec::new());
        }
        if !ds.is_finite() || !de.is_finite() {
            if ds.is_nan() || de.is_nan() {
                return Err(
                    b"cannot use non-numeric floating-point value to estimate length of arith-series",
                );
            }
            return Err(b"max length of a Tcl list exceeded");
        }
        (arith_series_len_dbl(ds, de, dstep, prec), dstep)
    };
    if dstep == 0.0 {
        return Ok(Vec::new());
    }
    if len > MAX_MATERIALIZE {
        return Err(b"max length of a Tcl list exceeded");
    }
    let mut out = Vec::with_capacity(len as usize);
    for i in 0..len {
        let d = arith_round(ds + (i as f64) * dstep, prec);
        out.push(ops.new_double(d));
    }
    Ok(out)
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
