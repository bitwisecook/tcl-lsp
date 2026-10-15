// SPDX-License-Identifier: AGPL-3.0-or-later
//! Currency of an actually entered interpreter operation, independent of purpose.
//!
//! A current receipt grants no native getter, command, value or expression
//! authority. Each operation selects those separately from its actual ingress.
//! Currency uses the actual execution-context epoch: changing and restoring
//! Host, dialect, pinned world or logical ingress cannot revive a receipt.
//! Legal Guest mutations invalidate their separate speculative policy guards.
//! No engine borrow is held across a Host call.

use super::{Interp, InterpState};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
};
use tcl_platform::Host;
use tcl_registry::InvocationDialect;
use tcl_runtime_api::{NativeExecutionError, RuntimeContext};
use tcl_syntax::raw_string::NativeValueAccessRefusal;

#[derive(Clone)]
pub(crate) struct NativeOperationCurrency {
    interpreter: Weak<InterpState>,
    host: Weak<dyn Host>,
    context: RuntimeContext,
    dialect: InvocationDialect,
    epoch: u64,
}

/// The stack distinguishes a genuine entered receipt from mandatory cleanup
/// whose original capture already refused. Cleanup never mints new currency.
pub(super) enum NativeOperationEntry {
    Current(Rc<NativeOperationCurrency>),
    RefusedCleanup(NativeExecutionError),
}

impl NativeOperationEntry {
    fn check(&self, interpreter: &Interp) -> Result<(), NativeExecutionError> {
        match self {
            Self::Current(currency) => currency.ensure_current_or_refuse(),
            Self::RefusedCleanup(first) => {
                interpreter.clone().refuse_native_execution(first.clone());
                Err(interpreter
                    .native_execution_refusal()
                    .unwrap_or_else(|| first.clone()))
            }
        }
    }
}

/// Actual saved target and original issue result. Saving a receipt does not
/// keep an operation entered or supply another semantic purpose.
#[derive(Clone)]
pub(crate) struct SavedNativeOperationCurrency {
    interpreter: Weak<InterpState>,
    original: Result<NativeOperationCurrency, NativeExecutionError>,
}

impl SavedNativeOperationCurrency {
    pub(crate) fn capture(interpreter: &Interp) -> Self {
        // Record the real target before issue can fail. A failed projection
        // cannot infer its cleanup target from a diagnostic or ambient entry.
        let target = Rc::downgrade(&interpreter.0);
        let original = NativeOperationCurrency::issue(interpreter);
        if let Err(first) = &original {
            interpreter.clone().refuse_native_execution(first.clone());
        }
        Self {
            interpreter: target,
            original,
        }
    }

    fn ensure_target(&self, actual: &Interp) -> Result<(), NativeExecutionError> {
        if !Weak::ptr_eq(&self.interpreter, &Rc::downgrade(&actual.0)) {
            let refusal = NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "saved native operation belongs to another interpreter",
                ),
            );
            actual.clone().refuse_native_execution(refusal.clone());
            return Err(actual.native_execution_refusal().unwrap_or(refusal));
        }
        Ok(())
    }

    pub(crate) fn check(&self, actual: &Interp) -> Result<(), NativeExecutionError> {
        self.ensure_target(actual)?;
        match &self.original {
            Ok(currency) => currency.ensure_current_or_refuse(),
            Err(first) => {
                actual.clone().refuse_native_execution(first.clone());
                Err(actual
                    .native_execution_refusal()
                    .unwrap_or_else(|| first.clone()))
            }
        }
    }
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
            inherited.check(interpreter)?;
            return match inherited.as_ref() {
                NativeOperationEntry::Current(currency) => Ok(currency.as_ref().clone()),
                NativeOperationEntry::RefusedCleanup(first) => Err(first.clone()),
            };
        }
        let host = interpreter.host();
        let epoch = interpreter
            .native_operation_context_epoch()
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
        let epoch = interpreter.native_operation_context_epoch();
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

    pub(crate) fn retain_refusal(&self, first: &NativeExecutionError) {
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
    entry: Rc<NativeOperationEntry>,
    _allocation: crate::obj::JimAllocationScope,
}

/// Mandatory cleanup attaches the saved original entry only for its actual
/// settlement lifetime. It exposes no fresh receipt or operand authority.
pub(crate) struct NativeSavedCleanupScope {
    interpreter: Interp,
    original: SavedNativeOperationCurrency,
    entry: Rc<NativeOperationEntry>,
    _allocation: crate::obj::JimAllocationScope,
}

