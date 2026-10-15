// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fixed-input software controls for original consumer contexts.

use super::*;
use std::cell::{Cell, RefCell};
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_runtime_api::NativeExecutionError;
use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};

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
    let crate::command::Command::Builtin(worker) = vm.lookup_command("info").unwrap() else {
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

#[test]
fn original_jim_namespace_eval_uses_its_actual_holder_without_c_tree_registration() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jim"));
    let outer =
        super::super::native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let completion = vm.eval_source("namespace eval originalJimHolder {set value HELD}; namespace eval originalJimHolder {set value}").unwrap();
    assert_eq!(completion.code, tcl_runtime_api::Code::Ok);
    assert_eq!(
        completion.result.resident_string_bytes().unwrap().as_ref(),
        b"HELD"
    );
    let namespace = vm
        .namespace_object_lookup(&Value::new_native_string_bytes(
            b"originalJimHolder".as_slice(),
        ))
        .unwrap()
        .unwrap();
    assert!(vm.native_namespace_token_has_owner(namespace));
    let world = vm.name_world.borrow();
    assert!(world.jim_namespace_objects.contains_key(&namespace));
    assert!(
        !world
            .namespaces
            .contains(&world.ns_arena[namespace.0 as usize])
    );
    drop(world);
    outer.check(&mut vm).unwrap();
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_jim_namespace_holder_withdrawal_refuses_invalid_and_retired_births() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    let namespace = vm.intern_jim_namespace_object(b"originalJimHolder");
    assert!(vm.native_namespace_token_has_owner(namespace));
    assert!(!vm.native_namespace_token_has_owner(tcl_core_types::NsId(u32::MAX)));
    let held_world = Rc::clone(&vm.name_world);
    vm.name_world
        .borrow_mut()
        .jim_namespace_objects
        .remove(&namespace);
    assert!(!vm.native_namespace_token_has_owner(namespace));
    let replacement = vm.intern_jim_namespace_object(b"originalJimHolder");
    assert_ne!(namespace, replacement);
    assert!(!vm.native_namespace_token_has_owner(namespace));
    assert!(vm.native_namespace_token_has_owner(replacement));
    held_world.borrow_mut().retire();
    assert!(!vm.native_namespace_token_has_owner(replacement));
}

#[test]
fn original_jim_namespace_consumer_rejects_foreign_world_with_same_numeric_slot() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    let foreign = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let original_world = Rc::clone(&vm.name_world);
    vm.name_world = Rc::clone(&foreign.name_world);
    // Root slots genuinely coincide; the retained owner, not the integer,
    // distinguishes the two current command/namespace worlds.
    assert!(vm.native_namespace_token_has_owner(tcl_core_types::ROOT_NS));
    let original = Value::new_native_string_bytes(b"QUIET".as_slice());
    let before = original.native_object_snapshot();
    let error = scope.read(&mut vm, &original).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    vm.name_world = original_world;
    assert_eq!(
        scope
            .finish(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(original.native_object_reference_count(), 1);
}

#[test]
fn original_variable_name_array_worker_retains_formatter_cause_before_later_operands() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software worker/callback transport; no external Tcl formatter or array chronology grant.
    let (mut vm, formatter, queries) = provided();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let result = prior(&mut vm);
    let original = numeric(&vm, i64::MIN);
    let later = Value::native_list_constructor(
        vec![
            numeric(&vm, i64::MIN + 1),
            Value::new_native_string_bytes(b"value".as_slice()),
        ],
        tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_4),
    );
    let original_count = original.native_object_reference_count();
    let later_before = later.native_object_snapshot();
    formatter.fail.set(true);
    let crate::command::Command::Builtin(worker) = vm.lookup_command("array").unwrap() else {
        panic!("actual C8.4 array worker");
    };
    let args = [
        Value::new_native_string_bytes(b"set".as_slice()),
        original,
        later,
    ];
    let completion = worker(&mut vm, &args);
    assert_eq!(completion.code, crate::Code::Error);
    let first = vm.execution_refusal.clone().unwrap();
    assert!(!matches!(
        first,
        NativeExecutionError::HostCommandRefusal(_)
    ));
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(queries.get(), 1);
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|stage| **stage == "format")
            .count(),
        1
    );
    assert!(args[1].resident_string_bytes().is_none());
    assert_eq!(args[1].native_object_reference_count(), original_count);
    assert_eq!(args[2].native_object_snapshot(), later_before);
    unchanged_result(&vm, &result);
}

