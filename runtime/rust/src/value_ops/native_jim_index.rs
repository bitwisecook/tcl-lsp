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

//! Actual Jim Index conversion on Runtime's original physical headers.
use crate::{
    interp::{native_arguments::OriginalArguments, Interp},
    obj::{self, TclObj, TclObjType},
};
use std::rc::Rc;
use tcl_cmd_core::{
    index::NativeIndexOps,
    native_jim_index::{self, NativeJimIndexObjects},
    CmdError,
};
use tcl_syntax::{
    native_jim_index::{JimIndex, JimIndexExpression},
    native_string::NativeStringProtocol,
    value::ValueError,
};

/// Allocation-only selected key transport; its constructor role is explicit.
#[derive(Clone)]
pub struct RuntimeIndexKey {
    pointer: *mut TclObj,
    state: Rc<RuntimeIndexKeyState>,
}
struct RuntimeIndexKeyState {
    _header: obj::NativeObjectLifetime,
    created: std::cell::Cell<bool>,
    result: std::cell::RefCell<Option<obj::Owned>>,
}
impl Drop for RuntimeIndexKeyState {
    fn drop(&mut self) {
        if self.created.get() {
            // Selected fresh-zero key cleanup follows the actual C Bounce door.
            // A separately published reference remains an independent owner.
            if obj::allocation_is_live(self._header.as_ptr()) {
                crate::interp::drop_fresh(self._header.as_ptr());
            }
        }
    }
}
impl RuntimeIndexKey {
    fn held(value: obj::Owned) -> Self {
        let key = Self::borrowed(value.as_ptr());
        *key.state.result.borrow_mut() = Some(value);
        key
    }
    fn borrowed(pointer: *mut TclObj) -> Self {
        Self {
            pointer,
            state: Rc::new(RuntimeIndexKeyState {
                _header: obj::NativeObjectLifetime::retain(pointer),
                created: std::cell::Cell::new(false),
                result: std::cell::RefCell::new(None),
            }),
        }
    }
}

