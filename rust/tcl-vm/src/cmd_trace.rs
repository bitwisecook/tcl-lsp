//! The `trace` command — variable traces.
//!
//! Supports `trace add|remove|info variable name ops command` (the surface the
//! Tcl library — `tcltest` etc. — relies on). The firing engine lives in
//! [`Vm`]: read traces fire before a read, write traces after a write (with the
//! old value restored if the callback aborts the write), and unset traces
//! before removal (callback errors ignored). See `interp.rs::fire_var_traces`.
//!
//! Command and execution traces are accepted but not yet fired.

use tcl_cmd_core::trace as core_trace;
use tcl_runtime_api::Completion;

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

/// Parse + validate a variable-trace ops word (`{read write}`) into op names, via
/// the shared core (this also gained the missing op validation — the VM used to
/// accept `trace add variable v bogus cmd`).
fn parse_ops(spec: &str) -> Result<Vec<String>, Completion<Value>> {
    core_trace::parse_ops(spec.as_bytes(), core_trace::TraceKind::Variable)
        .map(|ops| ops.iter().map(|o| (*o).to_string()).collect())
        .map_err(|e| err(e.into_message()))
}

fn trace_add(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((kind, args)) = rest.split_first() else {
        return err("wrong # args: should be \"trace add type ?arg ...?\"");
    };
    // Tcl resolves the type word with `Tcl_GetIndexFromObj`, so an
    // unambiguous prefix (`var` → `variable`) is accepted (set-2.4 / set-4.4).
    let kind = match core_trace::resolve_type(&kind.to_str()) {
        Ok(k) => k,
        Err(e) => return err(e.into_message()),
    };
    match kind {
        core_trace::TraceKind::Variable => match args {
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
        core_trace::TraceKind::Command | core_trace::TraceKind::Execution => ok(Value::empty()),
    }
}

fn trace_remove(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((kind, args)) = rest.split_first() else {
        return err("wrong # args: should be \"trace remove type ?arg ...?\"");
    };
    let kind = match core_trace::resolve_type(&kind.to_str()) {
        Ok(k) => k,
        Err(e) => return err(e.into_message()),
    };
    match kind {
        core_trace::TraceKind::Variable => match args {
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
        core_trace::TraceKind::Command | core_trace::TraceKind::Execution => ok(Value::empty()),
    }
}

fn trace_info(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((kind, args)) = rest.split_first() else {
        return err("wrong # args: should be \"trace info type name\"");
    };
    let kind = match core_trace::resolve_type(&kind.to_str()) {
        Ok(k) => k,
        Err(e) => return err(e.into_message()),
    };
    match kind {
        core_trace::TraceKind::Variable => trace_info_variable(vm, args),
        core_trace::TraceKind::Command | core_trace::TraceKind::Execution => {
            ok(Value::list(Vec::new()))
        }
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
