// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed-input software controls for original consumer contexts.

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
    withdraw: RefCell<Option<Rc<RefCell<super::super::native_name_world::NativeNameWorld>>>>,
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
        if let Some(world) = self.withdraw.borrow_mut().take() {
            world.borrow_mut().retire();
        }
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
fn original_name_consumer_keeps_restored_nested_frame_and_namespace_contexts() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let original = Value::new_native_string_bytes(b"original\0\xff".as_slice());
    let before = original.native_object_snapshot();
    vm.declare_namespace("nested");
    let namespace = vm
        .namespace_object_lookup(&Value::string("::nested"))
        .unwrap()
        .unwrap();
    vm.push_ns_eval_token_frame(namespace, vec![Value::string("namespace")]);
    let nested = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    assert_eq!(
        nested.read(&mut vm, &original).unwrap().as_ref(),
        b"original\0\xff"
    );
    nested.finish(&mut vm).unwrap();
    vm.pop_ns();
    vm.pop_call_frame();
    assert_eq!(
        scope.read(&mut vm, &original).unwrap().as_ref(),
        b"original\0\xff"
    );
    scope.finish(&mut vm).unwrap();
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(original.native_object_reference_count(), 1);
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
}

#[test]
fn original_name_consumer_refuses_reborn_same_level_before_the_next_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    vm.declare_namespace("nested");
    let namespace = vm
        .namespace_object_lookup(&Value::string("::nested"))
        .unwrap()
        .unwrap();
    vm.push_ns_eval_token_frame(namespace, vec![Value::string("namespace")]);
    let original_frame = vm.retained_frame_identity(false);
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    vm.pop_ns();
    vm.pop_call_frame();
    vm.push_ns_eval_token_frame(namespace, vec![Value::string("namespace")]);
    assert!(!Rc::ptr_eq(
        &original_frame,
        &vm.retained_frame_identity(false)
    ));
    let original = numeric(&vm, i64::MIN);
    let before = original.native_object_snapshot();
    let error = scope.read(&mut vm, &original).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(
        scope
            .finish(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
    assert_eq!(original.native_object_snapshot(), before);
    assert!(original.resident_string_bytes().is_none());
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    vm.pop_ns();
    vm.pop_call_frame();
}

#[test]
fn original_name_consumer_reached_formatter_withdraws_the_table_before_later_operands() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    let result = prior(&mut vm);
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let first = numeric(&vm, i64::MIN);
    let later = numeric(&vm, i64::MIN + 1);
    *formatter.withdraw.borrow_mut() = Some(Rc::clone(&vm.name_world));
    let error = scope.read(&mut vm, &first).unwrap_err();
    let cause = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&cause));
    assert_eq!(
        first.resident_string_bytes().unwrap().as_ref(),
        i64::MIN.to_string().as_bytes()
    );
    assert_eq!(first.native_object_reference_count(), 1);
    assert_eq!(
        scope
            .read(&mut vm, &later)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&cause)
    );
    assert!(later.resident_string_bytes().is_none());
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|stage| **stage == "format")
            .count(),
        1
    );
    assert_eq!(queries.get(), 1);
    let _ = crate::command::completion_from_cmd_error(&mut vm, error);
    assert_eq!(vm.execution_refusal.as_ref(), Some(&cause));
    assert_eq!(
        scope
            .finish(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&cause)
    );
    unchanged_result(&vm, &result);
}

#[test]
fn original_name_consumer_host_change_and_restore_keeps_the_first_context_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    let result = prior(&mut vm);
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let original = Value::new_native_string_bytes(b"name".as_slice());
    let before = original.native_object_snapshot();
    scope.read(&mut vm, &original).unwrap();
    let host = vm.host_rc();
    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
    vm.set_host(host);
    let error = scope.read(&mut vm, &original).unwrap_err();
    let cause = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&cause));
    assert_eq!(
        scope
            .finish(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&cause)
    );
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    unchanged_result(&vm, &result);
}

#[test]
fn original_name_consumer_pending_typed_cause_prevents_later_header_work() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    let result = prior(&mut vm);
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let original = numeric(&vm, i64::MIN);
    let before = original.native_object_snapshot();
    let _ = vm.refuse_host_command("FIRST_ORIGINAL_NAME_CAUSE".into());
    let first = vm.execution_refusal.clone().unwrap();
    let error = scope.read(&mut vm, &original).unwrap_err();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    let _ = crate::command::completion_from_cmd_error(&mut vm, error);
    assert_eq!(
        scope
            .finish(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    unchanged_result(&vm, &result);
}

#[test]
fn original_name_consumer_selected_namespace_does_not_rebind_from_its_old_spelling() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    vm.declare_namespace("selected");
    let written = Value::string("::selected");
    let selected = vm.namespace_object_lookup(&written).unwrap().unwrap();
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    assert!(
        vm.delete_namespace_path(&tcl_core_types::ByteNamespacePath::from_segments([
            b"selected".as_slice()
        ]))
    );
    vm.declare_namespace("selected");
    let replacement = vm
        .namespace_object_lookup(&Value::string("::selected"))
        .unwrap()
        .unwrap();
    assert_ne!(selected, replacement);
    let current = vm.native_namespace_token_has_owner(selected);
    let error = scope
        .require(&mut vm, current, "selected original namespace target")
        .unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert!(vm.native_namespace_token_has_owner(replacement));
    assert_eq!(
        scope
            .finish(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
}

#[test]
fn original_name_consumer_info_boundary_preserves_the_reached_getter_cause_and_result_owner() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, _) = provided();
    let result = prior(&mut vm);
    let original = numeric(&vm, i64::MIN);
    formatter.fail.set(true);
    let crate::Command::Builtin(worker) = vm.lookup_command("info").unwrap() else {
        panic!("actual C8.4 info worker");
    };
    let completion = worker(
        &mut vm,
        &[
            Value::new_native_string_bytes(b"functions".as_slice()),
            original.clone(),
        ],
    );
    assert_eq!(completion.code, crate::Code::Error);
    let first = vm.execution_refusal.clone().unwrap();
    assert!(!matches!(
        first,
        NativeExecutionError::HostCommandRefusal(_)
    ));
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|stage| **stage == "format")
            .count(),
        1
    );
    assert!(original.resident_string_bytes().is_none());
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    unchanged_result(&vm, &result);
}
