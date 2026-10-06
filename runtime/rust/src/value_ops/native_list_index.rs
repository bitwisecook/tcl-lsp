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

//! Original ListIndex getter and real owning return boundary on Runtime headers.
use crate::{
    interp::Interp,
    list,
    obj::{self, TclObj},
};
use tcl_cmd_core::{
    native_list_index::{self, NativeListIndexOps},
    CmdError,
};
use tcl_dialect::TclVersion;
use tcl_syntax::{
    native_end_offset::NativeEndOffset,
    number::Number,
    scalar_getter::{NativeNumberGetterKind, NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

/// Owns the exact original header returned by a reached native ListIndex getter.
///
/// The private owner keeps that header live through result publication; this
/// adapter exposes no constructor or pointer authority to external callers.
pub struct RuntimeIndexOwner {
    pointer: *mut TclObj,
    _owner: obj::Owned,
}
impl RuntimeIndexOwner {
    fn from_owned(owner: obj::Owned) -> Self {
        Self {
            pointer: owner.as_ptr(),
            _owner: owner,
        }
    }
    pub(crate) fn retain(pointer: *mut TclObj) -> Self {
        Self::from_owned(obj::Owned::retain(pointer))
    }
    pub(crate) fn as_ptr(&self) -> *mut TclObj {
        self.pointer
    }
    pub(crate) fn owned(&self) -> &obj::Owned {
        &self._owner
    }
}
impl NativeListIndexOps for Interp {
    type Owner = RuntimeIndexOwner;
    type Members = Vec<*mut TclObj>;
    fn index_version(&self) -> Result<TclVersion, ValueError> {
        self.native_invocation_dialect()
            .native_string_protocol()
            .and_then(|protocol| protocol.tcl_version())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original C ListIndex issuer",
            ))
    }
    fn index_original(owner: &RuntimeIndexOwner) -> &*mut TclObj {
        &owner.pointer
    }
    fn index_retain(original: &*mut TclObj) -> RuntimeIndexOwner {
        RuntimeIndexOwner::from_owned(obj::Owned::retain(*original))
    }
    fn index_empty(&mut self) -> RuntimeIndexOwner {
        RuntimeIndexOwner::from_owned(obj::Owned::fresh(obj::new_string_bytes(b"")))
    }
    fn index_is_list(original: &*mut TclObj) -> bool {
        core::ptr::eq(obj::obj_type_ptr(*original), &list::TCL_LIST_TYPE)
    }
    fn index_members(&mut self, original: &*mut TclObj) -> Result<Vec<*mut TclObj>, ValueError> {
        list::list_elements_native_checked(
            *original,
            tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?),
        )
    }
    fn index_list_copy(&mut self, original: &*mut TclObj) -> Result<RuntimeIndexOwner, ValueError> {
        list::native_list_copy(
            *original,
            tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?),
        )
        .map(RuntimeIndexOwner::from_owned)
    }
    fn index_number(&mut self, original: &*mut TclObj) -> Result<Option<Number>, ValueError> {
        let dialect = self.native_invocation_dialect();
        if self.index_version()? >= TclVersion::V9_0 {
            return crate::typed_value::native_number_probe(
                *original,
                dialect,
                NativeNumberGetterKind::Number,
            )
            .map(Result::ok);
        }
        Ok(
            match crate::typed_value::native_scalar_probe_with_environment(
                *original,
                dialect,
                NativeScalarGetterKind::Int,
                self.host().numeric_environment(),
            )? {
                Ok(NativeScalarGetterValue::Wide(integer)) => Some(Number::Int(integer)),
                _ => None,
            },
        )
    }
    fn index_clear_arithmetic_primary(original: &*mut TclObj) {
        obj::change_type(*original, core::ptr::null(), 0);
    }
    fn index_offset(original: &*mut TclObj) -> Option<NativeEndOffset> {
        obj::native_end_offset::cache(*original)
    }
    fn index_install_offset(
        original: &*mut TclObj,
        offset: NativeEndOffset,
    ) -> Result<(), ValueError> {
        obj::native_end_offset::install(*original, offset)
    }
    fn index_is_abstract(original: &*mut TclObj) -> bool {
        crate::native_arithseries::is_series(*original)
    }
}
impl Interp {
    pub(crate) fn original_list_index(
        &mut self,
        list: *mut TclObj,
        indices: &[*mut TclObj],
        path: bool,
    ) -> Result<RuntimeIndexOwner, CmdError> {
        if self
            .native_invocation_dialect()
            .native_string_protocol()
            .is_some_and(|protocol| protocol.tcl_version().is_some())
        {
            return if path && indices.len() == 1 {
                native_list_index::single(self, &list, &indices[0])
            } else {
                native_list_index::flat(self, &list, indices)
            };
        }
        let value = if path {
            tcl_cmd_core::list::lindex(self, &list, indices)
        } else {
            tcl_cmd_core::list::lindex_flat(self, &list, indices)
        }?;
        Ok(RuntimeIndexOwner::from_owned(obj::Owned::retain(value)))
    }
}
