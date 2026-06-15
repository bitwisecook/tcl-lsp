//! Commands and the M1 builtin set.
//!
//! For M1 a [`Command`] is a native Rust handler. The enum exists so M2 can add
//! `Proc`/`Alias`/`Ensemble` variants (the dispatcher already routes through it),
//! and the trampoline pushes a proc activation rather than recursing.

use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// A native builtin: receives argv *without* the command name (Tcl's `objv[1..]`).
pub type BuiltinFn = fn(&mut Vm, &[Value]) -> Completion<Value>;

/// A registered command. M1 only has builtins; proc/alias/ensemble land in M2.
#[derive(Clone, Copy)]
pub enum Command {
    /// A native Rust handler.
    Builtin(BuiltinFn),
}

/// Register the M1 builtin set on `vm`.
pub(crate) fn register_builtins(vm: &mut Vm) {
    vm.register("set", cmd_set);
    vm.register("puts", cmd_puts);
    vm.register("incr", cmd_incr);
    vm.register("expr", cmd_expr);
}

/// `set varName ?newValue?` — read or write a scalar.
fn cmd_set(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    match args {
        [name] => {
            let n = name.to_str();
            vm.get_var(&n)
                .map_or_else(|| err(format!("can't read \"{n}\": no such variable")), ok)
        }
        [name, value] => {
            vm.set_var(&name.to_str(), value.clone());
            ok(value.clone())
        }
        _ => err("wrong # args: should be \"set varName ?newValue?\""),
    }
}

/// `puts ?-nonewline? ?channelId? string` — write to the VM's output sink.
fn cmd_puts(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let mut rest = args;
    let mut newline = true;
    if let Some(first) = rest.first()
        && &*first.to_str() == "-nonewline"
    {
        newline = false;
        rest = &rest[1..];
    }
    let text = match rest {
        [string] => string.to_str(),
        // `puts channelId string` — M1 ignores the channel and writes stdout.
        [_channel, string] => string.to_str(),
        _ => {
            return err("wrong # args: should be \"puts ?-nonewline? ?channelId? string\"");
        }
    };
    vm.write_output(&text, newline);
    ok(Value::empty())
}

/// `incr varName ?increment?` — add to an integer variable (default 1; missing
/// variable starts at 0).
fn cmd_incr(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let (name, amount) = match args {
        [name] => (name.to_str(), 1),
        [name, inc] => match inc.as_int() {
            Ok(n) => (name.to_str(), n),
            Err(e) => return err(e.message),
        },
        _ => return err("wrong # args: should be \"incr varName ?increment?\""),
    };
    let old = match vm.get_var(&name) {
        Some(v) => match v.as_int() {
            Ok(n) => n,
            Err(e) => return err(e.message),
        },
        None => 0,
    };
    let next = Value::int(old.wrapping_add(amount));
    vm.set_var(&name, next.clone());
    ok(next)
}

/// `expr arg ?arg ...?` — concatenate the args and evaluate as an expression.
fn cmd_expr(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if args.is_empty() {
        return err("wrong # args: should be \"expr arg ?arg ...?\"");
    }
    let joined = args
        .iter()
        .map(|v| v.to_str().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    match vm.eval_expr(&joined) {
        Ok(v) => ok(v),
        Err(e) => err(e.message),
    }
}
