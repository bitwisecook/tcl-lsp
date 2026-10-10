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

//! The `dict` ensemble over the VM's typed dictionary representation.

use std::rc::Rc;

use tcl_runtime_api::completion_options::ControlOptionPolicy;
use tcl_runtime_api::{Code, Completion};

use crate::command::{BuiltinFn, settle_control_options};
use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) struct VmDictionaryObjects {
    string: tcl_syntax::native_string::NativeStringProtocol,
    names: tcl_syntax::naming::NativeNameProtocol,
    preparation: tcl_cmd_core::native_dictionary::NativeDictionaryPreparation,
    jim_context: Option<Rc<crate::value::NativeJimObjectContext>>,
}

impl VmDictionaryObjects {
    pub(crate) const fn string_protocol(&self) -> tcl_syntax::native_string::NativeStringProtocol {
        self.string
    }
    pub(crate) fn with_preparation(
        mut self,
        preparation: tcl_cmd_core::native_dictionary::NativeDictionaryPreparation,
    ) -> Self {
        self.preparation = preparation;
        self
    }
    pub(crate) fn selected(vm: &Vm) -> Result<Self, tcl_cmd_core::CmdError> {
        let dialect = vm.native_invocation_dialect();
        let string = dialect.native_string_materialization(Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        )).ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable("native dictionary object protocol"))?.protocol();
        let names = dialect.native_name_protocol().ok_or(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary diagnostic protocol",
            ),
        )?;
        Ok(Self {
            string,
            names,
            jim_context: if string.is_jim084() {
                Some(vm.native_jim_object_context()?)
            } else {
                None
            },
            preparation:
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::CopyBeforeConversion,
        })
    }
}

impl tcl_cmd_core::native_dictionary::NativeDictionaryObjects for VmDictionaryObjects {
    type Value = Value;
    type Prepared = crate::value::PreparedNativeDictionary;

    fn prepare(&self, original: Option<&Value>) -> Result<Self::Prepared, tcl_cmd_core::CmdError> {
        if let (Some(original), Some(context)) = (original, &self.jim_context) {
            original.bind_native_jim_context(context)?;
        }
        match original {
            Some(value) => match self.preparation {
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::CopyBeforeConversion => value.prepare_native_dictionary(self.string),
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::ConvertBeforeCopy => value.prepare_native_dictionary_after_conversion(self.string),
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::IncrementCommand => value.prepare_native_dictionary_for_increment(self.string),
            },
            None => Value::dict(Vec::new()).prepare_native_dictionary(self.string),
        }
        .map_err(Into::into)
    }

    fn prepare_child(
        &self,
        original: Option<&Value>,
    ) -> Result<Self::Prepared, tcl_cmd_core::CmdError> {
        if let (Some(original), Some(context)) = (original, &self.jim_context) {
            original.bind_native_jim_context(context)?;
        }
        match original {
            Some(value) => value.prepare_native_dictionary_after_conversion(self.string),
            None => Value::dict(Vec::new()).prepare_native_dictionary(self.string),
        }
        .map_err(Into::into)
    }

    fn with_member<R>(
        &self,
        dictionary: &Self::Prepared,
        key: &Value,
        operation: impl FnOnce(Option<&Value>) -> Result<R, tcl_cmd_core::CmdError>,
    ) -> Result<R, tcl_cmd_core::CmdError> {
        dictionary.with_member(key, operation)?
    }

    fn set_member(
        &self,
        dictionary: &mut Self::Prepared,
        key: &Value,
        value: Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        dictionary
            .set_member(key.clone(), value)
            .map_err(Into::into)
    }

    fn remove_member(
        &self,
        dictionary: &mut Self::Prepared,
        key: &Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        dictionary
            .remove_member(key)
            .map(|_| ())
            .map_err(Into::into)
    }

    fn missing_key(&self, key: &Value) -> tcl_cmd_core::CmdError {
        let Ok(original) = key.native_string_bytes(self.string) else {
            return tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary missing key string",
            )
            .into();
        };
        match tcl_syntax::naming::report_native_dictionary_missing_key(
            self.names,
            tcl_syntax::naming::NativeDictionaryMissingKeyOperation::UnsetIntermediate,
            &original,
        ) {
            Ok(report) => match report.error_code {
                Some(code) => tcl_cmd_core::CmdError::with_error_code_bytes(report.message, code),
                None => tcl_cmd_core::CmdError::new_bytes(report.message),
            },
            Err(_) => tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native dictionary missing-key diagnostic",
            )
            .into(),
        }
    }

    fn finish(&self, dictionary: Self::Prepared) -> Value {
        dictionary.into_value()
    }
}

pub(crate) fn register(vm: &mut Vm) {
    vm.refresh_scripted_distribution_libraries();
    let Some(namespace) = vm
        .native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Dict)
    else {
        vm.register_stock_builtin("dict", cmd_dict);
        return;
    };
    let subs = crate::environment::release_subcommands(
        vm.actual_native_execution_profile().name,
        "dict",
        DICT_SUBS,
    );
    vm.register_stock_namespace_ensemble("dict", namespace, DICT_MEMBERS, subs);
}

/// Repinning retains replaced public commands and selects native member tokens.
pub(crate) fn refresh_profile(vm: &mut Vm) {
    vm.refresh_scripted_distribution_libraries();
    if vm.stock_native_identity("dict").as_deref() != Some("dict") {
        return;
    }
    for &(member, _) in DICT_MEMBERS {
        let target = format!("::tcl::dict::{member}");
        if vm.stock_native_identity(&target).as_deref() == target.strip_prefix("::") {
            vm.remove_registered_command(target.trim_start_matches("::"));
        }
    }
    register(vm);
    if vm
        .native_invocation_dialect()
        .ensemble_implementation_namespace(tcl_registry::EnsembleImplementationFamily::Dict)
        .is_none()
    {
        vm.retire_unused_stock_ensemble_namespace("dict");
    }
}

const DICT_MEMBERS: &[(&str, BuiltinFn)] = &[
    ("append", |vm, args| member_op(vm, "append", args)),
    ("create", |vm, args| member_op(vm, "create", args)),
    ("exists", |vm, args| member_op(vm, "exists", args)),
    ("filter", |vm, args| member_op(vm, "filter", args)),
    ("for", |vm, args| member_op(vm, "for", args)),
    ("get", |vm, args| member_op(vm, "get", args)),
    ("getdef", |vm, args| member_op(vm, "getdef", args)),
    ("getwithdefault", |vm, args| {
        member_op(vm, "getwithdefault", args)
    }),
    ("incr", |vm, args| member_op(vm, "incr", args)),
    ("info", |vm, args| member_op(vm, "info", args)),
    ("keys", |vm, args| member_op(vm, "keys", args)),
    ("lappend", |vm, args| member_op(vm, "lappend", args)),
    ("map", |vm, args| member_op(vm, "map", args)),
    ("merge", |vm, args| member_op(vm, "merge", args)),
    ("remove", |vm, args| member_op(vm, "remove", args)),
    ("replace", |vm, args| member_op(vm, "replace", args)),
    ("set", |vm, args| member_op(vm, "set", args)),
    ("size", |vm, args| member_op(vm, "size", args)),
    ("unset", |vm, args| member_op(vm, "unset", args)),
    ("update", |vm, args| member_op(vm, "update", args)),
    ("values", |vm, args| member_op(vm, "values", args)),
    ("with", |vm, args| member_op(vm, "with", args)),
];

/// `dict`'s subcommand set, alphabetical as `TclMakeEnsemble` sorts it. The
/// registry filters this full implemented table for the selected Tcl release.
const DICT_SUBS: &[&str] = &[
    "append",
    "create",
    "exists",
    "filter",
    "for",
    "get",
    "getdef",
    "getwithdefault",
    "incr",
    "info",
    "keys",
    "lappend",
    "map",
    "merge",
    "remove",
    "replace",
    "set",
    "size",
    "unset",
    "update",
    "values",
    "with",
];

