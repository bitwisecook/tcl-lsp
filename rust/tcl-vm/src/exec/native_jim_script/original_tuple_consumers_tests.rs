// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Jim tuple consumer owners; software lifetime controls are independent of Native chronology.

use crate::interp::{
    native_name_consumers::OriginalNameConsumerScope,
    native_operation_currency::NativeOperationScope,
};
use crate::{Code, Value, Vm};
use std::{cell::Cell, rc::Rc};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

struct ObservedHost {
    actual: Rc<dyn tcl_platform::Host>,
    numeric_queries: Cell<usize>,
    formatter_queries: Cell<usize>,
}
impl tcl_platform::Host for ObservedHost {
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
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.actual.filesystem()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.numeric_queries.set(self.numeric_queries.get() + 1);
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.formatter_queries.set(self.formatter_queries.get() + 1);
        self.actual.native_integer_formatter()
    }
}
fn provided() -> (Vm, Rc<ObservedHost>) {
    let host = Rc::new(ObservedHost {
        actual: Rc::new(crate::host_native::NativeHost::new()),
        numeric_queries: Cell::new(0),
        formatter_queries: Cell::new(0),
    });
    let vm = crate::native_fixture::core_with_host(
        crate::environment::profile_for_dialect("jim"),
        host.clone(),
    );
    host.numeric_queries.set(0);
    host.formatter_queries.set(0);
    (vm, host)
}
fn no_scalar_query(host: &ObservedHost) {
    assert_eq!(host.numeric_queries.get(), 0);
    assert_eq!(host.formatter_queries.get(), 0);
}
fn source(vm: &mut Vm, bytes: &[u8]) -> crate::Completion<Value> {
    vm.eval_original_script_value(
        &Value::new_native_string_bytes(bytes),
        tcl_registry::native_eval_object::EvalObjectPurpose::Eval,
        None,
    )
    .unwrap()
}
fn original_result(vm: &mut Vm) -> crate::NativeObjectLifetimeLease {
    vm.adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
        .unwrap()
}
fn result_unchanged(vm: &Vm, original: &crate::NativeObjectLifetimeLease) {
    assert!(
        vm.with_native_interp_result(|result| result.is_same_object(original.value()))
            .unwrap()
    );
}
fn header_refusal() -> NativeExecutionError {
    NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::CommandProtocolUnavailable(
        "retired native object header",
    ))
}

