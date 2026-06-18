//! The `array` ensemble builtin — a thin adapter over the shared
//! [`tcl_cmd_core::array`] core. The read-side (`exists`/`size`/`names`/`get`) and
//! `unset` are shared over the VM's `VarStore`/`Frames`/`ValueOps`; `set` (whose
//! per-element write traces must fail the command) stays here. Sharing fixed the
//! VM's `array unset a` (no pattern), which used to iterate-and-unset elements
//! (leaving an empty array) instead of removing the whole array.

use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("array", cmd_array);
    // Ensemble member commands the codegen rewrites `array <sub>` into.
    vm.register("::tcl::array::exists", |vm, a| array_op(vm, "exists", a));
    vm.register("::tcl::array::names", |vm, a| array_op(vm, "names", a));
    vm.register("::tcl::array::get", |vm, a| array_op(vm, "get", a));
    vm.register("::tcl::array::set", |vm, a| array_op(vm, "set", a));
    vm.register("::tcl::array::size", |vm, a| array_op(vm, "size", a));
    vm.register("::tcl::array::unset", |vm, a| array_op(vm, "unset", a));
}

/// `array option arrayName ?arg ...?` — dispatch to the subcommand handler.
fn cmd_array(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"array option arrayName ?arg ...?\"");
    };
    array_op(vm, &sub.to_str(), rest)
}

fn array_op(vm: &mut Vm, sub: &str, rest: &[Value]) -> Completion<Value> {
    // The read-side + `unset` live in the shared core.
    if let Some(result) = tcl_cmd_core::array::dispatch(vm, sub, rest) {
        return match result {
            Ok(v) => ok(v),
            Err(e) => err(e.into_message()),
        };
    }
    // Per-runtime: `array set` (its per-element write traces must fail the
    // command) and the unknown-subcommand message.
    match sub {
        "set" => match rest {
            [n, list] => {
                let items = match list.as_list() {
                    Ok(i) => i,
                    Err(e) => return err(e.message),
                };
                if items.len() % 2 != 0 {
                    return err("list must have an even number of elements");
                }
                let name = n.to_str();
                let mut i = 0;
                while i + 1 < items.len() {
                    if let Err(e) =
                        vm.set_array_elem(&name, &items[i].to_str(), items[i + 1].clone())
                    {
                        return e;
                    }
                    i += 2;
                }
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"array set arrayName list\""),
        },
        other => err(format!(
            "unknown or ambiguous subcommand \"{other}\": must be exists, get, names, set, size, or unset"
        )),
    }
}
