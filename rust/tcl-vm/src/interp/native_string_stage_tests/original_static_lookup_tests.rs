// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software original static-table and cat stages, independent of Native observations.

use super::*;
use tcl_registry::native_index_lookup::NativeStaticIndexTable;
use tcl_syntax::value::ValueOps;

fn provided_jim() -> (Vm, Rc<Formatter>, Rc<Cell<usize>>) {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    let formatter = Rc::new(Formatter::default());
    let acquisitions = Rc::new(Cell::new(0));
    vm.set_host(Rc::new(FormatterHost {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
        acquisitions: Rc::clone(&acquisitions),
    }));
    (vm, formatter, acquisitions)
}

#[test]
fn original_static_lookup_keeps_one_formatter_and_cached_absent_string_without_query() {
    // Supplied software formatter, not an external Tcl callback experiment.
    let (mut vm, formatter, acquisitions) = provided();
    let original = value(&vm, i64::MIN);
    let references = original.native_object_reference_count();
    *formatter.original.borrow_mut() = Some(original.native_lifetime_lease());
    let table = NativeStaticIndexTable::supported_backend(&["-9223372036854775808"]);
    assert_eq!(
        vm.native_index_operand_checked(&original, &table, true, "option")
            .unwrap(),
        0
    );
    assert_eq!(acquisitions.get(), 1);
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
            .all(|count| *count == references)
    );
    let bytes = original.resident_string_bytes().unwrap();
    assert_eq!(bytes.as_ref(), b"-9223372036854775808");
    original.invalidate_native_string_for_test();
    assert_eq!(
        vm.native_index_operand_checked(&original, &table, true, "option")
            .unwrap(),
        0
    );
    assert!(original.resident_string_bytes().is_none());
    assert_eq!(original.native_object_reference_count(), references);
    assert_eq!(acquisitions.get(), 1);
    assert_eq!(formatter.calls.borrow().len(), 2);
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_static_lookup_preserves_counted_invalid_bytes_and_does_not_cache_miss() {
    let (mut vm, formatter, acquisitions) = provided();
    let original = Value::new_native_string_bytes(b"\xff".as_slice());
    let before = original.native_object_snapshot();
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", &["provide", "present"]);
    let error = table.index_of_original(&mut vm, &original).unwrap_err();
    assert_eq!(
        error.message_bytes(),
        b"bad option \"\xff\": must be provide or present"
    );
    assert_eq!(
        error.error_code_bytes(),
        Some(b"TCL LOOKUP INDEX option \xff".as_slice())
    );
    assert!(error.native_execution_refusal().is_none());
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_static_lookup_stops_retired_compound_before_cache_and_guest_projection() {
    let (mut vm, formatter, _) = provided();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let root = Value::native_list_constructor(
        vec![value(&vm, i64::MIN), value(&vm, i64::MIN + 1)],
        protocol(),
    );
    let backing = root.native_list_backing_in(protocol()).unwrap().unwrap();
    let later = backing.elements().unwrap()[1].native_lifetime_lease();
    let before = later.value().native_object_snapshot();
    let held = root.native_lifetime_lease();
    *formatter.retire_owner.borrow_mut() = Some(root);
    formatter.retire_at.set(Some("format"));
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", &["provide", "present"]);
    let error = table.index_of_original(&mut vm, held.value()).unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(
        first,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object header"
            )
        )
    );
    assert!(held.value().resident_string_bytes().is_none());
    assert!(held.value().native_index_cache().is_none());
    assert_eq!(later.value().native_object_snapshot(), before);
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|call| call.0 == "format")
            .count(),
        1
    );
    let guest = crate::command::completion_from_cmd_error(&mut vm, error);
    assert!(guest.result.resident_string_bytes().unwrap().is_empty());
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(vm.finish_host_execution(guest).unwrap_err(), first);
}

#[test]
fn original_index_mirror_checks_actual_currency_after_retirement_before_adoption() {
    // The software checker withdraws currency at a reached kernel boundary; no opaque Native free hook is claimed.
    let (mut vm, _, _) = provided();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let original = Value::new_native_string_bytes(b"pro".as_slice());
    let dialect = vm.actual_native_invocation_dialect();
    let protocol = dialect.native_index_lookup_protocol().unwrap();
    let table = NativeStaticIndexTable::supported_backend(&["provide", "present"]);
    let donor = Value::from_native_index_cache(
        table.cache(0),
        protocol.version(),
        dialect,
        original
            .resident_string_bytes()
            .zip(original.resident_string_storage_identity()),
    )
    .unwrap();
    let before_string = original.resident_string_bytes().unwrap();
    let stages = Cell::new(0);
    let error = original
        .adopt_native_object_representation_with_string_mutation_checked::<CmdError>(
            &donor,
            dialect,
            tcl_core_types::ResidentStringMutation::Preserve,
            &mut || {
                stages.set(stages.get() + 1);
                if stages.get() == 3 {
                    let host = vm.host_rc();
                    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
                    vm.set_host(host);
                }
                scope.check(&mut vm)
            },
        )
        .unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert_eq!(stages.get(), 3);
    assert!(original.native_index_cache().is_none());
    assert!(Rc::ptr_eq(
        &before_string,
        &original.resident_string_bytes().unwrap()
    ));
    assert!(donor.native_index_cache().is_some());
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}