#[test]
fn original_variable_name_dictionary_worker_refuses_retired_header_before_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // A real withdrawn original operand is not an apparent variable miss or a generic Host message.
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let result = prior(&mut vm);
    let original = Value::new_native_string_bytes(b"dictionaryTarget".as_slice());
    let original_lease = original.native_lifetime_lease();
    drop(original);
    let original = original_lease.into_value();
    assert!(!original.native_object_is_live());
    let before = original.native_object_snapshot();
    let crate::command::Command::Builtin(worker) = vm.lookup_command("::tcl::dict::set").unwrap()
    else {
        panic!("actual C8.6 dictionary set worker");
    };
    let args = [
        original,
        Value::new_native_string_bytes(b"key".as_slice()),
        Value::new_native_string_bytes(b"value".as_slice()),
    ];
    let completion = worker(&mut vm, &args);
    assert_eq!(completion.code, crate::Code::Error);
    let first = vm.execution_refusal.clone().unwrap();
    assert!(matches!(
        first,
        NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("retired native object header")
        )
    ));
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(args[0].native_object_snapshot(), before);
    assert!(
        vm.var_binding_from_bytes(b"dictionaryTarget", vm.current_level())
            .is_some_and(|binding| vm.raw_variable_binding(&binding).is_none())
    );
    unchanged_result(&vm, &result);
}

