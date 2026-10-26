// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-inspected setter stages and independent actual software owning roles.
use super::*;
use crate::Code;
use crate::cmd_oo::resolve_object_value;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{
    number::Number, raw_string::NativeValueAccessRefusal, scalar_getter::NativeScalarCache,
};

fn provided() -> (Vm, OoId, OoId) {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.1"));
    assert_eq!(
        vm.eval_source("oo::configurable create C {property a -get {return A}}; C create obj")
            .unwrap()
            .code,
        Code::Ok
    );
    let class = resolve_object_value(&mut vm, &Value::string("C"))
        .unwrap()
        .unwrap();
    let target = resolve_object_value(&mut vm, &Value::string("obj"))
        .unwrap()
        .unwrap();
    (vm, class, target)
}
fn numeric(vm: &Vm, value: i64) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(value)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}
fn supplied_old(
    vm: &mut Vm,
    target: OoId,
) -> (NativeObjectLifetimeLease, NativeObjectLifetimeLease) {
    let value = Value::new_native_string_bytes(b"-old".as_slice());
    let member = value.native_lifetime_lease();
    let key = vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(b"-old".as_slice()));
    vm.oo
        .objects
        .get_mut(&target)
        .unwrap()
        .readable_properties
        .insert(key.clone());
    vm.oo
        .property_members
        .entry((target, false, false))
        .or_default()
        .insert(key, value);
    let cache = vm.native_all_property_header(target, false, false).unwrap();
    assert_eq!(member.value().native_object_reference_count(), 2);
    assert_eq!(cache.value().native_object_reference_count(), 1);
    (member, cache)
}
struct Formatter {
    fail: bool,
    values: RefCell<Vec<i64>>,
    original: NativeObjectLifetimeLease,
    references: Cell<usize>,
    retired: Option<(NativeObjectLifetimeLease, NativeObjectLifetimeLease)>,
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
        self.values.borrow_mut().push(value);
        self.references
            .set(self.original.value().native_object_reference_count());
        if let Some((member, cache)) = &self.retired {
            assert_eq!(
                cache.value().native_object_reference_count(),
                0,
                "cache retired before String/hash"
            );
            assert!(!cache.value().native_object_is_live());
            assert_eq!(
                member.value().native_object_reference_count(),
                0,
                "old members retired before String/hash"
            );
            assert!(!member.value().native_object_is_live());
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
fn formatter(
    vm: &mut Vm,
    original: &Value,
    fail: bool,
    retired: Option<(NativeObjectLifetimeLease, NativeObjectLifetimeLease)>,
) -> Rc<Formatter> {
    let result = Rc::new(Formatter {
        fail,
        values: RefCell::new(Vec::new()),
        original: original.native_lifetime_lease(),
        references: Cell::new(0),
        retired,
    });
    vm.set_host(Rc::new(Host {
        actual: vm.host_rc(),
        formatter: Rc::clone(&result),
    }));
    result
}
#[test]
fn original_property_setter_retires_cache_and_members_before_sequential_hash() {
    // Software formatter/count observation at the inspected C91 stage; no Native callback chronology.
    for fail in [false, true] {
        let (mut vm, _, target) = provided();
        let retired = supplied_old(&mut vm, target);
        let incoming = numeric(&vm, 42);
        let later = numeric(&vm, 43);
        let callback = formatter(&mut vm, &incoming, fail, Some(retired));
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let recipe = vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .unwrap();
        let operation =
            OriginalPropertyOperation::enter(&mut vm, target, false, recipe.strings()).unwrap();
        let headers = [
            incoming.native_lifetime_lease(),
            later.native_lifetime_lease(),
        ];
        assert_eq!(
            incoming.native_object_reference_count(),
            1,
            "capture grants no incoming reference"
        );
        let reached =
            vm.install_original_property_members(target, false, false, &operation, &headers);
        assert_eq!(
            callback.references.get(),
            2,
            "caller plus actual incoming member role"
        );
        if fail {
            let error = reached.unwrap_err();
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(error.native_execution_refusal(), Some(&first));
            assert!(matches!(
                first,
                NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(_))
            ));
            assert_eq!(*callback.values.borrow(), [42]);
            assert_eq!(incoming.native_object_reference_count(), 1);
            assert_eq!(later.native_object_reference_count(), 1);
            assert!(!vm.oo.property_members.contains_key(&(target, false, false)));
            assert!(vm.oo.objects[&target].readable_properties.is_empty());
            assert!(operation.complete(&mut vm, Err::<(), _>(error)).is_err());
            let second = slot(
                &mut vm,
                target,
                false,
                false,
                SlotOp::Set,
                std::slice::from_ref(&later),
            )
            .unwrap_err();
            assert_eq!(second.native_execution_refusal(), Some(&first));
            assert_eq!(*callback.values.borrow(), [42]);
            assert_eq!(vm.execution_refusal, Some(first));
            assert!(scope.check(&mut vm).is_err());
        } else {
            assert_eq!(*callback.values.borrow(), [42, 43]);
            assert_eq!(reached.unwrap().len(), 2);
            operation.finish(&mut vm).unwrap();
            assert_eq!(incoming.native_object_reference_count(), 2);
            assert_eq!(later.native_object_reference_count(), 2);
            assert!(vm.execution_refusal.is_none());
            scope.check(&mut vm).unwrap();
        }
    }
}
#[test]
fn original_property_setter_acquires_all_incoming_roles_and_keeps_first_duplicate_header() {
    let (mut vm, _, target) = provided();
    let incoming = numeric(&vm, 42);
    let callback = formatter(&mut vm, &incoming, false, None);
    let recipe = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .unwrap();
    let operation =
        OriginalPropertyOperation::enter(&mut vm, target, false, recipe.strings()).unwrap();
    let headers = [
        incoming.native_lifetime_lease(),
        incoming.native_lifetime_lease(),
    ];
    let names = vm
        .install_original_property_members(target, false, false, &operation, &headers)
        .unwrap();
    assert_eq!(names.len(), 1);
    assert_eq!(
        callback.references.get(),
        3,
        "both incoming roles precede the first hash"
    );
    assert_eq!(*callback.values.borrow(), [42]);
    assert_eq!(
        incoming.native_object_reference_count(),
        2,
        "duplicate role retired; caller and first installed member survive"
    );
    assert!(
        vm.oo.property_members[&(target, false, false)]
            .values()
            .next()
            .unwrap()
            .is_same_object(&incoming)
    );
    operation.finish(&mut vm).unwrap();
}
#[test]
fn original_property_registration_compares_before_acquiring_working_list_member() {
    let (mut vm, _, target) = provided();
    let original = numeric(&vm, 42);
    let callback = formatter(&mut vm, &original, false, None);
    let key = vm
        .oo
        .intern_method_key(tcl_core_types::NameBytes::from(b"42".as_slice()));
    vm.oo
        .objects
        .get_mut(&target)
        .unwrap()
        .readable_properties
        .insert(key.clone());
    vm.oo
        .property_members
        .entry((target, false, false))
        .or_default()
        .insert(key, original);
    register(&mut vm, target, false, b"new", true, false).unwrap();
    assert_eq!(*callback.values.borrow(), [42]);
    assert_eq!(
        callback.references.get(),
        1,
        "comparison precedes the original member's List append"
    );
    assert_eq!(callback.original.value().native_object_reference_count(), 1);
    let members = &vm.oo.property_members[&(target, false, false)];
    assert_eq!(members.len(), 2);
    assert!(
        members
            .values()
            .any(|member| member.is_same_object(callback.original.value()))
    );
    assert!(
        members
            .values()
            .any(|member| member.resident_string_bytes().unwrap().as_ref() == b"-new")
    );
    assert!(vm.execution_refusal.is_none());
}
#[test]
fn original_property_slot_composition_keeps_original_headers_and_empty_append_is_noop() {
    let (mut vm, _, target) = provided();
    let first = Value::new_native_string_bytes(b"-first".as_slice());
    let second = Value::new_native_string_bytes(b"-second".as_slice());
    slot(
        &mut vm,
        target,
        false,
        false,
        SlotOp::Set,
        std::slice::from_ref(&first),
    )
    .unwrap();
    assert!(
        vm.oo.property_members[&(target, false, false)]
            .values()
            .next()
            .unwrap()
            .is_same_object(&first)
    );
    let cache = vm.native_all_property_header(target, false, false).unwrap();
    let before = first.native_object_reference_count();
    slot(&mut vm, target, false, false, SlotOp::Append, &[]).unwrap();
    assert_eq!(cache.value().native_object_reference_count(), 1);
    assert_eq!(first.native_object_reference_count(), before);
    slot(
        &mut vm,
        target,
        false,
        false,
        SlotOp::Append,
        std::slice::from_ref(&second),
    )
    .unwrap();
    assert!(!cache.value().native_object_is_live());
    assert_eq!(first.native_object_reference_count(), 2);
    assert_eq!(second.native_object_reference_count(), 2);
    assert_eq!(vm.oo.property_members[&(target, false, false)].len(), 2);
    slot(
        &mut vm,
        target,
        false,
        false,
        SlotOp::Remove,
        std::slice::from_ref(&first),
    )
    .unwrap();
    assert_eq!(first.native_object_reference_count(), 1);
    assert_eq!(second.native_object_reference_count(), 2);
    assert!(
        vm.oo.property_members[&(target, false, false)]
            .values()
            .next()
            .unwrap()
            .is_same_object(&second)
    );
    slot(&mut vm, target, false, false, SlotOp::Clear, &[]).unwrap();
    assert!(vm.oo.property_members[&(target, false, false)].is_empty());
    assert!(vm.oo.objects[&target].readable_properties.is_empty());
    assert_eq!(second.native_object_reference_count(), 1);
    assert!(vm.execution_refusal.is_none());
}
#[test]
fn original_property_facet_and_restored_context_refuse_before_setter_effects() {
    for restored_context in [false, true] {
        let (mut vm, class, _) = provided();
        let original = Value::new_native_string_bytes(b"-later".as_slice());
        let recipe = vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .unwrap();
        let operation =
            OriginalPropertyOperation::enter(&mut vm, class, true, recipe.strings()).unwrap();
        let cache = vm.native_all_property_header(class, true, false).unwrap();
        let current = vm.oo.property_members[&(class, true, false)]
            .values()
            .next()
            .unwrap()
            .native_lifetime_lease();
        let count = current.value().native_object_reference_count();
        if restored_context {
            vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.0"));
            vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl9.1"));
        } else {
            vm.oo.classes.get_mut(&class).unwrap().birth =
                super::super::super::Class::default().birth;
        }
        let error = vm
            .install_original_property_members(
                class,
                true,
                false,
                &operation,
                &[original.native_lifetime_lease()],
            )
            .unwrap_err();
        assert_eq!(
            error.native_execution_refusal(),
            vm.execution_refusal.as_ref()
        );
        assert_eq!(cache.value().native_object_reference_count(), 1);
        assert_eq!(current.value().native_object_reference_count(), count);
        assert_eq!(original.native_object_reference_count(), 1);
        assert!(operation.finish(&mut vm).is_err());
    }
}
#[test]
fn original_property_epoch_refusal_stops_later_publishers_with_full_first_cause() {
    let (mut vm, class, _) = provided();
    let cache = vm.native_all_property_header(class, true, false).unwrap();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    vm.oo.property_foundation_epoch = u64::MAX;
    let error = vm.native_property_method_created(class, true).unwrap_err();
    let first = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable(
            "original OO property Foundation epoch",
        ),
    );
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(vm.execution_refusal, Some(first.clone()));
    let method_epochs = vm.oo.native_methods.observation_epochs(class);
    let later = Value::new_native_string_bytes(b"-later".as_slice());
    let error = slot(
        &mut vm,
        class,
        true,
        false,
        SlotOp::Set,
        std::slice::from_ref(&later),
    )
    .unwrap_err();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(
        vm.oo.native_methods.observation_epochs(class),
        method_epochs
    );
    assert_eq!(cache.value().native_object_reference_count(), 1);
    assert_eq!(vm.oo.property_members[&(class, true, false)].len(), 1);
    assert_eq!(later.native_object_reference_count(), 1);
    assert!(scope.check(&mut vm).is_err());
}
