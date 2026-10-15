// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for actual original-name String ingress and first-cause transport.

use super::*;
use std::cell::{Cell, RefCell};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{
    native_string::NativeStringProtocol, number::Number, raw_string::NativeValueAccessRefusal,
    scalar_getter::NativeScalarCache,
};

#[derive(Default)]
struct Formatter {
    calls: RefCell<Vec<&'static str>>,
    retire_at: Cell<Option<&'static str>>,
    retired: RefCell<Option<Value>>,
    observed: RefCell<Option<crate::NativeObjectLifetimeLease>>,
    counts: RefCell<Vec<usize>>,
    fail: Cell<bool>,
}
impl Formatter {
    fn reached(&self, stage: &'static str) {
        self.calls.borrow_mut().push(stage);
        if let Some(original) = self.observed.borrow().as_ref() {
            self.counts
                .borrow_mut()
                .push(original.value().native_object_reference_count());
        }
        if self.retire_at.get() == Some(stage) {
            let retired = self.retired.borrow_mut().take();
            drop(retired);
        }
    }
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        self.reached("build");
        NativeIntegerFormatterBuild {
            version: [8, 4, 20, 0],
            sha256: [7; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        _: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        self.reached("format");
        if self.fail.get() {
            return Err(NativeIntegerFormatterUnavailable::Operation);
        }
        Ok(value.to_string().into_bytes())
    }
}
struct FormatterHost {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<Formatter>,
    queries: Rc<Cell<usize>>,
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
        self.queries.set(self.queries.get() + 1);
        Some(&*self.formatter)
    }
}
fn provided() -> (Vm, Rc<Formatter>, Rc<Cell<usize>>) {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
    let formatter = Rc::new(Formatter::default());
    let queries = Rc::new(Cell::new(0));
    vm.set_host(Rc::new(FormatterHost {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
        queries: Rc::clone(&queries),
    }));
    (vm, formatter, queries)
}
fn numeric(vm: &Vm, value: i64) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(value)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}
fn prior(vm: &mut Vm) -> crate::NativeObjectLifetimeLease {
    vm.adopt_native_interp_result(Value::new_native_string_bytes(b"BEFORE".as_slice()))
        .unwrap()
}
fn unchanged_result(vm: &Vm, original: &crate::NativeObjectLifetimeLease) {
    assert!(
        vm.with_native_interp_result(|value| value.is_same_object(original.value()))
            .unwrap()
    );
    assert_eq!(
        original.value().resident_string_bytes().unwrap().as_ref(),
        b"BEFORE"
    );
    assert_eq!(original.value().native_object_reference_count(), 1);
}

#[test]
fn original_name_resident_string_keeps_bytes_header_and_reference_roles_without_formatter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software currentness control; resident bytes do not issue a new name or Native purpose.
    let (mut vm, formatter, queries) = provided();
    let original = Value::new_native_string_bytes(b"a\0\xff::b".as_slice());
    let before = original.native_object_snapshot();
    let count = original.native_object_reference_count();
    let bytes = vm.native_name_operand_bytes_checked(&original).unwrap();
    assert_eq!(bytes.as_ref(), b"a\0\xff::b");
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(original.native_object_reference_count(), count);
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());
    let mut authored = Vm::new();
    let resident = Value::string("authored::name");
    assert_eq!(
        authored
            .native_name_operand_bytes_checked(&resident)
            .unwrap()
            .as_ref(),
        b"authored::name"
    );
    assert!(authored.execution_refusal.is_none());
}

