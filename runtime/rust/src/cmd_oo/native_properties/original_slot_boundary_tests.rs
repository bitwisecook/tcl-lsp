// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::cell::Cell;
use std::cell::RefCell;
#[cfg(debug_assertions)]
use tcl_syntax::native_object::NativeObjectCacheSnapshot;

thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static UPDATES: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static RETIREMENT_OUTSIDE_BORROW: Cell<bool> = const { Cell::new(false) };
    static RETIREMENT_EPOCH: Cell<u64> = const { Cell::new(0) };
}

fn native() -> (Interp, OoId) {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
            interpreter.eval_str(
                b"oo::class create PropertyBoundaryClass; PropertyBoundaryClass create PropertyBoundaryObject",
            ),
            Code::Ok,
        );
    let target = interpreter.oo_resolve_object(b"PropertyBoundaryClass");
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    UPDATES.with(|count| count.set(0));
    FREES.with(|count| count.set(0));
    (interpreter, target)
}

#[cfg(debug_assertions)]
fn reached_callback(interpreter: &mut Interp) {
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("first original property slot callback");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("original-property-slot".to_owned(), "1.0".to_owned()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            assert_eq!(
                interpreter.eval_str(b"PropertyBoundaryClass destroy"),
                Code::Ok,
            );
        }
        _ => {}
    }
}

#[cfg(debug_assertions)]
extern "C" fn update(original: *mut TclObj) {
    UPDATES.with(|count| count.set(count.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    reached_callback(&mut interpreter);
    unsafe { obj::set_native_updater_string_rep(original, b"p", false) };
}

#[cfg(debug_assertions)]
extern "C" fn free(_: *mut TclObj) {
    FREES.with(|count| count.set(count.get() + 1));
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    RETIREMENT_OUTSIDE_BORROW.with(|free| {
        free.set(interpreter.oo.try_borrow_mut().is_ok());
    });
    reached_callback(&mut interpreter);
    RETIREMENT_EPOCH.with(|epoch| epoch.set(interpreter.oo.borrow().property_foundation_epoch));
}

#[cfg(debug_assertions)]
static UPDATER: obj::TclObjType = obj::TclObjType {
    name: c"originalPropertySlotUpdater".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
#[cfg(debug_assertions)]
static RETIRING: obj::TclObjType = obj::TclObjType {
    name: c"originalPropertySlotRetirement".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

fn strings(interpreter: &Interp) -> tcl_syntax::native_string::NativeStringProtocol {
    interpreter
        .native_invocation_dialect()
        .native_property_lookup_protocol()
        .unwrap()
        .strings()
}

#[cfg(debug_assertions)]
#[test]
fn original_property_slot_list_callback_stops_before_cache_and_storage() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software checked transport; no C9.1 property storage-order equivalence.
    for mode in [1, 2] {
        let (mut interpreter, target) = native();
        MODE.with(|selected| selected.set(mode));
        let original = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 0));
        let original_cache = obj::native_object_snapshot(original.as_ptr())
            .unwrap()
            .cache;
        let original_primary = obj::internal_rep(original.as_ptr());
        assert!(matches!(original_cache, NativeObjectCacheSnapshot::Other));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let epoch = interpreter.oo.borrow().property_foundation_epoch;
        let failure = set_property_slot(&mut interpreter, target, true, false, original.as_ptr())
            .err()
            .unwrap_or_else(|| panic!("reached original getter refuses"));
        let first_cause = interpreter
            .native_execution_refusal()
            .expect("actual callback retains its typed first cause");
        assert_eq!(failure.native_execution_refusal(), Some(&first_cause));
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert_eq!(obj::bytes_of(original.as_ptr()), b"p");
        assert!(core::ptr::eq(
            obj::obj_type_ptr(original.as_ptr()),
            &UPDATER
        ));
        assert_eq!(
            obj::native_object_snapshot(original.as_ptr())
                .unwrap()
                .cache,
            original_cache,
        );
        assert_eq!(obj::internal_rep(original.as_ptr()), original_primary);
        let state = interpreter.oo.borrow();
        assert!(state.classes[&target].readable_properties.is_empty());
        assert!(!state.property_members.contains_key(&(target, true, false)));
        assert_eq!(state.property_foundation_epoch, epoch);
        drop(state);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_property_slot_member_callback_stops_later_member_and_reference_stages() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Allocation leases do not read Strings; the selected slot takes all incoming references before hashing.
    for mode in [1, 2] {
        let (mut interpreter, target) = native();
        MODE.with(|selected| selected.set(mode));
        let first = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 0));
        let later = obj::Owned::fresh(obj::alloc_typed(&UPDATER, 0));
        let original = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[first.as_ptr(), later.as_ptr()],
            strings(&interpreter),
        ));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let failure = set_property_slot(&mut interpreter, target, true, false, original.as_ptr())
            .err()
            .unwrap_or_else(|| panic!("first member getter refuses"));
        let first_cause = interpreter
            .native_execution_refusal()
            .expect("actual callback retains its typed first cause");
        assert_eq!(failure.native_execution_refusal(), Some(&first_cause));
        assert_eq!(UPDATES.with(Cell::get), 1);
        assert_eq!(obj::bytes_of(first.as_ptr()), b"p");
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 2);
        assert_eq!(unsafe { (*later.as_ptr()).ref_count }, 2);
        assert!(interpreter.oo.borrow().classes[&target]
            .readable_properties
            .is_empty());
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_property_slot_retirement_runs_outside_borrow_and_checks_before_epoch() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Selected C91 retirement precedes later adoption and epoch publication.
    for mode in [0, 1, 2, 3] {
        let (mut interpreter, target) = native();
        MODE.with(|selected| selected.set(0));
        let old = obj::Owned::fresh(obj::alloc_typed(&RETIRING, 0));
        unsafe { obj::set_native_updater_string_rep(old.as_ptr(), b"old", false) };
        let pointer = old.as_ptr();
        let memory = obj::NativeObjectLifetime::retain(pointer);
        let input = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[pointer],
            strings(&interpreter),
        ));
        assert!(set_property_slot(&mut interpreter, target, true, false, input.as_ptr(),).is_ok());
        drop(input);
        drop(old);
        let reported = interpreter
            .native_all_property_header(target, true, false)
            .unwrap();
        reported.check(&interpreter).unwrap();
        // Observe header retirement with an allocation lease after completing the report operation.
        let cached = obj::NativeObjectLifetime::retain(reported.header);
        drop(reported);
        assert_eq!(unsafe { (*pointer).ref_count }, 2);
        let new = obj::Owned::fresh(obj::new_string_bytes(b"new"));
        let input = obj::Owned::fresh(crate::list::new_list_obj_native(
            &[new.as_ptr()],
            strings(&interpreter),
        ));
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        let epoch = interpreter.oo.borrow().property_foundation_epoch;
        MODE.with(|selected| selected.set(mode));
        let result = set_property_slot(&mut interpreter, target, true, false, input.as_ptr());
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(RETIREMENT_OUTSIDE_BORROW.with(Cell::get));
        assert!(!obj::allocation_is_live(pointer));
        assert!(!obj::allocation_is_live(cached.as_ptr()));
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        if mode == 0 {
            assert!(result.is_ok());
            assert_eq!(interpreter.oo.borrow().property_foundation_epoch, epoch + 1);
        } else if mode == 3 {
            let failure = result
                .err()
                .unwrap_or_else(|| panic!("original target was deleted"));
            assert!(failure.native_execution_refusal().is_none());
            assert!(!interpreter.oo.borrow().objects.contains_key(&target));
            assert_eq!(
                interpreter.oo.borrow().property_foundation_epoch,
                RETIREMENT_EPOCH.with(Cell::get),
            );
        } else {
            let failure = result
                .err()
                .unwrap_or_else(|| panic!("reached original retirement refuses"));
            let first_cause = interpreter
                .native_execution_refusal()
                .expect("actual retirement retains its typed first cause");
            assert_eq!(failure.native_execution_refusal(), Some(&first_cause));
            let state = interpreter.oo.borrow();
            assert_eq!(
                state.classes[&target].readable_properties,
                [b"old".to_vec()]
            );
            assert_eq!(state.property_foundation_epoch, epoch);
            assert!(state.property_caches[&(target, true)].readable.is_none());
            assert!(!state.property_members.contains_key(&(target, true, false)));
            assert_eq!(unsafe { (*new.as_ptr()).ref_count }, 2);
            assert_eq!(UPDATES.with(Cell::get), 0);
            drop(state);
            assert_eq!(interpreter.get_obj_result(), prior);
        }
        drop(cached);
        drop(memory);
    }
}

