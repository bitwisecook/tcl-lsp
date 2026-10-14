// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked original command arguments within the actual entered operation.
//!
//! This adapter retains allocation safety and currency, not native references
//! or name/getter authority. String production belongs to the existing selected
//! original-object getter. A driver reads each operand at its actual command
//! stage and keeps this context alive through subsequent effects and cleanup.

use super::{
    native_operation_currency::{NativeOperationCurrency, NativeOperationScope},
    Code, Interp,
};
use crate::obj::{self, NativeObjectLifetime, TclObj};
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueError;

pub(crate) struct OriginalArguments {
    originals: Vec<NativeObjectLifetime>,
    interpreter: Interp,
    scope: NativeOperationScope,
}

/// The actually entered variable-frame birth. This weak observation grants
/// no variable/native role and remains usable after actual header cleanup.
pub(crate) struct OriginalArgumentFrame {
    activation: u64,
    birth: std::rc::Weak<()>,
}
impl OriginalArgumentFrame {
    pub(crate) fn capture(interpreter: &Interp) -> Self {
        let frames = interpreter.frames.borrow();
        Self {
            activation: frames.current_activation(),
            birth: frames.current_activation_owner(),
        }
    }
    pub(crate) fn check(
        &self,
        actual: &Interp,
        arguments: &OriginalArguments,
    ) -> Result<(), CmdError> {
        arguments.check_for(actual)?;
        self.check_frame(actual)
            .map_err(|error| arguments.retain_refusal(error.into()))
    }
    /// Observe the same captured frame inside another already entered owner.
    pub(crate) fn check_frame(&self, actual: &Interp) -> Result<(), ValueError> {
        let same = {
            let frames = actual.frames.borrow();
            self.birth.strong_count() != 0
                && frames.current_activation() == self.activation
                && std::rc::Weak::ptr_eq(&frames.current_activation_owner(), &self.birth)
        };
        if !same {
            return Err(ValueError::CommandProtocolUnavailable(
                "original compiled List frame or temporary",
            ));
        }
        Ok(())
    }
}

impl OriginalArguments {
    /// Borrow the live original argv for a complete command operation. Leases
    /// leave native sharing/counts unchanged and never revive a retired header.
    pub(crate) fn enter(interpreter: &Interp, argv: &[*mut TclObj]) -> Result<Self, CmdError> {
        let scope =
            NativeOperationScope::enter(interpreter).map_err(CmdError::from_execution_refusal)?;
        let originals = argv
            .iter()
            .map(|&original| {
                obj::check_native_liveness(original)?;
                Ok(NativeObjectLifetime::retain(original))
            })
            .collect::<Result<Vec<_>, ValueError>>();
        scope
            .currency()
            .ensure_current_or_refuse()
            .map_err(CmdError::from_execution_refusal)?;
        let result = Self {
            originals: originals.map_err(CmdError::from)?,
            interpreter: interpreter.clone(),
            scope,
        };
        result.check()?;
        Ok(result)
    }

    pub(crate) fn check(&self) -> Result<(), CmdError> {
        self.currency()
            .ensure_current_or_refuse()
            .map_err(CmdError::from_execution_refusal)
    }