impl NativeSavedCleanupScope {
    pub(crate) fn check(&self) -> Result<(), NativeExecutionError> {
        self.original.check(&self.interpreter)
    }
}

impl NativeOperationScope {
    pub(crate) fn enter(interpreter: &Interp) -> Result<Self, NativeExecutionError> {
        let currency = Rc::new(NativeOperationCurrency::issue(interpreter)?);
        currency.ensure_current_or_refuse()?;
        let pool = if interpreter
            .native_invocation_dialect()
            .native_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            Some(
                interpreter
                    .native_jim_object_context()
                    .map_err(|error| {
                        NativeExecutionError::ValueAccessRefusal(
                            error.native_access_refusal().expect("actual Jim context"),
                        )
                    })?
                    .allocation_pool(),
            )
        } else {
            None
        };
        let observed = Rc::clone(&currency);
        let allocation = crate::obj::JimAllocationScope::enter(
            pool,
            Rc::new(move || observed.ensure_current().is_ok()),
        );
        let entry = Rc::new(NativeOperationEntry::Current(currency.clone()));
        interpreter
            .entered_native_operations
            .borrow_mut()
            .push(entry.clone());
        Ok(Self {
            interpreter: interpreter.clone(),
            currency,
            entry,
            _allocation: allocation,
        })
    }

    /// Attach the original saved target for mandatory settlement. Foreign
    /// state is refused before publication. Same-target withdrawal retains the
    /// first cause while cleanup continues; nested work cannot start fresh.
    pub(crate) fn enter_saved_cleanup(
        saved: &SavedNativeOperationCurrency,
        interpreter: &Interp,
    ) -> Result<NativeSavedCleanupScope, NativeExecutionError> {
        saved.ensure_target(interpreter)?;
        let current = saved.check(interpreter);
        let pool = if current.is_ok()
            && interpreter
                .native_invocation_dialect()
                .native_string_protocol()
                == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            interpreter
                .jim_object_context
                .borrow()
                .as_ref()
                .map(|context| context.allocation_pool())
        } else {
            None
        };
        let saved_allocation = saved.clone();
        let target = interpreter.clone();
        let allocation = crate::obj::JimAllocationScope::enter(
            pool,
            Rc::new(move || saved_allocation.check(&target).is_ok()),
        );
        let entry = Rc::new(match &saved.original {
            Ok(currency) => NativeOperationEntry::Current(Rc::new(currency.clone())),
            Err(first) => NativeOperationEntry::RefusedCleanup(first.clone()),
        });
        interpreter
            .entered_native_operations
            .borrow_mut()
            .push(entry.clone());
        Ok(NativeSavedCleanupScope {
            interpreter: interpreter.clone(),
            original: saved.clone(),
            entry,
            _allocation: allocation,
        })
    }

    /// Begin the engine facade's independent evaluation episode. A callback
    /// nested inside an actual operation inherits that entry and cannot reset
    /// its first Host cause; only a genuinely outer engine call starts fresh.
    #[cfg(any(feature = "engine", test))]
    pub(crate) fn enter_engine_evaluation(
        interpreter: &Interp,
    ) -> Result<Self, NativeExecutionError> {
        if interpreter.entered_native_operations.borrow().is_empty() {
            interpreter.reset_native_compilation_admission();
        }
        Self::enter(interpreter)
    }

    pub(crate) fn currency(&self) -> &NativeOperationCurrency {
        &self.currency
    }
}

fn retire_entry(interpreter: &Interp, entry: &Rc<NativeOperationEntry>) {
    let retired = {
        let mut entered = interpreter.entered_native_operations.borrow_mut();
        // Argument/cleanup carriers can retire out of order. Withdraw only
        // this actual entry without removing a still-live sibling.
        let original = entered
            .iter()
            .position(|original| Rc::ptr_eq(original, entry));
        debug_assert!(original.is_some());
        original.map(|original| entered.remove(original))
    };
    drop(retired);
}

impl Drop for NativeOperationScope {
    fn drop(&mut self) {
        retire_entry(&self.interpreter, &self.entry);
    }
}

impl Drop for NativeSavedCleanupScope {
    fn drop(&mut self) {
        retire_entry(&self.interpreter, &self.entry);
    }
}

impl Interp {
    /// Observe actual context currency; equality grants no semantic purpose.
    pub(crate) fn native_operation_context_epoch(&self) -> Option<u64> {
        self.0.native_operation_context_epoch.get()
    }

