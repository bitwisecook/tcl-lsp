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

//! `info object` and `info class`, and the object lookups the `TclOO` opcodes share with them.

use super::configurable_properties::{slot_ref_class, slot_ref_obj};
use super::{
    Method, Step, StepRole, display_oo_bytes, err_code, method_list_value, native_context,
    object_message, original_name_operands, resolve_class_value, resolve_object_value,
};
use crate::command::Param;
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use std::collections::BTreeSet;
use tcl_registry::commands::tcl::{
    InfoOoEnsembleKind, InfoOoPropertiesOption, info_oo_subcommands,
    resolve_info_oo_properties_option_original,
};
use tcl_runtime_api::{Code, Completion, OoId};

/// Whether `name` (as written at the call site) resolves to a `TclOO` object —
/// `info object isa object` and the `tclooIsObject` opcode, neither of which
/// ever errors.
pub(crate) fn is_object(vm: &mut Vm, name: &Value) -> Result<bool, Completion<Value>> {
    let present = resolve_object_value(vm, name)?.is_some();
    if !present {
        object_lookup_failure(vm, name)?;
    }
    Ok(present)
}

/// Resolve an original native object command operand at the current token.
/// Missing binding is a guest lookup failure; unavailable lookup remains a host refusal.
pub(crate) fn object_key(vm: &mut Vm, name: &Value) -> Result<OoId, Completion<Value>> {
    if let Some(object) = resolve_object_value(vm, name)? {
        return Ok(object);
    }
    Err(object_lookup_failure(vm, name)?)
}

fn object_lookup_failure(
    vm: &mut Vm,
    name: &Value,
) -> Result<Completion<Value>, Completion<Value>> {
    original_name_operands::run_result(vm, None, |vm, original_names| {
        let original = original_names
            .read(vm, name)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        let original = tcl_core_types::c_string_extent(&original);
        let dialect = vm.actual_native_invocation_dialect();
        let protocol = dialect
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or_else(|| {
                vm.refuse_host_command("native TclOO object error producer".to_owned())
            })?;
        let materialization = dialect.native_string_materialization(None).ok_or_else(|| {
            vm.refuse_host_command("native TclOO object error String producer".to_owned())
        })?;
        let mut message = original.to_vec();
        message.extend_from_slice(b" does not refer to an object");
        let message = Value::new_native_string_bytes(message);
        message
            .retain_native_string_representation(materialization)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
        vm.adopt_native_interp_result(message.clone())
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error.into()))?;
        let code = Value::native_list_constructor(
            [b"TCL".as_slice(), b"LOOKUP", b"OBJECT", original]
                .into_iter()
                .map(Value::new_native_string_bytes)
                .collect(),
            protocol,
        );
        vm.retain_return_error_code(Some(&code), false);
        Ok(Completion::new_error_metadata(
            Code::Error,
            message,
            crate::command::options_dict(Code::Error, 0, &[("-errorcode", code)]),
        ))
    })
}

/// The class command name of the object `key` (`info object class`, the
/// `tclooClass` opcode). `key` must be a resolved object key — see
/// [`object_key`].
pub(crate) fn object_class_name(vm: &Vm, key: OoId) -> Value {
    native_context::object_name(vm, vm.oo.objects[&key].class)
}

/// The instance namespace of the object `key` (`info object namespace`, the
/// `tclooNamespace` opcode). `key` must be a resolved object key — see
/// [`object_key`].
pub(crate) fn object_namespace_name(vm: &mut Vm, key: OoId) -> Result<Value, Completion<Value>> {
    let Some(namespace) = vm.oo.objects[&key].namespace_token else {
        return Err(
            vm.refuse_host_command("original object namespace incarnation unavailable".to_owned())
        );
    };
    vm.native_namespace_result_object(
        namespace,
        tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::ObjectNamespace,
    )
    .map_err(|error| vm.refuse_host_command(error.to_string()))
}

/// Produce the actual wide creation epoch without guessing an overflowed value.
pub(crate) fn object_creation_id(vm: &mut Vm, key: OoId) -> Result<Value, Completion<Value>> {
    let epoch = i64::try_from(vm.oo.objects[&key].creation_id).map_err(|_| {
        vm.refuse_host_command(
            "original object creation epoch exceeds native wide integer".to_owned(),
        )
    })?;
    Ok(Value::int(epoch))
}

