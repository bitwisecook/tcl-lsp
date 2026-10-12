// SPDX-License-Identifier: AGPL-3.0-or-later
//! Software original operand observation and independent String projections.
use super::*;
use crate::interp::original_authored_operands::OriginalAuthoredOperandScope;
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueError;

#[test]
fn original_authored_operand_resident_none_has_no_entry_or_provider_donation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = Vm::new();
    let owner = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    let resident = Value::string("A\0🙂");
    let count = resident.native_object_reference_count();
    assert_eq!(
        owner.string(&mut vm, &resident).unwrap().as_ref(),
        "A\0🙂".as_bytes()
    );
    assert_eq!(
        owner
            .raw_bytes(&mut vm, &Value::int(i64::MIN))
            .unwrap()
            .as_ref(),
        i64::MIN.to_string().as_bytes()
    );
    assert_eq!(resident.native_object_reference_count(), count);
    assert!(
        NativeOperationScope::current_entered(&mut vm)
            .unwrap()
            .is_none()
    );
    assert!(owner.string(&mut vm, &Value::int(23)).is_err());
    let first = vm.execution_refusal.clone().unwrap();
    let refused = OriginalAuthoredOperandScope::capture(&mut vm)
        .err()
        .unwrap();
    assert_eq!(refused.native_execution_refusal(), Some(&first));
    assert!(!vm.native_operation_context.has_entered_operation());
    assert!(owner.finish(&mut vm).is_err());
}

#[test]
fn original_authored_operand_neutral_utf8_keeps_raw_units_and_legacy_scalar_worker() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let (mut vm, formatter, queries) = provided();
    let owner = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
    let original = value(&vm, i64::MIN);
    assert_eq!(
        owner.raw_bytes(&mut vm, &original).unwrap().as_ref(),
        i64::MIN.to_string().as_bytes()
    );
    assert!(formatter.calls.borrow().is_empty());
    assert_eq!(queries.get(), 0);
    let raw = Value::new_native_string_bytes(b"A\xc0\x80B".as_slice());
    assert_eq!(
        owner.raw_bytes(&mut vm, &raw).unwrap().as_ref(),
        b"A\xc0\x80B"
    );
    assert!(owner.unicode(&mut vm, &raw).is_err());
    assert!(vm.execution_refusal.is_some());
    assert!(!vm.native_operation_context.has_entered_operation());
    assert!(owner.finish(&mut vm).is_err());
}

#[test]
fn original_authored_operand_detects_frame_birth_and_original_host_aba_without_entry() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for host_aba in [false, true] {
        let (mut vm, formatter, queries) = provided();
        let owner = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
        if host_aba {
            let old = vm.host_rc();
            vm.set_host(Rc::new(crate::host_native::NativeHost::new()));
            vm.set_host(old);
        } else {
            vm.push_call_frame(Some("changed".into()), vec![]);
        }
        let untouched = value(&vm, i64::MIN);
        assert!(owner.string(&mut vm, &untouched).is_err());
        assert!(untouched.resident_string_bytes().is_none());
        assert!(formatter.calls.borrow().is_empty());
        assert_eq!(queries.get(), 0);
        assert!(vm.execution_refusal.is_some());
        assert!(!vm.native_operation_context.has_entered_operation());
        if !host_aba {
            vm.pop_call_frame();
        }
        assert!(owner.finish(&mut vm).is_err());
    }
}

#[test]
fn original_authored_operand_first_formatter_failure_survives_world_withdrawal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied software formatter and readonly world; no Native callback observation.
    let (mut vm, formatter, _) = provided();
    formatter.fail.set(true);
    *formatter.retire_name_world.borrow_mut() = Some(Rc::clone(&vm.name_world));
    let owner = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
    let original = value(&vm, i64::MIN);
    assert!(owner.string(&mut vm, &original).is_err());
    let expected: CmdError = ValueError::from(
        tcl_syntax::raw_string::NativeStringAccessError::Unavailable(
            tcl_syntax::native_string::NativeStringUnavailable::StringUpdater,
        ),
    )
    .into();
    assert_eq!(
        vm.execution_refusal.as_ref(),
        expected.native_execution_refusal()
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
    assert!(original.resident_string_bytes().is_none());
    assert!(!vm.native_operation_context.has_entered_operation());
    assert!(owner.finish(&mut vm).is_err());
}