    /// Retire receipts before an actual execution ingress changes. Exhaustion
    /// is permanent; neither resetting Guest state nor restoring equal context
    /// can revive an older receipt. Command policy has its own guard owner.
    pub(super) fn invalidate_native_operation_context(&self) {
        self.0.native_operation_context_epoch.set(
            self.0
                .native_operation_context_epoch
                .get()
                .and_then(|epoch| epoch.checked_add(1)),
        );
    }

    /// Exhaust the actual checked epoch owner without entering a new operation.
    #[cfg(test)]
    pub(crate) fn exhaust_native_operation_context_for_test(&self) {
        self.0.native_operation_context_epoch.set(Some(u64::MAX));
        self.invalidate_native_operation_context();
    }

    /// Observe the real operation stack for ownership tests without exposing
    /// the mutable stack or granting a new operation.
    #[cfg(test)]
    pub(crate) fn entered_native_operation_count(&self) -> usize {
        self.entered_native_operations.borrow().len()
    }

    /// Check the original entered operation without holding its stack borrow
    /// across a getter or callback. Neutral callers retain their explicit
    /// operation semantics; a previously reached Host cause is still terminal.
    pub(crate) fn check_entered_native_operation(&self) -> Result<(), NativeExecutionError> {
        if let Some(first) = self.native_execution_refusal() {
            return Err(first);
        }
        let entered = self.entered_native_operations.borrow().last().cloned();
        entered.map_or(Ok(()), |entry| entry.check(self))
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

    fn forwarded<T>(
        &self,
        operation: impl FnOnce(
            &dyn tcl_platform::NumericEnvironment,
        ) -> Result<T, tcl_platform::NumericEnvironmentUnavailable>,
    ) -> Result<T, tcl_platform::NumericEnvironmentUnavailable> {
        let check = || match self.currency {
            Some(currency) => currency
                .ensure_current_or_refuse()
                .map_err(|_| tcl_platform::NumericEnvironmentUnavailable::Target),
            None => Ok(()),
        };
        let checked = tcl_platform::CheckedNumericEnvironment::new(self.actual, &check, self.abi);
        operation(&checked)
    }
}

impl tcl_platform::NumericEnvironment for CheckedNumericEnvironment<'_> {
    fn c_integer_abi(
        &self,
    ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable> {
        self.forwarded(|actual| actual.c_integer_abi())
    }
    fn state(
        &self,
    ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable> {
        self.forwarded(|actual| actual.state())
    }
    fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
        self.forwarded(|actual| actual.reset())
    }
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.forwarded(|actual| actual.unsigned_c84(input, offset, long))
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.forwarded(|actual| actual.unsigned(input, offset, base))
    }
    fn unsigned_long(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.forwarded(|actual| actual.unsigned_long(input, offset, base))
    }
    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<tcl_platform::SignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.forwarded(|actual| actual.signed_long(input, base))
    }
    fn double(
        &self,
        input: &[u8],
        reset: bool,
    ) -> Result<tcl_platform::DoubleNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.forwarded(|actual| actual.double(input, reset))
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
    fn original_argument_carriers_retire_independently_without_withdrawing_live_siblings() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software ownership control; leases do not acquire native references.
        use crate::interp::native_arguments::OriginalArguments;

        let interpreter = original();
        let value = obj::Owned::fresh(obj::new_string_bytes(b"retained\0argument"));
        let argv = [value.as_ptr()];
        let earliest = OriginalArguments::enter(&interpreter, &argv).unwrap();
        let middle = OriginalArguments::enter(&interpreter, &argv).unwrap();
        let latest = OriginalArguments::enter(&interpreter, &argv).unwrap();
        assert_eq!(interpreter.entered_native_operation_count(), 3);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);

        drop(middle);
        assert_eq!(interpreter.entered_native_operation_count(), 2);
        earliest.check().unwrap();
        latest.check().unwrap();
        assert_eq!(latest.original(0).unwrap(), value.as_ptr());
        let nested = OriginalArguments::enter(&interpreter, &argv).unwrap();
        drop(earliest);
        assert_eq!(interpreter.entered_native_operation_count(), 2);
        latest.check().unwrap();
        nested.check().unwrap();
        drop(latest);
        assert_eq!(interpreter.entered_native_operation_count(), 1);
        nested.check().unwrap();
        assert_eq!(nested.original(0).unwrap(), value.as_ptr());
        drop(nested);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        assert!(interpreter.native_execution_refusal().is_none());
    }

    #[test]
    fn original_argument_early_retirement_does_not_refresh_a_surviving_context_receipt() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual context ABA remains terminal after a different carrier retires.
        use crate::interp::native_arguments::OriginalArguments;

        let mut interpreter = original();
        let value = obj::Owned::fresh(obj::new_string_bytes(b"original"));
        let argv = [value.as_ptr()];
        let earliest = OriginalArguments::enter(&interpreter, &argv).unwrap();
        let surviving = OriginalArguments::enter(&interpreter, &argv).unwrap();
        drop(earliest);
        assert_eq!(interpreter.entered_native_operation_count(), 1);
        surviving.check().unwrap();
        change_and_restore(&mut interpreter);
        let error = surviving.check().err().expect("original context retired");
        assert_eq!(error.native_execution_refusal(), Some(&stale()));
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert!(OriginalArguments::enter(&interpreter, &argv).is_err());
        assert_eq!(interpreter.entered_native_operation_count(), 1);
        drop(surviving);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert_eq!(interpreter.native_execution_refusal(), Some(stale()));
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
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

    #[test]
    fn legal_guest_command_policy_mutations_keep_original_context_and_scalar_issuer() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software interpreter/issuer purpose; no original provider result is claimed.
        use tcl_runtime_api::guard::GuardDomain;
        let mut interpreter = original();
        let value = obj::Owned::fresh(obj::new_string_bytes(b"17"));
        interpreter
            .bind_native_scalar_object(value.as_ptr())
            .unwrap();
        let references = unsafe { (*value.as_ptr()).ref_count };
        let context_epoch = interpreter.native_operation_context_epoch();
        let entry = NativeOperationScope::enter(&interpreter).unwrap();
        for source in [
            b"proc visible {} {return GUEST_OK}; interp hide {} visible hidden; interp expose {} hidden visible; visible".as_slice(),
            b"interp create child; interp eval child {set x CHILD_OK}; interp delete child; set marker GUEST_OK".as_slice(),
            b"interp create policyChild; interp recursionlimit {} 2048; interp limit policyChild commands -value 100000; interp bgerror {} list; interp delete policyChild; set marker GUEST_OK".as_slice(),
        ] {
            let compiler_epoch = interpreter.guards.borrow().domain_epoch(GuardDomain::Interpreter);
            assert_eq!(interpreter.eval_str(source), Code::Ok, "source {:?}, first cause {:?}, result {:?}", String::from_utf8_lossy(source), interpreter.native_execution_refusal(), interpreter.result_bytes());
            assert_eq!(interpreter.result_bytes(), b"GUEST_OK");
            assert_ne!(interpreter.guards.borrow().domain_epoch(GuardDomain::Interpreter), compiler_epoch);
            assert_eq!(interpreter.native_operation_context_epoch(), context_epoch);
            entry.currency().ensure_current().unwrap();
            let scalar = super::super::native_scalar_context::scalar_access(None, value.as_ptr()).unwrap();
            scalar.ensure_current().unwrap();
            let mut output = -777;
            assert_eq!(unsafe { crate::capi::Tcl_GetIntFromObj(core::ptr::null_mut(), value.as_ptr(), &mut output) }, 0);
            assert_eq!(output, 17);
            assert_eq!(unsafe { (*value.as_ptr()).ref_count }, references);
            assert_eq!(interpreter.result_bytes(), b"GUEST_OK");
            assert!(interpreter.native_execution_refusal().is_none());
        }
        // C's current-interpreter limit restriction is a Guest failure; it
        // cannot retire the original operation or its independent scalar issuer.
        assert_eq!(
            interpreter.eval_str(b"interp limit {} commands -value 100000"),
            Code::Error
        );
        assert_eq!(
            interpreter.result_bytes(),
            b"limits on current interpreter inaccessible"
        );
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(interpreter.native_operation_context_epoch(), context_epoch);
        entry.currency().ensure_current().unwrap();
        let scalar =
            super::super::native_scalar_context::scalar_access(None, value.as_ptr()).unwrap();
        scalar.ensure_current().unwrap();
        let mut output = -777;
        assert_eq!(
            unsafe {
                crate::capi::Tcl_GetIntFromObj(core::ptr::null_mut(), value.as_ptr(), &mut output)
            },
            0
        );
        assert_eq!(output, 17);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, references);
        assert_eq!(
            interpreter.result_bytes(),
            b"limits on current interpreter inaccessible"
        );
        drop(entry);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }

    #[test]
    fn operation_context_aba_and_first_host_do_not_refresh_original_receipts() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Real ingress changes with software receipt assertions; not a Native callback proof.
        for change in 0..3 {
            let mut interpreter = original();
            let value = obj::Owned::fresh(obj::new_string_bytes(b"17"));
            interpreter
                .bind_native_scalar_object(value.as_ptr())
                .unwrap();
            let before = obj::native_object_snapshot(value.as_ptr()).unwrap();
            let context = interpreter.runtime_context();
            let entry = NativeOperationScope::enter(&interpreter).unwrap();
            match change {
                0 => change_and_restore(&mut interpreter),
                1 => {
                    interpreter
                        .set_dialect_profile(crate::environment::profile_for_dialect("tcl8.4"));
                    interpreter
                        .set_dialect_profile(crate::environment::profile_for_dialect("tcl8.6"));
                }
                _ => {
                    let retained_host = interpreter.host();
                    let foreign = original();
                    interpreter.set_host(foreign.host());
                    interpreter.set_host(retained_host);
                }
            }
            assert_eq!(interpreter.runtime_context(), context);
            assert!(
                super::super::native_scalar_context::scalar_access(None, value.as_ptr()).is_err()
            );
            assert_eq!(obj::native_object_snapshot(value.as_ptr()).unwrap(), before);
            assert_eq!(entry.currency().ensure_current_or_refuse(), Err(stale()));
            assert!(NativeOperationCurrency::issue(&interpreter).is_err());
            drop(entry);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
        let mut interpreter = original();
        let entry = NativeOperationScope::enter(&interpreter).unwrap();
        interpreter.refuse_host_command("original context first Host");
        let first = interpreter.native_execution_refusal().unwrap();
        interpreter.set_host(interpreter.host());
        assert_eq!(
            entry.currency().ensure_current_or_refuse(),
            Err(first.clone())
        );
        assert_eq!(interpreter.native_execution_refusal(), Some(first));
        drop(entry);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }

    #[test]
    fn actual_logical_ingress_changes_context_without_granting_native_purpose() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Explicit authored ingress currency; no vendor/physical execution is asserted.
        for provider in 0..4 {
            let mut interpreter = Interp::with_host(crate::interp::default_host());
            interpreter.set_dialect_profile(tcl_dialect::DialectProfile::irules());
            let host = crate::environment::profile_for_dialect("tcl9.0");
            let entry = NativeOperationScope::enter(&interpreter).unwrap();
            let before = interpreter.native_operation_context_epoch();
            let selected = match provider {
                0 => interpreter.set_logical_eval_object_provider(tcl_registry::native_eval_object::LogicalEvalObjectProvider::Tcl84CoreSimulation, host),
                1 => interpreter.set_logical_source_word_provider(tcl_registry::invocation_words::LogicalSourceWordProvider::Tcl84CoreSimulation, host),
                2 => interpreter.set_logical_expression_parse_provider(tcl_registry::invocation_words::LogicalExpressionParseProvider::Tcl84CoreSimulation, host),
                _ => interpreter.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4)),
            };
            assert!(selected);
            assert_ne!(interpreter.native_operation_context_epoch(), before);
            assert_eq!(entry.currency().ensure_current_or_refuse(), Err(stale()));
            assert!(interpreter
                .native_invocation_dialect()
                .native_scalar_getter_protocol()
                .is_none());
            drop(entry);
        }
    }

    #[test]
    fn unchanged_pin_and_exhausted_context_epoch_keep_distinct_admission() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Counter exhaustion is a software owner control, not a Tcl behaviour claim.
        let mut interpreter = original();
        let entry = NativeOperationScope::enter(&interpreter).unwrap();
        let epoch = interpreter.native_operation_context_epoch();
        let context = interpreter.runtime_context();
        interpreter.pin_context(&context).unwrap();
        assert_eq!(interpreter.native_operation_context_epoch(), epoch);
        entry.currency().ensure_current().unwrap();
        drop(entry);
        interpreter
            .native_operation_context_epoch
            .set(Some(u64::MAX));
        let entry = NativeOperationScope::enter(&interpreter).unwrap();
        interpreter.set_host(interpreter.host());
        assert_eq!(interpreter.native_operation_context_epoch(), None);
        assert_eq!(entry.currency().ensure_current_or_refuse(), Err(stale()));
        drop(entry);
        interpreter.reset_native_compilation_admission();
        interpreter.pin_context(&context).unwrap();
        interpreter.set_host(interpreter.host());
        assert_eq!(interpreter.native_operation_context_epoch(), None);
        assert!(NativeOperationCurrency::issue(&interpreter).is_err());
        assert!(interpreter.native_scalar_object_issuer().is_err());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}
