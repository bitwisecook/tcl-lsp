// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual compiled Number stages retain the original indexed frame and cell.
use super::super::native_list_conversion::BodyObjectOperation;
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj::TclObjType;
use std::cell::Cell;
use tcl_registry::native_dictionary_compilation::NativeDictionaryMutationKind as Kind;
thread_local! {
    static STAGE: Cell<u8> = const { Cell::new(0) };
    static CHANGE: Cell<u8> = const { Cell::new(0) };
    static MUTATING: Cell<bool> = const { Cell::new(false) };
    static DUPLICATES: Cell<usize> = const { Cell::new(0) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static SOURCE_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static GETTER_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
}
fn change(original: *mut TclObj) {
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match CHANGE.get() {
        1 => {
            actual.refuse_host_command("original compiled Number stage");
        }
        2 => {
            let original = actual.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("compiled-number-original-frame".to_owned(), "1".to_owned())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&original).unwrap();
        }
        3 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        4 => {
            MUTATING.set(true);
            obj::change_type(original, core::ptr::null(), 0);
            obj::change_type(original, &NUMBER, 0);
            MUTATING.set(false);
        }
        _ => {}
    }
}
extern "C" fn duplicate(source: *mut TclObj, incoming: *mut TclObj) {
    DUPLICATES.set(DUPLICATES.get() + 1);
    SOURCE_REFS.set(unsafe { (*source).ref_count });
    obj::change_type(incoming, &NUMBER, 0);
    if STAGE.get() == 1 {
        change(source);
    }
}
extern "C" fn update(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    GETTER_REFS.set(unsafe { (*original).ref_count });
    if STAGE.get() == 2 {
        change(original);
    }
    unsafe {
        obj::set_native_updater_string_rep(original, b"40", false);
    }
}
extern "C" fn free(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    if MUTATING.get() {
        return;
    }
    if STAGE.get() == 3 {
        change(original);
    } else if STAGE.get() != 0 {
        let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        actual.refuse_host_command("later compiled Number working retirement");
    }
}
static NUMBER: TclObjType = TclObjType {
    name: c"originalCompiledNumberFrame".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        STAGE.set(0);
        CHANGE.set(0);
        MUTATING.set(false);
    }
}
fn exercise(profile: &str, stage: u8, mutation: u8) {
    let mut interp = fixture::native(profile);
    assert_eq!(
        interp.eval_str(b"proc p {} {set d {K 40}; dict incr d K 2; return $d}"),
        Code::Ok
    );
    assert_eq!(interp.eval_str(b"p"), Code::Ok);
    let proc = interp.proc_def(b"p").unwrap();
    let artifact = cache(proc.body.as_ptr()).expect("actual compiled Dictionary Incr");
    let instruction = artifact
        .scripts
        .values()
        .flat_map(|script| &script.commands)
        .find_map(|command| match &command.operation {
            Operation::DictionaryMutation(operation)
                if matches!(operation.recipe.kind, Kind::Incr(2)) =>
            {
                Some(operation)
            }
            _ => None,
        })
        .expect("actual original Dictionary Incr instruction");
    {
        let mut frames = interp.frames.borrow_mut();
        frames.push(crate::namespace::GLOBAL);
        frames
            .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
            .unwrap();
    }
    let _current = fixture::install(&mut interp, 0);
    let _reset = Reset;
    STAGE.set(0);
    let key = fixture::string(b"K");
    let member = obj::Owned::fresh(obj::alloc_typed(&NUMBER, 0));
    let header = obj::NativeObjectLifetime::retain(member.as_ptr());
    let dictionary = obj::Owned::fresh(
        crate::dict::new_dict_obj_native(
            &[(key.as_ptr(), member.as_ptr())],
            None,
            artifact.stamp.source_protocol,
        )
        .unwrap(),
    );
    drop(member);
    let target = EvaluatedTarget {
        root: instruction.receiver.clone(),
        element: None,
        original_name: None,
        original_index: None,
        combined: false,
    };
    interp
        .body_store_value(&target, Some(instruction.slot), &dictionary)
        .unwrap();
    let operation = BodyObjectOperation::capture(&interp).unwrap();
    interp.set_result_bytes(b"PRIOR");
    let prior = interp.get_obj_result();
    DUPLICATES.set(0);
    GETTERS.set(0);
    FREES.set(0);
    SOURCE_REFS.set(-1);
    GETTER_REFS.set(-1);
    CHANGE.set(mutation);
    STAGE.set(stage);
    let reached = interp.body_dictionary_mutation_update(instruction, &[key.as_ptr()], &operation);
    assert_eq!(DUPLICATES.get(), 1);
    assert_eq!(SOURCE_REFS.get(), 2); // actual original/prepared Dictionary member roles
    assert_eq!(unsafe { (*header.as_ptr()).ref_count }, 1);
    assert_eq!(GETTERS.get(), usize::from(stage != 1));
    if stage != 1 {
        assert_eq!(GETTER_REFS.get(), 1);
    } // actual working owner; no protective Native hold
    if stage == 0 {
        assert_eq!(reached.unwrap(), Code::Ok);
        operation.check(&interp).unwrap();
        assert_eq!(FREES.get(), 1);
        assert_eq!(
            operation
                .bytes(&interp, interp.get_obj_result())
                .unwrap()
                .as_ref(),
            b"K 42"
        );
    } else {
        assert_eq!(reached.unwrap_err(), Code::Error);
        assert!(operation.check(&interp).is_err());
        assert_eq!(interp.get_obj_result(), prior);
        assert_eq!(FREES.get(), if mutation == 4 { 2 } else { 1 });
        let first = interp.native_execution_refusal().unwrap();
        if mutation == 1 {
            assert!(
                matches!(first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref failure)
                if failure.reason == "original compiled Number stage")
            );
        } else {
            assert!(matches!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
            ));
        }
    }
    STAGE.set(0);
    CHANGE.set(0);
}
#[test]
fn original_compiled_increment_checks_cow_number_string_and_retirement_before_later_stages() {
    // naming.embedding.original-host-publication-and-fact-transport

    // Actual local compiled callbacks and reference roles; no external Native chronology.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for stage in [1, 2, 3] {
            for mutation in [1, 2, 3] {
                exercise(profile, stage, mutation);
            }
        }
        exercise(profile, 2, 4);
    }
}
#[test]
fn original_compiled_increment_keeps_actual_number_recipe_and_working_reference_schedule() {
    // naming.embedding.original-host-publication-and-fact-transport

    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        exercise(profile, 0, 0);
    }
}
