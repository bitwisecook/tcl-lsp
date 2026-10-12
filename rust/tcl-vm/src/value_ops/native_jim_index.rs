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

//! Actual Jim Index conversion on the VM's original headers.
use crate::{
    interp::{
        Vm, native_numeric_ops::OriginalNumericOps, native_operation_currency::NativeOperationScope,
    },
    value::{NativeObjectLifetimeLease, Value},
};
use std::rc::Rc;
use tcl_cmd_core::{
    CmdError,
    index::NativeIndexOps,
    native_jim_index::{self, NativeJimIndexObjects},
};
use tcl_syntax::{
    native_jim_index::{JimIndex, JimIndexExpression},
    native_string::NativeStringProtocol,
    value::ValueError,
};

/// Allocation-only selected key transport; native ownership is never Rc key copies.
#[derive(Clone)]
pub struct VmIndexKey(Rc<VmIndexKeyState>);
struct VmIndexKeyState {
    value: Value,
    created: std::cell::Cell<bool>,
    result: std::cell::RefCell<Option<Value>>,
}
impl Drop for VmIndexKeyState {
    fn drop(&mut self) {
        if self.created.get() {
            self.value.retire_unowned_native_header();
        }
    }
}
impl VmIndexKey {
    fn held(value: Value) -> Self {
        Self(Rc::new(VmIndexKeyState {
            value: value.native_lifetime_lease().into_value(),
            created: std::cell::Cell::new(false),
            result: std::cell::RefCell::new(Some(value)),
        }))
    }
    fn borrowed(original: &Value) -> Self {
        Self(Rc::new(VmIndexKeyState {
            value: original.native_lifetime_lease().into_value(),
            created: std::cell::Cell::new(false),
            result: std::cell::RefCell::new(None),
        }))
    }
}