#[cfg(test)]
type StringPairs = Vec<(String, Value)>;

/// `dict subcommand ?arg ...?` — dispatch to the subcommand handler.
fn cmd_dict(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"dict subcommand ?arg ...?\"",
        );
    };
    let word = match vm.native_name_operand_bytes(sub) {
        Ok(word) => word,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    // `dict` is a `TclMakeEnsemble` command: exact match, else a unique
    // prefix, so `dict k` is `dict keys`.
    // `getdef`/`getwithdefault` are Tcl 9 (TIP 342): under an 8.6 pin they
    // must not resolve, and — the reason this matters for words that have
    // nothing to do with them — must not make `dict g` ambiguous either.
    let subs = crate::environment::release_subcommands(
        vm.command_surface_profile().name,
        "dict",
        DICT_SUBS,
    );
    let Some(index) = tcl_cmd_core::ensemble::resolve_subcommand(subs, &word, true) else {
        return err(tcl_cmd_core::ensemble::unknown_subcommand_message(
            subs,
            &word,
            true,
            b"::tcl::dict",
        ));
    };
    if let Some(dispatch) = tcl_registry::dictionary_scope::scripted_dictionary_dispatch(
        vm.native_invocation_dialect(),
        subs[index],
        &word,
        rest.len(),
    ) {
        let Some(count) = dispatch.arguments() else {
            return crate::command::native_wrong_args_bytes(vm, dispatch.usage());
        };
        let target = Value::from_native_string_bytes(dispatch.command().to_vec());
        return vm.invoke_command_value_at(
            vm.current_ns_id(),
            &target,
            &rest[..count],
            &[],
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
    }
    let invoked = vm.invoked_name().unwrap_or("dict").to_owned();
    let usage_prefix = format!("{invoked} {}", subs[index]);
    dict_op(vm, subs[index], rest, &usage_prefix)
}

/// Dispatch a separately invocable ensemble implementation command. Its arity
/// text uses the actual command spelling, including a name installed by
/// `rename`, rather than reconstructing `dict <subcommand>`.
fn member_op(vm: &mut Vm, sub: &str, args: &[Value]) -> Completion<Value> {
    let invoked = vm
        .invoked_name()
        .map_or_else(|| format!("::tcl::dict::{sub}"), str::to_owned);
    dict_op(vm, sub, args, &invoked)
}

/// The dict's **canonical** ordered `(key-string, value)` pairs, from the one
/// [`ValueOps::dict_pairs`](tcl_syntax::value::ValueOps::dict_pairs) owner:
/// first-occurrence position, **last value winning** on a duplicate key
/// (`SetDictFromAny`, tclDictObj.c(9.0.4):589 → `Tcl_DictObjPut`). Decoding the
/// list rep straight into `chunks_exact(2)` pairs instead leaves both values of
/// a duplicate key present, so every [`lookup`] reads the *first*.
#[cfg(test)]
pub(crate) fn pairs(vm: &mut Vm, v: &Value) -> Result<StringPairs, Completion<Value>> {
    vm.dict_pairs(v)
}

/// Read the dict in variable `varname`, transform the value at `key` via `f`
/// (given the current value, if any), write it back, and return the new dict.
/// Shared by `dict incr`/`append`/`lappend`.
fn dict_update(
    vm: &mut Vm,
    varname: &Value,
    key: &Value,
    preparation: tcl_cmd_core::native_dictionary::NativeDictionaryPreparation,
    operation: impl FnOnce(&mut Vm, Option<&Value>) -> Result<Value, tcl_cmd_core::CmdError>,
) -> Completion<Value> {
    let Ok(name) = vm.native_name_operand_bytes(varname) else {
        return vm.refuse_host_command("native dictionary variable string is unavailable".into());
    };
    let objects = match VmDictionaryObjects::selected(vm) {
        Ok(objects) => objects.with_preparation(preparation),
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    match vm.dictionary_variable_update_bytes(
        &name,
        crate::interp::native_dictionary::DictionaryVariablePublication::CommandName,
        &objects,
        |vm, prepared| {
            tcl_cmd_core::native_dictionary::update_prepared_member(
                &objects,
                prepared,
                key,
                |member| operation(vm, member),
            )
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
        },
    ) {
        Ok(result) => Completion::new(Code::Ok, result.value, result.options),
        Err(error) => error,
    }
}

fn dictionary_path_update(
    vm: &mut Vm,
    name: &Value,
    keys: &[Value],
    value: Option<Value>,
) -> Completion<Value> {
    let Ok(name) = vm.native_name_operand_bytes(name) else {
        return vm.refuse_host_command("native dictionary variable string is unavailable".into());
    };
    match dictionary_path_update_bytes(
        vm,
        &name,
        crate::interp::native_dictionary::DictionaryVariablePublication::CommandName,
        keys,
        value,
    ) {
        Ok(result) => Completion::new(Code::Ok, result.value, result.options),
        Err(error) => error,
    }
}

fn dictionary_append(
    vm: &mut Vm,
    name: &Value,
    key: &Value,
    sources: &[Value],
) -> Completion<Value> {
    let Some(policy) = vm
        .native_invocation_dialect()
        .native_dictionary_append_policy()
    else {
        return vm.refuse_host_command(
            "native dictionary append primitive policy is unavailable".into(),
        );
    };
    let name = match vm.native_name_operand_bytes(name) {
        Ok(name) => name,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let objects = match VmDictionaryObjects::selected(vm) {
        Ok(objects) => objects,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    match vm.dictionary_variable_update_bytes(&name,
        crate::interp::native_dictionary::DictionaryVariablePublication::CommandName,
        &objects, |vm, prepared| {
            tcl_cmd_core::native_dictionary::update_prepared_member_if(&objects, prepared, key, |member| {
                if member.is_some() && sources.is_empty()
                    && policy.empty == tcl_registry::native_dictionary::NativeDictionaryEmptyAppend::StoreOnly {
                    Ok(None)
                } else {
                    append_member_values(vm, member, sources).map(Some)
                }
            }).map_err(|error| crate::command::completion_from_cmd_error(vm, error))
        }) {
        Ok(result) => Completion::new(Code::Ok, result.value, result.options),
        Err(error) => error,
    }
}

pub(crate) fn dictionary_path_update_bytes(
    vm: &mut Vm,
    name: &[u8],
    publication: crate::interp::native_dictionary::DictionaryVariablePublication,
    keys: &[Value],
    value: Option<Value>,
) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
    let objects = VmDictionaryObjects::selected(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let operation = |vm: &mut Vm, prepared| {
        match value {
            Some(value) => {
                tcl_cmd_core::native_dictionary::set_prepared_path(&objects, prepared, keys, value)
            }
            None => tcl_cmd_core::native_dictionary::remove_prepared_path(&objects, prepared, keys),
        }
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
    };
    match vm
        .native_invocation_dialect()
        .native_dictionary_path_publication()
    {
        Some(tcl_registry::native_dictionary::NativeDictionaryPathPublication::COriginalName) => {
            vm.dictionary_variable_update_bytes(name, publication, &objects, operation)
        }
        Some(
            tcl_registry::native_dictionary::NativeDictionaryPathPublication::JimOriginalVariable,
        ) => {
            if !matches!(
                publication,
                crate::interp::native_dictionary::DictionaryVariablePublication::CommandName
            ) {
                return Err(vm.refuse_host_command(
                    "Jim dictionary path has no C compiled-local receiver".into(),
                ));
            }
            vm.jim_dictionary_path_update_bytes(name, &objects, operation)
        }
        None => {
            Err(vm.refuse_host_command("native dictionary path publication is unavailable".into()))
        }
    }
}

pub(crate) fn dictionary_member_update_bytes(
    vm: &mut Vm,
    name: &[u8],
    publication: crate::interp::native_dictionary::DictionaryVariablePublication,
    key: &Value,
    operation: impl FnOnce(&mut Vm, Option<&Value>) -> Result<Value, tcl_cmd_core::CmdError>,
) -> Result<tcl_runtime_api::VariableUpdateResult<Value>, Completion<Value>> {
    let objects = VmDictionaryObjects::selected(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    vm.dictionary_variable_update_bytes(name, publication, &objects, |vm, prepared| {
        tcl_cmd_core::native_dictionary::update_prepared_member(&objects, prepared, key, |member| {
            operation(vm, member)
        })
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
    })
}

/// A missing compiled/implied member is a fresh integer; a generic explicit
/// amount is validated by the copying Bignum getter and retained unchanged.
pub(crate) fn increment_dictionary_member(
    vm: &mut Vm,
    original: Option<&Value>,
    amount: &Value,
    fresh_missing: bool,
) -> Result<Value, tcl_cmd_core::CmdError> {
    let objects = crate::value_ops::VmIncrementObjects::selected(vm.native_invocation_dialect())?;
    if original.is_none() {
        if fresh_missing {
            return Ok(amount.clone());
        }
        return tcl_cmd_core::native_increment::missing_dictionary_member(&objects, amount);
    }
    tcl_cmd_core::native_increment::increment(&objects, original, amount)
}

/// A compiled append adopts its already concatenated input on a missing member.
pub(crate) fn append_compiled_member_value(
    vm: &Vm,
    original: Option<&Value>,
    source: &Value,
) -> Result<Value, tcl_cmd_core::CmdError> {
    let Some(original) = original else {
        return Ok(source.clone());
    };
    let issued = vm
        .native_invocation_dialect()
        .native_object_append_protocol(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native compiled dictionary append",
        ))?;
    tcl_cmd_core::native_append::append_dictionary_operands(
        &crate::value::VmAppendObjects,
        issued.recipe(),
        Some(original),
        std::slice::from_ref(source),
    )
    .map_err(Into::into)
}

pub(crate) fn append_member_values(
    vm: &mut Vm,
    original: Option<&Value>,
    sources: &[Value],
) -> Result<Value, tcl_cmd_core::CmdError> {
    use tcl_registry::native_dictionary::NativeDictionaryMissingAppendMember;
    let policy = vm
        .native_invocation_dialect()
        .native_dictionary_append_policy()
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native dictionary member append policy",
        ))?;
    let issued = vm
        .native_invocation_dialect()
        .native_object_append_protocol(None)
        .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native dictionary member append",
        ))?;
    let empty = Value::empty();
    let receiver = if sources.is_empty()
        || policy.missing == NativeDictionaryMissingAppendMember::FreshEmptyReceiver
    {
        original.or(Some(&empty))
    } else {
        original
    };
    if let tcl_registry::native_dictionary::NativeDictionaryAppendInputs::Concatenate(cat) =
        policy.inputs
        && sources.len() > 1
    {
        let source = tcl_cmd_core::native_cat::concatenate(
            &crate::value::VmAppendObjects,
            cat.recipe(),
            sources,
            true,
        )?;
        return tcl_cmd_core::native_append::append_dictionary_operands(
            &crate::value::VmAppendObjects,
            issued.recipe(),
            receiver,
            std::slice::from_ref(&source),
        )
        .map_err(Into::into);
    }
    tcl_cmd_core::native_append::append_dictionary_operands(
        &crate::value::VmAppendObjects,
        issued.recipe(),
        receiver,
        sources,
    )
    .map_err(Into::into)
}

