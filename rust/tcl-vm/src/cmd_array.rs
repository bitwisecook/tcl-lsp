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
use crate::interp::native_operation_currency::{NativeOperationCurrency, NativeOperationScope};
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
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        let scope = match NativeOperationScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return completion_from_cmd_error(vm, error),
        };
        let result = cmd_array_entered(vm, args, scope.currency());
        match scope.check(vm) {
            Ok(()) => result,
            Err(error) => {
                drop(result);
                completion_from_cmd_error(vm, error)
            }
        }
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

fn cmd_array_entered(
    vm: &mut Vm,
    args: &[Value],
    currency: &NativeOperationCurrency,
) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"array subcommand ?arg ...?\"",
        );
    };
    let word = match original_bytes(vm, sub, currency) {
        Ok(word) => word,
        Err(error) => return error,
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
    let original_names =
        match crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
    let reached = (|| {
        let scope = match NativeOperationScope::enter(vm) {
            Ok(scope) => scope,
            Err(error) => return completion_from_cmd_error(vm, error),
        };
        let result = array_op_entered(vm, sub, rest, scope.currency());
        match scope.check(vm) {
            Ok(()) => result,
            Err(error) => {
                drop(result);
                completion_from_cmd_error(vm, error)
            }
        }
    })();
    if let Err(error) = original_names.finish(vm) {
        return crate::command::completion_from_cmd_error(vm, error);
    }
    reached
}

fn array_op_entered(
    vm: &mut Vm,
    sub: &str,
    rest: &[Value],
    currency: &NativeOperationCurrency,
) -> Completion<Value> {
    if sub == "default" {
        return array_default_command(vm, rest, None);
    }
    if sub == "set" {
        return array_set_original(vm, rest, currency);
    }
    // `LocateArray` is the one semantic entry for every array subcommand,
    // including the compiler-lowered `::tcl::array::*` commands above. Derive
    // the target from the dialect-selected registry member: it is the unique
    // VarRead/VarWrite argument, not a second command-name/index table.
    let trace_target = array_trace_target(vm, sub, rest);
    if sub == "for" {
        return array_for(vm, rest, trace_target, currency);
    }
    if let Some(name) = trace_target {
        let name = match original_bytes(vm, name, currency) {
            Ok(name) => name,
            Err(error) => return error,
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

fn original_bytes(
    vm: &mut Vm,
    original: &Value,
    currency: &NativeOperationCurrency,
) -> Result<std::rc::Rc<[u8]>, Completion<Value>> {
    let original_names = crate::interp::native_name_consumers::OriginalNameConsumerScope::enter(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let reached = (|| {
        currency
            .check(vm)
            .map_err(|error| completion_from_cmd_error(vm, error))?;
        original
            .check_native_header()
            .map_err(|error| completion_from_cmd_error(vm, error.into()))?;
        let reached = original_names.read(vm, original);
        currency
            .check(vm)
            .map_err(|error| completion_from_cmd_error(vm, error))?;
        reached.map_err(|error| completion_from_cmd_error(vm, error))
    })();
    original_names
        .finish(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    reached
}

fn original_list(
    vm: &mut Vm,
    original: &Value,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    currency: &NativeOperationCurrency,
) -> Result<crate::NativeListItems, Completion<Value>> {
    let scope = NativeOperationScope::current_entered(vm)
        .map_err(|error| {
            completion_from_cmd_error(vm, tcl_cmd_core::CmdError::from_execution_refusal(error))
        })?
        .ok_or_else(|| {
            completion_from_cmd_error(
                vm,
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original array List entry",
                )
                .into(),
            )
        })?;
    currency
        .check(vm)
        .map_err(|error| completion_from_cmd_error(vm, error))?;
    let reached = (|| {
        let ops =
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &scope)?;
        ops.list(original, protocol)
    })();
    currency
        .check(vm)
        .map_err(|error| completion_from_cmd_error(vm, error))?;
    reached.map_err(|error| completion_from_cmd_error(vm, error))
}

fn array_set_original(
    vm: &mut Vm,
    rest: &[Value],
    currency: &NativeOperationCurrency,
) -> Completion<Value> {
    let [name, list] = rest else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"array set arrayName list\"",
        );
    };
    let dialect = vm.actual_native_invocation_dialect();
    let (Some(selected), Some(strings)) = (
        dialect.native_array_set_protocol(),
        dialect.native_string_protocol(),
    ) else {
        return completion_from_cmd_error(
            vm,
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("original array set worker")
                .into(),
        );
    };
    match selected {
        tcl_registry::native_array::NativeArraySetProtocol::COriginalName => {
            let bytes = match original_bytes(vm, name, currency) {
                Ok(bytes) => bytes,
                Err(error) => return error,
            };
            vm.with_array_trace_target_bytes(&bytes, |vm, target| {
                array_set_c_after_trace(vm, name, &bytes, list, strings, target, currency)
            })
        }
        tcl_registry::native_array::NativeArraySetProtocol::JimOriginalVariable => {
            array_set_jim(vm, name, list, strings, currency)
        }
    }
}

fn array_set_c_after_trace(
    vm: &mut Vm,
    original_name: &Value,
    name: &[u8],
    list: &Value,
    strings: tcl_syntax::native_string::NativeStringProtocol,
    target: &ArrayTarget,
    currency: &NativeOperationCurrency,
) -> Completion<Value> {
    if let Err(error) = VarStore::array_key_bytes_checked_at(vm, target) {
        return completion_from_cmd_error(vm, error.into());
    }
    let Some(policy) = vm.name_policy_protocol() else {
        return completion_from_cmd_error(
            vm,
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("original array name policy")
                .into(),
        );
    };
    if policy
        .recipe()
        .combined_variable_input(name)
        .element()
        .is_some()
    {
        return match vm.ensure_array_bytes(name) {
            Err(error) => error,
            Ok(()) => completion_from_cmd_error(
                vm,
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "array element cannot own an array",
                )
                .into(),
            ),
        };
    }
    let items = match original_list(vm, list, strings, currency) {
        Ok(items) => items,
        Err(error) => return error,
    };
    if items.len() % 2 != 0 {
        return completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::argument_format("list must have an even number of elements"),
        );
    }
    if items.is_empty() {
        if let Err(error) = vm.ensure_array_bytes(name) {
            return error;
        }
    } else {
        for pair in items.as_chunks::<2>().0 {
            if let Err(error) = items.check_generation() {
                return completion_from_cmd_error(vm, error.into());
            }
            let reached = vm.store_original_c_array_member(original_name, &pair[0], &pair[1]);
            if let Err(error) = currency.check(vm) {
                drop(reached);
                return completion_from_cmd_error(vm, error);
            }
            if let Err(error) = reached {
                return error;
            }
        }
    }
    ok(Value::empty())
}