#[test]
fn original_variable_name_array_worker_keeps_current_counted_header_and_cell_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Current software success keeps the actual source name and selected existing array owner.
    let (mut vm, formatter, queries) = provided();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let original = Value::new_native_string_bytes(b"selectedArray".as_slice());
    let count = original.native_object_reference_count();
    let crate::command::Command::Builtin(worker) = vm.lookup_command("array").unwrap() else {
        panic!("actual C8.4 array worker");
    };
    let args = [
        Value::new_native_string_bytes(b"set".as_slice()),
        original,
        Value::new_native_string_bytes(b"key value".as_slice()),
    ];
    let completion = worker(&mut vm, &args);
    assert_eq!(completion.code, crate::Code::Ok);
    scope.check(&mut vm).unwrap();
    assert_eq!(args[1].native_object_reference_count(), count);
    assert_eq!(
        args[1].resident_string_bytes().unwrap().as_ref(),
        b"selectedArray"
    );
    let binding = vm
        .var_binding_from_bytes(b"selectedArray", vm.current_level())
        .unwrap();
    let array = vm.raw_variable_binding(&binding).unwrap();
    assert!(matches!(
        vm.var_arena.get(array).unwrap().state(),
        crate::interp::Local::Array(_)
    ));
    assert_eq!(
        vm.read_variable_result_bytes(b"selectedArray", Some(b"key"))
            .unwrap()
            .resident_string_bytes()
            .unwrap()
            .as_ref(),
        b"value"
    );
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_name_consumer_rejects_foreign_world_before_same_spelled_namespace_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Equal namespace labels do not replace the retained actual command-world owner.
    let (mut vm, formatter, queries) = provided();
    let original = numeric(&vm, i64::MIN);
    let before = original.native_object_snapshot();
    let result = prior(&mut vm);
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let foreign = provided().0;
    let world = Rc::clone(&vm.name_world);
    vm.name_world = Rc::clone(&foreign.name_world);
    assert_eq!(vm.current_ns_id(), foreign.current_ns_id());
    let error = scope.read(&mut vm, &original).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    vm.name_world = world;
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
fn original_variable_cache_stage_refuses_restored_context_before_member_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software stage observer after real List retirement; not an external free-hook chronology claim.
    let (mut vm, formatter, queries) = provided();
    let original_names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let protocol = vm.native_c_variable_name_protocol().unwrap();
    let dialect = vm.actual_native_invocation_dialect();
    let child = Value::new_native_string_bytes(b"cacheTarget".as_slice());
    let child_lease = child.native_lifetime_lease();
    let original = Value::native_list_constructor(
        vec![child],
        tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
    );
    original_names.read(&mut vm, &original).unwrap();
    assert_eq!(child_lease.value().native_object_reference_count(), 1);
    let built = Cell::new(0);
    let reached = Cell::new(false);
    let host = vm.host_rc();
    let error = original
        .install_native_parsed_variable_checked(
            dialect,
            || {
                built.set(built.get() + 1);
                Ok(tcl_syntax::native_variable_name::NativeParsedVariableName {
                    protocol,
                    array: None,
                })
            },
            &mut || {
                if !child_lease.value().native_object_is_live() && !reached.replace(true) {
                    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                    vm.set_host(Rc::clone(&host));
                }
                original_names.current_for_value(&mut vm, &original)
            },
        )
        .unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert!(reached.get());
    assert_eq!(built.get(), 0);
    assert_eq!(child_lease.value().native_object_reference_count(), 0);
    assert!(!child_lease.value().native_object_is_live());
    assert!(original.with_native_parsed_variable(|_| ()).is_none());
    assert_eq!(original.native_object_reference_count(), 1);
    assert_eq!(
        original.resident_string_bytes().unwrap().as_ref(),
        b"cacheTarget"
    );
    assert_eq!(
        original_names
            .finish(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
}

#[test]
fn original_variable_cache_stage_keeps_current_real_retirement_and_adoption_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Exact software worker order/currentness; no physical Tcl extension equivalence.
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm =
            crate::native_fixture::core(crate::environment::profile_for_dialect(environment));
        let original_names = OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let protocol = vm.native_c_variable_name_protocol().unwrap();
        let dialect = vm.actual_native_invocation_dialect();
        let child = Value::new_native_string_bytes(b"cacheTarget".as_slice());
        let child_lease = child.native_lifetime_lease();
        let original = Value::native_list_constructor(
            vec![child],
            tcl_syntax::native_string::NativeStringProtocol::C(protocol.version()),
        );
        original_names.read(&mut vm, &original).unwrap();
        let built = Cell::new(0);
        original
            .install_native_parsed_variable_checked(
                dialect,
                || {
                    assert!(!child_lease.value().native_object_is_live());
                    assert_eq!(child_lease.value().native_object_reference_count(), 0);
                    built.set(built.get() + 1);
                    Ok(tcl_syntax::native_variable_name::NativeParsedVariableName {
                        protocol,
                        array: None,
                    })
                },
                &mut || original_names.current_for_value(&mut vm, &original),
            )
            .unwrap();
        assert_eq!(built.get(), 1);
        assert!(
            original
                .with_native_parsed_variable(
                    |cache| cache.protocol == protocol && cache.array.is_none()
                )
                .unwrap()
        );
        assert_eq!(original.native_object_reference_count(), 1);
        assert_eq!(
            original.resident_string_bytes().unwrap().as_ref(),
            b"cacheTarget"
        );
        original_names.finish(&mut vm).unwrap();
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_variable_trace_registration_retains_prefix_failure_before_cell_creation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let result = prior(&mut vm);
    let original = numeric(&vm, i64::MIN);
    let before = original.native_object_snapshot();
    let counter = vm.next_var_trace_id;
    let cells = vm.var_arena.len();
    formatter.fail.set(true);
    let error = vm
        .add_var_trace_bytes_checked(b"notPublished", vec!["write".into()], &original, false)
        .unwrap_err();
    assert_eq!(error.code, crate::Code::Error);
    let first = vm.execution_refusal.clone().unwrap();
    assert!(!matches!(
        first,
        NativeExecutionError::HostCommandRefusal(_)
    ));
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(vm.next_var_trace_id, counter);
    assert_eq!(vm.var_arena.len(), cells);
    assert!(vm.registered_trace_cell_bytes(b"notPublished").is_none());
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(queries.get(), 1);
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|stage| **stage == "format")
            .count(),
        1
    );
    unchanged_result(&vm, &result);
}

