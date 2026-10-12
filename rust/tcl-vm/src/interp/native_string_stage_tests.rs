// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software formatter callback boundaries, with actual original VM and Value owners.

use super::*;
use crate::{Code, TclError};
use std::cell::{Cell, RefCell};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::number;
use tcl_syntax::scalar_getter::NativeScalarCache;

type Call = (&'static str, Option<(NativeIntegerKind, i64)>);
#[derive(Default)]
struct Formatter {
    calls: RefCell<Vec<Call>>,
    fail: Cell<bool>,
    format_other_vm: Cell<bool>,
    retire_name_world:
        RefCell<Option<Rc<RefCell<super::super::native_name_world::NativeNameWorld>>>>,
    original: RefCell<Option<crate::value::NativeObjectLifetimeLease>>,
    references: RefCell<Vec<usize>>,
    retire_at: Cell<Option<&'static str>>,
    retire_owner: RefCell<Option<Value>>,
    shimmer_root: RefCell<
        Option<(
            crate::value::NativeObjectLifetimeLease,
            tcl_registry::InvocationDialect,
        )>,
    >,
}
impl Formatter {
    fn record(&self, stage: &'static str, value: Option<(NativeIntegerKind, i64)>) {
        self.calls.borrow_mut().push((stage, value));
        if let Some(original) = self.original.borrow().as_ref() {
            self.references
                .borrow_mut()
                .push(original.value().native_object_reference_count());
        }
        if self.retire_at.get() == Some(stage) {
            let retired = self.retire_owner.borrow_mut().take();
            drop(retired);
        }
        if stage == "format" {
            if let Some(world) = self.retire_name_world.borrow_mut().take() {
                world.borrow_mut().retire();
            }
        }
        if stage == "format" && self.format_other_vm.replace(false) {
            let other =
                crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
            drop(other);
        }
        let shimmer = if stage == "format" {
            self.shimmer_root.borrow_mut().take()
        } else {
            None
        };
        if let Some((original, dialect)) = shimmer {
            original
                .value()
                .store_native_expression_long84(6, dialect)
                .unwrap();
        }
    }
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        self.record("build", None);
        NativeIntegerFormatterBuild {
            version: [8, 4, 20, 0],
            sha256: [7; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        kind: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        self.record("format", Some((kind, value)));
        if self.fail.get() {
            return Err(NativeIntegerFormatterUnavailable::Operation);
        }
        Ok(value.to_string().into_bytes())
    }
}
struct FormatterHost {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<Formatter>,
    acquisitions: Rc<Cell<usize>>,
}
impl tcl_platform::Host for FormatterHost {
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
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.acquisitions.set(self.acquisitions.get() + 1);
        Some(&*self.formatter)
    }
}
fn provided() -> (Vm, Rc<Formatter>, Rc<Cell<usize>>) {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
    let formatter = Rc::new(Formatter::default());
    let acquisitions = Rc::new(Cell::new(0));
    vm.set_host(Rc::new(FormatterHost {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
        acquisitions: Rc::clone(&acquisitions),
    }));
    (vm, formatter, acquisitions)
}
fn value(vm: &Vm, integer: i64) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(integer)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}
fn protocol() -> NativeStringProtocol {
    NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4)
}

#[test]
fn original_checked_string_and_concat_keep_actual_formatter_children_and_reference_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software formatter methods answer supplied Host stages; this is not an external Tcl formatter experiment.
    use tcl_syntax::value::ValueOps;
    for dictionary in [false, true] {
        let (mut vm, formatter, _) = provided();
        let first = value(&vm, i64::MIN);
        let second = value(&vm, i64::MIN + 1);
        let root = if dictionary {
            Value::dict(vec![(first, second)])
                .with_native_compound_string_protocol(protocol())
                .unwrap()
        } else {
            Value::native_list_constructor(vec![first, second], protocol())
        };
        let borrowed = if dictionary {
            root.with_cached_dictionary_representation(|pairs, _| {
                pairs[0].0.native_lifetime_lease()
            })
            .unwrap()
        } else {
            root.native_list_backing_in(protocol())
                .unwrap()
                .unwrap()
                .elements()
                .unwrap()[0]
                .native_lifetime_lease()
        };
        let count = borrowed.value().native_object_reference_count();
        let cache = borrowed.value().native_scalar_cache();
        *formatter.original.borrow_mut() = Some(borrowed);
        let bytes = vm.native_concat_string_bytes(&root).unwrap();
        assert_eq!(
            bytes.as_ref(),
            format!("{} {}", i64::MIN, i64::MIN + 1).as_bytes()
        );
        assert_eq!(
            formatter
                .calls
                .borrow()
                .iter()
                .filter(|call| call.0 == "format")
                .count(),
            2
        );
        assert!(
            formatter
                .references
                .borrow()
                .iter()
                .all(|actual| *actual == count)
        );
        assert_eq!(
            formatter
                .original
                .borrow()
                .as_ref()
                .unwrap()
                .value()
                .native_scalar_cache(),
            cache
        );
        let before = Rc::clone(&bytes);
        let calls = formatter.calls.borrow().len();
        let repeated = OriginalNumericOps::entered(&mut vm)
            .unwrap()
            .string(&root, protocol())
            .unwrap();
        assert!(Rc::ptr_eq(&before, &repeated));
        assert_eq!(formatter.calls.borrow().len(), calls);
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_checked_compound_string_stops_after_first_reached_formatter_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The checker withdraws the actual original VM after the recorded software callback stage.
    for stage in ["build", "format", "failed-format"] {
        let (mut vm, formatter, _) = provided();
        formatter.fail.set(stage == "failed-format");
        let root = Value::native_list_constructor(
            vec![value(&vm, i64::MIN), value(&vm, i64::MIN + 1)],
            protocol(),
        );
        let members = root.native_list_backing_in(protocol()).unwrap().unwrap();
        let children = members.elements().unwrap();
        let before = children
            .iter()
            .map(Value::native_object_snapshot)
            .collect::<Vec<_>>();
        let counts = children
            .iter()
            .map(Value::native_object_reference_count)
            .collect::<Vec<_>>();
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let withdrawn = Cell::new(false);
        let reached = root.native_string_bytes_checked::<CmdError>(
            protocol(),
            Some(&*formatter),
            &mut || {
                let target = if stage == "build" { "build" } else { "format" };
                if !withdrawn.get() && formatter.calls.borrow().iter().any(|call| call.0 == target)
                {
                    withdrawn.set(true);
                    let original = vm.host_rc();
                    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                    vm.set_host(original);
                }
                scope.check(&mut vm)
            },
        );
        let first = vm.execution_refusal.clone().unwrap();
        assert_eq!(
            reached.unwrap_err().native_execution_refusal(),
            Some(&first)
        );
        assert!(root.resident_string_bytes().is_none());
        assert_eq!(
            children
                .iter()
                .map(Value::native_object_snapshot)
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            children
                .iter()
                .map(Value::native_object_reference_count)
                .collect::<Vec<_>>(),
            counts
        );
        assert_eq!(
            formatter
                .calls
                .borrow()
                .iter()
                .filter(|call| call.0 == "build")
                .count(),
            1
        );
        assert_eq!(
            formatter
                .calls
                .borrow()
                .iter()
                .filter(|call| call.0 == "format")
                .count(),
            usize::from(stage != "build")
        );
        assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    }
}

