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

//! TIP 558 configurable properties: the property slots, the `property` definition command, the `configure` method and accessor calls.

use super::definition::{active_target, build_method};
use super::method_invocation::oo_invoke;
use super::{
    Class, Object, display_oo_bytes, err_code, native_properties, object_message,
    original_name_operands,
};
use crate::interp::{Vm, err, ok};
use crate::value::Value;
use std::collections::BTreeSet;
use tcl_registry::commands::tcl::TclOoPropertyKind;
use tcl_runtime_api::{Code, Completion, OoId};

fn property_slot_operation<'a>(
    vm: &mut Vm,
    args: &'a [Value],
    version: tcl_dialect::TclVersion,
) -> Result<(tcl_registry::definer::SlotOp, &'a [Value]), Completion<Value>> {
    let target = active_target(vm);
    original_name_operands::run_result(vm, target, |vm, original_names| {
        Ok(if let Some((first, rest)) = args.split_first() {
            let bytes = match original_names.read(vm, first) {
                Ok(bytes) => bytes,
                Err(error) => return Err(crate::command::completion_from_cmd_error(vm, error)),
            };
            if bytes.first() == Some(&b'-') {
                let selected = match tcl_syntax::naming::NativeNameProtocol::C(version)
                    .oo_method_input(&bytes)
                {
                    Ok(selected) => selected,
                    Err(error) => return Err(vm.refuse_host_command(error.to_string())),
                };
                let operation = match tcl_registry::definer::SlotOp::resolve_runtime(
                    selected.selected(),
                    version,
                ) {
                    Ok(operation) => operation,
                    Err(message) => return Err(err(message)),
                };
                (operation, rest)
            } else {
                (tcl_registry::definer::SlotOp::Set, args)
            }
        } else {
            (tcl_registry::definer::SlotOp::Set, args)
        })
    })
}

/// A TIP 558 property slot (`::oo::configuresupport::{obj,}{readable,writable}properties`).
///
/// Callable only inside an `oo::define`/`oo::objdefine` body; applies a slot
/// operation to the target's readable or writable property set. Supported ops
/// are `-set` (replace, also the default for bare names), `-append`, `-remove`,
/// and `-clear`; every op keeps the set sorted and duplicate-free.
pub(super) fn property_slot(
    vm: &mut Vm,
    args: &[Value],
    obj: bool,
    writable: bool,
) -> Completion<Value> {
    let target = active_target(vm);
    original_name_operands::run(vm, target, |vm, original_names| {
        let Some((is_class, target)) = active_target(vm) else {
            return err_code(
                "this command may only be called from within the context of an ::oo::define or ::oo::objdefine command",
                &["TCL", "OO", "MONKEY_BUSINESS"],
            );
        };
        let Some(tcl_syntax::naming::NativeNameProtocol::C(version)) =
            vm.actual_native_name_protocol()
        else {
            return vm.refuse_host_command("native property slot naming unavailable".to_owned());
        };
        let (operation, names) = match property_slot_operation(vm, args, version) {
            Ok(selected) => selected,
            Err(completion) => return completion,
        };
        if matches!(
            operation,
            tcl_registry::definer::SlotOp::AppendIfNew | tcl_registry::definer::SlotOp::Prepend
        ) {
            return vm
                .refuse_host_command("native property slot operation is unmodelled".to_owned());
        }
        if vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .is_some()
        {
            let class = is_class && !obj;
            let names = match native_properties::set_property_slot(
                vm, target, class, writable, operation, names,
            ) {
                Ok(names) => names,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            if let Err(error) = original_names.check_target(vm) {
                return crate::command::completion_from_cmd_error(vm, error);
            }
            if class {
                if let Some(entry) = vm.oo.classes.get_mut(&target) {
                    *slot_of_class(entry, writable) = names;
                }
            } else if let Some(entry) = vm.oo.objects.get_mut(&target) {
                *slot_of_obj(entry, writable) = names;
            }
            return ok(Value::empty());
        }
        let mut new_names = Vec::new();
        for original in names {
            let bytes = match original_names.read(vm, original) {
                Ok(bytes) => bytes,
                Err(error) => return crate::command::completion_from_cmd_error(vm, error),
            };
            new_names.push(
                vm.oo
                    .intern_method_key(tcl_core_types::NameBytes::from(bytes.as_ref())),
            );
        }

        // Resolve the target set: the per-object slot writes the object facet, the
        // class slot writes the class facet (a class is also an object).
        let set = if obj || !is_class {
            vm.oo
                .objects
                .get_mut(&target)
                .map(|o| slot_of_obj(o, writable))
        } else {
            vm.oo
                .classes
                .get_mut(&target)
                .map(|c| slot_of_class(c, writable))
        };
        let Some(set) = set else {
            return err(object_message(
                vm,
                target,
                b"",
                b" does not refer to an object",
            ));
        };
        let mut values: Vec<_> = set.iter().cloned().collect();
        tcl_registry::definer::SlotSpec {
            default_op: tcl_registry::definer::SlotOp::Set,
            dedup: true,
        }
        .apply_values(&mut values, operation, &new_names);
        *set = values.into_iter().collect();
        ok(Value::empty())
    })
}

fn slot_of_class(c: &mut Class, writable: bool) -> &mut BTreeSet<String> {
    if writable {
        &mut c.writable_properties
    } else {
        &mut c.readable_properties
    }
}

fn slot_of_obj(o: &mut Object, writable: bool) -> &mut BTreeSet<String> {
    if writable {
        &mut o.writable_properties
    } else {
        &mut o.readable_properties
    }
}

/// The readable (`writable == false`) or writable property names visible to
/// `configure` on `obj_key`: the union of the object's own slots and every
/// class slot along its MRO (`info object properties -all`), sorted & unique.
fn configure_props(vm: &Vm, obj_key: OoId, writable: bool) -> Vec<String> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for step in vm.oo.object_precedence(obj_key) {
        if step.is_object {
            if let Some(o) = vm.oo.objects.get(&step.provider) {
                names.extend(slot_ref_obj(o, writable).iter().cloned());
            }
        } else if let Some(c) = vm.oo.classes.get(&step.provider) {
            names.extend(slot_ref_class(c, writable).iter().cloned());
        }
    }
    names.into_iter().collect()
}

