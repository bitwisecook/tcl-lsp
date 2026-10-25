// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software property stage controls; Native header observations remain separate.
use super::*;
use crate::cmd_oo::resolve_object_value;
use crate::command::{Command, NativeCommand};
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::value::NativeObjectLifetimeLease;
use std::cell::{Cell, RefCell};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{
    number::Number, raw_string::NativeValueAccessRefusal, scalar_getter::NativeScalarCache,
};

struct Formatter {
    fail: bool,
    formats: Cell<usize>,
    original: NativeObjectLifetimeLease,
    references: Cell<usize>,
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
        self.formats.set(self.formats.get() + 1);
        self.references
            .set(self.original.value().native_object_reference_count());
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
fn interpreter() -> Vm {
    crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl9.1"))
}
fn instance() -> (Vm, OoId, OoId) {
    let mut vm = interpreter();
    let result = vm.eval_source("oo::configurable create C {property a -get {return A}; property z -get {return Z}}; C create o").unwrap();
    assert_eq!(result.code, Code::Ok);
    let class = resolve_object_value(&mut vm, &Value::string("C"))
        .unwrap()
        .unwrap();
    let target = resolve_object_value(&mut vm, &Value::string("o"))
        .unwrap()
        .unwrap();
    (vm, class, target)
}
#[test]
fn original_property_report_sort_uses_canonical_string_and_actual_member_roles() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual owning metadata and formatter stages are software fixtures, not Native SortPropList callbacks.
    for fail in [false, true] {
        let (mut vm, class, target) = instance();
        let numeric = Value::from_native_scalar_cache(
            NativeScalarCache::Number(Number::Int(42)),
            None,
            vm.actual_native_invocation_dialect(),
        )
        .unwrap();
        let formatter = Rc::new(Formatter {
            fail,
            formats: Cell::new(0),
            original: numeric.native_lifetime_lease(),
            references: Cell::new(0),
        });
        let key = vm
            .oo
            .intern_method_key(tcl_core_types::NameBytes::from(b"42".as_slice()));
        vm.oo
            .classes
            .get_mut(&class)
            .unwrap()
            .readable_properties
            .insert(key.clone());
        vm.oo
            .property_members
            .get_mut(&(class, true, false))
            .unwrap()
            .insert(key, numeric);
        vm.set_host(Rc::new(Host {
            actual: vm.host_rc(),
            formatter: Rc::clone(&formatter),
        }));
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let prior = vm
            .adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
            .unwrap();
        let reached = vm.native_all_property_header(target, false, false);
        assert_eq!(formatter.formats.get(), 1);
        assert_eq!(
            formatter.references.get(),
            2,
            "metadata and actual report member, no Native reference from the lease"
        );
        if fail {
            let Err(error) = reached else {
                panic!("formatter refusal must stop report publication");
            };
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(error.native_execution_refusal(), Some(&first));
            assert!(matches!(
                first,
                NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(_))
            ));
            assert!(!vm.oo.property_caches.contains_key(&(target, false)));
            assert_eq!(
                formatter.original.value().native_object_reference_count(),
                1
            );
            vm.with_native_interp_result(|current| assert!(current.is_same_object(prior.value())))
                .unwrap();
            assert!(scope.check(&mut vm).is_err());
            assert_eq!(vm.execution_refusal, Some(first));
        } else {
            let header = reached.unwrap();
            let names = header
                .value()
                .native_object_list_elements(tcl_syntax::native_string::NativeStringProtocol::C(
                    tcl_dialect::TclVersion::V9_1,
                ))
                .unwrap();
            assert!(
                names
                    .iter()
                    .any(|name| name.is_same_object(formatter.original.value()))
            );
            assert_eq!(
                formatter.original.value().native_object_reference_count(),
                2
            );
            assert!(vm.execution_refusal.is_none());
            scope.check(&mut vm).unwrap();
        }
    }
}
struct ChangeReport {
    class: OoId,
    facet: bool,
    calls: Rc<Cell<usize>>,
}
impl NativeCommand for ChangeReport {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.calls.set(self.calls.get() + 1);
        if self.facet {
            // Genuine fresh Class::default birth, an explicit software metadata fixture.
            vm.oo.classes.get_mut(&self.class).unwrap().birth =
                super::super::Class::default().birth;
            ok(Value::empty())
        } else {
            vm.eval_source("oo::define C {property b -get {return B}}")
                .unwrap()
        }
    }
}
#[test]
fn original_property_derivation_cannot_adopt_a_changed_roster_or_class_facet() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Real callback and fresh Class birth fixtures give no public Native demotion/lifetime claim.
    for facet in [false, true] {
        let (mut vm, class, target) = instance();
        let calls = Rc::new(Cell::new(0));
        vm.register_command(
            "change_report",
            Command::Native(Rc::new(ChangeReport {
                class,
                facet,
                calls: Rc::clone(&calls),
            })),
        );
        let strings = vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .unwrap()
            .strings();
        let operation = OriginalPropertyOperation::enter(&mut vm, target, false, strings).unwrap();
        let original = PropertyDerivation::capture(&vm, target, false, false, false).unwrap();
        assert_eq!(vm.invoke_command("change_report", &[]).code, Code::Ok);
        assert_eq!(calls.get(), 1);
        let error = original.check(&mut vm, &operation).unwrap_err();
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert!(!vm.oo.property_caches.contains_key(&(target, false)));
        assert!(operation.finish(&mut vm).is_err());
        assert_eq!(vm.execution_refusal, Some(first));
    }
}
#[test]
fn materialized_property_reports_preserve_native_snapshot_and_table_index_roles() {
    // naming.property.original-configure-retained-table-and-read-snapshot

    // Literal Native335 scripts and observed output tuples; C91 physical roles remain distinct from C90 results.
    let cases = [
        (
            include_str!(
                "../../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/configure-all-configurable.tcl"
            ),
            b"code 0 result {-a A -z Z} events {a z} properties {-a -b -z}".as_slice(),
        ),
        (
            include_str!(
                "../../../../../runtime/rust/tests/data/native_property_configure_retained_table_read_snapshot_original/335/multi-write-configurable.tcl"
            ),
            b"code 0 result {} events {a b} properties {-a -b -z}".as_slice(),
        ),
    ];
    for (script, expected) in cases {
        let mut vm = interpreter();
        let result = vm.eval_source(script).unwrap();
        assert_eq!(result.code, Code::Ok);
        assert!(vm.execution_refusal.is_none());
        let output = vm.eval_source("list code $code result $result events $events properties [info object properties o -all -readable]").unwrap();
        assert_eq!(output.code, Code::Ok);
        assert_eq!(output.result.string_bytes().as_ref(), expected);
        assert!(vm.execution_refusal.is_none());
    }
}
struct RetireKeyString {
    original: RefCell<Option<NativeObjectLifetimeLease>>,
    first: Cell<usize>,
    later: Rc<Cell<usize>>,
}
impl NativeCommand for RetireKeyString {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.first.set(self.first.get() + 1);
        self.original
            .borrow()
            .as_ref()
            .unwrap()
            .value()
            .invalidate_native_string_for_test();
        ok(Value::empty())
    }
}
struct LaterGetter(Rc<Cell<usize>>);
impl NativeCommand for LaterGetter {
    fn invoke(&self, _: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.0.set(self.0.get() + 1);
        ok(Value::empty())
    }
}
#[test]
fn property_output_key_refusal_stops_before_the_next_original_getter() {
    // naming.embedding.original-host-publication-and-fact-transport

    // A real first getter withdraws only its original key's String cache; this does not simulate Native hooks.
    let mut vm = interpreter();
    let later = Rc::new(Cell::new(0));
    let first = Rc::new(RetireKeyString {
        original: RefCell::new(None),
        first: Cell::new(0),
        later: Rc::clone(&later),
    });
    vm.register_command("retire_property_string", Command::Native(first.clone()));
    vm.register_command(
        "later_property_getter",
        Command::Native(Rc::new(LaterGetter(Rc::clone(&later)))),
    );
    assert_eq!(vm.eval_source("oo::configurable create C {property a -get {retire_property_string; return A}; property z -get {later_property_getter; return Z}}; C create o").unwrap().code, Code::Ok);
    let target = resolve_object_value(&mut vm, &Value::string("o"))
        .unwrap()
        .unwrap();
    let class = vm.oo.objects[&target].class;
    let a = vm.oo.property_members[&(class, true, false)]
        .values()
        .find(|value| value.resident_string_bytes().unwrap().as_ref() == b"-a")
        .unwrap()
        .native_lifetime_lease();
    *first.original.borrow_mut() = Some(a);
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let result = configure_all(&mut vm, target);
    assert_eq!(result.code, Code::Error);
    assert_eq!(first.first.get(), 1);
    assert_eq!(first.later.get(), 0);
    let cause = vm.execution_refusal.clone().unwrap();
    assert!(matches!(
        cause,
        NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(_))
    ));
    assert!(vm.oo.call_stack.is_empty());
    assert!(scope.check(&mut vm).is_err());
    assert_eq!(vm.execution_refusal, Some(cause));
}

