// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public primitive reads preserve original cache effects and nullable errors.

use super::{Interp, TclObj, TCL_ERROR};
use crate::{
    interp::{
        native_operation_currency::{CheckedNumericEnvironment, NativeOperationCurrency},
        native_scalar_context::{self, NativeScalarAccess},
    },
    typed_value,
};
use core::ffi::c_int;
use std::cell::Cell;
use tcl_syntax::{
    scalar_getter::{NativeScalarGetterFailure, NativeScalarGetterKind, NativeScalarGetterValue},
    value::ValueError,
};

pub use crate::interp::native_scalar_context::NativeScalarObjectAccessError;

/// Read the actual primitive without publishing a guest message or error code.
/// `None` requires an independently retained original object's engine issuer.
/// Host refusals remain typed; a rejected conversion is the inner `Err`.
///
/// # Errors
/// Returns the retained host cause or a missing, stale, foreign or unavailable
/// object/engine access. A reached guest conversion failure is the inner `Err`.
///
/// # Safety
/// The caller retains the original runtime object allocation for the entire read.
pub unsafe fn probe_scalar_getter(
    interpreter: Option<&Interp>,
    original: *mut TclObj,
    kind: NativeScalarGetterKind,
) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, NativeScalarObjectAccessError>
{
    let access = native_scalar_context::scalar_access(interpreter, original)?;
    probe(&access, original, kind)
}

/// Retain the actual scalar engine and Host ABI independently of the object's
/// representation. This operation changes neither cache nor native refcount.
///
/// # Errors
/// Returns the original host refusal, invalid object access or unavailable
/// engine/ABI issuer; an existing foreign or stale binding cannot be replaced.
///
/// # Safety
/// The caller retains the original runtime object allocation for the entire binding.
pub unsafe fn bind_scalar_getter_context(
    interpreter: &Interp,
    original: *mut TclObj,
) -> Result<(), NativeScalarObjectAccessError> {
    interpreter.bind_native_scalar_object(original)
}

fn probe(
    access: &NativeScalarAccess,
    original: *mut TclObj,
    kind: NativeScalarGetterKind,
) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, NativeScalarObjectAccessError>
{
    let currency = NativeOperationCurrency::issue(&access.interpreter)
        .map_err(NativeScalarObjectAccessError::Execution)?;
    let environment = access.host.numeric_environment();
    currency
        .ensure_current_or_refuse()
        .map_err(NativeScalarObjectAccessError::Execution)?;
    let environment = environment.ok_or(ValueError::ScalarNumericInputUnavailable)?;
    let abi = Cell::new(Some(access.c_integer_abi()));
    let environment = CheckedNumericEnvironment::new(environment, Some(&currency), &abi);
    let outcome = typed_value::native_scalar_probe_with_environment_and_currency(
        original,
        access.dialect,
        kind,
        Some(&environment),
        Some(&currency),
    );
    // Preserve a reached first Host cause before a generic unavailable result.
    currency
        .ensure_current_or_refuse()
        .map_err(NativeScalarObjectAccessError::Execution)?;
    access.ensure_current()?;
    Ok(outcome?)
}

fn guest_error(
    access: &NativeScalarAccess,
    original: *mut TclObj,
    kind: NativeScalarGetterKind,
    failure: NativeScalarGetterFailure,
) -> Result<ValueError, NativeScalarObjectAccessError> {
    let error =
        typed_value::native_scalar_failure_presentation(original, access.dialect, kind, failure);
    access.ensure_current()?;
    Ok(ValueError::NativeScalarGetter(Box::new(error?)))
}

/// A conversion failure is distinct from inaccessible original engine data.
pub(crate) enum ScalarReadError {
    Access(NativeScalarObjectAccessError),
    Guest(ValueError),
}

impl ScalarReadError {
    /// Publish on the original supplied interpreter, preserving its first Host.
    pub(crate) fn publish(self, interpreter: &mut Interp) -> c_int {
        match self {
            Self::Access(error) => {
                // SAFETY: this reference is the live original interpreter.
                unsafe { publish_access_error(interpreter, error) }
            }
            Self::Guest(error) => {
                interpreter.report_cmd_error(error.into());
                TCL_ERROR
            }
        }
    }
}

/// The live primitive read shared by C extension and compiler transport.
/// It probes once and renders only that reached failure. This result grants
/// no expression-instruction purpose or C callback bookkeeping.
pub(crate) fn read_scalar_for_interpreter(
    interpreter: &Interp,
    original: *mut TclObj,
    kind: NativeScalarGetterKind,
) -> Result<NativeScalarGetterValue, ScalarReadError> {
    let access = native_scalar_context::scalar_access(Some(interpreter), original)
        .map_err(ScalarReadError::Access)?;
    match probe(&access, original, kind).map_err(ScalarReadError::Access)? {
        Ok(value) => Ok(value),
        Err(failure) => {
            let error =
                guest_error(&access, original, kind, failure).map_err(ScalarReadError::Access)?;
            Err(ScalarReadError::Guest(error))
        }
    }
}

/// Execute once. Only a supplied interpreter renders a reached guest failure.
/// Null reads retain all getter effects and skip the diagnostic string access.
///
/// # Safety
/// The interpreter is null or live, and the original object is retained.
pub(super) unsafe fn read(
    interpreter: *mut Interp,
    original: *mut TclObj,
    kind: NativeScalarGetterKind,
) -> Result<NativeScalarGetterValue, c_int> {
    // SAFETY: the caller supplies this live interpreter or null.
    if let Some(interpreter) = unsafe { interpreter.as_mut() } {
        return read_scalar_for_interpreter(interpreter, original, kind).map_err(|error| {
            let guest = matches!(&error, ScalarReadError::Guest(_));
            let status = error.publish(interpreter);
            if guest && !interpreter.host_refusal_pending() {
                interpreter.note_c_api_error();
            }
            status
        });
    }
    let access = native_scalar_context::scalar_access(None, original).map_err(|_| TCL_ERROR)?;
    probe(&access, original, kind)
        .map_err(|_| TCL_ERROR)?
        .map_err(|_| TCL_ERROR)
}

/// A host refusal never becomes a guest scalar message or synthesized code.
///
/// # Safety
/// The interpreter is null or live.
pub(crate) unsafe fn publish_access_error(
    interpreter: *mut Interp,
    error: NativeScalarObjectAccessError,
) -> c_int {
    // SAFETY: forwarded from the caller's nullable-interpreter contract.
    if let Some(interpreter) = unsafe { interpreter.as_mut() } {
        match error {
            NativeScalarObjectAccessError::Execution(error) => {
                interpreter.refuse_native_execution(error);
            }
            NativeScalarObjectAccessError::Value(error) => {
                if let Some(cause) = error.native_access_refusal() {
                    interpreter.refuse_native_access(cause);
                } else {
                    interpreter.refuse_host_command(error.to_string());
                }
            }
        }
    }
    TCL_ERROR
}

/// Check only the writable Rust ABI width after the primitive has returned.
/// A failed transport cannot become a guest conversion failure.
pub(super) fn output_integer<T: TryFrom<i64>>(value: i64) -> Option<T> {
    value.try_into().ok()
}

pub(super) unsafe fn unexpected_output(interpreter: *mut Interp) -> c_int {
    // SAFETY: forwarded from each getter's nullable-interpreter contract.
    unsafe {
        publish_access_error(
            interpreter,
            ValueError::ScalarNumericInputUnavailable.into(),
        )
    }
}

#[cfg(test)]
mod tests;
