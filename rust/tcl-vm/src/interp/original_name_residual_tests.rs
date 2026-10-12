// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software controls for the actual original capture, target and cleanup owners.

use super::*;
use crate::Value;
use std::cell::Cell;
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_syntax::number::Number;
use tcl_syntax::scalar_getter::NativeScalarCache;

fn native(engine: &str) -> Vm {
    crate::native_fixture::core(crate::environment::profile_for_dialect(engine))
}
fn header_refusal() -> tcl_runtime_api::NativeExecutionError {
    tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "retired native object header",
        ),
    )
}

#[derive(Default)]
struct Formatter {
    original: RefCell<Option<crate::value::NativeObjectLifetimeLease>>,
    owners: RefCell<Vec<Value>>,
    references: RefCell<Vec<usize>>,
    calls: RefCell<Vec<&'static str>>,
    retire: Cell<bool>,
}
impl Formatter {
    fn reached(&self, stage: &'static str) {
        self.calls.borrow_mut().push(stage);
        if let Some(original) = self.original.borrow().as_ref() {
            self.references
                .borrow_mut()
                .push(original.value().native_object_reference_count());
        }
        if stage == "format" && self.retire.get() {
            let owners = std::mem::take(&mut *self.owners.borrow_mut());
            drop(owners);
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
        Ok(value.to_string().into_bytes())
    }
}
struct FormatterHost {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<Formatter>,
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
        Some(&*self.formatter)
    }
}
fn supplied_formatter() -> (Vm, Rc<Formatter>) {
    let mut vm = native("tcl8.4");
    let formatter = Rc::new(Formatter::default());
    vm.set_host(Rc::new(FormatterHost {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
    }));
    (vm, formatter)
}
fn integer(vm: &Vm) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(i64::MIN)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}

#[test]
fn original_body_capture_observes_actual_reference_role_before_working_copy() {
    // naming.procedure.counted-body-header-selection
    // docs/design/analysis/name-resolution-proofs/procedure-counted-body-header-selection.md
    // Real VM owners exercise the selected algorithm; no external extension reference measurement.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = native(engine);
        let original = Value::new_native_string_bytes(b"return A\0B".as_slice());
        assert_eq!(original.native_object_reference_count(), 1);
        let captured = vm.capture_procedure_body_value(&original).unwrap();
        assert!(captured.is_same_object(&original), "{engine}");
        assert_eq!(original.native_object_reference_count(), 2, "{engine}");
        drop(captured);
        let second_owner = original.clone();
        let before = original.native_object_reference_count();
        let captured = vm.capture_procedure_body_value(&original).unwrap();
        assert_eq!(
            captured.is_same_object(&original),
            engine == "jim",
            "{engine}"
        );
        assert_eq!(
            captured.resident_string_bytes().unwrap().as_ref(),
            b"return A\0B"
        );
        assert_eq!(
            original.native_object_reference_count(),
            before + usize::from(engine == "jim")
        );
        assert!(second_owner.is_same_object(&original));
        assert!(vm.execution_refusal.is_none());
    }
}

#[test]
fn original_shared_body_string_worker_keeps_exact_native_reference_count() {
    // naming.procedure.counted-body-header-selection
    // docs/design/analysis/name-resolution-proofs/procedure-counted-body-header-selection.md
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied formatter observes software method stages and the actual borrowed header role.
    let (mut vm, formatter) = supplied_formatter();
    let original = integer(&vm);
    let second_owner = original.clone();
    let borrowed = original.native_lifetime_lease();
    *formatter.original.borrow_mut() = Some(original.native_lifetime_lease());
    *formatter.owners.borrow_mut() = vec![original, second_owner];
    let captured = vm.capture_procedure_body_value(borrowed.value()).unwrap();
    assert!(!captured.is_same_object(borrowed.value()));
    assert_eq!(
        captured.resident_string_bytes().unwrap().as_ref(),
        i64::MIN.to_string().as_bytes()
    );
    assert!(formatter.calls.borrow().contains(&"format"));
    assert!(
        formatter
            .references
            .borrow()
            .iter()
            .all(|count| *count == 2)
    );
    assert_eq!(borrowed.value().native_object_reference_count(), 2);
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_body_formatter_retirement_precedes_copy_and_public_error_boundary() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The callback retires both genuine owners; allocation-only observations add no Native reference.
    let (mut vm, formatter) = supplied_formatter();
    let outer = native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let original = integer(&vm);
    let second_owner = original.clone();
    let borrowed = original.native_lifetime_lease();
    *formatter.original.borrow_mut() = Some(original.native_lifetime_lease());
    *formatter.owners.borrow_mut() = vec![original, second_owner];
    formatter.retire.set(true);
    let refused = vm
        .capture_procedure_body_value(borrowed.value())
        .unwrap_err();
    assert_eq!(refused.native_execution_refusal(), Some(&header_refusal()));
    assert_eq!(vm.execution_refusal, Some(header_refusal()));
    assert_eq!(borrowed.value().native_object_reference_count(), 0);
    assert!(borrowed.value().resident_string_bytes().is_none());
    assert!(formatter.calls.borrow().contains(&"format"));
    assert!(
        formatter
            .references
            .borrow()
            .iter()
            .all(|count| *count == 2)
    );
    let completion = crate::command::completion_from_cmd_error(&mut vm, refused);
    assert_eq!(
        vm.finish_host_execution(completion).unwrap_err(),
        header_refusal()
    );
    assert_eq!(outer.check_native(&mut vm).unwrap_err(), header_refusal());
}