/// Allocation-only argv retention and original operation currency.
pub struct RuntimeJimIndexScope(OriginalArguments);
/// Currency-only complete indexed command entry; no operand getter authority.
pub struct RuntimeIndexOperation(
    OriginalArguments,
    Option<crate::interp::OriginalSourceConstruction>,
);
impl RuntimeIndexOperation {
    pub(crate) fn original_currency(
        &self,
    ) -> &crate::interp::native_operation_currency::NativeOperationCurrency {
        self.0.currency()
    }
}
impl NativeIndexOps for Interp {
    type IndexOperation = RuntimeIndexOperation;
    type IndexBinaryReceipt = Option<obj::NativePrimaryReceipt>;
    type IndexKeyOwner = RuntimeIndexKey;
    fn index_borrowed_key(&mut self, original: &*mut TclObj) -> Result<RuntimeIndexKey, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        arguments.check_original_for(self, 0, *original)?;
        Ok(RuntimeIndexKey::borrowed(*original))
    }
    fn index_created_key(&mut self, created: *mut TclObj) -> Result<RuntimeIndexKey, CmdError> {
        // This exact constructor has reached; its cleanup role is retained even
        // if a subsequent original-operation check refuses.
        let key = RuntimeIndexKey::borrowed(created);
        key.state.created.set(true);
        Ok(key)
    }
    fn index_key_value(owner: &RuntimeIndexKey) -> &*mut TclObj {
        &owner.pointer
    }
    fn index_key_argument(&mut self, owner: &RuntimeIndexKey) -> Result<*mut TclObj, CmdError> {
        let arguments = OriginalArguments::enter(self, &[owner.pointer])?;
        arguments.check_original_for(self, 0, owner.pointer)?;
        Ok(owner.pointer)
    }
    fn index_key_result(
        &mut self,
        owner: &RuntimeIndexKey,
        created: bool,
        _ancestor: bool,
    ) -> Result<tcl_cmd_core::index::IndexResult<*mut TclObj>, CmdError> {
        let arguments = OriginalArguments::enter(self, &[owner.pointer])?;
        arguments.check_original_for(self, 0, owner.pointer)?;
        if created {
            owner.state.created.set(false);
        }
        Ok(if created {
            tcl_cmd_core::index::IndexResult::Created(owner.pointer)
        } else {
            tcl_cmd_core::index::IndexResult::Borrowed(owner.pointer)
        })
    }
    fn index_hold_key_result(
        &mut self,
        owner: &RuntimeIndexKey,
    ) -> Result<RuntimeIndexKey, CmdError> {
        let arguments = OriginalArguments::enter(self, &[owner.pointer])?;
        arguments.check_original_for(self, 0, owner.pointer)?;
        let held = RuntimeIndexKey::borrowed(owner.pointer);
        *held.state.result.borrow_mut() = Some(obj::Owned::retain(owner.pointer));
        Ok(held)
    }
    fn index_finish_held_key_result(
        &mut self,
        held: RuntimeIndexKey,
    ) -> Result<tcl_cmd_core::index::IndexResult<*mut TclObj>, CmdError> {
        // Currency is checked by the outer result owner after mandatory cleanup.
        // Transfer only this exact genuine child hold; no zero-header resurrection.
        let actual = held
            .state
            .result
            .borrow_mut()
            .take()
            .expect("selected child result hold");
        Ok(tcl_cmd_core::index::IndexResult::Detached(
            actual.into_native_unowned(),
        ))
    }

    fn enter_index_operation(&mut self) -> Result<Self::IndexOperation, CmdError> {
        let original = OriginalArguments::enter(self, &[])?;
        let source = self.original_source_construction()?;
        Ok(RuntimeIndexOperation(original, source))
    }
    fn check_index_operation(&mut self, original: &Self::IndexOperation) -> Result<(), CmdError> {
        original.0.check_for(self)?;
        if let Some(source) = &original.1 {
            source.check(self)?;
        }
        Ok(())
    }
    fn completed_jim_string_range(
        &mut self,
        original: &Self::IndexOperation,
        construction: &tcl_cmd_core::string::NativeJimStringRangeConstruction<'_, *mut TclObj>,
    ) -> Result<(), CmdError> {
        self.check_index_operation(original)?;
        self.record_original_namespace_info_range(original.1.as_ref(), construction)?;
        self.check_index_operation(original)
    }
    fn index_integer_result(
        &mut self,
        original: &Self::IndexOperation,
        integer: i64,
    ) -> Result<*mut TclObj, CmdError> {
        original.0.check_for(self)?;
        if self.index_c_version() != Some(tcl_dialect::TclVersion::V8_4) {
            return Ok(tcl_syntax::value::ValueOps::new_int(self, integer));
        }
        i32::try_from(integer).map_err(|_| {
            ValueError::CommandProtocolUnavailable("original C84 Index Int result width")
        })?;
        let cache = tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(integer);
        let issuer = self.native_scalar_object_issuer_for_original(&original.0)?;
        let protocol = self
            .native_invocation_dialect()
            .native_scalar_getter_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        issuer.validate_cache(&cache, protocol)?;
        original.0.check_for(self)?;
        let created = obj::Owned::fresh(obj::new_obj());
        let reached = (|| -> Result<(), CmdError> {
            obj::bind_scalar_object_context(created.as_ptr(), issuer)?;
            obj::adopt_native_scalar_cache(created.as_ptr(), cache, protocol)?;
            obj::invalidate_string(created.as_ptr());
            original.0.check_for(self)
        })();
        if let Err(first) = reached {
            self.retain_index_failure(&first);
            drop(created);
            let _ = original.0.check_for(self);
            return Err(first);
        }
        Ok(created.into_native_unowned())
    }

    fn retain_index_failure(&mut self, error: &CmdError) {
        if let Some(first) = error.native_execution_refusal() {
            self.refuse_native_execution(first.clone());
        }
    }
    fn discard_owned_index_result(&mut self, created: *mut TclObj) {
        // The shared worker supplies an actual constructor/returned transport,
        // never its Borrowed input role. Genuine Bounce preserves an independent
        // reference published by a reached callback or an original List owner.
        if obj::allocation_is_live(created) {
            crate::interp::drop_fresh(created);
        }
    }

    fn original_c_list_index(
        &mut self,
        list: &*mut TclObj,
        indices: &[*mut TclObj],
        path: bool,
    ) -> Result<Option<tcl_cmd_core::index::IndexResult<*mut TclObj>>, CmdError> {
        if self.index_c_version().is_none() {
            return Ok(None);
        }
        self.original_list_index(*list, indices, path).map(|owner| {
            Some(tcl_cmd_core::index::IndexResult::ReturnedOwnedTransport(
                owner.into_native_unowned(),
            ))
        })
    }
    fn original_c_list_range(
        &mut self,
        list: &*mut TclObj,
        first: &*mut TclObj,
        last: &*mut TclObj,
    ) -> Result<Option<tcl_cmd_core::index::IndexResult<*mut TclObj>>, CmdError> {
        if self.index_c_version().is_none() {
            return Ok(None);
        }
        self.original_list_range(*list, *first, *last).map(|owner| {
            Some(tcl_cmd_core::index::IndexResult::ReturnedOwnedTransport(
                owner.into_native_unowned(),
            ))
        })
    }
    fn index_c_version(&self) -> Option<tcl_dialect::TclVersion> {
        self.native_invocation_dialect()
            .native_string_protocol()
            .and_then(|protocol| protocol.tcl_version())
    }
    fn original_c_option_path(
        &mut self,
        original: &*mut TclObj,
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
        original: &*mut TclObj,
        command: tcl_cmd_core::native_option_index::OptionCommand,
    ) -> Result<Option<i64>, CmdError> {
        use tcl_registry::native_numeric_error::NativeNumericCommandPurpose as Purpose;
        use tcl_syntax::scalar_getter::NativeScalarGetterValue;
        let arguments = OriginalArguments::enter(self, &[*original])?;
        arguments.check_original_for(self, 0, *original)?;
        let dialect = self.native_invocation_dialect();
        let purpose = match command {
            tcl_cmd_core::native_option_index::OptionCommand::Search => Purpose::ListSearchStride,
            tcl_cmd_core::native_option_index::OptionCommand::Sort => Purpose::ListSortStride,
        };
        let selected = dialect.native_numeric_command_protocol(purpose).ok_or(
            ValueError::CommandProtocolUnavailable("original list stride purpose"),
        )?;
        dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let reached = {
            let host = self.host();
            crate::typed_value::native_scalar_getter_with_environment_and_currency(
                *original,
                dialect,
                selected.getter_kind(),
                host.numeric_environment(),
                Some(arguments.currency()),
            )
        };
        // Check the captured original full cause before classifying ValueError.
        arguments.check_original_for(self, 0, *original)?;
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
    }
    fn original_list_number(
        &mut self,
        original: &*mut TclObj,
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
        let arguments = OriginalArguments::enter(self, &[*original])?;
        arguments.check_original_for(self, 0, *original)?;
        let dialect = self.native_invocation_dialect();
        let selected = dialect.native_numeric_command_protocol(purpose).ok_or(
            ValueError::CommandProtocolUnavailable("original numeric list command purpose"),
        )?;
        dialect
            .native_string_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let reached = {
            let host = self.host();
            crate::typed_value::native_scalar_getter_with_environment_and_currency(
                *original,
                dialect,
                selected.getter_kind(),
                host.numeric_environment(),
                Some(arguments.currency()),
            )
        };
        // Full saved cause wins before projecting the scalar worker's legacy ValueError.
        arguments.check_original_for(self, 0, *original)?;
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
    }
    fn index_c84_sort_member_hold(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<RuntimeIndexKey, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        arguments.check_original_for(self, 0, *original)?;
        if self.index_c_version() != Some(tcl_dialect::TclVersion::V8_4) {
            return Err(
                ValueError::CommandProtocolUnavailable("original C84 sort member role").into(),
            );
        }
        let held = RuntimeIndexKey::borrowed(*original);
        *held.state.result.borrow_mut() = Some(obj::Owned::retain(*original));
        Ok(held)
    }
    fn index_sort_copy(&mut self, original: &*mut TclObj) -> Result<RuntimeIndexKey, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original sort List copy issuer",
            ))?;
        crate::list::native_list_copy_using(
            *original,
            protocol,
            &|| arguments.check_original_for(self, 0, *original),
            &|error| arguments.retain_refusal(error.into()),
        )
        .map(RuntimeIndexKey::held)
    }
    fn index_sort_duplicate(
        &mut self,
        original: &*mut TclObj,
        only_shared: bool,
    ) -> Result<RuntimeIndexKey, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        arguments.check_original_for(self, 0, *original)?;
        if only_shared && !obj::is_shared(*original) {
            return Ok(RuntimeIndexKey::borrowed(*original));
        }
        let copied = obj::duplicate_with_stage_check::<CmdError>(
            *original,
            &mut || arguments.check_original_for(self, 0, *original),
            &mut |error| {
                arguments.retain_refusal(error.clone());
            },
        )?;
        if self.index_jim_sort_selected() {
            // Jim_DuplicateObj is fresh-zero; Eval/result takes its own hold.
            self.index_created_key(copied)
        } else {
            Ok(RuntimeIndexKey::held(obj::Owned::fresh(copied)))
        }
    }
    fn index_sort_list(
        &mut self,
        members: Vec<*mut TclObj>,
        held: bool,
    ) -> Result<RuntimeIndexKey, CmdError> {
        let arguments = OriginalArguments::enter(self, &members)?;
        arguments.check_for(self)?;
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original sort List issuer",
            ))?;
        let list = crate::list::new_list_obj_native(&members, protocol);
        if held {
            Ok(RuntimeIndexKey::held(obj::Owned::fresh(list)))
        } else {
            self.index_created_key(list)
        }
    }
    fn index_sort_append(
        &mut self,
        list: &RuntimeIndexKey,
        members: &[*mut TclObj],
    ) -> Result<(), CmdError> {
        let arguments = OriginalArguments::enter(self, &[list.pointer])?;
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original sort List append issuer",
            ))?;
        crate::list::append_prepared_native_elements_using(
            list.pointer,
            members,
            protocol,
            &|| arguments.check_original_for(self, 0, list.pointer),
            &|error| arguments.retain_refusal(error.into()),
        )
    }
    fn index_sort_replace(
        &mut self,
        list: &RuntimeIndexKey,
        first: usize,
        delete: usize,
        members: &[*mut TclObj],
    ) -> Result<(), CmdError> {
        let arguments = OriginalArguments::enter(self, &[list.pointer])?;
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original sort List replacement issuer",
            ))?;
        crate::list::replace_prepared_native_range_using(
            list.pointer,
            first,
            delete,
            members,
            protocol,
            &|| arguments.check_original_for(self, 0, list.pointer),
            &|error| arguments.retain_refusal(error.into()),
        )
    }
    fn index_jim_sort_selected(&self) -> bool {
        self.native_invocation_dialect()
            .native_scalar_getter_protocol()
            .is_some_and(|selected| selected.is_jim084())
    }
    fn index_native_sort<E>(
        &mut self,
        indices: &mut [usize],
        compare: &mut impl FnMut(&mut Self, usize, usize) -> Result<core::cmp::Ordering, E>,
    ) -> Result<(), tcl_cmd_core::index::NativeSortError<E>> {
        let host = self.host();
        let environment = host.native_list_sorting_environment().ok_or_else(|| {
            tcl_cmd_core::index::NativeSortError::Operation(CmdError::from(
                ValueError::CommandProtocolUnavailable("original local C list sorting capability"),
            ))
        })?;
        tcl_cmd_core::index::native_sort_with_host(self, environment, indices, compare)
    }
    fn original_c_index(
        &mut self,
        original: &*mut TclObj,
        length: usize,
    ) -> Result<Option<i64>, CmdError> {
        let end = i64::try_from(length)
            .map_err(|_| ValueError::CommandProtocolUnavailable("original C Index length width"))?
            - 1;
        tcl_cmd_core::native_list_index::original_index(self, original, end, true)
    }
    fn index_c_select_binary(
        &mut self,
        original: &*mut TclObj,
        version: tcl_dialect::TclVersion,
    ) -> Result<Option<(Option<obj::NativePrimaryReceipt>, Rc<[u8]>)>, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        arguments.check_original_for(self, 0, *original)?;
        if self.index_c_version() != Some(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "original C binary selection release",
            )
            .into());
        }
        let snapshot = obj::native_object_snapshot(*original)?;
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
        if crate::bytearray::native_string_protocol(*original)
            != self.native_invocation_dialect().native_string_protocol()
        {
            return Err(ValueError::CommandProtocolUnavailable(
                "original C binary selection issuer",
            )
            .into());
        }
        Ok(Some((
            (version == tcl_dialect::TclVersion::V8_4)
                .then(|| obj::NativePrimaryReceipt::capture(*original))
                .transpose()?,
            bytes,
        )))
    }
    fn index_c_check_binary_storage(
        &mut self,
        original: &*mut TclObj,
        receipt: &Option<obj::NativePrimaryReceipt>,
    ) -> Result<(), CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        arguments.check_original_for(self, 0, *original)?;
        receipt
            .as_ref()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "unselected C84 binary storage receipt",
            ))?
            .check(*original)
            .map_err(|error| arguments.retain_refusal(CmdError::from(error)))
    }
    fn index_c_binary_bytes(&mut self, original: &*mut TclObj) -> Result<Rc<[u8]>, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        let dialect = self.native_invocation_dialect();
        let recipe = dialect.byte_array_string_recipe(None).ok_or(
            ValueError::CommandProtocolUnavailable("original C binary getter issuer"),
        )?;
        let policy =
            dialect
                .binary_data_conversion()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original C binary getter conversion",
                ))?;
        crate::bytearray::native_binary_bytes_checked(
            *original,
            policy,
            recipe,
            &mut || arguments.check_original_for(self, 0, *original),
            &mut || arguments.bytes(0).map(|bytes| bytes.to_vec()),
        )
        .map(Rc::from)
    }
    fn index_c_string_storage(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<tcl_cmd_core::string::NativeCStringStorage, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C String storage issuer",
            ))?;
        obj::native_c_string_storage_checked(
            *original,
            protocol,
            &mut || arguments.check_original_for(self, 0, *original),
            &mut || arguments.bytes(0),
        )
    }
    fn index_c_plain_char(
        &mut self,
    ) -> Result<tcl_syntax::native_tcl_utf::NativeCPlainChar, CmdError> {
        let arguments = OriginalArguments::enter(self, &[])?;
        arguments.check_for(self)?;
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
    ) -> Result<*mut TclObj, CmdError> {
        if count != bytes.len() {
            return Err(ValueError::CommandProtocolUnavailable(
                "original counted C String slice extent",
            )
            .into());
        }
        let arguments = OriginalArguments::enter(self, &[])?;
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original counted C String constructor",
            ))?;
        arguments.check_for(self)?;
        let created = obj::new_string_bytes(&bytes);
        let reached = obj::set_native_append_string_checked(
            created,
            protocol,
            Some((
                bytes,
                tcl_syntax::native_string::NativeStringStorageIdentity::Allocated,
            )),
            Some(count),
            None,
            &mut || {
                arguments.check_for(self)?;
                obj::check_native_liveness(created).map_err(CmdError::from)
            },
        );
        match reached {
            Ok(()) => Ok(created),
            Err(error) => {
                arguments.retain_refusal(error.clone());
                crate::interp::drop_fresh(created);
                Err(error)
            }
        }
    }
    fn index_c_unicode_units(&mut self, original: &*mut TclObj) -> Result<Rc<[u32]>, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        let protocol = self
            .native_invocation_dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C Unicode issuer",
            ))?;
        obj::native_unicode_units_checked(
            *original,
            protocol,
            &mut || arguments.check_original_for(self, 0, *original),
            &mut || arguments.bytes(0),
        )
    }
    fn index_c_pure_binary(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<Option<Rc<[u8]>>, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        arguments.check_original_for(self, 0, *original)?;
        let snapshot = obj::native_object_snapshot(*original)?;
        if let tcl_syntax::native_object::NativeObjectCacheSnapshot::ByteArray {
            bytes,
            proper: true,
        } = snapshot.cache
        {
            if crate::bytearray::native_string_protocol(*original)
                != self.native_invocation_dialect().native_string_protocol()
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
    fn index_c_original_is_shared(&mut self, original: &*mut TclObj) -> Result<bool, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        arguments.check_original_for(self, 0, *original)?;
        // SAFETY: exact live original; allocation leases leave native count unchanged.
        Ok(unsafe { (**original).ref_count > 1 })
    }
    fn index_c_duplicate_original(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<*mut TclObj, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        obj::duplicate_with_stage_check(
            *original,
            &mut || arguments.check_original_for(self, 0, *original),
            &mut |error| {
                arguments.retain_refusal(error.clone());
            },
        )
    }
    fn index_c_binary_result(&mut self, bytes: Rc<[u8]>) -> Result<*mut TclObj, CmdError> {
        let arguments = OriginalArguments::enter(self, &[])?;
        let recipe = self
            .native_invocation_dialect()
            .byte_array_string_recipe(None)
            .filter(|recipe| recipe.protocol().tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C binary replacement constructor",
            ))?;
        arguments.check_for(self)?;
        Ok(crate::bytearray::new_byte_array(&bytes, recipe))
    }
    fn index_c_binary_splice(
        &mut self,
        original: &*mut TclObj,
        bytes: Rc<[u8]>,
    ) -> Result<(), CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        let recipe = self
            .native_invocation_dialect()
            .byte_array_string_recipe(None)
            .filter(|recipe| {
                Some(recipe.protocol()) == crate::bytearray::native_string_protocol(*original)
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C binary replacement splice",
            ))?;
        crate::bytearray::set_native_append_bytes_checked(*original, &bytes, recipe, &mut || {
            arguments.check_original_for(self, 0, *original)
        })
    }
    fn index_c_append_object(
        &mut self,
        result: &*mut TclObj,
        original: &*mut TclObj,
    ) -> Result<(), CmdError> {
        super::native_indexed_append::append_object(self, *result, *original)
    }
    fn index_c_append_unicode(
        &mut self,
        result: &*mut TclObj,
        units: &[u32],
    ) -> Result<(), CmdError> {
        super::native_indexed_append::append_unicode(self, *result, units)
    }
    fn index_character_count(&mut self, original: &*mut TclObj) -> Result<usize, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
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
        obj::native_character_count_checked(
            *original,
            protocol,
            representation,
            &mut || arguments.check_original_for(self, 0, *original),
            &mut || arguments.bytes(0),
        )
    }
    fn index_list_elements(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<Vec<*mut TclObj>, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*original])?;
        let protocol = self.native_invocation_dialect().native_string_materialization(
            Some(tcl_registry::native_string_materialization::LogicalStringProvider::Tcl84CoreSimulation),
        ).ok_or(ValueError::CommandProtocolUnavailable("original Index container List issuer"))?.protocol();
        if protocol.is_jim084() {
            let context = self.native_jim_object_context();
            arguments.check_for(self)?;
            let context = context?;
            let bound = crate::native_source::bind_context(*original, &context);
            arguments.check_original_for(self, 0, *original)?;
            bound?;
        }
        crate::list::list_elements_native_using(
            *original,
            protocol,
            &|| arguments.check_original_for(self, 0, *original),
            &|error| arguments.retain_refusal(error.into()),
        )
    }

    fn index_list_length(&mut self, value: &*mut TclObj) -> Result<usize, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*value])?;
        list_length_using(
            self,
            value,
            &mut |actual, original| arguments.check_original_for(actual, 0, *original),
            |interpreter, original| interpreter.index_list_elements(original),
        )
    }
    fn index_list_member(
        &mut self,
        value: &*mut TclObj,
        index: usize,
    ) -> Result<Option<tcl_cmd_core::index::IndexResult<*mut TclObj>>, CmdError> {
        let arguments = OriginalArguments::enter(self, &[*value])?;
        let reached = if crate::native_arithseries::is_series(*value) {
            let protocol = self
                .native_invocation_dialect()
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "abstract native Index",
                ))?;
            crate::native_arithseries::index_in(*value, index, protocol)
                .map(|member| member.map(tcl_cmd_core::index::IndexResult::Created))
                .map_err(Into::into)
        } else {
            self.index_list_elements(value).map(|members| {
                members
                    .into_iter()
                    .nth(index)
                    .map(tcl_cmd_core::index::IndexResult::Borrowed)
            })
        };
        arguments.check_original_for(self, 0, *value)?;
        reached
    }
    fn original_jim_index_path(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<Option<Vec<*mut TclObj>>, CmdError> {
        if self.native_invocation_dialect().native_string_protocol()
            != Some(NativeStringProtocol::Jim084)
        {
            return Err(
                ValueError::CommandProtocolUnavailable("original Jim Index path issuer").into(),
            );
        }
        self.index_list_elements(original).map(Some)
    }
    fn original_jim_index(&mut self, original: &*mut TclObj) -> Result<Option<i32>, CmdError> {
        native_jim_index::original_index(self, original).map(Some)
    }
    fn index_operand_bytes(&mut self, original: &*mut TclObj) -> Result<Rc<[u8]>, CmdError> {
        OriginalArguments::enter(self, &[*original])?.bytes(0)
    }
    fn textual_jim_index(&mut self, source: &[u8]) -> Result<Option<i32>, CmdError> {
        let original = obj::Owned::fresh(obj::new_string_bytes(source));
        native_jim_index::original_index(self, &original.as_ptr()).map(Some)
    }
}
impl NativeJimIndexObjects for Interp {
    type Scope = RuntimeJimIndexScope;
    fn jim_index_scope(&mut self, original: &*mut TclObj) -> Result<Self::Scope, CmdError> {
        let scope = OriginalArguments::enter(self, &[*original])?;
        if self.native_invocation_dialect().native_string_protocol()
            != Some(NativeStringProtocol::Jim084)
        {
            return Err(ValueError::CommandProtocolUnavailable("original Jim Index issuer").into());
        }
        Ok(RuntimeJimIndexScope(scope))
    }
    fn jim_index_check(
        &mut self,
        scope: &Self::Scope,
        original: &*mut TclObj,
    ) -> Result<(), CmdError> {
        scope.0.check_original_for(self, 0, *original)
    }
    fn jim_index_integer(&mut self, original: &*mut TclObj) -> Result<Option<i64>, CmdError> {
        obj::check_native_liveness(*original)?;
        Ok(
            core::ptr::eq(obj::obj_type_ptr(*original), &obj::TCL_INT_TYPE)
                .then(|| i64::from_ne_bytes(obj::internal_rep(*original).to_ne_bytes())),
        )
    }
    fn jim_index_cached(&mut self, original: &*mut TclObj) -> Result<Option<i32>, CmdError> {
        obj::check_native_liveness(*original)?;
        Ok(
            core::ptr::eq(obj::obj_type_ptr(*original), &JIM_INDEX_TYPE).then(|| {
                tcl_syntax::number::native_int32_low_bits(i64::from_ne_bytes(
                    obj::internal_rep(*original).to_ne_bytes(),
                ))
            }),
        )
    }
    fn jim_index_string(
        &mut self,
        scope: &Self::Scope,
        original: &*mut TclObj,
    ) -> Result<Rc<[u8]>, CmdError> {
        self.jim_index_check(scope, original)?;
        scope.0.bytes(0)
    }
    fn jim_index_expression(
        &mut self,
        scope: &Self::Scope,
        original: &*mut TclObj,
        input: JimIndexExpression<'_>,
    ) -> Result<i64, CmdError> {
        self.jim_index_check(scope, original)?;
        let expression = match input {
            // Genuine GetWideExpr temporary reference, after original String.
            JimIndexExpression::Original => obj::Owned::retain(*original),
            JimIndexExpression::EndSuffix(source) => {
                obj::Owned::fresh(obj::new_string_bytes(source))
            }
        };
        self.jim_index_check(scope, original)?;
        let reached = crate::builtins::native_jim_wide_expression(self, expression.as_ptr());
        self.jim_index_check(scope, original)?;
        drop(expression);
        self.jim_index_check(scope, original)?;
        reached
    }
    fn jim_index_install(
        &mut self,
        scope: &Self::Scope,
        original: &*mut TclObj,
        index: JimIndex,
    ) -> Result<(), CmdError> {
        self.jim_index_check(scope, original)?;
        obj::change_type_with_stage_check(
            *original,
            &JIM_INDEX_TYPE,
            u64::from(u32::from_ne_bytes(index.0.to_ne_bytes())),
            &mut || self.jim_index_check(scope, original),
        )?;
        self.jim_index_check(scope, original)
    }
}
extern "C" fn update_jim_index(original: *mut TclObj) {
    let index = JimIndex(tcl_syntax::number::native_int32_low_bits(
        i64::from_ne_bytes(obj::internal_rep(original).to_ne_bytes()),
    ));
    // SAFETY: exact live scalar descriptor owns no child or callback payload.
    unsafe {
        obj::set_string_rep(original, &index.string_bytes());
    }
}
static JIM_INDEX_TYPE: TclObjType = TclObjType {
    name: c"index".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_jim_index),
    set_from_any_proc: None,
};

