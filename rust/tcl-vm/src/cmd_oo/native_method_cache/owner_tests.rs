// SPDX-License-Identifier: AGPL-3.0-or-later
//! VM record/entered-operation controls; no C physical clientData count claim.
use super::*;
use crate::cmd_oo::{native_method_key, oo_invoke_value, resolve_object_value};
use crate::command::{Command, NativeCommand};

fn native(version: &str) -> Vm {
    crate::native_fixture::interpreter(
        tcl_registry::model::ingress::resolve_environment(version).unit_profile(),
    )
}
fn source(vm: &mut Vm, script: &str) {
    let result = vm.eval_source(script).unwrap();
    assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
}
fn object(vm: &mut Vm, name: &str) -> OoId {
    resolve_object_value(vm, &Value::string(name))
        .unwrap()
        .unwrap()
}
fn call(vm: &mut Vm, target: OoId, selector: &Value) -> Completion<Value> {
    oo_invoke_value(vm, target, "m", selector, &[], true, &Value::string("obj"))
}
#[test]
fn original_vm_idle_selector_cache_does_not_retain_deleted_method_records() {
    // naming.tcloo.original-method-cache-and-active-call-lifetime
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
    // Native506b idle case0 is a separate physical MethodType observation.
    // These assertions cover the VM's exact Rc record and selector cache roles.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = native(version);
        source(
            &mut vm,
            "oo::class create C {method m {} {return RESULT}}; C create obj",
        );
        let target = object(&mut vm, "obj");
        let class = vm.oo.objects[&target].class;
        let owner = vm.oo.classes[&class].methods.owner("m").unwrap();
        let weak = Rc::downgrade(&owner);
        drop(owner);
        let selector = Value::string("m");
        assert_eq!(
            call(&mut vm, target, &selector)
                .result
                .resident_string_bytes()
                .unwrap()
                .as_ref(),
            b"RESULT"
        );
        let duplicate = selector.duplicate_native_object();
        let chain = selector.native_method_name_chain().unwrap();
        assert_eq!(
            weak.strong_count(),
            1,
            "only the real table owns this idle record"
        );
        assert!(Rc::ptr_eq(
            &chain,
            &duplicate.native_method_name_chain().unwrap()
        ));
        source(&mut vm, "oo::define C deletemethod m");
        assert!(weak.upgrade().is_none());
        assert!(selector.native_method_name_chain().is_some());
        assert!(duplicate.native_method_name_chain().is_some());
        assert!(
            ReachedMethodChain::from_cached(chain).is_none(),
            "missing exact records cannot be reconstructed by name"
        );
        assert!(vm.execution_refusal.is_none());
    }
}
struct MutateInherited {
    base: OoId,
    original: Weak<RefCell<Method>>,
    mode: u8,
    calls: Rc<std::cell::Cell<usize>>,
}
impl NativeCommand for MutateInherited {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(
            self.original.strong_count(),
            2,
            "the inherited table and ALL entered chain each own the record"
        );
        let old = self.original.upgrade().unwrap();
        let script = match self.mode {
            0 => "oo::define B deletemethod m",
            1 => "oo::define B method m {} {return REPLACED}",
            _ => "oo::define B deletemethod m; oo::define B method m {} {return NEW_RECORD}",
        };
        let result = vm.eval_source(script).unwrap();
        assert_eq!(result.code, Code::Ok);
        if self.mode == 1 {
            assert!(Rc::ptr_eq(
                &old,
                &vm.oo.classes[&self.base].methods.owner("m").unwrap()
            ));
        } else if self.mode == 2 {
            assert!(!Rc::ptr_eq(
                &old,
                &vm.oo.classes[&self.base].methods.owner("m").unwrap()
            ));
        }
        drop(old);
        assert_eq!(
            self.original.strong_count(),
            if self.mode == 1 { 2 } else { 1 }
        );
        ok(Value::empty())
    }
}
#[test]
fn original_vm_entered_chain_keeps_all_records_and_reads_current_same_record_payload() {
    // naming.tcloo.original-method-cache-and-active-call-lifetime
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
    // Real VM command callbacks and method bodies; no custom C MethodType grant.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..3 {
            let mut vm = native(version);
            source(
                &mut vm,
                "oo::class create B {method m {} {return BASE}}; oo::class create C {superclass B; method m {} {mutate_inherited; next}}; C create obj",
            );
            let base = object(&mut vm, "B");
            let target = object(&mut vm, "obj");
            let owner = vm.oo.classes[&base].methods.owner("m").unwrap();
            let weak = Rc::downgrade(&owner);
            drop(owner);
            let calls = Rc::new(std::cell::Cell::new(0));
            vm.register_command(
                "mutate_inherited",
                Command::Native(Rc::new(MutateInherited {
                    base,
                    original: weak.clone(),
                    mode,
                    calls: Rc::clone(&calls),
                })),
            );
            let result = call(&mut vm, target, &Value::string("m"));
            assert_eq!(result.code, Code::Ok, "{}", result.result.to_str());
            assert_eq!(
                result.result.resident_string_bytes().unwrap().as_ref(),
                if mode == 1 {
                    b"REPLACED".as_slice()
                } else {
                    b"BASE".as_slice()
                }
            );
            assert_eq!(calls.get(), 1);
            assert_eq!(weak.strong_count(), usize::from(mode == 1));
            assert!(vm.execution_refusal.is_none());
        }
    }
}
#[test]
fn original_vm_cache_world_does_not_adopt_a_fresh_stamp_after_actual_mutation() {
    // naming.tcloo.original-method-cache-and-active-call-lifetime
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = native(version);
        source(
            &mut vm,
            "oo::class create C {method m {} {return FIRST}}; C create obj",
        );
        let target = object(&mut vm, "obj");
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_tcloo_method_cache_protocol()
            .unwrap();
        let selector = Value::string("m");
        let world = OriginalCacheWorld::capture(&vm, target, true, protocol).unwrap();
        world
            .check(&mut vm, &scope, target, true, protocol, &selector)
            .unwrap();
        source(&mut vm, "oo::define C method m {} {return SECOND}");
        scope.check(&mut vm).unwrap(); // Guest mutation is not context ABA.
        assert!(
            world
                .check(&mut vm, &scope, target, true, protocol, &selector)
                .is_err()
        );
        assert!(selector.native_method_name_chain().is_none());
        assert!(vm.execution_refusal.is_none());
        assert_ne!(
            selected_stamp(&vm.oo, target, true, protocol).unwrap().1,
            world.stamp
        );
    }
}
struct WithdrawContext;
impl NativeCommand for WithdrawContext {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_pack_facts(vec![tcl_runtime_api::PackFactStamp {
            pack: "method-original-context".into(),
            content_hash: 1,
            vocabulary_version: "actual".into(),
            overlay_generation: 0,
            evaluator_revision: 0,
        }]);
        vm.set_pack_facts(Vec::new());
        ok(Value::string("LATER_GUEST"))
    }
}
#[test]
fn original_vm_method_callback_context_restore_keeps_first_host_and_reached_cache() {
    // naming.tcloo.original-method-cache-and-active-call-lifetime
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
    // Context refusal at a genuine VM NativeCommand callback, not a simulated C free hook.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = native(version);
        source(
            &mut vm,
            "oo::class create C {method m {} {withdraw_context}}; C create obj",
        );
        vm.register_command(
            "withdraw_context",
            Command::Native(Rc::new(WithdrawContext)),
        );
        let target = object(&mut vm, "obj");
        let selector = Value::string("m");
        let result = call(&mut vm, target, &selector);
        assert_eq!(result.code, Code::Error);
        let expected = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "entered VM original execution context",
            ),
        );
        assert_eq!(vm.execution_refusal, Some(expected.clone()));
        let reached = selector.native_method_name_chain().unwrap();
        let untouched = Value::string("m");
        let repeated = call(&mut vm, target, &untouched);
        assert_eq!(repeated.code, Code::Error);
        assert_eq!(vm.execution_refusal, Some(expected));
        assert!(untouched.native_method_name_chain().is_none());
        assert!(Rc::ptr_eq(
            &reached,
            &selector.native_method_name_chain().unwrap()
        ));
    }
}

