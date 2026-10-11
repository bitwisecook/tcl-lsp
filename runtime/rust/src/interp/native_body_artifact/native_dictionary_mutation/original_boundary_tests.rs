// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine compiled member COW, working retirement and original frame controls.
use super::super::native_list_conversion::BodyObjectOperation;
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use crate::obj::TclObjType;
use std::cell::Cell;
use tcl_registry::native_dictionary_compilation::NativeDictionaryMutationKind as Kind;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static INCOMING: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static DUPLICATES: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
}
extern "C" fn duplicate(source: *mut TclObj, incoming: *mut TclObj) {
    DUPLICATES.set(DUPLICATES.get() + 1);
    INCOMING.set(incoming);
    REFS.set(unsafe { (*source).ref_count });
    obj::change_type(incoming, &MEMBER, 0);
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.get() {
        1 => {
            actual.refuse_host_command("original Dictionary member COW");
        }
        2 => {
            let original = actual.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("original-dictionary-cow".to_owned(), "1".to_owned())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&original).unwrap();
        }
        3 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        4 => {
            // Genuine source-primary replacement and restoration, before the
            // checked duplicator retires its unadopted incoming copy.
            obj::change_type(source, core::ptr::null(), 0);
            obj::change_type(source, &MEMBER, 0);
        }
        _ => {}
    }
}
extern "C" fn free(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    if MODE.get() != 0 && (MODE.get() != 4 || INCOMING.get() == original) {
        let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
            .unwrap()
            .clone();
        actual.refuse_host_command("later unused Dictionary working release");
    }
}
extern "C" fn later(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    unsafe {
        obj::set_native_updater_string_rep(original, b"B", false);
    }
}
static MEMBER: TclObjType = TclObjType {
    name: c"originalDictionaryMemberCOW".as_ptr(),
    free_int_rep_proc: Some(free),
    dup_int_rep_proc: Some(duplicate),
    update_string_proc: None,
    set_from_any_proc: None,
};
static LATER: TclObjType = TclObjType {
    name: c"originalDictionaryMutationLater".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later),
    set_from_any_proc: None,
};
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.set(0);
        INCOMING.set(core::ptr::null_mut());
    }
}
fn exercise(profile: &str, kind: Kind, mode: u8) {
    let mut interp = fixture::native(profile);
    let source: &[u8] = if kind == Kind::Append {
        b"proc p {} {set d {K A}; dict append d K B; return $d}"
    } else {
        b"proc p {} {set d {K A}; dict lappend d K B; return $d}"
    };
    assert_eq!(interp.eval_str(source), Code::Ok);
    assert_eq!(interp.eval_str(b"p"), Code::Ok);
    let proc = interp.proc_def(b"p").unwrap();
    let artifact = cache(proc.body.as_ptr()).expect("actual compiled Dictionary mutation");
    let instruction = artifact
        .scripts
        .values()
        .flat_map(|script| &script.commands)
        .find_map(|command| match &command.operation {
            Operation::DictionaryMutation(operation) if operation.recipe.kind == kind => {
                Some(operation)
            }
            _ => None,
        })
        .expect("actual original Dictionary member instruction");
    {
        let mut frames = interp.frames.borrow_mut();
        frames.push(crate::namespace::GLOBAL);
        frames
            .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
            .unwrap();
    }
    let _current = fixture::install(&mut interp, 0);
    let _reset = Reset;
    MODE.set(0);
    let key = fixture::string(b"K");
    let member = obj::Owned::fresh(obj::alloc_typed(&MEMBER, 0));
    unsafe {
        obj::set_native_updater_string_rep(member.as_ptr(), b"A", false);
    }
    let member_header = obj::NativeObjectLifetime::retain(member.as_ptr());
    let protocol = artifact.stamp.source_protocol;
    let dictionary = obj::Owned::fresh(
        crate::dict::new_dict_obj_native(&[(key.as_ptr(), member.as_ptr())], None, protocol)
            .unwrap(),
    );
    // The actual source dictionary owns this member; the observer is allocation-only.
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
    let source = obj::Owned::fresh(obj::alloc_typed(&LATER, 0));
    let operation = BodyObjectOperation::capture(&interp).unwrap();
    interp.set_result_bytes(b"PRIOR");
    let prior = interp.get_obj_result();
    DUPLICATES.set(0);
    FREES.set(0);
    REFS.set(-1);
    GETTERS.set(0);
    MODE.set(mode);
    let reached = interp.body_dictionary_mutation_update(
        instruction,
        &[key.as_ptr(), source.as_ptr()],
        &operation,
    );
    assert_eq!(DUPLICATES.get(), 1);
    assert_eq!(REFS.get(), 2); // real original and prepared dictionary member roles
    assert_eq!(unsafe { (*member_header.as_ptr()).ref_count }, 1);
    assert_eq!(FREES.get(), if mode == 4 { 2 } else { 1 }); // reached source and working retirement
    if mode == 0 {
        assert_eq!(reached.unwrap(), Code::Ok);
        operation.check(&interp).unwrap();
        assert_eq!(GETTERS.get(), usize::from(kind == Kind::Append));
        let bytes = operation.bytes(&interp, interp.get_obj_result()).unwrap();
        assert_eq!(
            bytes.as_ref(),
            if kind == Kind::Append {
                b"K AB".as_slice()
            } else {
                b"K {A B}"
            }
        );
    } else {
        assert_eq!(reached.unwrap_err(), Code::Error);
        assert!(operation.check(&interp).is_err());
        assert_eq!(GETTERS.get(), 0);
        assert_eq!(interp.get_obj_result(), prior);
        let first = interp.native_execution_refusal().unwrap();
        if mode == 1 {
            assert!(
                matches!(first, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref failure)
                if failure.reason == "original Dictionary member COW")
            );
        } else {
            assert!(matches!(
                first,
                tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
            ));
        }
    }
    MODE.set(0);
}
#[test]
fn original_compiled_append_and_lappend_check_member_cow_before_working_cleanup_and_getters() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Local extension/actual compiled worker controls only; no native callback chronology.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for kind in [Kind::Append, Kind::Lappend] {
            for mode in [1, 2, 3, 4] {
                exercise(profile, kind, mode);
            }
        }
    }
}
#[test]
fn original_compiled_append_and_lappend_keep_current_reference_and_deferred_member_schedule() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for kind in [Kind::Append, Kind::Lappend] {
            exercise(profile, kind, 0);
        }
    }
}