/// Memory-only original header retention under the same actual operation.
pub struct VmJimIndexScope {
    operation: NativeOperationScope,
    header: NativeObjectLifetimeLease,
}
/// Currency-only complete indexed command entry; no operand getter authority.
pub struct VmIndexOperation(NativeOperationScope);
impl VmIndexOperation {
    pub(crate) fn original_scope(&self) -> &NativeOperationScope {
        &self.0
    }
}
impl NativeIndexOps for Vm {
    type IndexOperation = VmIndexOperation;
    type IndexBinaryReceipt = Option<crate::value::NativeBinaryStorageReceipt>;
    type IndexKeyOwner = VmIndexKey;
    fn index_borrowed_key(&mut self, original: &Value) -> Result<VmIndexKey, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        scope.check(self)?;
        original.check_native_header()?;
        Ok(VmIndexKey::borrowed(original))
    }
    fn index_created_key(&mut self, created: Value) -> Result<VmIndexKey, CmdError> {
        Ok(VmIndexKey(Rc::new(VmIndexKeyState {
            value: created.into_native_unowned_lifetime(),
            created: std::cell::Cell::new(true),
            result: std::cell::RefCell::new(None),
        })))
    }
    fn index_key_value(owner: &VmIndexKey) -> &Value {
        &owner.0.value
    }
    fn index_key_argument(&mut self, owner: &VmIndexKey) -> Result<Value, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        scope.check(self)?;
        owner.0.value.check_native_header()?;
        Ok(owner.0.value.native_lifetime_lease().into_value())
    }
    fn index_key_result(
        &mut self,
        owner: &VmIndexKey,
        created: bool,
        _ancestor: bool,
    ) -> Result<tcl_cmd_core::index::IndexResult<Value>, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        scope.check(self)?;
        owner.0.value.check_native_header()?;
        // The completed command result acquires its genuine result reference.
        let value = owner.0.value.clone();
        if created {
            owner.0.created.set(false);
        }
        Ok(if created {
            tcl_cmd_core::index::IndexResult::Created(value)
        } else {
            tcl_cmd_core::index::IndexResult::Borrowed(value)
        })
    }
    fn index_hold_key_result(&mut self, owner: &VmIndexKey) -> Result<VmIndexKey, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        scope.check(self)?;
        owner.0.value.check_native_header()?;
        let held = VmIndexKey::borrowed(&owner.0.value);
        *held.0.result.borrow_mut() = Some(owner.0.value.clone());
        Ok(held)
    }
    fn index_finish_held_key_result(
        &mut self,
        held: VmIndexKey,
    ) -> Result<tcl_cmd_core::index::IndexResult<Value>, CmdError> {
        Ok(tcl_cmd_core::index::IndexResult::Detached(
            held.0
                .result
                .borrow_mut()
                .take()
                .expect("selected child result hold"),
        ))
    }

    fn enter_index_operation(&mut self) -> Result<Self::IndexOperation, CmdError> {
        NativeOperationScope::enter(self).map(VmIndexOperation)
    }
    fn check_index_operation(&mut self, original: &Self::IndexOperation) -> Result<(), CmdError> {
        original.0.currency().check(self)
    }
    fn index_integer_result(
        &mut self,
        original: &Self::IndexOperation,
        integer: i64,
    ) -> Result<Value, CmdError> {
        original.0.check(self)?;
        if self.index_c_version() != Some(tcl_dialect::TclVersion::V8_4) {
            return Ok(tcl_syntax::value::ValueOps::new_int(self, integer));
        }
        let numeric = OriginalNumericOps::in_original_scope(self, &original.0)?;
        let dialect = numeric.dialect();
        i32::try_from(integer).map_err(|_| {
            ValueError::CommandProtocolUnavailable("original C84 Index Int result width")
        })?;
        let cache = tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(integer);
        let reached = numeric.with_environment(|environment, stage| {
            stage()?;
            let environment = environment.ok_or(ValueError::ScalarNumericInputUnavailable)?;
            let abi = environment
                .c_integer_abi()
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
            stage()?;
            let protocol = dialect
                .native_scalar_getter_protocol()
                .ok_or(ValueError::ScalarNumericInputUnavailable)?;
            protocol
                .validate_cache_abi(&cache, abi.char_bits, abi.int_bytes, abi.long_bytes)
                .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
            Value::from_native_scalar_cache(cache, None, dialect).map_err(CmdError::from)
        });
        if let Err(first) = &reached {
            numeric.retain_failure(first);
        }
        if let Err(first) = numeric.current() {
            numeric.retain_failure(&first);
            drop(reached);
            let _ = numeric.current();
            return Err(first);
        }
        reached
    }

    fn retain_index_failure(&mut self, error: &CmdError) {
        if let Some(first) = error.native_execution_refusal() {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(first.clone()));
        }
    }
    fn discard_owned_index_result(&mut self, created: Value) {
        drop(created);
    }

    fn original_c_list_index(
        &mut self,
        list: &Value,
        indices: &[Value],
        path: bool,
    ) -> Result<Option<tcl_cmd_core::index::IndexResult<Value>>, CmdError> {
        if self.index_c_version().is_none() {
            return Ok(None);
        }
        self.original_list_index(list, indices, path).map(|value| {
            Some(tcl_cmd_core::index::IndexResult::ReturnedOwnedTransport(
                value,
            ))
        })
    }
    fn original_c_list_range(
        &mut self,
        list: &Value,
        first: &Value,
        last: &Value,
    ) -> Result<Option<tcl_cmd_core::index::IndexResult<Value>>, CmdError> {
        if self.index_c_version().is_none() {
            return Ok(None);
        }
        self.original_list_range(list, first, last).map(|value| {
            Some(tcl_cmd_core::index::IndexResult::ReturnedOwnedTransport(
                value,
            ))
        })
    }
    fn index_c_version(&self) -> Option<tcl_dialect::TclVersion> {
        self.actual_native_invocation_dialect()
            .native_string_protocol()
            .and_then(|protocol| protocol.tcl_version())
    }
    fn original_c_option_path(
        &mut self,
        original: &Value,
        command: tcl_cmd_core::native_option_index::OptionCommand,
        report: bool,
    ) -> Result<Option<Vec<tcl_syntax::native_compiled_index::NativeCompiledListIndex>>, CmdError>
    {
        if self.index_c_version().is_none() {
            return Ok(None);
        }
        tcl_cmd_core::native_option_index::prepare_original(self, original, command, report)
            .map(Some)
    }
    fn original_list_stride(
        &mut self,
        original: &Value,
        command: tcl_cmd_core::native_option_index::OptionCommand,
    ) -> Result<Option<i64>, CmdError> {
        use crate::expr::{original_numeric_command_ops, with_original_numeric_command};
        use tcl_registry::native_numeric_error::NativeNumericCommandPurpose as Purpose;
        use tcl_syntax::scalar_getter::NativeScalarGetterValue;
        let purpose = match command {
            tcl_cmd_core::native_option_index::OptionCommand::Search => Purpose::ListSearchStride,
            tcl_cmd_core::native_option_index::OptionCommand::Sort => Purpose::ListSortStride,
        };
        with_original_numeric_command(self, CmdError::from, |vm, context| {
            original_numeric_command_ops(vm, context, purpose, |numeric, selected| {
                let reached = numeric.getter(original, selected.getter_kind());
                if let Err(first) = &reached {
                    numeric.retain_failure(first);
                }
                numeric.current_for_value(original)?;
                let NativeScalarGetterValue::Wide(integer) = reached? else {
                    return Err(ValueError::ScalarNumericInputUnavailable.into());
                };
                if let Some(error) = selected.invalid_list_stride_error(integer) {
                    return Err(CmdError::with_error_code_bytes(
                        error.message,
                        error.error_code,
                    ));
                }
                Ok(Some(integer))
            })
        })
    }
    fn original_list_number(
        &mut self,
        original: &Value,
        command: tcl_cmd_core::native_option_index::OptionCommand,
        kind: tcl_cmd_core::index::ListNumericKind,
    ) -> Result<Option<tcl_cmd_core::index::ListNumber>, CmdError> {
        use tcl_cmd_core::index::{ListNumber, ListNumericKind};
        use tcl_cmd_core::native_option_index::OptionCommand;
        use tcl_registry::native_numeric_error::NativeNumericCommandPurpose as Purpose;
        use tcl_syntax::scalar_getter::NativeScalarGetterValue;
        let purpose = match (command, kind) {
            (OptionCommand::Search, ListNumericKind::Integer) => Purpose::ListSearchInteger,
            (OptionCommand::Search, ListNumericKind::Real) => Purpose::ListSearchReal,
            (OptionCommand::Sort, ListNumericKind::Integer) => Purpose::ListSortInteger,
            (OptionCommand::Sort, ListNumericKind::Real) => Purpose::ListSortReal,
        };
        use crate::expr::{original_numeric_command_ops, with_original_numeric_command};
        with_original_numeric_command(self, CmdError::from, |vm, context| {
            original_numeric_command_ops(vm, context, purpose, |numeric, selected| {
                let reached = numeric.getter(original, selected.getter_kind());
                if let Err(first) = &reached {
                    numeric.retain_failure(first);
                }
                numeric.current_for_value(original)?;
                let number = match reached? {
                    NativeScalarGetterValue::Wide(integer) if kind == ListNumericKind::Integer => {
                        ListNumber::Integer(integer)
                    }
                    NativeScalarGetterValue::Double(real) if kind == ListNumericKind::Real => {
                        ListNumber::Real(real)
                    }
                    _ => return Err(ValueError::ScalarNumericInputUnavailable.into()),
                };
                Ok(Some(number))
            })
        })
    }
    fn index_c84_sort_member_hold(&mut self, original: &Value) -> Result<VmIndexKey, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        scope.check(self)?;
        original.check_native_header()?;
        if self.index_c_version() != Some(tcl_dialect::TclVersion::V8_4) {
            return Err(
                ValueError::CommandProtocolUnavailable("original C84 sort member role").into(),
            );
        }
        let held = VmIndexKey::borrowed(original);
        *held.0.result.borrow_mut() = Some(original.clone());
        Ok(held)
    }
    fn index_sort_copy(&mut self, original: &Value) -> Result<VmIndexKey, CmdError> {
        tcl_cmd_core::native_list_index::NativeListIndexOps::index_list_copy_checked(self, original)
            .map(VmIndexKey::held)
    }
    fn index_sort_duplicate(
        &mut self,
        original: &Value,
        only_shared: bool,
    ) -> Result<VmIndexKey, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &scope)?;
        numeric.current_for_value(original)?;
        if only_shared && !original.native_object_is_shared() {
            return Ok(VmIndexKey::borrowed(original));
        }
        let protocol = numeric.dialect().native_string_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("original sort duplicate issuer"),
        )?;
        let copied = original.duplicate_native_object_checked(
            protocol,
            &mut || numeric.current_for_value(original),
            &mut |value| numeric.string(value, protocol),
            &mut |error| numeric.retain_failure(error),
        )?;
        drop(numeric);
        if protocol.is_jim084() {
            // Jim_DuplicateObj is fresh-zero; Eval/result takes its own hold.
            self.index_created_key(copied)
        } else {
            Ok(VmIndexKey::held(copied))
        }
    }

    fn index_sort_list(&mut self, members: Vec<Value>, held: bool) -> Result<VmIndexKey, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        scope.check(self)?;
        let protocol = self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original sort List issuer",
            ))?;
        for member in &members {
            member.check_native_header()?;
        }
        let list = Value::native_list_constructor(members, protocol);
        if held {
            Ok(VmIndexKey::held(list))
        } else {
            self.index_created_key(list)
        }
    }
    fn index_sort_append(&mut self, list: &VmIndexKey, members: &[Value]) -> Result<(), CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &scope)?;
        let original = &list.0.value;
        let protocol = numeric.dialect().native_string_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("original sort List append issuer"),
        )?;
        let reached = original.native_list_append_prepared_elements_checked(
            members,
            protocol,
            &mut || numeric.current_for_value(original),
            &mut |value| numeric.string(value, protocol),
        );
        if let Err(first) = &reached {
            numeric.retain_failure(first);
        }
        numeric.current_for_value(original)?;
        reached
    }
    fn index_sort_replace(
        &mut self,
        list: &VmIndexKey,
        first: usize,
        delete: usize,
        members: &[Value],
    ) -> Result<(), CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &scope)?;
        let original = &list.0.value;
        let protocol = numeric.dialect().native_string_protocol().ok_or(
            ValueError::CommandProtocolUnavailable("original sort List replacement issuer"),
        )?;
        let reached = original.native_list_replace_prepared_range_checked(
            first,
            delete,
            members,
            protocol,
            &mut || numeric.current_for_value(original),
            &mut |value| numeric.string(value, protocol),
        );
        if let Err(first) = &reached {
            numeric.retain_failure(first);
        }
        numeric.current_for_value(original)?;
        reached
    }
    fn index_jim_sort_selected(&self) -> bool {
        self.actual_native_invocation_dialect()
            .native_scalar_getter_protocol()
            .is_some_and(|selected| selected.is_jim084())
    }
    fn index_native_sort<E>(
        &mut self,
        indices: &mut [usize],
        compare: &mut impl FnMut(&mut Self, usize, usize) -> Result<core::cmp::Ordering, E>,
    ) -> Result<(), tcl_cmd_core::index::NativeSortError<E>> {
        let host = self.host_rc();
        let environment = host.native_list_sorting_environment().ok_or_else(|| {
            tcl_cmd_core::index::NativeSortError::Operation(CmdError::from(
                ValueError::CommandProtocolUnavailable("original local C list sorting capability"),
            ))
        })?;
        tcl_cmd_core::index::native_sort_with_host(self, environment, indices, compare)
    }
    fn original_c_index(
        &mut self,
        original: &Value,
        length: usize,
    ) -> Result<Option<i64>, CmdError> {
        let end = i64::try_from(length)
            .map_err(|_| ValueError::CommandProtocolUnavailable("original C Index length width"))?
            - 1;
        tcl_cmd_core::native_list_index::original_index(self, original, end, true)
    }
    fn index_c_select_binary(
        &mut self,
        original: &Value,
        version: tcl_dialect::TclVersion,
    ) -> Result<Option<(Option<crate::value::NativeBinaryStorageReceipt>, Rc<[u8]>)>, CmdError>
    {
        let operation = NativeOperationScope::enter(self)?;
        operation.check(self)?;
        original.check_native_header()?;
        if self.index_c_version() != Some(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original C binary selection release",
            )
            .into());
        }
        let snapshot = original.native_object_snapshot();
        let tcl_syntax::native_object::NativeObjectCacheSnapshot::ByteArray { bytes, proper } =
            snapshot.cache
        else {
            return Ok(None);
        };
        let eligible = match version {
            tcl_dialect::TclVersion::V8_4 => true,
            tcl_dialect::TclVersion::V8_5 | tcl_dialect::TclVersion::V8_6 => {
                snapshot.resident.is_none()
            }
            tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1 => proper,
        };
        if !eligible {
            return Ok(None);
        }
        if original.native_byte_array_protocol()
            != self
                .actual_native_invocation_dialect()
                .native_string_protocol()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original C binary selection issuer",
            )
            .into());
        }
        Ok(Some((
            (version == tcl_dialect::TclVersion::V8_4)
                .then(|| original.capture_native_binary_storage())
                .transpose()?,
            bytes,
        )))
    }
    fn index_c_check_binary_storage(
        &mut self,
        original: &Value,
        receipt: &Option<crate::value::NativeBinaryStorageReceipt>,
    ) -> Result<(), CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        numeric.current_for_value(original)?;
        receipt
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "unselected C84 binary storage receipt",
            ))?
            .check(original)
            .map_err(|error| {
                let error = CmdError::from(error);
                numeric.retain_failure(&error);
                error
            })
    }
    fn index_c_binary_bytes(&mut self, original: &Value) -> Result<Rc<[u8]>, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        let dialect = numeric.dialect();
        let protocol = dialect
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C binary getter issuer",
            ))?;
        let policy =
            dialect
                .binary_data_conversion()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original C binary getter conversion",
                ))?;
        original.as_native_byte_array_checked(
            policy,
            protocol,
            &mut || numeric.current_for_value(original),
            &mut || numeric.string(original, protocol),
        )
    }
    fn index_c_string_storage(
        &mut self,
        original: &Value,
    ) -> Result<tcl_cmd_core::string::NativeCStringStorage, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        let protocol = numeric
            .dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C String storage issuer",
            ))?;
        original.native_c_string_storage_checked(
            protocol,
            &mut || numeric.current_for_value(original),
            &mut || numeric.string(original, protocol),
        )
    }
    fn index_c_plain_char(
        &mut self,
    ) -> Result<tcl_syntax::native_tcl_utf::NativeCPlainChar, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        operation.check(self)?;
        if self.index_c_version().is_none() {
            return Err(
                ValueError::CommandProtocolUnavailable("original C String ABI issuer").into(),
            );
        }
        Ok(tcl_runtime_api::native_c_string_abi::supported_backend_plain_char())
    }
    fn index_c_counted_string_result(
        &mut self,
        bytes: Rc<[u8]>,
        count: usize,
    ) -> Result<Value, CmdError> {
        if count != bytes.len() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original counted C String slice extent",
            )
            .into());
        }
        let operation = NativeOperationScope::enter(self)?;
        operation.check(self)?;
        let protocol = self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original counted C String constructor",
            ))?;
        let created = Value::new_native_allocated_string_bytes(bytes.clone());
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        let reached = created.native_append_string_checked(
            protocol,
            Some((
                bytes,
                tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
            )),
            Some(count),
            None,
            &mut || numeric.current_for_value(&created),
        );
        if let Err(error) = reached {
            numeric.retain_failure(&error);
            return Err(error);
        }
        Ok(created)
    }
    fn index_c_unicode_units(&mut self, original: &Value) -> Result<Rc<[u32]>, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        let protocol = numeric
            .dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C Unicode issuer",
            ))?;
        original.native_unicode_units_checked(
            protocol,
            &mut || numeric.current_for_value(original),
            &mut || numeric.string(original, protocol),
        )
    }
    fn index_c_pure_binary(&mut self, original: &Value) -> Result<Option<Rc<[u8]>>, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        operation.check(self)?;
        original.check_native_header()?;
        let snapshot = original.native_object_snapshot();
        if let tcl_syntax::native_object::NativeObjectCacheSnapshot::ByteArray {
            bytes,
            proper: true,
        } = snapshot.cache
        {
            if original.native_byte_array_protocol()
                != self
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
            {
                return Err(ValueError::CommandProtocolUnavailable(
                    "original C binary backing issuer",
                )
                .into());
            }
            return Ok(Some(bytes));
        }
        Ok(None)
    }
    fn index_c_original_is_shared(&mut self, original: &Value) -> Result<bool, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        operation.check(self)?;
        original.check_native_header()?;
        Ok(original.native_object_is_shared())
    }
    fn index_c_duplicate_original(&mut self, original: &Value) -> Result<Value, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        let protocol = numeric
            .dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C duplicate issuer",
            ))?;
        original.duplicate_native_object_checked(
            protocol,
            &mut || numeric.current_for_value(original),
            &mut |value| numeric.string(value, protocol),
            &mut |error: &CmdError| numeric.retain_failure(error),
        )
    }
    fn index_c_binary_result(&mut self, bytes: Rc<[u8]>) -> Result<Value, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        operation.check(self)?;
        Value::from_native_byte_array(bytes, self.actual_native_invocation_dialect())
            .map_err(Into::into)
    }
    fn index_c_binary_splice(&mut self, original: &Value, bytes: Rc<[u8]>) -> Result<(), CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        let protocol = numeric
            .dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C binary replacement splice",
            ))?;
        original.native_append_binary_checked(protocol, bytes, &mut || {
            numeric.current_for_value(original)
        })
    }
    fn index_c_append_object(&mut self, result: &Value, original: &Value) -> Result<(), CmdError> {
        super::native_indexed_append::append_object(self, result, original)
    }
    fn index_c_append_unicode(&mut self, result: &Value, units: &[u32]) -> Result<(), CmdError> {
        super::native_indexed_append::append_unicode(self, result, units)
    }
    fn index_character_count(&mut self, original: &Value) -> Result<usize, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        let dialect = self.native_invocation_dialect();
        let provider = Some(
            tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation,
        );
        let protocol = dialect
            .native_string_materialization(provider)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Index character String producer",
            ))?
            .protocol();
        let representation = dialect
            .string_length_representation_with_provider(provider)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Index character count representation",
            ))?;
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        original.native_character_count_checked(
            protocol,
            representation,
            &mut || numeric.current_for_value(original),
            &mut || numeric.string(original, protocol),
        )
    }
    fn index_list_elements(&mut self, original: &Value) -> Result<Vec<Value>, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        operation.check(self)?;
        original.check_native_header()?;
        let purpose =
            self.object_materialization()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original Index container List issuer",
                ))?;
        if purpose.protocol().is_jim084() {
            let context = self.native_jim_object_context();
            operation.check(self)?;
            let context = context?;
            let bound = original.bind_native_jim_context(&context);
            operation.check(self)?;
            original.check_native_header()?;
            bound?;
        }
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        let members = original.native_object_list_elements_with_strings(
            purpose.protocol(),
            &mut || numeric.current_for_value(original),
            &mut |value| numeric.string(value, purpose.protocol()),
        )?;
        numeric.current_for_value(original)?;
        Ok(members
            .iter()
            .map(|member| member.native_lifetime_lease().into_value())
            .collect())
    }

    fn index_list_length(&mut self, value: &Value) -> Result<usize, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        list_length_using(
            self,
            value,
            &mut |interpreter, original| {
                OriginalNumericOps::in_original_scope(interpreter, &operation)?
                    .current_for_value(original)
            },
            |interpreter, original| interpreter.index_list_elements(original),
        )
    }
    fn index_list_member(
        &mut self,
        value: &Value,
        index: usize,
    ) -> Result<Option<tcl_cmd_core::index::IndexResult<Value>>, CmdError> {
        self.index_list_elements(value).map(|members| {
            members
                .into_iter()
                .nth(index)
                .map(tcl_cmd_core::index::IndexResult::Borrowed)
        })
    }
    fn original_jim_index_path(
        &mut self,
        original: &Value,
    ) -> Result<Option<Vec<Value>>, CmdError> {
        if self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            != Some(NativeStringProtocol::Jim084)
        {
            return Err(
                ValueError::CommandProtocolUnavailable("original Jim Index path issuer").into(),
            );
        }
        self.index_list_elements(original).map(Some)
    }
    fn original_jim_index(&mut self, original: &Value) -> Result<Option<i32>, CmdError> {
        native_jim_index::original_index(self, original).map(Some)
    }
    fn index_operand_bytes(&mut self, original: &Value) -> Result<Rc<[u8]>, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        let purpose =
            self.object_materialization()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original Index String producer",
                ))?;
        OriginalNumericOps::in_original_scope(self, &operation)?
            .string(original, purpose.protocol())
    }
    fn textual_jim_index(&mut self, source: &[u8]) -> Result<Option<i32>, CmdError> {
        let original = Value::new_native_string_bytes(source);
        native_jim_index::original_index(self, &original).map(Some)
    }
}
impl NativeJimIndexObjects for Vm {
    type Scope = VmJimIndexScope;
    fn jim_index_scope(&mut self, original: &Value) -> Result<Self::Scope, CmdError> {
        let operation = NativeOperationScope::enter(self)?;
        let ops = OriginalNumericOps::in_original_scope(self, &operation)?;
        ops.current_for_value(original)?;
        if ops.dialect().native_string_protocol() != Some(NativeStringProtocol::Jim084) {
            return Err(ValueError::CommandProtocolUnavailable("original Jim Index issuer").into());
        }
        Ok(VmJimIndexScope {
            operation,
            header: original.native_lifetime_lease(),
        })
    }
    fn jim_index_check(&mut self, scope: &Self::Scope, original: &Value) -> Result<(), CmdError> {
        OriginalNumericOps::in_original_scope(self, &scope.operation)?
            .current_for_value(original)?;
        if scope.header.value().native_object_identity() != original.native_object_identity() {
            let refusal = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original Index header does not match captured argv",
                ),
            );
            let first = self.execution_refusal.get_or_insert(refusal).clone();
            return Err(CmdError::from_execution_refusal(first));
        }
        Ok(())
    }
    fn jim_index_integer(&mut self, original: &Value) -> Result<Option<i64>, CmdError> {
        original.native_integer_primary().map_err(Into::into)
    }
    fn jim_index_cached(&mut self, original: &Value) -> Result<Option<i32>, CmdError> {
        original.native_jim_index().map_err(Into::into)
    }
    fn jim_index_string(
        &mut self,
        scope: &Self::Scope,
        original: &Value,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.jim_index_check(scope, original)?;
        OriginalNumericOps::in_original_scope(self, &scope.operation)?
            .string(original, NativeStringProtocol::Jim084)
    }
    fn jim_index_expression(
        &mut self,
        scope: &Self::Scope,
        original: &Value,
        input: JimIndexExpression<'_>,
    ) -> Result<i64, CmdError> {
        self.jim_index_check(scope, original)?;
        let expression = match input {
            // The +1 is the reached SetIndexFromAny expression owner, not an
            // admission/lifetime lease around getters or old-primary cleanup.
            JimIndexExpression::Original => original.clone(),
            JimIndexExpression::EndSuffix(source) => Value::new_native_string_bytes(source),
        };
        self.jim_index_check(scope, original)?;
        let reached = self.native_jim_wide_expression(&expression);
        self.jim_index_check(scope, original)?;
        drop(expression);
        self.jim_index_check(scope, original)?;
        reached
    }
    fn jim_index_install(
        &mut self,
        scope: &Self::Scope,
        original: &Value,
        index: JimIndex,
    ) -> Result<(), CmdError> {
        let ops = OriginalNumericOps::in_original_scope(self, &scope.operation)?;
        original.install_native_jim_index_checked(index, &mut || ops.current_for_value(original))
    }
}