pub(crate) fn lappend_member_values(
    vm: &mut Vm,
    original: Option<&Value>,
    elements: &[Value],
) -> Result<Value, tcl_cmd_core::CmdError> {
    let protocol = VmDictionaryObjects::selected(vm)?.string_protocol();
    Value::native_list_append_elements(original, elements, protocol).map_err(Into::into)
}

pub(crate) fn put_original_member(
    vm: &mut Vm,
    current: &Value,
    key: &Value,
    value: Value,
) -> Result<Value, tcl_cmd_core::CmdError> {
    let objects = VmDictionaryObjects::selected(vm)?;
    tcl_cmd_core::native_dictionary::set_path(
        &objects,
        Some(current),
        std::slice::from_ref(key),
        value,
    )
}

#[cfg(test)]
fn set_path(
    vm: &mut Vm,
    current: &Value,
    keys: &[Value],
    value: Value,
) -> Result<Value, Completion<Value>> {
    let objects = VmDictionaryObjects::selected(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let retained_input = current.clone();
    tcl_cmd_core::native_dictionary::set_path(&objects, Some(&retained_input), keys, value)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
}

#[cfg(test)]
fn unset_path(vm: &mut Vm, current: &Value, keys: &[Value]) -> Result<Value, Completion<Value>> {
    let objects = VmDictionaryObjects::selected(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let retained_input = current.clone();
    tcl_cmd_core::native_dictionary::remove_path(&objects, Some(&retained_input), keys)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
}

fn dict_op(vm: &mut Vm, sub: &str, rest: &[Value], invoked: &str) -> Completion<Value> {
    // Pure dict subcommands live in the shared command core; the VM is a thin
    // adapter. Only the variable-*mutating* subcommands fall through to the
    // arms below.
    //
    // `dispatch_canon` answers `Some` unconditionally for the pure subcommands,
    // including `info`, so any VM-local arm for those would be unreachable —
    // and not merely inert: a `create` arm built with a plain `Value::list` of
    // the arguments would re-introduce the duplicate-key bug these
    // subcommands must avoid, so no such arm exists here.
    if let Some(result) = tcl_cmd_core::dict::dispatch_canon(vm, invoked.as_bytes(), sub, rest) {
        return match result {
            Ok(v) => ok(v),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        };
    }
    match sub {
        "set" => {
            // dict set dictVarName key ?key ...? value
            let [varname, keys @ .., value] = rest else {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"dict set dictVarName key ?key ...? value\"",
                );
            };
            if keys.is_empty() {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"dict set dictVarName key ?key ...? value\"",
                );
            }
            dictionary_path_update(vm, varname, keys, Some(value.clone()))
        }
        "unset" => {
            let [varname, keys @ ..] = rest else {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"dict unset dictVarName key ?key ...?\"",
                );
            };
            if keys.is_empty() {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"dict unset dictVarName key ?key ...?\"",
                );
            }
            dictionary_path_update(vm, varname, keys, None)
        }
        "incr" => {
            let [varname, key, amt @ ..] = rest else {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"dict incr dictVarName key ?increment?\"",
                );
            };
            if amt.len() > 1 {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"dict incr dictVarName key ?increment?\"",
                );
            }
            let one = Value::int(1);
            let inc = amt.first().unwrap_or(&one);
            dict_update(
                vm,
                varname,
                key,
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::IncrementCommand,
                |vm, old| increment_dictionary_member(vm, old, inc, amt.is_empty()),
            )
        }
        "append" => {
            let [varname, key, strs @ ..] = rest else {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"dict append dictVarName key ?value ...?\"",
                );
            };
            dictionary_append(vm, varname, key, strs)
        }
        "lappend" => {
            let [varname, key, vals @ ..] = rest else {
                return crate::command::native_wrong_arguments_message(
                    vm,
                    "wrong # args: should be \"dict lappend dictVarName key ?value ...?\"",
                );
            };
            dict_update(
                vm,
                varname,
                key,
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::CopyBeforeConversion,
                |vm, old| lappend_member_values(vm, old, vals),
            )
        }
        "for" => cmd_dict_for(vm, rest),
        "map" => cmd_dict_map(vm, rest),
        "update" => cmd_dict_update(vm, rest),
        "filter" => cmd_dict_filter(vm, rest),
        "with" => cmd_dict_with(vm, rest),
        // Unreachable from `cmd_dict` (every `DICT_SUBS` name is handled above
        // or by the shared core); the registered `::tcl::dict::*` entry points
        // pass canonical names.
        other => err(tcl_cmd_core::ensemble::unknown_subcommand_message(
            DICT_SUBS,
            other.as_bytes(),
            true,
            b"::tcl::dict",
        )),
    }
}