#[test]
fn original_vm_method_key_refuses_inherited_context_before_string_and_cache() {
    // naming.tcloo.original-method-cache-and-active-call-lifetime
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
    // A real installed context change/restore precedes the original key getter.
    // This is VM ingress ordering, not a C3 extension String-updater observation.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = native(version);
        let accepted = Value::int(17);
        assert!(accepted.resident_string_bytes().is_none());
        assert_eq!(native_method_key(&mut vm, &accepted).unwrap(), "17");
        assert_eq!(accepted.resident_string_bytes().unwrap().as_ref(), b"17");
        assert!(accepted.native_method_name_chain().is_none());
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        vm.set_pack_facts(vec![tcl_runtime_api::PackFactStamp {
            pack: "method-key-original-context".into(),
            content_hash: 1,
            vocabulary_version: "actual".into(),
            overlay_generation: 0,
            evaluator_revision: 0,
        }]);
        vm.set_pack_facts(Vec::new());
        let original = Value::int(23);
        let cache = original.native_scalar_cache();
        assert!(original.resident_string_bytes().is_none());
        let result = native_method_key(&mut vm, &original);
        assert!(result.is_err());
        assert!(original.resident_string_bytes().is_none());
        assert_eq!(original.native_scalar_cache(), cache);
        assert!(original.native_method_name_chain().is_none());
        let first = vm.execution_refusal.clone().unwrap();
        assert!(scope.check(&mut vm).is_err());
        assert_eq!(vm.execution_refusal, Some(first));
    }
}

