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

//! Original `ListIndex` getter and owning return boundary on the VM's headers.
use crate::{NativeListItems, Value, interp::Vm};
use tcl_cmd_core::{
    CmdError,
    native_list_index::{self, NativeIndexMembers, NativeListIndexOps},
};
use tcl_dialect::TclVersion;
use tcl_syntax::{
    native_end_offset::NativeEndOffset,
    number::Number,
    scalar_getter::{NativeNumberGetterKind, NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

impl NativeIndexMembers<Value> for NativeListItems {
    fn index_elements(&self) -> Result<&[Value], ValueError> {
        self.elements()
    }
}
/// Borrow the actual selected option header and its original native List array.
/// Neither the header lease nor its member snapshot supplies a native +1.
pub struct VmOptionMembers {
    original: Value,
    members: NativeListItems,
}
impl NativeIndexMembers<Value> for VmOptionMembers {
    fn index_elements(&self) -> Result<&[Value], ValueError> {
        self.original.check_native_header()?;
        let (current, _) = self.original.cached_list_representation().ok_or(
            ValueError::CommandProtocolUnavailable("withdrawn original option List backing"),
        )?;
        if !self.members.same_backing(&current) {
            return Err(ValueError::CommandProtocolUnavailable(
                "changed original option List backing",
            ));
        }
        self.members.elements()
    }
}
impl NativeListIndexOps for Vm {
    type Owner = Value;
    type Members = NativeListItems;
    type OptionMembers = VmOptionMembers;
    fn index_version(&self) -> Result<TclVersion, ValueError> {
        self.actual_native_invocation_dialect()
            .native_string_protocol()
            .and_then(tcl_syntax::native_string::NativeStringProtocol::tcl_version)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C ListIndex issuer",
            ))
    }
    fn index_option_layout(
        &self,
    ) -> Result<tcl_cmd_core::native_option_index::NativeOptionIndexLayout, CmdError> {
        self.index_version()?;
        tcl_cmd_core::native_option_index::NativeOptionIndexLayout::local().map_err(Into::into)
    }
    fn index_original(owner: &Value) -> &Value {
        owner
    }
    fn index_retain(original: &Value) -> Value {
        original.clone().into_native_reference()
    }
    fn index_empty(&mut self) -> Value {
        Value::empty()
    }
    fn index_int_c_string(&mut self, source: &[u8]) -> Result<Option<i32>, CmdError> {
        use crate::interp::{
            native_numeric_ops::OriginalNumericOps, native_operation_currency::NativeOperationScope,
        };
        let operation = NativeOperationScope::enter(self)?;
        if self.index_version()? != TclVersion::V8_6 {
            return Err(
                ValueError::CommandProtocolUnavailable("C8.6 suffix CString Int issuer").into(),
            );
        }
        operation.check(self)?;
        let temporary = Value::new_native_string_bytes(tcl_core_types::c_string_extent(source))
            .into_native_reference();
        let numeric = OriginalNumericOps::in_original_scope(self, &operation)?;
        let reached = numeric
            .probe(&temporary, NativeScalarGetterKind::Int)
            .map(|outcome| match outcome {
                Ok(NativeScalarGetterValue::Wide(integer)) => {
                    Some(tcl_syntax::number::native_int32_low_bits(integer))
                }
                _ => None,
            });
        if let Err(error) = &reached {
            numeric.retain_failure(error);
        }
        drop(temporary);
        numeric.current()?;
        reached
    }
    fn index_option_members_checked(
        &mut self,
        original: &Value,
    ) -> Result<VmOptionMembers, CmdError> {
        let members = self.index_members_checked(original)?;
        let view = VmOptionMembers {
            original: original.native_lifetime_lease().into_value(),
            members,
        };
        view.index_elements()?;
        Ok(view)
    }
    fn index_is_list(original: &Value) -> bool {
        original.cached_list_representation().is_some()
    }
    fn index_members(&mut self, original: &Value) -> Result<NativeListItems, ValueError> {
        let protocol = tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?);
        self.native_object_list_elements_in(original, protocol)
    }
    fn index_ordinary_members(&mut self, original: &Value) -> Result<NativeListItems, ValueError> {
        // Value's current native list conversion installs an ordinary primary.
        // An abstract Value adapter must implement this distinct purpose too.
        let protocol = tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?);
        self.native_object_list_elements_in(original, protocol)
    }
    fn index_list_copy(&mut self, original: &Value) -> Result<Value, ValueError> {
        original.native_list_copy(tcl_syntax::native_string::NativeStringProtocol::C(
            self.index_version()?,
        ))
    }
    fn index_members_checked(&mut self, original: &Value) -> Result<NativeListItems, CmdError> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let numeric =
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(self, &scope)?;
        let protocol = numeric
            .dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C List member issuer",
            ))?;
        original.native_object_list_elements_with_strings(
            protocol,
            &mut || numeric.current_for_value(original),
            &mut |value| numeric.string(value, protocol),
        )
    }
    fn index_ordinary_members_checked(
        &mut self,
        original: &Value,
    ) -> Result<NativeListItems, CmdError> {
        // VM's selected object getter installs ordinary storage; it does not
        // donate a Runtime arithmetic-series Index/GetElements producer.
        self.index_members_checked(original)
    }
    fn index_list_copy_checked(&mut self, original: &Value) -> Result<Value, CmdError> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let numeric =
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(self, &scope)?;
        let protocol = numeric
            .dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C List copy issuer",
            ))?;
        original.native_list_copy_checked(
            protocol,
            &mut || numeric.current_for_value(original),
            &mut |value| numeric.string(value, protocol),
        )
    }
    fn index_number(&mut self, original: &Value) -> Result<Option<Number>, ValueError> {
        let version = self.index_version()?;
        let ops = crate::interp::native_numeric_ops::OriginalNumericOps::entered_value(self)?;
        if version >= TclVersion::V9_0 {
            return ops
                .number_value(original, NativeNumberGetterKind::Number)
                .map(Result::ok);
        }
        Ok(
            match ops.probe_value(original, NativeScalarGetterKind::Int)? {
                Ok(NativeScalarGetterValue::Wide(integer)) => Some(Number::Int(integer)),
                _ => None,
            },
        )
    }
    fn index_number_checked(
        &mut self,
        original: &Value,
    ) -> Result<Option<Number>, tcl_cmd_core::CmdError> {
        let version = self.index_version()?;
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let ops =
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(self, &scope)?;
        if version >= TclVersion::V9_0 {
            return ops
                .number(original, NativeNumberGetterKind::Number)
                .map(Result::ok);
        }
        ops.probe(original, NativeScalarGetterKind::Int)
            .map(|outcome| match outcome {
                Ok(NativeScalarGetterValue::Wide(integer)) => Some(Number::Int(integer)),
                _ => None,
            })
    }
    fn index_clear_arithmetic_primary_checked(
        &mut self,
        original: &Value,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let ops =
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(self, &scope)?;
        original
            .clear_native_index_arithmetic_primary_checked(&mut || ops.current_for_value(original))
    }
    fn index_install_offset_checked(
        &mut self,
        original: &Value,
        offset: NativeEndOffset,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let ops =
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(self, &scope)?;
        original.install_native_end_offset_checked(offset, &mut || ops.current_for_value(original))
    }
    fn index_clear_arithmetic_primary(original: &Value) {
        original.clear_native_index_arithmetic_primary();
    }
    fn index_offset(original: &Value) -> Option<NativeEndOffset> {
        original.native_end_offset()
    }
    fn index_install_offset(original: &Value, offset: NativeEndOffset) -> Result<(), ValueError> {
        original.install_native_end_offset(offset)
    }
}
impl tcl_cmd_core::native_list_index::NativeListRangeOps for Vm {
    fn range_original_checked(
        &mut self,
        original: &Value,
        first: i64,
        last: i64,
    ) -> Result<Value, CmdError> {
        let scope = crate::interp::native_operation_currency::NativeOperationScope::enter(self)?;
        let numeric =
            crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(self, &scope)?;
        let protocol = numeric
            .dialect()
            .native_string_protocol()
            .filter(|protocol| protocol.tcl_version().is_some())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C List range issuer",
            ))?;
        original.native_list_command_range_checked(
            first,
            last,
            protocol,
            &mut || numeric.current_for_value(original),
            &mut |value| numeric.string(value, protocol),
        )
    }
    fn range_original(
        &mut self,
        original: &Value,
        first: i64,
        last: i64,
    ) -> Result<Value, ValueError> {
        original.native_list_command_range(
            first,
            last,
            tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?),
        )
    }
}
impl Vm {
    pub(crate) fn original_list_range(
        &mut self,
        list: &Value,
        first: &Value,
        last: &Value,
    ) -> Result<Value, CmdError> {
        if self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .is_some_and(|protocol| protocol.tcl_version().is_some())
        {
            return native_list_index::command_range(self, list, first, last);
        }
        tcl_cmd_core::list::lrange(self, list, first, last)
    }
    pub(crate) fn original_list_index(
        &mut self,
        list: &Value,
        indices: &[Value],
        path: bool,
    ) -> Result<Value, CmdError> {
        if self
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .is_some_and(|protocol| protocol.tcl_version().is_some())
        {
            return if path && indices.len() == 1 {
                native_list_index::single(self, list, &indices[0])
            } else {
                native_list_index::flat(self, list, indices)
            };
        }
        if path {
            tcl_cmd_core::list::lindex(self, list, indices)
        } else {
            tcl_cmd_core::list::lindex_flat(self, list, indices)
        }
    }
}
