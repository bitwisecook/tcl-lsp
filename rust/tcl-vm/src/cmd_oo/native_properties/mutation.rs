// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original property setters, owning working Lists and checked metadata effects.
use super::*;
#[cfg(test)]
mod tests;
use super::super::original_name_operands::OriginalOoNameScope;
use crate::interp::{
    native_numeric_ops::OriginalNumericOps, native_operation_currency::NativeOperationScope,
};
use crate::value::NativeObjectLifetimeLease;
use tcl_registry::definer::SlotOp;

fn retain(vm: &mut Vm, error: &CmdError) {
    crate::interp::oo_cleanup::retain_error(vm, error);
}
fn run_effect(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    work: impl FnOnce(&mut Vm, &OriginalOoNameScope) -> Result<(), CmdError>,
) -> Result<(), CmdError> {
    let scope = NativeOperationScope::enter(vm).map_err(|error| {
        retain(vm, &error);
        error
    })?;
    let names = OriginalOoNameScope::enter(vm, Some((class, target)))?;
    let reached = work(vm, &names);
    if let Err(error) = &reached {
        retain(vm, error);
    }
    let final_target = names.finish(vm);
    scope.check(vm)?;
    final_target?;
    reached
}

pub(in crate::cmd_oo) fn method_invalidation(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    created: bool,
) -> Result<(), CmdError> {
    run_effect(vm, target, class, |vm, original| {
        original.check(vm)?;
        if vm
            .actual_native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
            .is_none()
        {
            return Ok(());
        }
        let retired = if created {
            vm.oo
                .native_methods
                .method_created_checked(target, class)
                .map(|()| None)
        } else {
            let dependents = vm.oo.classes.iter().any(|(id, entry)| {
                *id != target && (entry.supers.contains(&target) || entry.mixins.contains(&target))
            }) || vm
                .oo
                .objects
                .values()
                .any(|entry| entry.class == target || entry.mixins.contains(&target));
            let representative = vm
                .oo
                .objects
                .get(&target)
                .is_some_and(|entry| !entry.mixins.is_empty());
            vm.oo.native_methods.structure_changed_checked(
                target,
                class,
                dependents,
                representative,
            )
        };
        let retired = retired.map_err(|error| {
            let error = CmdError::from(error);
            retain(vm, &error);
            error
        })?;
        drop(retired);
        original.check(vm)
    })
}

pub(super) fn invalidate(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    created: bool,
) -> Result<(), CmdError> {
    run_effect(vm, target, class, |vm, original| {
        if created {
            vm.native_method_created(target, class)?;
        } else {
            vm.native_method_structure_changed(target, class)?;
        }
        original.check(vm)?;
        let Some(recipe) = vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
        else {
            return Ok(());
        };
        let action = if created {
            recipe.method_created(class)
        } else {
            let subclasses = vm
                .oo
                .classes
                .iter()
                .any(|(id, entry)| *id != target && entry.supers.contains(&target));
            let instances = vm
                .oo
                .objects
                .values()
                .any(|entry| entry.class == target || entry.mixins.contains(&target));
            let mixin_dependents = vm
                .oo
                .classes
                .values()
                .any(|entry| entry.mixins.contains(&target));
            let representative = vm
                .oo
                .objects
                .get(&target)
                .is_some_and(|entry| !entry.mixins.is_empty());
            recipe.structure_changed(
                class,
                NativePropertyGraphDependents {
                    subclasses,
                    instances,
                    mixin_dependents,
                },
                representative,
            )
        };
        let retired = vm
            .oo
            .apply_property_invalidation(target, action)
            .map_err(|error| {
                let error = CmdError::from(error);
                retain(vm, &error);
                error
            })?;
        drop(retired);
        original.check(vm)
    })
}