#[test]
fn original_formal_list_capture_order_is_selected_before_body_reference_acquisition() {
    // naming.procedure.counted-body-header-selection
    // docs/design/analysis/name-resolution-proofs/procedure-counted-body-header-selection.md
    // Actual C capture skips formal parsing here; actual Jim reaches its original formal List first.
    for engine in ["tcl8.4", "tcl8.6", "tcl9.1", "jim"] {
        let mut vm = native(engine);
        let parameters = Value::new_native_string_bytes(b"{".as_slice());
        let body = Value::new_native_string_bytes(b"return SAME".as_slice());
        let reached = vm.capture_procedure_declaration_body(&parameters, &body);
        if engine == "jim" {
            let error = reached.unwrap_err();
            assert!(error.native_execution_refusal().is_none());
            assert_eq!(body.native_object_reference_count(), 1);
            assert!(vm.execution_refusal.is_none());
        } else {
            let captured = reached.unwrap();
            assert!(captured.is_same_object(&body));
            assert_eq!(body.native_object_reference_count(), 2);
            assert!(matches!(
                parameters.native_object_snapshot().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::None
            ));
        }
    }
}

#[test]
fn original_c_alias_query_keeps_target_global_late_binding_and_loop_cleanup() {
    // naming.alias.original-command-key-replacement-child-capture-and-rename
    // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
    // Physical TclPreventAliasLoop target-global query; no command execution or prefix receipt is issued.
    let mut vm = native("tcl8.6");
    let parent = vm.cur_interp();
    let child_name = vm.create_child(Some("child".to_owned()), false);
    let child = vm.child_id(&child_name).unwrap();
    let shadow = Value::new_native_string_bytes(b"::Other::back".as_slice());
    let shadowed = vm.interp_alias_at(
        child,
        &shadow,
        parent,
        vec![Value::new_native_string_bytes(b"notDefined".as_slice())],
    );
    assert_eq!(shadowed.code, Code::Ok);
    vm.in_interp(child, |child_vm| {
        let other = child_vm.activate_namespace_written("::Other");
        child_vm.push_ns_eval_token_frame(other, vec![]);
        assert_ne!(child_vm.current_ns_id(), ROOT_NS);
    });
    let front = Value::new_native_string_bytes(b"front".as_slice());
    let back = Value::new_native_string_bytes(b"back".as_slice());
    let late = vm.interp_alias_at(child, &back, parent, vec![front.clone()]);
    assert_eq!(late.code, Code::Ok);
    assert!(
        vm.st_of(child)
            .unwrap()
            .visible_command_at_key("back")
            .is_some()
    );
    let looped = vm.interp_alias_at(parent, &front, child, vec![back]);
    assert_eq!(looped.code, Code::Error);
    assert_eq!(
        looped.result.resident_string_bytes().unwrap().as_ref(),
        b"cannot define or rename alias \"front\": would create a loop"
    );
    assert!(vm.visible_command_at_key("front").is_none());
    assert!(
        vm.st_of(child)
            .unwrap()
            .visible_command_at_key("back")
            .is_some()
    );
    assert_eq!(vm.cur_interp(), parent);
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_jim_alias_creation_reaches_no_c_loop_string_worker() {
    // naming.alias.original-command-key-replacement-child-capture-and-rename
    // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
    // Selected physical Jim creation contains no TclPreventAliasLoop call; this control invokes no cycle.
    let mut vm = native("jim");
    let root = vm.cur_interp();
    let name = Value::new_native_string_bytes(b"self".as_slice());
    let completion = vm.interp_alias_at(root, &name, root, vec![name.clone()]);
    assert_eq!(completion.code, Code::Ok);
    assert!(vm.visible_command_at_key("self").is_some());
    let original = Value::new_native_string_bytes(b"retired".as_slice());
    let borrowed = original.native_lifetime_lease();
    drop(original);
    // Transfer only the allocation lease; this creates no genuine native reference.
    let candidate = Command::Alias {
        words: Rc::new(vec![borrowed.into_value()]),
        publication_name: NameBytes::from(b"self".as_slice()),
    };
    assert!(!vm.alias_chain_loops_for_rename("self", &candidate).unwrap());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_jim_static_duplicate_cleanup_retires_only_fresh_unbound_cells() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Same original List children and copied member references; failed install discards both fresh cells.
    let mut vm = native("jim");
    let protocol = vm
        .actual_native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    let first = Value::new_native_string_bytes(b"FIRST".as_slice());
    let second = Value::new_native_string_bytes(b"SECOND".as_slice());
    let specifier = |value: Value| {
        Value::native_list_constructor(
            vec![Value::new_native_string_bytes(b"x".as_slice()), value],
            protocol,
        )
    };
    let original = Value::native_list_constructor(
        vec![specifier(first.clone()), specifier(second.clone())],
        protocol,
    );
    let cells = vm.var_arena.len();
    let references = [
        first.native_object_reference_count(),
        second.native_object_reference_count(),
    ];
    let error = vm.prepare_procedure_statics_value(&original).err().unwrap();
    assert_eq!(error.code, Code::Error);
    assert_eq!(vm.var_arena.len(), cells);
    assert_eq!(
        [
            first.native_object_reference_count(),
            second.native_object_reference_count()
        ],
        references
    );
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_legacy_string_role_retains_exact_header_refusal_at_real_boundary() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    use tcl_syntax::value::ValueOps;
    let (mut vm, formatter) = supplied_formatter();
    let outer = native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let original = integer(&vm);
    let borrowed = original.native_lifetime_lease();
    *formatter.original.borrow_mut() = Some(original.native_lifetime_lease());
    formatter.owners.borrow_mut().push(original);
    formatter.retire.set(true);
    assert!(ValueOps::native_string_bytes(&mut vm, borrowed.value()).is_err());
    assert_eq!(vm.execution_refusal, Some(header_refusal()));
    assert_eq!(borrowed.value().native_object_reference_count(), 0);
    assert!(borrowed.value().resident_string_bytes().is_none());
    let completion = err(b"unwind".as_slice());
    assert_eq!(
        vm.finish_host_execution(completion).unwrap_err(),
        header_refusal()
    );
    assert_eq!(outer.check_native(&mut vm).unwrap_err(), header_refusal());
}

#[test]
fn original_alias_source_header_refusal_precedes_publication_and_keeps_prior_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = native("tcl8.6");
    let outer = native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let original = Value::new_native_string_bytes(b"withdrawnAlias".as_slice());
    let borrowed = original.native_lifetime_lease();
    drop(original);
    let root = vm.cur_interp();
    let refused = vm.interp_alias_at(
        root,
        borrowed.value(),
        root,
        vec![Value::new_native_string_bytes(b"list".as_slice())],
    );
    assert_eq!(vm.execution_refusal, Some(header_refusal()));
    assert!(vm.visible_command_at_key("withdrawnAlias").is_none());
    assert_eq!(
        vm.finish_host_execution(refused).unwrap_err(),
        header_refusal()
    );
    let valid = Value::new_native_string_bytes(b"laterAlias".as_slice());
    let refused = vm.interp_alias_at(
        root,
        &valid,
        root,
        vec![Value::new_native_string_bytes(b"list".as_slice())],
    );
    assert!(vm.visible_command_at_key("laterAlias").is_none());
    assert_eq!(
        vm.finish_host_execution(refused).unwrap_err(),
        header_refusal()
    );
    assert_eq!(outer.check_native(&mut vm).unwrap_err(), header_refusal());
}

