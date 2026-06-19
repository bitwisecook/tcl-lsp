//! `::tcl::mathop::*` — the `expr` operators as **real commands** (T1.5).
//!
//! C Tcl 9 (`tclMathOp.c`) exposes every `expr` operator as a command in
//! `::tcl::mathop::` with variadic fold / chained-comparison semantics. These
//! reuse the **shared numeric tower** ([`crate::bignum`], the same ops `expr`'s
//! `arith` walk uses) and the shared comparison rule — only the fold/identity/
//! arity wrapping is new. The operators are *not* on `expr`'s inline path
//! (the A3 contract: don't conflate `expr`'s op dispatch with the command), but
//! they exist as commands and are overridable like any other.
//!
//! Tower-gated like `expr`. Semantics verified against tclsh 9.0.

use core::cmp::Ordering;

use tcl_syntax::naming::qualifier_segments;

use crate::bignum::{self, ArithError};
use crate::expr::Owned;
use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};

/// Every operator spelling, registered as `::tcl::mathop::<op>`.
const MATHOPS: &[&str] = &[
    "~", "!", "+", "-", "*", "/", "%", "**", "&", "|", "^", "<<", ">>", "==", "!=", "<", "<=", ">",
    ">=", "eq", "ne", "in", "ni",
];

/// Register `::tcl::mathop::*`.
pub fn install(interp: &mut Interp) {
    for op in MATHOPS {
        let mut full = b"::tcl::mathop::".to_vec();
        full.extend_from_slice(op.as_bytes());
        interp.register_builtin(&full, mathop);
    }
}

type BinOpFn = fn(*mut TclObj, *mut TclObj) -> Result<*mut TclObj, ArithError>;

fn arith_error(interp: &mut Interp, e: ArithError) -> Code {
    expr_error(interp, crate::expr::arith_err(e))
}

/// Raise an `ExprError`, honouring its optional `-errorcode`.
fn expr_error(interp: &mut Interp, e: crate::expr::ExprError) -> Code {
    match e.code {
        Some(c) => interp.error_with_code(&e.msg, &c),
        None => interp.set_error(&e.msg),
    }
}

