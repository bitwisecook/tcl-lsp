// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual frame adapter controls with software Host/closed VM object boundaries.

use super::*;
use std::{cell::Cell, rc::Rc};
use tcl_platform::{Host, NumericEnvironment};
use tcl_test_support::numeric_environment::{NativeNumericEnvironment, RecordedNumericEnvironment};

struct RecordedHost {
    actual: Rc<dyn Host>,
    numeric: Rc<RecordedNumericEnvironment>,
    acquisitions: Rc<Cell<usize>>,
}
impl Host for RecordedHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
        self.acquisitions.set(self.acquisitions.get() + 1);
        Some(&*self.numeric)
    }
}
fn provided(available: bool) -> (Vm, Rc<RecordedNumericEnvironment>, Rc<Cell<usize>>) {
    let native = NativeNumericEnvironment;
    let numeric = Rc::new(if available {
        RecordedNumericEnvironment::with_c_integer_abi(
            native.state(),
            native.c_integer_abi().unwrap(),
        )
    } else {
        RecordedNumericEnvironment::new(native.state())
    });
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let acquisitions = Rc::new(Cell::new(0));
    vm.set_host(Rc::new(RecordedHost {
        actual: vm.host_rc(),
        numeric: Rc::clone(&numeric),
        acquisitions: Rc::clone(&acquisitions),
    }));
    acquisitions.set(0);
    (vm, numeric, acquisitions)
}

#[test]
fn original_frame_numeric_probe_preserves_cached_level_and_borrowed_reference_role() {

    // This adapter uses a genuine queried Host ABI, not an external Tcl frame/getter execution.
    let (mut vm, numeric, acquisitions) = provided(true);
    let original = Value::int(0);
    let alias = original.clone();
    let snapshot = original.native_object_snapshot();
    let count = original.native_object_reference_count();
    assert_eq!(
        crate::command::runtime_explicit_frame_selection(&mut vm, &original).unwrap(),
        0
    );
    assert!(original.is_same_object(&alias));
    assert_eq!(original.native_object_snapshot(), snapshot);
    assert_eq!(original.native_object_reference_count(), count);
    assert!(original.resident_string_bytes().is_none());
    assert!(acquisitions.get() > 0);
    assert_eq!(numeric.queries(), 0);
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_frame_numeric_unavailable_cannot_become_a_guest_bad_level_or_default() {

    let (mut vm, numeric, _) = provided(false);
    let original = Value::int(0);
    let before = original.native_object_snapshot();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let mut checked = CheckedFrameLevel::selected(&mut vm, &original, &scope).unwrap();
    let error = checked
        .native_frame_probe(NativeScalarGetterKind::Int)
        .unwrap_err();
    assert_eq!(
        error.native_execution_refusal(),
        Some(&tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable
        ))
    );
    drop(checked);
    assert_eq!(original.native_object_snapshot(), before);
    assert!(original.resident_string_bytes().is_none());
    assert_eq!(numeric.queries(), 0);
    let error = crate::command::runtime_explicit_frame_selection(&mut vm, &original).unwrap_err();
    assert_eq!(error.code, tcl_runtime_api::Code::Error);
    assert_eq!(original.native_object_snapshot(), before);
}

#[test]
fn original_frame_numeric_scope_preserves_first_refusal_before_callback_continuation() {

    // The actual Vm callback is a software context mutation, not an extension updater issuer.
    let (mut vm, numeric, acquisitions) = provided(true);
    let original = Value::int(0);
    let before = original.native_object_snapshot();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let continued = Cell::new(false);
    let mut checked = CheckedFrameLevel::selected(&mut vm, &original, &scope).unwrap();
    assert_eq!(
        checked
            .native_frame_probe(NativeScalarGetterKind::Int)
            .unwrap(),
        Ok(0)
    );
    let error = checked
        .numeric
        .with_vm(|vm| {
            let old = vm.host_rc();
            vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
            vm.set_host(old);
            Ok(())
        })
        .unwrap_err();
    let acquisitions_before = acquisitions.get();
    let after = checked
        .native_frame_probe(NativeScalarGetterKind::Wide)
        .unwrap_err();
    assert_eq!(
        after.native_execution_refusal(),
        error.native_execution_refusal()
    );
    assert_eq!(acquisitions.get(), acquisitions_before);
    assert!(checked.current().is_err());
    drop(checked);
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    if scope.check(&mut vm).is_ok() {
        continued.set(true);
    }
    assert!(!continued.get());
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(numeric.queries(), 0);
    assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
}