#[test]
fn original_property_slot_selected_list_keeps_member_identity_and_counted_names() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Existing selected List and first-win storage roles; no native phase comparison.
    let (mut interpreter, target) = native();
    MODE.with(|selected| selected.set(0));
    let first = obj::Owned::fresh(obj::new_string_bytes(b"p\0tail"));
    let second = obj::Owned::fresh(obj::new_string_bytes(b"p\0tail"));
    let original = obj::Owned::fresh(crate::list::new_list_obj_native(
        &[first.as_ptr(), second.as_ptr()],
        strings(&interpreter),
    ));
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert!(set_property_slot(&mut interpreter, target, true, false, original.as_ptr(),).is_ok());
    assert!(!obj::has_string_rep(original.as_ptr()));
    assert_eq!(UPDATES.with(Cell::get), 0);
    let state = interpreter.oo.borrow();
    assert_eq!(
        state.classes[&target].readable_properties,
        [b"p\0tail".to_vec()]
    );
    assert_eq!(
        state.property_members[&(target, true, false)][b"p\0tail".as_slice()].as_ptr(),
        first.as_ptr(),
    );
    drop(state);
    assert_eq!(unsafe { (*first.as_ptr()).ref_count }, 3);
    assert_eq!(unsafe { (*second.as_ptr()).ref_count }, 2);
    assert_eq!(interpreter.entered_native_operation_count(), 0);
}
