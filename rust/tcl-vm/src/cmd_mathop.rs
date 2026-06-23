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
    let mut ops = ExprEval { vm };
    match tcl_cmd_core::mathop::eval(&mut ops, op, args.to_vec()) {
        Ok(v) => ok(v),
        Err(MathopError::WrongArgs(usage)) => err(format!(
            "wrong # args: should be \"::tcl::mathop::{op} {usage}\""
        )),
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