// One selected original length policy; only the reached conversion door varies
// between a neutral ValueOps utility and the checked physical Index operation.
pub(crate) fn list_length_using<E: From<ValueError>>(
    interpreter: &mut Interp,
    value: &*mut TclObj,
    current: &mut impl FnMut(&mut Interp, &*mut TclObj) -> Result<(), E>,
    convert: impl FnOnce(&mut Interp, &*mut TclObj) -> Result<Vec<*mut TclObj>, E>,
) -> Result<usize, E> {
    current(interpreter, value)?;
    let reached = (|interpreter: &mut Interp| -> Result<usize, E> {
        obj::check_native_liveness(*value)?;
        if crate::native_arithseries::is_series(*value) {
            let protocol = interpreter
                .native_invocation_dialect()
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "abstract native Length",
                ))?;
            return crate::native_arithseries::length_in(*value, protocol)?
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "abstract native Length",
                ))
                .map_err(Into::into);
        }
        use tcl_registry::native_stock_list::{
            LogicalListLengthProvider, NativeObjectLengthAction,
        };
        let protocol = interpreter
            .native_invocation_dialect()
            .object_length_protocol(Some(LogicalListLengthProvider::Tcl84CoreSimulation))
            .ok_or(ValueError::CommandProtocolUnavailable("object list length"))?;
        let action = protocol
            .action(
                obj::stock_list_input_class(*value),
                obj::has_canonical_empty_string(*value),
            )
            .ok_or(ValueError::CommandProtocolUnavailable(
                "object list length storage",
            ))?;
        match action {
            NativeObjectLengthAction::CachedList => crate::list::cached_length(*value)
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
    use std::cell::{Cell, RefCell};
    #[cfg(have_tommath)]
    use tcl_cmd_core::regex::NativeRegexSource;
    thread_local! {
        static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
        static REFUSE: Cell<u8> = const { Cell::new(0) };
        static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    }
    fn actual() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    struct Current;
    impl Drop for Current {
        fn drop(&mut self) {
            crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        }
    }
    fn refuse_reached(stage: u8, original: *mut TclObj) {
        REFS.with(|refs| refs.set(unsafe { (*original).ref_count }));
        let selected = REFUSE.with(Cell::get);
        if selected == stage || selected == stage + 10 {
            let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
                .unwrap()
                .clone();
            if selected == stage {
                interpreter.refuse_host_command("original Index reached callback");
            } else {
                let original_context = interpreter.runtime_context();
                let mut changed = original_context.clone();
                changed.packages = vec![(
                    "original-index-context-control".to_owned(),
                    "1.0".to_owned(),
                )];
                interpreter.pin_context(&changed).unwrap();
                interpreter.pin_context(&original_context).unwrap();
            }
        }
    }
    extern "C" fn index_updater(original: *mut TclObj) {
        EVENTS.with(|events| events.borrow_mut().push("index-getter"));
        refuse_reached(1, original);
        unsafe { obj::set_native_updater_string_rep(original, b"end", false) };
    }
    extern "C" fn index_free(original: *mut TclObj) {
        EVENTS.with(|events| events.borrow_mut().push("index-free"));
        refuse_reached(2, original);
    }
    extern "C" fn list_updater(original: *mut TclObj) {
        EVENTS.with(|events| events.borrow_mut().push("container-getter"));
        unsafe { obj::set_native_updater_string_rep(original, b"a b c", false) };
    }
    static INDEX: TclObjType = TclObjType {
        name: c"indexOwnerControl".as_ptr(),
        free_int_rep_proc: Some(index_free),
        dup_int_rep_proc: None,
        update_string_proc: Some(index_updater),
        set_from_any_proc: None,
    };
    static CONTAINER: TclObjType = TclObjType {
        name: c"indexContainerControl".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(list_updater),
        set_from_any_proc: None,
    };
    #[cfg(have_tommath)]
    #[test]
    fn original_jim_container_and_regex_indices_share_one_original_expression_cache() {

        let mut interp = actual();
        let original = obj::Owned::fresh(obj::new_string_bytes(b"2*3"));
        assert_eq!(
            tcl_cmd_core::index::resolve_value(&mut interp, &original.as_ptr(), 10).unwrap(),
            6
        );
        assert!(core::ptr::eq(
            obj::obj_type_ptr(original.as_ptr()),
            &JIM_INDEX_TYPE
        ));
        assert_eq!(interp.jim_regex_index(&original.as_ptr()).unwrap(), 6);
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        let relative = obj::Owned::fresh(obj::new_string_bytes(b"end-2\0tail"));
        assert_eq!(
            tcl_cmd_core::index::resolve_value(&mut interp, &relative.as_ptr(), 10).unwrap(),
            7
        );
        assert_eq!(
            tcl_cmd_core::index::resolve_value(&mut interp, &relative.as_ptr(), 4).unwrap(),
            1
        );
        assert_eq!(obj::bytes_of(relative.as_ptr()), b"end-2\0tail");
        assert_eq!(unsafe { (*relative.as_ptr()).ref_count }, 1);
    }
    #[test]
    fn original_jim_index_integer_bypass_and_bad_string_preserve_primary_and_error_code_stage() {

        // Native case 8 observes failed String->Int; second call clamp is source/API control.
        let mut interp = actual();
        let original = obj::Owned::fresh(obj::new_string_bytes(b"2147483648"));
        let error = native_jim_index::original_index(&mut interp, &original.as_ptr()).unwrap_err();
        assert_eq!(
            error.error_code_update(),
            &tcl_cmd_core::CmdErrorCodeUpdate::Unchanged
        );
        assert!(error.native_execution_refusal().is_none());
        assert!(core::ptr::eq(
            obj::obj_type_ptr(original.as_ptr()),
            &obj::TCL_INT_TYPE
        ));
        assert_eq!(
            native_jim_index::original_index(&mut interp, &original.as_ptr()).unwrap(),
            i32::MAX
        );
        assert!(core::ptr::eq(
            obj::obj_type_ptr(original.as_ptr()),
            &obj::TCL_INT_TYPE
        ));
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
    }
    #[test]
    fn original_jim_index_getter_and_free_callbacks_preserve_full_cause_without_protective_references(
    ) {

        // Actual extension callbacks are software controls; no native hostile-callback chronology claim.
        for stage in [1, 2] {
            let mut interp = actual();
            let original = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            EVENTS.with(|events| events.borrow_mut().clear());
            REFUSE.with(|selected| selected.set(stage));
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
            let _current = Current;
            let error =
                native_jim_index::original_index(&mut interp, &original.as_ptr()).unwrap_err();
            let first = interp.native_execution_refusal().unwrap();
            assert_eq!(error.native_execution_refusal(), Some(&first));
            assert_eq!(REFS.with(Cell::get), 1);
            assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
            assert!(!core::ptr::eq(
                obj::obj_type_ptr(original.as_ptr()),
                &JIM_INDEX_TYPE
            ));
            assert_eq!(interp.get_obj_result(), prior);
            let expected = if stage == 1 {
                vec!["index-getter"]
            } else {
                vec!["index-getter", "index-free"]
            };
            assert_eq!(EVENTS.with(|events| events.borrow().clone()), expected);
            REFUSE.with(|selected| selected.set(0));
        }
    }
    #[test]
    fn original_jim_list_indices_precede_list_getters_but_string_range_gets_its_subject_first() {

        // Pinned-source order is checked with actual Runtime extension headers.
        // The original 17-case native probe measures conversion, not this callback program.
        for mode in 0..4 {
            let mut interp = actual();
            let index = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
            let container = obj::Owned::fresh(obj::alloc_typed(&CONTAINER, 0));
            EVENTS.with(|events| events.borrow_mut().clear());
            REFUSE.with(|selected| selected.set(0));
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
            let _current = Current;
            let result = match mode {
                0 => tcl_cmd_core::list::lindex_flat(
                    &mut interp,
                    &container.as_ptr(),
                    &[index.as_ptr()],
                ),
                1 => tcl_cmd_core::list::lrange(
                    &mut interp,
                    &container.as_ptr(),
                    &index.as_ptr(),
                    &index.as_ptr(),
                ),
                2 => tcl_cmd_core::string::index(
                    &mut interp,
                    &container.as_ptr(),
                    &index.as_ptr(),
                    tcl_syntax::number::NumberSyntax::Tcl90,
                ),
                _ => tcl_cmd_core::string::range(
                    &mut interp,
                    &container.as_ptr(),
                    &index.as_ptr(),
                    &index.as_ptr(),
                    tcl_syntax::number::NumberSyntax::Tcl90,
                ),
            }
            .unwrap();
            interp.set_result(result);
            let expected = if mode == 3 {
                vec!["container-getter", "index-getter", "index-free"]
            } else {
                vec!["index-getter", "index-free", "container-getter"]
            };
            assert_eq!(
                EVENTS.with(|events| events.borrow().clone()),
                expected,
                "mode {mode}"
            );
            assert_eq!(REFS.with(Cell::get), 1);
            assert_eq!(unsafe { (*index.as_ptr()).ref_count }, 1);
        }
    }
    #[test]
    fn original_jim_index_lifetime_lease_keeps_allocation_without_reviving_a_native_header() {

        let mut interp = actual();
        let original = obj::Owned::fresh(obj::new_string_bytes(b"end"));
        let pointer = original.as_ptr();
        assert_eq!(
            native_jim_index::original_index(&mut interp, &pointer).unwrap(),
            -1
        );
        let allocation = obj::NativeObjectLifetime::retain(pointer);
        drop(original);
        assert!(!obj::allocation_is_live(pointer));
        let error = native_jim_index::original_index(&mut interp, &pointer).unwrap_err();
        assert!(error.native_execution_refusal().is_some());
        drop(allocation);
    }
    #[test]
    fn original_indexed_string_count_refuses_before_later_indices_and_cache_adoption() {

        // Genuine Runtime extension callbacks test this checked API. Native
        // Jim command/cache observations remain the separate pinned17 rows.
        for stage in [1, 2, 11, 12] {
            let mut interp = actual();
            let subject = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
            let first = obj::Owned::fresh(obj::new_string_bytes(b"0"));
            let last = obj::Owned::fresh(obj::new_string_bytes(b"end"));
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            EVENTS.with(|events| events.borrow_mut().clear());
            REFUSE.with(|selected| selected.set(stage));
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
            let _current = Current;
            let failure = tcl_cmd_core::string::range(
                &mut interp,
                &subject.as_ptr(),
                &first.as_ptr(),
                &last.as_ptr(),
                tcl_syntax::number::NumberSyntax::Tcl90,
            )
            .unwrap_err();
            let original = interp.native_execution_refusal().unwrap();
            assert_eq!(failure.native_execution_refusal(), Some(&original));
            assert_eq!(interp.get_obj_result(), prior);
            assert!(!core::ptr::eq(
                obj::obj_type_ptr(first.as_ptr()),
                &JIM_INDEX_TYPE
            ));
            assert!(!core::ptr::eq(
                obj::obj_type_ptr(last.as_ptr()),
                &JIM_INDEX_TYPE
            ));
            assert!(!matches!(
                obj::native_object_snapshot(subject.as_ptr()).unwrap().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
            ));
            assert_eq!(unsafe { (*subject.as_ptr()).ref_count }, 1);
            assert_eq!(REFS.with(Cell::get), 1);
            REFUSE.with(|selected| selected.set(0));
        }
    }

    #[test]
    fn original_indexed_list_conversion_refuses_before_backing_adoption() {

        // Direct shared List API, original foreign updater/free, same actual
        // entered currency. No native adversarial-callback source claim.
        for stage in [1, 2, 11, 12] {
            let mut interp = actual();
            let subject = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
            let index = obj::Owned::fresh(obj::new_string_bytes(b"end"));
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            EVENTS.with(|events| events.borrow_mut().clear());
            REFUSE.with(|selected| selected.set(stage));
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut interp);
            let _current = Current;
            let failure =
                tcl_cmd_core::list::lindex_flat(&mut interp, &subject.as_ptr(), &[index.as_ptr()])
                    .unwrap_err();
            let original = interp.native_execution_refusal().unwrap();
            assert_eq!(failure.native_execution_refusal(), Some(&original));
            assert_eq!(interp.get_obj_result(), prior);
            assert!(crate::list::native_list_backing(subject.as_ptr()).is_none());
            assert!(core::ptr::eq(
                obj::obj_type_ptr(index.as_ptr()),
                &JIM_INDEX_TYPE
            ));
            assert_eq!(unsafe { (*subject.as_ptr()).ref_count }, 1);
            REFUSE.with(|selected| selected.set(0));
        }
    }
}

