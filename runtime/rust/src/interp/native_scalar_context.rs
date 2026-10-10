// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected scalar C ABI ownership, separate from object representations.

use super::{Interp, InterpState};
use crate::obj::{self, TclObj};
use std::rc::{Rc, Weak};
use tcl_platform::{Host, NativeCIntegerAbi};
use tcl_registry::InvocationDialect;
use tcl_runtime_api::{NativeExecutionError, RuntimeContext, guard::GuardDomain};
use tcl_syntax::{scalar_getter::NativeScalarGetterTarget, value::ValueError};

/// A checked scalar access preserves an existing engine refusal or the exact
/// value-owner failure. Neither category is a guest scalar conversion error.
#[derive(Debug, Clone)]
pub enum NativeScalarObjectAccessError {
    /// The actual interpreter already retains an execution refusal.
    Execution(NativeExecutionError),
    /// Original object access failed before a guest scalar result was available.
    Value(ValueError),
}

impl From<ValueError> for NativeScalarObjectAccessError {
    fn from(error: ValueError) -> Self {
        Self::Value(error)
    }
}

fn unavailable(reason: &'static str) -> NativeScalarObjectAccessError {
    ValueError::CommandProtocolUnavailable(reason).into()
}

/// A live engine issues this receipt; neither a cache nor a constructor can.
pub(crate) struct NativeScalarObjectContext {
    interpreter: Weak<InterpState>,
    host: Weak<dyn Host>,
    runtime: RuntimeContext,
    dialect: InvocationDialect,
    abi: NativeCIntegerAbi,
    epoch: u64,
}

pub(crate) struct NativeScalarAccess {
    issuer: Rc<NativeScalarObjectContext>,
    pub(crate) interpreter: Interp,
    pub(crate) host: Rc<dyn Host>,
    pub(crate) dialect: InvocationDialect,
}

impl NativeScalarObjectContext {
    fn issue(interpreter: &Interp) -> Result<Rc<Self>, NativeScalarObjectAccessError> {
        first_refusal(interpreter)?;
        let dialect = interpreter.native_invocation_dialect();
        dialect
            .native_scalar_getter_protocol()
            .ok_or_else(|| unavailable("native scalar C API execution protocol"))?;
        let host = interpreter.host();
        let abi = selected_abi(&host)?;
        let epoch = interpreter
            .guards
            .borrow()
            .domain_epoch(GuardDomain::Interpreter)
            .ok_or_else(|| unavailable("native scalar interpreter policy"))?;
        let issuer = Rc::new(Self {
            interpreter: Rc::downgrade(&interpreter.0),
            host: Rc::downgrade(&host),
            runtime: interpreter.runtime_context(),
            dialect,
            abi,
            epoch,
        });
        first_refusal(interpreter)?;
        if !issuer.matches_current(interpreter, &interpreter.host()) {
            return Err(unavailable("stale native scalar object issuer"));
        }
        Ok(issuer)
    }

    pub(crate) fn same_issuer(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.interpreter, &other.interpreter)
            && Weak::ptr_eq(&self.host, &other.host)
            && self.runtime == other.runtime
            && self.dialect == other.dialect
            && self.abi == other.abi
            && self.epoch == other.epoch
    }

    fn matches_current(&self, interpreter: &Interp, host: &Rc<dyn Host>) -> bool {
        let epoch = interpreter
            .guards
            .borrow()
            .domain_epoch(GuardDomain::Interpreter);
        Weak::ptr_eq(&self.host, &Rc::downgrade(host))
            && interpreter.runtime_context() == self.runtime
            && interpreter.native_invocation_dialect() == self.dialect
            && epoch == Some(self.epoch)
    }

    fn current(self: &Rc<Self>) -> Result<NativeScalarAccess, NativeScalarObjectAccessError> {
        let interpreter = self
            .interpreter
            .upgrade()
            .map(Interp)
            .ok_or_else(|| unavailable("retired native scalar interpreter"))?;
        first_refusal(&interpreter)?;
        let host = interpreter.host();
        if !self.matches_current(&interpreter, &host) {
            return Err(unavailable("stale native scalar object issuer"));
        }
        // Release every engine borrow before calling the actual Host adapter.
        let abi = selected_abi(&host)?;
        first_refusal(&interpreter)?;
        if abi != self.abi || !self.matches_current(&interpreter, &interpreter.host()) {
            return Err(unavailable("stale native scalar object issuer"));
        }
        Ok(NativeScalarAccess {
            issuer: self.clone(),
            interpreter,
            host,
            dialect: self.dialect,
        })
    }
}

