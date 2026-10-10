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

#[derive(Clone)]
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
        // Nested work inherits the original entry. Reissuing after a callback
        // would permit a changed and restored world to revive an operation.
        let inherited = interpreter
            .entered_native_operations
            .borrow()
            .last()
            .cloned();
        if let Some(inherited) = inherited {
            inherited.ensure_current_or_refuse()?;
            return Ok(inherited.as_ref().clone());
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

/// Retain one actually entered interpreter and inherit its original currency.
/// This scope grants no command, name, getter, value or expression purpose.
/// Mandatory cleanup may continue after refusal; checked stages may not.
pub(crate) struct NativeOperationScope {
    interpreter: Interp,
    currency: Rc<NativeOperationCurrency>,
    previous_len: usize,
}

impl NativeOperationScope {
    pub(crate) fn enter(interpreter: &Interp) -> Result<Self, NativeExecutionError> {
        let currency = Rc::new(NativeOperationCurrency::issue(interpreter)?);
        currency.ensure_current_or_refuse()?;
        let previous_len = interpreter.entered_native_operations.borrow().len();
        interpreter
            .entered_native_operations
            .borrow_mut()
            .push(currency.clone());
        Ok(Self {
            interpreter: interpreter.clone(),
            currency,
            previous_len,
        })
    }

    pub(crate) fn currency(&self) -> &NativeOperationCurrency {
        &self.currency
    }
}

impl Drop for NativeOperationScope {
    fn drop(&mut self) {
        let mut entered = self.interpreter.entered_native_operations.borrow_mut();
        debug_assert_eq!(entered.len(), self.previous_len + 1);
        let original = entered.pop();
        debug_assert!(original.is_some_and(|original| Rc::ptr_eq(&original, &self.currency)));
    }
}

impl Interp {
    /// Check the original entered operation without holding its stack borrow
    /// across a getter or callback. Neutral callers retain their explicit
    /// operation semantics; a previously reached Host cause is still terminal.
    pub(crate) fn check_entered_native_operation(&self) -> Result<(), NativeExecutionError> {
        if let Some(first) = self.native_execution_refusal() {
            return Err(first);
        }
        let entered = self.entered_native_operations.borrow().last().cloned();
        entered.map_or(Ok(()), |currency| currency.ensure_current_or_refuse())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{frame::VarError, interp::Code, obj};
    use std::cell::Cell;
    use tcl_runtime_api::native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    };

    fn original() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    fn change_and_restore(interpreter: &mut Interp) {
        let original = interpreter.runtime_context();
        let mut changed = original.clone();
        changed.packages = vec![("entered-operation-test".to_owned(), "1.0".to_owned())];
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
    }

    fn stale() -> NativeExecutionError {
        NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation"),
        )
    }

    #[test]
    fn original_nested_operation_stops_before_a_later_store_after_context_restore() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software-only original operation currency; no external Tcl process claim.
        let mut interpreter = original();
        let old = obj::Owned::fresh(obj::new_string_bytes(b"old\0value"));
        interpreter.var_set(b"x", old.as_ptr()).unwrap();
        let before = unsafe { (*old.as_ptr()).ref_count };
        let root = NativeOperationScope::enter(&interpreter).unwrap();
        let nested = NativeOperationScope::enter(&interpreter).unwrap();
        change_and_restore(&mut interpreter);
        let attempted = obj::Owned::fresh(obj::new_string_bytes(b"replacement"));
        assert_eq!(
            interpreter.var_set(b"x", attempted.as_ptr()),
            Err(VarError::NameProtocolUnavailable)
        );
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(unsafe { (*old.as_ptr()).ref_count }, before);
        assert_eq!(unsafe { (*attempted.as_ptr()).ref_count }, 1);
        assert!(NativeOperationScope::enter(&interpreter).is_err());
        assert!(NativeOperationCurrency::issue(&interpreter).is_err());
        drop(nested);
        drop(root);
        assert!(interpreter.entered_native_operations.borrow().is_empty());
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
    }

    struct Observer {
        calls: Rc<Cell<usize>>,
        change: bool,
        guest: bool,
    }
    impl NativeVariableObserver<Interp> for Observer {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            interpreter: &mut Interp,
            access: NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            assert_eq!(access.name1, b"x");
            assert_eq!(access.operation, NativeVariableTraceOperation::Write);
            self.calls.set(self.calls.get() + 1);
            if self.change {
                change_and_restore(interpreter);
            }
            if self.guest {
                Err(tcl_cmd_core::CmdError::with_error_code(
                    "ordinary trace failure",
                    "USER TRACE",
                ))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn original_observer_refusal_keeps_reached_store_and_stops_later_observers() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Both callbacks are genuine physical registrations; the first Host
        // cause does not permit trace-error rendering or a later callback.
        let mut interpreter = original();
        let name = obj::Owned::fresh(obj::new_string_bytes(b"x"));
        let later = Rc::new(Cell::new(0));
        let first = Rc::new(Cell::new(0));
        interpreter
            .add_native_variable_observer(
                name.as_ptr(),
                &[NativeVariableTraceOperation::Write],
                Rc::new(Observer {
                    calls: later.clone(),
                    change: false,
                    guest: false,
                }),
            )
            .unwrap();
        interpreter
            .add_native_variable_observer(
                name.as_ptr(),
                &[NativeVariableTraceOperation::Write],
                Rc::new(Observer {
                    calls: first.clone(),
                    change: true,
                    guest: true,
                }),
            )
            .unwrap();
        let value = obj::Owned::fresh(obj::new_string_bytes(b"reached\0store"));
        let entry = NativeOperationScope::enter(&interpreter).unwrap();
        assert_eq!(
            interpreter.var_set(b"x", value.as_ptr()),
            Err(VarError::TraceError)
        );
        assert_eq!(first.get(), 1);
        assert_eq!(later.get(), 0);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(
            unsafe { (*value.as_ptr()).ref_count },
            2,
            "the original store remains reached"
        );
        assert!(interpreter.active_var_trace_scopes.borrow().is_empty());
        drop(entry);
        assert!(interpreter.entered_native_operations.borrow().is_empty());
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
    }

    #[test]
    fn original_operation_retains_ordinary_guest_completion_and_nested_cells() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Positive current operation and full genuine Guest completion, with
        // no Host cause or external-provider equivalence inferred.
        let mut interpreter = original();
        let entry = NativeOperationScope::enter(&interpreter).unwrap();
        {
            let nested = NativeOperationScope::enter(&interpreter).unwrap();
            assert_eq!(
                interpreter.eval_str(b"set x {A B}; return -code 7 -level 0 $x"),
                Code::Other(7)
            );
            nested.currency().ensure_current().unwrap();
        }
        let completion = crate::state_traits::capture_completion_checked(
            &mut interpreter,
            entry.currency(),
            Code::Other(7),
        )
        .unwrap();
        assert_eq!(completion.code, tcl_runtime_api::Code::Other(7));
        let result = unsafe { obj::Owned::from_raw(completion.result) };
        let options = unsafe { obj::Owned::from_raw(completion.options) };
        assert_eq!(obj::bytes_of(result.as_ptr()), b"A B");
        let pairs =
            tcl_syntax::value::ValueOps::dict_pairs(&mut interpreter, &options.as_ptr()).unwrap();
        let option_code = pairs
            .iter()
            .find_map(|(key, value)| (obj::bytes_of(*key) == b"-code").then_some(*value))
            .unwrap();
        assert_eq!(obj::bytes_of(option_code), b"7");
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(
            interpreter.read_named_variable(b"x").unwrap(),
            interpreter.var_get(b"x").unwrap()
        );
        drop(entry);
        assert!(interpreter.entered_native_operations.borrow().is_empty());
    }

    fn changing_script_callback(interpreter: &mut Interp, _: &[*mut obj::TclObj]) -> Code {
        change_and_restore(interpreter);
        Code::Ok
    }

    #[test]
    fn original_script_trace_refusal_stops_the_later_script_command() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // This reaches a real source trace and command callback. Configuration
        // currency is independent of the selected variable/command identities.
        let mut interpreter = original();
        interpreter.register_builtin(b"change_context", changing_script_callback);
        assert_eq!(
            interpreter.eval_str(b"set x initial; trace add variable x write change_context"),
            Code::Ok
        );
        let entry = NativeOperationScope::enter(&interpreter).unwrap();
        assert_eq!(
            interpreter.eval_str(b"set x reached; set after unreached"),
            Code::Error
        );
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert!(interpreter.active_var_trace_scopes.borrow().is_empty());
        assert_eq!(interpreter.eval_depth.get(), 0);
        // Inspect actual storage only after the operation has stopped. These
        // are inspection controls, not new lookup or conversion authority.
        let actual = crate::vars::get(
            &interpreter.frames.borrow(),
            &interpreter.namespaces.borrow(),
            interpreter.current_ns.get(),
            b"x",
        )
        .unwrap();
        assert_eq!(obj::bytes_of(actual), b"reached");
        assert!(crate::vars::get(
            &interpreter.frames.borrow(),
            &interpreter.namespaces.borrow(),
            interpreter.current_ns.get(),
            b"after"
        )
        .is_none());
        drop(entry);
        assert!(interpreter.entered_native_operations.borrow().is_empty());
    }

    extern "C" fn entered_updater(original: *mut obj::TclObj) {
        // The actual test retains this original interpreter through the updater.
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        change_and_restore(&mut interpreter);
        unsafe { obj::set_native_updater_string_rep(original, b"reached\0bytes", false) };
    }
    static ENTERED_STRING_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"enteredOriginalString".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(entered_updater),
        set_from_any_proc: None,
    };

    #[test]
    fn original_string_updater_retains_reached_bytes_and_refuses_later_guest_capture() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // An authentic extension descriptor reaches its original header. Its
        // changed/restored context never creates a result-publication receipt.
        let mut interpreter = original();
        let value = obj::Owned::fresh(obj::alloc_typed(&ENTERED_STRING_TYPE, 0));
        let references = unsafe { (*value.as_ptr()).ref_count };
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
        let entry = NativeOperationScope::enter(&interpreter).unwrap();
        assert!(interpreter
            .native_object_string_bytes(value.as_ptr())
            .is_err());
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(obj::bytes_of(value.as_ptr()), b"reached\0bytes");
        assert_eq!(
            obj::obj_type_ptr(value.as_ptr()),
            &ENTERED_STRING_TYPE as *const _
        );
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, references);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert!(crate::state_traits::capture_completion_checked(
            &mut interpreter,
            entry.currency(),
            Code::Ok
        )
        .is_err());
        assert_eq!(interpreter.get_obj_result(), prior);
        drop(entry);
        crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
        assert!(interpreter.entered_native_operations.borrow().is_empty());
    }
}