#[test]
fn original_jim_enum_keeps_cache_hit_without_getter_and_foreign_index_refusal() {
    let (mut vm, formatter, acquisitions) = provided_jim();
    let original = Value::new_native_string_bytes(b"provide".as_slice());
    let table = NativeStaticIndexTable::supported_backend(&["provide", "present"]);
    let flags = tcl_registry::native_jim_enum::NativeJimEnumFlags::options(false);
    assert_eq!(
        vm.native_jim_enum_from_original_checked(&original, &table, flags, Some(b"option"))
            .unwrap(),
        Ok(0)
    );
    original.invalidate_native_string_for_test();
    assert_eq!(
        vm.native_jim_enum_from_original_checked(&original, &table, flags, Some(b"option"))
            .unwrap(),
        Ok(0)
    );
    assert!(original.resident_string_bytes().is_none());
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    let c = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let dialect = c.actual_native_invocation_dialect();
    let foreign = Value::from_native_index_cache(
        table.cache(0),
        tcl_dialect::TclVersion::V9_0,
        dialect,
        None,
    )
    .unwrap();
    assert!(
        vm.native_jim_enum_from_original_checked(&foreign, &table, flags, Some(b"option"))
            .is_err()
    );
    assert!(foreign.native_index_cache().is_some());
    assert!(foreign.resident_string_bytes().is_none());
    assert_eq!(acquisitions.get(), 0);
    assert!(vm.execution_refusal.is_some());
}

#[test]
fn original_static_usage_keeps_actual_canonical_index_and_retained_typed_host() {
    let (mut vm, formatter, acquisitions) = provided();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let original = Value::new_native_string_bytes(b"pro".as_slice());
    let table = NativeStaticIndexTable::supported_backend(&["provide", "present"]);
    vm.native_index_operand_checked(&original, &table, false, "option")
        .unwrap();
    original.invalidate_native_string_for_test();
    assert_eq!(
        vm.native_argument_usage_header(std::slice::from_ref(&original))
            .unwrap(),
        b"provide"
    );
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    let _ = vm.refuse_host_command("original static usage cause".into());
    let first = vm.execution_refusal.clone().unwrap();
    let completion = vm
        .native_argument_usage_header(std::slice::from_ref(&original))
        .unwrap_err();
    assert!(
        completion
            .result
            .resident_string_bytes()
            .unwrap()
            .is_empty()
    );
    assert_eq!(vm.execution_refusal.as_ref(), Some(&first));
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}

#[test]
fn original_jim_cat_keeps_empty_unary_and_compound_source_roles_without_host_query() {
    // Software checked worker roles only; no new public zero/unary observation.
    let (mut vm, formatter, acquisitions) = provided_jim();
    let empty = vm.native_jim_string_cat(&[]).unwrap().unwrap();
    assert_eq!(empty.native_object_type_name(), "none");
    assert_eq!(empty.resident_string_bytes().unwrap().as_ref(), b"");
    let child = Value::new_native_string_bytes(b"a b".as_slice());
    let root = Value::native_list_constructor(vec![child], NativeStringProtocol::Jim084);
    let count = root.native_object_reference_count();
    let unary = vm
        .native_jim_string_cat(std::slice::from_ref(&root))
        .unwrap()
        .unwrap();
    assert!(unary.is_same_object(&root));
    assert_eq!(root.native_object_reference_count(), count + 1);
    assert!(root.resident_string_bytes().is_none());
    drop(unary);
    assert_eq!(root.native_object_reference_count(), count);
    let suffix = Value::new_native_string_bytes(b"!".as_slice());
    let result = vm
        .native_jim_string_cat(&[root.native_lifetime_lease().into_value(), suffix])
        .unwrap()
        .unwrap();
    assert_eq!(result.resident_string_bytes().unwrap().as_ref(), b"{a b}!");
    assert_eq!(result.native_object_type_name(), "string");
    assert_eq!(root.native_object_reference_count(), count);
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_jim_cat_preserves_prior_full_host_before_later_operand_or_materialization() {
    let (mut vm, formatter, acquisitions) = provided_jim();
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let original = Value::int(17);
    let before = original.native_object_snapshot();
    let _ = vm.refuse_host_command("first original Jim cat cause".into());
    let first = vm.execution_refusal.clone().unwrap();
    let error = tcl_cmd_core::string::cat(&mut vm, std::slice::from_ref(&original)).unwrap_err();
    let completion = crate::command::completion_from_cmd_error(&mut vm, error);
    assert!(
        completion
            .result
            .resident_string_bytes()
            .unwrap()
            .is_empty()
    );
    assert_eq!(original.native_object_snapshot(), before);
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(vm.finish_host_execution(completion).unwrap_err(), first);
}

#[test]
fn original_jim_cat_authored_routing_checks_inherited_currency_without_native_issuer() {
    let (mut vm, formatter, acquisitions) = provided();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(
        vm.set_logical_name_provider(tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
            tcl_dialect::TclVersion::V8_4
        ))
    );
    assert!(vm.native_jim_string_cat(&[]).unwrap().is_none());
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let host = vm.host_rc();
    vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
    vm.set_host(host);
    assert!(vm.native_jim_string_cat(&[]).is_err());
    let first = vm.execution_refusal.clone().unwrap();
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
}