#[test]
fn original_tuple_children_keep_actual_owned_children_without_an_extra_native_hold() {
    // Actual child views protect allocation only; these software counts assert no additional Native hold.
    let (mut vm, host) = provided();
    let operation = NativeOperationScope::enter(&mut vm).unwrap();
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let original = Value::new_native_string_bytes(b"d(KEY)".as_slice());
    let (name, key) = vm
        .original_jim_tuple_children_in_context(&original, &names)
        .unwrap();
    assert_eq!(original.native_object_reference_count(), 1);
    assert_eq!(name.value().native_object_reference_count(), 1);
    assert_eq!(key.value().native_object_reference_count(), 1);
    assert_eq!(name.value().resident_string_bytes().unwrap().as_ref(), b"d");
    assert_eq!(
        key.value().resident_string_bytes().unwrap().as_ref(),
        b"KEY"
    );
    let (name_again, key_again) = vm
        .original_jim_tuple_children_in_context(&original, &names)
        .unwrap();
    assert!(name.value().is_same_object(name_again.value()));
    assert!(key.value().is_same_object(key_again.value()));
    assert_eq!(name.value().native_object_reference_count(), 1);
    assert_eq!(key.value().native_object_reference_count(), 1);
    no_scalar_query(&host);
    drop(name_again);
    drop(key_again);
    drop(original);
    assert!(!name.value().native_object_is_live());
    assert!(!key.value().native_object_is_live());
    assert_eq!(name.value().native_object_reference_count(), 0);
    assert_eq!(key.value().native_object_reference_count(), 0);
    drop(name);
    drop(key);
    names.finish(&mut vm).unwrap();
    operation.check_native(&mut vm).unwrap();
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_tuple_consumers_execute_the_actual_script_and_blocking_child_route() {
    // Both existing consumers receive genuine Jim source objects, not a compiler or recipe donor.
    let (mut vm, host) = provided();
    assert_eq!(
        source(&mut vm, b"set k KEY; set d [dict create KEY VALUE]").code,
        Code::Ok
    );
    let operation = NativeOperationScope::enter(&mut vm).unwrap();
    let original = Value::new_native_string_bytes(b"d(KEY)".as_slice());
    let result = vm
        .expand_native_jim_dictionary_substitution(&original)
        .unwrap();
    assert_eq!(result.resident_string_bytes().unwrap().as_ref(), b"VALUE");
    assert_eq!(original.native_object_reference_count(), 1);
    original
        .with_native_jim_dictionary_substitution(|name, key| {
            assert_eq!(name.native_object_reference_count(), 1);
            assert_eq!(key.native_object_reference_count(), 1);
        })
        .unwrap();
    drop(result);
    let script = source(&mut vm, b"set picked $d(KEY)");
    assert_eq!(script.code, Code::Ok);
    assert_eq!(
        script.result.resident_string_bytes().unwrap().as_ref(),
        b"VALUE"
    );
    no_scalar_query(&host);
    operation.check_native(&mut vm).unwrap();
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_tuple_consumer_refuses_a_retired_root_before_adoption_and_preserves_result() {
    let (mut vm, host) = provided();
    let operation = NativeOperationScope::enter(&mut vm).unwrap();
    let result = original_result(&mut vm);
    let original = Value::new_native_string_bytes(b"d(KEY)".as_slice());
    let lease = original.native_lifetime_lease();
    drop(original);
    let before = lease.value().native_object_snapshot();
    let reached = vm
        .expand_native_jim_dictionary_substitution(lease.value())
        .unwrap_err();
    let first = header_refusal();
    assert_eq!(reached.code, Code::Error);
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    assert_eq!(operation.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(lease.value().native_object_snapshot(), before);
    assert!(
        lease
            .value()
            .with_native_jim_dictionary_substitution(|_, _| ())
            .is_none()
    );
    no_scalar_query(&host);
    result_unchanged(&vm, &result);
}

#[test]
fn original_tuple_consumer_refuses_a_foreign_original_context_before_child_preparation() {
    // A live same-profile foreign context is not the addressed interpreter's original tuple issuer.
    let (mut vm, host) = provided();
    let (foreign, _) = provided();
    let foreign_context = foreign.native_jim_object_context().unwrap();
    let original = Value::new_native_string_bytes(b"d(KEY)".as_slice());
    original.bind_native_jim_context(&foreign_context).unwrap();
    let before = original.native_object_snapshot();
    let operation = NativeOperationScope::enter(&mut vm).unwrap();
    let result = original_result(&mut vm);
    let reached = vm
        .expand_native_jim_dictionary_substitution(&original)
        .unwrap_err();
    let first = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable("Jim original object interpreter"),
    );
    assert_eq!(reached.code, Code::Error);
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    assert_eq!(operation.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(original.native_object_snapshot(), before);
    assert!(
        original
            .with_native_jim_dictionary_substitution(|_, _| ())
            .is_none()
    );
    no_scalar_query(&host);
    result_unchanged(&vm, &result);
}

#[test]
fn original_tuple_consumer_preserves_prior_host_and_restored_context_refusals() {
    // The authentic outer entry stays live through the later boundary; no ended or parked receipt is reissued.
    for change_context in [false, true] {
        let (mut vm, host) = provided();
        let operation = NativeOperationScope::enter(&mut vm).unwrap();
        let result = original_result(&mut vm);
        let original = Value::new_native_string_bytes(b"d(KEY)".as_slice());
        let before = original.native_object_snapshot();
        let expected_prior = if change_context {
            let actual = vm.host_rc();
            vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
            vm.set_host(actual);
            None
        } else {
            let failure = vm.refuse_host_command("original tuple prior Host".into());
            assert_eq!(failure.code, Code::Error);
            Some(vm.execution_refusal.clone().unwrap())
        };
        let reached = vm
            .expand_native_jim_dictionary_substitution(&original)
            .unwrap_err();
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(reached.code, Code::Error);
        if change_context {
            assert_eq!(
                first,
                NativeExecutionError::ValueAccessRefusal(
                    NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "entered VM original execution context"
                    )
                )
            );
        } else {
            assert_eq!(first, expected_prior.unwrap());
            assert!(matches!(first, NativeExecutionError::HostCommandRefusal(_)));
        }
        assert_eq!(operation.check_native(&mut vm).unwrap_err(), first);
        assert_eq!(original.native_object_snapshot(), before);
        assert!(
            original
                .with_native_jim_dictionary_substitution(|_, _| ())
                .is_none()
        );
        no_scalar_query(&host);
        result_unchanged(&vm, &result);
    }
}

#[test]
fn original_parked_tuple_context_resumes_same_currency_without_an_entered_grant() {
    let (mut vm, host) = provided();
    let original = Value::new_native_string_bytes(b"d(KEY)".as_slice());
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let saved = names.operation_scope().currency().clone();
    let parked = names.park();
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    saved.check_native(&mut vm).unwrap();
    let names = parked.resume(&mut vm).unwrap();
    let entered = NativeOperationScope::current_entered(&mut vm)
        .unwrap()
        .unwrap();
    names.current_for_value(&mut vm, &original).unwrap();
    let (name, key) = vm
        .original_jim_tuple_children_in_context(&original, &names)
        .unwrap();
    assert_eq!(original.native_object_reference_count(), 1);
    assert_eq!(name.value().native_object_reference_count(), 1);
    assert_eq!(key.value().native_object_reference_count(), 1);
    drop((name, key));
    names.finish(&mut vm).unwrap();
    entered.check_native(&mut vm).unwrap();
    drop(entered);
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    saved.check_native(&mut vm).unwrap();
    no_scalar_query(&host);
}

#[test]
fn original_parked_tuple_context_refuses_foreign_target_before_entry_or_result_publication() {
    let (mut vm, host) = provided();
    let (mut foreign, foreign_host) = provided();
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let saved = names.operation_scope().currency().clone();
    let parked = names.park();
    let before = original_result(&mut foreign);
    let first = parked.resume(&mut foreign).err().unwrap();
    let expected = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable(
            "entered VM original execution context",
        ),
    );
    assert_eq!(first.native_execution_refusal(), Some(&expected));
    assert_eq!(foreign.execution_refusal.as_ref(), Some(&expected));
    assert_eq!(
        NativeOperationScope::enter_native(&mut foreign)
            .err()
            .unwrap(),
        expected
    );
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    saved.check_native(&mut vm).unwrap();
    result_unchanged(&foreign, &before);
    no_scalar_query(&host);
    no_scalar_query(&foreign_host);
}

#[test]
fn original_parked_tuple_context_refuses_restored_host_and_retires_same_target_state() {
    let (mut vm, host) = provided();
    let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let saved = names.operation_scope().currency().clone();
    let parked = names.park();
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    let before = original_result(&mut vm);
    let actual = vm.host_rc();
    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
    vm.set_host(actual);
    let first = parked.resume(&mut vm).err().unwrap();
    let expected = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable(
            "entered VM original execution context",
        ),
    );
    assert_eq!(first.native_execution_refusal(), Some(&expected));
    assert_eq!(vm.execution_refusal.as_ref(), Some(&expected));
    assert_eq!(saved.check_native(&mut vm).unwrap_err(), expected);
    assert_eq!(
        NativeOperationScope::enter_native(&mut vm).err().unwrap(),
        expected
    );
    result_unchanged(&vm, &before);
    no_scalar_query(&host);
}