#[test]
fn original_string_facade_refuses_wrong_protocol_and_withdrawn_scope_before_host_acquisition() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, acquisitions) = provided();
    let original = value(&vm, i64::MIN);
    let before = original.native_object_snapshot();
    let error = OriginalNumericOps::entered(&mut vm)
        .unwrap()
        .string(
            &original,
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6),
        )
        .unwrap_err();
    assert!(error.native_access_refusal().is_some());
    assert!(vm.execution_refusal.is_none());
    assert_eq!(acquisitions.get(), 0);
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let host = vm.host_rc();
    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
    vm.set_host(host);
    let error = OriginalNumericOps::entered(&mut vm).err().unwrap();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}

#[test]
fn original_checked_compound_string_refuses_a_retired_root_before_later_children() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The original VM Value owner is released at a software checker seam. The lease adds no native reference.
    let (mut vm, formatter, _) = provided();
    let owner = RefCell::new(Some(Value::native_list_constructor(
        vec![value(&vm, i64::MIN), value(&vm, i64::MIN + 1)],
        protocol(),
    )));
    let original = owner.borrow().as_ref().unwrap().native_lifetime_lease();
    assert_eq!(original.value().native_object_reference_count(), 1);
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let reached = original.value().native_string_bytes_checked::<CmdError>(
        protocol(),
        Some(&*formatter),
        &mut || {
            if formatter
                .calls
                .borrow()
                .iter()
                .any(|call| call.0 == "format")
            {
                drop(owner.borrow_mut().take());
            }
            scope.check(&mut vm)
        },
    );
    assert!(reached.unwrap_err().native_access_refusal().is_some());
    assert!(!original.value().native_object_is_live());
    assert!(original.value().resident_string_bytes().is_none());
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|call| call.0 == "format")
            .count(),
        1
    );
    assert!(vm.execution_refusal.is_none());
    scope.check_native(&mut vm).unwrap();
}

#[test]
fn original_string_facade_retains_header_refusal_at_actual_formatter_callbacks() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied software callbacks retire the actual VM root; no external object-type updater is asserted.
    for stage in ["build", "format", "failed-format"] {
        let (mut vm, formatter, acquisitions) = provided();
        formatter
            .retire_at
            .set(Some(if stage == "build" { "build" } else { "format" }));
        formatter.fail.set(stage == "failed-format");
        let root = Value::native_list_constructor(
            vec![value(&vm, i64::MIN), value(&vm, i64::MIN + 1)],
            protocol(),
        );
        let held = root.native_lifetime_lease();
        assert_eq!(root.native_object_reference_count(), 1);
        *formatter.original.borrow_mut() = Some(root.native_lifetime_lease());
        *formatter.retire_owner.borrow_mut() = Some(root);
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let reached = OriginalNumericOps::in_original_scope(&mut vm, &scope)
            .unwrap()
            .string(held.value(), protocol());
        let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object header",
            ),
        );
        assert_eq!(
            reached.unwrap_err().native_execution_refusal(),
            Some(&first)
        );
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        assert_eq!(acquisitions.get(), 1);
        assert!(!held.value().native_object_is_live());
        assert!(held.value().resident_string_bytes().is_none());
        assert!(
            formatter
                .references
                .borrow()
                .iter()
                .all(|count| *count == 1)
        );
        assert_eq!(
            formatter
                .calls
                .borrow()
                .iter()
                .filter(|call| call.0 == "build")
                .count(),
            1
        );
        assert_eq!(
            formatter
                .calls
                .borrow()
                .iter()
                .filter(|call| call.0 == "format")
                .count(),
            usize::from(stage != "build")
        );
        assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    }
}

#[test]
fn original_string_facade_retains_child_header_refusal_after_live_root_shimmer() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A software formatter shimmers the actual live root; original backing views pin allocation, not child ownership.
    let (mut vm, formatter, _) = provided();
    let root = Value::native_list_constructor(
        vec![value(&vm, i64::MIN), value(&vm, i64::MIN + 1)],
        protocol(),
    );
    let members = root.native_list_backing_in(protocol()).unwrap().unwrap();
    let child = members.elements().unwrap()[0].native_lifetime_lease();
    assert_eq!(root.native_object_reference_count(), 1);
    assert_eq!(child.value().native_object_reference_count(), 1);
    *formatter.shimmer_root.borrow_mut() = Some((
        root.native_lifetime_lease(),
        vm.actual_native_invocation_dialect(),
    ));
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let reached = OriginalNumericOps::in_original_scope(&mut vm, &scope)
        .unwrap()
        .string(&root, protocol());
    let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "retired native object header",
        ),
    );
    assert_eq!(
        reached.unwrap_err().native_execution_refusal(),
        Some(&first)
    );
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    assert!(root.native_object_is_live());
    assert_eq!(root.native_object_reference_count(), 1);
    assert_eq!(
        root.native_scalar_cache(),
        Some(NativeScalarCache::Tcl84Long(6))
    );
    assert!(root.resident_string_bytes().is_none());
    assert!(!child.value().native_object_is_live());
    assert!(child.value().resident_string_bytes().is_none());
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|call| call.0 == "format")
            .count(),
        1
    );
    assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}

#[test]
fn original_resident_string_checks_owner_and_protocol_without_querying_formatter_host() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // This observes actual software Host query counts, not external native String chronology.
    let (mut vm, formatter, acquisitions) = provided();
    let original = Value::new_native_string_bytes(b"already resident".as_slice());
    let before = original.native_object_snapshot();
    let count = original.native_object_reference_count();
    let bytes = original.resident_string_bytes().unwrap();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
    let output = ops.string(&original, protocol()).unwrap();
    assert!(Rc::ptr_eq(&bytes, &output));
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(original.native_object_reference_count(), count);
    assert!(
        ops.string(
            &original,
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6)
        )
        .is_err()
    );
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    drop(ops);
    let held = original.native_lifetime_lease();
    drop(original);
    let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
    let error = ops.string(held.value(), protocol()).unwrap_err();
    let first = ops.vm.borrow().execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    drop(ops);
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}