/// `property name ?-kind K? ?-get body? ?-set body? …` inside a configurable
/// class's (or its object's) definition body. Each spec is a property name
/// followed by its options; options bind to the preceding name.
pub(super) fn cmd_property(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Only a configurable definition target exposes `property`; anywhere else it
    // is simply not a command (C keeps it in the configurable define namespace).
    let Some((is_class, target)) = active_target(vm) else {
        return err("invalid command name \"property\"");
    };
    let allowed = if is_class {
        // The class must itself have been built by `oo::configurable` (a plain
        // subclass of a configurable class does *not* inherit the command).
        vm.oo.classes.get(&target).is_some_and(|c| c.configurable)
    } else {
        vm.oo.object_is_configurable(target)
    };
    if !allowed {
        return err("invalid command name \"property\"");
    }
    if vm
        .actual_native_invocation_dialect()
        .native_property_name_protocol()
        .is_some()
    {
        return native_properties::define_original_properties(vm, is_class, target, args);
    }
    vm.refuse_host_command("native property declaration protocol unavailable".to_owned())
}

pub(super) fn apply_property_bytes(
    vm: &mut Vm,
    is_class: bool,
    target: OoId,
    property_name: (&[u8], Option<&Value>),
    kind: TclOoPropertyKind,
    getter: Option<Value>,
    setter: Option<Value>,
) -> Result<(), Completion<Value>> {
    original_name_operands::run_result(vm, Some((is_class, target)), |vm, original_names| {
        let (name, original) = property_name;
        let (dashed, read_method, write_method) = property_accessor_keys(vm, name);
        let (readable, writable) = match kind {
            TclOoPropertyKind::Readable => (true, false),
            TclOoPropertyKind::Writable => (false, true),
            TclOoPropertyKind::ReadWrite => (true, true),
        };
        let get_m = match getter {
            Some(body) => Some(build_method(vm, &Value::empty(), &body, false)?),
            None if readable => {
                original.map(|original| native_properties::default_method(original, false))
            }
            None => None,
        };
        original_names
            .check_target(vm)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        let set_m = match setter {
            Some(body) => Some(build_method(vm, &Value::string("value"), &body, false)?),
            None if writable => {
                original.map(|original| native_properties::default_method(original, true))
            }
            None => None,
        };
        original_names
            .check_target(vm)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        if readable {
            vm.native_property_method_created(target, is_class)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        }
        if writable {
            vm.native_property_method_created(target, is_class)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        }
        if let Err(error) =
            native_properties::register_property(vm, target, is_class, name, readable, writable)
        {
            return Err(crate::command::completion_from_cmd_error(vm, error));
        }
        for (key, method) in [(read_method, get_m), (write_method, set_m)] {
            original_names
                .check_target(vm)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
            let (payload, record) = if is_class {
                let entry = vm
                    .oo
                    .classes
                    .get_mut(&target)
                    .expect("retained class facet");
                entry.readable_properties.remove(&dashed);
                entry.writable_properties.remove(&dashed);
                if readable {
                    entry.readable_properties.insert(dashed.clone());
                }
                if writable {
                    entry.writable_properties.insert(dashed.clone());
                }
                match method {
                    Some(method) => (entry.methods.insert_for_retirement(key, method), None),
                    None => (None, entry.methods.remove(&key)),
                }
            } else {
                let entry = vm.oo.objects.get_mut(&target).expect("retained object");
                entry.readable_properties.remove(&dashed);
                entry.writable_properties.remove(&dashed);
                if readable {
                    entry.readable_properties.insert(dashed.clone());
                }
                if writable {
                    entry.writable_properties.insert(dashed.clone());
                }
                match method {
                    Some(method) => (entry.methods.insert_for_retirement(key, method), None),
                    None => (None, entry.methods.remove(&key)),
                }
            };
            drop(payload);
            drop(record);
            original_names
                .check_target(vm)
                .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        }
        Ok(())
    })
}

