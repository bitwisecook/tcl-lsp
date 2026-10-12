// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hidden precision callbacks follow the selected physical variable cell.

use super::{
    native_operation_currency::{
        CheckedNumericEnvironment, NativeOperationCurrency, NativeOperationScope,
    },
    Interp, TraceAccess, VariableTraceGuestOutcome,
};
use crate::{frame::VariableReceiver, obj, vars::TraceHome};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{raw_string::NativeValueAccessRefusal, value::ValueError};

#[cfg(test)]
mod original_stage_tests;

impl Interp {
    pub(super) fn install_native_precision_trace(&mut self) {
        if let Err(first) = self.install_native_precision_trace_checked() {
            self.refuse_native_execution(first);
        }
    }

    pub(super) fn install_native_precision_trace_checked(
        &mut self,
    ) -> Result<(), NativeExecutionError> {
        let operation = NativeOperationScope::enter(self)?;
        operation.currency().ensure_current_or_refuse()?;
        self.native_precision_cell.set(None);
        if self
            .native_invocation_dialect()
            .native_bootstrap_protocol()
            .is_none()
        {
            return Ok(());
        }
        if let Some(name) = self
            .native_invocation_dialect()
            .double_string_policy()
            .and_then(tcl_dialect::DoubleStringPolicy::precision_variable)
        {
            self.install_native_precision_trace_at_checked(name.as_bytes())?;
        }
        operation.currency().ensure_current_or_refuse()
    }

    pub(super) fn install_native_precision_trace_at(&mut self, name: &[u8]) {
        if let Err(first) = self.install_native_precision_trace_at_checked(name) {
            self.refuse_native_execution(first);
        }
    }

    fn install_native_precision_trace_at_checked(
        &mut self,
        name: &[u8],
    ) -> Result<(), NativeExecutionError> {
        let operation = NativeOperationScope::enter(self)?;
        let currency = operation.currency();
        currency.ensure_current_or_refuse()?;
        self.native_precision_cell.set(None);
        if self.pending_delete.get()
            || self
                .native_invocation_dialect()
                .native_bootstrap_protocol()
                .is_none()
        {
            return Ok(());
        }
        let naming = self.require_variable_name_protocol();
        currency.ensure_current_or_refuse()?;
        if naming.is_err() {
            // The naming owner retains its precise operational cause.
            return self.check_entered_native_operation();
        }
        // TclPrecTraceProc reinstalls with GLOBAL_ONLY and the original
        // callback name. A proc-local upvar spelling can relocate this trace.
        let home = self.prepare_trace_registration(name, None, Some(0));
        currency.ensure_current_or_refuse()?;
        // The original callback ignores a Guest trace-registration failure.
        // Operational refusal and reached retirement remain checked separately.
        self.native_precision_cell
            .set(home.ok().and_then(|home| home.binding_id));
        currency.ensure_current_or_refuse()
    }

    pub(super) fn native_precision_trace_at(&self, home: &TraceHome) -> bool {
        self.native_invocation_dialect()
            .double_string_policy()
            .is_some_and(tcl_dialect::DoubleStringPolicy::has_precision_variable)
            && home.binding_id.is_some()
            && home.binding_id == self.native_precision_cell.get()
    }

