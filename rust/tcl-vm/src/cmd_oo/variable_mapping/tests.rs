// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual VM map/header stages, separate from inspected native C chronology.
use super::*;
use crate::cmd_oo::{Code, make_class_bytes, resolve_object_value};
use crate::value::NativeObjectLifetimeLease;
use std::cell::{Cell, RefCell};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};

fn provided() -> (Vm, OoId) {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.1"));
    let root = vm.oo.class_root.unwrap();
    assert_eq!(
        make_class_bytes(&mut vm, b"MappingC", None, root).code,
        Code::Ok
    );
    let target = resolve_object_value(&mut vm, &Value::string("MappingC"))
        .unwrap()
        .unwrap();
    (vm, target)
}
fn descriptor(value: &Value) -> DeclaredVariable {
    DeclaredVariable {
        original: value.native_lifetime_lease().into_value(),
        name: NameBytes::from(b"validated".as_slice()),
        full_name: None,
    }
}
fn numeric(vm: &Vm, number: i64) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(number)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}
struct Formatter {
    fail: bool,
    calls: RefCell<Vec<i64>>,
    old: NativeObjectLifetimeLease,
    incoming: Vec<NativeObjectLifetimeLease>,
    observed: Cell<bool>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: [9, 1, 0, 0],
            sha256: [7; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        _: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        self.calls.borrow_mut().push(value);
        if !self.observed.replace(true) {
            assert!(!self.old.value().native_object_is_live());
            assert_eq!(self.old.value().native_object_reference_count(), 0);
            for original in &self.incoming {
                assert_eq!(
                    original.value().native_object_reference_count(),
                    2,
                    "all actual incoming references precede any key getter"
                );
            }
        }
        if self.fail {
            Err(NativeIntegerFormatterUnavailable::Operation)
        } else {
            Ok(value.to_string().into_bytes())
        }
    }
}
struct Host {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<Formatter>,
}
impl tcl_platform::Host for Host {
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
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        Some(self.formatter.as_ref())
    }
}
fn formatting(vm: &mut Vm, target: OoId, originals: &[&Value], fail: bool) -> Rc<Formatter> {
    let old = Value::new_native_string_bytes(b"old".as_slice());
    let observer = old.native_lifetime_lease();
    vm.oo
        .classes
        .get_mut(&target)
        .unwrap()
        .variables
        .push(DeclaredVariable {
            original: old,
            name: NameBytes::from(b"old".as_slice()),
            full_name: None,
        });
    let formatter = Rc::new(Formatter {
        fail,
        calls: RefCell::new(Vec::new()),
        old: observer,
        incoming: originals
            .iter()
            .map(|value| value.native_lifetime_lease())
            .collect(),
        observed: Cell::new(false),
    });
    vm.set_host(Rc::new(Host {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
    }));
    formatter
}
#[test]
fn original_variable_mapping_acquires_all_and_retires_old_before_sequential_first_unique_keys() {
    // Actual VM roles/formatter are software observations; no Native callback equivalence.
    let (mut vm, target) = provided();
    let first = numeric(&vm, 42);
    let equal = numeric(&vm, 42);
    let later = numeric(&vm, 43);
    let formatter = formatting(&mut vm, target, &[&first, &equal, &later], false);
    let originals = [descriptor(&first), descriptor(&equal), descriptor(&later)];
    let names = OriginalOoNameScope::enter(&mut vm, Some((true, target))).unwrap();
    let policy = vm.name_policy_protocol().unwrap();
    install(&mut vm, &names, true, target, false, policy, &originals).unwrap();
    assert_eq!(*formatter.calls.borrow(), [42, 42, 43]);
    let map = &vm.oo.classes[&target].variables;
    assert_eq!(map.len(), 2);
    assert!(map[0].original.is_same_object(&first));
    assert_eq!(map[0].name.as_bytes(), b"42");
    assert_eq!(map[1].name.as_bytes(), b"43");
    assert_eq!(first.native_object_reference_count(), 2);
    assert_eq!(equal.native_object_reference_count(), 1);
    assert_eq!(later.native_object_reference_count(), 2);
    assert!(vm.oo.variable_mapping_retirements.is_empty());
    names.finish(&mut vm).unwrap();
}
#[test]
fn original_variable_mapping_string_refusal_keeps_full_first_cause_and_skips_later_key_and_adoption()
 {
    let (mut vm, target) = provided();
    let first = numeric(&vm, 42);
    let later = numeric(&vm, 43);
    let formatter = formatting(&mut vm, target, &[&first, &later], true);
    let outer = NativeOperationScope::enter(&mut vm).unwrap();
    let originals = [descriptor(&first), descriptor(&later)];
    let names = OriginalOoNameScope::enter(&mut vm, Some((true, target))).unwrap();
    let policy = vm.name_policy_protocol().unwrap();
    let reached = install(&mut vm, &names, true, target, false, policy, &originals).unwrap_err();
    let cause = vm.execution_refusal.clone().unwrap();
    assert_eq!(reached.native_execution_refusal(), Some(&cause));
    assert_eq!(*formatter.calls.borrow(), [42]);
    assert!(vm.oo.classes[&target].variables.is_empty());
    assert!(vm.oo.variable_mapping_retirements.is_empty());
    assert_eq!(first.native_object_reference_count(), 1);
    assert_eq!(later.native_object_reference_count(), 1);
    let second = install(&mut vm, &names, true, target, false, policy, &originals).unwrap_err();
    assert_eq!(second.native_execution_refusal(), Some(&cause));
    assert_eq!(*formatter.calls.borrow(), [42]);
    assert!(outer.check(&mut vm).is_err());
}
#[test]
fn original_private_variable_mapping_keeps_counted_key_and_stored_cstring_full_name_copy_role() {
    let (mut vm, target) = provided();
    let original = Value::new_native_string_bytes(b"p\0tail".as_slice());
    let names = OriginalOoNameScope::enter(&mut vm, Some((true, target))).unwrap();
    let policy = vm.name_policy_protocol().unwrap();
    install(
        &mut vm,
        &names,
        true,
        target,
        true,
        policy,
        &[descriptor(&original)],
    )
    .unwrap();
    let declared = &vm.oo.classes[&target].private_variables[0];
    assert!(declared.original.is_same_object(&original));
    assert_eq!(declared.name.as_bytes(), b"p\0tail");
    let full = declared.full_name.as_ref().unwrap();
    let lease = full.original.native_lifetime_lease();
    assert_eq!(
        full.name.as_bytes(),
        format!("{} : p", vm.oo.objects[&target].creation_id).as_bytes()
    );
    assert_eq!(lease.value().native_object_reference_count(), 1);
    let staged = observation(declared);
    assert!(staged.original.is_same_object(&original));
    assert!(
        staged
            .full_name
            .as_ref()
            .unwrap()
            .original
            .is_same_object(lease.value())
    );
    assert_eq!(original.native_object_reference_count(), 2);
    assert_eq!(lease.value().native_object_reference_count(), 1);
    drop(staged);
    assert_eq!(lease.value().native_object_reference_count(), 1);
    let copy = vm.oo.classes[&target].clone();
    assert!(
        copy.private_variables[0]
            .full_name
            .as_ref()
            .unwrap()
            .original
            .is_same_object(lease.value())
    );
    assert_eq!(
        copy.private_variables[0].private_storage(999_999),
        full.name
    );
    assert_eq!(lease.value().native_object_reference_count(), 2);
    drop(copy);
    assert_eq!(lease.value().native_object_reference_count(), 1);
    install(&mut vm, &names, true, target, true, policy, &[]).unwrap();
    assert!(!lease.value().native_object_is_live());
    assert_eq!(lease.value().native_object_reference_count(), 0);
    assert_eq!(original.native_object_reference_count(), 1);
    names.finish(&mut vm).unwrap();
}
#[test]
fn original_variable_mapping_allows_recursive_roster_and_ordinary_method_changes_without_world_epoch()
 {
    // Two actual entered mapping workers share target/currency; no Native reentry assertion.
    let (mut vm, target) = provided();
    let incoming = Value::string("outer");
    let inner = Value::string("inner");
    let names = OriginalOoNameScope::enter(&mut vm, Some((true, target))).unwrap();
    let policy = vm.name_policy_protocol().unwrap();
    let operation = NativeOperationScope::enter(&mut vm).unwrap();
    let cleanup = OriginalOoCleanup::enter(&mut vm);
    let creation = vm.oo.objects[&target].creation_id;
    let mut worker = Mapping {
        vm: &mut vm,
        names: &names,
        operation,
        cleanup,
        key: (target, true, false),
        policy,
        creation,
    };
    let acquired = worker.acquire_member(&descriptor(&incoming)).unwrap();
    let mut retired = worker.capture_old_mapping().unwrap();
    worker.retire_old_mapping(&mut retired).unwrap();
    install(
        worker.vm,
        worker.names,
        true,
        target,
        false,
        policy,
        &[descriptor(&inner)],
    )
    .unwrap();
    assert_eq!(inner.native_object_reference_count(), 2);
    let reached = worker.vm.invoke_command(
        "oo::define",
        &[
            Value::string("MappingC"),
            Value::string("method"),
            Value::string("ordinary"),
            Value::empty(),
            Value::string("return OK"),
        ],
    );
    assert_eq!(reached.code, Code::Ok);
    worker.check_mapping().unwrap();
    worker.begin_new_mapping(&mut retired).unwrap();
    assert_eq!(inner.native_object_reference_count(), 1);
    let key = worker.member_key(&acquired).unwrap();
    let actual = worker.prepare_unique_member(acquired, &key).unwrap();
    worker.publish_mapping(vec![actual], &mut retired).unwrap();
    worker.finish_mapping_retirement(Some(retired)).unwrap();
    assert_eq!(
        worker.vm.oo.classes[&target].variables[0].name.as_bytes(),
        b"outer"
    );
    assert_eq!(incoming.native_object_reference_count(), 2);
    assert!(worker.vm.execution_refusal.is_none());
    drop(worker);
    names.finish(&mut vm).unwrap();
}
#[test]
fn original_variable_mapping_refuses_changed_facet_and_context_before_any_incoming_role() {
    for context in [false, true] {
        let (mut vm, target) = provided();
        let original = Value::string("later");
        let outer = NativeOperationScope::enter(&mut vm).unwrap();
        let names = OriginalOoNameScope::enter(&mut vm, Some((true, target))).unwrap();
        let policy = vm.name_policy_protocol().unwrap();
        if context {
            let profile = crate::environment::profile_for_dialect("tcl9.0");
            vm.set_dialect_profile(profile);
            vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.1"));
        } else {
            vm.oo.classes.get_mut(&target).unwrap().birth = Rc::new(());
        }
        let error = install(
            &mut vm,
            &names,
            true,
            target,
            false,
            policy,
            &[descriptor(&original)],
        )
        .unwrap_err();
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(original.native_object_reference_count(), 1);
        assert!(vm.oo.classes[&target].variables.is_empty());
        assert!(vm.oo.variable_mapping_retirements.is_empty());
        assert!(outer.check(&mut vm).is_err());
    }
}