fn headers(vm: &Vm, target: OoId, class: bool, writable: bool) -> Vec<NativeObjectLifetimeLease> {
    vm.oo
        .property_members
        .get(&(target, class, writable))
        .into_iter()
        .flat_map(|members| members.values())
        .map(Value::native_lifetime_lease)
        .collect()
}
fn store_names(vm: &mut Vm, target: OoId, class: bool, writable: bool, names: BTreeSet<String>) {
    if class {
        if let Some(entry) = vm.oo.classes.get_mut(&target) {
            *slot_ref_mut_class(entry, writable) = names;
        }
    } else if let Some(entry) = vm.oo.objects.get_mut(&target) {
        *slot_ref_mut_object(entry, writable) = names;
    }
}
fn slot_ref_mut_class(entry: &mut super::super::Class, writable: bool) -> &mut BTreeSet<String> {
    if writable {
        &mut entry.writable_properties
    } else {
        &mut entry.readable_properties
    }
}
fn slot_ref_mut_object(entry: &mut super::super::Object, writable: bool) -> &mut BTreeSet<String> {
    if writable {
        &mut entry.writable_properties
    } else {
        &mut entry.readable_properties
    }
}
fn actual_header(
    operation: &OriginalPropertyOperation,
    vm: &mut Vm,
    original: &Value,
) -> Result<(), CmdError> {
    operation.check(vm)?;
    OriginalNumericOps::in_original_scope(vm, &operation.operation)?.current_for_value(original)
}

/// `SetPropertyList`'s incoming-reference stage follows cache retirement. Capture
/// leases themselves add no native member reference or String getter authority.
pub(super) fn install_members(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    writable: bool,
    operation: &OriginalPropertyOperation,
    originals: &[NativeObjectLifetimeLease],
) -> Result<BTreeSet<String>, CmdError> {
    use tcl_registry::native_property_lookup::NativePropertyMemberInstallationStep as Stage;
    operation.check(vm)?;
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original property member installer",
        ))?;
    vm.oo
        .property_origins
        .entry((target, class))
        .or_insert(recipe);
    let mut incoming = Vec::new();
    let mut adopted = BTreeMap::new();
    for stage in recipe.member_installation() {
        match stage {
            Stage::Cache => {
                let retired = vm
                    .oo
                    .property_caches
                    .get_mut(&(target, class))
                    .and_then(|cache| {
                        if writable {
                            cache.writable.take()
                        } else {
                            cache.readable.take()
                        }
                    });
                drop(retired);
                operation.check(vm)?;
            }
            Stage::IncomingReferences => {
                for original in originals {
                    actual_header(operation, vm, original.value())?;
                    incoming.push(Some(original.value().clone()));
                }
            }
            Stage::OldMembers => {
                let retired = vm.oo.property_members.remove(&(target, class, writable));
                store_names(vm, target, class, writable, BTreeSet::new());
                drop(retired);
                operation.check(vm)?;
            }
            Stage::UniqueHash => {
                for original in &mut incoming {
                    let actual = original.as_ref().expect("original incoming member role");
                    let bytes = operation.string(vm, actual)?;
                    let key = vm
                        .oo
                        .intern_method_key(tcl_core_types::NameBytes::from(bytes.as_ref()));
                    if let std::collections::btree_map::Entry::Vacant(entry) = adopted.entry(key) {
                        entry.insert(original.take().expect("selected member role"));
                    } else {
                        drop(original.take());
                        operation.check(vm)?;
                    }
                }
            }
        }
    }
    operation.check(vm)?;
    let names = adopted.keys().cloned().collect::<BTreeSet<_>>();
    let displaced = vm
        .oo
        .property_members
        .insert((target, class, writable), adopted);
    store_names(vm, target, class, writable, names.clone());
    drop(displaced);
    operation.check(vm)?;
    drop(incoming);
    operation.check(vm)?;
    Ok(names)
}

/// `GetPropertyList`'s caller role: the original member headers become actual List
/// members without sorting, rebuilding their spellings or pre-hashing them.
fn member_list(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    writable: bool,
    operation: &OriginalPropertyOperation,
) -> Result<Value, CmdError> {
    let original = headers(vm, target, class, writable);
    let mut members = Vec::with_capacity(original.len());
    for header in &original {
        actual_header(operation, vm, header.value())?;
        members.push(header.value().clone());
    }
    Ok(Value::native_list_constructor(members, operation.strings))
}