#[derive(Default)]
struct CompoundNumeric {
    calls: RefCell<Vec<(&'static str, Vec<u8>)>>,
}
impl CompoundNumeric {
    fn state_value() -> tcl_platform::NumericErrorState {
        tcl_platform::NumericErrorState {
            errno: 0,
            domain_error: false,
            range_error: false,
        }
    }
}
impl tcl_platform::NumericEnvironment for CompoundNumeric {
    fn c_integer_abi(
        &self,
    ) -> Result<tcl_platform::NativeCIntegerAbi, tcl_platform::NumericEnvironmentUnavailable> {
        self.calls.borrow_mut().push(("abi", Vec::new()));
        Ok(tcl_platform::NativeCIntegerAbi {
            char_bits: 8,
            int_bytes: 4,
            long_bytes: 8,
        })
    }
    fn state(
        &self,
    ) -> Result<tcl_platform::NumericErrorState, tcl_platform::NumericEnvironmentUnavailable> {
        self.calls.borrow_mut().push(("state", Vec::new()));
        Ok(Self::state_value())
    }
    fn reset(&self) -> Result<(), tcl_platform::NumericEnvironmentUnavailable> {
        self.calls.borrow_mut().push(("reset", Vec::new()));
        Ok(())
    }
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        _long: bool,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        tcl_platform::NumericEnvironment::unsigned(self, input, offset, 10)
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<tcl_platform::UnsignedNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.calls.borrow_mut().push(("unsigned", input.to_vec()));
        assert_eq!(base, 10);
        let mut value = 0_u64;
        let mut end = offset;
        for byte in &input[offset..] {
            if !byte.is_ascii_digit() {
                break;
            }
            value = value
                .saturating_mul(10)
                .saturating_add(u64::from(byte - b'0'));
            end += 1;
        }
        Ok(tcl_platform::UnsignedNumericConversion {
            value,
            end,
            before: Self::state_value(),
            after: Self::state_value(),
        })
    }
    fn double(
        &self,
        input: &[u8],
        _reset: bool,
    ) -> Result<tcl_platform::DoubleNumericConversion, tcl_platform::NumericEnvironmentUnavailable>
    {
        self.calls.borrow_mut().push(("double", input.to_vec()));
        let number = std::str::from_utf8(input)
            .ok()
            .and_then(|text| text.parse::<f64>().ok());
        Ok(tcl_platform::DoubleNumericConversion {
            value: number.unwrap_or(0.0),
            end: number.map_or(0, |_| input.len()),
            before: Self::state_value(),
            after: Self::state_value(),
        })
    }
}
struct CompoundHost {
    original: Rc<dyn tcl_platform::Host>,
    numeric: Rc<CompoundNumeric>,
}
impl tcl_platform::Host for CompoundHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.original.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.original.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.original.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.original.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        Some(&*self.numeric)
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.original.native_integer_formatter()
    }
}
fn compound_provided() -> (Vm, Rc<Formatter>, Rc<Cell<usize>>, Rc<CompoundNumeric>) {
    let (mut vm, formatter, queries) = provided();
    let numeric = Rc::new(CompoundNumeric::default());
    vm.set_host(Rc::new(CompoundHost {
        original: vm.host_rc(),
        numeric: Rc::clone(&numeric),
    }));
    (vm, formatter, queries, numeric)
}

#[test]
fn original_scalar_and_expression_compound_strings_keep_the_actual_formatter_once() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Numeric/formatter methods are authored software workers. This observes
    // forwarding/ref roles, not C strtod/strtoul or external execution chronology.
    use tcl_syntax::scalar_getter::{
        NativeScalarGetterKind as Kind, NativeScalarGetterValue as Scalar,
    };
    for getter in ["probe", "getter", "expression"] {
        let (mut vm, formatter, queries, numeric) = compound_provided();
        let root = Value::native_list_constructor(
            vec![
                Value::from_native_scalar_cache(
                    NativeScalarCache::Tcl84Long(i64::MIN),
                    None,
                    vm.actual_native_invocation_dialect(),
                )
                .unwrap(),
            ],
            protocol(),
        );
        let held = root.native_lifetime_lease();
        let child = root
            .native_list_backing_in(protocol())
            .unwrap()
            .unwrap()
            .elements()
            .unwrap()[0]
            .native_lifetime_lease();
        *formatter.original.borrow_mut() = Some(child);
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
        let scalar = match getter {
            "probe" => ops.probe(held.value(), Kind::Double).unwrap().unwrap(),
            "getter" => ops.getter(held.value(), Kind::Double).unwrap(),
            _ => ops.expression_integer84(held.value()).unwrap().unwrap(),
        };
        match scalar {
            Scalar::Double(actual) => assert_eq!(actual.to_bits(), (i64::MIN as f64).to_bits()),
            Scalar::Wide(actual) => assert_eq!(actual, i64::MIN),
            other => panic!("selected original scalar {other:?}"),
        }
        drop(ops);
        assert_eq!(queries.get(), 1);
        assert_eq!(
            formatter
                .calls
                .borrow()
                .iter()
                .filter(|call| call.0 == "format")
                .count(),
            1
        );
        assert!(
            formatter
                .references
                .borrow()
                .iter()
                .all(|count| *count == 1)
        );
        assert_eq!(held.value().native_object_reference_count(), 1);
        assert_eq!(
            held.value().resident_string_bytes().unwrap().as_ref(),
            b"-9223372036854775808"
        );
        assert!(
            numeric
                .calls
                .borrow()
                .iter()
                .any(|(stage, _)| matches!(*stage, "double" | "unsigned"))
        );
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_scalar_compound_formatter_withdrawal_stops_before_later_numeric_workers() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The supplied software formatter releases the real root reference;
    // allocation-only leases supply no extension/free callback issuer.
    use tcl_syntax::scalar_getter::NativeScalarGetterKind as Kind;
    for getter in ["probe", "getter", "expression"] {
        for stage in ["build", "format"] {
            let (mut vm, formatter, _, numeric) = compound_provided();
            let root = Value::native_list_constructor(
                vec![
                    Value::from_native_scalar_cache(
                        NativeScalarCache::Tcl84Long(i64::MIN),
                        None,
                        vm.actual_native_invocation_dialect(),
                    )
                    .unwrap(),
                ],
                protocol(),
            );
            let held = root.native_lifetime_lease();
            *formatter.retire_owner.borrow_mut() = Some(root);
            formatter.retire_at.set(Some(stage));
            let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
            let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
            let error = match getter {
                "probe" => ops.probe(held.value(), Kind::Double).unwrap_err(),
                "getter" => ops.getter(held.value(), Kind::Double).unwrap_err(),
                _ => ops.expression_integer84(held.value()).unwrap_err(),
            };
            let first = ops.vm.borrow().execution_refusal.clone().unwrap();
            assert_eq!(error.native_execution_refusal(), Some(&first));
            drop(ops);
            assert_eq!(formatter.calls.borrow().last().unwrap().0, stage);
            assert!(
                !numeric
                    .calls
                    .borrow()
                    .iter()
                    .any(|(stage, _)| matches!(*stage, "unsigned" | "double"))
            );
            assert!(!held.value().native_object_is_live());
            assert!(held.value().resident_string_bytes().is_none());
            assert!(held.value().native_scalar_cache().is_none());
            assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
            assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
        }
    }
}

#[test]
fn original_number_and_failure_string_providers_check_currentness_before_interpretation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // This directly supplies the String worker's result before withdrawing its
    // actual owner, so it claims no arbitrary callback in the closed Number parser.
    use tcl_syntax::scalar_getter::{
        NativeNumberGetterKind, NativeScalarGetterFailure, NativeScalarGetterKind,
    };
    for failure in [false, true] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
        let dialect = vm.actual_native_invocation_dialect();
        let original = Value::new_native_string_bytes(if failure {
            b"invalid".as_slice()
        } else {
            b"6".as_slice()
        });
        let held = original.native_lifetime_lease();
        let mut owner = Some(original);
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
        let mut check = || ops.current_for_value(held.value());
        let mut string = || {
            let result = held.value().resident_string_bytes().unwrap();
            drop(owner.take());
            Ok(result)
        };
        let error = if failure {
            held.value()
                .native_scalar_failure_presentation_checked_with_string(
                    dialect,
                    NativeScalarGetterKind::Wide,
                    NativeScalarGetterFailure::Invalid,
                    &mut check,
                    &mut string,
                )
                .unwrap_err()
        } else {
            held.value()
                .native_number_probe_checked_with_string(
                    dialect,
                    NativeNumberGetterKind::Number,
                    &mut check,
                    &mut string,
                )
                .unwrap_err()
        };
        let first = ops.vm.borrow().execution_refusal.clone().unwrap();
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert!(held.value().native_scalar_cache().is_none());
        drop(ops);
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    }
}

