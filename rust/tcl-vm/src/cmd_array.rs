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

//! The `array` ensemble builtin — a thin adapter over the shared
//! [`tcl_cmd_core::array`] core. The read-side (`exists`/`size`/`names`/`get`) and
//! `unset` are shared over the VM's `VarStore`/`Frames`/`ValueOps`; `set` (whose
//! per-element write traces must fail the command) stays here. An unshared
//! `array unset a` (no pattern) that iterates and unsets elements would leave
//! an empty array instead of removing the whole array.

use tcl_registry::{ArgRole, InvocationWord, InvocationWords};
use tcl_runtime_api::completion_options::{
    self as shared_options, ControlOptionPolicy, OptionValue,
};
use tcl_runtime_api::{ArrayTarget, Code, Completion, VarStore};

use crate::command::{completion_from_cmd_error, settle_control_options};
use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    let Some(namespace) = vm
        .native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Array)
    else {
        vm.register_stock_builtin("array", cmd_array);
        return;
    };
    let subs = crate::environment::release_subcommands(
        vm.runtime_version().dialect_profile_name(),
        "array",
        ARRAY_SUBS,
    );
    vm.register_stock_namespace_ensemble("array", namespace, ARRAY_MEMBERS, subs);
}

/// Repinning bootstrap replaces stock tokens while preserving user replacements.
pub(crate) fn refresh_profile(vm: &mut Vm) {
    if vm.stock_native_identity("array").as_deref() != Some("array") {
        return;
    }
    for &(member, _) in ARRAY_MEMBERS {
        let target = format!("::tcl::array::{member}");
        if vm.stock_native_identity(&target).as_deref() == target.strip_prefix("::") {
            vm.remove_registered_command(target.trim_start_matches("::"));
        }
    }
    register(vm);
    if vm
        .native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Array)
        .is_none()
    {
        vm.retire_unused_stock_ensemble_namespace("array");
    }
}

macro_rules! array_members {
    ($($function:ident => $member:literal),+ $(,)?) => {
        const ARRAY_MEMBERS: &[(&str, crate::command::BuiltinFn)] = &[
            $(($member, $function)),+
        ];
        $(fn $function(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            array_op(vm, $member, args)
        })+
    };
}

array_members! {
    array_anymore => "anymore", array_donesearch => "donesearch",
    array_nextelement => "nextelement", array_startsearch => "startsearch",
    array_default_member => "default",
    array_exists => "exists", array_for_member => "for", array_get => "get",
    array_names => "names", array_set => "set", array_size => "size",
    array_unset => "unset",
}

/// `array`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it.
/// C's table also carries `anymore`, `donesearch`, `nextelement`,
/// `startsearch`, and `statistics`. The dispatched members are filtered by
/// the selected release; `default` and `for` require Tcl 9.
///
/// tclsh 9.0.4, for contrast:
///   array x a -> unknown or ambiguous subcommand "x": must be anymore,
///                default, donesearch, exists, for, get, names, nextelement,
///                set, size, startsearch, statistics, or unset
const ARRAY_SUBS: &[&str] = &[
    "anymore",
    "default",
    "donesearch",
    "exists",
    "for",
    "get",
    "names",
    "nextelement",
    "set",
    "size",
    "startsearch",
    "unset",
];

/// `array option arrayName ?arg ...?` — dispatch to the subcommand handler.
fn cmd_array(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"array subcommand ?arg ...?\"",
        );
    };
    let word = match vm.native_name_operand_bytes(sub) {
        Ok(word) => word,
        Err(error) => {
            return vm
                .refuse_host_command(format!("native array subcommand is unavailable: {error:?}"));
        }
    };
    // `array` is a `TclMakeEnsemble` command: exact match, else a unique
    // prefix, so `array e a` is `array exists a`.
    // `for` and `default` are Tcl 9,
    // so under an earlier pin it must neither run nor claim the prefix `f`.
    let subs = crate::environment::release_subcommands(
        vm.command_surface_profile().name,
        "array",
        ARRAY_SUBS,
    );
    let Some(index) = tcl_cmd_core::ensemble::resolve_subcommand(subs, word.as_ref(), true) else {
        return err(tcl_cmd_core::ensemble::unknown_subcommand_message(
            subs,
            word.as_ref(),
            true,
            b"::tcl::array",
        ));
    };
    if subs[index] == "default" {
        return array_default_command(vm, rest, Some(sub));
    }
    array_op(vm, subs[index], rest)
}