#[test]
fn original_pending_dictionary_accept_uses_saved_context_and_stops_withdrawn_currency() {
    // This is the actual Subst token/Dictionary continuation, without a simulated extension callback.
    for withdraw in [false, true] {
        let (mut vm, host) = provided();
        assert_eq!(
            source(&mut vm, b"set d [dict create KEY VALUE]").code,
            Code::Ok
        );
        let script = Value::new_native_string_bytes(b"$d(KEY)".as_slice());
        let entry = crate::native_jim_script::NativeJimScriptEntry::borrowed(
            script.native_lifetime_lease(),
            Some(0),
        );
        let mut state =
            crate::native_jim_script::NativeJimScriptState::new(entry, &mut vm).unwrap();
        let crate::native_jim_script::ScriptStep::Token { index } = state.next(&mut vm).unwrap()
        else {
            panic!("actual indexed Subst token");
        };
        let tuple = state.original_token(index).1.native_lifetime_lease();
        let names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let saved = names.operation_scope().currency().clone();
        let (name, key) = vm
            .original_jim_tuple_children_in_context(tuple.value(), &names)
            .unwrap();
        assert_eq!(
            key.value().resident_string_bytes().unwrap().as_ref(),
            b"KEY"
        );
        let root_count = tuple.value().native_object_reference_count();
        state.begin_dictionary(tuple.value().native_lifetime_lease(), name, names.park());
        assert!(
            NativeOperationScope::current_entered(&mut vm)
                .unwrap()
                .is_none()
        );
        let before = original_result(&mut vm);
        if withdraw {
            let actual = vm.host_rc();
            vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
            vm.set_host(actual);
        }
        state.accept(
            &mut vm,
            crate::interp::ok(Value::new_native_string_bytes(b"KEY".as_slice())),
        );
        let crate::native_jim_script::ScriptStep::Complete(completion) =
            state.next(&mut vm).unwrap()
        else {
            panic!("actual Dictionary completion");
        };
        if withdraw {
            let first = NativeExecutionError::ValueAccessRefusal(
                NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "entered VM original execution context",
                ),
            );
            assert_eq!(completion.code, Code::Error);
            assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
            assert_eq!(saved.check_native(&mut vm).unwrap_err(), first);
            assert_eq!(
                NativeOperationScope::enter_native(&mut vm).err().unwrap(),
                first
            );
            result_unchanged(&vm, &before);
        } else {
            assert_eq!(completion.code, Code::Ok);
            assert_eq!(
                completion.result.resident_string_bytes().unwrap().as_ref(),
                b"VALUE"
            );
            saved.check_native(&mut vm).unwrap();
            assert!(vm.execution_refusal.is_none());
            assert!(
                NativeOperationScope::current_entered(&mut vm)
                    .unwrap()
                    .is_none()
            );
        }
        assert_eq!(tuple.value().native_object_reference_count(), root_count);
        assert_eq!(key.value().native_object_reference_count(), 1);
        no_scalar_query(&host);
    }
}