struct ReplacePrivateRoster {
    target: OoId,
    class: OoId,
    hits: Rc<Cell<usize>>,
}
impl NativeCommand for ReplacePrivateRoster {
    fn invoke(&self, vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        self.hits.set(self.hits.get() + 1);
        let strings = vm
            .actual_native_invocation_dialect()
            .native_property_lookup_protocol()
            .unwrap()
            .strings();
        let mut operation =
            OriginalPropertyOperation::enter(vm, self.target, false, strings).unwrap();
        operation.select_declaring(vm).unwrap();
        let before = vm.oo.classes[&self.class].private_variables[0]
            .original
            .native_object_reference_count();
        let roster =
            original_stages::PrivateVariableRoster::capture(vm, self.class, false).unwrap();
        let count = vm.oo.classes[&self.class].private_variables[0]
            .original
            .native_object_reference_count();
        let method_epochs = vm.oo.native_methods.observation_epochs(self.target);
        let changed = vm
            .eval_source("oo::define C {private variable -set replacement}")
            .unwrap();
        assert_eq!(changed.code, Code::Ok);
        assert_eq!(
            vm.oo.native_methods.observation_epochs(self.target),
            method_epochs
        );
        assert_eq!(
            count, before,
            "captured declarations add only allocation leases"
        );
        let reached = roster.check(vm, &operation);
        assert!(reached.is_err());
        let cause = vm.execution_refusal.clone().unwrap();
        assert_eq!(
            reached.as_ref().unwrap_err().native_execution_refusal(),
            Some(&cause)
        );
        let failed = crate::command::completion_from_cmd_error(vm, reached.unwrap_err());
        operation.complete_invocation(vm, failed)
    }
}
#[test]
fn private_property_inventory_replacement_is_independent_of_method_epochs() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual entered method and declaration publisher; no Native callback or method-epoch permission.
    let mut vm = interpreter();
    assert_eq!(vm.eval_source("oo::configurable create C {private variable original; method checkPrivate {} {replace_private_roster; set reached_after 1}}; C create o").unwrap().code, Code::Ok);
    let class = resolve_object_value(&mut vm, &Value::string("C"))
        .unwrap()
        .unwrap();
    let target = resolve_object_value(&mut vm, &Value::string("o"))
        .unwrap()
        .unwrap();
    let hits = Rc::new(Cell::new(0));
    vm.register_command(
        "replace_private_roster",
        Command::Native(Rc::new(ReplacePrivateRoster {
            target,
            class,
            hits: Rc::clone(&hits),
        })),
    );
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let result = super::super::oo_invoke_value(
        &mut vm,
        target,
        "checkPrivate",
        &Value::new_native_string_bytes(b"checkPrivate".as_slice()),
        &[],
        true,
        &Value::string("o"),
    );
    assert_eq!(result.code, Code::Error);
    assert_eq!(hits.get(), 1);
    assert_eq!(
        vm.oo.classes[&class].private_variables[0].name.as_bytes(),
        b"replacement"
    );
    assert!(vm.oo.call_stack.is_empty());
    let cause = NativeExecutionError::ValueAccessRefusal(
        NativeValueAccessRefusal::CommandProtocolUnavailable(
            "original property private-variable roster",
        ),
    );
    assert_eq!(vm.execution_refusal, Some(cause.clone()));
    assert!(scope.check(&mut vm).is_err());
    assert_eq!(vm.execution_refusal, Some(cause));
}

