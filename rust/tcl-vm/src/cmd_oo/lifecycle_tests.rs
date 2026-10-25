// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual OO payload and physical cleanup controls; Native callback chronology is separate.
use super::*;
use crate::command::{Command, NativeCommand};
use crate::interp::{
    native_operation_currency::NativeOperationScope, oo_cleanup::OriginalOoCleanup,
};
use crate::value::NativeObjectLifetimeLease;
use std::cell::Cell;
use tcl_runtime_api::NativeExecutionError;

fn provided() -> (Vm, OoId, OoId) {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.1"));
    let reached = vm
        .eval_source("oo::configurable create C {property a -get {return A}}; C create obj")
        .unwrap();
    assert_eq!(reached.code, Code::Ok, "{}", reached.result.to_str());
    let class = resolve_object_value(&mut vm, &Value::string("C"))
        .unwrap()
        .unwrap();
    let object = resolve_object_value(&mut vm, &Value::string("obj"))
        .unwrap()
        .unwrap();
    (vm, class, object)
}
fn declaration(vm: &mut Vm, source: &str) -> (Method, NativeObjectLifetimeLease) {
    let body = Value::new_native_string_bytes(source.as_bytes());
    let method = match build_method(vm, &Value::empty(), &body, true) {
        Ok(method) => method,
        Err(error) => panic!("actual declaration failed: {:?}", error.code),
    };
    assert!(method.body_src.is_same_object(&body));
    let lease = body.native_lifetime_lease();
    drop(body);
    assert_eq!(lease.value().native_object_reference_count(), 1);
    (method, lease)
}
fn retained_name(vm: &Vm, object: OoId) -> NativeObjectLifetimeLease {
    let actual = native_context::object_name(vm, object);
    let lease = actual.native_lifetime_lease();
    drop(actual);
    assert_eq!(lease.value().native_object_reference_count(), 1);
    lease
}
struct Destructor {
    calls: Rc<Cell<usize>>,
    mode: u8,
}
impl NativeCommand for Destructor {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        match self.mode {
            0 => ok(Value::empty()),
            1 => err("DESTRUCTOR_GUEST"),
            2 => vm.refuse_host_command("FIRST_DESTRUCTOR".into()),
            3 => {
                vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
                vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.1"));
                ok(Value::empty())
            }
            _ => unreachable!(),
        }
    }
}
fn install_destructor(vm: &mut Vm, class: OoId, mode: u8) -> Rc<Cell<usize>> {
    let calls = Rc::new(Cell::new(0));
    vm.register_command(
        "lifecycle_destructor",
        Command::Native(Rc::new(Destructor {
            calls: Rc::clone(&calls),
            mode,
        })),
    );
    let (method, _) = declaration(vm, "lifecycle_destructor");
    vm.oo
        .classes
        .get_mut(&class)
        .unwrap()
        .destructor
        .install(method);
    calls
}
fn dead(lease: &NativeObjectLifetimeLease) {
    assert!(!lease.value().native_object_is_live());
    assert_eq!(lease.value().native_object_reference_count(), 0);
}
#[test]
fn original_oo_class_cleanup_retires_actual_names_properties_and_whole_records() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual VM owning roles and NsIds; no C free-callback or MethodType chronology claim.
    let (mut vm, class, object) = provided();
    let class_namespace = vm.oo.objects[&class].namespace_token.unwrap();
    let object_namespace = vm.oo.objects[&object].namespace_token.unwrap();
    let class_name = retained_name(&vm, class);
    let object_name = retained_name(&vm, object);
    let (class_method, class_body) = declaration(&mut vm, "return CLASS");
    let (object_method, object_body) = declaration(&mut vm, "return OBJECT");
    vm.oo
        .classes
        .get_mut(&class)
        .unwrap()
        .methods
        .insert("m".into(), class_method);
    vm.oo
        .objects
        .get_mut(&object)
        .unwrap()
        .methods
        .insert("m".into(), object_method);
    let selected_class = Rc::downgrade(&vm.oo.classes[&class].methods.owner("m").unwrap());
    let selected_object = Rc::downgrade(&vm.oo.objects[&object].methods.owner("m").unwrap());
    let variable = Value::new_native_string_bytes(b"stored_declaration".as_slice());
    let variable_lease = variable.native_lifetime_lease();
    vm.oo
        .classes
        .get_mut(&class)
        .unwrap()
        .private_variables
        .push(DeclaredVariable {
            original: variable,
            name: NameBytes::from(b"stored_declaration".as_slice()),
            full_name: None,
        });
    let member = vm.oo.property_members[&(class, true, false)]
        .values()
        .next()
        .unwrap()
        .native_lifetime_lease();
    let cached = vm.native_all_property_header(class, true, false).unwrap();
    assert_eq!(cached.value().native_object_reference_count(), 1);
    assert!(member.value().native_object_reference_count() > 0);
    let reached = oo_destroy(&mut vm, class);
    assert_eq!(reached.code, Code::Ok);
    assert!(!vm.oo.objects.contains_key(&class));
    assert!(!vm.oo.objects.contains_key(&object));
    assert!(!vm.oo.classes.contains_key(&class));
    assert!(selected_class.upgrade().is_none());
    assert!(selected_object.upgrade().is_none());
    assert!(!vm.native_namespace_token_has_owner(class_namespace));
    assert!(!vm.native_namespace_token_has_owner(object_namespace));
    assert!(!vm.oo.property_origins.contains_key(&(class, true)));
    assert!(!vm.oo.property_members.keys().any(|(id, _, _)| *id == class));
    assert!(!vm.oo.property_caches.keys().any(|(id, _)| *id == class));
    for original in [
        &class_name,
        &object_name,
        &class_body,
        &object_body,
        &variable_lease,
        &member,
        &cached,
    ] {
        dead(original);
    }
    assert!(vm.execution_refusal.is_none());
}
#[test]
fn original_oo_cleanup_after_entry_refusal_skips_destructor_and_completes_owned_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, class, object) = provided();
    let calls = install_destructor(&mut vm, class, 0);
    let namespace = vm.oo.objects[&object].namespace_token.unwrap();
    let name = retained_name(&vm, object);
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    drop(vm.refuse_host_command("FIRST_BEFORE_CLEANUP".into()));
    let first = vm.execution_refusal.clone().unwrap();
    let reached = oo_destroy(&mut vm, class);
    assert_eq!(reached.code, Code::Error);
    assert_eq!(calls.get(), 0);
    assert_eq!(vm.execution_refusal, Some(first.clone()));
    assert!(!vm.oo.objects.contains_key(&class));
    assert!(!vm.oo.objects.contains_key(&object));
    assert!(!vm.oo.classes.contains_key(&class));
    assert!(!vm.native_namespace_token_has_owner(namespace));
    dead(&name);
    assert_eq!(
        scope.check(&mut vm).unwrap_err().native_execution_refusal(),
        Some(&first)
    );
}
#[test]
fn original_oo_destructor_guest_host_and_restored_context_preserve_cleanup_and_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Real destructor dispatch supplies Guest/Host/context effects, rather than a fabricated payload Drop.
    for mode in 0..=3 {
        let (mut vm, class, object) = provided();
        let calls = install_destructor(&mut vm, class, mode);
        let namespace = vm.oo.objects[&object].namespace_token.unwrap();
        let name = retained_name(&vm, object);
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let reached = oo_destroy(&mut vm, object);
        assert_eq!(calls.get(), 1);
        assert!(!vm.oo.objects.contains_key(&object));
        assert!(vm.oo.classes.contains_key(&class));
        assert!(!vm.native_namespace_token_has_owner(namespace));
        dead(&name);
        match mode {
            0 => {
                assert_eq!(reached.code, Code::Ok);
                assert!(vm.execution_refusal.is_none());
                scope.check(&mut vm).unwrap();
            }
            1 => {
                assert_eq!(reached.code, Code::Error);
                assert_eq!(reached.result.string_bytes().as_ref(), b"DESTRUCTOR_GUEST");
                assert!(vm.execution_refusal.is_none());
                scope.check(&mut vm).unwrap();
            }
            2 => {
                assert_eq!(reached.code, Code::Error);
                let first = vm.execution_refusal.clone().unwrap();
                assert!(
                    matches!(&first, NativeExecutionError::HostCommandRefusal(reason) if reason.reason == "FIRST_DESTRUCTOR")
                );
                assert_eq!(
                    scope.check(&mut vm).unwrap_err().native_execution_refusal(),
                    Some(&first)
                );
            }
            3 => {
                assert_eq!(reached.code, Code::Error);
                let first = vm.execution_refusal.clone().unwrap();
                assert!(matches!(first, NativeExecutionError::ValueAccessRefusal(_)));
                assert_eq!(
                    scope.check(&mut vm).unwrap_err().native_execution_refusal(),
                    Some(&first)
                );
            }
            _ => unreachable!(),
        }
    }
}
#[test]
fn original_oo_cleanup_uses_parked_original_interpreter_without_retiring_child_records() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, class, object) = provided();
    let namespace = vm.oo.objects[&object].namespace_token.unwrap();
    let name = retained_name(&vm, object);
    vm.create_child(Some("cleanup_child".into()), false);
    let child = vm.child_id("cleanup_child").unwrap();
    let owner = OriginalOoCleanup::enter(&mut vm);
    vm.in_interp(child, |vm| {
        let child_root = vm.oo.object_root.unwrap();
        let child_name = native_context::object_name(vm, child_root);
        let child_namespace = vm.oo.objects[&child_root].namespace_token.unwrap();
        teardown_original(vm, object, &owner).unwrap();
        assert!(vm.oo.objects.contains_key(&child_root));
        assert!(vm.native_namespace_token_has_owner(child_namespace));
        assert!(child_name.native_object_is_live());
        owner.check_cleanup(vm).unwrap();
    });
    assert!(!vm.oo.objects.contains_key(&object));
    assert!(vm.oo.classes.contains_key(&class));
    assert!(!vm.native_namespace_token_has_owner(namespace));
    dead(&name);
    assert!(vm.execution_refusal.is_none());
}
#[test]
fn original_oo_rename_continuation_keeps_record_and_refuses_withdrawn_or_restored_context() {
    // naming.embedding.original-host-publication-and-fact-transport

    for mode in 0..3 {
        let (mut vm, _, object) = provided();
        let name = retained_name(&vm, object);
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let original = original_oo_rename(&mut vm, object).unwrap();
        let old_display = vm.oo.names[&object].clone();
        if mode == 1 {
            teardown_checked(&mut vm, object).unwrap();
            let fresh = vm.eval_source("C create obj").unwrap();
            assert_eq!(fresh.code, Code::Ok);
            let replacement = resolve_object_value(&mut vm, &Value::string("obj"))
                .unwrap()
                .unwrap();
            assert_ne!(replacement, object);
        } else if mode == 2 {
            vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
            vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.1"));
        }
        let reached = oo_command_renamed(
            &mut vm,
            object,
            "later_metadata".into(),
            NameBytes::from(b"later_metadata".as_slice()),
            &original,
        );
        if mode == 0 {
            reached.unwrap();
            dead(&name);
            assert_eq!(
                vm.oo.command_keys[&object],
                CommandSidecarKey::visible("later_metadata")
            );
            assert_eq!(vm.oo.names[&object].as_bytes(), b"later_metadata");
            scope.check(&mut vm).unwrap();
        } else {
            let first = reached.unwrap_err();
            assert_eq!(vm.oo.names[&object], old_display);
            assert!(
                vm.oo
                    .command_keys
                    .get(&object)
                    .is_none_or(|key| key.name() != "later_metadata")
            );
            assert_eq!(
                scope.check(&mut vm).unwrap_err().native_execution_refusal(),
                first.native_execution_refusal()
            );
            if mode == 1 {
                dead(&name);
            } else {
                assert!(name.value().native_object_is_live());
            }
        }
    }
}
#[test]
fn original_oo_cascade_keeps_original_records_and_stops_later_destructors_after_host_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, class, first_object) = provided();
    let result = vm.eval_source("C create later").unwrap();
    assert_eq!(result.code, Code::Ok);
    let later = resolve_object_value(&mut vm, &Value::string("later"))
        .unwrap()
        .unwrap();
    let first_ns = vm.oo.objects[&first_object].namespace_token.unwrap();
    let later_ns = vm.oo.objects[&later].namespace_token.unwrap();
    let calls = install_destructor(&mut vm, class, 2);
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let result = oo_destroy(&mut vm, class);
    assert_eq!(result.code, Code::Error);
    assert_eq!(calls.get(), 1);
    assert!(!vm.oo.objects.contains_key(&first_object));
    assert!(!vm.oo.objects.contains_key(&later));
    assert!(!vm.oo.objects.contains_key(&class));
    assert!(!vm.oo.classes.contains_key(&class));
    assert!(!vm.native_namespace_token_has_owner(first_ns));
    assert!(!vm.native_namespace_token_has_owner(later_ns));
    let first = vm.execution_refusal.clone().unwrap();
    assert!(
        matches!(&first, NativeExecutionError::HostCommandRefusal(reason) if reason.reason == "FIRST_DESTRUCTOR")
    );
    assert_eq!(
        scope.check(&mut vm).unwrap_err().native_execution_refusal(),
        Some(&first)
    );
}