fn variable_info_options(
    vm: &mut Vm,
    kind: InfoOoEnsembleKind,
    rest: &[Value],
) -> Result<bool, Completion<Value>> {
    original_name_operands::run_result(vm, None, |vm, original_names| {
        let selected = tcl_registry::native_tcloo_info::NativeTclooVariableInfoProtocol::select(
            vm.actual_native_invocation_dialect(),
        )
        .ok_or_else(|| {
            vm.refuse_host_command("TclOO variable info protocol is unavailable".to_owned())
        })?;
        if !selected.accepts_count(rest.len()) {
            return Err(crate::command::native_wrong_args(vm, selected.usage(kind)));
        }
        let Some(option) = rest.get(1) else {
            return Ok(false);
        };
        let original = original_names
            .read(vm, option)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        selected
            .private_option(&original)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
    })
}

/// `info object subcommand object ?arg…?`.
pub(crate) fn info_object(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    original_name_operands::run(vm, None, |vm, original_names| {
        let Some((sub, rest)) = args.split_first() else {
            return crate::command::native_wrong_args(
                vm,
                "info object subcommand object ?arg ...?",
            );
        };
        let sub = match original_names.read(vm, sub) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let sub = tcl_core_types::c_string_extent(&sub);
        let subcommands = info_oo_subcommands(InfoOoEnsembleKind::Object, vm.runtime_version());
        let sub = match subcommands.resolve(sub) {
            Ok(sub) => sub,
            Err(message) => return err(message),
        };
        // `isa` is the odd one out: `info object isa category object ?arg?` — the
        // object comes *after* the category.
        if sub == "isa" {
            return info_object_isa(vm, rest);
        }
        let private_variables = if sub == "variables" {
            match variable_info_options(vm, InfoOoEnsembleKind::Object, rest) {
                Ok(private) => private,
                Err(completion) => return completion,
            }
        } else {
            false
        };
        let Some(obj_arg) = rest.first() else {
            if sub == "methods" {
                return crate::command::native_wrong_args(
                    vm,
                    tcl_registry::native_tcloo_info::NativeTclooMethodInfoProtocol::usage(
                        InfoOoEnsembleKind::Object,
                    ),
                );
            }
            return err(format!(
                "wrong # args: should be \"info object {sub} object ?arg ...?\""
            ));
        };
        let obj = match object_key(vm, obj_arg) {
            Ok(k) => k,
            Err(c) => return c,
        };
        if let Err(error) = original_names.select_target(vm, false, obj) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let extra = &rest[1..];
        match sub {
            "class" => {
                if let Some(cls) = extra.first() {
                    let class = match resolve_class_value(vm, cls) {
                        Ok(class) => class,
                        Err(completion) => return completion,
                    };
                    if let Err(error) = original_names.check_target(vm) {
                        return crate::command::completion_from_cmd_error(vm, error);
                    }
                    let is_a = class.is_some_and(|class| vm.oo.is_a(obj, class));
                    ok(Value::int(i64::from(is_a)))
                } else {
                    ok(object_class_name(vm, obj))
                }
            }
            "namespace" => object_namespace_name(vm, obj).map_or_else(|completion| completion, ok),
            "creationid" => object_creation_id(vm, obj).map_or_else(|completion| completion, ok),
            "mixins" => ok(Value::list(
                vm.oo.objects[&obj]
                    .mixins
                    .iter()
                    .map(|m| Value::from_native_string_bytes(display_oo_bytes(vm, *m)))
                    .collect(),
            )),
            "variables" => {
                let o = &vm.oo.objects[&obj];
                let names = if private_variables {
                    &o.private_variables
                } else {
                    &o.variables
                };
                ok(Value::list(
                    names.iter().map(|entry| entry.original.clone()).collect(),
                ))
            }
            "vars" => info_object_variable_names(vm, obj, extra),
            "methods" => info_object_methods(vm, obj, extra),
            "properties" => info_object_properties(vm, obj, extra),
            "filters" => ok(filter_names(&vm.oo.objects[&obj].filters)),
            "call" => info_call(vm, &vm.oo.object_precedence(obj), extra, false),
            _ => err(format!("unsupported info object subcommand \"{sub}\"")),
        }
    })
}