#[cfg(all(test, have_tommath))]
mod replacement_native_rows629 {
    use super::*;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/native_original_string_replacement629/software_controls.rs"
    ));

    fn kind(value: *mut TclObj) -> String {
        let descriptor = obj::obj_type_ptr(value);
        if descriptor.is_null() {
            return "none".to_owned();
        }
        // SAFETY: selected live original/fresh result descriptor.
        unsafe {
            std::ffi::CStr::from_ptr((*descriptor).name)
                .to_str()
                .unwrap()
                .to_owned()
        }
    }

    #[test]
    fn original_replacement_matches_every_native_observed_projection() {

        // Direct software API controls compare retained public cache/alias/string
        // projections, not EvalObjv stack holds, guest-result publication, native
        // process counters, private C fields, callback chronology or teardown.
        // C84/85 signed-char observations apply to this actual local signed backend.
        assert_eq!(
            tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
            tcl_syntax::native_tcl_utf::NativeCPlainChar::Signed
        );
        for row in ROWS {
            let mut interpreter = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(row.dialect),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let inputs = row.input.map(unhex);
            let subject = obj::Owned::fresh(obj::new_string_bytes(&inputs[0]));
            let first =
                (!row.alias_first).then(|| obj::Owned::fresh(obj::new_string_bytes(&inputs[1])));
            let first_pointer = first.as_ref().map_or(subject.as_ptr(), obj::Owned::as_ptr);
            let last =
                (!row.alias_last).then(|| obj::Owned::fresh(obj::new_string_bytes(&inputs[2])));
            let last_pointer = last.as_ref().map_or(first_pointer, obj::Owned::as_ptr);
            let replacement = (!row.alias_replacement)
                .then(|| obj::Owned::fresh(obj::new_string_bytes(&inputs[3])));
            let replacement_pointer = replacement
                .as_ref()
                .map_or(subject.as_ptr(), obj::Owned::as_ptr);
            let args = [
                subject.as_ptr(),
                first_pointer,
                last_pointer,
                replacement_pointer,
            ];
            let reached = tcl_cmd_core::string::replace(
                &mut interpreter,
                &args[..if row.omit { 3 } else { 4 }],
                tcl_syntax::number::NumberSyntax::Tcl90,
            );
            let label = (row.dialect, row.case, row.stream_sha);
            for (argument, expected) in args.iter().zip(row.argument_types) {
                assert_eq!(kind(*argument), expected, "{label:?}");
            }
            match reached {
                Ok(result) => {
                    assert_eq!(row.code, 0, "{label:?}");
                    let result = obj::Owned::retain(result); // genuine returned-result reference
                    let snapshot = obj::native_object_snapshot(result.as_ptr()).unwrap();
                    assert_eq!(kind(result.as_ptr()), row.result_type, "{label:?}");
                    assert_eq!(snapshot.resident.is_some(), row.resident, "{label:?}");
                    assert_eq!(
                        result.as_ptr() == subject.as_ptr(),
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
                        interpreter
                            .native_object_string_bytes(result.as_ptr())
                            .unwrap()
                            .as_ref(),
                        unhex(row.output),
                        "{label:?}"
                    );
                }
                Err(error) => {
                    assert_eq!(row.code, 1, "{label:?}");
                    assert!(error.native_execution_refusal().is_none(), "{label:?}");
                    assert_eq!(error.message_bytes(), unhex(row.output), "{label:?}");
                    // CmdCore returns a Guest error object; it does not publish
                    // the original native interpreter's error-result header.
                }
            }
            assert_eq!(
                interpreter
                    .native_object_string_bytes(subject.as_ptr())
                    .unwrap()
                    .as_ref(),
                unhex(row.subject_output),
                "{label:?}"
            );
        }
    }
}

