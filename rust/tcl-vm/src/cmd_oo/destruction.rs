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

//! `destroy`, the destructor chain, record teardown, and the object's response to its command being renamed, hidden, deleted or replaced.

use super::method_invocation::run_step;
use super::{OoState, Step, display_oo_bytes, native_method_cache, original_name_operands};
use crate::interp::{CommandSidecarKey, Vm, ok};
use crate::value::Value;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use tcl_native_model::native_tcloo_method_definition::NativeTclooMethodReportRole;
use tcl_runtime_api::{Code, Completion, OoId};

// destroy / destructor

/// Run the destructor chain, then tear the object (and, if a class, its
/// descendants) down.
pub(super) fn oo_destroy(vm: &mut Vm, obj_key: OoId) -> Completion<Value> {
    let owner = crate::interp::oo_cleanup::OriginalOoCleanup::enter(vm);
    let entered = oo_destroy_entered(vm, obj_key, &owner);
    if let Err(error) = owner.check_cleanup(vm) {
        drop(entered);
        let final_error = owner.check_cleanup(vm).err().unwrap_or(error);
        return crate::command::completion_from_cmd_error(vm, final_error);
    }
    entered
}

fn oo_destroy_entered(
    vm: &mut Vm,
    obj_key: OoId,
    owner: &crate::interp::oo_cleanup::OriginalOoCleanup,
) -> Completion<Value> {
    let state = match owner.oo_mut(vm) {
        Ok(state) => state,
        Err(error) => return crate::command::completion_from_cmd_error(vm, error),
    };
    match state.objects.get_mut(&obj_key) {
        Some(object) if object.destroyed => return ok(Value::empty()),
        Some(object) => object.destroyed = true,
        None => return ok(Value::empty()),
    }
    let derived: Vec<_> = if state.classes.contains_key(&obj_key) {
        state
            .objects
            .iter()
            .filter(|(id, object)| {
                **id != obj_key
                    && (object.class == obj_key
                        || state
                            .classes
                            .get(*id)
                            .is_some_and(|class| class.supers.contains(&obj_key)))
            })
            .map(|(id, _)| *id)
            .collect()
    } else {
        Vec::new()
    };
    // Descendant record cleanup is mandatory, including after the first cause.
    for descendant in derived {
        let reached = oo_destroy_entered(vm, descendant, owner);
        drop(reached);
        let _ = owner.check_cleanup(vm);
    }
    let mut result = ok(Value::empty());
    if owner.check(vm).is_ok() {
        let class = vm.oo.objects.get(&obj_key).map(|object| object.class);
        if let Some(class) = class {
            let chain: Vec<Step> = vm
                .oo
                .class_linear_of(class)
                .into_iter()
                .filter(|step| {
                    vm.oo
                        .classes
                        .get(&step.provider)
                        .is_some_and(|class| class.destructor.is_some())
                })
                .collect();
            let chain: native_method_cache::ReachedMethodChain =
                native_method_cache::retain_owners(
                    &vm.oo,
                    chain,
                    "<destructor>",
                    NativeTclooMethodReportRole::Destructor,
                )
                .into();
            let chain = chain.with_report_role(NativeTclooMethodReportRole::Destructor);
            if !chain.is_empty() {
                let name = Value::from_native_string_bytes(display_oo_bytes(vm, obj_key));
                let reached = run_step(
                    vm,
                    obj_key,
                    &chain,
                    0,
                    "<destructor>".to_owned(),
                    &[],
                    false,
                    name.clone(),
                    vec![name, Value::string("destroy")],
                    Vec::new(),
                );
                drop(chain);
                if !reached.code.is_ok() && reached.code != Code::Return {
                    result = reached;
                }
            }
        }
    }
    let cleanup = teardown_original(vm, obj_key, owner);
    if let Err(error) = cleanup {
        drop(result);
        return crate::command::completion_from_cmd_error(vm, error);
    }
    result
}

pub(super) struct OriginalConstructedRecord {
    pub(super) creation: u64,
    pub(super) name: Weak<RefCell<Option<Value>>>,
}

impl OriginalConstructedRecord {
    pub(super) fn capture(oo: &OoState, object: OoId) -> Self {
        let actual = &oo.objects[&object];
        Self {
            creation: actual.creation_id,
            name: Rc::downgrade(&actual.cached_name),
        }
    }
}

/// Failure cleanup follows the allocated record in its original physical interpreter.
pub(super) fn finish_failed_constructor(
    vm: &mut Vm,
    object: OoId,
    original_record: &OriginalConstructedRecord,
    owner: &crate::interp::oo_cleanup::OriginalOoCleanup,
    result: Completion<Value>,
) -> Completion<Value> {
    if let Err(error) = owner.check(vm) {
        crate::interp::oo_cleanup::retain_error(vm, &error);
    }
    let same_record = owner.oo_mut(vm).map(|oo| {
        oo.objects.get(&object).is_none_or(|actual| {
            actual.creation_id == original_record.creation
                && Weak::ptr_eq(&original_record.name, &Rc::downgrade(&actual.cached_name))
        })
    });
    let retired = match same_record {
        Ok(true) => teardown_original(vm, object, owner),
        Ok(false) => {
            let error = tcl_cmd_core::CmdError::from(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "original constructed OO record",
                ),
            );
            crate::interp::oo_cleanup::retain_error(vm, &error);
            Err(error)
        }
        Err(error) => Err(error),
    };
    if let Err(error) = retired {
        crate::interp::oo_cleanup::retain_error(vm, &error);
        if owner.check(vm).is_err() {
            return owner.finish_completion(vm, result);
        }
        drop(result);
        let completion = crate::command::completion_from_cmd_error(vm, error);
        return owner.finish_completion(vm, completion);
    }
    owner.finish_completion(vm, result)
}