fn info_object_variable_names(vm: &mut Vm, obj: OoId, extra: &[Value]) -> Completion<Value> {
    original_name_operands::run(vm, Some((false, obj)), |vm, original_names| {
        let namespace = vm.definition_namespace_token(&vm.oo.objects[&obj].ns.clone());
        let pattern = match extra
            .first()
            .map(|value| original_names.read(vm, value))
            .transpose()
        {
            Ok(pattern) => pattern,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let Some(policy) = vm.name_policy_protocol() else {
            return vm.refuse_host_command(
                "TclOO variable enumeration protocol is unavailable".to_owned(),
            );
        };
        let matcher = tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy);
        let mut names = Vec::new();
        for name in tcl_runtime_api::Namespaces::vars_in_bytes(vm, namespace) {
            if !vm.tcloo_storage_variable_is_defined(namespace, &name) {
                continue;
            }
            if let Some(pattern) = pattern.as_ref() {
                match matcher.match_name_pattern(
                    tcl_syntax::native_glob::NativeNameGlobPurpose::OoObjectNamesSearch,
                    pattern,
                    &name,
                ) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(error) => return vm.refuse_host_command(error.to_string()),
                }
            }
            names.push(Value::from_native_string_bytes(name));
        }
        ok(Value::list(names))
    })
}

/// `info object properties object ?-all? ?-readable|-writable?` (TIP 558).
///
/// Reports the readable (default) or writable properties declared directly on
/// the object; `-all` unions in the object's full precedence (its mixins, then
/// its class MRO). Sorted and duplicate-free.
fn info_object_properties(vm: &mut Vm, obj: OoId, extra: &[Value]) -> Completion<Value> {
    let (all, writable) = match info_properties_options(vm, extra) {
        Ok(flags) => flags,
        Err(error) => return error,
    };
    if all
        && vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .is_some()
    {
        return match vm.native_all_property_header(obj, false, writable) {
            Ok(header) => ok(header.value().clone()),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        };
    }
    if vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        return match vm.native_declared_property_header(obj, false, writable) {
            Ok(header) => ok(header),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        };
    }
    let mut names: BTreeSet<String> = BTreeSet::new();
    if all {
        for step in vm.oo.object_precedence(obj) {
            if step.is_object {
                if let Some(o) = vm.oo.objects.get(&step.provider) {
                    names.extend(slot_ref_obj(o, writable).iter().cloned());
                }
            } else if let Some(c) = vm.oo.classes.get(&step.provider) {
                names.extend(slot_ref_class(c, writable).iter().cloned());
            }
        }
    } else if let Some(o) = vm.oo.objects.get(&obj) {
        names.extend(slot_ref_obj(o, writable).iter().cloned());
    }
    ok(Value::list(
        names
            .into_iter()
            .map(|key| Value::new_native_string_bytes(vm.oo.method_name_bytes(&key)))
            .collect(),
    ))
}

/// `info object isa`'s category word, in C table order (`categories[]`,
/// `tclOOInfo.c`): `Tcl_GetIndexFromObj(…, "category", 0)`, so `cl`/`ob`/`t`
/// abbreviate, `m` is ambiguous (metaclass/mixin), and the empty word — a
/// prefix of all five — is `ambiguous category ""`. The Oxford comma before
/// `or` is `Tcl_GetIndexFromObj`'s, not `TclOO`'s method-list style.
const ISA_CATEGORIES: tcl_cmd_core::prefix::OptionTable<'static> =
    tcl_cmd_core::prefix::OptionTable::abbreviating(
        "category",
        &["class", "metaclass", "mixin", "object", "typeof"],
    );