#[test]
fn original_expression_source_preparation_keeps_current_cache_and_explicit_independent_route() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual source/cache handoff; this does not execute or compare an external compiler.
    for engine in ["tcl8.4", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(engine));
        let source = Value::new_native_string_bytes(b"6 + 18".as_slice());
        let before = source.native_object_reference_count();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        vm.prepare_expression_value_in_original_scope(&source, &scope)
            .unwrap();
        assert!(source.native_expression_cache_present(), "{engine}");
        vm.prepare_expression_value_in_original_scope(&source, &scope)
            .unwrap();
        assert_eq!(source.native_object_reference_count(), before, "{engine}");
        assert_eq!(source.resident_string_bytes().unwrap().as_ref(), b"6 + 18");
        assert!(vm.execution_refusal.is_none());
        assert!(scope.check_native(&mut vm).is_ok());
        let independent = Value::new_native_string_bytes(b"6 + 18".as_slice());
        vm.prepare_expression_value(&independent).unwrap();
        assert!(independent.native_expression_cache_present());
    }
}

#[test]
fn original_expression_source_checks_actual_formatter_refusal_before_parser_and_cache() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual Host formatter drops the real root; no external updater/compiler chronology.
    for stage in ["build", "format", "failed-format"] {
        let (mut vm, formatter, acquisitions) = provided();
        let root = Value::native_list_constructor(vec![value(&vm, i64::MIN)], protocol());
        let held = root.native_lifetime_lease();
        *formatter.retire_owner.borrow_mut() = Some(root);
        formatter
            .retire_at
            .set(Some(if stage == "build" { "build" } else { "format" }));
        formatter.fail.set(stage == "failed-format");
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let error = vm
            .prepare_original_expression_value(held.value())
            .unwrap_err();
        let first = vm.execution_refusal.clone().unwrap();
        assert!(matches!(error,
            TclError::Host(crate::error::TclHostFailure::Execution(ref actual)) if actual == &first));
        assert_eq!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "retired native object header"
                )
            )
        );
        assert!(!held.value().native_object_is_live());
        assert!(held.value().resident_string_bytes().is_none());
        assert!(!held.value().native_expression_cache_present());
        assert_eq!(acquisitions.get(), 1);
        assert_eq!(
            formatter.calls.borrow().last().unwrap().0,
            if stage == "build" { "build" } else { "format" }
        );
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
        assert_eq!(vm.try_invoke_command("set", &[]).unwrap_err(), first);
    }
}

#[test]
fn original_expression_source_keeps_live_root_child_refusal_before_cache_replacement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The real formatter shimmers a live root, retiring its actual child backing.
    let (mut vm, formatter, _) = provided();
    let root = Value::native_list_constructor(vec![value(&vm, i64::MIN)], protocol());
    *formatter.shimmer_root.borrow_mut() = Some((
        root.native_lifetime_lease(),
        vm.actual_native_invocation_dialect(),
    ));
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let error = vm.prepare_original_expression_value(&root).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert!(matches!(error,
        TclError::Host(crate::error::TclHostFailure::Execution(ref actual)) if actual == &first));
    assert_eq!(
        first,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object header"
            )
        )
    );
    assert!(root.native_object_is_live());
    assert_eq!(
        root.native_scalar_cache(),
        Some(NativeScalarCache::Tcl84Long(6))
    );
    assert!(!root.native_expression_cache_present());
    assert!(root.resident_string_bytes().is_none());
    assert_eq!(formatter.calls.borrow().last().unwrap().0, "format");
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}

#[test]
fn original_expression_cache_publication_checks_actual_header_before_replacement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The supplied checker withdraws the original header at the cache worker;
    // it supplies no arbitrary callback or external cache-retirement chronology.
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let independent = Value::new_native_string_bytes(b"6 + 18".as_slice());
    let node = vm.prepare_expression_value(&independent).unwrap();
    let dialect = vm.native_invocation_dialect();
    let profile = vm.expression_source_profile().cache_key();
    let policy = vm.native_compiler_policy();
    let original = Value::int(6);
    let held = original.native_lifetime_lease();
    let mut owner = Some(original);
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let ops = OriginalNumericOps::in_original_scope(&mut vm, &scope).unwrap();
    let mut calls = 0;
    let error = {
        let mut check = || {
            calls += 1;
            if calls == 2 {
                drop(owner.take());
            }
            ops.current_for_value(held.value())
        };
        held.value()
            .cache_expression_checked(dialect, profile, &policy, &node, &mut check)
            .unwrap_err()
    };
    drop(ops);
    assert_eq!(calls, 2);
    assert!(!held.value().native_expression_cache_present());
    assert_eq!(
        error.native_execution_refusal(),
        vm.execution_refusal.as_ref()
    );
    assert!(error.native_execution_refusal().is_some());
    assert_eq!(
        scope.check_native(&mut vm).unwrap_err(),
        *error.native_execution_refusal().unwrap()
    );
}

#[test]
fn original_integer_operations_keep_selected_cache_and_magnitude_reference_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_syntax::{number::Radix, value::ValueOps};
    let (mut vm, formatter, acquisitions) = provided();
    let current = value(&vm, -6);
    let refs = current.native_object_reference_count();
    let before = current.native_object_snapshot();
    let magnitude = vm
        .integer_magnitude(&current, Radix::Hex, tcl_dialect::NumberSyntax::Tcl84)
        .unwrap();
    assert!(magnitude.negative);
    assert_eq!(magnitude.digits, "6");
    assert_eq!(current.native_object_snapshot(), before);
    assert_eq!(current.native_object_reference_count(), refs);
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    let amount = Value::new_native_string_bytes(b"06".as_slice());
    let result = ValueOps::int_add(&mut vm, Some(&current), &amount).unwrap();
    assert_eq!(result.integer_representation(), Some(0));
    assert_eq!(amount.integer_representation(), Some(6));
    assert_eq!(amount.resident_string_bytes().unwrap().as_ref(), b"06");
    assert_eq!(current.native_object_reference_count(), refs);
    assert!(vm.execution_refusal.is_none());
    let mut modern = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let magnitude = Value::new_native_string_bytes(b"18446744073709551617".as_slice());
    let before = magnitude.native_object_snapshot();
    let formatted = tcl_cmd_core::format::format_cmd_with_syntax(
        &mut modern,
        &[Value::string("%lld"), magnitude.clone()],
        tcl_dialect::NumberSyntax::Tcl90,
    )
    .unwrap();
    assert_eq!(
        formatted.resident_string_bytes().unwrap().as_ref(),
        b"18446744073709551617"
    );
    assert_eq!(magnitude.native_object_snapshot(), before);
    assert!(modern.execution_refusal.is_none());
}

