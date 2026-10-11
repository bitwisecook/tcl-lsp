// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim root scheduling and captured storage, independent of native runs.
use super::*;
use std::cell::{Cell, RefCell};
use tcl_registry::native_dictionary::NativeDictionaryRootPurpose as Purpose;

thread_local! {
    static OWNER: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static READS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}
fn native() -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect("jim"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn text(value: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(value))
}
fn arm(interpreter: &Interp, mode: u8) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.clone()));
    MODE.with(|value| value.set(mode));
    READS.with(|value| value.set(0));
    FREES.with(|value| value.set(0));
    Cleanup
}
fn effect() {
    let mode = MODE.with(Cell::get);
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().clone());
    match mode {
        1 => {
            interpreter.refuse_host_command("first original Jim dictionary stage");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("original-jim-dictionary".to_owned(), "1.0".to_owned())];
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            assert!(interpreter.var_unset(b"a"));
            let replacement = text(b"REPLACEMENT");
            interpreter.var_set(b"a", replacement.as_ptr()).unwrap();
        }
        _ => {}
    }
}
extern "C" fn update(value: *mut TclObj) {
    READS.with(|reads| reads.set(reads.get() + 1));
    effect();
    unsafe { obj::set_native_updater_string_rep(value, b"k", false) };
}
extern "C" fn free(_: *mut TclObj) {
    FREES.with(|frees| frees.set(frees.get() + 1));
    effect();
}
static LAZY: obj::TclObjType = obj::TclObjType {
    name: c"originalJimDictionaryKey".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
static RETIRE: obj::TclObjType = obj::TclObjType {
    name: c"originalJimDictionaryPrimary".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn success<T>(result: Result<T, JimDictionaryPathFailure>) -> T {
    match result {
        Ok(value) => value,
        Err(JimDictionaryPathFailure::Command(error)) => {
            panic!("unexpected command failure: {error:?}")
        }
        Err(JimDictionaryPathFailure::Guest(code)) => panic!("unexpected completion: {code:?}"),
        Err(JimDictionaryPathFailure::MissingKey(key)) => panic!("unexpected missing key: {key:?}"),
    }
}
fn assert_original_cause(interpreter: &Interp, mode: u8) {
    let cause = interpreter
        .native_execution_refusal()
        .expect("original typed stage cause");
    match mode {
        1 => {
            let tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) = cause else {
                panic!("original Host cause was lost");
            };
            assert_eq!(first.reason, "first original Jim dictionary stage");
        }
        2 => assert_eq!(
            cause,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "stale entered native operation"
                ),
            )
        ),
        3 => assert!(matches!(
            cause,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        )),
        _ => panic!("unselected refusal mode"),
    }
}
fn current(interpreter: &Interp, name: &[u8]) -> Option<*mut TclObj> {
    crate::vars::get(
        &interpreter.frames.borrow(),
        &interpreter.namespaces(),
        interpreter.current_ns(),
        name,
    )
}
fn path(
    interpreter: &mut Interp,
    name: *mut TclObj,
    keys: &[*mut TclObj],
    value: Option<*mut TclObj>,
    purpose: Purpose,
) -> Result<Option<*mut TclObj>, JimDictionaryPathFailure> {
    let mut argv = vec![name];
    argv.extend_from_slice(keys);
    argv.extend(value);
    let arguments = OriginalArguments::enter(interpreter, &argv).unwrap();
    let result =
        interpreter.mutate_original_jim_dictionary_path(name, keys, value, purpose, &arguments);
    drop(arguments);
    result
}