/// The `configure` instance method of a configurable object. Forms:
/// `configure` (list every readable property as `-name value`), `configure
/// -name` (read one), `configure -name value …` (write pairs).
fn property_accessor_keys(vm: &mut Vm, name: &[u8]) -> (String, String, String) {
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_name_protocol()
        .expect("selected property name protocol");
    let dashed_bytes = recipe.dashed_name(name);
    let (read_bytes, write_bytes) = recipe.accessor_names(name);
    let dashed = vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(dashed_bytes));
    let read = vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(read_bytes));
    let write = vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(write_bytes));
    (dashed, read, write)
}

pub(super) fn configure_method(vm: &mut Vm, obj_key: OoId, args: &[Value]) -> Completion<Value> {
    if vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .is_some()
    {
        return native_properties::configure_native(vm, obj_key, args);
    }
    match args.len() {
        0 => {
            let mut out = Vec::new();
            for dashed in configure_props(vm, obj_key, false) {
                let val = match read_property(vm, obj_key, &dashed) {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                out.push(Value::new_native_string_bytes(
                    vm.oo.method_name_bytes(&dashed),
                ));
                out.push(val);
            }
            ok(Value::list(out))
        }
        1 => {
            let dashed = match gate_property(vm, obj_key, &args[0], false, &mut None, false) {
                Ok(name) => name,
                Err(e) => return e,
            };
            match read_property(vm, obj_key, &dashed) {
                Ok(v) => ok(v),
                Err(e) => e,
            }
        }
        n if n % 2 == 0 => {
            let mut i = 0;
            let mut property_table = None;
            while i < n {
                let dashed =
                    match gate_property(vm, obj_key, &args[i], true, &mut property_table, true) {
                        Ok(name) => name,
                        Err(e) => return e,
                    };
                if let Err(e) = write_property(vm, obj_key, &dashed, args[i + 1].clone()) {
                    return e;
                }
                i += 2;
            }
            ok(Value::empty())
        }
        _ => {
            let original = vm
                .invoked_name_value()
                .unwrap_or_else(|| Value::from_native_string_bytes(display_oo_bytes(vm, obj_key)));
            let mut usage =
                match vm.native_argument_usage_header(&[original, Value::string("configure")]) {
                    Ok(header) => header,
                    Err(completion) => return completion,
                };
            usage.extend_from_slice(b" ?-option value ...?");
            crate::command::native_wrong_args_bytes(vm, &usage)
        }
    }
}

/// Resolve `dashed` to a property valid for the requested access, producing
/// the `bad`/`ambiguous property`/`is write only`/`is read only` errors
/// otherwise.
///
/// C's `oo::configuresupport` reaches this through
/// `tcl::prefix match -message property`, so an abbreviation resolves
/// (`-ye` → `-yellow`) and a word prefixing several — `""` and a lone `-`
/// included — is `ambiguous property`. The resolved *canonical* name is what
/// the accessor then reads or writes.
fn gate_property(
    vm: &mut Vm,
    obj_key: OoId,
    original: &Value,
    for_write: bool,
    _table: &mut Option<Value>,
    _retain_table: bool,
) -> Result<String, Completion<Value>> {
    original_name_operands::run_result(vm, Some((false, obj_key)), |vm, original_names| {
        let recipe = vm
            .actual_native_invocation_dialect()
            .native_property_name_protocol()
            .ok_or_else(|| {
                vm.refuse_host_command("native property name protocol unavailable".to_owned())
            })?;
        let bytes = original_names
            .read(vm, original)
            .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
        let wanted = configure_props(vm, obj_key, for_write);
        let names: Vec<_> = wanted
            .iter()
            .map(|key| vm.oo.method_name_bytes(key))
            .collect();
        let miss = match recipe.lookup(&bytes, &names) {
            Ok(index) => return Ok(wanted[index].clone()),
            Err(message) => message,
        };
        let other = configure_props(vm, obj_key, !for_write);
        let names: Vec<_> = other
            .iter()
            .map(|key| vm.oo.method_name_bytes(key))
            .collect();
        let mut message = miss;
        if let Ok(index) = recipe.lookup(&bytes, &names) {
            message = b"property \"".to_vec();
            message.extend_from_slice(tcl_core_types::c_string_extent(names[index]));
            message.extend_from_slice(if for_write {
                b"\" is read only"
            } else {
                b"\" is write only"
            });
        }
        Err(crate::command::completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::lookup_index_bytes(
                message,
                b"property",
                tcl_core_types::c_string_extent(&bytes),
            )
            .with_native_string_result(recipe.strings()),
        ))
    })
}