// One selected original length policy; only the reached conversion door varies
// between a neutral ValueOps utility and the checked physical Index operation.
pub(super) fn list_length_using<E: From<ValueError>>(
    interpreter: &mut Vm,
    value: &Value,
    current: &mut impl FnMut(&mut Vm, &Value) -> Result<(), E>,
    convert: impl FnOnce(&mut Vm, &Value) -> Result<Vec<Value>, E>,
) -> Result<usize, E> {
    current(interpreter, value)?;
    let reached = (|interpreter: &mut Vm| -> Result<usize, E> {
        use tcl_registry::native_stock_list::NativeObjectLengthAction;
        let purpose = interpreter
            .object_materialization()
            .ok_or(ValueError::CommandProtocolUnavailable("object list length"))?;
        value.check_native_header()?;
        let protocol = purpose.length_protocol();
        let class = value.stock_list_input_class();
        let canonical_empty = match value.resident_string_storage_identity() {
            Some(tcl_syntax::native_string::NativeStringStorageIdentity::CanonicalEmpty) => true,
            Some(tcl_syntax::native_string::NativeStringStorageIdentity::Unknown)
                if value
                    .resident_string_bytes()
                    .is_some_and(|bytes| bytes.is_empty()) =>
            {
                if protocol.action(class, true) != protocol.action(class, false) {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "object list length storage identity",
                    )
                    .into());
                }
                false
            }
            _ => false,
        };
        match protocol.action(class, canonical_empty).ok_or(
            ValueError::CommandProtocolUnavailable("object list length storage"),
        )? {
            NativeObjectLengthAction::CachedList => value
                .native_list_backing_in(purpose.protocol())?
                .map(|backing| backing.len())
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "object list length storage",
                ))
                .map_err(Into::into),
            NativeObjectLengthAction::Constant(length) => Ok(length),
            NativeObjectLengthAction::ConvertToList => {
                convert(interpreter, value).map(|elements| elements.len())
            }
        }
    })(interpreter);
    current(interpreter, value)?;
    reached
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_cmd_core::regex::NativeRegexSource;
    fn actual() -> Vm {
        crate::native_fixture::core(crate::environment::profile_for_dialect("jim"))
    }
    #[test]
    fn original_index_scope_refuses_another_header_before_getters_or_adoption() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual software receiver/header control; equal spelling is no receipt.
        let mut vm = actual();
        let original = Value::new_native_string_bytes(b"end".as_slice());
        let alternate = Value::new_native_string_bytes(b"end".as_slice());
        let scope = vm.jim_index_scope(&original).unwrap();
        vm.jim_index_check(&scope, &original).unwrap();
        let refused = vm.jim_index_check(&scope, &alternate).unwrap_err();
        assert_eq!(
            refused.native_execution_refusal(),
            vm.execution_refusal.as_ref()
        );
        assert_eq!(original.native_object_type_name(), "none");
        assert_eq!(alternate.native_object_type_name(), "none");
        assert_eq!(original.native_object_reference_count(), 1);
        assert_eq!(alternate.native_object_reference_count(), 1);
    }

    #[test]
    fn original_jim_container_indices_reuse_the_original_safe_expression_primary() {
        // naming.jim.original-index-conversion-and-result-object
        // docs/design/analysis/name-resolution-proofs/jim-original-index-conversion-and-result-object.md
        // Same original 17-case operands; this tests command integration, not a
        // new native command-order observation or an invented C Index protocol.
        let mut vm = actual();
        let index = Value::new_native_string_bytes(b"2*3".as_slice());
        assert_eq!(
            tcl_cmd_core::index::resolve_value(&mut vm, &index, 10).unwrap(),
            6
        );
        assert_eq!(index.native_object_type_name(), "index");
        assert_eq!(index.native_object_reference_count(), 1);
        let cached = index.native_object_snapshot();
        assert_eq!(vm.jim_regex_index(&index).unwrap(), 6);
        assert_eq!(index.native_object_snapshot(), cached);
        let relative = Value::new_native_string_bytes(b"end-2\0tail".as_slice());
        assert_eq!(
            tcl_cmd_core::index::resolve_value(&mut vm, &relative, 10).unwrap(),
            7
        );
        assert_eq!(
            tcl_cmd_core::index::resolve_value(&mut vm, &relative, 4).unwrap(),
            1
        );
        assert_eq!(relative.native_object_type_name(), "index");
        assert_eq!(
            relative.resident_string_bytes().unwrap().as_ref(),
            b"end-2\0tail"
        );
        assert_eq!(relative.native_object_reference_count(), 1);
    }
    #[test]
    fn original_jim_integer_bypass_is_distinct_from_first_string_index_conversion() {
        // naming.jim.original-index-conversion-and-result-object
        // docs/design/analysis/name-resolution-proofs/jim-original-index-conversion-and-result-object.md
        // Retained native case 8 establishes failed String conversion leaves Int;
        // subsequent Int clamp is a pinned-source/API control, not a second probe.
        let mut vm = actual();
        let integer = Value::int(i64::MAX);
        assert_eq!(
            native_jim_index::original_index(&mut vm, &integer).unwrap(),
            i32::MAX
        );
        assert_eq!(integer.native_object_type_name(), "int");
        assert!(integer.resident_string_bytes().is_none());
        let original = Value::new_native_string_bytes(b"2147483648".as_slice());
        let error = native_jim_index::original_index(&mut vm, &original).unwrap_err();
        assert!(error.native_execution_refusal().is_none());
        assert_eq!(
            error.error_code_update(),
            &tcl_cmd_core::CmdErrorCodeUpdate::Unchanged
        );
        assert_eq!(original.native_object_type_name(), "int");
        assert_eq!(
            native_jim_index::original_index(&mut vm, &original).unwrap(),
            i32::MAX
        );
        assert_eq!(original.native_object_type_name(), "int");
    }
    #[test]
    fn original_jim_index_cache_cannot_revive_a_retired_header_through_a_lifetime_lease() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software ownership control with an actual VM header and native roles.
        let mut vm = actual();
        let original = Value::new_native_string_bytes(b"end".as_slice());
        assert_eq!(
            native_jim_index::original_index(&mut vm, &original).unwrap(),
            -1
        );
        let lease = original.native_lifetime_lease();
        assert_eq!(original.native_object_reference_count(), 1);
        drop(original);
        assert!(!lease.value().native_object_is_live());
        let error = native_jim_index::original_index(&mut vm, lease.value()).unwrap_err();
        assert_eq!(
            error.native_execution_refusal(),
            vm.execution_refusal.as_ref()
        );
        assert!(error.native_execution_refusal().is_some());
        assert_eq!(lease.value().native_object_reference_count(), 0);
    }
    #[test]
    fn original_jim_index_cache_preserves_the_complete_earlier_host_cause() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut vm = actual();
        let original = Value::new_native_string_bytes(b"end".as_slice());
        assert_eq!(
            native_jim_index::original_index(&mut vm, &original).unwrap(),
            -1
        );
        let before = original.native_object_snapshot();
        let _ = vm.refuse_host_command("original Index full first cause".to_owned());
        let first = vm.execution_refusal.clone().unwrap();
        let error = native_jim_index::original_index(&mut vm, &original).unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(original.native_object_snapshot(), before);
        assert_eq!(original.native_object_reference_count(), 1);
    }
    #[test]
    fn original_jim_list_and_string_consumers_publish_the_reached_safe_expression_result() {
        // naming.jim.original-index-conversion-and-result-object
        // docs/design/analysis/name-resolution-proofs/jim-original-index-conversion-and-result-object.md
        // Integration control: the original GetWideExpr result publication is
        // independently observable before the command adapter publishes its result.
        let mut vm = actual();
        let list = Value::new_native_string_bytes(b"a b c d e f g".as_slice());
        let first = Value::new_native_string_bytes(b"2*3".as_slice());
        let last = Value::new_native_string_bytes(b"end".as_slice());
        let result = tcl_cmd_core::list::lrange(&mut vm, &list, &first, &last).unwrap();
        assert_eq!(result.string_bytes().as_ref(), b"g");
        vm.with_native_interp_result(|result| assert_eq!(result.string_bytes().as_ref(), b"6"));
        assert_eq!(first.native_object_type_name(), "index");
        let source = Value::new_native_string_bytes(b"abcdefg".as_slice());
        assert_eq!(
            tcl_cmd_core::string::index(&mut vm, &source, &first)
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"g"
        );
        assert_eq!(first.native_object_reference_count(), 1);
    }
}