fn dictionary_scope_plan(
    vm: &mut Vm,
    spec: tcl_registry::dictionary_scope::DictionaryScopeSpec,
    count: usize,
) -> Result<tcl_registry::dictionary_scope::DictionaryScopePlan, Completion<Value>> {
    let words = vec![tcl_registry::InvocationWord::Dynamic; count];
    match spec.select(
        tcl_registry::InvocationArguments::structured(&words)
            .with_dialect(vm.native_invocation_dialect()),
        0,
    ) {
        tcl_registry::dictionary_scope::DictionaryScopeSelection::Selected(plan) => Ok(plan),
        _ => Err(vm.refuse_host_command("native dictionary scope protocol is unavailable".into())),
    }
}

fn reflect_dictionary_scope_leaf(
    vm: &mut Vm,
    mut leaf: crate::value::PreparedNativeDictionary,
    mappings: &[(Value, Value)],
) -> Result<Value, tcl_cmd_core::CmdError> {
    for (key, variable) in mappings {
        let name = vm.native_name_operand_bytes(variable).map_err(|_| {
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "dictionary scope variable string",
            )
        })?;
        if let Ok(value) = vm.read_variable_result_bytes(&name, None) {
            let value = if leaf.is_same_object(&value) {
                leaf.duplicate_value(&value)
            } else {
                value
            };
            leaf.set_member(key.clone(), value)?;
        } else {
            if vm.refused_completion().is_some() {
                return Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "dictionary scope target read",
                )
                .into());
            }
            vm.publish_swallowed_trace_error();
            leaf.remove_member(key)?;
        }
    }
    Ok(leaf.into_value())
}

pub(crate) fn expand_dictionary_scope(
    vm: &mut Vm,
    dictionary: &Value,
    path: &Value,
) -> Result<Value, Completion<Value>> {
    let objects = VmDictionaryObjects::selected(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    let path = vm
        .native_object_list_elements_in(path, objects.string_protocol())
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
    let leaf = if path.is_empty() {
        dictionary.clone()
    } else {
        tcl_cmd_core::dict::get(vm, dictionary, &path)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?
    };
    let pairs = native_dictionary_pairs(vm, &leaf)?;
    let keys = pairs.into_iter().map(|(key, _)| key).collect::<Vec<_>>();
    for key in &keys {
        let bytes = key
            .native_string_bytes(objects.string_protocol())
            .map_err(|error| vm.refuse_host_command(error.to_string()))?;
        let value = leaf
            .with_cached_dictionary_member(&bytes, Value::retain_borrowed_member)
            .expect("selected Dictionary cache")
            .expect("retained Dictionary key");
        let name = vm
            .native_name_operand_bytes(key)
            .map_err(|error| vm.refuse_host_command(error.to_string()))?;
        vm.store_var_result_bytes(&name, value)?;
    }
    Ok(Value::native_list_constructor(
        keys,
        objects.string_protocol(),
    ))
}

pub(crate) fn recombine_dictionary_scope(
    vm: &mut Vm,
    name: &[u8],
    publication: crate::interp::native_dictionary::DictionaryVariablePublication,
    path: &Value,
    state: &Value,
) -> Result<(), Completion<Value>> {
    let objects = VmDictionaryObjects::selected(vm)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?
        .with_preparation(
            tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::ConvertBeforeCopy,
        );
    let path = vm
        .native_object_list_elements_in(path, objects.string_protocol())
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
    vm.dictionary_scope_writeback_bytes(name, publication, &objects, |vm, root| {
        tcl_cmd_core::native_dictionary::transform_existing_prepared_path(
            &objects,
            root,
            &path,
            |leaf| {
                let keys = vm.native_object_list_elements_in(state, objects.string_protocol())?;
                let mappings = keys
                    .iter()
                    .cloned()
                    .map(|key| (key.clone(), key))
                    .collect::<Vec<_>>();
                reflect_dictionary_scope_leaf(vm, leaf, &mappings)
            },
        )
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
    })
}

/// Map original dictionary keys into actual caller variables and synchronize
/// the reached subtree according to the selected native completion policy.
fn cmd_dict_with(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((variable, tail)) = rest.split_first() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"dict with dictVarName ?key ...? script\"",
        );
    };
    let Some((body, path)) = tail.split_last() else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"dict with dictVarName ?key ...? script\"",
        );
    };
    let plan = match dictionary_scope_plan(
        vm,
        tcl_registry::dictionary_scope::DictionaryScopeSpec::With,
        rest.len(),
    ) {
        Ok(plan) => plan,
        Err(error) => return error,
    };
    let name = match vm.native_name_operand_bytes(variable) {
        Ok(name) => name,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let root = match vm.read_variable_result_bytes(&name, None) {
        Ok(root) => root,
        Err(error) => return error,
    };
    let leaf = if path.is_empty() {
        root.clone()
    } else {
        match tcl_cmd_core::dict::get(vm, &root, path) {
            Ok(leaf) => leaf,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        }
    };
    let pairs = match native_dictionary_pairs(vm, &leaf) {
        Ok(pairs) => pairs,
        Err(error) => return error,
    };
    let mut mappings = Vec::with_capacity(pairs.len());
    for (key, value) in pairs {
        let written = match vm.native_name_operand_bytes(&key) {
            Ok(name) => name,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        if let Err(error) = vm.store_var_result_bytes(&written, value) {
            return error;
        }
        mappings.push((key.clone(), key));
    }
    drop(leaf);
    drop(root);
    let mut outcome = vm.eval_value_at_level(vm.current_level(), body);
    if let Some(refusal) = vm.refused_completion() {
        return refusal;
    }
    if plan.writeback == tcl_registry::dictionary_scope::DictionaryWriteback::NormalOnly
        && outcome.code != Code::Ok
    {
        return outcome;
    }
    // The saved body result owns a native reference while writeback invokes
    // commands that can replace the interpreter's current result.
    outcome.result = outcome.result.into_native_reference();
    let objects = match VmDictionaryObjects::selected(vm) {
        Ok(objects) => objects.with_preparation(
            tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::ConvertBeforeCopy,
        ),
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let publication = if objects.string_protocol().is_jim084() {
        crate::interp::native_dictionary::DictionaryVariablePublication::CommandName
    } else {
        crate::interp::native_dictionary::DictionaryVariablePublication::RetainedLocalCell
    };
    let writeback =
        vm.dictionary_scope_writeback_bytes(&name, publication, &objects, |vm, root| {
            tcl_cmd_core::native_dictionary::transform_existing_prepared_path(
                &objects,
                root,
                path,
                |leaf| reflect_dictionary_scope_leaf(vm, leaf, &mappings),
            )
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
        });
    if let Some(refusal) = vm.refused_completion() {
        return refusal;
    }
    if let Err(mut error) = writeback {
        if plan.writeback_failure
            == tcl_registry::dictionary_scope::DictionaryWritebackFailure::NormalWithErrorResult
        {
            error.code = Code::Ok;
        }
        return error;
    }
    outcome
}

fn native_dictionary_pairs(
    vm: &mut Vm,
    value: &Value,
) -> Result<Vec<(Value, Value)>, Completion<Value>> {
    tcl_syntax::value::ValueOps::dict_pairs(vm, value)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))
}

fn native_dictionary_loop_names(
    vm: &mut Vm,
    vars: &Value,
) -> Result<DictionaryIterationNames, Completion<Value>> {
    let names = tcl_syntax::value::ValueOps::list_elements(vm, vars)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
    let [key, value] = names.as_slice() else {
        return Err(err("must have exactly two variable names"));
    };
    let key = vm
        .native_name_operand_bytes(key)
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    let value = vm
        .native_name_operand_bytes(value)
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    Ok((key, value))
}

