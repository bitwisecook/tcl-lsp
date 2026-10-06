// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual concat headers, borrowed original children and selected getters.
use crate::{
    interp::Interp,
    list,
    obj::{self, TclObj},
};
use std::rc::Rc;
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
use tcl_syntax::native_string::{NativeStringProtocol, NativeStringStorageIdentity as Storage};
use tcl_syntax::value::{NativeConcatFirstElement, NativeConcatListShape as Shape, ValueError};

fn protocol(interp: &Interp) -> Result<NativeStringProtocol, ValueError> {
    interp
        .native_invocation_dialect()
        .native_string_protocol()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "native concat string issuer",
        ))
}
pub(super) fn shape(interp: &Interp, value: *mut TclObj) -> Result<Shape, ValueError> {
    obj::check_native_liveness(value)?;
    let selected = protocol(interp)?;
    if crate::native_arithseries::is_series(value) {
        crate::native_arithseries::length_in(value, selected)?;
        return Ok(Shape::Indexed);
    }
    list::seal_string_protocol(value, selected)?;
    let snapshot = obj::native_object_snapshot(value)?;
    Ok(match snapshot.cache {
        Cache::List { length, canonical } => Shape::List {
            length,
            canonical,
            resident_length: snapshot.resident.as_ref().map(|bytes| bytes.len()),
        },
        _ => Shape::Other,
    })
}
pub(super) fn empty(interp: &Interp) -> Result<*mut TclObj, ValueError> {
    Ok(list::new_list_obj_native(&[], protocol(interp)?))
}
pub(super) fn copy(interp: &Interp, value: *mut TclObj) -> Result<*mut TclObj, ValueError> {
    Ok(list::native_list_copy(value, protocol(interp)?)?.into_native_unowned())
}
pub(super) fn append(
    interp: &Interp,
    target: *mut TclObj,
    source: *mut TclObj,
) -> Result<(), ValueError> {
    let selected = protocol(interp)?;
    let members = list::list_elements_native_checked(source, selected)?;
    if !members.is_empty() {
        list::append_prepared_native_elements(target, &members, selected)?;
    }
    Ok(())
}
pub(super) fn first(
    interp: &mut Interp,
    source: *mut TclObj,
) -> Result<Option<NativeConcatFirstElement<*mut TclObj>>, ValueError> {
    let selected = protocol(interp)?;
    if crate::native_arithseries::is_series(source) {
        return crate::native_arithseries::index_in(source, 0, selected)?
            .map(
                |original| match interp.native_object_string_bytes(original) {
                    Ok(bytes) => Ok(NativeConcatFirstElement {
                        bytes,
                        temporary: Some(original),
                    }),
                    Err(error) => {
                        drop(obj::Owned::fresh(original));
                        Err(error)
                    }
                },
            )
            .transpose();
    }
    let members = list::list_elements_native_checked(source, selected)?;
    members
        .first()
        .map(|&value| {
            interp
                .native_object_string_bytes(value)
                .map(|bytes| NativeConcatFirstElement {
                    bytes,
                    temporary: None,
                })
        })
        .transpose()
}
pub(super) fn string(interp: &Interp, bytes: &[u8]) -> Result<*mut TclObj, ValueError> {
    let selected = protocol(interp)?;
    let value = obj::Owned::fresh(obj::new_string_bytes(bytes));
    if selected
        .tcl_version()
        .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_5)
    {
        obj::set_native_append_string(
            value.as_ptr(),
            selected,
            Some((Rc::from(bytes), Storage::Allocated)),
            None,
            None,
        )?;
    } else {
        // SAFETY: the fresh live header owns this allocated result storage.
        unsafe {
            obj::set_native_updater_string_rep(value.as_ptr(), bytes, false);
        }
    }
    Ok(value.into_native_unowned())
}