#[cfg(all(test, have_tommath))]
mod direct_binary_native_rows634a {
    use super::*;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/native_original_direct_binary_replacement634a/software_controls.rs"
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

        // Direct parsed-range API, exact input constructor/native references;
        // pre-result-getter public primaries/aliases and counted String bytes.
        // This test does not invoke EvalObjv, parse indices or model its argv holds.
        for row in DIRECT_ROWS {
            let mut interpreter = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(row.dialect),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let dialect = interpreter.native_invocation_dialect();
            let recipe = dialect.byte_array_string_recipe(None).unwrap();
            let subject = obj::Owned::fresh(crate::bytearray::new_byte_array(
                &[0x41, 0, 0xff, 0x5a],
                recipe,
            ));
            let replacement =
                obj::Owned::fresh(crate::bytearray::new_byte_array(&[0x51, 0xfe], recipe));
            let extra = row.shared.then(|| obj::Owned::retain(subject.as_ptr()));
            if row.resident_subject {
                interpreter
                    .native_object_string_bytes(subject.as_ptr())
                    .unwrap();
            }
            if row.resident_replacement {
                interpreter
                    .native_object_string_bytes(replacement.as_ptr())
                    .unwrap();
            }
            assert_eq!(
                unsafe { (*subject.as_ptr()).ref_count },
                if row.shared { 2 } else { 1 }
            );
            let reached = tcl_cmd_core::string::replace_c_parsed_range(
                &mut interpreter,
                &subject.as_ptr(),
                row.first,
                row.count,
                (!row.omit).then_some(&replacement.as_ptr()),
                row.in_place,
            );
            let label = (row.dialect, row.case, row.stream_sha);
            match row.output {
                Some(output) => {
                    let result = reached.unwrap();
                    assert_eq!(result == subject.as_ptr(), row.alias_subject, "{label:?}");
                    assert_eq!(
                        result == replacement.as_ptr(),
                        row.alias_replacement,
                        "{label:?}"
                    );
                    let snapshot = obj::native_object_snapshot(result).unwrap();
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
                    let result = obj::Owned::retain(result); // actual returned-result reference
                    assert_eq!(
                        interpreter
                            .native_object_string_bytes(result.as_ptr())
                            .unwrap()
                            .as_ref(),
                        unhex(output),
                        "{label:?}"
                    );
                }
                None => {
                    // The reached native branch has an unspecified initial prefix.
                    // Assert refusal of that missing producer, never its observed bytes.
                    let failure = reached.unwrap_err();
                    assert!(matches!(failure.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("uninitialised original C binary replacement constructor")))), "{label:?}");
                }
            }
            assert_eq!(
                interpreter
                    .native_object_string_bytes(subject.as_ptr())
                    .unwrap()
                    .as_ref(),
                unhex(row.subject_output),
                "{label:?}"
            );
            drop(extra);
        }
    }
}

