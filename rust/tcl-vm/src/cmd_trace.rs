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
//! path never puts the old one back (`TclPtrSetVarIdx`, `tclVar.c`; issue
//! #1438). See `interp.rs::fire_var_traces`.
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
use tcl_dialect::model::surface_admits;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("trace", cmd_trace);
}

/// The `trace` option words the emulated release carries, in the registry's
/// declaration order — which is C's `traceOptions[]` order, so the `bad
/// option` / `ambiguous option` enumeration matches byte for byte. The three
/// legacy forms are gated to `SpecSurface::TCL8X`, so 9.0+ sees only
/// `add`/`info`/`remove` (C drops them behind `TCL_REMOVE_OBSOLETE_TRACES`).
fn visible_options(vm: &Vm) -> Vec<&'static str> {
    // The emulated release's name resolves through the one ingress seam;
    // the option table is gated on the resolved environment's document
    // authoring mask (ledger row B1), which is the mask the retired
    // `by_name(name).surface_query()` read handed back.
    let dialect = Some(crate::environment::surface_point_for_dialect(
        vm.runtime_version().dialect_profile_name(),
    ));
    let registry = tcl_registry::default_registry();
    let Some(spec) = registry.get_for_surface("trace", dialect) else {
        return Vec::new();
    };
    spec.subcommands
        .iter()
        .filter(|sub| {
            sub.surface
                .or(spec.surface)
                .is_none_or(|gate| surface_admits(gate, dialect.as_ref()))
        })
        .map(|sub| sub.name)
        .collect()
}

fn cmd_trace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"trace option ?arg ...?\"");
    };
    let options = visible_options(vm);
    let option = match core_trace::resolve_option(&sub.to_str(), &options) {
        Ok(o) => o,
        Err(e) => return err(e.into_message()),
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
        // A registry-declared option this engine has no arm for. Reporting it
        // as unknown keeps a data-only spec edit (a new subcommand or alias)
        // from turning into a panic in a shipped interpreter.
        _ => err(format!(
            "bad option \"{}\": must be {}",
            sub.to_str(),
            tcl_cmd_core::prefix::choice_list(&options)
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
                if let Err(e) = vm.ensure_trace_variable(&name.to_str()) {
                    return e;
                }
                vm.add_var_trace(&name.to_str(), ops, command.to_str().to_string(), false);
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
                vm.remove_cmd_trace(execution, &name.to_str(), &ops, &command.to_str())
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
        core_trace::TraceKind::Command | core_trace::TraceKind::Execution => {
            vm.cmd_trace_entries(kind == core_trace::TraceKind::Execution, &name.to_str())
        }
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

/// Legacy `trace vinfo name` → the variable's `{letters command}` pairs, with
/// the operations rendered as the `rwua` letter string C's `TRACE_OLD_VINFO`
/// arm builds (not the word list `trace info variable` reports).
fn trace_vinfo(vm: &Vm, args: &[Value]) -> Completion<Value> {
    let [name] = args else {
        return err("wrong # args: should be \"trace vinfo name\"");
    };
    ok(Value::list(
        vm.var_trace_info(&name.to_str())
            .into_iter()
            .map(|(ops, cmd)| {
                Value::list(vec![
                    Value::string(core_trace::legacy_ops_letters(&ops)),
                    Value::string(cmd),
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
        return err(format!(
            "wrong # args: should be \"trace {form} name ops command\""
        ));
    };
    let ops: Vec<String> = match core_trace::parse_legacy_variable_ops(ops.to_str().as_bytes()) {
        Ok(o) => o.iter().map(|s| (*s).to_string()).collect(),
        Err(e) => return err(e.into_message()),
    };
    let command = command.to_str().to_string();
    if add {
        if let Err(e) = vm.ensure_trace_variable(&name.to_str()) {
            return e;
        }
        vm.add_var_trace(&name.to_str(), ops, command, true);
    } else {
        vm.remove_var_trace(&name.to_str(), &ops, &command);
    }
    ok(Value::empty())
}