    /// Check the captured operation against its actual physical receiver.
    /// This comparison grants no getter authority and cannot transfer an
    /// original receipt to another interpreter with equal context values.
    pub(crate) fn check_for(&self, actual: &Interp) -> Result<(), CmdError> {
        if !Rc::ptr_eq(&self.interpreter.0, &actual.0) {
            let refusal = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original arguments belong to another interpreter",
                ),
            );
            actual.clone().refuse_native_execution(refusal.clone());
            return Err(CmdError::from_execution_refusal(
                actual.native_execution_refusal().unwrap_or(refusal),
            ));
        }
        self.check()
    }

    /// Validate the exact original header at a selected physical worker stage.
    /// Allocation retention and equal spelling cannot substitute another argv.
    pub(crate) fn check_original_for(
        &self,
        actual: &Interp,
        index: usize,
        original: *mut TclObj,
    ) -> Result<(), CmdError> {
        self.check_for(actual)?;
        if self.original(index)? != original {
            let refusal = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original argument header does not match captured argv",
                ),
            );
            actual.clone().refuse_native_execution(refusal.clone());
            return Err(CmdError::from_execution_refusal(
                actual.native_execution_refusal().unwrap_or(refusal),
            ));
        }
        Ok(())
    }

    /// Borrow a captured live original without acquiring a native reference.
    /// Native cache/worker consumers select their own actual purpose.
    pub(crate) fn original(&self, index: usize) -> Result<*mut TclObj, CmdError> {
        self.check()?;
        let original = self.originals.get(index).ok_or_else(|| {
            self.retain_refusal(
                ValueError::CommandProtocolUnavailable("original argument index").into(),
            )
        })?;
        original
            .checked_ptr()
            .map_err(|error| self.retain_refusal(error.into()))
    }

    /// Original neutral currency, independent of getter/frame/name purpose.
    pub(crate) fn currency(&self) -> &NativeOperationCurrency {
        self.scope.currency()
    }

    /// Reach exactly one original String getter. Later operands are untouched.
    /// Counted bytes are independent of any subsequent name-purpose selection.
    pub(crate) fn bytes(&self, index: usize) -> Result<Rc<[u8]>, CmdError> {
        let original = self.original(index)?;
        let result = self.interpreter.native_object_string_bytes(original);
        self.check()?;
        obj::check_native_liveness(original).map_err(|error| self.retain_refusal(error.into()))?;
        result.map_err(|error| self.retain_refusal(error.into()))
    }

    pub(crate) fn finish(&self, code: Code) -> Code {
        if self.check().is_err() {
            Code::Error
        } else {
            code
        }
    }

    pub(crate) fn retain_refusal(&self, error: CmdError) -> CmdError {
        if let Some(first) = error.native_execution_refusal() {
            let mut interpreter = self.interpreter.clone();
            interpreter.refuse_native_execution(first.clone());
        }
        error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::raw_string::NativeValueAccessRefusal;

    thread_local! {
        static CALLS: Cell<usize> = const { Cell::new(0) };
        static CHANGE: Cell<bool> = const { Cell::new(false) };
    }
    fn native() -> Interp {
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
        changed.packages = vec![("original-argument-test".to_owned(), "1.0".to_owned())];
        interpreter.pin_context(&changed).unwrap();
        interpreter.pin_context(&original).unwrap();
    }
    extern "C" fn updater(original: *mut TclObj) {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        let mut interpreter = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        if CHANGE.with(Cell::get) {
            change_and_restore(&mut interpreter);
        } else {
            interpreter.refuse_host_command("first original naming argument updater");
        }
        unsafe { obj::set_native_updater_string_rep(original, b"reached\0name", false) };
    }
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalNamingArgument".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(updater),
        set_from_any_proc: None,
    };

    #[test]
    fn original_argument_entry_keeps_counts_and_does_not_read_later_strings() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Software API control; counted transport grants no external naming equivalence.
        let interpreter = native();
        let first = obj::Owned::fresh(obj::new_string_bytes(b"A\0\xff"));
        let later = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        CALLS.with(|calls| calls.set(0));
        let arguments =
            OriginalArguments::enter(&interpreter, &[first.as_ptr(), later.as_ptr()]).unwrap();
        assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 1);
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(arguments.bytes(0).unwrap().as_ref(), b"A\0\xff");
        assert_eq!(CALLS.with(Cell::get), 0);
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(arguments.finish(Code::Other(7)), Code::Other(7));
        drop(arguments);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }

    #[test]
    fn original_argument_getter_first_host_or_context_restore_stops_later_read_and_effect() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Genuine extension updater effects remain reached; no native C/Jim order claim.
        for change in [false, true] {
            let mut interpreter = native();
            let first = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
            let later = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
            let before = unsafe { (*first.as_ptr()).ref_count };
            interpreter.set_result_bytes(b"PRIOR");
            let prior = interpreter.get_obj_result();
            CALLS.with(|calls| calls.set(0));
            CHANGE.with(|selected| selected.set(change));
            crate::codegen_abi::tcl_runtime_set_current_interp(&mut interpreter);
            let arguments =
                OriginalArguments::enter(&interpreter, &[first.as_ptr(), later.as_ptr()]).unwrap();
            let failure = arguments.bytes(0).err().expect("original updater refuses");
            let retained = interpreter.native_execution_refusal().unwrap();
            assert_eq!(failure.native_execution_refusal(), Some(&retained));
            assert!(arguments.bytes(1).is_err());
            assert_eq!(CALLS.with(Cell::get), 1);
            assert_eq!(obj::bytes_of(first.as_ptr()), b"reached\0name");
            assert!(!obj::has_string_rep(later.as_ptr()));
            assert_eq!(unsafe { (*first.as_ptr()).ref_count }, before);
            assert_eq!(arguments.finish(Code::Ok), Code::Error);
            assert_eq!(interpreter.get_obj_result(), prior);
            drop(arguments);
            crate::codegen_abi::tcl_runtime_set_current_interp(core::ptr::null_mut());
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            assert_eq!(interpreter.native_execution_refusal(), Some(retained));
        }
    }

    #[test]
    fn original_argument_lifetime_does_not_revive_a_retired_native_header() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let interpreter = native();
        let original = obj::Owned::fresh(obj::new_string_bytes(b"retired"));
        let pointer = original.as_ptr();
        let arguments = OriginalArguments::enter(&interpreter, &[pointer]).unwrap();
        drop(original);
        assert!(!obj::allocation_is_live(pointer));
        let failure = arguments
            .bytes(0)
            .err()
            .expect("allocation lease supplies no native role");
        let expected = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("retired native object"),
        );
        assert_eq!(failure.native_execution_refusal(), Some(&expected));
        assert_eq!(interpreter.native_execution_refusal(), Some(expected));
        drop(arguments);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }

    fn changing_worker(interpreter: &mut Interp, _argv: &[*mut TclObj]) -> Code {
        assert!(interpreter.entered_native_operation_count() > 0);
        change_and_restore(interpreter);
        interpreter.set_result_bytes(b"reached original worker");
        Code::Other(7)
    }
    #[test]
    fn original_dispatch_owns_currency_before_worker_and_preserves_reached_guest_state() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let mut interpreter = native();
        interpreter.register_builtin(b"originalChangingWorker", changing_worker);
        let name = obj::Owned::fresh(obj::new_string_bytes(b"originalChangingWorker"));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        assert_eq!(interpreter.dispatch(&[name.as_ptr()]), Code::Error);
        assert_eq!(
            obj::bytes_of(interpreter.get_obj_result()),
            b"reached original worker"
        );
        assert_eq!(
            interpreter.native_execution_refusal(),
            Some(NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                ),
            ))
        );
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(test)]
mod original_receiver_tests {
    use super::*;
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::raw_string::NativeValueAccessRefusal;