#[cfg(all(test, have_tommath))]
mod index_key_ownership_controls {
    use super::*;
    fn actual() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.0"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    fn series() -> obj::Owned {
        let plan = tcl_cmd_core::lseq::decode(&[b"5"])
            .unwrap_or_else(|_| panic!("valid five-element original series fixture"));
        let value = tcl_cmd_core::lseq::prepare_series(&plan).unwrap().unwrap();
        obj::Owned::fresh(
            crate::native_arithseries::new_series(
                value,
                NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0),
            )
            .unwrap(),
        )
    }
    #[test]
    fn abstract_index_key_clones_keep_fresh_zero_role_without_native_references_or_element_cache() {

        // Genuine Runtime abstract Index constructor and allocation guard API.
        // Guard/refcount/cache cleanup predicates are software controls, not new
        // native callback/refcount observations from the counted command proof.
        let mut interpreter = actual();
        let original = series();
        let key =
            tcl_cmd_core::index::drill(&mut interpreter, &original.as_ptr(), &[b"2".to_vec()])
                .unwrap();
        let pointer = *tcl_cmd_core::index::key_value::<Interp>(&key);
        let lifetime = obj::NativeObjectLifetime::retain(pointer);
        assert_eq!(unsafe { (*pointer).ref_count }, 0);
        assert!(!crate::native_arithseries::has_element_cache(
            original.as_ptr()
        ));
        assert_eq!(
            crate::native_arithseries::native_header_count(original.as_ptr()),
            1
        );
        let clone = key.clone();
        assert_eq!(unsafe { (*pointer).ref_count }, 0);
        drop(key);
        assert!(obj::allocation_is_live(pointer));
        drop(clone);
        assert!(!obj::allocation_is_live(pointer));
        drop(lifetime);
    }
    #[test]
    fn nested_selected_member_has_actual_hold_before_created_parent_cleanup_and_detached_result() {

        // Shared key-result API preserves child lifetime across actual created
        // parent retirement; this is an independent software ownership control.
        let mut interpreter = actual();
        let original = series();
        let key = tcl_cmd_core::index::drill(
            &mut interpreter,
            &original.as_ptr(),
            &[b"2".to_vec(), b"0".to_vec()],
        )
        .unwrap();
        let child = *tcl_cmd_core::index::key_value::<Interp>(&key);
        let lifetime = obj::NativeObjectLifetime::retain(child);
        assert_eq!(unsafe { (*child).ref_count }, 1); // created List parent owns member
        let result = tcl_cmd_core::index::key_result(&mut interpreter, key).unwrap();
        let tcl_cmd_core::index::IndexResult::Detached(result) = result else {
            panic!("actual selected child transport")
        };
        assert_eq!(result, child);
        assert_eq!(unsafe { (*child).ref_count }, 0);
        assert!(obj::allocation_is_live(child));
        assert_eq!(obj::bytes_of(child), b"2");
        interpreter.discard_owned_index_result(result);
        assert!(!obj::allocation_is_live(child));
        drop(lifetime);
        assert!(!crate::native_arithseries::has_element_cache(
            original.as_ptr()
        ));
    }
}