type DictionaryIterationNames = (std::rc::Rc<[u8]>, std::rc::Rc<[u8]>);

fn cmd_dict_for(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [vars, dictionary, body] = rest else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"dict for {keyVar valueVar} dictionary script\"",
        );
    };
    let (key_name, value_name) = match native_dictionary_loop_names(vm, vars) {
        Ok(names) => names,
        Err(error) => return error,
    };
    let pairs = match native_dictionary_pairs(vm, dictionary) {
        Ok(pairs) => pairs,
        Err(error) => return error,
    };
    for (key, value) in pairs {
        if let Err(error) = vm.store_var_result_bytes(&key_name, key) {
            return error;
        }
        if let Err(error) = vm.store_var_result_bytes(&value_name, value) {
            return error;
        }
        let completion = vm.eval_value_at_level(vm.current_level(), body);
        if let Some(refusal) = vm.refused_completion() {
            return refusal;
        }
        match completion.code {
            Code::Ok | Code::Continue => {}
            Code::Break => break,
            _ => return completion,
        }
    }
    settle_control_options(ok(Value::empty()), ControlOptionPolicy::FRESH_SETTLED)
}

/// Map preserves original key objects, including a key assigned by the body.
fn cmd_dict_map(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [vars, dictionary, body] = rest else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"dict map {keyVarName valueVarName} dictionary script\"",
        );
    };
    let (key_name, value_name) = match native_dictionary_loop_names(vm, vars) {
        Ok(names) => names,
        Err(error) => return error,
    };
    let pairs = match native_dictionary_pairs(vm, dictionary) {
        Ok(pairs) => pairs,
        Err(error) => return error,
    };
    let objects = match VmDictionaryObjects::selected(vm) {
        Ok(objects) => objects,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    let mut output =
        match Value::dict(Vec::new()).prepare_native_dictionary(objects.string_protocol()) {
            Ok(output) => output,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
        };
    let mut last_options = Value::empty();
    for (key, value) in pairs {
        if let Err(error) = vm.store_var_result_bytes(&key_name, key) {
            return error;
        }
        if let Err(error) = vm.store_var_result_bytes(&value_name, value) {
            return error;
        }
        let completion = vm.eval_value_at_level(vm.current_level(), body);
        if let Some(refusal) = vm.refused_completion() {
            return refusal;
        }
        match completion.code {
            Code::Ok => {
                let key = match vm.read_variable_result_bytes(&key_name, None) {
                    Ok(key) => key,
                    Err(error) => return error,
                };
                if let Err(error) = output.set_member(key, completion.result) {
                    return crate::command::completion_from_cmd_error(vm, error.into());
                }
                last_options = completion.options;
            }
            Code::Continue => last_options = completion.options,
            Code::Break => {
                return settle_control_options(
                    ok(Value::empty()),
                    ControlOptionPolicy::FRESH_SETTLED,
                );
            }
            _ => return completion,
        }
    }
    settle_control_options(
        Completion::new(Code::Ok, output.into_value(), last_options),
        ControlOptionPolicy::FRESH_FORWARDED,
    )
}