#[test]
fn original_frame_temporary_int_refuses_foreign_protocol_without_host_query_or_cache_publication() {

    let (mut vm, numeric, acquisitions) = provided(true);
    let original = Value::string("#0");
    let before = original.native_object_snapshot();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let mut checked = CheckedFrameLevel::selected(&mut vm, &original, &scope).unwrap();
    let foreign = tcl_registry::InvocationDialect::of_profile(
        crate::environment::profile_for_dialect("tcl8.4"),
    )
    .native_scalar_getter_protocol()
    .unwrap();
    let error = checked
        .native_frame_temporary_int(foreign, b"0")
        .unwrap_err();
    assert_eq!(
        error.native_access_refusal(),
        Some(tcl_syntax::raw_string::NativeValueAccessRefusal::ScalarNumericInputUnavailable)
    );
    assert_eq!(acquisitions.get(), 0);
    assert_eq!(numeric.queries(), 0);
    assert_eq!(original.native_object_snapshot(), before);
    assert!(checked.native_frame_cache().unwrap().is_none());
}

struct FormatterHost {
    actual: Rc<dyn Host>,
    owner: std::cell::RefCell<Option<Value>>,
    original: crate::value::NativeObjectLifetimeLease,
    retire: bool,
    formats: Cell<usize>,
    numeric_acquisitions: Cell<usize>,
    references: std::cell::RefCell<Vec<usize>>,
    numeric: NativeNumericEnvironment,
}
impl tcl_platform::NativeIntegerFormatter for FormatterHost {
    fn build(&self) -> tcl_platform::NativeIntegerFormatterBuild {
        tcl_platform::NativeIntegerFormatterBuild {
            version: [8, 4, 20, 0],
            sha256: [5; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        _kind: tcl_platform::NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, tcl_platform::NativeIntegerFormatterUnavailable> {
        self.formats.set(self.formats.get() + 1);
        self.references
            .borrow_mut()
            .push(self.original.value().native_object_reference_count());
        if self.retire {
            drop(self.owner.borrow_mut().take());
        }
        Ok(value.to_string().into_bytes())
    }
}
impl Host for FormatterHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        Some(self)
    }
    fn numeric_environment(&self) -> Option<&dyn NumericEnvironment> {
        self.numeric_acquisitions
            .set(self.numeric_acquisitions.get() + 1);
        Some(&self.numeric)
    }
}
#[test]
fn original_frame_string_stage_keeps_formatter_reference_role_and_stops_retired_level() {

    // Closed VM object retirement and software formatter transport do not attest an external Tcl updater.
    for retire in [false, true] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
        let owner = Value::from_native_scalar_cache(
            tcl_syntax::scalar_getter::NativeScalarCache::Number(tcl_syntax::number::Number::Int(
                0,
            )),
            None,
            vm.actual_native_invocation_dialect(),
        )
        .unwrap();
        let original = owner.native_lifetime_lease();
        let count = original.value().native_object_reference_count();
        let host = Rc::new(FormatterHost {
            actual: vm.host_rc(),
            original: original.value().native_lifetime_lease(),
            owner: std::cell::RefCell::new(Some(owner)),
            retire,
            formats: Cell::new(0),
            numeric_acquisitions: Cell::new(0),
            references: std::cell::RefCell::new(Vec::new()),
            numeric: NativeNumericEnvironment,
        });
        vm.set_host(host.clone());
        let reached = crate::command::runtime_explicit_frame_selection(&mut vm, original.value());
        assert_eq!(host.formats.get(), 1);
        assert_eq!(host.references.borrow().as_slice(), &[count]);
        if retire {
            assert_eq!(reached.unwrap_err().code, tcl_runtime_api::Code::Error);
            assert!(!original.value().native_object_is_live());
            assert!(original.value().resident_string_bytes().is_none());
            assert_eq!(host.numeric_acquisitions.get(), 0);
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "retired native object header"
                    )
                )
            );
            assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
        } else {
            assert_eq!(reached.unwrap(), 0);
            assert_eq!(
                original.value().resident_string_bytes().unwrap().as_ref(),
                b"0"
            );
            assert_eq!(original.value().native_object_reference_count(), count);
            assert!(host.numeric_acquisitions.get() > 0);
            assert!(vm.execution_refusal.is_none());
        }
    }
}