/// `info object isa category object ?arg?`.
fn info_object_isa(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    // C checks the arity in two stages (`InfoObjectIsACmd`, tclOOInfo.c).
    // First, `category objName` must both be present — *before* the category
    // word is resolved, so `info object isa object` is this message and not a
    // category error.
    let (Some(kind), Some(obj_arg)) = (rest.first(), rest.get(1)) else {
        return crate::command::native_wrong_args(vm, "info object isa category objName ?arg ...?");
    };
    let kind = match vm.native_static_option_index_checked(
        kind,
        ISA_CATEGORIES.names(),
        false,
        "category",
    ) {
        Ok(i) => ISA_CATEGORIES.names()[i],
        Err(e) => return crate::command::completion_from_cmd_error(vm, e),
    };
    // Then each category pins an *exact* count — C's second stage,
    // `Tcl_WrongNumArgs(interp, 2, objv, …)`, whose noun carries the resolved
    // category word (an index-typed argument prints as its table entry). So
    // `info object isa t o` is `"info object isa typeof objName className"`,
    // and a trailing extra word is an error rather than being ignored.
    let (want, tail) = match kind {
        "mixin" | "typeof" => (3, " objName className"),
        // class / metaclass / object.
        _ => (2, " objName"),
    };
    if rest.len() != want {
        return err(format!(
            "wrong # args: should be \"info object isa {kind}{tail}\""
        ));
    }
    let obj = match resolve_object_value(vm, obj_arg) {
        Ok(object) => object,
        Err(completion) => return completion,
    };
    // `isa object` on a non-object is a plain false, not an error — the same
    // test the `tclooIsObject` opcode this form compiles to performs.
    if obj.is_none()
        && let Err(completion) = object_lookup_failure(vm, obj_arg)
    {
        return completion;
    }
    if kind == "object" {
        return ok(Value::int(i64::from(obj.is_some())));
    }
    let Some(obj) = obj else {
        return ok(Value::int(0));
    };
    let res = match kind {
        "class" => vm.oo.classes.contains_key(&obj),
        "metaclass" => vm.oo.is_metaclass(obj),
        "typeof" | "mixin" => {
            let class = match rest.get(2) {
                Some(original) => match resolve_class_value(vm, original) {
                    Ok(class) => class,
                    Err(completion) => return completion,
                },
                None => None,
            };
            class.is_some_and(|class| {
                if kind == "typeof" {
                    vm.oo.is_a(obj, class)
                } else {
                    vm.oo.objects[&obj].mixins.contains(&class)
                }
            })
        }
        // Unreachable: `ISA_CATEGORIES` has exactly these five names.
        _ => false,
    };
    ok(Value::int(i64::from(res)))
}

/// `InfoClassDefnNsCmd`: the class's (or, with `-instance`, its instances')
/// definition namespace, or nothing when none was set.
fn info_class_definition_namespace(vm: &mut Vm, cls: OoId, extra: &[Value]) -> Completion<Value> {
    const KINDS: &[&str] = &["-class", "-instance"];
    let instance = match extra.first() {
        None => false,
        Some(kind) => match vm.native_static_option_index_checked(kind, KINDS, false, "kind") {
            Ok(index) => index == 1,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        },
    };
    let class = &vm.oo.classes[&cls];
    let namespace = if instance {
        class.object_definition_namespace
    } else {
        class.definition_namespace
    };
    ok(namespace.map_or_else(Value::empty, |namespace| {
        Value::from_native_string_bytes(tcl_syntax::naming::root_unrooted_key_bytes(
            vm.ns_name_bytes(namespace).as_bytes(),
        ))
    }))
}

