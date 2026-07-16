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

//! The `trace` command — variable, command, and execution traces.
//!
//! Supports `trace add|remove|info variable name ops command` (the surface the
//! Tcl library — `tcltest` etc. — relies on). The firing engine lives in
//! [`Vm`]: read traces fire before a read, write traces after a write (with the
//! old value restored if the callback aborts the write), and unset traces
//! before removal (callback errors ignored). See `interp.rs::fire_var_traces`.
//!
//! Command traces (`rename`/`delete`) and execution traces (`enter`/`leave`/
//! `enterstep`/`leavestep`) fire too (M16.3), all tclsh-pinned in
//! `tests/command_traces_e2e.rs`: names arrive fully qualified, an
//! enter-trace error aborts the command, a leave-trace error replaces its
//! result, rename/delete callback errors are ignored, traces follow a
//! `rename`, and redefinition fires the `delete` trace.

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
        "add" => trace_add_remove(vm, "add", rest, true),
        "remove" => trace_add_remove(vm, "remove", rest, false),
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

/// `trace add|remove TYPE name opList command` (the `sub` word is echoed in the
/// wrong-`#`-args message, as C's `Tcl_WrongNumArgs(interp, 3, objv, …)` does).
/// All three trace types take exactly `name opList command`; the type word is
/// resolved first (a bad type out-ranks wrong-`#`-args), then the arg count, then
/// the op list — matching `TraceVariableObjCmd`/`Command`/`Execution`'s order.
fn trace_add_remove(vm: &mut Vm, sub: &str, rest: &[Value], add: bool) -> Completion<Value> {
    let Some((kindw, args)) = rest.split_first() else {
        return err(format!(
            "wrong # args: should be \"trace {sub} type ?arg ...?\""
        ));
    };
    // Tcl resolves the type word with `Tcl_GetIndexFromObj`, so an
    // unambiguous prefix (`var` → `variable`) is accepted (set-2.4 / set-4.4).
    let typeword = kindw.to_str();
    let kind = match core_trace::resolve_type(&typeword) {
        Ok(k) => k,
        Err(e) => return err(e.into_message()),
    };
    let [name, ops, command] = args else {
        return err(format!(
            "wrong # args: should be \"trace {sub} {typeword} name opList command\""
        ));
    };
    // Validate the op list against the type's table (`bad operation …`).
    let ops: Vec<String> = match core_trace::parse_ops(ops.to_str().as_bytes(), kind) {
        Ok(o) => o.iter().map(|s| (*s).to_string()).collect(),
        Err(e) => return err(e.into_message()),
    };
    match kind {
        core_trace::TraceKind::Variable => {
            if add {
                vm.add_var_trace(&name.to_str(), ops, command.to_str().to_string());
            } else {
                vm.remove_var_trace(&name.to_str(), &ops, &command.to_str());
            }
            ok(Value::empty())
        }
        core_trace::TraceKind::Command | core_trace::TraceKind::Execution => {
            let execution = kind == core_trace::TraceKind::Execution;
            if add {
                vm.add_cmd_trace(execution, &name.to_str(), ops, command.to_str().to_string())
            } else {
                vm.remove_cmd_trace(execution, &name.to_str(), &ops, &command.to_str());
                ok(Value::empty())
            }
        }
    }
}

fn trace_info(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((kindw, args)) = rest.split_first() else {
        return err("wrong # args: should be \"trace info type name\"");
    };
    let typeword = kindw.to_str();
    let kind = match core_trace::resolve_type(&typeword) {
        Ok(k) => k,
        Err(e) => return err(e.into_message()),
    };
    let [name] = args else {
        return err(format!(
            "wrong # args: should be \"trace info {typeword} name\""
        ));
    };
    match kind {
        core_trace::TraceKind::Variable => ok(var_trace_entries(vm, &name.to_str())),
        core_trace::TraceKind::Command | core_trace::TraceKind::Execution => ok(
            vm.cmd_trace_entries(kind == core_trace::TraceKind::Execution, &name.to_str()),
        ),
    }
}

/// The `{ops command}` pairs registered on variable `name` (newest first), as the
/// `trace info variable` result list.
fn var_trace_entries(vm: &Vm, name: &str) -> Value {
    Value::list(
        vm.var_trace_info(name)
            .into_iter()
            .map(|(ops, cmd)| {
                Value::list(vec![
                    Value::list(ops.into_iter().map(Value::string).collect()),
                    Value::string(cmd),
                ])
            })
            .collect(),
    )
}

/// Legacy `trace vinfo name` → the variable's `{ops command}` pairs.
fn trace_info_variable(vm: &Vm, args: &[Value]) -> Completion<Value> {
    let [name] = args else {
        return err("wrong # args: should be \"trace info variable name\"");
    };
    ok(var_trace_entries(vm, &name.to_str()))
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