#[cfg(all(test, have_tommath))]
mod indexed_binary_native_projections633 {
    use super::*;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/native_original_binary_replacement633/software_controls.rs"
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

        // Every native633 input controls supported cache/type/String geometry.
        // This direct API supplies its own explicit caller references. It does
        // not copy measured EvalObjv temporary holds, fresh aliases, reference
        // counts or subject bytes after an independently allowed in-place splice.
        // Direct helper sharing/result roles have separate634a controls.
        for row in INDEXED_BINARY_ROWS {
            let mut interpreter = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(row.dialect),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let recipe = interpreter
                .native_invocation_dialect()
                .byte_array_string_recipe(None)
                .unwrap();
            let input = row.input.map(unhex);
            let subject = obj::Owned::fresh(if row.binary_subject {
                crate::bytearray::new_byte_array(&input[0], recipe)
            } else {
                obj::new_string_bytes(&input[0])
            });
            let first = obj::Owned::fresh(obj::new_string_bytes(&input[1]));
            let last = obj::Owned::fresh(obj::new_string_bytes(&input[2]));
            let replacement = obj::Owned::fresh(if row.binary_replacement {
                crate::bytearray::new_byte_array(&input[3], recipe)
            } else {
                obj::new_string_bytes(&input[3])
            });
            let extra = row.shared.then(|| obj::Owned::retain(subject.as_ptr()));
            if row.resident_subject {
                interpreter
                    .native_object_string_bytes(subject.as_ptr())
                    .unwrap();
            }
            if row.resident_replacement {
                interpreter
                    .native_object_string_bytes(replacement.as_ptr())
                    .unwrap();
            }
            let args = [
                subject.as_ptr(),
                first.as_ptr(),
                last.as_ptr(),
                replacement.as_ptr(),
            ];
            let reached = tcl_cmd_core::string::replace(
                &mut interpreter,
                &args[..if row.omit { 3 } else { 4 }],
                tcl_syntax::number::NumberSyntax::Tcl90,
            );
            let label = (row.dialect, row.case, row.stream_sha);
            for (argument, expected) in args.iter().zip(row.argument_types) {
                let descriptor = obj::obj_type_ptr(*argument);
                let actual = if descriptor.is_null() {
                    "none"
                } else {
                    unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
                        .to_str()
                        .unwrap()
                };
                assert_eq!(actual, expected, "{label:?}");
            }
            match row.output {
                Some(output) => {
                    let result = reached.unwrap();
                    let descriptor = obj::obj_type_ptr(result);
                    let kind = if descriptor.is_null() { "none" } else { unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }.to_str().unwrap() };
                    assert_eq!(kind, row.result_type, "{label:?}");
                    assert_eq!(obj::native_object_snapshot(result).unwrap().resident.is_some(), row.resident, "{label:?}");
                    let result = obj::Owned::retain(result);
                    assert_eq!(interpreter.native_object_string_bytes(result.as_ptr()).unwrap().as_ref(), unhex(output), "{label:?}");
                }
                None => assert!(matches!(reached.unwrap_err().native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("uninitialised original C binary replacement constructor")))), "{label:?}"),
            }
            drop(extra);
        }
    }
}

#[cfg(all(test, have_tommath))]
mod parsed_empty_duplicate_control {
    use super::*;
    #[test]
    fn omitted_zero_count_without_in_place_uses_original_duplicate_constructor() {

        // The pinned helper source includes this early duplicate branch. This
        // direct software constructor control is not one of the16 native rows.
        let mut interpreter = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.0"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let subject = obj::Owned::fresh(obj::new_string_bytes(b"a\0b"));
        let duplicate = tcl_cmd_core::string::replace_c_parsed_range(
            &mut interpreter,
            &subject.as_ptr(),
            1,
            0,
            None,
            false,
        )
        .unwrap();
        assert_ne!(duplicate, subject.as_ptr());
        assert_eq!(unsafe { (*duplicate).ref_count }, 0);
        let duplicate = obj::Owned::fresh(duplicate);
        assert_eq!(obj::bytes_of(duplicate.as_ptr()), b"a\0b");
        assert_eq!(unsafe { (*subject.as_ptr()).ref_count }, 1);
    }
}