/// `info class subcommand class ?arg…?`.
pub(crate) fn info_class(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    original_name_operands::run(vm, None, |vm, original_names| {
        let Some((sub, rest)) = args.split_first() else {
            return crate::command::native_wrong_args(vm, "info class subcommand class ?arg ...?");
        };
        let sub = match original_names.read(vm, sub) {
            Ok(bytes) => bytes,
            Err(error) => return crate::command::completion_from_cmd_error(vm, error),
        };
        let sub = tcl_core_types::c_string_extent(&sub);
        let subcommands = info_oo_subcommands(InfoOoEnsembleKind::Class, vm.runtime_version());
        let sub = match subcommands.resolve(sub) {
            Ok(sub) => sub,
            Err(message) => return err(message),
        };
        let private_variables = if sub == "variables" {
            match variable_info_options(vm, InfoOoEnsembleKind::Class, rest) {
                Ok(private) => private,
                Err(completion) => return completion,
            }
        } else {
            false
        };
        if sub == "definitionnamespace" && !(1..=2).contains(&rest.len()) {
            return err(
                "wrong # args: should be \"info class definitionnamespace className ?kind?\"",
            );
        }
        let Some(cls_arg) = rest.first() else {
            if sub == "methods" {
                return crate::command::native_wrong_args(
                    vm,
                    tcl_registry::native_tcloo_info::NativeTclooMethodInfoProtocol::usage(
                        InfoOoEnsembleKind::Class,
                    ),
                );
            }
            return err(format!(
                "wrong # args: should be \"info class {sub} class ?arg ...?\""
            ));
        };
        let cls = match object_key(vm, cls_arg) {
            Ok(cls) => cls,
            Err(completion) => return completion,
        };
        if !vm.oo.classes.contains_key(&cls) {
            return err(object_message(vm, cls, b"\"", b"\" is not a class"));
        }
        if let Err(error) = original_names.select_target(vm, true, cls) {
            return crate::command::completion_from_cmd_error(vm, error);
        }
        let extra = &rest[1..];
        match sub {
            "superclasses" => ok(Value::list(
                vm.oo.classes[&cls]
                    .supers
                    .iter()
                    .map(|s| Value::from_native_string_bytes(display_oo_bytes(vm, *s)))
                    .collect(),
            )),
            "mixins" => ok(Value::list(
                vm.oo.classes[&cls]
                    .mixins
                    .iter()
                    .map(|m| Value::from_native_string_bytes(display_oo_bytes(vm, *m)))
                    .collect(),
            )),
            "subclasses" | "instances" => info_class_related_names(vm, cls, extra, sub),
            "methods" => info_class_methods(vm, cls, extra),
            "constructor" => {
                lifecycle_definition(vm.oo.classes[&cls].constructor.get().as_ref(), true)
            }
            "destructor" => {
                lifecycle_definition(vm.oo.classes[&cls].destructor.get().as_ref(), false)
            }
            "variables" => {
                let c = &vm.oo.classes[&cls];
                let names = if private_variables {
                    &c.private_variables
                } else {
                    &c.variables
                };
                ok(Value::list(
                    names.iter().map(|entry| entry.original.clone()).collect(),
                ))
            }
            "properties" => info_class_properties(vm, cls, extra),
            "filters" => ok(filter_names(&vm.oo.classes[&cls].filters)),
            "call" => info_call(vm, &vm.oo.class_linear_of(cls), extra, true),
            "definitionnamespace" => info_class_definition_namespace(vm, cls, extra),
            _ => err(format!("unsupported info class subcommand \"{sub}\"")),
        }
    })
}

fn lifecycle_definition(method: Option<&Method>, parameters: bool) -> Completion<Value> {
    match method {
        Some(method)
            if method.intrinsic.is_some()
                || method.visibility_only
                || method.forward.is_some()
                || method.property.is_some() =>
        {
            err_code(
                "definition not available for this kind of method",
                &["TCL", "OO", "METHOD_TYPE"],
            )
        }
        Some(method) if parameters => ok(Value::list(vec![
            params_value(&method.parameters.values),
            method.body_src.clone(),
        ])),
        Some(method) => ok(method.body_src.clone()),
        None => ok(Value::empty()),
    }
}

fn info_class_related_names(
    vm: &mut Vm,
    cls: OoId,
    extra: &[Value],
    sub: &str,
) -> Completion<Value> {
    original_name_operands::run(vm, Some((true, cls)), |vm, original_names| {
        let pattern = match extra.first() {
            Some(original) => match original_names.read(vm, original) {
                Ok(bytes) => Some(bytes),
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            },
            None => None,
        };
        let tokens: Vec<_> = if sub == "subclasses" {
            vm.oo
                .classes
                .iter()
                .filter(|(key, class)| **key != cls && class.supers.contains(&cls))
                .map(|(key, _)| *key)
                .collect()
        } else {
            vm.oo
                .objects
                .iter()
                .filter(|(key, object)| object.class == cls && !vm.oo.classes.contains_key(*key))
                .map(|(key, _)| *key)
                .collect()
        };
        let Some(policy) = vm.name_policy_protocol() else {
            return vm
                .refuse_host_command("native TclOO enumeration protocol unavailable".to_owned());
        };
        let matcher = tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy);
        let mut names = Vec::new();
        for token in tokens {
            let name = display_oo_bytes(vm, token);
            if let Some(pattern) = pattern.as_ref() {
                match matcher.match_name_pattern(
                    tcl_syntax::native_glob::NativeNameGlobPurpose::OoObjectNamesSearch,
                    pattern,
                    &name,
                ) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(error) => return vm.refuse_host_command(error.to_string()),
                }
            }
            names.push(name);
        }
        names.sort();
        ok(Value::list(
            names
                .into_iter()
                .map(Value::from_native_string_bytes)
                .collect(),
        ))
    })
}