#[test]
fn original_frame_single_script_list_checks_header_before_default_selection() {

    // Closed VM List/header controls do not attest an external updater callback.
    for engine in ["tcl9.0", "tcl9.1"] {
        for retire in [false, true] {
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect(engine));
            vm.push_call_frame(None, vec![]);
            let protocol = vm
                .actual_native_invocation_dialect()
                .native_string_protocol()
                .unwrap();
            let mut owner = Some(Value::native_list_constructor(
                vec![Value::string("set"), Value::string("x")],
                protocol,
            ));
            let original = owner.as_ref().unwrap().native_lifetime_lease();
            let before = original.value().native_object_snapshot();
            let references = original.value().native_object_reference_count();
            assert!(original.value().resident_string_bytes().is_none());
            // The public refusal assertion belongs to this same active operation.
            let scope = NativeOperationScope::enter(&mut vm).unwrap();
            if retire {
                drop(owner.take());
            }
            let reached = crate::command::runtime_frame_selection(
                &mut vm,
                tcl_registry::FrameEffectSpec::UPLEVEL,
                std::slice::from_ref(original.value()),
            );
            if retire {
                assert_eq!(reached.unwrap_err().code, tcl_runtime_api::Code::Error);
                let first = vm.execution_refusal.clone().unwrap();
                assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "retired native object header",
                    ),
                ));
                assert!(!original.value().native_object_is_live());
                assert!(original.value().resident_string_bytes().is_none());
                assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
                assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
            } else {
                scope.check(&mut vm).unwrap();
                assert_eq!(reached.unwrap(), (0, 0));
                assert_eq!(original.value().native_object_snapshot(), before);
                assert_eq!(original.value().native_object_reference_count(), references);
                assert!(vm.execution_refusal.is_none());
            }
            vm.pop_call_frame();
        }
    }
}

#[test]
fn original_frame_list_worker_checks_header_after_ok_and_guest_error() {

    // The closure retires the real closed VM owner after the actual cached List probe.
    for engine in ["tcl9.0", "tcl9.1"] {
        for retire in [false, true] {
            for guest_error in [false, true] {
                let mut vm =
                    crate::native_fixture::core(crate::environment::profile_for_dialect(engine));
                let protocol = vm
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                let mut owner = Some(Value::native_list_constructor(
                    vec![Value::string("set"), Value::string("x")],
                    protocol,
                ));
                let original = owner.as_ref().unwrap().native_lifetime_lease();
                let before = original.value().native_object_snapshot();
                let references = original.value().native_object_reference_count();
                let scope = NativeOperationScope::enter(&mut vm).unwrap();
                let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
                let reached = ops.with_vm_for_value(original.value(), |vm| {
                    let length = tcl_syntax::value::ValueOps::list_len(vm, original.value())
                        .map_err(CmdError::from)?;
                    assert_eq!(length, 2);
                    assert_eq!(original.value().native_object_reference_count(), references);
                    if retire {
                        drop(owner.take());
                    }
                    if guest_error {
                        Err(CmdError::with_error_code(
                            "original List worker guest error",
                            "TEST GUEST",
                        ))
                    } else {
                        Ok(length)
                    }
                });
                drop(ops);
                if retire {
                    let first = vm.execution_refusal.clone().unwrap();
                    assert_eq!(first, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "retired native object header",
                        ),
                    ));
                    assert_eq!(
                        reached.unwrap_err().native_execution_refusal(),
                        Some(&first)
                    );
                    assert!(!original.value().native_object_is_live());
                    assert!(original.value().resident_string_bytes().is_none());
                    assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
                } else {
                    if guest_error {
                        let error = reached.unwrap_err();
                        assert_eq!(error.message_bytes(), b"original List worker guest error");
                        assert_eq!(error.error_code_bytes(), Some(b"TEST GUEST".as_slice()));
                        assert!(error.native_execution_refusal().is_none());
                    } else {
                        assert_eq!(reached.unwrap(), 2);
                    }
                    assert_eq!(original.value().native_object_snapshot(), before);
                    assert_eq!(original.value().native_object_reference_count(), references);
                    assert!(vm.execution_refusal.is_none());
                }
            }
        }
    }
}