/// Explicit original key/variable pairs surround a caller-frame body.
fn cmd_dict_update(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    const USAGE: &str =
        "wrong # args: should be \"dict update dictVarName key varName ?key varName ...? script\"";
    let [variable, tail @ ..] = rest else {
        return crate::command::native_wrong_arguments_message(vm, USAGE);
    };
    let Some((body, pairs)) = tail.split_last() else {
        return crate::command::native_wrong_arguments_message(vm, USAGE);
    };
    if pairs.is_empty() || !pairs.len().is_multiple_of(2) {
        return crate::command::native_wrong_arguments_message(vm, USAGE);
    }
    let plan = match dictionary_scope_plan(
        vm,
        tcl_registry::dictionary_scope::DictionaryScopeSpec::Update,
        rest.len(),
    ) {
        Ok(plan) => plan,
        Err(error) => return error,
    };
    if plan.implementation
        != tcl_registry::dictionary_scope::DictionaryScopeImplementation::NativePrimitive
    {
        return vm
            .refuse_host_command("dictionary update requires its actual scripted wrapper".into());
    }
    let name = match vm.native_name_operand_bytes(variable) {
        Ok(name) => name,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let root = match vm.read_variable_result_bytes(&name, None) {
        Ok(root) => root,
        Err(error) => return error,
    };
    let objects = match VmDictionaryObjects::selected(vm) {
        Ok(objects) => objects.with_preparation(
            tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::ConvertBeforeCopy,
        ),
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    if let Err(error) = vm.native_object_dict_pairs_in(&root, objects.string_protocol()) {
        return crate::command::completion_from_cmd_error(vm, error.into());
    }
    let mut mappings = Vec::with_capacity(pairs.len() / 2);
    for pair in pairs.as_chunks::<2>().0 {
        let name = match vm.native_name_operand_bytes(&pair[1]) {
            Ok(name) => name,
            Err(error) => return vm.refuse_host_command(error.to_string()),
        };
        let selected = match tcl_cmd_core::dict::getdef(vm, &root, &pair[..1], &Value::empty()) {
            Ok(value) => value,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let exists = match tcl_cmd_core::dict::exists(vm, &root, &pair[..1]) {
            Ok(exists) => exists,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let exists = match tcl_syntax::value::ValueOps::as_bool(vm, &exists) {
            Ok(exists) => exists,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
        };
        if exists {
            if let Err(error) = vm.store_var_result_bytes(&name, selected) {
                return error;
            }
        } else {
            let _ = vm.unset_var_bytes(&name);
            if let Some(refusal) = vm.refused_completion() {
                return refusal;
            }
        }
        mappings.push((pair[0].clone(), pair[1].clone()));
    }
    drop(root);
    let mut outcome = vm.eval_value_at_level(vm.current_level(), body);
    if let Some(refusal) = vm.refused_completion() {
        return refusal;
    }
    outcome.result = outcome.result.into_native_reference();
    match vm.dictionary_scope_writeback_bytes(
        &name,
        crate::interp::native_dictionary::DictionaryVariablePublication::CommandName,
        &objects,
        |vm, root| {
            reflect_dictionary_scope_leaf(vm, root, &mappings)
                .map(Some)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
        },
    ) {
        Ok(()) => outcome,
        Err(error) => error,
    }
}

/// `dict filter dictionary script {keyVar valueVar} body` — the Family-B `script`
/// filter type (the pure `key`/`value` globs are handled by the shared
/// `tcl_cmd_core::dict` core, which returns `None` only for `script`). Keeps each
/// pair whose body result is true; the body's completion code drives the loop
/// (OK ⇒ keep iff true; CONTINUE ⇒ skip; BREAK ⇒ stop; else ⇒ propagate).
fn cmd_dict_filter(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let [dictionary, _script, vars, body] = rest else {
        return crate::command::native_wrong_arguments_message(
            vm,
            "wrong # args: should be \"dict filter dictionary script {keyVarName valueVarName} filterScript\"",
        );
    };
    let pairs = match native_dictionary_pairs(vm, dictionary) {
        Ok(pairs) => pairs,
        Err(error) => return error,
    };
    let (key_name, value_name) = match native_dictionary_loop_names(vm, vars) {
        Ok(names) => names,
        Err(error) => return error,
    };
    let mut kept = Vec::new();
    for (key, value) in pairs {
        if let Err(error) = vm.store_var_result_bytes(&key_name, key.clone()) {
            return error;
        }
        if let Err(error) = vm.store_var_result_bytes(&value_name, value.clone()) {
            return error;
        }
        let completion = vm.eval_value_at_level(vm.current_level(), body);
        if let Some(refusal) = vm.refused_completion() {
            return refusal;
        }
        match completion.code {
            Code::Ok => match tcl_syntax::value::ValueOps::as_bool(vm, &completion.result) {
                Ok(true) => kept.push((key, value)),
                Ok(false) => {}
                Err(error) => return crate::command::completion_from_cmd_error(vm, error.into()),
            },
            Code::Continue => {}
            Code::Break => break,
            _ => return completion,
        }
    }
    ok(Value::dict(kept))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_dictionary_surface_is_independent_of_embedded_source_grammar() {
        let mut vm = Vm::new();
        vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
        assert!(vm.set_command_surface_profile(
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
        ));
        let result = cmd_dict(
            &mut vm,
            &[
                Value::string("create"),
                Value::string("key"),
                Value::string("value"),
            ],
        );
        assert_eq!(result.code, crate::Code::Ok);
        assert_eq!(&*result.result.to_str(), "key value");
        assert_eq!(
            vm.runtime_version(),
            tcl_dialect::DialectProfile::irules().vm_runtime_version
        );
    }

    /// A dict `Value`'s top-level `(key, value-string)` pairs.
    fn top_pairs(vm: &mut Vm, v: &Value) -> Vec<(String, String)> {
        pairs(vm, v)
            .expect("dict value is a valid list")
            .into_iter()
            .map(|(k, val)| (k, val.to_str().to_string()))
            .collect()
    }

    /// `set_path`/`unset_path` recursing once per multi-key `dict
    /// set`/`dict unset` path segment natively has no depth cap: an
    /// unguarded `dict set d {*}[lrepeat 100000 k] v` overflows the native
    /// stack (SIGABRT), empirically between depth 3000 and 3500 on a 2 MiB
    /// thread (`cargo test`'s per-test default). The iterative
    /// implementation (mirroring `get_path`'s existing iterative style) has
    /// no such cap; this test checks `set_path`'s result for exact
    /// correctness at depth 2000, comfortably past that crash range —
    /// descending back down through the same key at every level must land
    /// on the value that was set, not merely survival.
    ///
    /// Deliberately NOT 50,000+: a dict this deep is represented as an
    /// equally deep nested `Value::list` chain, and `Value` has no custom
    /// `Drop` impl — the compiler-generated recursive drop glue that runs
    /// when `set`/`cur` go out of scope at the end of this test is its own,
    /// unrelated native-stack risk (empirically, SIGABRT between depth 3500
    /// and 4000 on a 2 MiB thread for construction+drop alone), a separate
    /// concern in `Value`'s representation itself, out of scope here.
    #[test]
    fn deeply_nested_dict_set_and_unset_survive() {
        const DEPTH: usize = 2_000;
        let vm = &mut Vm::new();
        let keys: Vec<Value> = (0..DEPTH).map(|_| Value::string("k")).collect();
        let set =
            set_path(vm, &Value::empty(), &keys, Value::string("v")).expect("set_path survives");
        let mut cur = set.clone();
        for _ in 0..DEPTH {
            let ps = pairs(vm, &cur).expect("valid dict at every level");
            assert_eq!(ps.len(), 1);
            assert_eq!(ps[0].0, "k");
            cur = ps[0].1.clone();
        }
        assert_eq!(&*cur.to_str(), "v");

        // `unset_path` must also survive the same depth — the assertion is
        // that it returns at all, not the exact shape of what remains (each
        // level's "k" entry survives holding an emptied-out subdict, matching
        // `dict unset`'s "leaf only" removal semantics — the whole chain does
        // not collapse away).
        let _ = unset_path(vm, &set, &keys).expect("unset_path survives");
    }

    /// A moderately nested `dict set`/`dict unset` path (well within
    /// realistic use) is byte-for-byte unaffected by the iterative
    /// rewrite.
    #[test]
    fn moderately_nested_dict_set_and_unset_unaffected() {
        let vm = &mut Vm::new();
        let s = Value::string;

        // `dict set {} a 1` -> {a 1}.
        let d = set_path(vm, &Value::list(vec![]), &[s("a")], s("1")).unwrap();
        assert_eq!(top_pairs(vm, &d), [("a".into(), "1".into())]);

        // Updating an existing key keeps its position (order-preserving).
        let base = Value::list(vec![s("a"), s("1"), s("b"), s("2")]);
        let updated = set_path(vm, &base, &[s("a")], s("9")).unwrap();
        assert_eq!(
            top_pairs(vm, &updated),
            [("a".into(), "9".into()), ("b".into(), "2".into())]
        );

        // A new key appends at the end.
        let appended = set_path(vm, &base, &[s("c")], s("3")).unwrap();
        assert_eq!(
            top_pairs(vm, &appended),
            [
                ("a".into(), "1".into()),
                ("b".into(), "2".into()),
                ("c".into(), "3".into())
            ]
        );

        // A multi-key path auto-vivifies intermediate dicts.
        let nested = set_path(vm, &Value::list(vec![]), &[s("a"), s("b")], s("1")).unwrap();
        assert_eq!(top_pairs(vm, &nested), [("a".into(), "b 1".into())]);

        // Removing a present key drops just that pair.
        let removed = unset_path(vm, &base, &[s("a")]).unwrap();
        assert_eq!(top_pairs(vm, &removed), [("b".into(), "2".into())]);

        // Removing an absent key leaves the dict unchanged.
        let untouched = unset_path(vm, &base, &[s("z")]).unwrap();
        assert_eq!(
            top_pairs(vm, &untouched),
            [("a".into(), "1".into()), ("b".into(), "2".into())]
        );

        // A nested unset rewrites only the inner dict.
        let inner = Value::list(vec![s("b"), s("1"), s("c"), s("2")]);
        let outer = Value::list(vec![s("a"), inner]);
        let nested_unset = unset_path(vm, &outer, &[s("a"), s("b")]).unwrap();
        assert_eq!(top_pairs(vm, &nested_unset), [("a".into(), "c 2".into())]);

        let error = unset_path(vm, &outer, &[s("a"), s("z"), s("q")]).unwrap_err();
        assert_eq!(
            error.result.string_bytes().as_ref(),
            b"key \"z\" not known in dictionary"
        );
    }
}

#[cfg(test)]
mod native_rmw_fixture_tests {
    use crate::{Code, Value, Vm};
    use tcl_syntax::value::ValueOps;

    fn unhex(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0);
        hex.as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn compare_dictionary_process(
        suite: &str,
        engine: &str,
        process: DictionaryProcess<'_>,
        compared: &mut usize,
    ) {
        let DictionaryProcess {
            name,
            code,
            result,
            source,
            profile,
            result_window,
        } = process;
        let receipt_start = std::time::Instant::now();
        tcl_test_support::oracle_row_progress(suite, engine, name, None);
        tcl_test_support::oracle_phase_progress(suite, engine, name, "vm-new-start", receipt_start);
        // Both original C drivers call Jim_InitStaticExtensions, which loads
        // stdlib; native core bootstrap alone intentionally has no dict update.
        let mut vm = crate::native_fixture::interpreter_with_dictionary_library(profile);
        tcl_test_support::oracle_phase_progress(
            suite,
            engine,
            name,
            "vm-new-complete",
            receipt_start,
        );
        tcl_test_support::oracle_phase_progress(
            suite,
            engine,
            name,
            "profile-complete",
            receipt_start,
        );
        tcl_test_support::oracle_phase_progress(
            suite,
            engine,
            name,
            "compiler-complete",
            receipt_start,
        );
        let completion = vm
            .try_eval_source_bytes(source)
            .unwrap_or_else(|error| panic!("{engine}/{name}: {error:?}"));
        tcl_test_support::oracle_phase_progress(
            suite,
            engine,
            name,
            "eval-complete",
            receipt_start,
        );
        assert_eq!(completion.code.as_int(), code, "{engine}/{name}");
        compare_dictionary_result_window(
            &mut vm,
            &completion.result,
            result,
            result_window,
            engine,
            name,
        );
        tcl_test_support::oracle_phase_progress(
            suite,
            engine,
            name,
            "assertions-complete",
            receipt_start,
        );
        drop(vm);
        tcl_test_support::oracle_phase_progress(
            suite,
            engine,
            name,
            "vm-drop-complete",
            receipt_start,
        );
        *compared += 1;
        tcl_test_support::oracle_row_progress(suite, engine, name, Some(*compared));
    }

    #[derive(Clone, Copy)]
    enum DictionaryResultWindow {
        Complete,
        CompletionAndCallerReadStatus,
    }

    fn compare_dictionary_result_window(
        vm: &mut Vm,
        actual: &Value,
        expected: &[u8],
        window: DictionaryResultWindow,
        engine: &str,
        name: &str,
    ) {
        match window {
            DictionaryResultWindow::Complete => {
                let actual = vm.native_string_bytes(actual).unwrap();
                assert_eq!(actual.as_ref(), expected, "{engine}/{name}");
            }
            DictionaryResultWindow::CompletionAndCallerReadStatus => {
                // Independent native226/C85 and native398/C86 lifetime
                // observations reach final free of the still-selected header.
                // These captured diagnostics/status fields remain fixed public
                // observations; status and outer completion follow the free and
                // certify no storage, runtime completion or effect permission.
                // The later freed-value bytes supply no equivalence premise.
                let expected = vm.new_bytes(expected);
                let expected_fields = vm.list_elements(&expected).unwrap();
                let actual_fields = vm.list_elements(actual).unwrap();
                assert_eq!(expected_fields.len(), 4, "{engine}/{name}");
                assert_eq!(actual_fields.len(), 4, "{engine}/{name}");
                for index in 0..3 {
                    let expected = vm.native_string_bytes(&expected_fields[index]).unwrap();
                    let actual = vm.native_string_bytes(&actual_fields[index]).unwrap();
                    assert_eq!(
                        actual.as_ref(),
                        expected.as_ref(),
                        "{engine}/{name}/{index}"
                    );
                }
            }
        }
    }

    #[derive(Clone, Copy)]
    struct DictionaryProcess<'a> {
        name: &'a str,
        code: i64,
        result: &'a [u8],
        source: &'a [u8],
        profile: &'static tcl_dialect::DialectProfile,
        result_window: DictionaryResultWindow,
    }

    #[test]
    fn dictionary_body_writeback_compares_original_native_observation_windows() {
        // Native proofs: naming.dictionary.update-with-body-and-observer-frontiers;
        // naming.dictionary.c85-original-update-write-error-object-lifetime
        // docs/design/analysis/name-resolution-proofs/dictionary-c85-original-update-write-error-object-lifetime.md
        // naming.dictionary.original-update-write-error-object-lifetime
        // docs/design/analysis/name-resolution-proofs/dictionary-original-update-write-error-object-lifetime.md
        // All 329 captured outer completions remain compared; 327 retain whole
        // public observations. Independent current C85/C86 lifetime observations
        // bound two diagnostic/status comparisons. Those later captured fields are
        // not defined storage, runtime completion or effect guarantees.
        let sources = include_str!("../tests/data/native_dictionary_body/cases.tsv")
            .lines()
            .map(|row| row.split_once('\t').unwrap())
            .collect::<std::collections::BTreeMap<_, _>>();
        let engines = [
            (
                "tcl8.4",
                56,
                include_str!("../tests/data/native_dictionary_body/8.4.20.tsv"),
            ),
            (
                "tcl8.5",
                56,
                include_str!("../tests/data/native_dictionary_body/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                55,
                include_str!("../tests/data/native_dictionary_body/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                53,
                include_str!("../tests/data/native_dictionary_body/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                53,
                include_str!("../tests/data/native_dictionary_body/9.1.0.tsv"),
            ),
            (
                "jim",
                56,
                include_str!("../tests/data/native_dictionary_body/Jim.tsv"),
            ),
        ];
        let mut compared = 0;
        let mut whole_results = 0;
        let mut bounded_results = 0;
        for (engine, count, expected) in engines {
            assert_eq!(expected.lines().count(), count);
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            for row in expected.lines() {
                let mut fields = row.splitn(3, '\t');
                let name = fields.next().unwrap();
                let code: i64 = fields.next().unwrap().parse().unwrap();
                let result = unhex(fields.next().unwrap());
                let source = unhex(sources[name]);
                let result_window = if matches!(engine, "tcl8.5" | "tcl8.6")
                    && name == "compiled-update-dict-write-error"
                {
                    assert_eq!(
                        source.as_slice(),
                        include_bytes!(
                            "../../tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source.tcl"
                        )
                    );
                    bounded_results += 1;
                    DictionaryResultWindow::CompletionAndCallerReadStatus
                } else {
                    whole_results += 1;
                    DictionaryResultWindow::Complete
                };
                compare_dictionary_process(
                    "dictionary-body329",
                    engine,
                    DictionaryProcess {
                        name,
                        code,
                        result: &result,
                        source: &source,
                        profile,
                        result_window,
                    },
                    &mut compared,
                );
            }
        }
        assert_eq!(compared, 329);
        assert_eq!(whole_results, 327);
        assert_eq!(bounded_results, 2);
    }

    #[test]
    fn c85_dictionary_write_error_retains_defined_host_storage() {
        // Native proof: naming.dictionary.c85-original-update-write-error-object-lifetime
        // docs/design/analysis/name-resolution-proofs/dictionary-c85-original-update-write-error-object-lifetime.md
        // Current-build private counters and public bytes are separate evidence.
        // These live-header/value assertions test the VM's safe ownership policy;
        // they do not reproduce or certify native freed-object content.
        let baseline = include_bytes!(
            "../../tcl-registry/tests/data/native_dict_write_error_lifetime226/baseline-current-case9.stdout"
        );
        let observed = include_bytes!(
            "../../tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.stdout"
        );
        assert_eq!(baseline, observed);
        assert_eq!(
            include_str!(
                "../../tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case9.stderr"
            ),
            "DICT_RELEASE_OBSERVER|errors=1|refcount_before_release=1|same_cell=1|defined_scalar=1|final_free=1\n"
        );
        for negative in [
            include_str!(
                "../../tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case0.stderr"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_dict_write_error_lifetime226/observed-case19.stderr"
            ),
        ] {
            assert_eq!(
                negative,
                "DICT_RELEASE_OBSERVER|errors=0|refcount_before_release=-1|same_cell=-1|defined_scalar=-1|final_free=0\n"
            );
        }
        let source = include_bytes!(
            "../../tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source.tcl"
        );
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.5").unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let completion = vm.try_eval_source_bytes(source).unwrap();
        assert_eq!(completion.code, Code::Ok);
        let fields = vm.list_elements(&completion.result).unwrap();
        assert_eq!(fields.len(), 4);
        for (field, expected) in fields[..3].iter().zip([
            b"1".as_slice(),
            b"can't set \"d\": WRITE".as_slice(),
            b"0".as_slice(),
        ]) {
            assert_eq!(vm.native_string_bytes(field).unwrap().as_ref(), expected);
        }
        assert!(fields[3].native_object_is_live());
        assert_eq!(
            vm.native_string_bytes(&fields[3]).unwrap().as_ref(),
            b"k BASE"
        );
        let key = vm.new_bytes(b"k");
        let member = tcl_cmd_core::dict::get(&mut vm, &fields[3], &[key]).unwrap();
        assert!(member.native_object_is_live());
        assert_eq!(vm.native_string_bytes(&member).unwrap().as_ref(), b"BASE");
    }

    #[test]
    fn c86_dictionary_write_error_retains_its_own_defined_host_storage() {
        // naming.dictionary.original-update-write-error-object-lifetime
        // docs/design/analysis/name-resolution-proofs/dictionary-original-update-write-error-object-lifetime.md
        // The native counter window is independent of the VM's safe policy.
        // Neither native freed contents nor original compiled frame ownership
        // are reconstructed from this software header/member control.
        let baseline = include_bytes!(
            "../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/baseline-case9.stdout"
        );
        let observed = include_bytes!(
            "../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case9.stdout"
        );
        assert_eq!(baseline, observed);
        assert_eq!(
            include_str!(
                "../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case9.stderr"
            ),
            "DICT_RELEASE_OBSERVER|errors=1|refcount_before_release=1|same_cell=1|defined_scalar=1|final_free=1\n",
        );
        for control in [
            include_str!(
                "../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case0.stderr"
            ),
            include_str!(
                "../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/8.6.18/observed-case19.stderr"
            ),
        ] {
            assert_eq!(
                control,
                "DICT_RELEASE_OBSERVER|errors=0|refcount_before_release=-1|same_cell=-1|defined_scalar=-1|final_free=0\n"
            );
        }
        let source = include_bytes!(
            "../../tcl-registry/tests/data/native_dict_update_write_error_lifetime398/request/original-source.tcl"
        );
        assert_eq!(source.as_slice(), include_bytes!("../../tcl-registry/tests/data/native_dict_write_error_lifetime226/request/original-source.tcl").as_slice());
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
        let mut vm = crate::native_fixture::interpreter(profile);
        let completion = vm.try_eval_source_bytes(source).unwrap();
        assert_eq!(completion.code, Code::Ok);
        let fields = vm.list_elements(&completion.result).unwrap();
        assert_eq!(fields.len(), 4);
        for (field, expected) in fields[..3].iter().zip([
            b"1".as_slice(),
            b"can't set \"d\": WRITE".as_slice(),
            b"0".as_slice(),
        ]) {
            assert_eq!(vm.native_string_bytes(field).unwrap().as_ref(), expected);
        }
        assert!(fields[3].native_object_is_live());
        assert_eq!(
            vm.native_string_bytes(&fields[3]).unwrap().as_ref(),
            b"k BASE"
        );
        let key = vm.new_bytes(b"k");
        let member = tcl_cmd_core::dict::get(&mut vm, &fields[3], &[key]).unwrap();
        assert!(member.native_object_is_live());
        assert_eq!(vm.native_string_bytes(&member).unwrap().as_ref(), b"BASE");
    }

    // Native proof: naming.variable.dictionary-rmw-command-versus-selected-local-cell
    // docs/design/analysis/name-resolution-proofs/variable.dictionary-rmw-command-versus-selected-local-cell.md
    #[test]
    fn dictionary_command_and_local_cell_publication_match_345_complete_native_processes() {
        const CASES: &str = include_str!("../tests/data/native_dictionary_rmw/cases.tsv");
        let sources = CASES
            .lines()
            .map(|row| row.split_once('\t').unwrap())
            .collect::<std::collections::BTreeMap<_, _>>();
        let engines = [
            (
                "tcl8.4",
                60,
                include_str!("../tests/data/native_dictionary_rmw/8.4.20.tsv"),
            ),
            (
                "tcl8.5",
                57,
                include_str!("../tests/data/native_dictionary_rmw/8.5.19.tsv"),
            ),
            (
                "tcl8.6",
                56,
                include_str!("../tests/data/native_dictionary_rmw/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                56,
                include_str!("../tests/data/native_dictionary_rmw/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                56,
                include_str!("../tests/data/native_dictionary_rmw/9.1.0.tsv"),
            ),
            (
                "jim",
                60,
                include_str!("../tests/data/native_dictionary_rmw/Jim.tsv"),
            ),
        ];
        let mut compared = 0;
        for (engine, count, expected) in engines {
            assert_eq!(expected.lines().count(), count);
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            for row in expected.lines() {
                let mut fields = row.splitn(3, '\t');
                let name = fields.next().unwrap();
                let code: i64 = fields.next().unwrap().parse().unwrap();
                let result = unhex(fields.next().unwrap());
                let source = unhex(sources[name]);
                compare_dictionary_process(
                    "dictionary-rmw345",
                    engine,
                    DictionaryProcess {
                        name,
                        code,
                        result: &result,
                        source: &source,
                        profile,
                        result_window: DictionaryResultWindow::Complete,
                    },
                    &mut compared,
                );
            }
        }
        assert_eq!(compared, 345);
    }
}

#[cfg(test)]
#[path = "cmd_dict/scripted_tests.rs"]
mod scripted_tests;

#[cfg(test)]
mod native_path_publication_tests {
    fn original_result(rows: &str) -> (i64, Vec<u8>) {
        let fields = rows
            .lines()
            .find(|row| row.starts_with("ORIGINAL|"))
            .unwrap()
            .split('|')
            .collect::<Vec<_>>();
        let bytes = fields[2].as_bytes();
        assert_eq!(bytes.len() % 2, 0);
        (
            fields[1].parse().unwrap(),
            bytes
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect(),
        )
    }

    #[test]
    fn original_path_missing_variable_publication_matches_all_15_available_native_windows() {
        // naming.dictionary.original-path-missing-variable-publication
        // docs/design/analysis/name-resolution-proofs/dictionary-original-path-missing-variable-publication.md
        // Each complete original caller result keeps the error, existence,
        // caller read and alias/member publication together. No private object
        // header or C lookup receipt follows from these public observations.
        // C84's three original NA rows are retained independently: this test
        // checks absent purpose, without normalising a raw Return to C API Ok.
        let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
        let cases = [("missing-root-failed-path-unset", include_bytes!("../../tcl-registry/tests/data/native_dictionary_path_publication319/missing-root-failed-path-unset.tcl").as_slice(), [include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/8.4.20/missing-root-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/8.5.19/missing-root-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/8.6.18/missing-root-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/9.0.4/missing-root-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/9.1.0/missing-root-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/jim/missing-root-failed-path-unset/stdout")]),
("missing-member-failed-path-unset", include_bytes!("../../tcl-registry/tests/data/native_dictionary_path_publication319/missing-member-failed-path-unset.tcl").as_slice(), [include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/8.4.20/missing-member-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/8.5.19/missing-member-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/8.6.18/missing-member-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/9.0.4/missing-member-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/9.1.0/missing-member-failed-path-unset/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/jim/missing-member-failed-path-unset/stdout")]),
("linked-root-path-set", include_bytes!("../../tcl-registry/tests/data/native_dictionary_path_publication319/linked-root-path-set.tcl").as_slice(), [include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/8.4.20/linked-root-path-set/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/8.5.19/linked-root-path-set/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/8.6.18/linked-root-path-set/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/9.0.4/linked-root-path-set/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/9.1.0/linked-root-path-set/stdout"),
include_str!("../../tcl-registry/tests/data/native_dictionary_path_publication319/jim/linked-root-path-set/stdout")])];
        let mut compared = 0;
        let mut unsupported = 0;
        for (case, source, columns) in cases {
            for (engine, column) in providers.iter().zip(columns) {
                let (expected_code, expected_result) = original_result(column);
                let profile =
                    tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
                if *engine == "tcl8.4" {
                    assert_eq!(expected_code, 0, "{engine}/{case} native API boundary");
                    assert_eq!(expected_result, b"NOT_APPLICABLE");
                    assert!(
                        tcl_registry::InvocationDialect::of_profile(profile)
                            .native_dictionary_path_publication()
                            .is_none()
                    );
                    unsupported += 1;
                    continue;
                }
                let mut vm = crate::native_fixture::interpreter(profile);
                let completion = vm
                    .try_eval_source_bytes(source)
                    .unwrap_or_else(|error| panic!("{engine}/{case}: {error:?}"));
                assert_eq!(completion.code.as_int(), expected_code, "{engine}/{case}");
                let actual = vm.native_name_operand_bytes(&completion.result).unwrap();
                assert_eq!(actual.as_ref(), expected_result, "{engine}/{case}");
                assert!(vm.refused_completion().is_none(), "{engine}/{case}");
                compared += 1;
            }
        }
        assert_eq!(compared, 15);
        assert_eq!(unsupported, 3);
    }
}
