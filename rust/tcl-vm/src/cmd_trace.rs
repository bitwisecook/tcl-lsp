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
//! Tcl library — `tcltest` etc. — relies on), plus the deprecated 8.x
//! `trace variable|vdelete|vinfo` forms, which the registry retires at 9.0.
//! The firing engine lives in [`Vm`]: read traces fire before a read, write
//! traces after a write, and unset traces before removal (callback errors
//! ignored). A write callback's error fails the *command* but never un-stores
//! the value — C swaps the value in before calling the traces and its error
//! path never puts the old one back (`TclPtrSetVarIdx`, `tclVar.c`).
//! See `interp.rs::fire_var_traces`.
//!
//! Command traces (`rename`/`delete`) and execution traces (`enter`/`leave`/
//! `enterstep`/`leavestep`) fire too, all tclsh-pinned in
//! `tests/command_traces_e2e.rs`: names arrive fully qualified, an
//! enter-trace error aborts the command, a leave-trace error replaces its
//! result, rename/delete callback errors are ignored, traces follow a
//! `rename`, and redefinition fires the `delete` trace.

use tcl_cmd_core::trace as core_trace;
use tcl_runtime_api::Completion;

use crate::interp::{Vm, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register_stock_builtin("trace", cmd_trace);
}

fn cmd_trace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"trace option ?arg ...?\"",
        );
    };
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_string_protocol();
    let option = match core_trace::resolve_option_original(vm, sub, protocol) {
        Ok(o) => o,
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    match option {
        "add" => trace_add_remove(vm, "add", rest, true),
        "remove" => trace_add_remove(vm, "remove", rest, false),
        "info" => trace_info(vm, rest),
        // Legacy 8.x forms map onto the variable engine (C rewrites them into
        // `trace add|remove variable` with the letters expanded).
        "variable" => legacy_variable(vm, rest, true),
        "vdelete" => legacy_variable(vm, rest, false),
        "vinfo" => trace_vinfo(vm, rest),
        _ => unreachable!("selected trace declaration is exhaustive"),
    }
}

/// `trace add|remove TYPE name opList command` (the `sub` word is echoed in the
/// wrong-`#`-args message, as C's `Tcl_WrongNumArgs(interp, 3, objv, …)` does).
/// All three trace types take exactly `name opList command`; the type word is
/// resolved first (a bad type out-ranks wrong-`#`-args), then the arg count, then
/// the op list — matching `TraceVariableObjCmd`/`Command`/`Execution`'s order.
fn trace_add_remove(vm: &mut Vm, sub: &str, rest: &[Value], add: bool) -> Completion<Value> {
    let Some((kindw, args)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            format!("wrong # args: should be \"trace {sub} type ?arg ...?\""),
        );
    };
    // Tcl resolves the type word with `Tcl_GetIndexFromObj`, so an
    // unambiguous prefix (`var` → `variable`) is accepted (set-2.4 / set-4.4).
    let kind = match core_trace::resolve_type_original(vm, kindw) {
        Ok(k) => k,
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    let [name, ops, command] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            format!(
                "wrong # args: should be \"trace {sub} {} name opList command\"",
                kindw.to_str()
            ),
        );
    };
    // Validate the op list against the type's table (`bad operation …`).
    let ops: Vec<String> = match core_trace::parse_ops_original(vm, ops, kind) {
        Ok(o) => o.iter().map(|s| (*s).to_string()).collect(),
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    match kind {
        core_trace::TraceKind::Variable => {
            let name = match vm.native_name_operand_bytes(name) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return vm.refuse_host_command(format!(
                        "variable trace name is unavailable: {error}"
                    ));
                }
            };
            if add {
                if let Err(e) = vm.ensure_trace_variable_bytes(&name) {
                    return e;
                }
                vm.add_var_trace_bytes(&name, ops, command.clone(), false);
            } else {
                let prefix = match vm.native_name_operand_bytes(command) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        return vm.refuse_host_command(format!(
                            "variable trace prefix is unavailable: {error}"
                        ));
                    }
                };
                vm.remove_var_trace_bytes(&name, &ops, &prefix);
            }
            ok(Value::empty())
        }
        core_trace::TraceKind::Command | core_trace::TraceKind::Execution => {
            let execution = kind == core_trace::TraceKind::Execution;
            if add {
                vm.add_cmd_trace(execution, &name.to_str(), ops, command.to_str().to_string())
            } else {
                vm.remove_cmd_trace(execution, &name.to_str(), &ops, &command.to_str())
            }
        }
    }
}