#[test]
fn original_oo_cleanup_epoch_exhaustion_retains_first_cause_and_retires_whole_payload() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual selected metadata epoch exhaustion is a software refusal, not Native counter overflow evidence.
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.1"));
    let reached = vm
        .eval_source("oo::class create C {}; C create object")
        .unwrap();
    assert_eq!(reached.code, Code::Ok);
    let object = super::super::resolve_object_value(&mut vm, &Value::string("object"))
        .unwrap()
        .unwrap();
    let namespace = vm.oo.objects[&object].namespace_token.unwrap();
    let original = super::super::native_context::object_name(&vm, object);
    let name = original.native_lifetime_lease();
    drop(original);
    vm.oo.native_methods.epochs.insert(object, u64::MAX);
    let scope =
        crate::interp::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let reached = super::super::oo_destroy(&mut vm, object);
    assert_eq!(reached.code, Code::Error);
    assert!(!vm.oo.objects.contains_key(&object));
    assert!(!vm.native_namespace_token_has_owner(namespace));
    assert!(!name.value().native_object_is_live());
    assert_eq!(name.value().native_object_reference_count(), 0);
    let first = vm.execution_refusal.clone().unwrap();
    assert!(matches!(
        &first,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original OO method object epoch"
            )
        )
    ));
    assert_eq!(
        scope.check(&mut vm).unwrap_err().native_execution_refusal(),
        Some(&first)
    );
}

#[test]
fn original_method_creation_epoch_refuses_after_actual_same_record_payload_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual VM record and header roles; no external Native free or epoch claim.
    let mut vm = native("tcl9.1");
    source(&mut vm, "oo::class create C {}");
    let class = object(&mut vm, "C");
    let before = Value::new_native_string_bytes(b"return BEFORE".as_slice());
    let old_body = before.native_lifetime_lease();
    assert_eq!(
        super::super::def_method(
            &mut vm,
            true,
            class,
            &[Value::string("m"), Value::empty(), before]
        )
        .code,
        Code::Ok
    );
    assert_eq!(old_body.value().native_object_reference_count(), 1);
    let owner = vm.oo.classes[&class].methods.owner("m").unwrap();
    let record = Rc::downgrade(&owner);
    drop(owner);
    vm.oo.native_methods.foundation = u64::MAX;
    let property_epoch = vm.oo.property_foundation_epoch;
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let incoming = Value::new_native_string_bytes(b"return AFTER".as_slice());
    let reached = super::super::def_method(
        &mut vm,
        true,
        class,
        &[Value::string("m"), Value::empty(), incoming.clone()],
    );
    assert_eq!(reached.code, Code::Error);
    assert!(!old_body.value().native_object_is_live());
    assert_eq!(old_body.value().native_object_reference_count(), 0);
    let current = vm.oo.classes[&class].methods.owner("m").unwrap();
    assert!(Rc::ptr_eq(&record.upgrade().unwrap(), &current));
    assert!(current.borrow().body_src.is_same_object(&incoming));
    drop(current);
    assert_eq!(record.strong_count(), 1);
    assert_eq!(incoming.native_object_reference_count(), 2);
    assert_eq!(vm.oo.property_foundation_epoch, property_epoch);
    let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "original OO method Foundation epoch",
        ),
    );
    assert_eq!(vm.execution_refusal, Some(first.clone()));
    let later = Value::new_native_string_bytes(b"return LATER".as_slice());
    assert_eq!(
        super::super::def_method(
            &mut vm,
            true,
            class,
            &[Value::string("later"), Value::empty(), later.clone()]
        )
        .code,
        Code::Error
    );
    assert!(vm.oo.classes[&class].methods.owner("later").is_none());
    assert_eq!(later.native_object_reference_count(), 1);
    assert_eq!(
        scope.check(&mut vm).unwrap_err().native_execution_refusal(),
        Some(&first)
    );
    assert_eq!(vm.execution_refusal, Some(first));
}