fn first_refusal(interpreter: &Interp) -> Result<(), NativeScalarObjectAccessError> {
    match interpreter.native_execution_refusal() {
        Some(error) => Err(NativeScalarObjectAccessError::Execution(error)),
        None => Ok(()),
    }
}

fn selected_abi(host: &Rc<dyn Host>) -> Result<NativeCIntegerAbi, NativeScalarObjectAccessError> {
    let abi = host
        .numeric_environment()
        .ok_or(ValueError::ScalarNumericInputUnavailable)?
        .c_integer_abi()
        .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
    NativeScalarGetterTarget::from_c_integer_abi(abi.char_bits, abi.int_bytes, abi.long_bytes)
        .map_err(|_| ValueError::ScalarNumericInputUnavailable)?;
    // These are output-layout checks after an actual Host ABI observation.
    // They select no parsing, primitive recipe, or interpreter authority.
    if u32::from(abi.int_bytes) * u32::from(abi.char_bits) != core::ffi::c_int::BITS
        || u32::from(abi.long_bytes) * u32::from(abi.char_bits) != core::ffi::c_long::BITS
    {
        return Err(unavailable("native scalar C output ABI"));
    }
    Ok(abi)
}

impl Interp {
    /// Capture the actual scalar engine and queried ABI before creating a
    /// physical scalar representation.
    pub(crate) fn native_scalar_object_issuer(
        &self,
    ) -> Result<Rc<NativeScalarObjectContext>, NativeScalarObjectAccessError> {
        NativeScalarObjectContext::issue(self)
    }

    /// Bind an original object to this actual engine and Host ABI. The receipt
    /// carries no object reference and cannot keep the interpreter alive.
    pub(crate) fn bind_native_scalar_object(
        &self,
        value: *mut TclObj,
    ) -> Result<(), NativeScalarObjectAccessError> {
        first_refusal(self)?;
        obj::check_native_liveness(value)?;
        let incoming = self.native_scalar_object_issuer()?;
        obj::validate_scalar_object_context(value, &incoming)?;
        self.associate_native_jim_arguments(&[value])?;
        obj::bind_scalar_object_context(value, incoming)?;
        Ok(())
    }
}

/// A null C interpreter has only this independently retained original issuer.
pub(crate) fn scalar_access(
    interpreter: Option<&Interp>,
    value: *mut TclObj,
) -> Result<NativeScalarAccess, NativeScalarObjectAccessError> {
    if let Some(interpreter) = interpreter {
        first_refusal(interpreter)?;
        interpreter.bind_native_scalar_object(value)?;
    }
    let issuer = obj::scalar_object_context(value)?
        .ok_or_else(|| unavailable("native scalar C API object issuer"))?;
    let access = issuer.current()?;
    access
        .interpreter
        .associate_native_jim_arguments(&[value])?;
    if let Some(interpreter) = interpreter {
        if !Rc::ptr_eq(&interpreter.0, &access.interpreter.0) {
            return Err(unavailable("foreign native scalar object issuer"));
        }
    }
    Ok(access)
}

impl NativeScalarAccess {
    pub(crate) fn ensure_current(&self) -> Result<(), NativeScalarObjectAccessError> {
        self.issuer.current().map(|_| ())
    }
}