#[test]
fn original_property_temporary_table_does_not_reread_its_frozen_member_strings() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Explicit software String withdrawal checks frozen table use, not Native char-pointer mutation.
    let (mut vm, _, target) = instance();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let mut table = None;
    let first = lookup_property(
        &mut vm,
        target,
        &Value::new_native_string_bytes(b"-a".as_slice()),
        false,
        &mut table,
        true,
    )
    .unwrap();
    assert_eq!(
        table
            .as_ref()
            .unwrap()
            .words
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<_>>(),
        vec![b"-a".as_slice(), b"-z".as_slice()]
    );
    first.value().invalidate_native_string_for_test();
    assert!(first.value().resident_string_bytes().is_none());
    let second = lookup_property(
        &mut vm,
        target,
        &Value::new_native_string_bytes(b"-a".as_slice()),
        false,
        &mut table,
        true,
    )
    .unwrap();
    assert!(second.value().is_same_object(first.value()));
    assert!(first.value().resident_string_bytes().is_none());
    assert!(vm.execution_refusal.is_none());
    drop(second);
    drop(first);
    drop(table);
    scope.check(&mut vm).unwrap();
}

#[test]
fn original_dictionary_constructor_retains_first_key_refusal_before_input_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Same constructor kernel and real formatter/input roles; no Native Dictionary callback chronology.
    let (mut vm, _, target) = instance();
    let first = Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(42)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap();
    let later = Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(43)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap();
    let later_observation = later.native_lifetime_lease();
    let formatter = Rc::new(Formatter {
        fail: true,
        formats: Cell::new(0),
        original: first.native_lifetime_lease(),
        references: Cell::new(0),
    });
    vm.set_host(Rc::new(Host {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
    }));
    let outer = NativeOperationScope::enter(&mut vm).unwrap();
    let strings = vm
        .actual_native_invocation_dialect()
        .native_property_lookup_protocol()
        .unwrap()
        .strings();
    let operation = OriginalPropertyOperation::enter(&mut vm, target, false, strings).unwrap();
    let reached = operation.dictionary(
        &mut vm,
        vec![
            (first, Value::new_native_string_bytes(b"one".as_slice())),
            (later, Value::new_native_string_bytes(b"two".as_slice())),
        ],
    );
    let reached = operation.complete(&mut vm, reached);
    assert!(reached.is_err());
    assert_eq!(formatter.formats.get(), 1);
    assert_eq!(formatter.references.get(), 1);
    assert_eq!(
        formatter.original.value().native_object_reference_count(),
        0
    );
    assert_eq!(later_observation.value().native_object_reference_count(), 0);
    assert!(later_observation.value().resident_string_bytes().is_none());
    let cause = vm.execution_refusal.clone().unwrap();
    assert_eq!(
        reached.unwrap_err().native_execution_refusal(),
        Some(&cause)
    );
    assert!(matches!(
        cause,
        NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(_))
    ));
    assert!(outer.check(&mut vm).is_err());
    assert_eq!(vm.execution_refusal, Some(cause));
}

#[test]
fn original_cached_accessor_reaches_success_without_a_property_string_getter() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual successful getter and cached accessor, with an explicit software String-withdrawal input.
    let (mut vm, class, target) = instance();
    let property = vm.oo.property_members[&(class, true, false)]
        .values()
        .find(|value| value.resident_string_bytes().unwrap().as_ref() == b"-a")
        .unwrap()
        .native_lifetime_lease();
    let first = read_original_property(&mut vm, target, property.value()).unwrap();
    assert_eq!(first.resident_string_bytes().unwrap().as_ref(), b"A");
    assert!(property.value().native_property_name_is_cached());
    property.value().invalidate_native_string_for_test();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let second = read_original_property(&mut vm, target, property.value()).unwrap();
    assert_eq!(second.resident_string_bytes().unwrap().as_ref(), b"A");
    assert!(property.value().resident_string_bytes().is_none());
    assert!(vm.execution_refusal.is_none());
    assert!(vm.oo.call_stack.is_empty());
    drop(first);
    drop(second);
    drop(property);
    scope.check(&mut vm).unwrap();
}