#[test]
fn original_integer_magnitude_stops_actual_formatter_header_withdrawal_before_guest_render() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The supplied software formatter retires a genuine VM root, without an external C callback claim.
    use tcl_syntax::{number::Radix, value::ValueOps};
    let (mut vm, formatter, _) = provided();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let original = Value::native_list_constructor(vec![value(&vm, i64::MIN)], protocol());
    let held = original.native_lifetime_lease();
    *formatter.retire_owner.borrow_mut() = Some(original);
    formatter.retire_at.set(Some("format"));
    let error = vm
        .integer_magnitude(held.value(), Radix::Dec, tcl_dialect::NumberSyntax::Tcl84)
        .unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert!(error.native_access_refusal().is_some());
    assert!(matches!(
        first,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
    ));
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert!(held.value().resident_string_bytes().is_none());
    assert!(held.value().native_scalar_cache().is_none());
    let calls = formatter.calls.borrow().len();
    let guest = vm.native_integer_error(&error);
    assert_eq!(guest.code, tcl_runtime_api::Code::Error);
    assert!(guest.result.resident_string_bytes().unwrap().is_empty());
    assert_eq!(formatter.calls.borrow().len(), calls);
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    assert_eq!(vm.finish_host_execution(guest).unwrap_err(), first);
}

#[test]
fn original_integer_addition_stops_withdrawn_compound_before_cache_and_result_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_syntax::value::ValueOps;
    let (mut vm, formatter, _) = provided();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let original = Value::native_list_constructor(vec![value(&vm, i64::MIN)], protocol());
    let held = original.native_lifetime_lease();
    *formatter.retire_owner.borrow_mut() = Some(original);
    formatter.retire_at.set(Some("format"));
    let error = ValueOps::int_add(&mut vm, None, held.value()).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert!(error.native_access_refusal().is_some());
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert!(held.value().resident_string_bytes().is_none());
    assert!(held.value().native_scalar_cache().is_none());
    let calls = formatter.calls.borrow().len();
    let repeated = ValueOps::int_add(&mut vm, None, &Value::int(1)).unwrap_err();
    assert!(repeated.native_access_refusal().is_some());
    assert_eq!(formatter.calls.borrow().len(), calls);
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
}

#[test]
fn original_integer_operations_preserve_guest_input_and_authored_routing_without_host_queries() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_syntax::{number::Radix, value::ValueOps};
    let (mut vm, formatter, acquisitions) = provided();
    for addition in [false, true] {
        let original = Value::new_native_string_bytes(b"\xffBAD\0tail".as_slice());
        let before = original.native_object_snapshot();
        let error = if addition {
            ValueOps::int_add(&mut vm, None, &original).unwrap_err()
        } else {
            vm.integer_magnitude(&original, Radix::Dec, tcl_dialect::NumberSyntax::Tcl84)
                .unwrap_err()
        };
        assert_eq!(
            error,
            ValueError::NotIntegerBytes(b"\xffBAD\0tail".to_vec())
        );
        assert_eq!(original.native_object_snapshot(), before);
        assert!(vm.execution_refusal.is_none());
    }
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
    ));
    let amount = Value::new_native_string_bytes(b"06".as_slice());
    let result = ValueOps::int_add(&mut vm, Some(&Value::int(1)), &amount).unwrap();
    assert_eq!(result.integer_representation(), Some(7));
    assert!(amount.integer_representation().is_none());
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_integer_error_keeps_prior_typed_cause_before_misleading_guest_projection() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, acquisitions) = provided();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let _ = vm.refuse_host_command("original integer operation".into());
    let first = vm.execution_refusal.clone().unwrap();
    let guest = vm.native_integer_error(&ValueError::NotIntegerBytes(b"misleading guest".to_vec()));
    assert!(guest.result.resident_string_bytes().unwrap().is_empty());
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(vm.finish_host_execution(guest).unwrap_err(), first);
}

fn fixed_math(vm: &mut Vm, name: &str, args: &[Value]) -> tcl_runtime_api::Completion<Value> {
    let surface =
        tcl_registry::expr_surface::RuntimeExprSurface::for_profile(vm.native_execution_profile());
    let tcl_registry::expr_surface::MathFunctionCallTarget::FixedBuiltin(spec) =
        surface.math_function_call_target(name)
    else {
        panic!("fixed-table fixture must retain its actual selected registration");
    };
    vm.invoke_fixed_math_builtin(spec, args)
}

#[test]
fn original_math_dispatch_keeps_cached_positives_and_object_preserving_integer_results() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, acquisitions) = provided();
    let first = Value::int(2);
    let second = Value::int(3);
    let before = first.native_object_snapshot();
    let result = fixed_math(&mut vm, "pow", &[first.clone(), second]);
    assert_eq!(result.code, Code::Ok);
    assert_eq!(
        result.result.as_double().unwrap().to_bits(),
        8.0_f64.to_bits()
    );
    assert_eq!(first.native_object_snapshot(), before);
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());
    let mut modern = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let original = Value::new_native_string_bytes(b"0x10".as_slice());
    let result = modern
        .try_invoke_command("::tcl::mathfunc::entier", std::slice::from_ref(&original))
        .unwrap();
    assert_eq!(result.code, Code::Ok);
    assert!(result.result.is_same_object(&original));
    assert_eq!(
        result.result.resident_string_bytes().unwrap().as_ref(),
        b"0x10"
    );
    assert_eq!(result.result.integer_representation(), Some(16));
}

#[test]
fn original_math_dispatch_stops_formatter_refusal_before_later_operand_and_random_seed() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A supplied software Host formatter retires the genuine root at a reached stage.
    for name in ["pow", "srand"] {
        let (mut vm, formatter, _) = provided();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let original = Value::native_list_constructor(vec![value(&vm, i64::MIN)], protocol());
        let held = original.native_lifetime_lease();
        *formatter.retire_owner.borrow_mut() = Some(original);
        formatter.retire_at.set(Some("format"));
        let before_seed = vm.rand_seed;
        let later = Value::new_native_string_bytes(b"2".as_slice());
        let later_before = later.native_object_snapshot();
        let completion = if name == "srand" {
            fixed_math(&mut vm, name, std::slice::from_ref(held.value()))
        } else {
            let arguments = [
                held.value().native_lifetime_lease().into_value(),
                later.native_lifetime_lease().into_value(),
            ];
            fixed_math(&mut vm, name, &arguments)
        };
        let first = vm.execution_refusal.clone().unwrap();
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
        assert!(held.value().resident_string_bytes().is_none());
        assert!(held.value().native_scalar_cache().is_none());
        assert_eq!(later.native_object_snapshot(), later_before);
        assert_eq!(vm.rand_seed, before_seed);
        assert_eq!(formatter.calls.borrow().last().unwrap().0, "format");
        assert_eq!(vm.finish_host_execution(completion).unwrap_err(), first);
    }
}

