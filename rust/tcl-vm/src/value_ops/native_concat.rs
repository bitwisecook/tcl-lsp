// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual concat headers, borrowed original children and selected getters.
use crate::{interp::Vm, value::Value};
use std::rc::Rc;
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity as Storage};
use tcl_syntax::value::{NativeConcatFirstElement, NativeConcatListShape as Shape, ValueError};

fn protocol(vm: &Vm) -> Result<NativeStringProtocol, ValueError> {
    vm.actual_native_invocation_dialect()
        .native_string_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native concat string issuer",
        ))
}
pub(super) fn shape(vm: &Vm, value: &Value) -> Result<Shape, ValueError> {
    let selected = protocol(vm)?;
    value.check_native_header()?;
    let _ = value.native_list_backing_in(selected)?;
    let snapshot = value.native_object_snapshot();
    Ok(match snapshot.cache {
        Cache::List { length, canonical } => Shape::List {
            length,
            canonical,
            resident_length: snapshot.resident.as_ref().map(|bytes| bytes.len()),
        },
        _ => Shape::Other,
    })
}
pub(super) fn empty(vm: &Vm) -> Result<Value, ValueError> {
    Ok(Value::native_list_constructor(vec![], protocol(vm)?))
}
pub(super) fn copy(vm: &Vm, value: &Value) -> Result<Value, ValueError> {
    value.native_list_copy(protocol(vm)?)
}
pub(super) fn append(vm: &Vm, target: &Value, source: &Value) -> Result<(), ValueError> {
    let selected = protocol(vm)?;
    let members = vm.native_object_list_elements_in(source, selected)?;
    let elements = members.elements()?;
    if !elements.is_empty() {
        target.native_list_append_prepared_elements(elements, selected)?;
    }
    Ok(())
}
pub(super) fn first(
    vm: &mut Vm,
    source: &Value,
) -> Result<Option<NativeConcatFirstElement<Value>>, ValueError> {
    let scope = crate::interp::native_operation_currency::NativeOperationScope::enter_native(vm)
        .map_err(|_| crate::interp::native_numeric_ops::OriginalNumericOps::value_unwind())?;
    let members = vm.native_object_list_elements_in(source, protocol(vm)?)?;
    scope
        .check_native(vm)
        .map_err(|_| crate::interp::native_numeric_ops::OriginalNumericOps::value_unwind())?;
    members
        .elements()?
        .first()
        .map(|value| {
            bytes(vm, value).map(|bytes| NativeConcatFirstElement {
                bytes,
                temporary: None,
            })
        })
        .transpose()
}
pub(super) fn bytes(vm: &mut Vm, value: &Value) -> Result<Rc<[u8]>, ValueError> {
    let selected = protocol(vm)?;
    crate::interp::native_numeric_ops::OriginalNumericOps::entered_value(vm)?
        .string_value(value, selected)
}

pub(super) fn string(vm: &Vm, bytes: &[u8]) -> Result<Value, ValueError> {
    let selected = protocol(vm)?;
    if selected
        .tcl_version()
        .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_5)
    {
        Value::from_native_string_cache(
            Cache::String {
                protocol: selected,
                num_chars: None,
                unicode: None,
            },
            vm.actual_native_invocation_dialect(),
            Some((Rc::from(bytes), Storage::Allocated)),
        )
    } else {
        Ok(Value::new_native_allocated_string_bytes(Rc::<[u8]>::from(
            bytes,
        )))
    }
}