/// Derive the selected accessor key from the retained counted property name.
fn property_accessor_key(vm: &mut Vm, dashed: &str, writable: bool) -> String {
    let original = vm.oo.method_name_bytes(dashed).to_vec();
    let name = original.strip_prefix(b"-").unwrap_or(&original);
    let (_, read, write) = property_accessor_keys(vm, name);
    if writable { write } else { read }
}

fn read_property(vm: &mut Vm, obj_key: OoId, dashed: &str) -> Result<Value, Completion<Value>> {
    let accessor = property_accessor_key(vm, dashed, false);
    let invoked = Value::new_native_string_bytes(display_oo_bytes(vm, obj_key));
    let completion = oo_invoke(vm, obj_key, &accessor, &[], false, &invoked);
    accessor_result(completion, vm.oo.method_name_bytes(dashed), false)
}

fn write_property(
    vm: &mut Vm,
    obj_key: OoId,
    dashed: &str,
    value: Value,
) -> Result<(), Completion<Value>> {
    let accessor = property_accessor_key(vm, dashed, true);
    let invoked = Value::new_native_string_bytes(display_oo_bytes(vm, obj_key));
    let completion = oo_invoke(vm, obj_key, &accessor, &[value], false, &invoked);
    accessor_result(completion, vm.oo.method_name_bytes(dashed), true)?;
    Ok(())
}

/// Map a custom accessor's completion into a value (or propagate its error).
/// A `break`/`continue` from the accessor body becomes the `TclOO` diagnostic
/// `property {getter,setter} for -name did a {break,continue}`; other non-ok
/// codes (errors, `return -level N`) propagate unchanged.
fn accessor_result(
    comp: Completion<Value>,
    dashed: &[u8],
    is_set: bool,
) -> Result<Value, Completion<Value>> {
    match comp.code {
        Code::Ok => Ok(comp.result),
        Code::Break | Code::Continue => {
            let role = if is_set { "setter" } else { "getter" };
            let verb = if comp.code == Code::Break {
                "break"
            } else {
                "continue"
            };
            let mut message = format!("property {role} for ").into_bytes();
            message.extend_from_slice(dashed);
            message.extend_from_slice(format!(" did a {verb}").as_bytes());
            Err(err(message))
        }
        // A residual `return -level N` (N>1 in the body) or an error propagates
        // unchanged, so `configure` transparently forwards it to its caller.
        _ => Err(comp),
    }
}

pub(super) fn slot_ref_class(c: &Class, writable: bool) -> &BTreeSet<String> {
    if writable {
        &c.writable_properties
    } else {
        &c.readable_properties
    }
}

pub(super) fn slot_ref_obj(o: &Object, writable: bool) -> &BTreeSet<String> {
    if writable {
        &o.writable_properties
    } else {
        &o.readable_properties
    }
}