    fn actual() -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("tcl8.6"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    #[test]
    fn captured_arguments_accept_only_the_same_physical_receiver_and_original_header() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        // Actual software ownership controls; equal native context is no issuer.
        let interpreter = actual();
        let clone = interpreter.clone();
        let mut other = actual();
        assert_eq!(interpreter.runtime_context(), other.runtime_context());
        let original = obj::Owned::fresh(obj::new_string_bytes(b"end"));
        let alternate = obj::Owned::fresh(obj::new_string_bytes(b"end"));
        let captured = OriginalArguments::enter(&interpreter, &[original.as_ptr()]).unwrap();
        captured.check_for(&clone).unwrap();
        captured
            .check_original_for(&clone, 0, original.as_ptr())
            .unwrap();
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        other.set_result_bytes(b"PRIOR");
        let prior = other.get_obj_result();
        let refused = captured.check_for(&other).unwrap_err();
        assert_eq!(
            refused.native_execution_refusal(),
            other.native_execution_refusal().as_ref()
        );
        assert_eq!(other.get_obj_result(), prior);
        captured.check_for(&interpreter).unwrap();
        let wrong = captured
            .check_original_for(&interpreter, 0, alternate.as_ptr())
            .unwrap_err();
        assert_eq!(
            wrong.native_execution_refusal(),
            Some(&NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original argument header does not match captured argv",
                ),
            ))
        );
        assert_eq!(obj::bytes_of(original.as_ptr()), b"end");
        assert_eq!(obj::bytes_of(alternate.as_ptr()), b"end");
    }

    #[test]
    fn foreign_receiver_mismatch_keeps_its_existing_complete_first_cause() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        let interpreter = actual();
        let mut receiver = actual();
        let captured = OriginalArguments::enter(&interpreter, &[]).unwrap();
        receiver.refuse_host_command("original receiver complete first cause");
        let first = receiver.native_execution_refusal().unwrap();
        let refused = captured.check_for(&receiver).unwrap_err();
        assert_eq!(refused.native_execution_refusal(), Some(&first));
        captured.check_for(&interpreter).unwrap();
    }
}