#[test]
fn original_authored_operand_retainer_precedes_native_compound_backing_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual backing lifetime with a supplied software checker, not Native callback chronology.
    let (mut vm, formatter, queries) = provided();
    let first = value(&vm, i64::MIN);
    let first_view = first.native_lifetime_lease();
    let root = Value::native_list_constructor(vec![first], protocol());
    let owner = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
    let retained_before_drop = Cell::new(false);
    let replaced = Cell::new(false);
    let dialect = vm.actual_native_invocation_dialect();
    let vm = RefCell::new(&mut vm);
    let reached = root.string_bytes_checked_for_value_retaining::<CmdError>(
        &mut |actual| {
            owner.current_for_value(&mut vm.borrow_mut(), actual)?;
            if actual.is_same_object(first_view.value()) && !replaced.replace(true) {
                root.store_native_expression_long84(6, dialect)
                    .map_err(CmdError::from)?;
            }
            Ok(())
        },
        &|error| {
            retained_before_drop.set(first_view.value().native_object_is_live());
            owner.retain(&mut vm.borrow_mut(), &error.clone().into());
        },
    );
    assert!(reached.is_err());
    assert!(retained_before_drop.get());
    assert!(!first_view.value().native_object_is_live());
    assert!(formatter.calls.borrow().is_empty());
    assert_eq!(queries.get(), 0);
    assert!(vm.borrow().execution_refusal.is_some());
    assert!(owner.finish(&mut vm.borrow_mut()).is_err());
}

#[test]
fn original_authored_operand_successful_formatter_frame_change_stops_cache_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The supplied checker changes the actual frame after observing a formatter stage.
    let (mut vm, formatter, _) = provided();
    let owner = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
    let original = value(&vm, i64::MIN);
    let mutated = Cell::new(false);
    let vm = RefCell::new(&mut vm);
    let reached = original.native_string_bytes_checked_for_value_retaining::<CmdError>(
        protocol(),
        Some(&*formatter),
        &mut |actual| {
            if formatter
                .calls
                .borrow()
                .iter()
                .any(|call| call.0 == "format")
                && !mutated.replace(true)
            {
                vm.borrow_mut()
                    .push_call_frame(Some("after_format".into()), vec![]);
            }
            owner.current_for_value(&mut vm.borrow_mut(), actual)
        },
        &|error| owner.retain(&mut vm.borrow_mut(), &error.clone().into()),
    );
    assert!(reached.is_err());
    assert!(mutated.get());
    assert!(original.resident_string_bytes().is_none());
    vm.borrow_mut().pop_call_frame();
    assert!(owner.finish(&mut vm.borrow_mut()).is_err());
}

#[test]
fn original_string_intrinsic_root_header_failure_is_retained_before_child_backing_drop() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied software registration-style checker; the actual String worker owns the root check.
    let (mut vm, formatter, queries) = provided();
    let child = Value::string("RESIDENT_CHILD");
    let child_view = child.native_lifetime_lease();
    let root = Value::native_list_constructor(vec![child], protocol());
    let root_view = root.native_lifetime_lease();
    let root_owner = RefCell::new(Some(root));
    let original = OriginalAuthoredOperandScope::capture(&mut vm).unwrap();
    let retained_while_live = Cell::new(false);
    let vm = RefCell::new(&mut vm);
    let reached = root_view
        .value()
        .native_string_bytes_checked_for_value_retaining::<CmdError>(
            protocol(),
            None,
            &mut |actual| {
                original.check(&mut vm.borrow_mut())?;
                if actual.is_same_object(child_view.value()) {
                    drop(root_owner.borrow_mut().take());
                }
                Ok(())
            },
            &|error| {
                retained_while_live.set(child_view.value().native_object_is_live());
                original.retain(&mut vm.borrow_mut(), &error.clone().into());
            },
        );
    assert!(reached.is_err());
    assert!(retained_while_live.get());
    assert!(!root_view.value().native_object_is_live());
    assert!(!child_view.value().native_object_is_live());
    let expected: CmdError =
        ValueError::CommandProtocolUnavailable("retired native object header").into();
    assert_eq!(
        vm.borrow().execution_refusal.as_ref(),
        expected.native_execution_refusal()
    );
    assert!(formatter.calls.borrow().is_empty());
    assert_eq!(queries.get(), 0);
    assert!(original.finish(&mut vm.borrow_mut()).is_err());
}
