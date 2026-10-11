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
    CmdError,
    native_list_index::{self, NativeListIndexOps},
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
    pub(crate) fn as_ptr(&self) -> *mut TclObj {
        self.pointer
    }
    pub(super) fn into_native_unowned(self) -> *mut TclObj {
        self._owner.into_native_unowned()
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
    fn index_ordinary_members(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<Vec<*mut TclObj>, ValueError> {
        list::ordinary_list_elements_native_checked(
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
    fn index_members_checked(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<Vec<*mut TclObj>, CmdError> {
        let arguments =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[*original])?;
        let protocol = tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?);
        list::list_elements_native_using(
            *original,
            protocol,
            &|| arguments.check_original_for(self, 0, *original),
            &|error| arguments.retain_refusal(error.into()),
        )
    }
    fn index_ordinary_members_checked(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<Vec<*mut TclObj>, CmdError> {
        let arguments =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[*original])?;
        let protocol = tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?);
        list::ordinary_list_elements_native_using(
            *original,
            protocol,
            &|| arguments.check_original_for(self, 0, *original),
            &|error| arguments.retain_refusal(error.into()),
        )
    }
    fn index_list_copy_checked(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<RuntimeIndexOwner, CmdError> {
        let arguments =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[*original])?;
        let protocol = tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?);
        list::native_list_copy_using(
            *original,
            protocol,
            &|| arguments.check_original_for(self, 0, *original),
            &|error| arguments.retain_refusal(error.into()),
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
    fn index_number_checked(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<Option<Number>, tcl_cmd_core::CmdError> {
        let arguments =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[*original])?;
        let dialect = self.native_invocation_dialect();
        let reached = if self.index_version()? >= TclVersion::V9_0 {
            crate::typed_value::native_number_probe_with_currency(
                *original,
                dialect,
                NativeNumberGetterKind::Number,
                Some(arguments.currency()),
            )
            .map(Result::ok)
        } else {
            crate::typed_value::native_scalar_probe_with_environment_and_currency(
                *original,
                dialect,
                NativeScalarGetterKind::Int,
                self.host().numeric_environment(),
                Some(arguments.currency()),
            )
            .map(|outcome| match outcome {
                Ok(NativeScalarGetterValue::Wide(integer)) => Some(Number::Int(integer)),
                _ => None,
            })
        };
        arguments.check_original_for(self, 0, *original)?;
        reached.map_err(Into::into)
    }
    fn index_clear_arithmetic_primary_checked(
        &mut self,
        original: &*mut TclObj,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let arguments =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[*original])?;
        obj::change_type_with_stage_check(*original, core::ptr::null(), 0, &mut || {
            arguments.check_original_for(self, 0, *original)
        })?;
        arguments.check_original_for(self, 0, *original)
    }
    fn index_install_offset_checked(
        &mut self,
        original: &*mut TclObj,
        offset: NativeEndOffset,
    ) -> Result<(), tcl_cmd_core::CmdError> {
        let arguments =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[*original])?;
        obj::native_end_offset::install_checked(*original, offset, &mut || {
            arguments.check_original_for(self, 0, *original)
        })
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
impl tcl_cmd_core::native_list_index::NativeListRangeOps for Interp {
    fn range_original_checked(
        &mut self,
        original: &*mut TclObj,
        first: i64,
        last: i64,
    ) -> Result<RuntimeIndexOwner, CmdError> {
        let arguments =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[*original])?;
        let protocol = tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?);
        list::native_list_command_range_using(
            *original,
            first,
            last,
            protocol,
            &|| arguments.check_original_for(self, 0, *original),
            &|error| arguments.retain_refusal(error.into()),
        )
        .map(RuntimeIndexOwner::from_owned)
    }
    fn range_original(
        &mut self,
        original: &*mut TclObj,
        first: i64,
        last: i64,
    ) -> Result<RuntimeIndexOwner, ValueError> {
        list::native_list_command_range(
            *original,
            first,
            last,
            tcl_syntax::native_string::NativeStringProtocol::C(self.index_version()?),
        )
        .map(RuntimeIndexOwner::from_owned)
    }
}
impl Interp {
    pub(crate) fn original_list_range(
        &mut self,
        list: *mut TclObj,
        first: *mut TclObj,
        last: *mut TclObj,
    ) -> Result<RuntimeIndexOwner, CmdError> {
        if self
            .native_invocation_dialect()
            .native_string_protocol()
            .is_some_and(|protocol| protocol.tcl_version().is_some())
        {
            return native_list_index::command_range(self, &list, &first, &last);
        }
        let value = tcl_cmd_core::list::lrange(self, &list, &first, &last)?;
        Ok(RuntimeIndexOwner::from_owned(obj::Owned::retain(value)))
    }
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

#[cfg(all(test, have_tommath))]
mod returned_owner_controls {
    use super::*;
    use tcl_cmd_core::index::NativeIndexOps;
    fn actual() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl9.0"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }
    #[test]
    fn c_returned_member_transport_preserves_original_member_reference_and_actual_result_hold() {
        // naming.list.original-range-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-range-objects-and-instructions.md
        // Existing native owner API's software reference control. No measured
        // C process reference or callback schedule is inferred from this test.
        let mut interpreter = actual();
        let original = obj::Owned::fresh(list::new_list_obj_native(
            &[obj::new_string_bytes(b"a")],
            tcl_syntax::native_string::NativeStringProtocol::C(TclVersion::V9_0),
        ));
        let index = obj::Owned::fresh(obj::new_string_bytes(b"0"));
        let member = list::list_elements_native_checked(
            original.as_ptr(),
            tcl_syntax::native_string::NativeStringProtocol::C(TclVersion::V9_0),
        )
        .unwrap()[0];
        assert_eq!(unsafe { (*member).ref_count }, 1);
        let result = interpreter
            .original_c_list_index(&original.as_ptr(), &[index.as_ptr()], false)
            .unwrap()
            .unwrap();
        let tcl_cmd_core::index::IndexResult::ReturnedOwnedTransport(result) = result else {
            panic!("genuine C returned owner")
        };
        assert_eq!(result, member);
        assert_eq!(unsafe { (*member).ref_count }, 1);
        interpreter.discard_owned_index_result(result);
        assert_eq!(unsafe { (*member).ref_count }, 1);
        let result = obj::Owned::retain(member); // genuine result publication
        assert_eq!(unsafe { (*member).ref_count }, 2);
        drop(original);
        assert_eq!(unsafe { (*member).ref_count }, 1);
        assert_eq!(obj::bytes_of(result.as_ptr()), b"a");
    }
    #[test]
    fn c_fresh_empty_returned_transport_has_zero_count_and_exact_fresh_cleanup() {
        // naming.list.original-range-objects-and-instructions
        // docs/design/analysis/name-resolution-proofs/list.original-range-objects-and-instructions.md
        // Genuine software constructor/owner API control, not native evidence.
        let mut interpreter = actual();
        let original = obj::Owned::fresh(obj::new_string_bytes(b"a"));
        let index = obj::Owned::fresh(obj::new_string_bytes(b"99"));
        let result = interpreter
            .original_c_list_index(&original.as_ptr(), &[index.as_ptr()], false)
            .unwrap()
            .unwrap();
        let tcl_cmd_core::index::IndexResult::ReturnedOwnedTransport(result) = result else {
            panic!("genuine C returned owner")
        };
        let lease = obj::NativeObjectLifetime::retain(result);
        assert_eq!(unsafe { (*result).ref_count }, 0);
        assert!(obj::allocation_is_live(result));
        interpreter.discard_owned_index_result(result);
        assert!(!obj::allocation_is_live(result));
        drop(lease);
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
    }
}