#[cfg(test)]
mod replacement_native_rows629 {
    use super::*;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../runtime/rust/tests/data/native_original_string_replacement629/software_controls.rs"
    ));

    #[test]
    fn original_replacement_matches_every_native_observed_projection() {
        // naming.string.original-counted-replacement-and-index-stages
        // docs/design/analysis/name-resolution-proofs/string-original-counted-replacement-and-index-stages.md
        // Direct software API controls compare only public cache/alias/string
        // projections. Allocation-only argv transports do not pretend to reproduce
        // native EvalObjv stack/reference holds or private cache/callback chronology.
        assert_eq!(
            tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
            tcl_syntax::native_tcl_utf::NativeCPlainChar::Signed
        );
        for row in ROWS {
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect(row.dialect));
            let inputs = row.input.map(unhex);
            let subject = Value::new_native_string_bytes(inputs[0].as_slice());
            let first =
                (!row.alias_first).then(|| Value::new_native_string_bytes(inputs[1].as_slice()));
            let first = first.as_ref().unwrap_or(&subject);
            let last =
                (!row.alias_last).then(|| Value::new_native_string_bytes(inputs[2].as_slice()));
            let last = last.as_ref().unwrap_or(first);
            let replacement = (!row.alias_replacement)
                .then(|| Value::new_native_string_bytes(inputs[3].as_slice()));
            let replacement = replacement.as_ref().unwrap_or(&subject);
            let args = [&subject, first, last, replacement]
                .map(|v| v.native_lifetime_lease().into_value());
            let reached =
                tcl_cmd_core::string::replace(&mut vm, &args[..if row.omit { 3 } else { 4 }]);
            let label = (row.dialect, row.case, row.stream_sha);
            for (argument, expected) in args.iter().zip(row.argument_types) {
                assert_eq!(argument.native_object_type_name(), expected, "{label:?}");
            }
            match reached {
                Ok(result) => {
                    assert_eq!(row.code, 0, "{label:?}");
                    let snapshot = result.native_object_snapshot();
                    assert_eq!(
                        result.native_object_type_name(),
                        row.result_type,
                        "{label:?}"
                    );
                    assert_eq!(snapshot.resident.is_some(), row.resident, "{label:?}");
                    assert_eq!(
                        result.native_object_identity() == subject.native_object_identity(),
                        row.alias_subject,
                        "{label:?}"
                    );
                    if let tcl_syntax::native_object::NativeObjectCacheSnapshot::JimString {
                        num_chars,
                    } = snapshot.cache
                    {
                        assert_eq!(num_chars, row.count, "{label:?}");
                    }
                    assert_eq!(
                        vm.index_operand_bytes(&result).unwrap().as_ref(),
                        unhex(row.output),
                        "{label:?}"
                    );
                }
                Err(error) => {
                    assert_eq!(row.code, 1, "{label:?}");
                    assert!(error.native_execution_refusal().is_none(), "{label:?}");
                    assert_eq!(error.message_bytes(), unhex(row.output), "{label:?}");
                }
            }
            assert_eq!(
                vm.index_operand_bytes(&subject).unwrap().as_ref(),
                unhex(row.subject_output),
                "{label:?}"
            );
        }
    }
}