#[test]
fn original_jim_root_sharing_precedes_path_conversion_and_follows_array_conversion() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software stages justified by Jim_SetDictKeysVector and jim-array.c;
    // no provider observation or native reference equivalence is inferred.
    for purpose in [Purpose::VariableSet, Purpose::ArraySet] {
        let mut interpreter = native();
        let _cleanup = arm(&interpreter, 0);
        let name = text(b"a");
        let key = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        let old = text(b"1");
        let source = Owned::fresh(interpreter.new_list_object(&[key.as_ptr(), old.as_ptr()]));
        interpreter
            .assign_original_named_variable(name.as_ptr(), source.as_ptr())
            .unwrap();
        assert_eq!(unsafe { (*source.as_ptr()).ref_count }, 2);
        assert!(!obj::has_string_rep(source.as_ptr()));
        let arguments = OriginalArguments::enter(&interpreter, &[name.as_ptr()]).unwrap();
        let captured = interpreter
            .capture_original_jim_dictionary_root(name.as_ptr(), arguments.currency())
            .unwrap()
            .unwrap();
        assert_eq!(unsafe { (*source.as_ptr()).ref_count }, 2);
        let selected = interpreter
            .native_invocation_dialect()
            .native_dictionary_root_protocol(purpose)
            .unwrap();
        let prepared = crate::dict::EnteredNativeDictionary::prepare_original(
            captured.original(),
            selected,
            NativeStringProtocol::Jim084,
            arguments.currency(),
            captured.stage_check(),
        )
        .unwrap();
        assert_ne!(prepared.as_ptr(), source.as_ptr());
        assert_eq!(unsafe { (*prepared.as_ptr()).ref_count }, 0);
        assert_eq!(READS.with(Cell::get), 1);
        if purpose == Purpose::VariableSet {
            assert_eq!(
                obj::obj_type_ptr(source.as_ptr()),
                &crate::list::TCL_LIST_TYPE as *const obj::TclObjType
            );
            assert!(!obj::has_string_rep(source.as_ptr()));
        } else {
            assert_eq!(
                obj::obj_type_ptr(source.as_ptr()),
                &crate::dict::TCL_DICT_TYPE as *const obj::TclObjType
            );
            assert!(obj::has_string_rep(source.as_ptr()));
        }
        captured.publish(prepared.as_ptr()).unwrap();
        assert_eq!(current(&interpreter, b"a"), Some(prepared.as_ptr()));
        assert_eq!(unsafe { (*source.as_ptr()).ref_count }, 1);
        assert_eq!(unsafe { (*prepared.as_ptr()).ref_count }, 1);
        drop(prepared);
        drop(captured);
        drop(arguments);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_jim_missing_set_generic_unset_and_required_unset_have_distinct_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for purpose in [
        Purpose::VariableSet,
        Purpose::VariableUnset,
        Purpose::ExistingVariableUnset,
    ] {
        let mut interpreter = native();
        let name = text(b"missing");
        let key = text(b"k");
        let value = text(b"V");
        let supplied = (purpose == Purpose::VariableSet).then_some(value.as_ptr());
        let result = success(path(
            &mut interpreter,
            name.as_ptr(),
            &[key.as_ptr()],
            supplied,
            purpose,
        ));
        if purpose == Purpose::ExistingVariableUnset {
            assert!(result.is_none());
            assert!(current(&interpreter, b"missing").is_none());
            assert!(obj::obj_type_ptr(name.as_ptr()).is_null());
        } else {
            let root = result.expect("actual published root");
            assert_eq!(current(&interpreter, b"missing"), Some(root));
            assert_eq!(
                crate::dict::original_cached_member(root, b"k").unwrap(),
                supplied
            );
            assert_eq!(unsafe { (*root).ref_count }, 1);
        }
        assert!(interpreter.native_execution_refusal().is_none());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

#[test]
fn original_jim_copied_child_is_published_before_later_conversion_failure() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = native();
    let _cleanup = arm(&interpreter, 0);
    let name = text(b"a");
    let head = text(b"head");
    let x = text(b"x");
    let one = text(b"1");
    let odd = text(b"odd");
    let child =
        Owned::fresh(interpreter.new_list_object(&[x.as_ptr(), one.as_ptr(), odd.as_ptr()]));
    let root = Owned::fresh(
        crate::dict::new_dict_obj_native(
            &[(head.as_ptr(), child.as_ptr())],
            None,
            NativeStringProtocol::Jim084,
        )
        .unwrap(),
    );
    interpreter
        .assign_original_named_variable(name.as_ptr(), root.as_ptr())
        .unwrap();
    let pointer = root.as_ptr();
    let lease = obj::NativeObjectLifetime::retain(pointer);
    drop(root);
    let later = Owned::fresh(obj::alloc_typed(&LAZY, 0));
    let value = text(b"V");
    let result = path(
        &mut interpreter,
        name.as_ptr(),
        &[head.as_ptr(), later.as_ptr()],
        Some(value.as_ptr()),
        Purpose::VariableSet,
    );
    assert!(
        matches!(result, Err(JimDictionaryPathFailure::Command(ref error)) if error.native_execution_refusal().is_none())
    );
    assert_eq!(current(&interpreter, b"a"), Some(pointer));
    let published = crate::dict::original_cached_member(pointer, b"head")
        .unwrap()
        .unwrap();
    assert_ne!(published, child.as_ptr());
    assert_eq!(
        obj::obj_type_ptr(published),
        &crate::list::TCL_LIST_TYPE as *const obj::TclObjType
    );
    assert_eq!(unsafe { (*published).ref_count }, 1);
    assert_eq!(unsafe { (*child.as_ptr()).ref_count }, 1);
    assert!(!obj::has_string_rep(child.as_ptr()));
    assert_eq!(READS.with(Cell::get), 0);
    assert!(!obj::has_string_rep(later.as_ptr()));
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    drop(lease);
}

#[test]
fn original_jim_dictionary_getter_refusal_stops_later_keys_and_never_adopts_a_recreated_cell() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2, 3] {
        let mut interpreter = native();
        let _cleanup = arm(&interpreter, mode);
        let name = text(b"a");
        let root = Owned::fresh(
            crate::dict::new_dict_obj_native(&[], None, NativeStringProtocol::Jim084).unwrap(),
        );
        interpreter
            .assign_original_named_variable(name.as_ptr(), root.as_ptr())
            .unwrap();
        let pointer = root.as_ptr();
        let lease = obj::NativeObjectLifetime::retain(pointer);
        drop(root);
        let prior = text(b"PRIOR");
        interpreter.set_result(prior.as_ptr());
        let first = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        let later = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        let value = text(b"V");
        let result = path(
            &mut interpreter,
            name.as_ptr(),
            &[first.as_ptr(), later.as_ptr()],
            Some(value.as_ptr()),
            Purpose::VariableSet,
        );
        let failure = match result {
            Err(error) => error,
            Ok(_) => panic!("reached getter must refuse"),
        };
        assert_eq!(READS.with(Cell::get), 1);
        assert!(obj::has_string_rep(first.as_ptr()));
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(failure.report(&mut interpreter), Code::Error);
        assert_original_cause(&interpreter, mode);
        let cause = interpreter
            .native_execution_refusal()
            .expect("first original cause");
        assert!(interpreter.var_get(b"a").is_none());
        assert_eq!(interpreter.native_execution_refusal(), Some(cause));
        if mode == 3 {
            let replacement = current(&interpreter, b"a").unwrap();
            assert_ne!(replacement, pointer);
            assert_eq!(obj::bytes_of(replacement), b"REPLACEMENT");
        } else {
            assert_eq!(current(&interpreter, b"a"), Some(pointer));
        }
        assert_eq!(interpreter.result_obj(), prior.as_ptr());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        drop(lease);
    }
}

