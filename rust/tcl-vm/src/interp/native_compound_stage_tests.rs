// SPDX-License-Identifier: AGPL-3.0-or-later
//! Supplied original String stages in List-copy and Dictionary conversion.

use super::*;
use crate::value::NativeObjectLifetimeLease;
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_syntax::{native_string::NativeStringProtocol, scalar_getter::NativeScalarCache};

#[derive(Default)]
struct Formatter {
    calls: Cell<usize>,
    fail: Cell<bool>,
    retire: RefCell<Option<Value>>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: [8, 6, 18, 0],
            sha256: [7; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        _kind: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        self.calls.set(self.calls.get() + 1);
        drop(self.retire.borrow_mut().take());
        if self.fail.get() {
            return Err(NativeIntegerFormatterUnavailable::Operation);
        }
        Ok(value.to_string().into_bytes())
    }
}
struct Host {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<Formatter>,
    acquisitions: Rc<Cell<usize>>,
    numeric: Rc<Cell<usize>>,
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
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.numeric.set(self.numeric.get() + 1);
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.acquisitions.set(self.acquisitions.get() + 1);
        Some(&*self.formatter)
    }
}
type ProvidedCompoundControls = (Vm, Rc<Formatter>, Rc<Cell<usize>>, Rc<Cell<usize>>);

fn provided() -> ProvidedCompoundControls {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let formatter = Rc::new(Formatter::default());
    let acquisitions = Rc::new(Cell::new(0));
    let numeric = Rc::new(Cell::new(0));
    vm.set_host(Rc::new(Host {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
        acquisitions: Rc::clone(&acquisitions),
        numeric: Rc::clone(&numeric),
    }));
    acquisitions.set(0);
    numeric.set(0);
    (vm, formatter, acquisitions, numeric)
}
fn protocol() -> NativeStringProtocol {
    NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6)
}
fn integer(vm: &Vm, number: i64) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(number)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}
fn retired_header() -> tcl_runtime_api::NativeExecutionError {
    tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "retired native object header",
        ),
    )
}

#[test]
fn original_compound_copy_keeps_actual_backing_member_roles_and_queries_no_host() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, formatter, acquisitions, numeric) = provided();
    let root = Value::native_list_constructor(
        vec![
            Value::new_native_string_bytes(b"a".as_slice()),
            Value::new_native_string_bytes(b"b".as_slice()),
        ],
        protocol(),
    );
    let backing = root.native_list_backing_in(protocol()).unwrap().unwrap();
    let before = backing
        .elements()
        .unwrap()
        .iter()
        .map(Value::native_object_reference_count)
        .collect::<Vec<_>>();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let copied = OriginalNumericOps::in_original_scope(&mut vm, &scope)
        .unwrap()
        .copy_list_with_currency(&root, protocol(), scope.currency())
        .unwrap();
    assert!(!copied.is_same_object(&root));
    assert!(copied.native_list_attachment_matches(&backing));
    assert_eq!(
        backing
            .elements()
            .unwrap()
            .iter()
            .map(Value::native_object_reference_count)
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(acquisitions.get(), 0);
    assert_eq!(numeric.get(), 0);
    assert_eq!(formatter.calls.get(), 0);
    drop(copied);
    scope.check_native(&mut vm).unwrap();
}

#[test]
fn original_compound_dictionary_member_uses_the_supplied_key_string_once() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, formatter, acquisitions, numeric) = provided();
    let root = Value::new_native_string_bytes(b"old value".as_slice());
    let key = integer(&vm, 42);
    let key_view = key.native_lifetime_lease();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
    let mut prepared = ops
        .prepare_dictionary_with_currency(&root, protocol(), scope.currency())
        .unwrap();
    assert_eq!(acquisitions.get(), 0);
    ops.set_dictionary_member_with_currency(
        &mut prepared,
        key,
        Value::new_native_string_bytes(b"answer".as_slice()),
        protocol(),
        scope.currency(),
    )
    .unwrap();
    assert_eq!(
        key_view.value().resident_string_bytes().unwrap().as_ref(),
        b"42"
    );
    assert_eq!(formatter.calls.get(), 1);
    assert_eq!(acquisitions.get(), 1);
    assert_eq!(numeric.get(), 0);
    let reached = prepared
        .original()
        .with_cached_dictionary_member(b"42", |value| {
            value.unwrap().resident_string_bytes().unwrap()
        })
        .unwrap();
    assert_eq!(reached.as_ref(), b"answer");
    drop(prepared);
    drop(ops);
    scope.check_native(&mut vm).unwrap();
}

#[test]
fn original_compound_dictionary_conversion_stops_at_actual_root_retirement_before_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport

    // The supplied software formatter drops the actual final root owner.
    let (mut vm, formatter, acquisitions, numeric) = provided();
    let root = Value::native_list_constructor(
        vec![
            integer(&vm, 7),
            Value::new_native_string_bytes(b"value".as_slice()),
        ],
        protocol(),
    );
    let held: NativeObjectLifetimeLease = root.native_lifetime_lease();
    *formatter.retire.borrow_mut() = Some(root);
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let reached = OriginalNumericOps::in_original_scope(&mut vm, &scope)
        .unwrap()
        .prepare_dictionary_with_currency(held.value(), protocol(), scope.currency());
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(first, retired_header());
    assert_eq!(
        reached.err().unwrap().native_execution_refusal(),
        Some(&first)
    );
    assert!(!held.value().native_object_is_live());
    assert!(held.value().cached_dictionary_representation().is_none());
    assert_eq!(formatter.calls.get(), 1);
    assert_eq!(acquisitions.get(), 1);
    assert_eq!(numeric.get(), 0);
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}

#[test]
fn original_compound_failed_string_preserves_typed_host_before_dictionary_guest_work() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, formatter, acquisitions, numeric) = provided();
    formatter.fail.set(true);
    let root = Value::native_list_constructor(
        vec![
            integer(&vm, 7),
            Value::new_native_string_bytes(b"value".as_slice()),
        ],
        protocol(),
    );
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let reached = OriginalNumericOps::in_original_scope(&mut vm, &scope)
        .unwrap()
        .prepare_dictionary_with_currency(&root, protocol(), scope.currency());
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(
        reached.err().unwrap().native_execution_refusal(),
        Some(&first)
    );
    assert!(root.cached_dictionary_representation().is_none());
    assert!(root.native_object_is_live());
    assert_eq!(formatter.calls.get(), 1);
    assert_eq!(acquisitions.get(), 1);
    assert_eq!(numeric.get(), 0);
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}

#[test]
fn original_compound_protocol_refusal_precedes_formatter_or_conversion() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, formatter, acquisitions, numeric) = provided();
    let root = Value::new_native_string_bytes(b"key value".as_slice());
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let result = OriginalNumericOps::in_original_scope(&mut vm, &scope)
        .unwrap()
        .prepare_dictionary_with_currency(
            &root,
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
            scope.currency(),
        );
    assert!(result.is_err());
    assert!(root.cached_dictionary_representation().is_none());
    assert_eq!(formatter.calls.get(), 0);
    assert_eq!(acquisitions.get(), 0);
    assert_eq!(numeric.get(), 0);
    scope.check_native(&mut vm).unwrap();
}
