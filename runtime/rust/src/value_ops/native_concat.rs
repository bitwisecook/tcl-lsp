// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual concat headers, borrowed original children and selected getters.
use crate::{
    interp::{native_arguments::OriginalArguments, Interp},
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
    let arguments = OriginalArguments::enter(interp, &[value]).map_err(unwind)?;
    arguments
        .check_original_for(interp, 0, value)
        .map_err(unwind)?;
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
    let original = OriginalArguments::enter(interp, &[]).map_err(unwind)?;
    original.check_for(interp).map_err(unwind)?;
    Ok(list::new_list_obj_native(&[], protocol(interp)?))
}
// The portable ValueOps port keeps its ValueError shape. Full Host/currency
// failures stay retained in the actual original operation; this carrier only
// unwinds the same worker and cannot classify them as a new Guest error.
fn unwind(error: tcl_cmd_core::CmdError) -> ValueError {
    error
        .native_access_refusal()
        .map(Into::into)
        .unwrap_or(ValueError::CommandProtocolUnavailable(
            "native concat original operation",
        ))
}
fn retained(arguments: &OriginalArguments, error: ValueError) -> ValueError {
    let _ = arguments.retain_refusal(error.clone().into());
    error
}
pub(super) fn copy(interp: &Interp, value: *mut TclObj) -> Result<*mut TclObj, ValueError> {
    let arguments = OriginalArguments::enter(interp, &[value]).map_err(unwind)?;
    let copy = list::native_list_copy_using(
        value,
        protocol(interp)?,
        &|| {
            arguments
                .check_original_for(interp, 0, value)
                .map_err(unwind)
        },
        &|error| retained(&arguments, error),
    )?;
    arguments
        .check_original_for(interp, 0, value)
        .map_err(unwind)?;
    Ok(copy.into_native_unowned())
}
pub(super) fn append(
    interp: &Interp,
    target: *mut TclObj,
    source: *mut TclObj,
) -> Result<(), ValueError> {
    let arguments = OriginalArguments::enter(interp, &[target, source]).map_err(unwind)?;
    let selected = protocol(interp)?;
    let check = || {
        arguments
            .check_original_for(interp, 0, target)
            .map_err(unwind)?;
        arguments
            .check_original_for(interp, 1, source)
            .map_err(unwind)
    };
    let members = list::list_elements_native_using(source, selected, &check, &|error| {
        retained(&arguments, error)
    })?;
    if !members.is_empty() {
        list::append_prepared_native_elements_using(
            target,
            &members,
            selected,
            &check,
            &|error| retained(&arguments, error),
        )?;
    }
    check()
}
pub(super) fn first(
    interp: &mut Interp,
    source: *mut TclObj,
) -> Result<Option<NativeConcatFirstElement<*mut TclObj>>, ValueError> {
    let arguments = OriginalArguments::enter(interp, &[source]).map_err(unwind)?;
    let selected = protocol(interp)?;
    if crate::native_arithseries::is_series(source) {
        let created = crate::native_arithseries::index_in(source, 0, selected)
            .map_err(|error| retained(&arguments, error))?;
        if let Err(first) = arguments.check_original_for(interp, 0, source) {
            if let Some(original) = created {
                drop(obj::Owned::fresh(original));
            }
            let _ = arguments.check_for(interp);
            return Err(unwind(first));
        }
        if let Some(original) = created {
            let actual = OriginalArguments::enter(interp, &[original]).map_err(unwind);
            let reached = match actual {
                Ok(actual) => {
                    let bytes = interp.native_object_string_bytes(original);
                    let checked = actual
                        .check_original_for(interp, 0, original)
                        .map_err(unwind);
                    match checked {
                        Ok(()) => bytes.map_err(|error| retained(&arguments, error)),
                        Err(first) => Err(first),
                    }
                }
                Err(first) => Err(first),
            };
            let bytes = match reached {
                Ok(bytes) => bytes,
                Err(error) => {
                    let first = retained(&arguments, error);
                    drop(obj::Owned::fresh(original));
                    arguments.check_for(interp).map_err(unwind)?;
                    return Err(first);
                }
            };
            let current = arguments
                .check_original_for(interp, 0, source)
                .map_err(unwind);
            if let Err(first) = current {
                drop(obj::Owned::fresh(original));
                arguments.check_for(interp).map_err(unwind)?;
                return Err(first);
            }
            return Ok(Some(NativeConcatFirstElement {
                bytes,
                temporary: Some(original),
            }));
        }
        return Ok(None);
    }
    let actual = interp.clone();
    let members = list::list_elements_native_using(
        source,
        selected,
        &|| {
            arguments
                .check_original_for(&actual, 0, source)
                .map_err(unwind)
        },
        &|error| retained(&arguments, error),
    )?;
    if let Some(&value) = members.first() {
        let member = OriginalArguments::enter(interp, &[value]).map_err(unwind)?;
        let reached = interp.native_object_string_bytes(value);
        member
            .check_original_for(interp, 0, value)
            .map_err(unwind)?;
        arguments
            .check_original_for(interp, 0, source)
            .map_err(unwind)?;
        return reached
            .map(|bytes| {
                Some(NativeConcatFirstElement {
                    bytes,
                    temporary: None,
                })
            })
            .map_err(|error| retained(&arguments, error));
    }
    Ok(None)
}

pub(super) fn string(interp: &Interp, bytes: &[u8]) -> Result<*mut TclObj, ValueError> {
    let original = OriginalArguments::enter(interp, &[]).map_err(unwind)?;
    original.check_for(interp).map_err(unwind)?;
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

impl Interp {
    /// Preserve the actual complete input operation through the same selected
    /// concat worker, including its reached working-header/member retirement.
    /// Currency is independent of Native/Logical concat capability selection.
    pub(crate) fn concat_original_arguments(
        &mut self,
        values: &[*mut TclObj],
    ) -> Result<*mut TclObj, tcl_cmd_core::CmdError> {
        let arguments = OriginalArguments::enter(self, values)?;
        let reached = tcl_cmd_core::list::concat_selected(self, values);
        if let Err(first) = arguments.check_for(self) {
            if let Ok(created) = reached {
                drop(obj::Owned::fresh(created));
            }
            // Reached retirement cannot replace the already retained cause.
            let _ = arguments.check_for(self);
            return Err(first);
        }
        reached.map_err(|error| arguments.retain_refusal(error))
    }
}

#[cfg(test)]
#[path = "native_concat/original_conversion_tests.rs"]
mod original_conversion_tests;