    fn native_precision_receiver(
        &self,
        home: &TraceHome,
        element: Option<&[u8]>,
    ) -> Option<VariableReceiver> {
        let (receiver, _) = crate::vars::capture_original_namespace_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            home.ns?,
            &home.base,
            element.map(<[u8]>::to_vec),
            false,
        )
        .ok()??;
        (receiver.binding_id() == home.binding_id).then_some(receiver)
    }

    pub(super) fn native_precision_trace(
        &mut self,
        home: &TraceHome,
        access: &TraceAccess,
        op: &[u8],
    ) -> Result<VariableTraceGuestOutcome, NativeExecutionError> {
        let operation = NativeOperationScope::enter(self)?;
        let currency = operation.currency();
        currency.ensure_current_or_refuse()?;
        let outcome = (|| {
            if !self.native_precision_trace_at(home) {
                return Ok(VariableTraceGuestOutcome::Complete);
            }
            if op == b"unset" {
                if access.match_elem.is_none() {
                    self.install_native_precision_trace_at_checked(&access.reported)?;
                }
                return Ok(VariableTraceGuestOutcome::Complete);
            }
            if op == b"array" {
                return Ok(VariableTraceGuestOutcome::Complete);
            }
            // The intrinsic belongs to the containing array. C8.x element
            // aliases suppress that array's callbacks, including this one.
            if access.match_elem.is_some() {
                let cell = self.variable_trace_scope(home, access.match_elem.as_deref());
                if !access.whole_array
                    || self
                        .active_var_trace_scopes
                        .borrow()
                        .contains(&cell.array())
                {
                    return Ok(VariableTraceGuestOutcome::Complete);
                }
            }
            let receiver = self.native_precision_receiver(home, access.match_elem.as_deref());
            currency.ensure_current_or_refuse()?;
            let Some(receiver) = receiver else {
                return Ok(VariableTraceGuestOutcome::Complete);
            };
            let policy = self
                .native_invocation_dialect()
                .double_string_policy()
                .expect("selected hidden precision callback");
            if op == b"read" {
                self.restore_native_precision(&receiver, policy, currency)?;
                return Ok(VariableTraceGuestOutcome::Complete);
            }
            self.write_native_precision(&receiver, policy, currency)
        })();
        // Selected receivers and original objects retire before this check.
        currency.ensure_current_or_refuse()?;
        outcome
    }

    fn write_native_precision(
        &mut self,
        receiver: &VariableReceiver,
        policy: tcl_dialect::DoubleStringPolicy,
        currency: &NativeOperationCurrency,
    ) -> Result<VariableTraceGuestOutcome, NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        // TclPrecTraceProc tests safety before either original getter.
        let error = if self.is_safe() {
            Some(b"can't modify precision from a safe interpreter".as_slice())
        } else if let Some(precision) =
            self.original_native_precision(receiver, policy, currency)?
        {
            currency.ensure_current_or_refuse()?;
            obj::set_double_precision(policy, precision);
            None
        } else {
            Some(b"improper value for precision".as_slice())
        };
        if let Some(error) = error {
            if policy == tcl_dialect::DoubleStringPolicy::Tcl84Precision {
                self.restore_native_precision(receiver, policy, currency)?;
            }
            currency.ensure_current_or_refuse()?;
            self.traces.borrow_mut().pending_err = Some(error.to_vec());
            return Ok(VariableTraceGuestOutcome::GuestError);
        }
        currency.ensure_current_or_refuse()?;
        Ok(VariableTraceGuestOutcome::Complete)
    }

    fn original_native_precision(
        &mut self,
        receiver: &VariableReceiver,
        policy: tcl_dialect::DoubleStringPolicy,
        currency: &NativeOperationCurrency,
    ) -> Result<Option<u8>, NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let pointer = receiver.read().ok().flatten();
        currency.ensure_current_or_refuse()?;
        let original = if let Some(pointer) = pointer {
            self.precision_original_current(currency, pointer)?;
            // The pinned GetVar2/GetVar2Ex workers borrow the cell's native
            // reference. Retain only allocation safety through a callback.
            Some(obj::NativeObjectLifetime::retain(pointer))
        } else {
            None
        };
        let outcome = (|| {
            let host = self.host();
            if policy == tcl_dialect::DoubleStringPolicy::Tcl84Precision {
                // C8.4 reaches String before its decimal strtoul stage. Its
                // original NULL getter result explicitly selects the empty
                // C string; it still reaches the decimal conversion worker.
                let bytes = match &original {
                    Some(original) => {
                        let reached = self.native_object_string_bytes(original.as_ptr());
                        let bytes = self.precision_access_outcome(currency, reached)?;
                        self.precision_original_current(currency, original.as_ptr())?;
                        bytes.unwrap_or_default()
                    }
                    None => std::rc::Rc::<[u8]>::default(),
                };
                let actual = host.numeric_environment();
                currency.ensure_current_or_refuse()?;
                let actual = actual.ok_or_else(|| self.precision_numeric_unavailable())?;
                let abi = std::cell::Cell::new(None);
                let environment = CheckedNumericEnvironment::new(actual, Some(currency), &abi);
                let converted =
                    tcl_cmd_core::native_numeric::c84_precision_from_host(&bytes, &environment);
                return Ok(self
                    .precision_access_outcome(currency, converted)?
                    .flatten());
            }
            let Some(original) = &original else {
                return Ok(None);
            };
            // C8.5/C8.6 reach the error-neutral Int getter on this original;
            // a cached numeric failure does not require String or diagnostics.
            let actual = host.numeric_environment();
            currency.ensure_current_or_refuse()?;
            let abi = std::cell::Cell::new(None);
            let environment =
                actual.map(|actual| CheckedNumericEnvironment::new(actual, Some(currency), &abi));
            let converted = crate::typed_value::native_scalar_probe_with_environment_and_currency(
                original.as_ptr(),
                self.native_invocation_dialect(),
                tcl_syntax::scalar_getter::NativeScalarGetterKind::Int,
                environment
                    .as_ref()
                    .map(|environment| environment as &dyn tcl_platform::NumericEnvironment),
                Some(currency),
            );
            let converted = self.precision_access_outcome(currency, converted)?;
            self.precision_original_current(currency, original.as_ptr())?;
            Ok(match converted {
                Some(Ok(tcl_syntax::scalar_getter::NativeScalarGetterValue::Wide(integer))) => {
                    tcl_syntax::number::double_precision_from_native_integer(policy, integer)
                }
                Some(Err(_)) | None => None,
                // The selected Int worker has no Double/Boolean success.
                Some(Ok(_)) => return Err(self.precision_numeric_unavailable()),
            })
        })();
        drop(original);
        currency.ensure_current_or_refuse()?;
        outcome
    }

    fn precision_original_current(
        &mut self,
        currency: &NativeOperationCurrency,
        original: *mut obj::TclObj,
    ) -> Result<(), NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let reached = obj::check_native_liveness(original);
        match self.precision_access_outcome(currency, reached)? {
            Some(()) => Ok(()),
            None => Err(self.precision_numeric_unavailable()),
        }
    }

    fn precision_access_outcome<T>(
        &mut self,
        currency: &NativeOperationCurrency,
        reached: Result<T, ValueError>,
    ) -> Result<Option<T>, NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        match reached {
            Ok(value) => Ok(Some(value)),
            Err(error) => match error.native_access_refusal() {
                Some(cause) => {
                    let first = NativeExecutionError::ValueAccessRefusal(cause);
                    self.refuse_native_execution(first.clone());
                    Err(first)
                }
                // Tcl_GetVar2/Int(NULL) failing as Guest selects the callback's
                // fixed improper-precision reason, not primitive presentation.
                None => Ok(None),
            },
        }
    }

    fn precision_numeric_unavailable(&mut self) -> NativeExecutionError {
        let first = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::ScalarNumericInputUnavailable,
        );
        self.refuse_native_execution(first.clone());
        first
    }

    fn restore_native_precision(
        &mut self,
        receiver: &VariableReceiver,
        policy: tcl_dialect::DoubleStringPolicy,
        currency: &NativeOperationCurrency,
    ) -> Result<(), NativeExecutionError> {
        currency.ensure_current_or_refuse()?;
        let value = obj::Owned::fresh(obj::new_wide_int_obj(i64::from(obj::double_precision(
            policy,
        ))));
        let reached = receiver.store(value.as_ptr());
        currency.ensure_current_or_refuse()?;
        // The pinned read/safe/invalid callback ignores this Guest setter
        // outcome. A free hook's actual Host cause remains an outer refusal.
        let _ = reached;
        drop(value);
        currency.ensure_current_or_refuse()
    }
}

