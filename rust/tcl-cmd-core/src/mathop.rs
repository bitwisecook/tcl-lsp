//! `::tcl::mathop::*` — the `expr` operators as commands, shared over
//! [`ExprOps`](tcl_syntax::expr::ExprOps).
//!
//! C Tcl 9 (`tclMathOp.c`) exposes every `expr` operator as a command with
//! variadic fold / chained-comparison semantics. Those reduce to the same
//! per-operator primitives `expr` itself uses, which both runtimes already
//! expose via `ExprOps` — so this core needs **no new value seam**: the WASM
//! runtime drives it over its bignum tower, the bytecode VM over its
//! `i64`+`double` model, each via its own `ExprOps` impl. Only the fold /
//! identity / arity wrapping lives here.
//!
//! Semantics verified against tclsh 9.0.

use core::cmp::Ordering;

use tcl_syntax::expr::ExprOps;
use tcl_syntax::expr::ast::{BinOp, UnaryOp};

/// A `mathop` failure: a wrong-args (arity) error carrying the operator's usage
/// string, or an arithmetic/coercion error from the value ops.
pub enum MathopError<E> {
    /// `wrong # args` — the value is the `<usage>` part of the message.
    WrongArgs(&'static str),
    /// An error from an [`ExprOps`] operation (overflow, divide-by-zero, …).
    Op(E),
}

/// Evaluate `::tcl::mathop::<op>` over the already-evaluated `args` (the operator
/// spelling is the command-name tail). The number model is whatever `ops`
/// provides.
pub fn eval<O: ExprOps>(
    ops: &mut O,
    op: &str,
    args: Vec<O::Value>,
) -> Result<O::Value, MathopError<O::Error>> {
    use MathopError::Op;
    match op {
        // Variadic fold with an identity (0 args → identity).
        "+" => fold(ops, args, "0", BinOp::Add),
        "*" => fold(ops, args, "1", BinOp::Mul),
        "&" => fold(ops, args, "-1", BinOp::BitAnd),
        "|" => fold(ops, args, "0", BinOp::BitOr),
        "^" => fold(ops, args, "0", BinOp::BitXor),
        // Subtraction: 1 arg negates, ≥2 left-folds.
        "-" => match args.len() {
            0 => Err(MathopError::WrongArgs("value ?value ...?")),
            1 => ops.unary(UnaryOp::Neg, into_one(args)).map_err(Op),
            _ => left_fold(ops, args, BinOp::Sub),
        },
        // Division: 1 arg is the reciprocal (1.0/x), ≥2 left-folds.
        "/" => match args.len() {
            0 => Err(MathopError::WrongArgs("value ?value ...?")),
            1 => {
                let one = ops.literal("1.0").map_err(Op)?;
                ops.arith(BinOp::Div, one, into_one(args)).map_err(Op)
            }
            _ => left_fold(ops, args, BinOp::Div),
        },
        // Exponentiation: right-associative fold (0 args → 1).
        "**" => pow_fold(ops, args),
        // Strict binaries.
        "%" => binary(ops, args, "integer integer", BinOp::Mod),
        "<<" => binary(ops, args, "integer shift", BinOp::LShift),
        ">>" => binary(ops, args, "integer shift", BinOp::RShift),
        // Unaries.
        "~" => match into_n::<O, 1>(args) {
            Some([v]) => ops.unary(UnaryOp::BitNot, v).map_err(Op),
            None => Err(MathopError::WrongArgs("integer")),
        },
        // `!` shares `expr`'s unary-not path so a non-boolean/NaN operand is the
        // operand-type error (`cannot use … as operand of "!"`), not the generic
        // "expected boolean value".
        "!" => match into_n::<O, 1>(args) {
            Some([v]) => ops.unary(UnaryOp::Not, v).map_err(Op),
            None => Err(MathopError::WrongArgs("boolean")),
        },
        // Chained comparisons (vacuously true for <2 args).
        "==" => Ok(bool_chain(ops, &args, false, |o| o == Ordering::Equal)),
        "<" => Ok(bool_chain(ops, &args, false, |o| o == Ordering::Less)),
        ">" => Ok(bool_chain(ops, &args, false, |o| o == Ordering::Greater)),
        "<=" => Ok(bool_chain(ops, &args, false, |o| o != Ordering::Greater)),
        ">=" => Ok(bool_chain(ops, &args, false, |o| o != Ordering::Less)),
        "eq" => Ok(bool_chain(ops, &args, true, |o| o == Ordering::Equal)),
        // String-only chained ordering (the `eq`-style spellings of `<`/`>`/…).
        "lt" => Ok(bool_chain(ops, &args, true, |o| o == Ordering::Less)),
        "gt" => Ok(bool_chain(ops, &args, true, |o| o == Ordering::Greater)),
        "le" => Ok(bool_chain(ops, &args, true, |o| o != Ordering::Greater)),
        "ge" => Ok(bool_chain(ops, &args, true, |o| o != Ordering::Less)),
        // Strict-binary (in)equality.
        "!=" => ne_binary(ops, args, false),
        "ne" => ne_binary(ops, args, true),
        // List membership.
        "in" => membership(ops, args, false),
        "ni" => membership(ops, args, true),
        _ => Err(MathopError::WrongArgs("value ?value ...?")),
    }
}

/// Variadic fold over `op` (`+`/`*`/`&`/`|`/`^`).
///
/// No args → `identity`. Otherwise the *first* argument seeds the accumulator
/// (matching C: `+ a b` is `a + b`, so `a` is the left operand — `+ x 0`
/// reports x as the *left* operand, not the right). A lone argument is still
/// validated by combining it with the identity on the right, so `+ x` errors on
/// the left operand while `+ 5` stays 5.
fn fold<O: ExprOps>(
    ops: &mut O,
    args: Vec<O::Value>,
    identity: &str,
    op: BinOp,
) -> Result<O::Value, MathopError<O::Error>> {
    let mut it = args.into_iter();
    let Some(first) = it.next() else {
        return ops.literal(identity).map_err(MathopError::Op);
    };
    let mut acc = first;
    let mut folded = false;
    for a in it {
        acc = ops.arith(op, acc, a).map_err(MathopError::Op)?;
        folded = true;
    }
    if !folded {
        let id = ops.literal(identity).map_err(MathopError::Op)?;
        acc = ops.arith(op, acc, id).map_err(MathopError::Op)?;
    }
    Ok(acc)
}

/// Left-fold `op` over the first arg then the rest (for `-`/`/` with ≥2 args).
fn left_fold<O: ExprOps>(
    ops: &mut O,
    args: Vec<O::Value>,
    op: BinOp,
) -> Result<O::Value, MathopError<O::Error>> {
    let mut it = args.into_iter();
    let mut acc = it.next().expect("≥2 args");
    for a in it {
        acc = ops.arith(op, acc, a).map_err(MathopError::Op)?;
    }
    Ok(acc)
}

/// Right-associative fold for `**` (`a ** b ** c` = `a ** (b ** c)`).
fn pow_fold<O: ExprOps>(
    ops: &mut O,
    args: Vec<O::Value>,
) -> Result<O::Value, MathopError<O::Error>> {
    let mut it = args.into_iter().rev();
    let Some(mut acc) = it.next() else {
        return ops.literal("1").map_err(MathopError::Op); // 0 args → 1
    };
    for a in it {
        acc = ops.arith(BinOp::Pow, a, acc).map_err(MathopError::Op)?;
    }
    Ok(acc)
}

/// A strict-binary arithmetic op (`%`/`<<`/`>>`): exactly two operands.
fn binary<O: ExprOps>(
    ops: &mut O,
    args: Vec<O::Value>,
    usage: &'static str,
    op: BinOp,
) -> Result<O::Value, MathopError<O::Error>> {
    match into_n::<O, 2>(args) {
        Some([a, b]) => ops.arith(op, a, b).map_err(MathopError::Op),
        None => Err(MathopError::WrongArgs(usage)),
    }
}

/// Chained comparison → a boolean value: every adjacent pair must satisfy
/// `pred` (vacuously true for fewer than two operands). `string_only` forces a
/// string compare (`eq`); otherwise it is numeric-if-both-numeric else string.
fn bool_chain<O: ExprOps>(
    ops: &mut O,
    args: &[O::Value],
    string_only: bool,
    pred: impl Fn(Ordering) -> bool,
) -> O::Value {
    for w in args.windows(2) {
        let ord = compare(ops, &w[0], &w[1], string_only);
        if !pred(ord) {
            return ops.bool_value(false);
        }
    }
    ops.bool_value(true)
}

/// Strict-binary inequality (`!=` numeric-or-string, `ne` string-only).
fn ne_binary<O: ExprOps>(
    ops: &mut O,
    args: Vec<O::Value>,
    string_only: bool,
) -> Result<O::Value, MathopError<O::Error>> {
    match into_n::<O, 2>(args) {
        Some([a, b]) => {
            let ne = compare(ops, &a, &b, string_only) != Ordering::Equal;
            Ok(ops.bool_value(ne))
        }
        None => Err(MathopError::WrongArgs("value value")),
    }
}

/// `in` / `ni` list membership.
fn membership<O: ExprOps>(
    ops: &mut O,
    args: Vec<O::Value>,
    negate: bool,
) -> Result<O::Value, MathopError<O::Error>> {
    match into_n::<O, 2>(args) {
        Some([needle, list]) => {
            let member = ops.in_list(&needle, &list).map_err(MathopError::Op)?;
            Ok(ops.bool_value(member ^ negate))
        }
        None => Err(MathopError::WrongArgs("value list")),
    }
}

/// Compare two operands by the `expr` rule (numeric if both look numeric, else a
/// string compare); `string_only` forces the string compare (`eq`/`ne`).
fn compare<O: ExprOps>(ops: &mut O, a: &O::Value, b: &O::Value, string_only: bool) -> Ordering {
    if string_only {
        ops.compare_string(a, b)
    } else {
        match ops.compare_numeric(a, b) {
            Some(o) => o,
            None => ops.compare_string(a, b),
        }
    }
}

/// Move the single element out of a one-element vec (the caller checked `len`).
fn into_one<T>(args: Vec<T>) -> T {
    args.into_iter().next().expect("one arg")
}

/// `args` as a fixed `[V; N]`, or `None` if its length is not `N`.
fn into_n<O: ExprOps, const N: usize>(args: Vec<O::Value>) -> Option<[O::Value; N]> {
    <[O::Value; N]>::try_from(args).ok()
}