pub(super) fn teardown_original(
    vm: &mut Vm,
    obj_key: OoId,
    owner: &crate::interp::oo_cleanup::OriginalOoCleanup,
) -> Result<(), tcl_cmd_core::CmdError> {
    let mut first = owner.check_cleanup(vm).err();
    let (command_key, namespace) = {
        let state = owner.oo_mut(vm)?;
        (
            state.command_keys.remove(&obj_key),
            state
                .objects
                .get(&obj_key)
                .and_then(|object| object.namespace_token),
        )
    };
    if let Some(command_key) = command_key
        && let Err(error) = owner.retire_command(vm, obj_key, &command_key)
    {
        first.get_or_insert(error);
    }
    if let Some(namespace) = namespace
        && let Err(error) = owner.retire_namespace(vm, namespace)
    {
        first.get_or_insert(error);
    }
    let retired_name = owner
        .oo_mut(vm)?
        .objects
        .get(&obj_key)
        .and_then(|object| object.cached_name.borrow_mut().take());
    drop(retired_name);
    if let Err(error) = owner.check_cleanup(vm) {
        first.get_or_insert(error);
    }
    let structure = vm.retire_original_property_structure(obj_key, owner);
    if let Err(error) = structure {
        first.get_or_insert(error);
    }
    // Withdraw cache membership before releasing any reached cache payload.
    let retired_properties = vm.retire_native_properties(obj_key, owner);
    if let Err(error) = retired_properties {
        first.get_or_insert(error);
    }
    let (object, class) = {
        let state = owner.oo_mut(vm)?;
        (
            state.objects.remove(&obj_key),
            state.classes.remove(&obj_key),
        )
    };
    drop(object);
    if let Err(error) = owner.check_cleanup(vm) {
        first.get_or_insert(error);
    }
    drop(class);
    if let Err(error) = owner.check_cleanup(vm) {
        first.get_or_insert(error);
    }
    // Reporting bytes remain attached to the never-reused retired object token.
    first.map_or(Ok(()), Err)
}

pub(crate) struct OriginalOoRename {
    pub(super) names: original_name_operands::OriginalOoNameScope,
}

pub(crate) fn original_oo_rename(
    vm: &mut Vm,
    object: OoId,
) -> Result<OriginalOoRename, tcl_cmd_core::CmdError> {
    let class = vm.oo.classes.contains_key(&object);
    let names = original_name_operands::OriginalOoNameScope::enter(vm, Some((class, object)))?;
    Ok(OriginalOoRename { names })
}

/// Retire the original name before publishing later public name metadata.
pub(crate) fn oo_command_renamed(
    vm: &mut Vm,
    object: OoId,
    new_key: String,
    new_display: tcl_core_types::NameBytes,
    original: &OriginalOoRename,
) -> Result<(), tcl_cmd_core::CmdError> {
    original.names.check_target(vm)?;
    let retired = vm
        .oo
        .objects
        .get(&object)
        .and_then(|owner| owner.cached_name.borrow_mut().take());
    drop(retired);
    original.names.check_target(vm)?;
    vm.oo
        .command_keys
        .insert(object, CommandSidecarKey::visible(new_key));
    vm.oo.names.insert(object, new_display);
    original.names.check_target(vm)
}

/// Relocate the command-table sidecar without changing `TclOO`'s public object
/// name. Hiding is not a Tcl command rename: `self object` continues to report
/// the visible name the object had before it became hidden.
pub(crate) fn oo_command_hidden(vm: &mut Vm, object: OoId, token: String) {
    vm.oo
        .command_keys
        .insert(object, CommandSidecarKey::hidden(token));
}

pub(crate) fn oo_command_exposed(vm: &mut Vm, object: OoId, new_key: String) {
    vm.oo
        .command_keys
        .insert(object, CommandSidecarKey::visible(new_key));
}

/// Run the `TclOO` delete lifecycle after the command mutation owner's delete
/// trace. The ordinary command mutation owner removes the table entry through
/// [`Vm::remove_command_exact`]; [`teardown_original`] reaches that same exact-key seam.
pub(crate) fn oo_command_deleted(vm: &mut Vm, object: OoId) {
    let owner = crate::interp::oo_cleanup::OriginalOoCleanup::enter(vm);
    if let Ok(state) = owner.oo_mut(vm) {
        state.command_keys.remove(&object);
    }
    let reached = oo_destroy_entered(vm, object, &owner);
    drop(reached);
    if let Err(error) = owner.check_cleanup(vm) {
        crate::interp::oo_cleanup::retain_error(vm, &error);
    }
}

/// Run the `TclOO` lifecycle for a command implementation being replaced in
/// place. Tcl keeps the command-table token for the new implementation, while
/// the old object/class is destroyed (including descendants and destructors).
/// Detaching only this root key makes [`teardown_original`] leave that table entry to
/// the registration owner; cascaded objects retain their keys and are deleted
/// normally.
pub(crate) fn oo_command_replaced(vm: &mut Vm, object: OoId) {
    let owner = crate::interp::oo_cleanup::OriginalOoCleanup::enter(vm);
    if let Ok(state) = owner.oo_mut(vm) {
        state.command_keys.remove(&object);
    }
    let reached = oo_destroy_entered(vm, object, &owner);
    drop(reached);
    if let Err(error) = owner.check_cleanup(vm) {
        crate::interp::oo_cleanup::retain_error(vm, &error);
    }
}