#[test]
fn original_constructor_and_destructor_replace_actual_slot_roles_before_epoch_continuation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Same real definition consumers and MethodSlot roles; software order only.
    for constructor in [true, false] {
        for exhausted in [false, true] {
            let mut vm = native("tcl9.1");
            source(&mut vm, "oo::class create C {}");
            let class = object(&mut vm, "C");
            let before = Value::new_native_string_bytes(b"return BEFORE".as_slice());
            let old_body = before.native_lifetime_lease();
            let reached = if constructor {
                super::super::def_constructor(&mut vm, true, class, &[Value::empty(), before])
            } else {
                super::super::def_destructor(&mut vm, true, class, &[before])
            };
            assert_eq!(reached.code, Code::Ok);
            assert_eq!(old_body.value().native_object_reference_count(), 1);
            let old = if constructor {
                vm.oo.classes[&class].constructor.owner()
            } else {
                vm.oo.classes[&class].destructor.owner()
            }
            .unwrap();
            let record = Rc::downgrade(&old);
            drop(old);
            if exhausted {
                vm.oo.native_methods.foundation = u64::MAX;
            }
            let scope = NativeOperationScope::enter(&mut vm).unwrap();
            let incoming = Value::new_native_string_bytes(b"return AFTER".as_slice());
            let reached = if constructor {
                super::super::def_constructor(
                    &mut vm,
                    true,
                    class,
                    &[Value::empty(), incoming.clone()],
                )
            } else {
                super::super::def_destructor(&mut vm, true, class, &[incoming.clone()])
            };
            assert_eq!(reached.code, if exhausted { Code::Error } else { Code::Ok });
            assert!(record.upgrade().is_none());
            assert!(!old_body.value().native_object_is_live());
            assert_eq!(old_body.value().native_object_reference_count(), 0);
            let current = if constructor {
                vm.oo.classes[&class].constructor.owner()
            } else {
                vm.oo.classes[&class].destructor.owner()
            }
            .unwrap();
            assert!(current.borrow().body_src.is_same_object(&incoming));
            drop(current);
            assert_eq!(incoming.native_object_reference_count(), 2);
            if exhausted {
                let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "original OO method Foundation epoch",
                    ),
                );
                assert_eq!(vm.execution_refusal, Some(first.clone()));
                assert_eq!(
                    scope.check(&mut vm).unwrap_err().native_execution_refusal(),
                    Some(&first)
                );
            } else {
                let cleared = if constructor {
                    super::super::def_constructor(
                        &mut vm,
                        true,
                        class,
                        &[Value::empty(), Value::empty()],
                    )
                } else {
                    super::super::def_destructor(&mut vm, true, class, &[Value::empty()])
                };
                assert_eq!(cleared.code, Code::Ok);
                assert_eq!(incoming.native_object_reference_count(), 1);
                assert!(
                    if constructor {
                        vm.oo.classes[&class].constructor.owner()
                    } else {
                        vm.oo.classes[&class].destructor.owner()
                    }
                    .is_none()
                );
                scope.check(&mut vm).unwrap();
                assert!(vm.execution_refusal.is_none());
            }
        }
    }
}