#[cfg(test)]
mod tests {
    use super::Interp;

    #[test]
    fn original_precision_unset_matches_all_six_native_sources() {
        // naming.variable.original-precision-unset-publication
        // docs/design/analysis/name-resolution-proofs/variable-original-precision-unset-publication.md
        // Exact public results only; these rows do not observe hidden cell
        // identities or establish a pointer-equivalence claim.
        macro_rules! provider {
            ($engine:literal, $version:literal) => {
                (
                    $engine,
                    [
                        include_str!(concat!(
                            "../../../../rust/tcl-registry/tests/data/native_precision_unset319/",
                            $version,
                            "/root/stdout"
                        )),
                        include_str!(concat!(
                            "../../../../rust/tcl-registry/tests/data/native_precision_unset319/",
                            $version,
                            "/linked/stdout"
                        )),
                        include_str!(concat!(
                            "../../../../rust/tcl-registry/tests/data/native_precision_unset319/",
                            $version,
                            "/unrelated/stdout"
                        )),
                    ],
                )
            };
        }
        let sources = [
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_precision_unset319/root.tcl"
            ),
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_precision_unset319/linked.tcl"
            ),
            include_str!(
                "../../../../rust/tcl-registry/tests/data/native_precision_unset319/unrelated.tcl"
            ),
        ];
        for (engine, outputs) in [
            provider!("tcl8.4", "8.4.20"),
            provider!("tcl8.5", "8.5.19"),
            provider!("tcl8.6", "8.6.18"),
            provider!("tcl9.0", "9.0.4"),
            provider!("tcl9.1", "9.1.0"),
            provider!("jim", "jim"),
        ] {
            for (source, stdout) in sources.iter().zip(outputs) {
                let original = stdout
                    .lines()
                    .find_map(|line| line.strip_prefix("ORIGINAL|0|"))
                    .unwrap();
                let expected: Vec<u8> = original
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                crate::counters::reset();
                {
                    let mut interp = Interp::with_native_core(
                        super::super::default_host(),
                        crate::environment::profile_for_dialect(engine),
                        tcl_registry::special_vars::NativeBootstrapInputs::default(),
                    )
                    .unwrap();
                    assert_eq!(
                        interp.eval_str(source.as_bytes()),
                        super::super::Code::Ok,
                        "{engine}: {:?}, admission {:?}, access {:?}",
                        interp.result_bytes(),
                        interp.native_compilation_admission_error(),
                        interp.native_access_refusal()
                    );
                    assert!(!interp.host_refusal_pending(), "{engine}");
                    assert_eq!(interp.result_bytes(), expected, "{engine}");
                }
                assert_eq!(crate::counters::finalize(), 0, "{engine}");
                assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
            }
        }
    }
}