#[test]
fn original_math_option_and_wide_workers_distinguish_withdrawal_from_genuine_absence() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for kind in ["int", "wide", "i128", "double", "bool", "big"] {
        let (mut vm, formatter, _) = provided();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let original = Value::int(6);
        let held = original.native_lifetime_lease();
        drop(original);
        let error = crate::expr::with_original_numeric_context(&mut vm, |context| match kind {
            "int" => held.value().as_int_with_stages(Some(context)).map(|_| ()),
            "wide" => held.value().as_wide_with_stages(Some(context)).map(|_| ()),
            "i128" => held.value().as_i128_with_stages(Some(context)).map(|_| ()),
            "double" => held
                .value()
                .as_double_with_stages(Some(context))
                .map(|_| ()),
            "bool" => held.value().as_bool_with_stages(Some(context)).map(|_| ()),
            "big" => {
                crate::expr::value_as_bigint_with_stages(held.value(), Some(context)).map(|_| ())
            }
            _ => unreachable!(),
        })
        .unwrap_err();
        let first = vm.execution_refusal.clone().unwrap();
        assert!(
            matches!(error, TclError::Host(crate::error::TclHostFailure::Execution(ref actual)) if actual == &first)
        );
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
        assert!(formatter.calls.borrow().is_empty());
        assert!(held.value().resident_string_bytes().is_none());
    }
    let (mut vm, _, _) = provided();
    let missing = Value::new_native_string_bytes(b"not_an_integer".as_slice());
    let (small, big) = crate::expr::with_original_numeric_context(&mut vm, |context| {
        Ok((
            missing.as_i128_with_stages(Some(context))?,
            crate::expr::value_as_bigint_with_stages(&missing, Some(context))?,
        ))
    })
    .unwrap();
    assert_eq!(small, None);
    assert_eq!(big, None);
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_math_grammar_remains_selected_when_formatter_constructs_another_vm() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software callback constructs a second VM; it supplies no external getter chronology.
    let (mut vm, formatter, _) = provided();
    let selected = vm.numeric_policy_context().numbers;
    assert_eq!(selected, tcl_dialect::NumberSyntax::Tcl84);
    let original = Value::native_list_constructor(vec![value(&vm, i64::MIN)], protocol());
    let later = Value::new_native_string_bytes(b"0d2".as_slice());
    let before = later.native_object_snapshot();
    formatter.format_other_vm.set(true);
    let completion = fixed_math(&mut vm, "pow", &[original, later.clone()]);
    assert_eq!(completion.code, Code::Error);
    assert_eq!(number::runtime_syntax(), tcl_dialect::NumberSyntax::Tcl90);
    assert_eq!(
        completion.result.resident_string_bytes().unwrap().as_ref(),
        b"expected floating-point number but got \"0d2\""
    );
    assert_eq!(later.native_object_snapshot(), before);
    assert!(vm.execution_refusal.is_none());
    // The independent pure adapter still observes its explicitly ambient grammar.
    assert_eq!(later.as_int().unwrap(), 2);
    number::set_runtime_syntax(selected);
}

#[test]
fn original_math_classification_boolean_and_big_workers_preserve_selected_guest_purposes() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let classification = vm
        .try_invoke_command("fpclassify", &[Value::string("NaN")])
        .unwrap();
    assert_eq!(classification.code, Code::Ok);
    assert_eq!(
        classification
            .result
            .resident_string_bytes()
            .unwrap()
            .as_ref(),
        b"nan"
    );
    let boolean = vm
        .try_invoke_command("::tcl::mathfunc::bool", &[Value::string("tru")])
        .unwrap();
    assert_eq!(boolean.code, Code::Ok);
    assert_eq!(boolean.result.integer_representation(), Some(1));
    let absolute = vm
        .try_invoke_command(
            "::tcl::mathfunc::abs",
            &[Value::string("-18446744073709551617")],
        )
        .unwrap();
    assert_eq!(absolute.code, Code::Ok);
    assert_eq!(
        absolute.result.resident_string_bytes().unwrap().as_ref(),
        b"18446744073709551617"
    );
    let failed = vm
        .try_invoke_command("::tcl::mathfunc::double", &[Value::string("not_a_number")])
        .unwrap();
    assert_eq!(failed.code, Code::Error);
    assert!(vm.execution_refusal.is_none());
}

fn original_read_channel(vm: &mut Vm) -> String {
    let access =
        tcl_cmd_core::channel::resolve_open_access_mode(vm.runtime_version(), "r").unwrap();
    let config = tcl_cmd_core::channel::ChannelConfig::for_open_access(
        vm.runtime_version(),
        vm.system_encoding(),
        access,
    );
    vm.add_channel(crate::cmd_chan::Channel::Read {
        data: b"abcdef".to_vec(),
        pos: 0,
        config,
        path: Some(Rc::from(b"original-owned-channel".as_slice())),
    })
}

fn original_channel_cursor(vm: &Vm, name: &str) -> usize {
    let crate::cmd_chan::Channel::Read { pos, .. } = vm.channel(name).unwrap() else {
        panic!("actual owned read channel required");
    };
    *pos
}

#[test]
fn original_io_and_debug_resident_arguments_keep_cursor_flag_and_no_formatter_queries() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software command adapters only; no public C/Jim default-routing assertion.
    let (mut vm, formatter, acquisitions) = provided();
    let channel = original_read_channel(&mut vm);
    let result = vm
        .try_invoke_command(
            "read",
            &[Value::string(channel.as_str()), Value::string("2")],
        )
        .unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(
        result.result.resident_string_bytes().unwrap().as_ref(),
        b"ab"
    );
    assert_eq!(original_channel_cursor(&vm, &channel), 2);
    let result = vm
        .try_invoke_command("seek", &[Value::string(channel.as_str()), Value::int(1)])
        .unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(original_channel_cursor(&vm, &channel), 1);
    let result = vm
        .try_invoke_command(
            "seek",
            &[
                Value::string(channel.as_str()),
                Value::int(2),
                Value::string("current"),
            ],
        )
        .unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(original_channel_cursor(&vm, &channel), 3);
    let mut debug = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.5"));
    debug.set_host(Rc::new(FormatterHost {
        actual: debug.host_rc(),
        formatter: Rc::clone(&formatter),
        acquisitions: Rc::clone(&acquisitions),
    }));
    let result = debug
        .debug_apply(
            &[Value::string("-frame"), Value::bool(true)],
            b"fixture -frame bool",
        )
        .unwrap();
    assert_eq!(result.integer_representation(), Some(1));
    assert!(debug.debug_frame);
    assert!(debug.execution_refusal.is_none());
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());

    let mut modern = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let formatter = Rc::new(Formatter::default());
    let acquisitions = Rc::new(Cell::new(0));
    modern.set_host(Rc::new(FormatterHost {
        actual: modern.host_rc(),
        formatter: Rc::clone(&formatter),
        acquisitions: Rc::clone(&acquisitions),
    }));
    let data = Value::byte_array(b"abc".to_vec());
    let result = modern
        .try_invoke_command(
            "binary",
            &[
                Value::string("encode"),
                Value::string("base64"),
                Value::string("-maxlen"),
                Value::int(2),
                data.native_lifetime_lease().into_value(),
            ],
        )
        .unwrap();
    assert_eq!(result.code, Code::Ok);
    assert_eq!(result.result.string_bytes().as_ref(), b"YW\nJj");
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(modern.execution_refusal.is_none());
}