fn array_op(vm: &mut Vm, sub: &str, rest: &[Value]) -> Completion<Value> {
    if sub == "default" {
        return array_default_command(vm, rest, None);
    }
    // `LocateArray` is the one semantic entry for every array subcommand,
    // including the compiler-lowered `::tcl::array::*` commands above. Derive
    // the target from the dialect-selected registry member: it is the unique
    // VarRead/VarWrite argument, not a second command-name/index table.
    let trace_target = array_trace_target(vm, sub, rest);
    if sub == "for" {
        return array_for(vm, rest, trace_target.as_ref());
    }
    if let Some(name) = trace_target {
        let name = match vm.native_name_operand_bytes(&name) {
            Ok(name) => name,
            Err(error) => {
                return vm
                    .refuse_host_command(format!("native array name is unavailable: {error:?}"));
            }
        };
        return vm.with_array_trace_target_bytes(&name, |vm, target| {
            array_op_after_trace(vm, sub, rest, Some(target))
        });
    }
    array_op_after_trace(vm, sub, rest, None)
}

fn array_default_usage(
    vm: &mut Vm,
    head: &Value,
    member: Option<&Value>,
    option: Option<&Value>,
    suffix: &[u8],
) -> Completion<Value> {
    let mut header = vec![head.clone()];
    header.extend(member.cloned());
    header.extend(option.cloned());
    let header = tcl_cmd_core::ensemble::rewrite_argument_usage(
        &header,
        &vm.native_invocation.usage_rewrites,
    );
    let usage = match vm.native_argument_usage_header(&header) {
        Ok(usage) => usage,
        Err(refusal) => return refusal,
    };
    let mut message = b"wrong # args: should be \"".to_vec();
    message.extend_from_slice(&usage);
    message.push(b' ');
    message.extend_from_slice(suffix);
    message.push(b'"');
    crate::command::native_wrong_arguments_message(vm, message)
}

