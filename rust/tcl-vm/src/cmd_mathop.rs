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

//! `::tcl::mathop::*` — the `expr` operators as commands, newly added to the VM
//! over the shared `tcl_cmd_core::mathop` fold/chain logic and the VM's
//! `ExprEval` (`ExprOps`). The VM had no `mathop` before; it now gets every
//! operator over its `i64`+`double` number model (the runtime drives the same
//! core over its bignum tower).

use tcl_runtime_api::Completion;

use crate::expr::ExprEval;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// Drive one operator through the shared core over the VM's `ExprOps`.
fn dispatch(vm: &mut Vm, op: &str, args: &[Value]) -> Completion<Value> {
    use tcl_cmd_core::mathop::MathopError;
    // The usage names the operator as it was invoked: via `namespace path` the
    // word is just `!`, so the message reads `should be "! boolean"` rather than
    // the resolved `::tcl::mathop::!` (mathop-3.9/4.9/…).
    let invoked = vm
        .invoked_name()
        .map_or_else(|| format!("::tcl::mathop::{op}"), str::to_owned);
    let mut ops = ExprEval { vm };
    match tcl_cmd_core::mathop::eval(&mut ops, op, args.to_vec()) {
        Ok(v) => ok(v),
        Err(MathopError::WrongArgs(usage)) => {
            err(format!("wrong # args: should be \"{invoked} {usage}\""))
        }
        Err(MathopError::Op(e)) => err(e.message),
    }
}

/// Generate one fn-pointer builtin per operator (the VM's `BuiltinFn` is a bare
/// fn pointer, so it can't capture the op name) and register each under
/// `::tcl::mathop::<op>`.
macro_rules! mathops {
    ($($op:literal => $fn:ident),* $(,)?) => {
        $(
            fn $fn(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
                dispatch(vm, $op, args)
            }
        )*
        pub(crate) fn register(vm: &mut Vm) {
            $( vm.register(concat!("::tcl::mathop::", $op), $fn); )*
            // C exports every operator from `::tcl::mathop`, so
            // `namespace import ::tcl::mathop::*` works (mathop-25.*).
            vm.declare_namespace_exports("tcl::mathop", &["*"]);
        }
    };
}

mathops! {
    "~" => mathop_bitnot, "!" => mathop_not,
    "+" => mathop_add, "-" => mathop_sub, "*" => mathop_mul, "/" => mathop_div,
    "%" => mathop_mod, "**" => mathop_pow,
    "&" => mathop_band, "|" => mathop_bor, "^" => mathop_bxor,
    "<<" => mathop_shl, ">>" => mathop_shr,
    "==" => mathop_eq, "!=" => mathop_ne,
    "<" => mathop_lt, "<=" => mathop_le, ">" => mathop_gt, ">=" => mathop_ge,
    "eq" => mathop_seq, "ne" => mathop_sne,
    "lt" => mathop_slt, "le" => mathop_sle, "gt" => mathop_sgt, "ge" => mathop_sge,
    "in" => mathop_in, "ni" => mathop_ni,
}

#[cfg(test)]
mod tests {
    use tcl_syntax::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS};

    use crate::interp::Vm;

    /// Every operator spelling with a `::tcl::mathop` command form, per
    /// `tcl_syntax::expr::operators` — the same derivation
    /// `runtime/rust/src/cmd_mathop.rs` uses to build its registration list
    /// mechanically. The VM's own list above is a `macro_rules!` invocation
    /// (fn-pointer-per-operator, so it can't be generated the same way), so
    /// this test is this crate's half of issue #983/#987's registry/runtime
    /// convergence: a drift gate proving the macro's hand-typed spellings
    /// still exactly match layer 1, in both directions.
    fn expected_mathop_spellings() -> Vec<&'static str> {
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

    #[test]
    fn every_layer1_mathop_operator_is_registered_in_the_vm() {
        let vm = Vm::new();
        for spelling in expected_mathop_spellings() {
            let full = format!("::tcl::mathop::{spelling}");
            assert!(
                vm.lookup_command(&full).is_some(),
                "{full}: exists in tcl_syntax::expr::operators but not registered in the VM"
            );
        }
    }

    #[test]
    fn the_vm_registers_no_mathop_command_beyond_layer1() {
        let vm = Vm::new();
        let expected = expected_mathop_spellings();
        // The macro's own operator list, kept in sync with `expected` by this
        // test — if a future edit adds/removes a spelling here without a
        // matching layer-1 change (or vice versa), one of these two tests
        // catches it.
        let registered = [
            "~", "!", "+", "-", "*", "/", "%", "**", "&", "|", "^", "<<", ">>", "==", "!=", "<",
            "<=", ">", ">=", "eq", "ne", "lt", "le", "gt", "ge", "in", "ni",
        ];
        for spelling in registered {
            let full = format!("::tcl::mathop::{spelling}");
            assert!(
                vm.lookup_command(&full).is_some(),
                "{full}: in this test's own registered list but not actually registered"
            );
            assert!(
                expected.contains(&spelling),
                "{full}: registered in the VM but has no `mathop_shape` in \
                 tcl_syntax::expr::operators — stale entry?"
            );
        }
        assert_eq!(
            registered.len(),
            expected.len(),
            "the VM's registered mathop spellings and layer 1's mathop-shaped \
             operators have drifted apart (different counts)"
        );
    }
}