struct WriteRole {
    selected: Rc<crate::interp::native_variable_names::OriginalCVariableCapture>,
    pending: bool,
    calls: Cell<usize>,
    published: Cell<*mut TclObj>,
    cleanup: std::cell::RefCell<Option<Box<dyn FnOnce()>>>,
    currency: crate::interp::native_operation_currency::NativeOperationCurrency,
}
impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Interp> for WriteRole {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _actual: &mut Interp,
        access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(
            access.operation,
            tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write
        );
        self.calls.set(self.calls.get() + 1);
        let incoming = obj::new_string_bytes(b"TRACE_REPLACEMENT");
        self.published.set(incoming);
        if self.pending {
            let current = self.selected.receiver.read_initial().unwrap().unwrap();
            let (prepared, retired) = self
                .selected
                .receiver
                .publish_c84_pending_scalar(current, incoming)
                .unwrap();
            retired
                .finish(|| self.currency.ensure_current_or_refuse())
                .unwrap();
            let selected = self.selected.clone();
            let currency = self.currency.clone();
            *self.cleanup.borrow_mut() = Some(Box::new(move || {
                let _ = selected
                    .receiver
                    .discard_c84_pending_scalar(&prepared)
                    .finish(|| currency.ensure_current_or_refuse());
            }));
        } else {
            self.selected.receiver.store(incoming).unwrap();
        }
        Ok(())
    }
}
#[test]
fn original_compiled_mutation_refuses_pending_write_role_and_preserves_live_trace_replacement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Pending-role publication is a software callback transport control, not a
    // modern Native setter schedule or ordinary scalar authority grant.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for pending in [false, true] {
            let mut interp = fixture::native(profile);
            assert_eq!(
                interp.eval_str(b"proc p {} {set d {K A}; dict set d K B; return $d}"),
                Code::Ok
            );
            assert_eq!(interp.eval_str(b"p"), Code::Ok);
            let proc = interp.proc_def(b"p").unwrap();
            let artifact = cache(proc.body.as_ptr()).unwrap();
            let instruction = artifact
                .scripts
                .values()
                .flat_map(|script| &script.commands)
                .find_map(|command| match &command.operation {
                    Operation::DictionaryMutation(operation)
                        if operation.recipe.kind == Kind::Set =>
                    {
                        Some(operation)
                    }
                    _ => None,
                })
                .unwrap();
            {
                let mut frames = interp.frames.borrow_mut();
                frames.push(crate::namespace::GLOBAL);
                frames
                    .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
                    .unwrap();
            }
            let _current = fixture::install(&mut interp, 0);
            let initial = fixture::string(b"K A");
            let target = EvaluatedTarget {
                root: instruction.receiver.clone(),
                element: None,
                original_name: None,
                original_index: None,
                combined: false,
            };
            interp
                .body_store_value(&target, Some(instruction.slot), &initial)
                .unwrap();
            let selected = Rc::new(
                interp
                    .body_capture_target(&target, Some(instruction.slot), true)
                    .unwrap(),
            );
            let operation = BodyObjectOperation::capture(&interp).unwrap();
            let currency =
                crate::interp::native_operation_currency::NativeOperationCurrency::issue(&interp)
                    .unwrap();
            let observer = Rc::new(WriteRole {
                selected: selected.clone(),
                pending,
                calls: Cell::new(0),
                published: Cell::new(core::ptr::null_mut()),
                cleanup: Default::default(),
                currency,
            });
            let name = fixture::string(&instruction.receiver);
            let token = interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation::Write],
                    observer.clone(),
                )
                .unwrap();
            let key = fixture::string(b"K");
            let value = fixture::string(b"B");
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            let reached = interp.body_dictionary_mutation_update(
                instruction,
                &[key.as_ptr(), value.as_ptr()],
                &operation,
            );
            assert_eq!(observer.calls.get(), 1);
            if pending {
                assert_eq!(reached.unwrap_err(), Code::Error);
                assert_eq!(interp.get_obj_result(), prior);
                assert!(operation.check(&interp).is_err());
                assert_eq!(
                    selected.receiver.read_initial(),
                    Err(crate::frame::VarError::NameProtocolUnavailable)
                );
                assert_eq!(unsafe { (*observer.published.get()).ref_count }, 0);
                assert!(matches!(
                    interp.native_execution_refusal(),
                    Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_))
                ));
                let cleanup = observer.cleanup.borrow_mut().take().unwrap();
                cleanup();
                assert!(!obj::allocation_is_live(observer.published.get()));
            } else {
                assert_eq!(reached.unwrap(), Code::Ok);
                operation.check(&interp).unwrap();
                assert_eq!(interp.get_obj_result(), observer.published.get());
                assert_eq!(
                    operation
                        .bytes(&interp, interp.get_obj_result())
                        .unwrap()
                        .as_ref(),
                    b"TRACE_REPLACEMENT"
                );
            }
            interp.remove_native_variable_observer(&token);
        }
    }
}
