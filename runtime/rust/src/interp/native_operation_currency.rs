// SPDX-License-Identifier: AGPL-3.0-or-later
//! Currency of an actually entered interpreter operation, independent of purpose.
//!
//! A current receipt grants no native getter, command, value or expression
//! authority. Each operation selects those separately from its actual ingress.
//! Currency uses the existing interpreter guard epoch so changing and restoring
//! an otherwise equal world cannot revive a receipt. No engine borrow is held
//! across a Host call.

use super::{Interp, InterpState};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};
use tcl_platform::Host;
use tcl_registry::InvocationDialect;
use tcl_runtime_api::{guard::GuardDomain, NativeExecutionError, RuntimeContext};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

pub(crate) struct NativeOperationCurrency {
    interpreter: Weak<InterpState>,
    host: Weak<dyn Host>,
    context: RuntimeContext,
    dialect: InvocationDialect,
    epoch: u64,
}

fn unavailable() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "stale entered native operation",
    ))
}

impl NativeOperationCurrency {
    pub(crate) fn issue(interpreter: &Interp) -> Result<Self, NativeExecutionError> {
        if let Some(first) = interpreter.native_execution_refusal() {
            return Err(first);
        }
        let host = interpreter.host();
        let epoch = interpreter
            .guards
            .borrow()
            .domain_epoch(GuardDomain::Interpreter)
            .ok_or_else(unavailable)?;
        let receipt = Self {
            interpreter: Rc::downgrade(&interpreter.0),
            host: Rc::downgrade(&host),
            context: interpreter.runtime_context(),
            dialect: interpreter.native_invocation_dialect(),
            epoch,
        };
        receipt.ensure_current()?;
        Ok(receipt)
    }

    pub(crate) fn ensure_current(&self) -> Result<(), NativeExecutionError> {
        let interpreter = self
            .interpreter
            .upgrade()
            .map(Interp)
            .ok_or_else(unavailable)?;
        if let Some(first) = interpreter.native_execution_refusal() {
            return Err(first);
        }
        let current_host = interpreter.host();
        let epoch = interpreter
            .guards
            .borrow()
            .domain_epoch(GuardDomain::Interpreter);
        if !Weak::ptr_eq(&self.host, &Rc::downgrade(&current_host))
            || interpreter.runtime_context() != self.context
            || interpreter.native_invocation_dialect() != self.dialect
            || epoch != Some(self.epoch)
        {
            return Err(unavailable());
        }
        Ok(())
    }

    /// Retain the actual first cause when a reached object stage invalidates
    /// this operation before a later conversion or publication.
    pub(crate) fn ensure_current_or_refuse(&self) -> Result<(), NativeExecutionError> {
        let current = self.ensure_current();
        if let Err(first) = &current {
            self.retain_refusal(first);
        }
        current
    }

    /// Check around exactly one reached Host call. Retain its effects and the
    /// original first refusal even when that call also reports unavailability.
    pub(crate) fn host_call<T>(
        &self,
        operation: impl FnOnce() -> Result<T, tcl_platform::NumericEnvironmentUnavailable>,
    ) -> Result<T, tcl_platform::NumericEnvironmentUnavailable> {
        let first = self.ensure_current();
        if let Err(first) = first {
            return self.refuse(first);
        }
        let result = operation();
        if let Err(first) = self.ensure_current() {
            return self.refuse(first);
        }
        result
    }

    fn refuse<T>(
        &self,
        first: NativeExecutionError,
    ) -> Result<T, tcl_platform::NumericEnvironmentUnavailable> {
        self.retain_refusal(&first);
        Err(tcl_platform::NumericEnvironmentUnavailable::Target)
    }

    fn retain_refusal(&self, first: &NativeExecutionError) {
        if let Some(mut interpreter) = self.interpreter.upgrade().map(Interp) {
            interpreter.refuse_native_execution(first.clone());
        }
    }
}

/// Each real numeric Host stage belongs to the original entered operation.
/// The ABI cell can start from an independently sealed object issuer; neutral
/// instruction utilities explicitly keep their own unissued first observation.
pub(crate) struct CheckedNumericEnvironment<'a> {
    actual: &'a dyn tcl_platform::NumericEnvironment,
    currency: Option<&'a NativeOperationCurrency>,
    abi: &'a Cell<Option<tcl_platform::NativeCIntegerAbi>>,
}

impl<'a> CheckedNumericEnvironment<'a> {
    pub(crate) fn new(
        actual: &'a dyn tcl_platform::NumericEnvironment,
        currency: Option<&'a NativeOperationCurrency>,
        abi: &'a Cell<Option<tcl_platform::NativeCIntegerAbi>>,
    ) -> Self {
        Self {
            actual,
            currency,
            abi,
        }
    }

    fn call<T>(
        &self,
        operation: impl FnOnce() -> Result<T, tcl_platform::NumericEnvironmentUnavailable>,
    ) -> Result<T, tcl_platform::NumericEnvironmentUnavailable> {
        match self.currency {
            Some(currency) => currency.host_call(operation),
            None => operation(),
        }
    }
}

impl tcl_platform::NumericEnvironment for CheckedNumericEnvironment<'_> {
    fn c_integer_abi(
        &self,
    ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable> {
        let actual = self.call(|| self.actual.c_integer_abi())?;
        if self.abi.get().is_some_and(|before| before != actual) {
            return Err(tcl_platform::NumericEnvironmentUnavailable::Target);
        }
        self.abi.set(Some(actual));
        Ok(actual)
    }
    fn state(
        &self,
    ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable> {
        self.call(|| self.actual.state())
    }
    fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
        self.call(|| self.actual.reset())
    }
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.call(|| self.actual.unsigned_c84(input, offset, long))
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.call(|| self.actual.unsigned(input, offset, base))
    }
    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<tcl_platform::SignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.call(|| self.actual.signed_long(input, base))
    }
    fn double(
        &self,
        input: &[u8],
        reset: bool,
    ) -> Result<tcl_platform::DoubleNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.call(|| self.actual.double(input, reset))
    }
}
