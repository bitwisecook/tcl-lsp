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

//! Actual original end-offset primary and release-selected native string updater.
use super::{TclObj, TclObjType};
use tcl_syntax::{native_end_offset::NativeEndOffset, value::ValueError};
extern "C" fn free(object: *mut TclObj) {
    // SAFETY: the exact descriptor owns this boxed conversion record.
    unsafe {
        drop(Box::from_raw(
            super::internal_rep(object) as usize as *mut NativeEndOffset
        ));
    }
}
extern "C" fn duplicate(source: *mut TclObj, target: *mut TclObj) {
    install_unchecked(
        target,
        cache(source).expect("original end-offset descriptor"),
    );
}
extern "C" fn update(object: *mut TclObj) {
    let bytes = cache(object)
        .expect("original end-offset descriptor")
        .string_update()
        .expect("legacy end-offset updater");
    // SAFETY: the legacy descriptor owns this updater and its live header.
    unsafe {
        super::set_native_updater_string_rep(object, &bytes, false);
    }
}
static LEGACY_TYPE: TclObjType = TclObjType {
    name: c"end-offset".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
static MODERN_TYPE: TclObjType = TclObjType {
    name: c"end-offset".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
fn install_unchecked(object: *mut TclObj, offset: NativeEndOffset) {
    let descriptor = if offset.version() < tcl_dialect::TclVersion::V9_0 {
        &LEGACY_TYPE
    } else {
        &MODERN_TYPE
    };
    super::change_type(
        object,
        descriptor,
        Box::into_raw(Box::new(offset)) as usize as u64,
    );
}
pub(crate) fn cache(object: *mut TclObj) -> Option<NativeEndOffset> {
    let descriptor = super::obj_type_ptr(object);
    if !core::ptr::eq(descriptor, &LEGACY_TYPE) && !core::ptr::eq(descriptor, &MODERN_TYPE) {
        return None;
    }
    // SAFETY: exact descriptor identity authenticates the boxed representation.
    Some(unsafe { *(super::internal_rep(object) as usize as *const NativeEndOffset) })
}
pub(crate) fn install(object: *mut TclObj, offset: NativeEndOffset) -> Result<(), ValueError> {
    super::check_native_liveness(object)?;
    if !super::has_string_rep(object) {
        return Err(ValueError::CommandProtocolUnavailable(
            "original end-offset resident string",
        ));
    }
    install_unchecked(object, offset);
    Ok(())
}