/// A method body compiles once and is reused until something C would
/// recompile it for changes: C keeps one `Proc` per method and
/// `TclProcCompileProc` reuses its bytecode while the compile epoch holds.
/// tclsh 9.0.4 prints `33`, `R`, `3`, `4` for the calls below.
#[test]
fn a_method_body_compiles_once_until_its_definition_or_command_world_changes() {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.0"));
    let run = |vm: &mut Vm, source: &str| {
        let completion = vm.eval_source(source).unwrap();
        assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
        completion.result.to_str().to_string()
    };
    run(
        &mut vm,
        "oo::class create C {method m {} {expr {1+2}}}; C create o",
    );
    let class = resolve_object_value(&mut vm, &Value::string("C"))
        .unwrap()
        .unwrap();
    let body = |vm: &Vm| {
        vm.oo
            .classes
            .get(&class)
            .and_then(|class| class.methods.get("m"))
            .and_then(|method| method.compiled_body)
            .map(|(_, unit)| unit.asm)
            .expect("the method body is compiled")
    };
    assert_eq!(run(&mut vm, "o m"), "3");
    let first = body(&vm);
    assert_eq!(run(&mut vm, "o m"), "3");
    assert!(
        Rc::ptr_eq(&first, &body(&vm)),
        "a second call must reuse the compiled body"
    );

    // A command-world change invalidates it, as C's compile epoch does.
    run(&mut vm, "rename expr _e; proc expr args {return R}");
    assert_eq!(run(&mut vm, "o m"), "R");
    let replaced = body(&vm);
    assert!(!Rc::ptr_eq(&first, &replaced));
    run(&mut vm, "rename expr {}; rename _e expr");
    assert_eq!(run(&mut vm, "o m"), "3");
    let restored = body(&vm);
    assert!(!Rc::ptr_eq(&replaced, &restored));
    assert_eq!(run(&mut vm, "o m"), "3");
    assert!(Rc::ptr_eq(&restored, &body(&vm)));

    // So does redefining the method.
    run(&mut vm, "oo::define C method m {} {expr {2+2}}");
    assert_eq!(run(&mut vm, "o m"), "4");
    assert!(!Rc::ptr_eq(&restored, &body(&vm)));
}