/// `info class properties class ?-all? ?-readable|-writable?` (TIP 558).
///
/// Reports the readable (default) or writable property names declared on the
/// class; `-all` unions in every class up the MRO (mixins + superclasses). The
/// result is sorted and duplicate-free.
fn info_class_properties(vm: &mut Vm, cls: OoId, extra: &[Value]) -> Completion<Value> {
    let (all, writable) = match info_properties_options(vm, extra) {
        Ok(flags) => flags,
        Err(error) => return error,
    };
    if all
        && vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .is_some()
    {
        return match vm.native_all_property_header(cls, true, writable) {
            Ok(header) => ok(header.value().clone()),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        };
    }
    if vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        return match vm.native_declared_property_header(cls, true, writable) {
            Ok(header) => ok(header),
            Err(error) => crate::command::completion_from_cmd_error(vm, error),
        };
    }
    let mut names: BTreeSet<String> = BTreeSet::new();
    if all {
        for step in vm.oo.class_linear_of(cls) {
            if let Some(c) = vm.oo.classes.get(&step.provider) {
                names.extend(slot_ref_class(c, writable).iter().cloned());
            }
        }
    } else if let Some(c) = vm.oo.classes.get(&cls) {
        names.extend(slot_ref_class(c, writable).iter().cloned());
    }
    ok(Value::list(
        names
            .into_iter()
            .map(|key| Value::new_native_string_bytes(vm.oo.method_name_bytes(&key)))
            .collect(),
    ))
}

fn info_properties_options(
    vm: &mut Vm,
    extra: &[Value],
) -> Result<(bool, bool), Completion<Value>> {
    let (mut all, mut writable) = (false, false);
    for value in extra {
        match resolve_info_oo_properties_option_original(vm, value) {
            Ok(InfoOoPropertiesOption::All) => all = true,
            Ok(InfoOoPropertiesOption::Readable) => writable = false,
            Ok(InfoOoPropertiesOption::Writable) => writable = true,
            Err(error) => return Err(crate::command::completion_from_cmd_error(vm, error)),
        }
    }
    Ok((all, writable))
}

fn method_info_options(
    vm: &mut Vm,
    extra: &[Value],
) -> Result<tcl_registry::native_tcloo_info::NativeTclooMethodInfoSelection, Completion<Value>> {
    let selected = tcl_registry::native_tcloo_info::NativeTclooMethodInfoProtocol::select(
        vm.actual_native_invocation_dialect(),
    )
    .ok_or_else(|| vm.refuse_host_command("TclOO method info protocol is unavailable".into()))?;
    selected
        .parse_original(vm, extra)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))
}

fn info_object_methods(vm: &mut Vm, obj: OoId, extra: &[Value]) -> Completion<Value> {
    let selected = match method_info_options(vm, extra) {
        Ok(selected) => selected,
        Err(error) => return error,
    };
    let names = if selected.recurse() {
        vm.oo.visible_methods(obj, selected.includes_unexported())
    } else {
        let object = &vm.oo.objects[&obj];
        object
            .methods
            .iter()
            .filter(|(name, method)| {
                let method = method.borrow();
                let public = !object.unexported.contains(*name)
                    && (object.exported.contains(*name) || method.exported);
                !method.visibility_only && selected.matches_local(public, false, method.private)
            })
            .map(|(name, _)| name.clone())
            .collect()
    };
    ok(method_list_value(vm, names))
}

/// The original options are selected after the actual class lookup.
fn info_class_methods(vm: &mut Vm, cls: OoId, extra: &[Value]) -> Completion<Value> {
    let selected = match method_info_options(vm, extra) {
        Ok(selected) => selected,
        Err(error) => return error,
    };
    let mut names = BTreeSet::new();
    if selected.recurse() {
        for step in vm.oo.class_linear_of(cls) {
            if let Some(class) = vm.oo.classes.get(&step.provider) {
                for (name, method) in class.methods.iter() {
                    let method = method.borrow();
                    if !method.visibility_only
                        && !method.private
                        && (selected.includes_unexported() || vm.oo.step_exported(&step, name))
                    {
                        names.insert(name.clone());
                    }
                }
            }
        }
    } else {
        let class = &vm.oo.classes[&cls];
        for (name, method) in class.methods.iter() {
            let method = method.borrow();
            let public = !class.unexported.contains(name)
                && (class.exported.contains(name) || method.exported);
            if !method.visibility_only && selected.matches_local(public, false, method.private) {
                names.insert(name.clone());
            }
        }
    }
    ok(method_list_value(vm, names))
}

