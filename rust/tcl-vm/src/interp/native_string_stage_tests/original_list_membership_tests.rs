// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original expression membership shares supplied String and List owners.
use super::*;
use tcl_syntax::expr::ExprOps;

fn membership(vm: &mut Vm, needle: &Value, list: &Value) -> Result<bool, TclError> {
    ExprOps::in_list(&mut crate::expr::ExprEval::new(vm), needle, list)
}

#[test]
fn original_membership_cached_list_keeps_members_and_adds_no_host_query() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, formatter, acquisitions) = provided();
    let list = Value::native_list_constructor(
        vec![
            Value::new_native_string_bytes(b"first".as_slice()),
            Value::new_native_string_bytes(b"second".as_slice()),
        ],
        protocol(),
    );
    let backing = list.native_list_backing_in(protocol()).unwrap().unwrap();
    let items = backing.elements().unwrap();
    let counts = items
        .iter()
        .map(Value::native_object_reference_count)
        .collect::<Vec<_>>();
    assert!(
        membership(
            &mut vm,
            &Value::new_native_string_bytes(b"second".as_slice()),
            &list
        )
        .unwrap()
    );
    assert!(
        !membership(
            &mut vm,
            &Value::new_native_string_bytes(b"absent".as_slice()),
            &list
        )
        .unwrap()
    );
    let after_backing = list.native_list_backing_in(protocol()).unwrap().unwrap();
    let after = after_backing.elements().unwrap();
    for (index, item) in after.iter().enumerate() {
        assert!(item.is_same_object(&items[index]));
        assert_eq!(item.native_object_reference_count(), counts[index]);
    }
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_membership_formats_reached_numeric_member_once_and_keeps_its_native_role() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, formatter, acquisitions) = provided();
    let list = Value::native_list_constructor(vec![value(&vm, i64::MIN)], protocol());
    let backing = list.native_list_backing_in(protocol()).unwrap().unwrap();
    let items = backing.elements().unwrap();
    let count = items[0].native_object_reference_count();
    *formatter.original.borrow_mut() = Some(items[0].native_lifetime_lease());
    assert!(
        membership(
            &mut vm,
            &Value::new_native_string_bytes(i64::MIN.to_string().into_bytes()),
            &list
        )
        .unwrap()
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
            .all(|observed| *observed == count)
    );
    assert_eq!(items[0].native_object_reference_count(), count);
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_membership_live_root_child_withdrawal_preserves_typed_cause_before_later_member() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual software formatter shimmers the live original List and retires its child owners.
    let (mut vm, formatter, acquisitions) = provided();
    let list = Value::native_list_constructor(
        vec![value(&vm, i64::MIN), value(&vm, i64::MIN + 1)],
        protocol(),
    );
    *formatter.shimmer_root.borrow_mut() = Some((
        list.native_lifetime_lease(),
        vm.actual_native_invocation_dialect(),
    ));
    let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
    let failure = membership(
        &mut vm,
        &Value::new_native_string_bytes(b"absent".as_slice()),
        &list,
    )
    .unwrap_err();
    let first = vm.execution_refusal.clone().unwrap();
    assert!(
        matches!(failure,TclError::Host(crate::error::TclHostFailure::Execution(ref actual)) if actual==&first)
    );
    assert_eq!(
        first,
        tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "retired native object header"
            )
        )
    );
    assert_eq!(
        formatter
            .calls
            .borrow()
            .iter()
            .filter(|call| call.0 == "format")
            .count(),
        1
    );
    assert_eq!(acquisitions.get(), 1);
    assert!(list.native_object_is_live());
    assert_eq!(
        list.native_scalar_cache(),
        Some(NativeScalarCache::Tcl84Long(6))
    );
    assert!(list.resident_string_bytes().is_none());
    assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
}

#[test]
fn original_membership_list_string_refusal_stops_before_parse_cache_or_guest_error() {
    // naming.embedding.original-host-publication-and-fact-transport

    for failed in [false, true] {
        let (mut vm, formatter, acquisitions) = provided();
        let root = value(&vm, i64::MIN);
        let held = root.native_lifetime_lease();
        *formatter.retire_owner.borrow_mut() = Some(root);
        formatter.retire_at.set(Some("format"));
        formatter.fail.set(failed);
        let scope = NativeOperationScope::enter_native(&mut vm).unwrap();
        let failure = membership(
            &mut vm,
            &Value::new_native_string_bytes(b"absent".as_slice()),
            held.value(),
        )
        .unwrap_err();
        let first = vm.execution_refusal.clone().unwrap();
        assert!(
            matches!(failure,TclError::Host(crate::error::TclHostFailure::Execution(ref actual)) if actual==&first)
        );
        assert!(!held.value().native_object_is_live());
        assert!(held.value().cached_list_representation().is_none());
        assert!(held.value().resident_string_bytes().is_none());
        assert_eq!(
            formatter
                .calls
                .borrow()
                .iter()
                .filter(|call| call.0 == "format")
                .count(),
            1
        );
        assert_eq!(acquisitions.get(), 1);
        assert_eq!(scope.check_native(&mut vm).unwrap_err(), first);
    }
}

#[test]
fn original_membership_parse_failure_uses_retained_source_without_another_string_stage() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, formatter, acquisitions) = provided();
    let list = Value::new_native_string_bytes(b"{bad".as_slice());
    let source = list.resident_string_bytes().unwrap();
    let failure = membership(
        &mut vm,
        &Value::new_native_string_bytes(b"bad".as_slice()),
        &list,
    )
    .unwrap_err();
    assert!(failure.is_guest_error());
    assert!(
        failure
            .message_bytes()
            .unwrap()
            .starts_with(b"unmatched open brace")
    );
    assert!(list.cached_list_representation().is_none());
    assert!(Rc::ptr_eq(&source, &list.resident_string_bytes().unwrap()));
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_membership_authored_configured_parser_stays_independent_from_native_list_recipe() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, formatter, acquisitions) = provided();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core
    ));
    assert!(
        membership(
            &mut vm,
            &Value::new_native_string_bytes(b"a b".as_slice()),
            &Value::new_native_string_bytes(b"{a b} c".as_slice())
        )
        .unwrap()
    );
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    let configured = Value::new_native_string_bytes(b"{a}tail".as_slice());
    let pure = configured
        .native_list_elements(
            tcl_dialect::ListParse::Lenient,
            tcl_dialect::EscapeSyntax::Jim,
        )
        .unwrap();
    assert_eq!(pure[0].resident_string_bytes().unwrap().as_ref(), b"a");
    let native = Value::new_native_string_bytes(b"{a}tail".as_slice());
    assert!(
        native
            .native_object_list_elements_checked(protocol(), &mut || Ok(()))
            .is_err()
    );
    assert!(vm.execution_refusal.is_none());
}

#[test]
fn original_membership_foreign_selected_protocol_refuses_before_formatter_or_list_cache() {
    // naming.embedding.original-host-publication-and-fact-transport

    let (mut vm, formatter, acquisitions) = provided();
    let list = value(&vm, i64::MIN);
    let before = list.native_scalar_cache();
    vm.set_runtime_version(tcl_dialect::TclVersion::V9_0);
    let failure = membership(
        &mut vm,
        &Value::new_native_string_bytes(b"absent".as_slice()),
        &list,
    )
    .unwrap_err();
    assert!(failure.is_host());
    assert_eq!(acquisitions.get(), 0);
    assert!(formatter.calls.borrow().is_empty());
    assert_eq!(list.native_scalar_cache(), before);
    assert!(list.cached_list_representation().is_none());
    assert!(list.resident_string_bytes().is_none());
}