#[cfg(test)]
mod direct_binary_native_rows634a {
    use super::*;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../runtime/rust/tests/data/native_original_direct_binary_replacement634a/software_controls.rs"
    ));
    fn unhex(bytes: &str) -> Vec<u8> {
        bytes
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn original_direct_c9_binary_replacement_matches_deterministic_native_roles() {
        // naming.string.original-binary-replacement-direct-helper-sharing
        // docs/design/analysis/name-resolution-proofs/string-original-binary-replacement-direct-helper-sharing.md
        // Direct checked parsed-range API compares public cache/alias/String
        // projections. VM Value result ownership is separate from C refCount0.
        // No index-parser, EvalObjv argv-hold or native process claim is made.
        for row in DIRECT_ROWS {
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect(row.dialect));
            let dialect = vm.actual_native_invocation_dialect();
            let subject =
                Value::from_native_byte_array(Rc::from([0x41, 0, 0xff, 0x5a]), dialect).unwrap();
            let replacement =
                Value::from_native_byte_array(Rc::from([0x51, 0xfe]), dialect).unwrap();
            let extra = row.shared.then(|| subject.clone());
            if row.resident_subject {
                vm.index_operand_bytes(&subject).unwrap();
            }
            if row.resident_replacement {
                vm.index_operand_bytes(&replacement).unwrap();
            }
            assert_eq!(subject.native_object_is_shared(), row.shared);
            let subject_before = subject.native_object_snapshot();
            let reached = tcl_cmd_core::string::replace_c_parsed_range(
                &mut vm,
                &subject,
                row.first,
                row.count,
                (!row.omit).then_some(&replacement),
                row.in_place,
            );
            let label = (row.dialect, row.case, row.stream_sha);
            match row.output {
                Some(output) => {
                    let result = reached.unwrap();
                    assert_eq!(
                        result.is_same_object(&subject),
                        row.alias_subject,
                        "{label:?}"
                    );
                    assert_eq!(
                        result.is_same_object(&replacement),
                        row.alias_replacement,
                        "{label:?}"
                    );
                    let snapshot = result.native_object_snapshot();
                    assert!(
                        matches!(
                            snapshot.cache,
                            tcl_syntax::native_object::NativeObjectCacheSnapshot::ByteArray {
                                proper: true,
                                ..
                            }
                        ),
                        "{label:?}"
                    );
                    assert!(snapshot.resident.is_none(), "{label:?}");
                    assert_eq!(
                        vm.index_operand_bytes(&result).unwrap().as_ref(),
                        unhex(output),
                        "{label:?}"
                    );
                }
                None => {
                    assert!(matches!(reached.unwrap_err().native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("uninitialised original C binary replacement constructor")))), "{label:?}");
                    // Pure public inspection after refusal supplies no original
                    // getter or successful continuation credit to this Vm.
                    assert_eq!(
                        subject.native_object_snapshot(),
                        subject_before,
                        "{label:?}"
                    );
                }
            }
            if row.output.is_some() {
                assert_eq!(
                    vm.index_operand_bytes(&subject).unwrap().as_ref(),
                    unhex(row.subject_output),
                    "{label:?}"
                );
            }
            drop(extra);
        }
    }
}