#[test]
fn original_jim_tuple_retirement_refuses_before_root_lookup_or_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2] {
        let mut interpreter = native();
        let _cleanup = arm(&interpreter, mode);
        let original = Owned::fresh(obj::alloc_typed(&RETIRE, 0));
        unsafe { obj::set_string_rep(original.as_ptr(), b"a(k)") };
        let value = text(b"V");
        let prior = text(b"PRIOR");
        interpreter.set_result(prior.as_ptr());
        assert_eq!(
            interpreter.assign_native_jim_dictionary_sugar(original.as_ptr(), value.as_ptr()),
            Err(Code::Error)
        );
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(READS.with(Cell::get), 0);
        assert!(
            native_substitution::with_dictionary_substitution(original.as_ptr(), |_, _| ())
                .is_none()
        );
        assert!(obj::obj_type_ptr(original.as_ptr()).is_null());
        assert!(current(&interpreter, b"a").is_none());
        assert_original_cause(&interpreter, mode);
        assert_eq!(interpreter.result_obj(), prior.as_ptr());
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

fn update_original_member(interpreter: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let arguments = match OriginalArguments::enter(interpreter, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interpreter.report_cmd_error(error),
    };
    let reached = interpreter.mutate_original_jim_dictionary_path(
        argv[1],
        &argv[2..3],
        Some(argv[3]),
        Purpose::VariableSet,
        &arguments,
    );
    match reached {
        Ok(Some(root)) => {
            interpreter.set_result(root);
            match arguments.check() {
                Ok(()) => Code::Ok,
                Err(error) => interpreter.report_cmd_error(error),
            }
        }
        Ok(None) => panic!("actual member update had no root"),
        Err(error) => error.report(interpreter),
    }
}
#[test]
fn original_jim_member_root_and_retained_alias_publish_through_the_same_original_parent() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interpreter = native();
    interpreter.register_builtin(b"original_member_update", update_original_member);
    assert_eq!(interpreter.eval_str(b"set a [dict create slot [dict create old 1]]; proc update {} {upvar #0 a alias; original_member_update alias(slot) new 2}; update"), Code::Ok);
    let root = current(&interpreter, b"a").unwrap();
    let member = crate::dict::original_cached_member(root, b"slot")
        .unwrap()
        .unwrap();
    let new = crate::dict::original_cached_member(member, b"new")
        .unwrap()
        .unwrap();
    assert_eq!(obj::bytes_of(new), b"2");
    assert!(interpreter.native_execution_refusal().is_none());
    assert_eq!(interpreter.entered_native_operation_count(), 0);
    assert_eq!(interpreter.current_level(), 0);
}

