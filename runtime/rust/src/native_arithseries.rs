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

//! Original C9 arithmetic-series headers and purpose-specific abstract-list
//! providers. The exported TclObjType ABI is unchanged: Length and Index are
//! private checked dispatch, admitted only by this exact retained descriptor.
use crate::obj::{self, TclObj, TclObjType};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use tcl_cmd_core::lseq::{self, Series};
use tcl_syntax::{
    native_string::NativeStringProtocol, number::Number,
    raw_string::NativeMaterializationLimitError, value::ValueError,
};

struct Backing {
    series: Series,
    headers: Cell<usize>,
    elements: RefCell<Option<Vec<*mut TclObj>>>,
}
impl Drop for Backing {
    fn drop(&mut self) {
        if let Some(elements) = self.elements.get_mut().take() {
            for element in elements {
                // SAFETY: the actual element cache owns exactly one reference.
                unsafe { obj::decr_ref_count(element) };
            }
        }
    }
}
struct Header {
    backing: Rc<Backing>,
    protocol: NativeStringProtocol,
}
impl Drop for Header {
    fn drop(&mut self) {
        self.backing.headers.set(self.backing.headers.get() - 1);
    }
}
static TYPE: TclObjType = TclObjType {
    name: c"arithseries".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: Some(update_string),
    set_from_any_proc: None,
};
extern "C" fn free(value: *mut TclObj) {
    // SAFETY: the exact descriptor owns the boxed header.
    unsafe {
        drop(Box::from_raw(
            obj::internal_rep(value) as usize as *mut Header
        ));
    }
}
extern "C" fn duplicate(source: *mut TclObj, target: *mut TclObj) {
    let source = header(source);
    source.backing.headers.set(source.backing.headers.get() + 1);
    let copy = Box::new(Header {
        backing: Rc::clone(&source.backing),
        protocol: source.protocol,
    });
    obj::change_type(target, &TYPE, Box::into_raw(copy) as usize as u64);
}
extern "C" fn update_string(value: *mut TclObj) {
    // Failure leaves bytes absent. Checked access transports the same refusal;
    // this no-interpreter ABI slot cannot publish a guest completion.
    let _ = materialize_string(value, header(value).protocol);
}
fn header(value: *mut TclObj) -> &'static Header {
    assert!(is_series(value), "arithmetic-series descriptor");
    // SAFETY: exact descriptor owns the live Header until its current caller's
    // original header owner releases or changes its primary representation.
    unsafe { &*(obj::internal_rep(value) as usize as *const Header) }
}
pub(crate) fn is_series(value: *mut TclObj) -> bool {
    core::ptr::eq(obj::obj_type_ptr(value), &TYPE)
}
fn validate(value: *mut TclObj, protocol: NativeStringProtocol) -> Result<(), ValueError> {
    if !obj::allocation_is_live(value) {
        return Err(ValueError::CommandProtocolUnavailable(
            "retired arithmetic-series header",
        ));
    }
    if header(value).protocol != protocol {
        return Err(ValueError::CommandProtocolUnavailable(
            "arithmetic-series issuer",
        ));
    }
    Ok(())
}
pub(crate) fn new_series(
    series: Series,
    protocol: NativeStringProtocol,
) -> Result<*mut TclObj, ValueError> {
    if !protocol
        .tcl_version()
        .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
    {
        return Err(ValueError::CommandProtocolUnavailable(
            "native arithmetic-series constructor",
        ));
    }
    let backing = Rc::new(Backing {
        series,
        headers: Cell::new(1),
        elements: RefCell::new(None),
    });
    Ok(obj::alloc_typed(
        &TYPE,
        Box::into_raw(Box::new(Header { backing, protocol })) as usize as u64,
    ))
}
/// Observe the actual original cache before any Index/GetElements call.
#[cfg(test)]
pub(crate) fn has_element_cache(value: *mut TclObj) -> bool {
    assert!(is_series(value), "actual arithmetic-series observer");
    header(value).backing.elements.borrow().is_some()
}
/// Observe genuine original abstract backing header roles.
#[cfg(test)]
pub(crate) fn native_header_count(value: *mut TclObj) -> usize {
    assert!(is_series(value), "actual arithmetic-series observer");
    header(value).backing.headers.get()
}
pub(crate) fn length(value: *mut TclObj) -> Option<usize> {
    is_series(value).then(|| header(value).backing.series.length())
}
pub(crate) fn length_in(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<Option<usize>, ValueError> {
    if !is_series(value) {
        return Ok(None);
    }
    validate(value, protocol)?;
    Ok(length(value))
}
fn fresh_number(number: Number) -> *mut TclObj {
    match number {
        Number::Int(value) => obj::new_wide_int_obj(value),
        Number::Double(value) => obj::new_double_obj(value),
        _ => unreachable!("validated arithmetic series emits wide integers or doubles"),
    }
}
pub(crate) fn index(value: *mut TclObj, index: usize) -> Option<*mut TclObj> {
    header(value)
        .backing
        .series
        .number_at(index)
        .map(fresh_number)
}
pub(crate) fn index_in(
    value: *mut TclObj,
    index: usize,
    protocol: NativeStringProtocol,
) -> Result<Option<*mut TclObj>, ValueError> {
    validate(value, protocol)?;
    Ok(self::index(value, index))
}
fn materialization_length(length: usize) -> Result<(), ValueError> {
    if length as u128 > lseq::MAX_MATERIALIZE as u128 {
        return Err(ValueError::NativeMaterialization(
            NativeMaterializationLimitError::new(length as u128, lseq::MAX_MATERIALIZE as u64),
        ));
    }
    Ok(())
}
/// Fresh indexed members for ordinary List conversion, without populating the
/// abstract GetElements cache. The returned owners survive retirement of the
/// original abstract primary while a List takes its member references.
pub(crate) fn ordinary_members(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<Vec<obj::Owned>, ValueError> {
    validate(value, protocol)?;
    let length = header(value).backing.series.length();
    materialization_length(length)?;
    let mut members = Vec::new();
    members.try_reserve_exact(length).map_err(|_| {
        ValueError::NativeMaterialization(NativeMaterializationLimitError::new(
            length as u128,
            lseq::MAX_MATERIALIZE as u64,
        ))
    })?;
    for coordinate in 0..length {
        let member = index(value, coordinate).expect("validated abstract List coordinate");
        members.push(obj::Owned::fresh(member));
    }
    Ok(members)
}
pub(crate) fn elements_retained(value: *mut TclObj) -> Result<Vec<*mut TclObj>, ValueError> {
    elements(value, header(value).protocol)
}
pub(crate) fn elements(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<Vec<*mut TclObj>, ValueError> {
    validate(value, protocol)?;
    let backing = &header(value).backing;
    let length = backing.series.length();
    materialization_length(length)?;
    if length == 0 {
        return Ok(Vec::new());
    }
    if backing.elements.borrow().is_none() {
        let mut elements = Vec::new();
        elements.try_reserve_exact(length).map_err(|_| {
            ValueError::NativeMaterialization(NativeMaterializationLimitError::new(
                length as u128,
                lseq::MAX_MATERIALIZE as u64,
            ))
        })?;
        for index in 0..length {
            let element = fresh_number(backing.series.number_at(index).expect("captured Length"));
            // SAFETY: native GetElements creates exactly one owning cache ref.
            unsafe { obj::incr_ref_count(element) };
            elements.push(element);
        }
        *backing.elements.borrow_mut() = Some(elements);
    }
    Ok(backing
        .elements
        .borrow()
        .as_ref()
        .expect("actual element cache")
        .clone())
}
pub(crate) fn materialize_string(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<Vec<u8>, ValueError> {
    validate(value, protocol)?;
    if obj::has_string_rep(value) {
        return Ok(obj::bytes_of(value));
    }
    let series = header(value).backing.series;
    materialization_length(series.length())?;
    let mut bytes = Vec::new();
    for index in 0..series.length() {
        let number = series.number_at(index).expect("captured Length");
        let spelling = match number {
            Number::Int(value) => value.to_string(),
            Number::Double(value) => {
                let policy = tcl_dialect::DoubleStringPolicy::for_tcl_version(
                    protocol.tcl_version().expect("actual C issuer"),
                );
                let format = policy
                    .format(obj::double_precision(policy))
                    .expect("validated precision");
                tcl_syntax::number::format_double_native_selected(value, policy, format)
            }
            _ => unreachable!("validated arithmetic payload"),
        };
        if index > 0 {
            bytes.push(b' ');
        }
        bytes.extend_from_slice(spelling.as_bytes());
    }
    // SAFETY: actual C9 UpdateString uses Tcl_InitStringRep; zero is canonical.
    unsafe { obj::set_native_updater_string_rep(value, &bytes, true) };
    Ok(bytes)
}

/// Genuine duplicated abstract header retained for the generic-loop cursor.
/// Only Index manufactures a fresh unreferenced value, immediately before WRITE.
pub(crate) struct NativeEachLoopAbstractValues {
    header: obj::Owned,
    length: usize,
}
pub(crate) fn capture_native_each_loop_abstract(
    value: *mut TclObj,
    protocol: NativeStringProtocol,
) -> Result<Option<NativeEachLoopAbstractValues>, ValueError> {
    let Some(length) = length_in(value, protocol)? else {
        return Ok(None);
    };
    let header = obj::Owned::fresh(obj::duplicate(value));
    Ok(Some(NativeEachLoopAbstractValues { header, length }))
}
impl NativeEachLoopAbstractValues {
    pub(crate) fn length(&self) -> usize {
        self.length
    }
    pub(crate) fn element(&self, index: usize) -> Result<*mut TclObj, ValueError> {
        index_in(
            self.header.as_ptr(),
            index,
            header(self.header.as_ptr()).protocol,
        )?
        .ok_or(ValueError::CommandProtocolUnavailable(
            "abstract Index outside captured Length",
        ))
    }
}

#[cfg(test)]
pub(crate) fn physical_state(value: *mut TclObj) -> Option<(usize, usize, bool)> {
    is_series(value).then(|| {
        let backing = &header(value).backing;
        (
            backing.series.length(),
            backing.headers.get(),
            backing.elements.borrow().is_some(),
        )
    })
}

#[cfg(test)]
mod tests;