#[cfg(test)]
mod indexed_binary_native_projections633 {
    use super::*;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../runtime/rust/tests/data/native_original_binary_replacement633/software_controls.rs"
    ));
    fn unhex(bytes: &str) -> Vec<u8> {
        bytes
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }
    #[test]
    fn original_indexed_binary_inputs_match_supported_native_public_projections() {
        // naming.string.original-binary-replacement-sharing-and-result-roles
        // docs/design/analysis/name-resolution-proofs/string-original-binary-replacement-sharing-and-result-roles.md
        // Every native633 input controls supported cache/type/String geometry.
        // This direct API supplies its own explicit caller references. It does
        // not copy measured EvalObjv temporary holds, fresh aliases, reference
        // counts or subject bytes after an independently allowed in-place splice.
        // Direct helper sharing/result roles have separate634a controls.
        for row in INDEXED_BINARY_ROWS {
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect(row.dialect));
            let dialect = vm.actual_native_invocation_dialect();
            let input = row.input.map(unhex);
            let subject = if row.binary_subject {
                Value::from_native_byte_array(Rc::from(input[0].as_slice()), dialect).unwrap()
            } else {
                Value::new_native_string_bytes(input[0].as_slice())
            };
            let first = Value::new_native_string_bytes(input[1].as_slice());
            let last = Value::new_native_string_bytes(input[2].as_slice());
            let replacement = if row.binary_replacement {
                Value::from_native_byte_array(Rc::from(input[3].as_slice()), dialect).unwrap()
            } else {
                Value::new_native_string_bytes(input[3].as_slice())
            };
            let extra = row.shared.then(|| subject.clone());
            if row.resident_subject {
                vm.index_operand_bytes(&subject).unwrap();
            }
            if row.resident_replacement {
                vm.index_operand_bytes(&replacement).unwrap();
            }
            let args = [&subject, &first, &last, &replacement]
                .map(|value| value.native_lifetime_lease().into_value());
            let reached =
                tcl_cmd_core::string::replace(&mut vm, &args[..if row.omit { 3 } else { 4 }]);
            let label = (row.dialect, row.case, row.stream_sha);
            for (argument, expected) in args.iter().zip(row.argument_types) {
                assert_eq!(argument.native_object_type_name(), expected, "{label:?}");
            }
            match row.output {
                Some(output) => {
                    let result = reached.unwrap();
                    assert_eq!(result.native_object_type_name(), row.result_type, "{label:?}");
                    assert_eq!(result.native_object_snapshot().resident.is_some(), row.resident, "{label:?}");
                    assert_eq!(vm.index_operand_bytes(&result).unwrap().as_ref(), unhex(output), "{label:?}");
                }
                None => assert!(matches!(reached.unwrap_err().native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("uninitialised original C binary replacement constructor")))), "{label:?}"),
            }
            drop(extra);
        }
    }
}