fn array_default_command(vm: &mut Vm, args: &[Value], member: Option<&Value>) -> Completion<Value> {
    let Some(head) = vm.invoked_name_value() else {
        return vm.refuse_host_command("array default original invocation is unavailable".into());
    };
    let Some(protocol) = vm
        .actual_native_invocation_dialect()
        .native_array_default_protocol()
    else {
        return vm
            .refuse_host_command("native array default command protocol is unavailable".into());
    };
    if !(2..=3).contains(&args.len()) {
        return array_default_usage(vm, &head, member, None, b"option arrayName ?value?");
    }
    let option = match tcl_cmd_core::array::prepare_default_option_original(vm, protocol, &args[0])
    {
        Ok(option) => option,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let name = match tcl_syntax::value::ValueOps::native_string_bytes(vm, &args[1]) {
        Ok(name) => name,
        Err(error) => return completion_from_cmd_error(vm, error.into()),
    };
    vm.with_array_trace_target_bytes(&name, |vm, target| {
        match tcl_cmd_core::array::default_at(vm, protocol, option, &args[1..], Some(target)) {
            Ok(tcl_cmd_core::array::ArrayDefaultCommandResult::Value(value)) => ok(value),
            Ok(tcl_cmd_core::array::ArrayDefaultCommandResult::WrongArguments { suffix }) => {
                array_default_usage(vm, &head, member, Some(&args[0]), suffix)
            }
            Err(error) => completion_from_cmd_error(vm, error),
        }
    })
}

fn settle_array_read_result(
    vm: &Vm,
    result: tcl_cmd_core::array::ArrayCommandResult<Value>,
) -> Completion<Value> {
    let Some(miss) = result.read_miss else {
        return ok(result.value);
    };
    let carried =
        shared_options::retained_array_read_options(&miss, |bytes| Value::from_string_bytes(bytes));
    let rows = shared_options::plan(vm.runtime_version(), Code::Ok, 0, &carried, None);
    let options = Value::list(
        rows.into_iter()
            .flat_map(|(key, value)| {
                let value = match value {
                    OptionValue::Integer(value) => Value::int(value),
                    OptionValue::Value(value) => value,
                };
                [Value::from_string_bytes(key), value]
            })
            .collect(),
    );
    Completion::new(Code::Ok, result.value, options)
}

fn array_op_after_trace(
    vm: &mut Vm,
    sub: &str,
    rest: &[Value],
    target: Option<&ArrayTarget>,
) -> Completion<Value> {
    if let Some(result) =
        tcl_cmd_core::native_array_search::dispatch_bytes(vm, sub.as_bytes(), rest, target)
    {
        return match result {
            Ok(value) => ok(value),
            Err(error) => completion_from_cmd_error(vm, error),
        };
    }
    // The read-side + `unset` live in the shared core.
    if let Some(result) = tcl_cmd_core::array::dispatch_bytes_at(vm, sub.as_bytes(), rest, target) {
        return match result {
            Ok(result) => settle_array_read_result(vm, result),
            Err(e) => completion_from_cmd_error(vm, e),
        };
    }
    // Per-runtime: `array set` (its per-element write traces must fail the
    // command), `array for` (iterates a body), and the unknown-subcommand message.
    match sub {
        "set" => match rest {
            [n, list] => array_set_after_trace(vm, n, list),
            _ => crate::command::native_wrong_arguments_message(
                vm,
                "wrong # args: should be \"array set arrayName list\"",
            ),
        },
        // Unreachable from `cmd_array` (every `ARRAY_SUBS` name is handled
        // above or by the shared core); the registered `::tcl::array::*`
        // entry points pass canonical names.
        other => err(tcl_cmd_core::ensemble::unknown_subcommand_message(
            ARRAY_SUBS,
            other.as_bytes(),
            true,
            b"::tcl::array",
        )),
    }
}

fn array_set_after_trace(vm: &mut Vm, n: &Value, list: &Value) -> Completion<Value> {
    // C's `Tcl_ArrayObjCmd` set path resolves the target through
    // the standard variable lookup *before* it looks at the list,
    // and that lookup parses the name: an element-form name yields
    // a scalar element cell, never an array, so the command refuses
    // it. The check therefore precedes both the list
    // parse and the even-length test.
    //
    // Oracle, identical on tclsh 8.4.20 / 8.5.19 / 8.6.14 / 9.0.4 /
    // 9.1 (`catch` result : message):
    //
    //   array set (x) {a 1}     -> 1:can't set "(x)": variable isn't array
    //   array set (x) {}        -> 1:can't set "(x)": variable isn't array
    //   array set (x) {a}       -> 1:can't set "(x)": variable isn't array
    //   array set (x) "a \{b"   -> 1:can't set "(x)": variable isn't array
    //   array set arr(k) {a 1}  -> 1:can't set "arr(k)": variable isn't array
    //   array set {arr(k)} {a 1}-> 1:can't set "arr(k)": variable isn't array
    //   array set okarr {a}     -> 1:list must have an even number of elements
    //   array set {a)b} {a 1}   -> 0:            (a `)` with no `(` is a name)
    //   array set {a(b} {a 1}   -> 0:            (an unclosed `(` is a name)
    //
    // 8.6+ also carry `errorCode` `TCL LOOKUP VARNAME <name>` (8.4 /
    // 8.5: `NONE`); the VM's shared `TCL LOOKUP VARNAME` spelling
    // omits the trailing name element here as it does at its
    // sibling site (`missing_parent_ns`).
    let name = match vm.native_name_operand_bytes(n) {
        Ok(name) => name,
        Err(error) => {
            return vm.refuse_host_command(format!("native array name is unavailable: {error:?}"));
        }
    };
    let target =
        match VarStore::array_target_bytes(vm, tcl_runtime_api::FrameId(vm.current_level()), &name)
        {
            Ok(target) => target,
            Err(error) => return completion_from_cmd_error(vm, error.into()),
        };
    if let Err(error) = VarStore::array_key_bytes_checked_at(vm, &target) {
        return completion_from_cmd_error(vm, error.into());
    }
    let Some(policy) = vm.name_policy_protocol() else {
        return vm.refuse_host_command("native array name policy is unavailable".into());
    };
    if !vm.dictionary_variable_containers()
        && policy
            .recipe()
            .combined_variable_input(&name)
            .element()
            .is_some()
    {
        return match vm.ensure_array_bytes(&name) {
            Err(error) => error,
            Ok(()) => vm.refuse_host_command("array element cannot own an array".into()),
        };
    }
    let items = match tcl_syntax::value::ValueOps::list_elements(vm, list) {
        Ok(i) => i,
        Err(e) => return completion_from_cmd_error(vm, e.into()),
    };
    if items.len() % 2 != 0 {
        return completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::argument_format("list must have an even number of elements"),
        );
    }
    if items.is_empty() {
        // `array set a {}` still materialises an empty array; onto an
        // existing scalar it errors. C words *this* case as the
        // command (`can't array set "a"`), distinct from the
        // per-element `set` message taken on a non-empty list.
        if let Err(e) = vm.ensure_array_bytes(&name) {
            return e;
        }
    } else {
        // Write element by element so a scalar target fails at the
        // *element* write — naming `a(key)`, as C's `TclArraySet`
        // does — rather than pre-checked under the bare name.
        let mut i = 0;
        while i + 1 < items.len() {
            let key = match vm.native_name_operand_bytes(&items[i]) {
                Ok(key) => key,
                Err(error) => {
                    return vm.refuse_host_command(format!(
                        "native array key is unavailable: {error:?}"
                    ));
                }
            };
            if let Err(e) = vm.set_array_elem_bytes(&name, &key, items[i + 1].clone()) {
                return e;
            }
            i += 2;
        }
    }
    ok(Value::empty())
}

