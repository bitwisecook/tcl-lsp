//! `lseq` (Tcl 8.7/9.0) — arithmetic-sequence list generator.
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
//! `0.0 0.1 0.2 0.3 0.4 0.5`. We materialise a concrete list (the C lazy
//! abstract-list object is representation-only and incompatible-by-design).
//!
//! Needs the numeric tower (`expr` for expression-valued arguments, doubles), so
//! the whole module is gated on `have_tommath` like `if`/`while`/`for`.
//!
//! Semantics verified against tclsh 9.0.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![cfg(have_tommath)]

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};
use tcl_syntax::number::{self, Number};

/// Register `lseq`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"lseq", lseq);
}

/// We materialise a concrete list (the C lazy *abstract* series is
/// representation-only, incompatible-by-design — `lseq 10 2147483647` builds a
/// 2-billion-element series lazily in C). Beyond this cap we raise C's
/// `TclNewArithSeriesObj` "max length" error rather than OOM-aborting; no
/// legitimate behavioural test asks for a list anywhere near this size.
const MAX_MATERIALIZE: i64 = 100_000_000;

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
#[derive(Clone, Copy)]
struct Num {
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
/// or `None` if it is not a number (a keyword or an expression).
fn as_number(bytes: &[u8]) -> Option<Num> {
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

fn syntax_err(interp: &mut Interp) -> Code {
    interp.set_error(b"wrong # args: should be \"lseq n ??op? n ??by? n??\"")
}

fn lseq(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let nargs = argv.len() - 1;
    if nargs == 0 || nargs > 5 {
        return syntax_err(interp);
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
    for (idx, &a) in argv[1..].iter().enumerate() {
        let bytes = obj_bytes(a);
        let is_last = idx == nargs - 1;
        // 1) a plain number (when numbers are allowed here);
        let num = if allowed_num { as_number(&bytes) } else { None };
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
        if allowed_kw {
            if let Some(op) = as_keyword(&bytes) {
                if is_last {
                    let mut m = b"missing \"".to_vec();
                    m.extend_from_slice(&bytes);
                    m.extend_from_slice(b"\" value.");
                    return interp.set_error(&m);
                }
                decoded.push(Arg::Kw(op));
                allowed_num = true;
                allowed_kw = false;
                continue;
            }
        }
        // 3) otherwise, if a number is allowed, evaluate as an expression.
        if allowed_num {
            let obj = match crate::builtins::eval_expr_obj(interp, &bytes) {
                Ok(o) => o,
                Err(c) => return c,
            };
            let text = obj_bytes(obj);
            let parsed = as_number(&text);
            // SAFETY: `obj` is the owned (+1) expr result; we are done with it.
            unsafe { obj::decr_ref_count(obj) };
            match parsed {
                Some(n) => {
                    if n.is_double {
                        use_doubles += 1;
                    }
                    decoded.push(Arg::Num(n));
                    rem_nums -= 1;
                    allowed_kw = true;
                    allowed_num = !(rem_nums == 1 && (nargs - 1 - idx) == 2);
                }
                None => return syntax_err(interp),
            }
            continue;
        }
        return syntax_err(interp);
    }

    // -- dispatch on the decode key (number→1, keyword→2 per position) ---------
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
            _ => return syntax_err(interp),
        },
        // lseq n n by n
        1121 => {
            start = n(0);
            end = Some(n(1));
            match op(2) {
                Op::By => step = Some(n(3)),
                _ => return syntax_err(interp),
            }
        }
        // lseq n (to|count) n by n
        12121 => {
            match op(3) {
                Op::By => step = Some(n(4)),
                _ => return syntax_err(interp),
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
                _ => return syntax_err(interp),
            }
        }
        _ => return syntax_err(interp),
    }

    // A double-valued count is converted to an integer and does not, by itself,
    // make the sequence use doubles (C: "Don't consider Count type ...").
    if let Some(c) = count {
        if c.is_double {
            use_doubles = use_doubles.saturating_sub(1);
            if !c.d.is_finite() || c.d.floor() != c.d {
                return interp.set_error(b"expected integer but got non-integer count");
            }
        }
    }

    let elems = if use_doubles > 0 {
        build_double(&mut start, &end, &step, &count)
    } else {
        build_int(&start, &end, &step, &count)
    };
    let elems = match elems {
        Ok(v) => v,
        Err(msg) => return interp.set_error(msg),
    };
    interp.set_result(crate::list::new_list_obj(&elems));
    Code::Ok
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

/// Integer arithmetic series → element objects (`Tcl_WideInt` path).
fn build_int(
    start: &Num,
    end: &Option<Num>,
    step: &Option<Num>,
    count: &Option<Num>,
) -> Result<Vec<*mut TclObj>, &'static [u8]> {
    let s = start.i;
    // Length is computed in i128 so an extreme `end - start` (e.g.
    // `lseq 10 9223372036854775000`) cannot overflow i64 before the cap check.
    let (len, st): (i128, i64) = if let Some(c) = count {
        // count given: len = count, step defaults to 1.
        let st = step.map_or(1, |x| x.i);
        (i128::from(c.i).max(0), st)
    } else {
        let e = end.expect("int series without end or count has a count").i;
        // step defaults to ±1 by direction when omitted.
        let st = match step {
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
        out.push(obj::new_wide_int_obj(v));
    }
    Ok(out)
}

/// Double arithmetic series → element objects, with C's precision matching.
fn build_double(
    start: &mut Num,
    end: &Option<Num>,
    step: &Option<Num>,
    count: &Option<Num>,
) -> Result<Vec<*mut TclObj>, &'static [u8]> {
    let ds = start.d;
    let prec = {
        // maxObjPrecision(start, end, step) — count is excluded.
        let mut p = step.map_or(0, |x| x.prec);
        p = p.max(start.prec);
        if let Some(e) = end {
            p = p.max(e.prec);
        }
        p
    };
    let (len, dstep) = if let Some(c) = count {
        let dstep = step.map_or(1.0, |x| x.d);
        (c.i.max(0), dstep)
    } else {
        let de = end.expect("double series without end or count").d;
        let dstep = match step {
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
                return Err(b"cannot use non-numeric floating-point value to estimate length of arith-series");
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
        out.push(obj::new_double_obj(d));
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
    if len < 0.0 {
        0
    } else {
        len as i64
    }
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    /// Evaluate `src` leak-checked, asserting success, and return the result.
    fn ok(src: &[u8]) -> Vec<u8> {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(counters::finalize(), 0, "leak");
        assert_eq!(counters::double_free_count(), 0);
        assert_eq!(
            code,
            Code::Ok,
            "result={:?}",
            String::from_utf8_lossy(&bytes)
        );
        bytes
    }

    fn err(src: &[u8]) -> Vec<u8> {
        counters::reset();
        let (code, bytes);
        {
            let mut i = Interp::new();
            code = i.eval_str(src);
            bytes = i.result_bytes();
        }
        assert_eq!(counters::finalize(), 0, "leak");
        assert_eq!(code, Code::Error);
        bytes
    }

    #[test]
    fn lseq_integer_forms() {
        assert_eq!(ok(b"lseq 5"), b"0 1 2 3 4");
        assert_eq!(ok(b"lseq 0"), b"");
        assert_eq!(ok(b"lseq -5"), b""); // negative count → empty
        assert_eq!(ok(b"lseq 1 .. 10"), b"1 2 3 4 5 6 7 8 9 10");
        assert_eq!(ok(b"lseq 10 .. 1"), b"10 9 8 7 6 5 4 3 2 1");
        assert_eq!(ok(b"lseq 1 to 10 by 2"), b"1 3 5 7 9");
        assert_eq!(ok(b"lseq 1 to 10 by -2"), b""); // wrong-sign step → empty
        assert_eq!(ok(b"lseq 1 to 5 by 0"), b""); // zero step → empty
        assert_eq!(ok(b"lseq 5 count 5"), b"5 6 7 8 9");
        assert_eq!(ok(b"lseq 5 count 5 by -2"), b"5 3 1 -1 -3");
        assert_eq!(ok(b"lseq 3 by 2"), b"0 2 4");
        assert_eq!(ok(b"lseq 1 5"), b"1 2 3 4 5");
        assert_eq!(ok(b"lseq 5 1"), b"5 4 3 2 1");
    }

    #[test]
    fn lseq_double_precision() {
        // Precision matches the inputs' fractional digits (`maxObjPrecision`).
        assert_eq!(ok(b"lseq 0 0.5 by 0.1"), b"0.0 0.1 0.2 0.3 0.4 0.5");
        assert_eq!(ok(b"lseq 25. to 5. by -5"), b"25.0 20.0 15.0 10.0 5.0");
        assert_eq!(
            ok(b"lseq 3.5 18.5 1.5"),
            b"3.5 5.0 6.5 8.0 9.5 11.0 12.5 14.0 15.5 17.0 18.5"
        );
        // A double-valued count is used as an integer and stays an int sequence.
        assert_eq!(ok(b"lseq 5 count 5.0"), b"5 6 7 8 9");
    }

    #[test]
    fn lseq_expression_args() {
        assert_eq!(ok(b"lseq 1+2 to 10"), b"3 4 5 6 7 8 9 10");
        assert_eq!(ok(b"set n 3; lseq $n*2"), b"0 1 2 3 4 5");
    }

    #[test]
    fn lseq_errors() {
        assert_eq!(
            err(b"lseq"),
            b"wrong # args: should be \"lseq n ??op? n ??by? n??\""
        );
        assert_eq!(
            err(b"lseq 12 to 24 by 2 count"),
            b"wrong # args: should be \"lseq n ??op? n ??by? n??\""
        );
        // Huge series are capped rather than OOM-aborting.
        assert_eq!(
            err(b"lseq 10 2147483647"),
            b"max length of a Tcl list exceeded"
        );
    }
}