#[test]
fn original_alias_retained_payload_cannot_revive_a_deleted_target_interpreter() {
    // naming.alias.original-command-key-replacement-child-capture-and-rename
    // docs/design/analysis/name-resolution-proofs/alias-original-command-key-replacement-child-capture-and-rename.md
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual publication/deletion supplies the payload and dead target; no command lookup or execution is admitted.
    let mut vm = native("tcl8.6");
    let parent = vm.cur_interp();
    let name = vm.create_child(Some("child".to_owned()), false);
    let child = vm.child_id(&name).unwrap();
    let definition = Value::new_native_string_bytes(b"holder".as_slice());
    let reached = vm.interp_alias_at(
        parent,
        &definition,
        child,
        vec![Value::new_native_string_bytes(b"missing".as_slice())],
    );
    assert_eq!(reached.code, Code::Ok);
    let retained = vm.visible_command_at_key("holder").unwrap();
    assert!(vm.delete_interp(child));
    assert!(!vm.interp_alive(child));
    let outer = native_operation_currency::NativeOperationScope::enter(&mut vm).unwrap();
    let refused = vm
        .alias_chain_loops_for_rename("list", &retained)
        .unwrap_err();
    let expected = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "original alias target interpreter",
        ),
    );
    assert_eq!(refused.native_execution_refusal(), Some(&expected));
    assert_eq!(vm.execution_refusal, Some(expected.clone()));
    assert_eq!(vm.cur_interp(), parent);
    let completion = crate::command::completion_from_cmd_error(&mut vm, refused);
    assert_eq!(vm.finish_host_execution(completion).unwrap_err(), expected);
    assert_eq!(outer.check_native(&mut vm).unwrap_err(), expected);
}