#[cfg(test)]
mod parsed_empty_duplicate_control {
    use super::*;
    #[test]
    fn omitted_zero_count_without_in_place_uses_original_duplicate_constructor() {
        // naming.string.original-binary-replacement-direct-helper-sharing
        // docs/design/analysis/name-resolution-proofs/string-original-binary-replacement-direct-helper-sharing.md
        // The pinned helper source includes this early duplicate branch. This
        // direct software constructor control is not one of the16 native rows.
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
        let subject = Value::new_native_string_bytes(b"a\0b".as_slice());
        let duplicate =
            tcl_cmd_core::string::replace_c_parsed_range(&mut vm, &subject, 1, 0, None, false)
                .unwrap();
        assert!(!duplicate.is_same_object(&subject));
        assert_eq!(subject.native_object_reference_count(), 1);
        assert_eq!(duplicate.resident_string_bytes().unwrap().as_ref(), b"a\0b");
    }
}

#[cfg(test)]
mod original_index_range_native_rows635 {
    use super::*;
    include!(
        "../../../../runtime/rust/tests/data/native_original_index_range635/software_controls.rs"
    );
    #[test]
    fn original_index_and_range_project_exact_measured_public_predicates() {
        // naming.string.original-counted-index-range-storage-and-results
        // docs/design/analysis/name-resolution-proofs/string-original-counted-index-range-storage-and-results.md
        // Direct API provides its own original owners. We compare public type,
        // residency, deterministic byte and alias predicates, not EvalObjv
        // temporary refcounts or unobserved native private Unicode caches.
        assert_eq!(ROWS.len(), 116);
        for row in ROWS {
            let mut ops =
                crate::native_fixture::core(crate::environment::profile_for_dialect(row.dialect));
            let dialect = ops.actual_native_invocation_dialect();
            let inputs = row.input.map(unhex);
            let subject = match row.producer {
                0 => Value::new_native_string_bytes(inputs[0].as_slice()),
                1 => {
                    Value::from_native_byte_array(Rc::from(inputs[0].as_slice()), dialect).unwrap()
                }
                2 => Value::from_native_unicode_units(
                    Rc::from([0x41, 0xd83d, 0xde00, 0x5a]),
                    dialect,
                )
                .unwrap(),
                _ => unreachable!(),
            };
            let first =
                (!row.alias_first).then(|| Value::new_native_string_bytes(inputs[1].as_slice()));
            let first = first.as_ref().unwrap_or(&subject);
            let last =
                (!row.alias_last).then(|| Value::new_native_string_bytes(inputs[2].as_slice()));
            let last = last.as_ref().unwrap_or(first);
            let extra = row.shared.then(|| subject.clone());
            if row.materialise {
                ops.index_operand_bytes(&subject).unwrap();
            }
            let args =
                [&subject, first, last].map(|value| value.native_lifetime_lease().into_value());
            let reached = if row.range {
                tcl_cmd_core::string::range(&mut ops, &args[0], &args[1], &args[2])
            } else {
                tcl_cmd_core::string::index(&mut ops, &args[0], &args[1])
            };
            let label = (row.dialect, row.case, row.stream_sha);
            for (argument, expected) in args.iter().zip(row.argument_types) {
                assert_eq!(argument.native_object_type_name(), expected, "{label:?}");
            }
            if row.code == 0 {
                let result = reached.unwrap();
                assert_eq!(
                    result.native_object_type_name(),
                    row.result_type,
                    "{label:?}"
                );
                assert_eq!(
                    result.native_object_snapshot().resident.is_some(),
                    row.resident,
                    "{label:?}"
                );
                assert_eq!(
                    result.is_same_object(&subject),
                    row.alias_subject,
                    "{label:?}"
                );
                assert_eq!(
                    ops.index_operand_bytes(&result).unwrap().as_ref(),
                    unhex(row.output),
                    "{label:?}"
                );
            } else {
                let error = reached.unwrap_err();
                assert!(error.native_execution_refusal().is_none(), "{label:?}");
                assert_eq!(error.message_bytes(), unhex(row.output), "{label:?}");
            }
            assert_eq!(
                ops.index_operand_bytes(&subject).unwrap().as_ref(),
                unhex(row.subject_output),
                "{label:?}"
            );
            drop(extra);
        }
    }
}