#[test]
fn original_jim_cached_dictionary_and_scalar_roots_keep_their_selected_copy_roles() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for cached in [false, true] {
        for held in [false, true] {
            let mut interpreter = native();
            let name = text(b"a");
            let key = text(b"k");
            let old = text(b"OLD");
            let source = if cached {
                Owned::fresh(
                    crate::dict::new_dict_obj_native(
                        &[(key.as_ptr(), old.as_ptr())],
                        None,
                        NativeStringProtocol::Jim084,
                    )
                    .unwrap(),
                )
            } else {
                text(b"k OLD")
            };
            let source_ptr = source.as_ptr();
            interpreter
                .assign_original_named_variable(name.as_ptr(), source_ptr)
                .unwrap();
            if !held {
                drop(source);
            }
            let native_refs = unsafe { (*source_ptr).ref_count };
            assert_eq!(native_refs, if held { 2 } else { 1 });
            let new = text(b"NEW");
            let updated = success(path(
                &mut interpreter,
                name.as_ptr(),
                &[key.as_ptr()],
                Some(new.as_ptr()),
                Purpose::VariableSet,
            ))
            .unwrap();
            assert_eq!(updated == source_ptr, !held);
            assert_eq!(current(&interpreter, b"a"), Some(updated));
            assert_eq!(
                obj::obj_type_ptr(updated),
                &crate::dict::TCL_DICT_TYPE as *const obj::TclObjType
            );
            assert_eq!(
                crate::dict::original_cached_member(updated, b"k").unwrap(),
                Some(new.as_ptr())
            );
            if held {
                assert_eq!(unsafe { (*source_ptr).ref_count }, 1);
                assert_eq!(
                    obj::obj_type_ptr(source_ptr),
                    if cached {
                        &crate::dict::TCL_DICT_TYPE
                    } else {
                        core::ptr::null()
                    }
                );
                if cached {
                    assert_eq!(
                        crate::dict::original_cached_member(source_ptr, b"k").unwrap(),
                        Some(old.as_ptr())
                    );
                } else {
                    assert_eq!(obj::bytes_of(source_ptr), b"k OLD");
                }
            }
            assert_eq!(unsafe { (*updated).ref_count }, 1);
            assert!(interpreter.native_execution_refusal().is_none());
            assert_eq!(interpreter.entered_native_operation_count(), 0);
        }
    }
}

