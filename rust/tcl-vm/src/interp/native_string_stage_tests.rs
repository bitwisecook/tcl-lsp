// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software formatter callback boundaries, with actual original VM and Value owners.

use super::*;
use std::cell::{Cell, RefCell};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_syntax::native_string::NativeStringProtocol;
use tcl_syntax::scalar_getter::NativeScalarCache;

type Call = (&'static str, Option<(NativeIntegerKind, i64)>);
#[derive(Default)]
struct Formatter {
    calls: RefCell<Vec<Call>>,
    fail: Cell<bool>,
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
    let original = Value::new_native_string_bytes(b"already resident");
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