pub(super) fn slot(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    writable: bool,
    selected: SlotOp,
    names: &[Value],
) -> Result<BTreeSet<String>, CmdError> {
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "property metadata slot issuer",
        ))?;
    let operation = OriginalPropertyOperation::enter(vm, target, class, recipe.strings())?;
    let reached = (|| {
        if names.is_empty() && matches!(selected, SlotOp::Append | SlotOp::Remove) {
            return Ok(vm
                .oo
                .property_members
                .get(&(target, class, writable))
                .map(|members| members.keys().cloned().collect())
                .unwrap_or_default());
        }
        // ResolveAll/Slot_Set's actual caller List retains original headers;
        // this is distinct from the setter's later incoming-reference stage.
        let resolved = Value::native_list_constructor(names.to_vec(), recipe.strings());
        let working = match selected {
            SlotOp::Set => resolved,
            SlotOp::Clear => {
                drop(resolved);
                Value::native_list_constructor(Vec::new(), recipe.strings())
            }
            SlotOp::Append => {
                let list = member_list(vm, target, class, writable, &operation)?;
                let actual = operation.list(vm, &resolved)?;
                list.native_list_append_prepared_elements(actual.as_slice(), recipe.strings())?;
                drop(actual);
                drop(resolved);
                operation.check(vm)?;
                list
            }
            SlotOp::Remove => {
                let old = member_list(vm, target, class, writable, &operation)?;
                let queries = operation.list(vm, &resolved)?;
                let mut removed = BTreeSet::new();
                for query in queries.iter() {
                    removed.insert(operation.string(vm, query)?);
                }
                drop(queries);
                drop(resolved);
                operation.check(vm)?;
                let members = operation.list(vm, &old)?;
                let mut retained = Vec::new();
                for member in members.iter() {
                    let bytes = operation.string(vm, member)?;
                    if !removed.contains(&bytes) {
                        actual_header(&operation, vm, member)?;
                        retained.push(member.clone());
                    }
                }
                drop(members);
                drop(old);
                operation.check(vm)?;
                Value::native_list_constructor(retained, recipe.strings())
            }
            _ => {
                return Err(ValueError::CommandProtocolUnavailable(
                    "native property slot operation",
                )
                .into());
            }
        };
        let members = operation.list(vm, &working)?;
        let originals = members
            .iter()
            .map(Value::native_lifetime_lease)
            .collect::<Vec<_>>();
        let names =
            vm.install_original_property_members(target, class, writable, &operation, &originals)?;
        if class {
            vm.native_property_structure_changed(target, true)?;
        }
        drop(members);
        drop(working);
        operation.check(vm)?;
        Ok(names)
    })();
    operation.complete(vm, reached)
}

/// `BuildPropertyList` performs each actual comparison before that member's List
/// append reference. It retains no native +1 merely to guard the old roster.
fn registration_list(
    vm: &mut Vm,
    operation: &OriginalPropertyOperation,
    originals: &[NativeObjectLifetimeLease],
    property: &Value,
    adding: bool,
) -> Result<(Value, bool), CmdError> {
    let mut present = false;
    let mut changed = false;
    let mut members = Vec::new();
    for original in originals {
        let equal =
            original_stages::compare(vm, operation, property, original.value(), |_| Ok(()))?
                == std::cmp::Ordering::Equal;
        if equal {
            present = true;
            if !adding {
                changed = true;
                continue;
            }
        }
        actual_header(operation, vm, original.value())?;
        members.push(original.value().clone());
    }
    if !present && adding {
        actual_header(operation, vm, property)?;
        members.push(property.clone());
        changed = true;
    }
    Ok((
        Value::native_list_constructor(members, operation.strings),
        changed,
    ))
}
pub(super) fn register(
    vm: &mut Vm,
    target: OoId,
    class: bool,
    name: &[u8],
    readable: bool,
    writable: bool,
) -> Result<(), CmdError> {
    let Some(recipe) = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
    else {
        return Ok(());
    };
    let operation = OriginalPropertyOperation::enter(vm, target, class, recipe.strings())?;
    let reached = (|| {
        let mut bytes = b"-".to_vec();
        bytes.extend_from_slice(tcl_core_types::c_string_extent(name));
        let original = Value::new_native_string_bytes(bytes);
        original.retain_native_string_representation(recipe.materialization())?;
        let mut changed = false;
        for (direction, adding) in [(false, readable), (true, writable)] {
            let originals = headers(vm, target, class, direction);
            let (working, selected) =
                registration_list(vm, &operation, &originals, &original, adding)?;
            if selected {
                let actual = operation.list(vm, &working)?;
                let headers = actual
                    .iter()
                    .map(Value::native_lifetime_lease)
                    .collect::<Vec<_>>();
                vm.install_original_property_members(
                    target, class, direction, &operation, &headers,
                )?;
                changed = true;
            }
            drop(working);
            operation.check(vm)?;
        }
        if changed && class {
            vm.native_property_structure_changed(target, true)?;
        }
        drop(original);
        operation.check(vm)
    })();
    operation.complete(vm, reached)
}