#[test]
fn original_jim_variable_cache_retirement_refuses_before_root_conversion_and_key_getter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2] {
        let mut interpreter = native();
        let key = text(b"k");
        let old = text(b"OLD");
        let root = Owned::fresh(
            crate::dict::new_dict_obj_native(
                &[(key.as_ptr(), old.as_ptr())],
                None,
                NativeStringProtocol::Jim084,
            )
            .unwrap(),
        );
        interpreter.var_set(b"a", root.as_ptr()).unwrap();
        let name = Owned::fresh(obj::alloc_typed(&RETIRE, 0));
        unsafe { obj::set_native_updater_string_rep(name.as_ptr(), b"a", false) };
        let later = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        let value = text(b"V");
        interpreter.set_result_bytes(b"CACHE PRIOR");
        let prior = interpreter.get_obj_result();
        let _cleanup = arm(&interpreter, mode);
        let failure = path(
            &mut interpreter,
            name.as_ptr(),
            &[later.as_ptr()],
            Some(value.as_ptr()),
            Purpose::VariableSet,
        )
        .err()
        .expect("actual variable-cache retirement refuses");
        assert_eq!(failure.report(&mut interpreter), Code::Error);
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(READS.with(Cell::get), 0);
        assert_eq!(obj::obj_type_ptr(name.as_ptr()), core::ptr::null());
        assert!(!obj::has_string_rep(later.as_ptr()));
        assert_eq!(current(&interpreter, b"a"), Some(root.as_ptr()));
        assert_eq!(
            crate::dict::original_cached_member(root.as_ptr(), b"k").unwrap(),
            Some(old.as_ptr())
        );
        assert_eq!(unsafe { (*root.as_ptr()).ref_count }, 2);
        assert_eq!(interpreter.get_obj_result(), prior);
        assert_original_cause(&interpreter, mode);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
    }
}