fn array_trace_target(vm: &Vm, sub: &str, rest: &[Value]) -> Option<Value> {
    let profile = vm.command_surface_profile();
    let registry = crate::environment::store_for_profile(profile);
    let words: Vec<InvocationWord<'_>> = std::iter::once(InvocationWord::Literal(sub))
        .chain(rest.iter().map(|_| InvocationWord::Dynamic))
        .collect();
    let resolved = registry
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("array"), &words),
            Some(crate::environment::surface_point(profile)),
        )
        .resolved()?;
    if resolved.subcommand.resolved()?.canonical_name != sub {
        return None;
    }
    let facts = resolved.facts();
    let index = facts
        .sole_argument_index_for_roles(words.len(), &[ArgRole::VarRead, ArgRole::VarWrite])?
        .checked_sub(1)?;
    rest.get(index).cloned()
}

/// `array for {keyVar valueVar} arrayName script` — iterate the array's elements,
/// binding the two vars and running the body once per pair (mirrors `dict for`;
/// `break`/`continue` apply, an error/return propagates). The element set is
/// snapshotted up front so body mutations don't perturb the walk.
fn array_for(vm: &mut Vm, rest: &[Value], trace_target: Option<&Value>) -> Completion<Value> {
    let [vars, arrname, body] = rest else {
        return crate::command::native_wrong_args(vm, "array for {key value} arrayName script");
    };
    let vnames = match tcl_syntax::value::ValueOps::list_elements(vm, vars) {
        Ok(names) => names,
        Err(error) => return completion_from_cmd_error(vm, error.into()),
    };
    let [kvar, vvar] = vnames.as_slice() else {
        return err("must have two variable names");
    };
    let kvar = match vm.native_name_operand_bytes(kvar) {
        Ok(name) => name,
        Err(error) => {
            return vm
                .refuse_host_command(format!("native loop variable is unavailable: {error:?}"));
        }
    };
    let vvar = match vm.native_name_operand_bytes(vvar) {
        Ok(name) => name,
        Err(error) => {
            return vm
                .refuse_host_command(format!("native loop variable is unavailable: {error:?}"));
        }
    };
    let name = match vm.native_name_operand_bytes(trace_target.unwrap_or(arrname)) {
        Ok(name) => name,
        Err(error) => {
            return vm.refuse_host_command(format!("native array name is unavailable: {error:?}"));
        }
    };
    vm.with_array_trace_target_bytes(&name, |vm, target| {
        array_for_after_trace(vm, &kvar, &vvar, &name, body, target)
    })
}

