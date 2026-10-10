// SPDX-License-Identifier: AGPL-3.0-or-later
//! Currency of an actually entered interpreter operation, independent of purpose.
//!
//! A current receipt grants no native getter, command, value or expression
//! authority. Each operation selects those separately from its actual ingress.
//! Currency uses the existing interpreter guard epoch so changing and restoring
//! an otherwise equal world cannot revive a receipt. No engine borrow is held
//! across a Host call.

use super::{Interp, InterpState};
use std::rc::{Rc, Weak};
use tcl_platform::Host;
use tcl_registry::InvocationDialect;
use tcl_runtime_api::{NativeExecutionError, RuntimeContext, guard::GuardDomain};
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
        if let Some(mut interpreter) = self.interpreter.upgrade().map(Interp) {
            interpreter.refuse_native_execution(first);
        }
        Err(tcl_platform::NumericEnvironmentUnavailable::Target)
    }
}
