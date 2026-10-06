// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Increment's physical receiver survives callbacks without a borrowed guard.

use super::{Code, Interp};
use crate::{
    frame::VarError,
    obj::Owned,
    obj::{self, TclObj},
};
use tcl_registry::native_rmw::{NativeRmwOperation, NativeRmwReadFailure, NativeRmwReadPolicy};

impl Interp {
    /// Execute the selected C read/modify/write protocol, or leave another
    /// native container protocol to its existing adapter.
    pub(crate) fn increment_captured(
        &mut self,
        name: &[u8],
        base: &[u8],
        element: Option<&[u8]>,
        amount: Option<*mut TclObj>,
        legacy_amount: Option<tcl_cmd_core::native_increment::PreparedLegacyIncrementAmount>,
    ) -> Option<Code> {
        let dialect = self.native_invocation_dialect();
        if dialect.tcl_version.is_none() && legacy_amount.is_none() {
            return None;
        }
        // Jim dictionary-sugar receivers use their separate selected container owner.
        if dialect.tcl_version.is_none() && element.is_some() {
            return None;
        }
        let policy = dialect.native_rmw_read_policy(NativeRmwOperation::Increment)?;
        Some(self.increment_receiver(name, base, element, amount, legacy_amount, policy))
    }

    fn increment_receiver(
        &mut self,
        name: &[u8],
        base: &[u8],
        element: Option<&[u8]>,
        amount: Option<*mut TclObj>,
        legacy_amount: Option<tcl_cmd_core::native_increment::PreparedLegacyIncrementAmount>,
        policy: NativeRmwReadPolicy,
    ) -> Code {
        let captured = crate::vars::capture_variable_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            self.current_ns.get(),
            base,
            element,
        );
        let receiver = match captured {
            Ok(receiver) => receiver,
            Err(error) => return self.increment_receiver_error(name, base, error, policy),
        };
        let home = self.trace_identity(base);
        self.increment_retained_receiver(
            (name, base, element),
            (amount, legacy_amount),
            policy,
            (receiver, home),
        )
    }

    /// Execute an admitted indexed instruction against its physical compiled
    /// cell. Reporting bytes never repeat receiver lookup.
    pub(super) fn increment_original_compiled_target(
        &mut self,
        root: &[u8],
        element: Option<&[u8]>,
        slot: Option<usize>,
        originals: (Option<*mut TclObj>, Option<*mut TclObj>, bool),
        amount: *mut TclObj,
    ) -> Code {
        let dialect = self.native_invocation_dialect();
        let legacy = if let Some(protocol) = dialect.native_legacy_increment_protocol() {
            match tcl_cmd_core::native_increment::prepare_legacy_amount(
                &mut crate::value_ops::RuntimeLegacyIncrementAmountOps(self),
                protocol.recipe(),
                &amount,
            ) {
                Ok(amount) => Some(amount),
                Err(error) => return self.report_cmd_error(error),
            }
        } else {
            None
        };
        if dialect.native_rmw_amount_validation(NativeRmwOperation::Increment)
            == Some(tcl_registry::native_rmw::NativeRmwAmountValidation::BeforeRead)
            && legacy.is_none()
        {
            match tcl_syntax::value::ValueOps::int_add(self, None, &amount) {
                Ok(value) => drop(Owned::fresh(value)),
                Err(error) => return crate::value_ops::integer_error(self, error),
            }
        }
        let Some(policy) = dialect.native_rmw_read_policy(NativeRmwOperation::Increment) else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native compiled increment policy",
                )
                .into(),
            );
        };
        let (original_name, original_element, combined) = originals;
        let captured = if let Some(slot) = slot {
            let element = match original_element
                .map(|value| {
                    tcl_syntax::value::ValueOps::native_string_bytes(self, &value)
                        .map(|bytes| bytes.to_vec())
                })
                .transpose()
            {
                Ok(element) => element,
                Err(error) => return self.report_cmd_error(error.into()),
            };
            let capture_result = crate::vars::capture_original_indexed_receiver(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                slot,
                element.clone(),
                true,
            );
            match capture_result {
                Ok(Some((receiver, home))) => {
                    super::native_variable_names::OriginalCVariableCapture {
                        receiver,
                        home,
                        root: root.to_vec(),
                        element,
                    }
                }
                Ok(None) => return self.no_such_variable(root, element.as_deref()),
                Err(error) => return self.increment_receiver_error(root, root, error, policy),
            }
        } else {
            let Some(original_name) = original_name else {
                return self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "compiled increment original stack name",
                    )
                    .into(),
                );
            };
            let purpose = tcl_syntax::native_variable_name::NativeVariableNameLookupPurpose::Write;
            let captured = if combined {
                self.capture_original_c_variable_report(original_name, purpose)
            } else {
                self.capture_original_c_parts_report(original_name, original_element, purpose)
            };
            match captured {
                Ok(Some(captured)) => captured,
                Ok(None) => return self.no_such_variable(root, element),
                Err(code) => return code,
            }
        };
        if dialect
            .tcl_version
            .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_5)
        {
            captured
                .receiver
                .retain_original_element_key(original_element);
        }
        self.increment_retained_receiver(
            (&captured.root, &captured.root, captured.element.as_deref()),
            (Some(amount), legacy),
            policy,
            (captured.receiver, captured.home),
        )
    }

    fn increment_retained_receiver(
        &mut self,
        target: (&[u8], &[u8], Option<&[u8]>),
        amounts: (
            Option<*mut TclObj>,
            Option<tcl_cmd_core::native_increment::PreparedLegacyIncrementAmount>,
        ),
        policy: NativeRmwReadPolicy,
        captured: (crate::frame::VariableReceiver, crate::vars::TraceHome),
    ) -> Code {
        let (name, base, element) = target;
        let (amount, legacy_amount) = amounts;
        let (receiver, home) = captured;
        let access = self.trace_access(name, base, element, &home, false);
        let read_error = self.fire_var_trace_resolved(&home, &access, b"read");
        if self.host_refusal_pending() {
            return Code::Error;
        }
        let current = if read_error {
            let reason = self
                .traces
                .borrow_mut()
                .pending_err
                .take()
                .unwrap_or_default();
            if policy == NativeRmwReadPolicy::RequireContents {
                return self.var_trace_error(name, b"read", &reason);
            }
            None
        } else {
            match receiver.read_initial() {
                Ok(value) => value,
                // Modern Tcl treats a failed fetch as zero, then reports the
                // actual captured receiver's failure at the store boundary.
                Err(_) if policy == NativeRmwReadPolicy::InitialiseZero => None,
                Err(error) => return self.increment_receiver_error(name, base, error, policy),
            }
        };
        if current.is_none() && policy == NativeRmwReadPolicy::RequireContents {
            let failure = if receiver.is_element() {
                NativeRmwReadFailure::MissingElement
            } else {
                NativeRmwReadFailure::MissingVariable
            };
            return self.increment_read_failure(name, base, failure, policy);
        }
        let default_amount = (amount.is_none() && legacy_amount.is_none())
            .then(|| Owned::fresh(obj::new_wide_int_obj(1)));
        let sum = if let Some(amount) = legacy_amount {
            let objects = match crate::value_ops::RuntimeLegacyIncrementObjects::selected(self) {
                Ok(objects) => objects,
                Err(error) => return self.report_cmd_error(error),
            };
            let original = current.map(crate::value_ops::RuntimeAppendValue::borrowed);
            match tcl_cmd_core::native_increment::increment_legacy(
                &objects,
                original.as_ref(),
                amount,
            ) {
                Ok(value) => value,
                Err(error) => return self.report_cmd_error(error),
            }
        } else if self
            .native_invocation_dialect()
            .native_scalar_getter_protocol()
            .is_some_and(|protocol| protocol.supports_number_getter())
        {
            let objects = match crate::value_ops::RuntimeIncrementObjects::selected(
                self.native_invocation_dialect(),
            ) {
                Ok(objects) => objects,
                Err(error) => return self.report_cmd_error(error),
            };
            let original = current.map(crate::value_ops::RuntimeAppendValue::borrowed);
            let amount = amount.unwrap_or_else(|| default_amount.as_ref().unwrap().as_ptr());
            let amount = crate::value_ops::RuntimeAppendValue::borrowed(amount);
            match tcl_cmd_core::native_increment::increment(&objects, original.as_ref(), &amount) {
                Ok(value) => value,
                Err(error) => return self.report_cmd_error(error),
            }
        } else {
            let amount = amount.unwrap_or_else(|| default_amount.as_ref().unwrap().as_ptr());
            match tcl_syntax::value::ValueOps::int_add(self, current.as_ref(), &amount) {
                Ok(value) => crate::value_ops::RuntimeAppendValue::retain(value),
                Err(error) => return crate::value_ops::integer_error(self, error),
            }
        };
        if let Err(error) = receiver.store(sum.as_ptr()) {
            return self.increment_receiver_error(name, base, error, policy);
        }
        drop(sum);
        if self.fire_var_trace_resolved(&home, &access, b"write") {
            return crate::builtins::var_error(self, name, VarError::TraceError);
        }
        match receiver.read() {
            Ok(Some(value)) => self.set_result(value),
            Ok(None) | Err(_) => self.set_result_bytes(b""),
        }
        Code::Ok
    }

    fn increment_receiver_error(
        &mut self,
        name: &[u8],
        base: &[u8],
        error: VarError,
        policy: NativeRmwReadPolicy,
    ) -> Code {
        let failure = match error {
            VarError::DeletedArray => NativeRmwReadFailure::RetiredArray,
            VarError::DeletedNamespace => NativeRmwReadFailure::RetiredNamespace,
            VarError::IsArray => NativeRmwReadFailure::ArrayValue,
            VarError::IsScalar => NativeRmwReadFailure::NonArrayElement,
            VarError::NoSuchNamespace => NativeRmwReadFailure::MissingNamespace,
            _ => return crate::builtins::var_error(self, name, error),
        };
        self.increment_read_failure(name, base, failure, policy)
    }

    fn increment_read_failure(
        &mut self,
        name: &[u8],
        base: &[u8],
        failure: NativeRmwReadFailure,
        policy: NativeRmwReadPolicy,
    ) -> Code {
        let message = policy.failure_message_bytes(name, failure);
        match self
            .native_invocation_dialect()
            .native_rmw_failure_error_code(name, base, failure)
        {
            Some(code) => {
                let code = rmw_error_code(self, code);
                self.error_with_code(&message, &code)
            }
            None => self.set_error(&message),
        }
    }
}

fn rmw_error_code(
    interp: &Interp,
    code: tcl_registry::native_rmw::NativeRmwFailureErrorCode,
) -> Vec<u8> {
    use tcl_registry::native_rmw::NativeRmwFailureErrorCode as Code;
    match code {
        Code::None => b"NONE".to_vec(),
        Code::WriteVariable => b"TCL WRITE VARNAME".to_vec(),
        Code::LookupVariable(name) => {
            let words = [b"TCL".as_slice(), b"LOOKUP", b"VARNAME", name.as_slice()];
            let objects: Vec<_> = words
                .iter()
                .map(|word| Owned::fresh(super::new_string(word)))
                .collect();
            let pointers: Vec<_> = objects.iter().map(Owned::as_ptr).collect();
            let list = Owned::fresh(interp.new_list_object(&pointers));
            super::obj_bytes(list.as_ptr())
        }
    }
}