fn original_compound_ops<T>(
    vm: &mut Vm,
    currency: &NativeOperationCurrency,
    operation: impl FnOnce(
        &crate::interp::native_numeric_ops::OriginalNumericOps<'_>,
    ) -> Result<T, tcl_cmd_core::CmdError>,
) -> Result<T, tcl_cmd_core::CmdError> {
    currency.check(vm)?;
    let scope = NativeOperationScope::enter(vm)?;
    let reached = {
        let ops =
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &scope)?;
        operation(&ops)
    };
    if let Err(first) = scope.check(vm) {
        drop(reached);
        scope.check(vm)?;
        return Err(first);
    }
    reached
}

fn array_set_jim(
    vm: &mut Vm,
    name: &Value,
    list: &Value,
    strings: tcl_syntax::native_string::NativeStringProtocol,
    currency: &NativeOperationCurrency,
) -> Completion<Value> {
    let items = match original_list(vm, list, strings, currency) {
        Ok(items) => items,
        Err(error) => return error,
    };
    if items.len() % 2 != 0 {
        return completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::argument_format("list must have an even number of elements"),
        );
    }
    let result: Result<(), Completion<Value>> = (|| {
        let original = vm.original_jim_array_root(name, currency)?;
        let Some(original) = original else {
            // Jim's absent-target branch adopts the very same original List;
            // no key operand is read before this variable publication.
            list.check_native_header()
                .map_err(|error| completion_from_cmd_error(vm, error.into()))?;
            let reached = vm.store_original_named_variable(name, list.clone());
            currency
                .check(vm)
                .map_err(|error| completion_from_cmd_error(vm, error))?;
            reached?;
            return Ok(());
        };
        let context = vm
            .native_jim_object_context()
            .map_err(|error| completion_from_cmd_error(vm, error.into()))?;
        original
            .value()
            .bind_native_jim_context(&context)
            .map_err(|error| completion_from_cmd_error(vm, error.into()))?;
        // DictSize conversion precedes the sharing decision; the original root
        // lease adds no native hold before either stage.
        let reached = original_compound_ops(vm, currency, |ops| {
            ops.prepare_dictionary_with_currency(original.value(), strings, currency)
        });
        currency
            .check(vm)
            .map_err(|error| completion_from_cmd_error(vm, error))?;
        let mut working = reached.map_err(|error| completion_from_cmd_error(vm, error))?;
        for pair in items.as_chunks::<2>().0 {
            items
                .check_generation()
                .map_err(|error| completion_from_cmd_error(vm, error.into()))?;
            let reached = original_compound_ops(vm, currency, |ops| {
                ops.set_dictionary_member_with_currency(
                    &mut working,
                    pair[0].native_lifetime_lease().into_value(),
                    pair[1].native_lifetime_lease().into_value(),
                    strings,
                    currency,
                )
            });
            currency
                .check(vm)
                .map_err(|error| completion_from_cmd_error(vm, error))?;
            reached.map_err(|error| completion_from_cmd_error(vm, error))?;
        }
        let reached = vm.store_original_named_variable(name, working.into_value());
        currency
            .check(vm)
            .map_err(|error| completion_from_cmd_error(vm, error))?;
        reached?;
        Ok(())
    })();
    drop(items);
    if let Err(error) = currency.check(vm) {
        return completion_from_cmd_error(vm, error);
    }
    match result {
        Ok(()) => ok(Value::empty()),
        Err(error) => error,
    }
}

