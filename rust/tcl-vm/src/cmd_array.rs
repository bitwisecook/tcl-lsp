//! The `array` ensemble builtin.

use tcl_runtime_api::Completion;
use tcl_syntax::glob::string_match;

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

fn flatten(pairs: Vec<(String, Value)>) -> Vec<Value> {
    let mut v = Vec::with_capacity(pairs.len() * 2);
    for (k, val) in pairs {
        v.push(Value::string(k));
        v.push(val);
    }
    v
}

fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

fn array_op(vm: &mut Vm, sub: &str, rest: &[Value]) -> Completion<Value> {
    match sub {
        "exists" => match rest {
            [n] => ok(Value::bool(vm.array_is(&n.to_str()))),
            _ => err("wrong # args: should be \"array exists arrayName\""),
        },
        "names" => match rest {
            [n] => ok(Value::list(
                vm.array_pairs(&n.to_str())
                    .into_iter()
                    .map(|(k, _)| Value::string(k))
                    .collect(),
            )),
            [n, pat] => {
                let p = pat.to_str();
                ok(Value::list(
                    vm.array_pairs(&n.to_str())
                        .into_iter()
                        .filter(|(k, _)| string_match(&p, k))
                        .map(|(k, _)| Value::string(k))
                        .collect(),
                ))
            }
            _ => err("wrong # args: should be \"array names arrayName ?pattern?\""),
        },
        "get" => match rest {
            [n] => ok(Value::list(flatten(vm.array_pairs(&n.to_str())))),
            [n, pat] => {
                let p = pat.to_str();
                ok(Value::list(flatten(
                    vm.array_pairs(&n.to_str())
                        .into_iter()
                        .filter(|(k, _)| string_match(&p, k))
                        .collect(),
                )))
            }
            _ => err("wrong # args: should be \"array get arrayName ?pattern?\""),
        },
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
        "size" => match rest {
            [n] => ok(Value::int(ilen(vm.array_pairs(&n.to_str()).len()))),
            _ => err("wrong # args: should be \"array size arrayName\""),
        },
        "unset" => match rest {
            [n] => {
                let name = n.to_str();
                for (k, _) in vm.array_pairs(&name) {
                    vm.array_unset_elem(&name, &k);
                }
                ok(Value::empty())
            }
            [n, pat] => {
                let name = n.to_str();
                let p = pat.to_str();
                for (k, _) in vm.array_pairs(&name) {
                    if string_match(&p, &k) {
                        vm.array_unset_elem(&name, &k);
                    }
                }
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"array unset arrayName ?pattern?\""),
        },
        other => err(format!(
            "unknown or ambiguous subcommand \"{other}\": must be exists, get, names, set, size, or unset"
        )),
    }
}