thread_local! {
    static PREPARED_ROOT: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static PREPARED_MODE: Cell<u8> = const { Cell::new(0) };
}
extern "C" fn shimmer_prepared_key(original: *mut TclObj) {
    READS.with(|reads| reads.set(reads.get() + 1));
    let mode = PREPARED_MODE.with(Cell::get);
    if mode != 0 {
        let parent = PREPARED_ROOT.with(Cell::get);
        assert!(!parent.is_null());
        // Reached original List conversion retires the actual prepared Dict;
        // its cell/context identity remains unchanged. Mode2 installs a new
        // actual Dict backing at the same header, not the entered incarnation.
        let members =
            crate::list::list_elements_native_checked(parent, NativeStringProtocol::Jim084)
                .unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(
            obj::obj_type_ptr(parent),
            &crate::list::TCL_LIST_TYPE as *const obj::TclObjType
        );
        if mode == 2 {
            crate::dict::ensure_dict_native(parent, NativeStringProtocol::Jim084).unwrap();
            assert_eq!(
                obj::obj_type_ptr(parent),
                &crate::dict::TCL_DICT_TYPE as *const obj::TclObjType
            );
        }
    }
    unsafe { obj::set_native_updater_string_rep(original, b"k", false) };
}
static SHIMMER_KEY: obj::TclObjType = obj::TclObjType {
    name: c"originalPreparedDictionaryKey".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(shimmer_prepared_key),
    set_from_any_proc: None,
};
#[test]
fn original_jim_prepared_backing_refuses_key_shimmer_and_restore_before_typed_member_access() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software continuation control. No original native callback timing or
    // backing identity measurement is claimed from these custom updaters.
    for operation in 0..4 {
        for mode in 0..3 {
            let mut interpreter = native();
            let _cleanup = arm(&interpreter, 0);
            let name = text(b"a");
            let old_key = text(b"k");
            let old_value = text(b"old");
            let value = text(b"new");
            let root = crate::dict::new_dict_obj_native(
                &[(old_key.as_ptr(), old_value.as_ptr())],
                None,
                NativeStringProtocol::Jim084,
            )
            .unwrap();
            let lease = obj::NativeObjectLifetime::retain(root);
            interpreter
                .assign_original_named_variable(name.as_ptr(), root)
                .unwrap();
            assert_eq!(unsafe { (*root).ref_count }, 1);
            let key = Owned::fresh(obj::alloc_typed(&SHIMMER_KEY, 0));
            let arguments = OriginalArguments::enter(
                &interpreter,
                &[name.as_ptr(), key.as_ptr(), value.as_ptr()],
            )
            .unwrap();
            let captured = interpreter
                .capture_original_jim_dictionary_root(name.as_ptr(), arguments.currency())
                .unwrap()
                .unwrap();
            let selected = interpreter
                .native_invocation_dialect()
                .native_dictionary_root_protocol(Purpose::VariableSet)
                .unwrap();
            let mut prepared = crate::dict::EnteredNativeDictionary::prepare_original(
                root,
                selected,
                NativeStringProtocol::Jim084,
                arguments.currency(),
                captured.stage_check(),
            )
            .unwrap();
            assert_eq!(prepared.as_ptr(), root);
            assert_eq!(unsafe { (*root).ref_count }, 1);
            interpreter.set_result_bytes(b"PRIOR");
            let prior = Owned::retain(interpreter.get_obj_result());
            let context = interpreter.runtime_context();
            let value_before = obj::native_object_snapshot(value.as_ptr()).unwrap();
            PREPARED_ROOT.with(|slot| slot.set(root));
            PREPARED_MODE.with(|slot| slot.set(mode));
            let reached = match operation {
                0 => prepared.get_member(key.as_ptr()).map(|original| {
                    if mode == 0 {
                        assert_eq!(original, Some(old_value.as_ptr()));
                    }
                }),
                1 => prepared.capture_member(key.as_ptr()).map(|member| {
                    if mode == 0 {
                        assert_eq!(member.original(), Some(old_value.as_ptr()));
                    }
                }),
                2 => prepared.set_member(key.as_ptr(), value.as_ptr()),
                _ => prepared.remove_member(key.as_ptr()).map(|removed| {
                    assert!(removed);
                }),
            };
            assert_eq!(READS.with(Cell::get), 1);
            assert!(obj::has_string_rep(key.as_ptr()));
            assert_eq!(current(&interpreter, b"a"), Some(root));
            assert_eq!(interpreter.runtime_context(), context);
            if mode == 0 {
                assert!(reached.is_ok());
                assert!(interpreter.native_execution_refusal().is_none());
            } else {
                let error = match reached {
                    Err(error) => error,
                    Ok(()) => panic!("changed prepared backing reached typed member access"),
                };
                assert!(matches!(
                    &error,
                    ValueError::CommandProtocolUnavailable(
                        "prepared original Jim Dictionary backing"
                    )
                ));
                assert_eq!(interpreter.report_cmd_error(error.into()), Code::Error);
                assert_eq!(
                    interpreter.native_execution_refusal(),
                    Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                            "prepared original Jim Dictionary backing",
                        ),
                    )),
                );
                assert_eq!(
                    obj::native_object_snapshot(value.as_ptr()).unwrap(),
                    value_before
                );
                if mode == 1 {
                    assert_eq!(
                        obj::obj_type_ptr(root),
                        &crate::list::TCL_LIST_TYPE as *const obj::TclObjType
                    );
                } else {
                    assert_eq!(
                        obj::obj_type_ptr(root),
                        &crate::dict::TCL_DICT_TYPE as *const obj::TclObjType
                    );
                }
            }
            assert_eq!(interpreter.get_obj_result(), prior.as_ptr());
            assert_eq!(unsafe { (*root).ref_count }, 1);
            drop(prepared);
            drop(captured);
            drop(arguments);
            assert_eq!(interpreter.entered_native_operation_count(), 0);
            PREPARED_ROOT.with(|slot| slot.set(core::ptr::null_mut()));
            PREPARED_MODE.with(|slot| slot.set(0));
            drop(lease);
        }
    }
}