fn array_trace_target<'a>(vm: &Vm, sub: &str, rest: &'a [Value]) -> Option<&'a Value> {
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
    rest.get(index)
}

/// `array for {keyVar valueVar} arrayName script` — iterate the array's elements,
/// binding the two vars and running the body once per pair (mirrors `dict for`;
/// `break`/`continue` apply, an error/return propagates). The element set is
/// snapshotted up front so body mutations don't perturb the walk.
fn array_for(
    vm: &mut Vm,
    rest: &[Value],
    trace_target: Option<&Value>,
    currency: &NativeOperationCurrency,
) -> Completion<Value> {
    let [vars, arrname, body] = rest else {
        return crate::command::native_wrong_args(vm, "array for {key value} arrayName script");
    };
    let Some(strings) = vm
        .actual_native_invocation_dialect()
        .native_array_for_protocol()
    else {
        return completion_from_cmd_error(
            vm,
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("original array for worker")
                .into(),
        );
    };
    let result = (|| {
        // ArrayForNRCmd validates the original List length first; it neither
        // copies that List nor reads its variable members before LocateArray.
        let names = original_list(vm, vars, strings, currency)?;
        if names.len() != 2 {
            return Err(crate::command::err_with_code(
                "must have two variable names",
                "TCL SYNTAX array for",
            ));
        }
        drop(names);
        let name = original_bytes(vm, trace_target.unwrap_or(arrname), currency)?;
        Ok(vm.with_array_trace_target_bytes(&name, |vm, target| {
            array_for_after_trace(vm, vars, &name, body, strings, target, currency)
        }))
    })();
    if let Err(error) = currency.check(vm) {
        return completion_from_cmd_error(vm, error);
    }
    match result {
        Ok(completion) | Err(completion) => completion,
    }
}

fn array_for_after_trace(
    vm: &mut Vm,
    variables: &Value,
    name: &[u8],
    body: &Value,
    strings: tcl_syntax::native_string::NativeStringProtocol,
    target: &ArrayTarget,
    currency: &NativeOperationCurrency,
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
    let copied = match original_compound_ops(vm, currency, |ops| {
        ops.copy_list_with_currency(variables, strings, currency)
    }) {
        Ok(copied) => copied,
        Err(error) => return completion_from_cmd_error(vm, error),
    };
    let names = match original_list(vm, &copied, strings, currency) {
        Ok(names) => names,
        Err(error) => return error,
    };
    let [kvar, vvar] = names.as_ref().as_slice() else {
        return completion_from_cmd_error(
            vm,
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("array for copied variables")
                .into(),
        );
    };
    let script = body.clone();
    let result = array_for_loop(
        vm, kvar, vvar, name, &script, target, &keys, revision, currency,
    );
    drop(script);
    drop(names);
    drop(copied);
    match currency.check(vm) {
        Ok(()) => result,
        Err(error) => {
            drop(result);
            completion_from_cmd_error(vm, error)
        }
    }
}

fn array_for_loop(
    vm: &mut Vm,
    kvar: &Value,
    vvar: &Value,
    name: &[u8],
    body: &Value,
    target: &ArrayTarget,
    keys: &[Vec<u8>],
    revision: Option<u64>,
    currency: &NativeOperationCurrency,
) -> Completion<Value> {
    for key in keys {
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
        let original_key = match vm.original_array_key_at(target, key) {
            Ok(key) => key,
            Err(error) => return completion_from_cmd_error(vm, error.into()),
        };
        let reached = vm.read_elem_traced_bytes(name, key);
        if let Err(error) = currency.check(vm) {
            drop(reached);
            return completion_from_cmd_error(vm, error);
        }
        let value = match reached {
            Ok(value) => value,
            Err(error) => {
                if vm.refused_completion().is_some() {
                    return error;
                }
                vm.publish_swallowed_trace_error();
                None
            }
        };
        let value = value.map(|value| {
            let lease = value.native_lifetime_lease();
            drop(value);
            lease
        });
        if value.is_none() {
            continue;
        }
        if let Err(error) = original_key.value().check_native_header() {
            return completion_from_cmd_error(vm, error.into());
        }
        let reached = vm.store_original_named_variable(kvar, original_key.clone().into_value());
        if let Err(error) = currency.check(vm) {
            drop(reached);
            return completion_from_cmd_error(vm, error);
        }
        if let Err(error) = reached {
            return error;
        }
        if let Some(value) = value {
            if let Err(error) = value.value().check_native_header() {
                return completion_from_cmd_error(vm, error.into());
            }
            let reached = vm.store_original_named_variable(vvar, value.into_value());
            if let Err(error) = currency.check(vm) {
                drop(reached);
                return completion_from_cmd_error(vm, error);
            }
            if let Err(error) = reached {
                return error;
            }
        }
        let completion = vm.eval_value_at_level(vm.current_level(), body);
        drop(original_key);
        if let Err(error) = currency.check(vm) {
            drop(completion);
            return completion_from_cmd_error(vm, error);
        }
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

#[cfg(test)]
#[path = "cmd_array/original_argument_tests.rs"]
mod original_argument_tests;
