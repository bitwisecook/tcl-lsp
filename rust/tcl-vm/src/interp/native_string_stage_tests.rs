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