/// Render a param list as an `info class constructor`-style arg spec (`args` is
/// already the last element when present).
fn params_value(params: &[Param]) -> Value {
    Value::list(
        params
            .iter()
            .map(|p| match &p.default {
                Some(d) => {
                    Value::list(vec![Value::from_string_bytes(p.name.as_bytes()), d.clone()])
                }
                None => Value::from_string_bytes(p.name.as_bytes()),
            })
            .collect(),
    )
}

fn filter_names(names: &[tcl_core_types::NameBytes]) -> Value {
    Value::list(
        names
            .iter()
            .map(|n| Value::from_native_string_bytes(n.as_bytes().to_vec()))
            .collect(),
    )
}

/// Render a call chain as `info … call` / `self call` report it: one
/// `{callType name declarer methodType}` element per step, then — for an
/// `unknown` chain, or a direct call of `unknown` — `oo::object`'s built-in
/// handler that terminates it.
pub(super) fn render_call_chain(
    vm: &Vm,
    chain: &[Step],
    method: &str,
    unknown_chain: bool,
) -> Vec<Value> {
    let bytes =
        |name: &str| Value::from_native_string_bytes(vm.oo.method_name_bytes(name).to_vec());
    let mut out: Vec<Value> = chain
        .iter()
        .map(|step| {
            let (call_type, name) = match &step.role {
                StepRole::Filter(f) => ("filter", f.as_str()),
                StepRole::Unknown => ("unknown", "unknown"),
                StepRole::Method => ("method", method),
            };
            let record = step
                .method_owner
                .as_ref()
                .map(|owner| owner.borrow().clone())
                .or_else(|| vm.oo.step_method(step, name));
            let method_type = match record {
                Some(m) if m.forward.is_some() => Value::string("forward"),
                Some(m) if m.intrinsic.is_some() => {
                    let mut text = b"core method: \"".to_vec();
                    text.extend_from_slice(vm.oo.method_name_bytes(name));
                    text.push(b'"');
                    Value::from_native_string_bytes(text)
                }
                _ => Value::string("method"),
            };
            let declarer = if step.is_object {
                Value::string("object")
            } else {
                Value::from_native_string_bytes(display_oo_bytes(vm, step.provider))
            };
            Value::list(vec![
                Value::string(call_type),
                bytes(name),
                declarer,
                method_type,
            ])
        })
        .collect();
    if (unknown_chain || method == "unknown")
        && let Some(root) = vm.oo.object_root
    {
        out.push(Value::list(vec![
            Value::string(if unknown_chain { "unknown" } else { "method" }),
            Value::string("unknown"),
            Value::from_native_string_bytes(display_oo_bytes(vm, root)),
            Value::string("core method: \"unknown\""),
        ]));
    }
    out
}

/// `info object call obj method` / `info class call cls method`: the chain a
/// call would run — filters, then the implementations, or the `unknown`
/// handlers when there are none.
fn info_call(vm: &mut Vm, precedence: &[Step], extra: &[Value], class: bool) -> Completion<Value> {
    let [method] = extra else {
        let usage = if class {
            "info class call className methodName"
        } else {
            "info object call objName methodName"
        };
        return crate::command::native_wrong_args(vm, usage);
    };
    let method = match super::native_method_key(vm, method) {
        Ok(method) => method,
        Err(completion) => return completion,
    };
    let mut chain = vm.oo.active_filters(precedence);
    let steps: Vec<Step> = precedence
        .iter()
        .filter(|s| {
            vm.oo
                .step_method(s, &method)
                .is_some_and(|m| !m.visibility_only)
        })
        .cloned()
        .collect();
    let unknown = steps.is_empty();
    if unknown {
        chain.extend(vm.oo.unknown_handlers(precedence));
    } else {
        chain.extend(steps);
    }
    let name = if unknown { "unknown" } else { method.as_str() };
    ok(Value::list(render_call_chain(vm, &chain, name, unknown)))
}