#[test]
fn original_named_variable_read_retains_first_getter_failure_before_receiver_lookup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    let scope = NativeOperationScope::enter(&mut vm).unwrap();
    let result = prior(&mut vm);
    let original = numeric(&vm, i64::MIN);
    let count = original.native_object_reference_count();
    let cells = vm.var_arena.len();
    formatter.fail.set(true);
    let error = vm.read_original_named_variable(&original).unwrap_err();
    assert_eq!(error.code, crate::Code::Error);
    let first = vm.execution_refusal.clone().unwrap();
    assert!(!matches!(
        first,
        NativeExecutionError::HostCommandRefusal(_)
    ));
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(vm.var_arena.len(), cells);
    assert!(original.resident_string_bytes().is_none());
    assert_eq!(original.native_object_reference_count(), count);
    assert_eq!(queries.get(), 1);
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|stage| **stage == "format")
            .count(),
        1
    );
    unchanged_result(&vm, &result);
}

#[test]
fn original_name_selected_compiled_slot_refuses_replacement_before_the_next_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    vm.install_compiled_local_slots(&[tcl_core_types::NameBytes::from(b"local".as_slice())])
        .unwrap();
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    let binding = vm.compiled_local_binding(0).unwrap();
    let original_cell = vm.raw_variable_binding(&binding).unwrap();
    let selected = scope.capture_binding(&mut vm, &binding).unwrap();
    let replacement = vm
        .var_arena
        .alloc(crate::vars::VarState::Undefined)
        .unwrap();
    vm.frames.last_mut().unwrap().compiled_locals[0].1 = replacement;
    assert_ne!(original_cell, replacement);
    let error = selected.check(&mut vm, &scope).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    let original = numeric(&vm, i64::MIN);
    let snapshot = original.native_object_snapshot();
    assert_eq!(
        scope
            .read(&mut vm, &original)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
    assert_eq!(original.native_object_snapshot(), snapshot);
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert_eq!(
        scope
            .finish(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
}

#[test]
fn original_name_explicit_namespace_refuses_reborn_target_before_the_next_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    vm.declare_namespace("target");
    let target = vm
        .namespace_object_lookup(&Value::string("::target"))
        .unwrap()
        .unwrap();
    let scope = OriginalNameConsumerScope::enter_for_namespace(&mut vm, Some(target)).unwrap();
    assert!(
        vm.delete_namespace_path(&tcl_core_types::ByteNamespacePath::from_segments([
            b"target".as_slice()
        ]))
    );
    vm.declare_namespace("target");
    let replacement = vm
        .namespace_object_lookup(&Value::string("::target"))
        .unwrap()
        .unwrap();
    assert_ne!(target, replacement);
    let original = numeric(&vm, i64::MIN);
    let snapshot = original.native_object_snapshot();
    let error = scope.read(&mut vm, &original).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(original.native_object_snapshot(), snapshot);
    assert_eq!(queries.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.native_namespace_token_has_owner(replacement));
    assert_eq!(
        scope
            .finish(&mut vm)
            .unwrap_err()
            .native_execution_refusal(),
        Some(&first)
    );
}

#[test]
fn original_jim_dictionary_tuple_checks_real_retirement_before_adopting_children() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software stage observation; no external Jim callback chronology claim.
    for withdraw in [false, true] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
        let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let context = vm.native_jim_object_context().unwrap();
        let member = Value::new_native_string_bytes(b"d(k)".as_slice());
        let member_lease = member.native_lifetime_lease();
        let original = Value::native_list_constructor(
            vec![member],
            tcl_syntax::native_string::NativeStringProtocol::Jim084,
        );
        original.bind_native_jim_context(&context).unwrap();
        scope.read(&mut vm, &original).unwrap();
        assert_eq!(member_lease.value().native_object_reference_count(), 1);
        let reached_retirement = Cell::new(false);
        let host = vm.host_rc();
        let reached =
            original.ensure_native_jim_dictionary_substitution_checked(&context, &mut || {
                if !member_lease.value().native_object_is_live()
                    && !reached_retirement.replace(true)
                    && withdraw
                {
                    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                    vm.set_host(Rc::clone(&host));
                }
                scope.current_for_value(&mut vm, &original)
            });
        assert!(reached_retirement.get());
        assert!(!member_lease.value().native_object_is_live());
        assert_eq!(member_lease.value().native_object_reference_count(), 0);
        assert_eq!(original.native_object_reference_count(), 1);
        assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"d(k)");
        if withdraw {
            let error = reached.unwrap_err();
            let first = vm.execution_refusal.clone().unwrap();
            assert_eq!(error.native_execution_refusal(), Some(&first));
            assert!(
                original
                    .with_native_jim_dictionary_substitution(|_, _| ())
                    .is_none()
            );
            assert_eq!(
                scope
                    .finish(&mut vm)
                    .unwrap_err()
                    .native_execution_refusal(),
                Some(&first)
            );
        } else {
            reached.unwrap();
            original
                .with_native_jim_dictionary_substitution(|name, key| {
                    assert_eq!(name.resident_string_bytes().unwrap().as_ref(), b"d");
                    assert_eq!(key.resident_string_bytes().unwrap().as_ref(), b"k");
                    assert_eq!(name.native_object_reference_count(), 1);
                    assert_eq!(key.native_object_reference_count(), 1);
                })
                .unwrap();
            scope.finish(&mut vm).unwrap();
            assert!(vm.execution_refusal.is_none());
        }
    }
}