#[test]
fn original_dictionary_cursor_borrows_members_without_native_holds_and_detects_changed_epoch() {
    let root = Value::from_native_dictionary_cache(vec![(
        Value::new_native_string_bytes(b"KEY".as_slice()),
        Value::new_native_string_bytes(b"BEFORE".as_slice()),
    )]);
    let cursor = root.original_dictionary_cursor().unwrap();
    let (key, value) = cursor.member(&root, 0).unwrap().unwrap();
    assert_eq!(root.native_object_reference_count(), 1);
    assert_eq!(key.value().native_object_reference_count(), 1);
    assert_eq!(value.value().native_object_reference_count(), 1);
    assert_eq!(
        key.value().resident_string_bytes().unwrap().as_ref(),
        b"KEY"
    );
    assert_eq!(
        value.value().resident_string_bytes().unwrap().as_ref(),
        b"BEFORE"
    );
    assert!(cursor.member(&root, 1).unwrap().is_none());
    let updated = root
        .native_dictionary_set_member(
            Value::new_native_string_bytes(b"KEY".as_slice()),
            Value::new_native_string_bytes(b"AFTER".as_slice()),
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        )
        .unwrap();
    assert!(updated.is_same_object(&root));
    drop(updated);
    assert_eq!(root.native_object_reference_count(), 1);
    assert_eq!(
        cursor.check(&root).unwrap_err(),
        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original Dictionary lookup table"
        )
    );
    assert!(!value.value().native_object_is_live());
    assert_eq!(value.value().native_object_reference_count(), 0);
    let current = root.original_dictionary_cursor().unwrap();
    let (_, now) = current.member(&root, 0).unwrap().unwrap();
    assert_eq!(
        now.value().resident_string_bytes().unwrap().as_ref(),
        b"AFTER"
    );
    assert_eq!(now.value().native_object_reference_count(), 1);
    let original_root = root.native_lifetime_lease();
    drop(root);
    assert_eq!(
        current.check(original_root.value()).unwrap_err(),
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("retired native object header")
    );
    assert!(!key.value().native_object_is_live());
    assert!(!now.value().native_object_is_live());
}
