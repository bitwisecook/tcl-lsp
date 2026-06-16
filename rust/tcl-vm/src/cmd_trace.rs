//! The `trace` command — variable traces.
//!
//! Supports `trace add|remove|info variable name ops command` (the surface the
//! Tcl library — `tcltest` etc. — relies on). The firing engine lives in
//! [`Vm`]: read traces fire before a read, write traces after a write (with the
//! old value restored if the callback aborts the write), and unset traces
//! before removal (callback errors ignored). See `interp.rs::fire_var_traces`.
//!
//! Command and execution traces are accepted but not yet fired.

use tcl_runtime_api::Completion;
use tcl_syntax::list::split_list;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("trace", cmd_trace);
}

fn cmd_trace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"trace option ?arg ...?\"");
    };
    match &*sub.to_str() {
        "add" => trace_add(vm, rest),
        "remove" => trace_remove(vm, rest),
        "info" => trace_info(vm, rest),
        // Legacy 8.x forms map onto the variable engine.
        "variable" => legacy_variable(vm, rest, true),
        "vdelete" => legacy_variable(vm, rest, false),
        "vinfo" => trace_info_variable(vm, rest),
        other => err(format!(
            "bad option \"{other}\": must be add, info, or remove"
        )),
    }
}

/// Parse an ops word (`{read write}`) into a normalised list of op names.
fn parse_ops(spec: &str) -> Result<Vec<String>, Completion<Value>> {
    split_list(spec)
        .map(|ops| ops.iter().map(ToString::to_string).collect())
        .map_err(|e| err(e.message().to_string()))
}

fn trace_add(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((kind, args)) = rest.split_first() else {
        return err("wrong # args: should be \"trace add type ?arg ...?\"");
    };
    match &*kind.to_str() {
        "variable" => match args {
            [name, ops, command] => {
                let ops = match parse_ops(&ops.to_str()) {
                    Ok(o) => o,
                    Err(c) => return c,
                };
                vm.add_var_trace(&name.to_str(), ops, command.to_str().to_string());
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"trace add variable name ops command\""),
        },
        // Command / execution traces: accepted, not yet fired.
        "command" | "execution" => ok(Value::empty()),
        other => err(format!(
            "bad type \"{other}\": must be command, execution, or variable"
        )),
    }
}

fn trace_remove(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((kind, args)) = rest.split_first() else {
        return err("wrong # args: should be \"trace remove type ?arg ...?\"");
    };
    match &*kind.to_str() {
        "variable" => match args {
            [name, ops, command] => {
                let ops = match parse_ops(&ops.to_str()) {
                    Ok(o) => o,
                    Err(c) => return c,
                };
                vm.remove_var_trace(&name.to_str(), &ops, &command.to_str());
                ok(Value::empty())
            }
            _ => err("wrong # args: should be \"trace remove variable name ops command\""),
        },
        "command" | "execution" => ok(Value::empty()),
        other => err(format!(
            "bad type \"{other}\": must be command, execution, or variable"
        )),
    }
}

fn trace_info(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((kind, args)) = rest.split_first() else {
        return err("wrong # args: should be \"trace info type name\"");
    };
    match &*kind.to_str() {
        "variable" => trace_info_variable(vm, args),
        "command" | "execution" => ok(Value::list(Vec::new())),
        other => err(format!(
            "bad type \"{other}\": must be command, execution, or variable"
        )),
    }
}

/// `trace info variable name` → list of `{ops command}` pairs (newest first).
fn trace_info_variable(vm: &Vm, args: &[Value]) -> Completion<Value> {
    let [name] = args else {
        return err("wrong # args: should be \"trace info variable name\"");
    };
    let entries: Vec<Value> = vm
        .var_trace_info(&name.to_str())
        .into_iter()
        .map(|(ops, cmd)| {
            Value::list(vec![
                Value::list(ops.into_iter().map(Value::string).collect()),
                Value::string(cmd),
            ])
        })
        .collect();
    ok(Value::list(entries))
}

/// Legacy `trace variable name ops command` / `trace vdelete name ops command`.
/// The legacy op word uses single letters (`r`/`w`/`u`/`a`); normalise them.
fn legacy_variable(vm: &mut Vm, args: &[Value], add: bool) -> Completion<Value> {
    let [name, ops, command] = args else {
        return err("wrong # args: should be \"trace variable name ops command\"");
    };
    let ops: Vec<String> = ops
        .to_str()
        .chars()
        .filter_map(|c| match c {
            'r' => Some("read".to_string()),
            'w' => Some("write".to_string()),
            'u' => Some("unset".to_string()),
            'a' => Some("array".to_string()),
            _ => None,
        })
        .collect();
    let command = command.to_str().to_string();
    if add {
        vm.add_var_trace(&name.to_str(), ops, command);
    } else {
        vm.remove_var_trace(&name.to_str(), &ops, &command);
    }
    ok(Value::empty())
}
