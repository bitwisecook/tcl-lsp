// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual VM record/header roles, without a custom Native deletion callback.
use super::*;
use crate::command::{Command, NativeCommand};
use crate::interp::native_operation_currency::NativeOperationScope;
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::raw_string::NativeValueAccessRefusal;

fn provided(version: &str) -> (Vm, OoId, OoId) {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect(version));
    let setup = vm
        .eval_source("oo::class create C {}; C create obj")
        .unwrap();
    assert_eq!(setup.code, Code::Ok);
    let class = resolve_object_value(&mut vm, &Value::string("C"))
        .unwrap()
        .unwrap();
    let object = resolve_object_value(&mut vm, &Value::string("obj"))
        .unwrap()
        .unwrap();
    (vm, class, object)
}
fn declaration(vm: &mut Vm, body: &str) -> (Method, crate::value::NativeObjectLifetimeLease) {
    let original = Value::new_native_string_bytes(body.as_bytes());
    assert_eq!(original.native_object_reference_count(), 1);
    let method = match build_method(vm, &Value::empty(), &original, true) {
        Ok(method) => method,
        Err(error) => panic!("genuine method declaration failed: {:?}", error.code),
    };
    assert!(method.body_src.is_same_object(&original));
    let lease = original.native_lifetime_lease();
    drop(original);
    assert_eq!(lease.value().native_object_reference_count(), 1);
    (method, lease)
}
struct MethodEntry(Rc<Cell<usize>>);
impl NativeCommand for MethodEntry {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        self.0.set(self.0.get() + 1);
        ok(Value::empty())
    }
}
#[test]
fn original_method_table_replacement_preserves_record_and_retires_original_payload() {
    // Real build_method/table producers and VM header retirement, not C MethodType chronology.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let (mut vm, class, object) = provided(version);
        let (original, old_body) = declaration(&mut vm, "return OLD");
        vm.oo
            .classes
            .get_mut(&class)
            .unwrap()
            .methods
            .insert("m".into(), original);
        let owner = vm.oo.classes[&class].methods.owner("m").unwrap();
        let selected = Rc::downgrade(&owner);
        drop(owner);
        assert_eq!(selected.strong_count(), 1);
        assert!(old_body.value().native_object_is_live());
        let (replacement, new_body) = declaration(&mut vm, "return NEW");
        vm.oo
            .classes
            .get_mut(&class)
            .unwrap()
            .methods
            .insert("m".into(), replacement);
        let current = vm.oo.classes[&class].methods.owner("m").unwrap();
        assert!(selected.ptr_eq(&Rc::downgrade(&current)));
        assert!(current.borrow().body_src.is_same_object(new_body.value()));
        assert!(!old_body.value().native_object_is_live());
        assert_eq!(old_body.value().native_object_reference_count(), 0);
        assert_eq!(new_body.value().native_object_reference_count(), 1);
        drop(current);
        assert_eq!(selected.strong_count(), 1);
        let result = oo_invoke_value(
            &mut vm,
            object,
            "m",
            &Value::new_native_string_bytes(b"m".as_slice()),
            &[],
            true,
            &Value::new_native_string_bytes(b"obj".as_slice()),
        );
        assert_eq!(result.code, Code::Ok);
        assert_eq!(result.result.string_bytes().as_ref(), b"NEW");
        assert!(vm.execution_refusal.is_none());
    }
}
#[test]
fn original_unnamed_slot_replacement_keeps_only_genuine_entered_original_owner() {
    // Actual unnamed slots and retained chains; allocation leases add no native body references.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for role in [
            NativeTclooMethodReportRole::Constructor,
            NativeTclooMethodReportRole::Destructor,
        ] {
            let (mut vm, class, _) = provided(version);
            let (original, old_body) = declaration(&mut vm, "return OLD");
            let slot = if role == NativeTclooMethodReportRole::Constructor {
                &mut vm.oo.classes.get_mut(&class).unwrap().constructor
            } else {
                &mut vm.oo.classes.get_mut(&class).unwrap().destructor
            };
            slot.install(original);
            let original_owner = slot.owner().unwrap();
            let selected = Rc::downgrade(&original_owner);
            drop(original_owner);
            let steps = vm
                .oo
                .class_linear_of(class)
                .into_iter()
                .filter(|step| {
                    let provider = &vm.oo.classes[&step.provider];
                    if role == NativeTclooMethodReportRole::Constructor {
                        provider.constructor.is_some()
                    } else {
                        provider.destructor.is_some()
                    }
                })
                .collect();
            let active = native_method_cache::retain_owners(&vm.oo, steps, "", role);
            assert_eq!(selected.strong_count(), 2);
            let (replacement, new_body) = declaration(&mut vm, "return NEW");
            let slot = if role == NativeTclooMethodReportRole::Constructor {
                &mut vm.oo.classes.get_mut(&class).unwrap().constructor
            } else {
                &mut vm.oo.classes.get_mut(&class).unwrap().destructor
            };
            slot.install(replacement);
            let current = slot.owner().unwrap();
            assert!(!selected.ptr_eq(&Rc::downgrade(&current)));
            assert_eq!(selected.strong_count(), 1);
            assert_eq!(old_body.value().native_object_reference_count(), 1);
            assert!(old_body.value().native_object_is_live());
            assert!(
                active[0]
                    .method_owner
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .body_src
                    .is_same_object(old_body.value())
            );
            drop(active);
            assert!(selected.upgrade().is_none());
            assert!(!old_body.value().native_object_is_live());
            assert_eq!(old_body.value().native_object_reference_count(), 0);
            assert_eq!(new_body.value().native_object_reference_count(), 1);
            assert!(current.borrow().body_src.is_same_object(new_body.value()));
            assert!(vm.execution_refusal.is_none());
        }
    }
}
#[test]
fn original_missing_method_role_never_revives_a_present_table_record() {
    // A real precedence observation intentionally lacks the executable record role.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let (mut vm, class, object) = provided(version);
        let calls = Rc::new(Cell::new(0));
        vm.register_command(
            "retirement_method_entry",
            Command::Native(Rc::new(MethodEntry(Rc::clone(&calls)))),
        );
        let (method, _) = declaration(
            &mut vm,
            "retirement_method_entry; set method_ran 1; return FRESH",
        );
        vm.oo
            .classes
            .get_mut(&class)
            .unwrap()
            .methods
            .insert("m".into(), method);
        let step = vm
            .oo
            .object_precedence(object)
            .into_iter()
            .find(|step| vm.oo.step_method(step, "m").is_some())
            .unwrap();
        assert!(step.method_owner.is_none());
        let chain = native_method_cache::ReachedMethodChain::from(vec![step]);
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let prior = vm
            .adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
            .unwrap();
        let result = run_step(
            &mut vm,
            object,
            &chain,
            0,
            "m".into(),
            &[],
            true,
            Value::new_native_string_bytes(b"obj".as_slice()),
            vec![Value::string("obj"), Value::string("m")],
            Vec::new(),
        );
        assert_eq!(result.code, Code::Error);
        let first = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original TclOO selected method record",
            ),
        );
        assert_eq!(vm.execution_refusal, Some(first.clone()));
        assert_eq!(calls.get(), 0);
        assert!(vm.get_var_bytes(b"method_ran").is_none());
        assert!(vm.oo.classes[&class].methods.contains_key("m"));
        assert!(
            vm.oo.classes[&class]
                .methods
                .owner("m")
                .unwrap()
                .borrow()
                .compiled_body
                .is_none()
        );
        assert!(vm.oo.call_stack.is_empty());
        vm.with_native_interp_result(|result| assert!(result.is_same_object(prior.value())))
            .unwrap();
        assert!(scope.check(&mut vm).is_err());
        assert_eq!(vm.execution_refusal, Some(first));
    }
}