fn array_for_after_trace(
    vm: &mut Vm,
    kvar: &[u8],
    vvar: &[u8],
    name: &[u8],
    body: &Value,
    target: &ArrayTarget,
) -> Completion<Value> {
    let keys = match VarStore::array_search_key_bytes_at(vm, target) {
        Ok(Some(keys)) => keys,
        Ok(None) => {
            let mut message = b"\"".to_vec();
            message.extend_from_slice(name);
            message.extend_from_slice(b"\" isn't an array");
            let code = Value::list(vec![
                Value::string("TCL"),
                Value::string("LOOKUP"),
                Value::string("ARRAY"),
                Value::from_string_bytes(name),
            ]);
            return crate::command::err_with_code(message, code.string_bytes());
        }
        Err(error) => return completion_from_cmd_error(vm, error.into()),
    };
    let revision = VarStore::array_revision_at(vm, target);
    for key in &keys {
        match same_array_target(vm, name, target, revision) {
            Ok(true) => {}
            Ok(false) => return array_for_changed(),
            Err(error) => return completion_from_cmd_error(vm, error.into()),
        }
        match VarStore::array_elem_exists_bytes_at(vm, target, key) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(error) => return completion_from_cmd_error(vm, error.into()),
        }
        let value = match vm.read_elem_traced_bytes(name, key) {
            Ok(value) => value,
            Err(error) => {
                if vm.refused_completion().is_some() {
                    return error;
                }
                vm.publish_swallowed_trace_error();
                None
            }
        };
        if let Err(error) = vm.set_var_bytes(kvar, Value::from_string_bytes(key.clone())) {
            return error;
        }
        if let Some(value) = value
            && let Err(error) = vm.set_var_bytes(vvar, value)
        {
            return error;
        }
        let completion = vm.eval_value_at_level(vm.current_level(), body);
        match completion.code {
            Code::Ok | Code::Continue => {}
            Code::Break => break,
            _ => return completion,
        }
        match same_array_target(vm, name, target, revision) {
            Ok(true) => {}
            Ok(false) => return array_for_changed(),
            Err(error) => return completion_from_cmd_error(vm, error.into()),
        }
    }
    settle_control_options(vm, ok(Value::empty()), ControlOptionPolicy::FRESH_SETTLED)
}

fn same_array_target(
    vm: &Vm,
    name: &[u8],
    original: &ArrayTarget,
    revision: Option<u64>,
) -> Result<bool, tcl_syntax::value::ValueError> {
    let current = VarStore::array_target_bytes(vm, tcl_runtime_api::Frames::current(vm), name)?;
    Ok(current.cell_id() == original.cell_id()
        && VarStore::array_key_bytes_checked_at(vm, original)?.is_some()
        && revision
            .is_none_or(|expected| VarStore::array_revision_at(vm, original) == Some(expected)))
}

fn array_for_changed() -> Completion<Value> {
    crate::command::err_with_code("array changed during iteration", "TCL READ array for")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_array_unset_preserves_constant_error_identity() {
        let mut vm = Vm::new();
        vm.ensure_array("a").expect("array");
        vm.mark_constant("a");

        let completion = array_op_after_trace(&mut vm, "unset", &[Value::string("a")], None);

        assert_eq!(completion.code, Code::Error);
        assert_eq!(
            completion.result.to_str().as_ref(),
            r#"can't unset "a": variable is a constant"#
        );
        assert_eq!(
            crate::command::resolved_error_code(&mut vm, &completion)
                .unwrap()
                .to_str()
                .as_ref(),
            "TCL UNSET CONST"
        );
        assert!(VarStore::array_keys(&vm, tcl_runtime_api::Frames::current(&vm), "a").is_some());
    }
}

#[cfg(test)]
#[path = "cmd_array/native_unset_tests.rs"]
mod native_unset_tests;