fn trace_info(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((kindw, args)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"trace info type name\"",
        );
    };
    let kind = match core_trace::resolve_type_original(vm, kindw) {
        Ok(k) => k,
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    let [name] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            format!(
                "wrong # args: should be \"trace info {} name\"",
                kindw.to_str()
            ),
        );
    };
    match kind {
        core_trace::TraceKind::Variable => {
            let name = match vm.native_name_operand_bytes(name) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return vm.refuse_host_command(format!(
                        "variable trace name is unavailable: {error}"
                    ));
                }
            };
            ok(var_trace_entries(vm, &name))
        }
        core_trace::TraceKind::Command | core_trace::TraceKind::Execution => {
            vm.cmd_trace_entries(kind == core_trace::TraceKind::Execution, &name.to_str())
        }
    }
}

/// The `{ops command}` pairs registered on variable `name` (newest first), as the
/// `trace info variable` result list.
fn var_trace_entries(vm: &Vm, name: &[u8]) -> Value {
    Value::list(
        vm.var_trace_info_bytes(name)
            .into_iter()
            .map(|(ops, cmd)| {
                Value::list(vec![
                    Value::list(ops.into_iter().map(Value::string).collect()),
                    cmd,
                ])
            })
            .collect(),
    )
}

/// Legacy `trace vinfo name` → the variable's `{letters command}` pairs, with
/// the operations rendered as the `rwua` letter string C's `TRACE_OLD_VINFO`
/// arm builds (not the word list `trace info variable` reports).
fn trace_vinfo(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [name] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"trace vinfo name\"",
        );
    };
    let name = match vm.native_name_operand_bytes(name) {
        Ok(bytes) => bytes,
        Err(error) => {
            return vm.refuse_host_command(format!("variable trace name is unavailable: {error}"));
        }
    };
    ok(Value::list(
        vm.var_trace_info_bytes(&name)
            .into_iter()
            .map(|(ops, cmd)| {
                Value::list(vec![
                    Value::string(core_trace::legacy_ops_letters(&ops)),
                    cmd,
                ])
            })
            .collect(),
    ))
}

/// Legacy `trace variable name ops command` / `trace vdelete name ops command`.
/// The op word is a concatenation of the letters `r`/`w`/`u`/`a`; the shared
/// parser expands and validates it, so a non-`rwua` byte is C's `bad
/// operations "…": should be one or more of rwua` rather than a
/// silently-installed never-firing trace, and the stored set is the same
/// canonical set `trace add variable` produces (so a `vdelete` matches an
/// `add`-installed trace and vice versa).
fn legacy_variable(vm: &mut Vm, args: &[Value], add: bool) -> Completion<Value> {
    let form = if add { "variable" } else { "vdelete" };
    let [name, ops, command] = args else {
        return crate::command::native_wrong_arguments_message(
            vm,
            format!("wrong # args: should be \"trace {form} name ops command\""),
        );
    };
    let ops: Vec<String> = match core_trace::parse_legacy_variable_ops(ops.to_str().as_bytes()) {
        Ok(o) => o.iter().map(|s| (*s).to_string()).collect(),
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    let name = match vm.native_name_operand_bytes(name) {
        Ok(bytes) => bytes,
        Err(error) => {
            return vm.refuse_host_command(format!("variable trace name is unavailable: {error}"));
        }
    };
    if add {
        if let Err(e) = vm.ensure_trace_variable_bytes(&name) {
            return e;
        }
        vm.add_var_trace_bytes(&name, ops, command.clone(), true);
    } else {
        let prefix = match vm.native_name_operand_bytes(command) {
            Ok(bytes) => bytes,
            Err(error) => {
                return vm
                    .refuse_host_command(format!("variable trace prefix is unavailable: {error}"));
            }
        };
        vm.remove_var_trace_bytes(&name, &ops, &prefix);
    }
    ok(Value::empty())
}