#[test]
fn original_io_formatter_withdrawal_stops_read_and_seek_before_mutation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The actual supplied software formatter retires the original borrowed root.
    for purpose in ["read", "seek", "seek-current"] {
        let (mut vm, formatter, _) = provided();
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let channel = original_read_channel(&mut vm);
        let root = Value::native_list_constructor(vec![value(&vm, i64::MIN)], protocol());
        let held = root.native_lifetime_lease();
        *formatter.retire_owner.borrow_mut() = Some(root);
        formatter.retire_at.set(Some("format"));
        let mut arguments = vec![
            Value::string(channel.as_str()),
            held.value().native_lifetime_lease().into_value(),
        ];
        if purpose == "seek-current" {
            arguments.push(Value::string("current"));
        }
        let first = vm
            .try_invoke_command(if purpose == "read" { "read" } else { "seek" }, &arguments)
            .unwrap_err();
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        assert_eq!(original_channel_cursor(&vm, &channel), 0);
        assert!(!vm.debug_frame);
        assert!(held.value().resident_string_bytes().is_none());
        assert!(held.value().native_scalar_cache().is_none());
        assert_eq!(formatter.calls.borrow().last().unwrap().0, "format");
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
    }
}

#[test]
fn original_binary_maxlen_retired_header_refuses_before_guest_line_length_and_data_conversion() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine entered VM/header refusal; no arbitrary binary extension chronology.
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let count = Value::int(2);
    let held = count.native_lifetime_lease();
    drop(count);
    let data = Value::byte_array(b"abc".to_vec());
    let before = data.native_object_snapshot();
    let arguments = [
        Value::string("encode"),
        Value::string("base64"),
        Value::string("-maxlen"),
        held.value().native_lifetime_lease().into_value(),
        data.native_lifetime_lease().into_value(),
    ];
    let first = vm.try_invoke_command("binary", &arguments).unwrap_err();
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    assert_eq!(data.native_object_snapshot(), before);
    assert!(held.value().resident_string_bytes().is_none());
    assert_eq!(
        first,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object header"
            )
        )
    );
}

#[test]
fn original_numeric_command_scope_stops_host_aba_before_backend_and_retires_unpublished_result() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual set_host changes in the software worker; no Native I/O callback grant.
    struct ReachedResult(Box<dyn Fn()>);
    impl Drop for ReachedResult {
        fn drop(&mut self) {
            (self.0)();
        }
    }
    let (mut vm, _, _) = provided();
    let outer = NativeOperationScope::enter_native(&mut vm).unwrap();
    let initial = vm.host_rc();
    let independent =
        crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
    let replacement = independent.host_rc();
    let changed_backend = Rc::new(Cell::new(false));
    let retired_under_original = Rc::new(Cell::new(false));
    let owner = Rc::downgrade(&vm.native_operation_context);
    let failure = crate::expr::with_original_numeric_command(
        &mut vm,
        tcl_cmd_core::CmdError::from_execution_refusal,
        |vm, original| {
            assert!(original.original.is_some());
            original.current(vm).unwrap();
            vm.set_host(Rc::clone(&replacement));
            vm.set_host(Rc::clone(&initial));
            assert!(original.current(vm).is_err());
            if original.current(vm).is_ok() {
                changed_backend.set(true);
            }
            let retired = Rc::clone(&retired_under_original);
            Ok(ReachedResult(Box::new(move || {
                retired.set(owner.upgrade().unwrap().has_entered_operation());
            })))
        },
    )
    .err()
    .unwrap();
    let first = failure.native_execution_refusal().unwrap();
    assert_eq!(outer.check_native(&mut vm).unwrap_err(), *first);
    assert_eq!(vm.execution_refusal.as_ref(), Some(first));
    assert!(!changed_backend.get());
    assert!(retired_under_original.get());
}

#[test]
fn original_numeric_command_prior_host_cause_remains_typed_before_guest_defaults() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, acquisitions) = provided();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let channel = original_read_channel(&mut vm);
    let prior = vm.refuse_host_command("original command receipt declined".to_owned());
    let first = vm.execution_refusal.clone().unwrap();
    for name in ["read", "seek"] {
        let error = vm
            .try_invoke_command(
                name,
                &[Value::string(channel.as_str()), Value::string("bad")],
            )
            .unwrap_err();
        assert_eq!(error, first);
        assert_eq!(original_channel_cursor(&vm, &channel), 0);
    }
    let error = vm
        .debug_apply(
            &[Value::string("-frame"), Value::string("bad")],
            b"fixture -frame bool",
        )
        .unwrap_err();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert!(!vm.debug_frame);
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert_eq!(vm.finish_host_execution(prior).unwrap_err(), first);
}

#[test]
fn original_debug_retired_header_stops_supported_boolean_before_flag_or_guest_error() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.5"));
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let operand = Value::bool(true);
    let held = operand.native_lifetime_lease();
    drop(operand);
    let error = vm
        .debug_apply(
            &[
                Value::string("-frame"),
                held.value().native_lifetime_lease().into_value(),
            ],
            b"fixture -frame bool",
        )
        .unwrap_err();
    let first = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "retired native object header",
        ),
    );
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert!(!vm.debug_frame);
}

fn original_numeric_public_error(completion: crate::Completion<Value>) -> (Rc<[u8]>, Rc<[u8]>) {
    assert_eq!(completion.code, Code::Error);
    let error = crate::TclError::from_completion(completion);
    (
        error.message_bytes().unwrap(),
        error.error_code_bytes().unwrap().unwrap(),
    )
}

#[test]
fn original_public_read_counts_preserve_selected_errors_and_untouched_cursor() {
    // naming.numeric.io-debug-public-conversion-and-side-effect-order
    // docs/design/analysis/name-resolution-proofs/numeric-io-debug-public-conversion-and-side-effect-order.md
    // Native628 proves public ASCII results/code/cursor only; these are source/API controls.
    for (dialect, bad, negative) in [
        (
            "tcl8.4",
            b"bad argument \"bad\": should be \"nonewline\"".as_slice(),
            b"bad argument \"-1\": should be \"nonewline\"".as_slice(),
        ),
        (
            "tcl8.5",
            b"expected integer but got \"bad\"".as_slice(),
            b"expected non-negative integer but got \"-1\"".as_slice(),
        ),
        (
            "tcl8.6",
            b"expected non-negative integer but got \"bad\"".as_slice(),
            b"expected non-negative integer but got \"-1\"".as_slice(),
        ),
        (
            "tcl9.0",
            b"expected non-negative integer but got \"bad\"".as_slice(),
            b"expected non-negative integer but got \"-1\"".as_slice(),
        ),
        (
            "tcl9.1",
            b"expected non-negative integer but got \"bad\"".as_slice(),
            b"expected non-negative integer but got \"-1\"".as_slice(),
        ),
    ] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(dialect));
        let channel = original_read_channel(&mut vm);
        for (count, expected) in [("bad", bad), ("-1", negative)] {
            let completion = vm
                .try_invoke_command(
                    "read",
                    &[Value::string(channel.as_str()), Value::string(count)],
                )
                .unwrap();
            let (message, code) = original_numeric_public_error(completion);
            assert_eq!(message.as_ref(), expected, "{dialect} {count}");
            assert_eq!(
                code.as_ref(),
                if dialect == "tcl8.4" {
                    b"NONE".as_slice()
                } else {
                    b"TCL VALUE NUMBER".as_slice()
                }
            );
            assert_eq!(original_channel_cursor(&vm, &channel), 0);
            assert!(vm.execution_refusal.is_none());
        }
        let result = vm
            .try_invoke_command("read", &[Value::string(channel.as_str()), Value::int(2)])
            .unwrap();
        assert_eq!(result.code, Code::Ok);
        assert_eq!(
            result.result.resident_string_bytes().unwrap().as_ref(),
            b"ab"
        );
        assert_eq!(original_channel_cursor(&vm, &channel), 2);
    }
}