#[cfg(all(test, have_tommath))]
mod original_index_range_native_rows635 {
    use super::*;
    include!("../../tests/data/native_original_index_range635/software_controls.rs");
    fn type_name(value: *mut TclObj) -> &'static str {
        let kind = obj::obj_type_ptr(value);
        if kind.is_null() {
            "none"
        } else {
            unsafe { std::ffi::CStr::from_ptr((*kind).name) }
                .to_str()
                .unwrap()
        }
    }
    #[test]
    fn original_index_and_range_project_exact_measured_public_predicates() {

        // Direct API provides its own original owners. We compare public type,
        // residency, deterministic byte and alias predicates, not EvalObjv
        // temporary refcounts or unobserved native private Unicode caches.
        assert_eq!(ROWS.len(), 116);
        for row in ROWS {
            let mut ops = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(row.dialect),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let dialect = ops.native_invocation_dialect();
            let inputs = row.input.map(unhex);
            let subject = obj::Owned::fresh(match row.producer {
                0 => obj::new_string_bytes(&inputs[0]),
                1 => crate::bytearray::new_byte_array(
                    &inputs[0],
                    dialect.byte_array_string_recipe(None).unwrap(),
                ),
                2 => obj::new_native_unicode_obj(Rc::from([0x41, 0xd83d, 0xde00, 0x5a]), dialect)
                    .unwrap(),
                _ => unreachable!(),
            });
            let first =
                (!row.alias_first).then(|| obj::Owned::fresh(obj::new_string_bytes(&inputs[1])));
            let first = first.as_ref().unwrap_or(&subject);
            let last =
                (!row.alias_last).then(|| obj::Owned::fresh(obj::new_string_bytes(&inputs[2])));
            let last = last.as_ref().unwrap_or(first);
            let extra = row.shared.then(|| obj::Owned::retain(subject.as_ptr()));
            if row.materialise {
                ops.index_operand_bytes(&subject.as_ptr()).unwrap();
            }
            let args = [subject.as_ptr(), first.as_ptr(), last.as_ptr()];
            let reached = if row.range {
                tcl_cmd_core::string::range(
                    &mut ops,
                    &args[0],
                    &args[1],
                    &args[2],
                    tcl_syntax::number::NumberSyntax::Tcl90,
                )
            } else {
                tcl_cmd_core::string::index(
                    &mut ops,
                    &args[0],
                    &args[1],
                    tcl_syntax::number::NumberSyntax::Tcl90,
                )
            };
            let label = (row.dialect, row.case, row.stream_sha);
            for (argument, expected) in args.iter().zip(row.argument_types) {
                assert_eq!(type_name(*argument), expected, "{label:?}");
            }
            if row.code == 0 {
                let result = obj::Owned::fresh(reached.unwrap());
                assert_eq!(type_name(result.as_ptr()), row.result_type, "{label:?}");
                assert_eq!(
                    obj::has_string_rep(result.as_ptr()),
                    row.resident,
                    "{label:?}"
                );
                assert_eq!(
                    result.as_ptr() == subject.as_ptr(),
                    row.alias_subject,
                    "{label:?}"
                );
                assert_eq!(
                    ops.index_operand_bytes(&result.as_ptr()).unwrap().as_ref(),
                    unhex(row.output),
                    "{label:?}"
                );
            } else {
                let error = reached.unwrap_err();
                assert!(error.native_execution_refusal().is_none(), "{label:?}");
                assert_eq!(error.message_bytes(), unhex(row.output), "{label:?}");
            }
            assert_eq!(
                ops.index_operand_bytes(&subject.as_ptr()).unwrap().as_ref(),
                unhex(row.subject_output),
                "{label:?}"
            );
            drop(extra);
        }
    }
}

#[cfg(test)]
mod original_c_string_stage_controls637 {
    use super::*;
    use std::cell::Cell;
    thread_local! {
        static TARGET: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
        static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
        static INDEX_CALLS: Cell<usize> = const { Cell::new(0) };
    }
    struct Current;
    impl Drop for Current {
        fn drop(&mut self) {
            crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        }
    }
    static REPLACED: TclObjType = TclObjType {
        name: c"originalStringSelectedBackingControl".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    extern "C" fn index_update(original: *mut TclObj) {
        INDEX_CALLS.with(|calls| calls.set(calls.get() + 1));
        let subject = TARGET.with(Cell::get);
        if !subject.is_null() {
            REFS.with(|refs| refs.set(unsafe { (*subject).ref_count }));
            obj::change_type(subject, &REPLACED, 0);
            unsafe { obj::set_native_updater_string_rep(subject, b"XY", false) };
        }
        unsafe { obj::set_native_updater_string_rep(original, b"0", false) };
    }
    static INDEX: TclObjType = TclObjType {
        name: c"originalStringIndexStageControl".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(index_update),
        set_from_any_proc: None,
    };
    extern "C" fn subject_update(original: *mut TclObj) {
        unsafe { obj::set_native_updater_string_rep(original, b"A\xc3\xa9Z", false) };
    }
    extern "C" fn subject_free(original: *mut TclObj) {
        REFS.with(|refs| refs.set(unsafe { (*original).ref_count }));
        let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        actual.refuse_host_command("original String primary retirement");
    }
    static SUBJECT: TclObjType = TclObjType {
        name: c"originalStringSubjectStageControl".as_ptr(),
        free_int_rep_proc: Some(subject_free),
        dup_int_rep_proc: None,
        update_string_proc: Some(subject_update),
        set_from_any_proc: None,
    };
    fn actual(profile: &str) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(profile),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    #[test]
    fn original_c_binary_pointer_and_reread_stages_use_actual_header_and_no_protective_reference() {


        // Real software extension updater mutates the selected subject during
        // its original index getter. Native635 public rows do not observe this
        // callback; pinned command branches bound the captured/reread schedule.
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut ops = actual(profile);
            let subject = obj::Owned::fresh(crate::bytearray::new_byte_array(
                b"ABC",
                ops.native_invocation_dialect()
                    .byte_array_string_recipe(None)
                    .unwrap(),
            ));
            let operand = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
            TARGET.with(|target| target.set(subject.as_ptr()));
            INDEX_CALLS.with(|calls| calls.set(0));
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut ops);
            let _current = Current;
            let reached = tcl_cmd_core::string::index(
                &mut ops,
                &subject.as_ptr(),
                &operand.as_ptr(),
                tcl_syntax::number::NumberSyntax::Tcl90,
            );
            assert_eq!(INDEX_CALLS.with(Cell::get), 1);
            assert_eq!(REFS.with(Cell::get), 1);
            if profile == "tcl8.4" {
                let error = reached.unwrap_err();
                assert!(matches!(error.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("retired original C binary storage")))));
                assert_eq!(
                    error.native_execution_refusal(),
                    ops.native_execution_refusal().as_ref()
                );
                assert!(core::ptr::eq(
                    obj::obj_type_ptr(subject.as_ptr()),
                    &REPLACED
                ));
            } else {
                let result = obj::Owned::fresh(reached.unwrap());
                assert_eq!(
                    ops.index_operand_bytes(&result.as_ptr()).unwrap().as_ref(),
                    b"X"
                );
                if profile == "tcl8.5" {
                    assert!(crate::bytearray::native_cache_snapshot(subject.as_ptr()).is_some());
                } else {
                    assert!(matches!(
                        obj::native_object_snapshot(subject.as_ptr()).unwrap().cache,
                        tcl_syntax::native_object::NativeObjectCacheSnapshot::String {
                            num_chars: Some(2),
                            ..
                        }
                    ));
                }
            }
            assert_eq!(unsafe { (*subject.as_ptr()).ref_count }, 1);
            assert_eq!(unsafe { (*operand.as_ptr()).ref_count }, 1);
            TARGET.with(|target| target.set(core::ptr::null_mut()));
        }
    }
    #[test]
    fn original_count_free_callback_preserves_first_cause_before_any_index_getter_or_string_cache_adoption(
    ) {

        // Actual original extension free callback, not native adversarial evidence.
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0"] {
            let mut ops = actual(profile);
            let subject = obj::Owned::fresh(obj::alloc_typed(&SUBJECT, 0));
            let operand = obj::Owned::fresh(obj::alloc_typed(&INDEX, 0));
            INDEX_CALLS.with(|calls| calls.set(0));
            TARGET.with(|target| target.set(core::ptr::null_mut()));
            ops.set_result_bytes(b"PRIOR");
            let prior = ops.get_obj_result();
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut ops);
            let _current = Current;
            let error = tcl_cmd_core::string::index(
                &mut ops,
                &subject.as_ptr(),
                &operand.as_ptr(),
                tcl_syntax::number::NumberSyntax::Tcl90,
            )
            .unwrap_err();
            assert_eq!(
                error.native_execution_refusal(),
                ops.native_execution_refusal().as_ref()
            );
            assert_eq!(REFS.with(Cell::get), 1);
            assert_eq!(INDEX_CALLS.with(Cell::get), 0);
            assert_eq!(ops.get_obj_result(), prior);
            assert!(!matches!(
                obj::native_object_snapshot(subject.as_ptr()).unwrap().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
            ));
        }
    }
    #[test]
    fn original_primary_receipt_refuses_scalar_change_and_restore() {

        // Actual software epoch control; no native private-epoch observation.
        let value = obj::Owned::fresh(obj::alloc_typed(&REPLACED, 7));
        let receipt = obj::NativePrimaryReceipt::capture(value.as_ptr()).unwrap();
        obj::change_type(value.as_ptr(), &REPLACED, 8);
        obj::change_type(value.as_ptr(), &REPLACED, 7);
        assert!(receipt.check(value.as_ptr()).is_err());
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    }
}

#[cfg(test)]
#[path = "native_jim_index/c84_integer_result_tests.rs"]
mod c84_integer_result_tests;