#[test]
fn selected_namespace_retirement_keeps_only_the_actual_entered_incarnation() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(dialect));
        let entered = vm.activate_namespace_operand(b"::kept").unwrap();
        vm.push_ns_token("kept", entered);
        let strict = QueryNamespace::capture(&vm, entered).unwrap();
        let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let names = [Value::string("::kept")];
        assert!(
            tcl_cmd_core::namespace::delete_original_with_selected_retirement(
                &mut vm,
                &names,
                |vm, selected| scope.retire_selected_namespace(vm, selected),
            )
            .unwrap()
            .is_none()
        );
        assert!(
            !strict.current(&vm),
            "generic query currency remains strict"
        );
        scope.check(&mut vm).unwrap();
        let replacement = vm.activate_namespace_operand(b"::kept").unwrap();
        assert_ne!(entered, replacement);
        scope.check(&mut vm).unwrap();
        vm.pop_ns();
        vm.push_ns_token("kept", replacement);
        assert!(
            scope.check(&mut vm).is_err(),
            "same spelling cannot replace entered birth"
        );
    }
}

#[test]
fn unrelated_namespace_retirement_cannot_issue_the_entered_operation_receipt() {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect("tcl8.6"));
    let entered = vm.activate_namespace_operand(b"::entered").unwrap();
    let other = vm.activate_namespace_operand(b"::other").unwrap();
    vm.push_ns_token("entered", entered);
    let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
    tcl_cmd_core::namespace::NamespaceDeleteBackend::delete_selected_namespace(&mut vm, entered)
        .unwrap();
    assert!(scope.retire_selected_namespace(&mut vm, other).is_err());
    assert_eq!(
        tcl_runtime_api::Namespaces::find_namespace_bytes_checked(
            &vm,
            tcl_core_types::ROOT_NS,
            b"::other",
        )
        .unwrap(),
        Some(other)
    );
}

#[test]
fn selected_ancestor_retirement_retains_the_actual_entered_child() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect(dialect));
        let ancestor = vm.activate_namespace_operand(b"::ancestor").unwrap();
        let entered = vm.activate_namespace_operand(b"::ancestor::child").unwrap();
        vm.push_ns_token("ancestor::child", entered);
        let strict = QueryNamespace::capture(&vm, entered).unwrap();
        let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
        scope.retire_selected_namespace(&mut vm, ancestor).unwrap();
        assert!(!strict.current(&vm));
        scope.check(&mut vm).unwrap();
        let replacement = vm.activate_namespace_operand(b"::ancestor::child").unwrap();
        assert_ne!(entered, replacement);
        scope.check(&mut vm).unwrap();
        vm.pop_ns();
        vm.push_ns_token("ancestor::child", replacement);
        assert!(scope.check(&mut vm).is_err());
    }
}
