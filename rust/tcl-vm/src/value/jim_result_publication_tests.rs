// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
#[cfg(debug_assertions)]
use crate::interp::native_operation_currency::NativeOperationScope;

struct ObserverCleanup(usize);
impl Drop for ObserverCleanup {
    fn drop(&mut self) {
        NATIVE_RETIREMENT_OBSERVERS.with(|observers| {
            observers.borrow_mut().remove(&self.0);
        });
    }
}

fn observe(original: &Value, callback: impl FnOnce(&Value) + 'static) -> ObserverCleanup {
    let identity = Rc::as_ptr(&original.0) as usize;
    NATIVE_RETIREMENT_OBSERVERS.with(|observers| {
        assert!(
            observers
                .borrow_mut()
                .insert(identity, Box::new(callback))
                .is_none()
        );
    });
    ObserverCleanup(identity)
}

#[cfg(debug_assertions)]
#[test]
fn original_jim_result_publication_retires_old_slot_before_mandatory_publication() {
    // VM software retirement observer; the original Jim primitive window
    // is independently captured in original-info-body.c and its stdout.
    for mode in 0..=2 {
        let vm = Rc::new(RefCell::new(crate::native_fixture::core(
            crate::environment::profile_for_dialect("jim"),
        )));
        let context = vm.borrow().native_jim_object_context().unwrap();
        let operation = NativeOperationScope::enter(&mut vm.borrow_mut()).unwrap();
        let old = Value::new_native_string_bytes(b"PRIOR".as_slice());
        let old_view = old.native_lifetime_lease();
        context.publish_result(&old);
        drop(old);
        let body = Rc::new(RefCell::new(Some(Value::native_list_constructor(
            vec![Value::new_native_string_bytes(b"return".as_slice())],
            NativeStringProtocol::Jim084,
        ))));
        let incoming = body.borrow().as_ref().unwrap().native_lifetime_lease();
        let reached = Rc::new(Cell::new(0));
        let _observer = observe(old_view.value(), {
            let context = Rc::clone(&context);
            let body = Rc::clone(&body);
            let incoming = incoming.clone();
            let reached = Rc::clone(&reached);
            let vm = Rc::clone(&vm);
            move |retiring| {
                reached.set(reached.get() + 1);
                assert_eq!(retiring.native_object_reference_count(), 0);
                assert!(retiring.check_native_header().is_err());
                assert!(context.with_result_object(|current| current.is_same_object(retiring)));
                assert_eq!(incoming.value().native_object_reference_count(), 2);
                assert!(incoming.value().resident_string_bytes().is_none());
                drop(body.borrow_mut().take());
                assert_eq!(incoming.value().native_object_reference_count(), 1);
                if mode == 1 {
                    let refusal = vm
                        .borrow_mut()
                        .refuse_host_command("result retirement observer".into());
                    assert_eq!(refusal.code, tcl_runtime_api::Code::Error);
                } else if mode == 2 {
                    vm.borrow_mut()
                        .set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
                }
            }
        });
        context.publish_result(incoming.value());
        assert_eq!(reached.get(), 1);
        assert!(context.with_result_object(|current| current.is_same_object(incoming.value())));
        assert_eq!(incoming.value().native_object_reference_count(), 1);
        assert!(!old_view.value().native_object_is_live());
        if mode == 0 {
            operation.check(&mut vm.borrow_mut()).unwrap();
        } else {
            assert!(operation.check(&mut vm.borrow_mut()).is_err());
            let first = vm.borrow().execution_refusal.clone().unwrap();
            assert!(operation.check(&mut vm.borrow_mut()).is_err());
            assert_eq!(vm.borrow().execution_refusal.as_ref(), Some(&first));
        }
    }
}

#[test]
fn original_jim_borrowed_result_adoption_adds_only_the_actual_result_reference() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    let context = vm.native_jim_object_context().unwrap();
    let old = Value::new_native_string_bytes(b"PRIOR".as_slice());
    let old_view = old.native_lifetime_lease();
    context.publish_result(&old);
    drop(old);
    let body = Value::native_list_constructor(Vec::new(), NativeStringProtocol::Jim084);
    let incoming = body.native_lifetime_lease();
    let _observer = observe(old_view.value(), {
        let context = Rc::clone(&context);
        let incoming = incoming.clone();
        move |retiring| {
            assert_eq!(retiring.native_object_reference_count(), 0);
            assert!(context.with_result_object(|current| current.is_same_object(retiring)));
            assert_eq!(incoming.value().native_object_reference_count(), 2);
        }
    });
    let result = vm
        .adopt_native_interp_result(incoming.clone().into_value())
        .unwrap();
    assert!(result.value().is_same_object(&body));
    assert_eq!(body.native_object_reference_count(), 2);
    assert!(body.resident_string_bytes().is_none());
}

fn original_body_fixture() -> (
    Rc<RefCell<crate::Vm>>,
    NativeObjectLifetimeLease,
    std::rc::Weak<crate::command::ProcDef>,
) {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    let body = Value::native_list_constructor(
        vec![
            Value::new_native_string_bytes(b"return".as_slice()),
            Value::new_native_string_bytes(b"BODY".as_slice()),
        ],
        NativeStringProtocol::Jim084,
    );
    let original = body.native_lifetime_lease();
    let defined = vm.invoke_host_original_object_vector(
        &Value::string("proc"),
        &[
            Value::string("p"),
            Value::native_list_constructor(Vec::new(), NativeStringProtocol::Jim084),
            body.clone(),
        ],
    );
    assert_eq!(defined.code, tcl_runtime_api::Code::Ok);
    let declaration = vm.proc_def_bytes(b"p").unwrap();
    assert!(declaration.body_src.is_same_object(&body));
    let weak = Rc::downgrade(&declaration);
    drop(declaration);
    drop(defined);
    drop(body);
    assert_eq!(original.value().native_object_reference_count(), 1);
    assert!(original.value().resident_string_bytes().is_none());
    (Rc::new(RefCell::new(vm)), original, weak)
}

