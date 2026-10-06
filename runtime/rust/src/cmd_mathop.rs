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

//! `::tcl::mathop::*` — the `expr` operators as **real commands**.
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

use crate::interp::{Code, Interp};
use crate::obj::Owned;
use crate::obj::TclObj;
use tcl_syntax::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS};

/// Every operator spelling with a `::tcl::mathop` command form — derived from
/// `tcl_syntax::expr::operators`, the single source of truth for which
/// operators exist and whether they have a mathop command form at all,
/// rather than a hand-typed list that could silently drift from the
/// operator grammar it mirrors.
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
    if interp
        .native_invocation_dialect()
        .native_string_protocol()
        .and_then(|protocol| protocol.tcl_version())
        .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_5)
    {
        let mut namespaces = interp.namespaces_mut();
        let namespace = namespaces
            .find_namespace(crate::namespace::GLOBAL, b"::tcl::mathop")
            .expect("registered math operator namespace");
        namespaces.export(namespace, b"*");
    }
}

fn expr_error(interp: &mut Interp, e: crate::expr_error::ExprError) -> Code {
    interp.report_expr_error(e)
}

/// A no-op `ExprCtx`: `mathop`'s operands are already evaluated, so the
/// `$var`/`[cmd]`/`func()` resolution is never reached.
struct NoCtx(tcl_registry::InvocationDialect);
impl crate::expr::ExprCtx for NoCtx {
    fn invocation_dialect(&self) -> tcl_registry::InvocationDialect {
        self.0
    }
    fn read_var(&mut self, _: &str) -> Result<Owned, crate::expr_error::ExprError> {
        unreachable!("mathop operands are pre-evaluated")
    }
    fn eval_command(&mut self, _: &str) -> Result<Owned, crate::expr_error::ExprError> {
        unreachable!("mathop operands are pre-evaluated")
    }
    fn call_function(
        &mut self,
        _: &str,
        _: &[Owned],
    ) -> Result<Owned, crate::expr_error::ExprError> {
        unreachable!("mathop operands are pre-evaluated")
    }
}

/// Selected handler identity supplies the operation; the original argv head
/// supplies only native usage presentation when arity validation fails.
fn mathop(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_cmd_core::mathop::MathopError;
    let Some(op) = interp
        .active_native_builtin_identity()
        .and_then(|identity| tcl_cmd_core::mathop::operation_for_handler_identity(&identity))
    else {
        return interp.refuse_native_access(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "math operator handler identity",
            ),
        );
    };
    // Borrow each operand (+1, released when the `Owned` wrappers drop).
    let args: Vec<Owned> = argv[1..].iter().map(|&a| Owned::retain(a)).collect();
    let mut ctx = NoCtx(interp.native_invocation_dialect());
    match crate::expr::eval_mathop(op, args, &mut ctx) {
        Ok(result) => {
            interp.set_result(result.as_ptr());
            Code::Ok
        }
        Err(MathopError::WrongArgs(usage)) => {
            interp.wrong_args_for_invocation(argv, usage.as_bytes())
        }
        Err(MathopError::Op(e)) => expr_error(interp, e),
    }
}

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    #[test]
    fn selected_mathop_identity_matches_all_60_native_name_and_argv_controls() {
        let rows =
            include_str!("../../../rust/tcl-cmd-core/tests/data/native_mathop_identity/rows.txt");
        let decode = |text: &str| {
            text.as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()
        };
        let mut count = 0;
        for row in rows.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(fields[0]),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            let code = interp.eval_str(&decode(fields[4]));
            assert_eq!(
                code.as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{}/{}",
                fields[0],
                fields[1]
            );
            assert_eq!(
                interp.result_bytes(),
                decode(fields[3]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            count += 1;
        }
        assert_eq!(count, 60);
    }

    #[test]
    fn original_mathop_compilation_matches_all_84_native_controls() {
        let rows = include_str!(
            "../../../rust/tcl-cmd-core/tests/data/native_mathop_compilation/rows.txt"
        );
        let decode = |text: &str| {
            text.as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()
        };
        let mut count = 0;
        for row in rows.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(fields[0]),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            let code = interp.eval_str(&decode(fields[4]));
            assert_eq!(
                code.as_int(),
                fields[2].parse::<i64>().unwrap(),
                "{}/{}",
                fields[0],
                fields[1]
            );
            assert_eq!(
                interp.result_bytes(),
                decode(fields[3]),
                "{}/{}",
                fields[0],
                fields[1]
            );
            count += 1;
        }
        assert_eq!(count, 84);
    }

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