#[test]
fn original_public_seek_errors_preserve_conversion_order_and_negative_target_position() {
    // naming.numeric.io-debug-public-conversion-and-side-effect-order
    // docs/design/analysis/name-resolution-proofs/numeric-io-debug-public-conversion-and-side-effect-order.md
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let profile = crate::environment::profile_for_dialect(dialect);
        let mut vm = crate::native_fixture::core(profile);
        let channel = original_read_channel(&mut vm);
        let invalid = vm
            .try_invoke_command(
                "seek",
                &[
                    Value::string(channel.as_str()),
                    Value::string("bad"),
                    Value::string("invalid-origin"),
                ],
            )
            .unwrap();
        let (message, _) = original_numeric_public_error(invalid);
        assert_eq!(
            message.as_ref(),
            if dialect == "jim" {
                format!("wrong # args: should be \"{channel} seek offset ?start|current|end\"")
                    .as_bytes()
                    .to_vec()
            } else {
                b"expected integer but got \"bad\"".to_vec()
            }
        );
        assert_eq!(original_channel_cursor(&vm, &channel), 0);
        let result = vm
            .try_invoke_command("seek", &[Value::string(channel.as_str()), Value::int(3)])
            .unwrap();
        assert_eq!(result.code, Code::Ok);
        let failed = vm
            .try_invoke_command("seek", &[Value::string(channel.as_str()), Value::int(-1)])
            .unwrap();
        let (message, code) = original_numeric_public_error(failed);
        if dialect == "jim" {
            assert_eq!(
                message.as_ref(),
                b"original-owned-channel: Invalid argument"
            );
            assert_eq!(code.as_ref(), b"NONE");
        } else {
            assert_eq!(
                message.as_ref(),
                format!("error during seek on \"{channel}\": invalid argument").as_bytes()
            );
            assert_eq!(code.as_ref(), b"POSIX EINVAL {invalid argument}");
        }
        assert_eq!(original_channel_cursor(&vm, &channel), 3);
        let result = vm
            .try_invoke_command(
                "seek",
                &[
                    Value::string(channel.as_str()),
                    Value::int(-1),
                    Value::string("current"),
                ],
            )
            .unwrap();
        assert_eq!(result.code, Code::Ok);
        assert_eq!(original_channel_cursor(&vm, &channel), 2);
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_public_binary_line_length_uses_actual_primitive_and_negative_code() {
    // naming.numeric.io-debug-public-conversion-and-side-effect-order
    // docs/design/analysis/name-resolution-proofs/numeric-io-debug-public-conversion-and-side-effect-order.md
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(dialect));
        for (input, expected) in [
            ("bad", b"expected integer but got \"bad\"".as_slice()),
            ("-1", b"line length out of range".as_slice()),
        ] {
            let completion = vm
                .try_invoke_command(
                    "binary",
                    &[
                        Value::string("encode"),
                        Value::string("base64"),
                        Value::string("-maxlen"),
                        Value::string(input),
                        Value::byte_array(b"abc".to_vec()),
                    ],
                )
                .unwrap();
            let (message, code) = original_numeric_public_error(completion);
            assert_eq!(message.as_ref(), expected);
            let expected_code = if input == "-1" {
                b"TCL BINARY ENCODE LINE_LENGTH".as_slice()
            } else if dialect == "tcl8.6" {
                b"TCL VALUE INTEGER".as_slice()
            } else {
                b"TCL VALUE NUMBER".as_slice()
            };
            assert_eq!(code.as_ref(), expected_code);
            assert!(vm.execution_refusal.is_none());
        }
    }
}

#[test]
fn original_public_debug_bad_boolean_refuses_before_flag_and_numeric_boolean_succeeds() {
    // naming.numeric.io-debug-public-conversion-and-side-effect-order
    // docs/design/analysis/name-resolution-proofs/numeric-io-debug-public-conversion-and-side-effect-order.md
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(dialect));
        let error = vm
            .debug_apply(
                &[Value::string("-frame"), Value::string("bad")],
                b"fixture -frame bool",
            )
            .unwrap_err();
        let completion = crate::command::completion_from_cmd_error(&mut vm, error);
        let (message, code) = original_numeric_public_error(completion);
        assert_eq!(message.as_ref(), b"expected boolean value but got \"bad\"");
        assert_eq!(code.as_ref(), b"TCL VALUE NUMBER");
        assert!(!vm.debug_frame);
        let result = vm
            .debug_apply(
                &[Value::string("-frame"), Value::string("1.5")],
                b"fixture -frame bool",
            )
            .unwrap();
        assert_eq!(result.integer_representation(), Some(1));
        assert!(vm.debug_frame);
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_open_channel_retains_the_actual_checked_path_for_jim_seek_error() {
    // naming.numeric.io-debug-public-conversion-and-side-effect-order
    // docs/design/analysis/name-resolution-proofs/numeric-io-debug-public-conversion-and-side-effect-order.md
    // Software channel integration with an actual owned file, not opaque I/O callbacks.
    let path = std::env::temp_dir().join(format!(
        "tcl-vm-original-open-seek-{}.txt",
        std::process::id()
    ));
    std::fs::write(&path, b"abcdef").unwrap();
    let written = path.to_str().unwrap();
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    let opened = vm
        .try_invoke_command("open", &[Value::string(written)])
        .unwrap();
    assert_eq!(opened.code, Code::Ok);
    let id = opened.result.try_to_str().unwrap();
    let crate::cmd_chan::Channel::Read { path: retained, .. } = vm.channel(id.as_ref()).unwrap()
    else {
        panic!("actual opened read channel required");
    };
    assert_eq!(retained.as_deref(), Some(written.as_bytes()));
    let failed = vm
        .try_invoke_command("seek", &[Value::string(id.as_ref()), Value::int(-1)])
        .unwrap();
    let (message, code) = original_numeric_public_error(failed);
    let mut expected = written.as_bytes().to_vec();
    expected.extend_from_slice(b": Invalid argument");
    assert_eq!(message.as_ref(), expected.as_slice());
    assert_eq!(code.as_ref(), b"NONE");
    assert_eq!(original_channel_cursor(&vm, id.as_ref()), 0);
    assert_eq!(
        vm.try_invoke_command("close", &[Value::string(id.as_ref())])
            .unwrap()
            .code,
        Code::Ok
    );
    std::fs::remove_file(&path).unwrap();
}

#[path = "native_string_stage_tests/original_list_membership_tests.rs"]
mod original_list_membership_tests;

#[path = "native_string_stage_tests/original_static_lookup_tests.rs"]
mod original_static_lookup_tests;

#[path = "native_string_stage_tests/callback_unicode_words_tests.rs"]
mod callback_unicode_words_tests;

#[path = "native_string_stage_tests/original_authored_operand_tests.rs"]
mod original_authored_operand_tests;
