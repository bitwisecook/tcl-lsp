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

use tcl_syntax::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS};
use tcl_syntax::naming::qualifier_segments;

use crate::expr::Owned;
use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Every operator spelling with a `::tcl::mathop` command form — derived from
/// `tcl_syntax::expr::operators`, the single source of truth for which
/// operators exist and whether they have a mathop command form at all
/// (issue #983's registry/runtime convergence: this used to be a hand-typed
/// list that could silently drift from the operator grammar it mirrors).
///
/// `BinOp`/`UnaryOp` share a spelling for `-`/`+` (`Sub`/`Neg`, `Add`/`Pos`) —
/// one command handles both the fold and the single-argument reading, so the
/// binary pass alone already covers them; the unary pass only contributes
/// truly unary-only spellings (`~`, `!`).
fn mathop_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = ALL_BIN_OPS
        .iter()
        .filter_map(|op| op.spec().mathop_shape.map(|_| op.spec().spelling))
        .collect();
    for op in ALL_UNARY_OPS {
        if op.spec().mathop_shape.is_some() {
            let spelling = op.spec().spelling;
            if !names.contains(&spelling) {
                names.push(spelling);
            }
        }
    }
    names
}

/// Register `::tcl::mathop::*`.
pub fn install(interp: &mut Interp) {
    for op in mathop_names() {
        let mut full = b"::tcl::mathop::".to_vec();
        full.extend_from_slice(op.as_bytes());
        interp.register_builtin(&full, mathop);
    }
}

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

/// A no-op `ExprCtx`: `mathop`'s operands are already evaluated, so the
/// `$var`/`[cmd]`/`func()` resolution is never reached.
struct NoCtx;
impl crate::expr::ExprCtx for NoCtx {
    fn read_var(&mut self, _: &str) -> Result<Owned, crate::expr::ExprError> {
        unreachable!("mathop operands are pre-evaluated")
    }
    fn eval_command(&mut self, _: &str) -> Result<Owned, crate::expr::ExprError> {
        unreachable!("mathop operands are pre-evaluated")
    }
    fn call_function(&mut self, _: &str, _: &[Owned]) -> Result<Owned, crate::expr::ExprError> {
        unreachable!("mathop operands are pre-evaluated")
    }
}

/// The one builtin behind every operator; `argv[0]`'s tail selects the op. The
/// fold / chained-comparison / arity logic is shared (`tcl_cmd_core::mathop`),
/// driven over this runtime's `ExprOps` (the bignum tower) so the result matches
/// `expr`.
fn mathop(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_cmd_core::mathop::MathopError;
    let name0 = obj_bytes(argv[0]);
    let op = qualifier_segments(&name0)
        .last()
        .copied()
        .unwrap_or(&name0[..]);
    let op_str = core::str::from_utf8(op).unwrap_or("");
    // Borrow each operand (+1, released when the `Owned` wrappers drop).
    let args: Vec<Owned> = argv[1..].iter().map(|&a| Owned::retain(a)).collect();
    let mut ctx = NoCtx;
    match crate::expr::eval_mathop(op_str, args, &mut ctx) {
        Ok(result) => {
            interp.set_result(result.as_ptr());
            Code::Ok
        }
        Err(MathopError::WrongArgs(usage)) => wrong(interp, op, usage.as_bytes()),
        Err(MathopError::Op(e)) => expr_error(interp, e),
    }
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