#[test]
fn original_name_selected_string_uses_actual_compound_formatter_without_reference_donation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied Host formatter stages are software observations, not external Tcl header conclusions.
    let (mut vm, formatter, queries) = provided();
    let root = Value::native_list_constructor(
        vec![numeric(&vm, i64::MIN), numeric(&vm, i64::MIN + 1)],
        NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
    );
    let members = root
        .native_list_backing_in(NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4))
        .unwrap()
        .unwrap();
    let first = members.elements().unwrap()[0].native_lifetime_lease();
    let count = first.value().native_object_reference_count();
    *formatter.observed.borrow_mut() = Some(first.clone());
    assert!(root.resident_string_bytes().is_none());
    let bytes = vm.native_name_operand_bytes_checked(&root).unwrap();
    assert_eq!(
        bytes.as_ref(),
        format!("{} {}", i64::MIN, i64::MIN + 1).as_bytes()
    );
    assert_eq!(queries.get(), 1);
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|stage| **stage == "format")
            .count(),
        2
    );
    assert!(
        formatter
            .counts
            .borrow()
            .iter()
            .all(|value| *value == count)
    );
    assert_eq!(first.value().native_object_reference_count(), count);
    assert_eq!(root.native_object_reference_count(), 1);
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_name_caller_check_stops_a_compound_getter_after_reentrant_frame_change() {
    for change in [false, true] {
        let (mut vm, formatter, _) = provided();
        vm.push_call_frame(Some("name_getter_caller_control".into()), Vec::new());
        let scope =
            super::super::native_name_consumers::OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let root = Value::native_list_constructor(
            vec![numeric(&vm, i64::MIN), numeric(&vm, i64::MIN + 1)],
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
        );
        let changed = Cell::new(false);
        let reached = vm.native_name_operand_bytes_checked_using(&root, &|vm| {
            if change && formatter.calls.borrow().contains(&"format") && !changed.replace(true) {
                vm.push_call_frame(Some("reentrant_name_getter_frame".into()), Vec::new());
            }
            scope.check(vm)
        });
        assert_eq!(reached.is_err(), change);
        assert_eq!(
            formatter
                .calls
                .borrow()
                .iter()
                .filter(|stage| **stage == "format")
                .count(),
            if change { 1 } else { 2 }
        );
        assert_eq!(root.resident_string_bytes().is_none(), change);
        if change {
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(
                reached.unwrap_err().native_execution_refusal(),
                Some(&first)
            );
            vm.pop_call_frame();
            assert!(scope.check(&mut vm).is_err());
            assert_eq!(vm.execution_refusal, Some(first));
        } else {
            scope.finish(&mut vm).unwrap();
            assert!(vm.execution_refusal.is_none());
        }
        vm.pop_call_frame();
    }
}

#[test]
fn original_name_formatter_withdrawal_retains_exact_first_cause_and_stops_later_children() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software formatter callbacks withdraw the original root; allocation leases add no Native hold.
    for stage in ["build", "format", "failed-format"] {
        let (mut vm, formatter, queries) = provided();
        let result = prior(&mut vm);
        formatter
            .retire_at
            .set(Some(if stage == "build" { "build" } else { "format" }));
        formatter.fail.set(stage == "failed-format");
        let root = Value::native_list_constructor(
            vec![numeric(&vm, i64::MIN), numeric(&vm, i64::MIN + 1)],
            NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
        );
        let original = root.native_lifetime_lease();
        *formatter.observed.borrow_mut() = Some(root.native_lifetime_lease());
        *formatter.retired.borrow_mut() = Some(root);
        let error = vm
            .native_name_operand_bytes_checked(original.value())
            .unwrap_err();
        let first = NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("retired native object header"),
        );
        assert_eq!(error.native_execution_refusal(), Some(&first));
        assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
        assert_eq!(queries.get(), 1);
        assert!(!original.value().native_object_is_live());
        assert!(original.value().resident_string_bytes().is_none());
        assert!(formatter.counts.borrow().iter().all(|count| *count == 1));
        assert_eq!(
            formatter
                .calls
                .borrow()
                .iter()
                .filter(|value| **value == "format")
                .count(),
            usize::from(stage != "build")
        );
        unchanged_result(&vm, &result);
    }
}

#[test]
fn original_name_inherits_context_before_resident_read_after_actual_host_change_and_restore() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // This genuine Scope invalidation is independent of ordinary name/table mutations.
    let (mut vm, formatter, queries) = provided();
    let result = prior(&mut vm);
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let original = Value::new_native_string_bytes(b"name".as_slice());
    let before = original.native_object_snapshot();
    assert_eq!(
        vm.native_name_operand_bytes_checked(&original)
            .unwrap()
            .as_ref(),
        b"name"
    );
    let host = vm.host_rc();
    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
    vm.set_host(host);
    let error = vm.native_name_operand_bytes_checked(&original).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(original.native_object_reference_count(), 1);
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    unchanged_result(&vm, &result);
}

#[test]
fn original_name_pending_host_refusal_precedes_resident_projection_and_preserves_result() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A genuine prior typed Host refusal cannot become a Guest diagnostic or an apparent name miss.
    let (mut vm, formatter, queries) = provided();
    let result = prior(&mut vm);
    let original = Value::new_native_string_bytes(b"name".as_slice());
    let before = original.native_object_snapshot();
    let _unwind = vm.refuse_host_command("FIRST_NAME_HOST".into());
    let first = vm.execution_refusal.clone().unwrap();
    let error = vm.native_name_operand_bytes_checked(&original).unwrap_err();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(original.native_object_reference_count(), 1);
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    unchanged_result(&vm, &result);
}
