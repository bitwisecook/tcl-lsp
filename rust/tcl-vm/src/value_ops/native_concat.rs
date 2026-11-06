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