#[cfg(test)]
mod original_c_string_cache_controls637 {
    use super::*;
    #[test]
    fn original_c_count_retains_source_backed_eager_and_lazy_unicode_caches() {
        // naming.string.original-counted-index-range-storage-and-results
        // docs/design/analysis/name-resolution-proofs/string-original-counted-index-range-storage-and-results.md
        // Private cache shape is a pinned-source/software control, not a public
        // field in the native635 streams. Getter/result bytes have separate rows.
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect(profile));
            let subject = Value::new_native_string_bytes(b"A\xc3\xa9Z".as_slice());
            assert_eq!(vm.index_character_count(&subject).unwrap(), 3);
            let tcl_syntax::native_object::NativeObjectCacheSnapshot::String {
                unicode,
                num_chars,
                ..
            } = subject.native_object_snapshot().cache
            else {
                panic!("original String count");
            };
            assert_eq!(num_chars, Some(3));
            assert_eq!(unicode.is_some(), matches!(profile, "tcl8.4" | "tcl8.5"));
            let storage = vm.index_c_string_storage(&subject).unwrap();
            assert_eq!(storage.count, 3);
            assert_eq!(storage.bytes.unwrap().as_ref(), b"A\xc3\xa9Z");
            assert_eq!(subject.native_object_reference_count(), 1);
        }
    }
    #[test]
    fn original_c_binary_backing_receipt_does_not_keep_a_replaced_backing_alive() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual Weak backing/header control. This grants no VM general primary
        // epoch or native C84 adversarial pointer-lifetime observation.
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
        let subject = Value::from_native_byte_array(
            Rc::from(b"ABC".as_slice()),
            vm.actual_native_invocation_dialect(),
        )
        .unwrap();
        let version = tcl_dialect::TclVersion::V8_4;
        let (receipt, _) = vm
            .index_c_select_binary(&subject, version)
            .unwrap()
            .unwrap();
        vm.index_c_binary_splice(&subject, Rc::from(b"XYZ".as_slice()))
            .unwrap();
        let error = vm
            .index_c_check_binary_storage(&subject, &receipt)
            .unwrap_err();
        assert_eq!(
            error.native_execution_refusal(),
            vm.execution_refusal.as_ref()
        );
        assert_eq!(subject.native_object_reference_count(), 1);
        let tcl_syntax::native_object::NativeObjectCacheSnapshot::ByteArray { bytes, .. } =
            subject.native_object_snapshot().cache
        else {
            panic!("replacement backing");
        };
        assert_eq!(bytes.as_ref(), b"XYZ");
    }
}

#[cfg(test)]
#[path = "native_jim_index/c84_integer_result_tests.rs"]
mod c84_integer_result_tests;