/// `wrong # args: should be "::tcl::mathop::<op> <usage>"`.
fn wrong(interp: &mut Interp, op: &[u8], usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"::tcl::mathop::".to_vec();
    m.extend_from_slice(op);
    m.push(b' ');
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

/// The one builtin behind every operator; `argv[0]`'s tail selects the op.
fn mathop(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let name0 = obj_bytes(argv[0]);
    let op = qualifier_segments(&name0)
        .last()
        .copied()
        .unwrap_or(&name0[..])
        .to_vec();
    let args = &argv[1..];
    match op.as_slice() {
        // Variadic fold with an identity (0 args → identity).
        b"+" => fold(interp, args, 0, bignum::add),
        b"*" => fold(interp, args, 1, bignum::mul),
        b"&" => fold(interp, args, -1, bignum::band),
        b"|" => fold(interp, args, 0, bignum::bor),
        b"^" => fold(interp, args, 0, bignum::bxor),
        // Subtraction: 1 arg negates, ≥2 left-folds.
        b"-" => sub_or_neg(interp, args),
        // Division: 1 arg is the reciprocal (1.0/x), ≥2 left-folds.
        b"/" => div_or_recip(interp, args),
        // Exponentiation: right-associative fold (0 args → 1).
        b"**" => pow_fold(interp, args),
        // Strict binaries.
        b"%" => binary(interp, args, b"%", b"integer integer", bignum::mod_),
        b"<<" => binary(interp, args, b"<<", b"integer shift", bignum::shl),
        b">>" => binary(interp, args, b">>", b"integer shift", bignum::shr),
        // Unaries.
        b"~" => {
            if args.len() != 1 {
                return wrong(interp, b"~", b"integer");
            }
            match bignum::bnot(args[0]) {
                Ok(r) => set_owned(interp, r),
                Err(e) => arith_error(interp, e),
            }
        }
        b"!" => {
            if args.len() != 1 {
                return wrong(interp, b"!", b"boolean");
            }
            match crate::expr::to_bool(args[0]) {
                Ok(b) => bool_result(interp, !b),
                Err(e) => expr_error(interp, e),
            }
        }
        // Chained comparisons (vacuously true for <2 args).
        b"==" => chain(interp, args, false, |o| o == Ordering::Equal),
        b"<" => chain(interp, args, false, |o| o == Ordering::Less),
        b">" => chain(interp, args, false, |o| o == Ordering::Greater),
        b"<=" => chain(interp, args, false, |o| o != Ordering::Greater),
        b">=" => chain(interp, args, false, |o| o != Ordering::Less),
        b"eq" => chain(interp, args, true, |o| o == Ordering::Equal),
        // Strict-binary (in)equality.
        b"!=" => ne_binary(interp, args, b"!=", false),
        b"ne" => ne_binary(interp, args, b"ne", true),
        // List membership.
        b"in" => membership(interp, args, b"in", false),
        b"ni" => membership(interp, args, b"ni", true),
        _ => interp.set_error(b"unknown math operator"),
    }
}

/// Adopt a fresh (`rc 0`) tower result into the interp result.
fn set_owned(interp: &mut Interp, r: *mut TclObj) -> Code {
    interp.set_result(r); // adopts rc-0 (retains to rc 1)
    Code::Ok
}

fn bool_result(interp: &mut Interp, b: bool) -> Code {
    interp.set_result(obj::new_wide_int_obj(i64::from(b)));
    Code::Ok
}

/// Variadic fold from `identity` over `op` (`+`/`*`/`&`/`|`/`^`).
fn fold(interp: &mut Interp, args: &[*mut TclObj], identity: i64, op: BinOpFn) -> Code {
    let mut acc = Owned::retain(obj::new_wide_int_obj(identity));
    for &a in args {
        match op(acc.as_ptr(), a) {
            Ok(r) => acc = Owned::retain(r), // r is rc 0; retain to 1, old acc drops
            Err(e) => return arith_error(interp, e),
        }
    }
    interp.set_result(acc.as_ptr());
    Code::Ok
}

fn sub_or_neg(interp: &mut Interp, args: &[*mut TclObj]) -> Code {
    match args {
        [] => wrong(interp, b"-", b"value ?value ...?"),
        [x] => match bignum::neg(*x) {
            Ok(r) => set_owned(interp, r),
            Err(e) => arith_error(interp, e),
        },
        [first, rest @ ..] => left_fold(interp, *first, rest, bignum::sub),
    }
}

fn div_or_recip(interp: &mut Interp, args: &[*mut TclObj]) -> Code {
    match args {
        [] => wrong(interp, b"/", b"value ?value ...?"),
        [x] => {
            // Reciprocal: `1.0 / x` (always a double, matching tclsh).
            let one = Owned::retain(obj::new_double_obj(1.0));
            match bignum::div(one.as_ptr(), *x) {
                Ok(r) => set_owned(interp, r),
                Err(e) => arith_error(interp, e),
            }
        }
        [first, rest @ ..] => left_fold(interp, *first, rest, bignum::div),
    }
}

/// Left-fold `op` over `first` then `rest` (for `-`/`/` with ≥2 args).
fn left_fold(interp: &mut Interp, first: *mut TclObj, rest: &[*mut TclObj], op: BinOpFn) -> Code {
    let mut acc = Owned::retain(first);
    for &a in rest {
        match op(acc.as_ptr(), a) {
            Ok(r) => acc = Owned::retain(r),
            Err(e) => return arith_error(interp, e),
        }
    }
    interp.set_result(acc.as_ptr());
    Code::Ok
}

/// Right-associative fold for `**` (`a ** b ** c` = `a ** (b ** c)`).
fn pow_fold(interp: &mut Interp, args: &[*mut TclObj]) -> Code {
    let Some((last, init)) = args.split_last() else {
        interp.set_result(obj::new_wide_int_obj(1)); // 0 args → 1
        return Code::Ok;
    };
    let mut acc = Owned::retain(*last);
    for &a in init.iter().rev() {
        match bignum::pow(a, acc.as_ptr()) {
            Ok(r) => acc = Owned::retain(r),
            Err(e) => return arith_error(interp, e),
        }
    }
    interp.set_result(acc.as_ptr());
    Code::Ok
}

fn binary(interp: &mut Interp, args: &[*mut TclObj], op: &[u8], usage: &[u8], f: BinOpFn) -> Code {
    if args.len() != 2 {
        return wrong(interp, op, usage);
    }
    match f(args[0], args[1]) {
        Ok(r) => set_owned(interp, r),
        Err(e) => arith_error(interp, e),
    }
}

/// Compare two values by the `expr` rule: numeric if both parse as numbers, else
/// a byte-wise string compare.
fn compare(a: *mut TclObj, b: *mut TclObj) -> Ordering {
    match bignum::compare(a, b) {
        Some(ord) => ord,
        None => obj::bytes_of(a).cmp(&obj::bytes_of(b)),
    }
}

/// Chained comparison: every adjacent pair must satisfy `pred` (vacuously true
/// for fewer than two operands). `string_only` forces a string compare (`eq`).
fn chain(
    interp: &mut Interp,
    args: &[*mut TclObj],
    string_only: bool,
    pred: fn(Ordering) -> bool,
) -> Code {
    for w in args.windows(2) {
        let ord = if string_only {
            obj::bytes_of(w[0]).cmp(&obj::bytes_of(w[1]))
        } else {
            compare(w[0], w[1])
        };
        if !pred(ord) {
            return bool_result(interp, false);
        }
    }
    bool_result(interp, true)
}

/// Strict-binary inequality (`!=` numeric-or-string, `ne` string-only).
fn ne_binary(interp: &mut Interp, args: &[*mut TclObj], op: &[u8], string_only: bool) -> Code {
    if args.len() != 2 {
        return wrong(interp, op, b"value value");
    }
    let ne = if string_only {
        obj::bytes_of(args[0]) != obj::bytes_of(args[1])
    } else {
        compare(args[0], args[1]) != Ordering::Equal
    };
    bool_result(interp, ne)
}

/// `in` / `ni` list membership.
fn membership(interp: &mut Interp, args: &[*mut TclObj], op: &[u8], negate: bool) -> Code {
    if args.len() != 2 {
        return wrong(interp, op, b"value list");
    }
    let hay = obj_bytes(args[1]);
    let Ok(s) = core::str::from_utf8(&hay) else {
        return interp.set_error(b"unmatched open brace in list");
    };
    let Ok(elems) = tcl_syntax::list::split_list(s) else {
        return interp.set_error(b"unmatched open brace in list");
    };
    let needle = obj_bytes(args[0]);
    let member = elems.iter().any(|e| e.as_bytes() == needle.as_slice());
    bool_result(interp, member ^ negate)
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
        assert_eq!(counters::double_free_count(), 0);
    }

    fn ev(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?}",
            String::from_utf8_lossy(src)
        );
        i.result_bytes()
    }

    #[test]
    fn arithmetic_folds_and_identities() {
        leak_free(|i| {
            assert_eq!(ev(i, b"::tcl::mathop::+"), b"0");
            assert_eq!(ev(i, b"::tcl::mathop::*"), b"1");
            assert_eq!(ev(i, b"::tcl::mathop::&"), b"-1");
            assert_eq!(ev(i, b"::tcl::mathop::+ 1 2 3"), b"6");
            assert_eq!(ev(i, b"::tcl::mathop::* 2 3 4"), b"24");
            assert_eq!(ev(i, b"::tcl::mathop::+ 1 2.5"), b"3.5");
        });
    }

    #[test]
    fn sub_and_div() {
        leak_free(|i| {
            assert_eq!(ev(i, b"::tcl::mathop::- 5"), b"-5"); // negate
            assert_eq!(ev(i, b"::tcl::mathop::- 10 1 2"), b"7"); // left fold
            assert_eq!(ev(i, b"::tcl::mathop::/ 8"), b"0.125"); // reciprocal
            assert_eq!(ev(i, b"::tcl::mathop::/ 100 2 5"), b"10"); // int floor fold
            assert_eq!(ev(i, b"::tcl::mathop::/ 7 2"), b"3");
            assert_eq!(i.eval_str(b"::tcl::mathop::-"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"::tcl::mathop::- value ?value ...?\""
            );
        });
    }

    #[test]
    fn pow_is_right_associative() {
        leak_free(|i| {
            assert_eq!(ev(i, b"::tcl::mathop::** 2 3 2"), b"512"); // 2^(3^2)
            assert_eq!(ev(i, b"::tcl::mathop::** 2"), b"2");
            assert_eq!(ev(i, b"::tcl::mathop::**"), b"1");
        });
    }

    #[test]
    fn binaries_and_unaries() {
        leak_free(|i| {
            assert_eq!(ev(i, b"::tcl::mathop::% 17 5"), b"2");
            assert_eq!(ev(i, b"::tcl::mathop::<< 1 4"), b"16");
            assert_eq!(ev(i, b"::tcl::mathop::>> 256 2"), b"64");
            assert_eq!(ev(i, b"::tcl::mathop::~ 5"), b"-6");
            assert_eq!(ev(i, b"::tcl::mathop::! 0"), b"1");
            assert_eq!(ev(i, b"::tcl::mathop::! 5"), b"0");
            assert_eq!(i.eval_str(b"::tcl::mathop::<<"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"::tcl::mathop::<< integer shift\""
            );
        });
    }

    #[test]
    fn comparisons_chained_and_binary() {
        leak_free(|i| {
            assert_eq!(ev(i, b"::tcl::mathop::== 3 3 3"), b"1");
            assert_eq!(ev(i, b"::tcl::mathop::< 1 2 3"), b"1");
            assert_eq!(ev(i, b"::tcl::mathop::< 1 3 2"), b"0");
            assert_eq!(ev(i, b"::tcl::mathop::<"), b"1"); // vacuous
            assert_eq!(ev(i, b"::tcl::mathop::!= 1 2"), b"1");
            assert_eq!(ev(i, b"::tcl::mathop::eq a a"), b"1");
            assert_eq!(ev(i, b"::tcl::mathop::ne a b"), b"1");
            assert_eq!(ev(i, b"::tcl::mathop::in b {a b c}"), b"1");
            assert_eq!(ev(i, b"::tcl::mathop::ni z {a b c}"), b"1");
            // `!=` is strict-binary, not chained.
            assert_eq!(i.eval_str(b"::tcl::mathop::!= 1 2 3"), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"::tcl::mathop::!= value value\""
            );
        });
    }
}