#[test]
fn original_jim_info_body_public_caller_keeps_lazy_body_and_warm_original_name() {
    let (vm, body, _) = original_body_fixture();
    let name = Value::string("p");
    for cold in [true, false] {
        if !cold {
            name.invalidate_native_string_for_test();
            assert!(name.resident_string_bytes().is_none());
        }
        let result = vm.borrow_mut().invoke_host_original_object_vector(
            &Value::string("info"),
            &[Value::string("body"), name.clone()],
        );
        assert_eq!(result.code, tcl_runtime_api::Code::Ok);
        assert!(result.result.is_same_object(body.value()));
        assert!(body.value().resident_string_bytes().is_none());
        assert_eq!(body.value().native_object_type_name(), "list");
        assert_eq!(body.value().native_object_reference_count(), 2);
        assert_eq!(name.resident_string_bytes().is_some(), cold);
        assert!(vm.borrow().execution_refusal.is_none());
    }
}

#[test]
fn original_jim_info_body_capsule_keeps_only_weak_declaration_and_allocation_handles() {
    let (vm, body, declaration) = original_body_fixture();
    let name = Value::string("p");
    let warmed = tcl_cmd_core::info::body_result(&mut *vm.borrow_mut(), &name).unwrap();
    drop(warmed);
    let strong = declaration.strong_count();
    let tcl_cmd_core::info::BodyResult::Original(owner) =
        tcl_cmd_core::info::body_result(&mut *vm.borrow_mut(), &name).unwrap()
    else {
        panic!("actual original Jim body");
    };
    assert_eq!(declaration.strong_count(), strong);
    let original = owner.checked_value(&mut vm.borrow_mut()).unwrap();
    assert!(original.is_same_object(body.value()));
    assert_eq!(body.value().native_object_reference_count(), 1);
    drop(original);
    vm.borrow_mut().remove_registered_command("p");
    assert!(!body.value().native_object_is_live());
    assert!(owner.checked_value(&mut vm.borrow_mut()).is_err());
    assert!(vm.borrow().execution_refusal.is_some());
    assert!(body.value().resident_string_bytes().is_none());
}

#[cfg(debug_assertions)]
#[test]
fn original_jim_info_body_primitive_publication_allows_deletion_and_preserves_first_refusal() {
    // A direct capsule/setter window after prior-result installation;
    // ordinary public dispatch resets its prior result before this worker.
    for mode in 0..=2 {
        let (vm, body, _) = original_body_fixture();
        let context = vm.borrow().native_jim_object_context().unwrap();
        let prior = Value::new_native_string_bytes(b"PRIOR".as_slice());
        let prior_view = prior.native_lifetime_lease();
        context.publish_result(&prior);
        drop(prior);
        let name = Value::string("p");
        let tcl_cmd_core::info::BodyResult::Original(owner) =
            tcl_cmd_core::info::body_result(&mut *vm.borrow_mut(), &name).unwrap()
        else {
            panic!("actual original Jim body");
        };
        let original = owner.checked_value(&mut vm.borrow_mut()).unwrap();
        let _observer = observe(prior_view.value(), {
            let vm = Rc::clone(&vm);
            let context = Rc::clone(&context);
            let body = body.clone();
            move |retiring| {
                assert_eq!(retiring.native_object_reference_count(), 0);
                assert!(context.with_result_object(|current| current.is_same_object(retiring)));
                assert_eq!(body.value().native_object_reference_count(), 2);
                vm.borrow_mut().remove_registered_command("p");
                assert_eq!(body.value().native_object_reference_count(), 1);
                if mode == 1 {
                    let refused = vm
                        .borrow_mut()
                        .refuse_host_command("original body result observer".into());
                    assert_eq!(refused.code, tcl_runtime_api::Code::Error);
                } else if mode == 2 {
                    vm.borrow_mut()
                        .set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
                }
            }
        });
        context.publish_result(&original);
        drop(original);
        let finished = owner.finish(&mut vm.borrow_mut());
        assert_eq!(finished.is_ok(), mode == 0);
        assert!(context.with_result_object(|current| current.is_same_object(body.value())));
        assert_eq!(body.value().native_object_reference_count(), 1);
        assert!(body.value().resident_string_bytes().is_none());
        if mode != 0 {
            let first = vm.borrow().execution_refusal.clone().unwrap();
            assert_eq!(
                finished.unwrap_err().native_execution_refusal(),
                Some(&first)
            );
        }
    }
}

#[test]
fn original_jim_info_body_capsule_refuses_foreign_consumer_and_c_keeps_byte_compatibility() {
    let (vm, _, _) = original_body_fixture();
    let name = Value::string("p");
    let tcl_cmd_core::info::BodyResult::Original(owner) =
        tcl_cmd_core::info::body_result(&mut *vm.borrow_mut(), &name).unwrap()
    else {
        panic!("actual original Jim body");
    };
    let mut foreign = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    assert!(owner.checked_value(&mut foreign).is_err());
    let mut native_c =
        crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let untouched = Value::int(42);
    assert!(untouched.resident_string_bytes().is_none());
    assert!(
        tcl_runtime_api::Procs::proc_body_original_value(&mut native_c, &untouched)
            .unwrap()
            .is_none()
    );
    assert!(untouched.resident_string_bytes().is_none());
}
