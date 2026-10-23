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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual original variable/cache callbacks; software transport only.
use super::native_list_conversion::BodyObjectOperation;
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::{
    native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    },
    NativeExecutionError,
};
use tcl_syntax::native_variable_name::{NativeParsedVariableElement, NativeParsedVariableName};
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
    static ROOT_GETTERS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static PARENT: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static ROOT: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static ELEMENT: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
}
fn actual() -> Interp {
    unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone()
}
fn withdraw() {
    match MODE.get() {
        1 => {
            actual().refuse_host_command("original compiled variable callback");
        }
        2 => {
            let interp = actual();
            let mut frames = interp.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(GLOBAL);
        }
        _ => {}
    }
}
extern "C" fn update_name(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    withdraw();
    unsafe {
        obj::set_native_updater_string_rep(original, b"variable_name", false);
    }
}
extern "C" fn update_root(original: *mut TclObj) {
    ROOT_GETTERS.set(ROOT_GETTERS.get() + 1);
    if MODE.get() == 5 {
        obj::change_type(PARENT.get(), &MARKER, 0);
    }
    unsafe {
        obj::set_native_updater_string_rep(original, b"variable_array", false);
    }
}
extern "C" fn update_element(original: *mut TclObj) {
    GETTERS.set(GETTERS.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    if matches!(MODE.get(), 3 | 4) {
        let protocol = actual().native_c_variable_name_protocol().unwrap();
        // Only mode4's real callback creates incoming cache roles. Mode3
        // releases the original cache's last root/element Native owners.
        let replacement = (MODE.get() == 4).then(|| NativeParsedVariableName {
            protocol,
            array: Some((
                obj::Owned::retain(ROOT.get()),
                NativeParsedVariableElement::Object(obj::Owned::retain(ELEMENT.get())),
            )),
        });
        obj::change_type(PARENT.get(), &MARKER, 0);
        if let Some(replacement) = replacement {
            obj::native_variable_name::install_parsed_with_stage_check::<tcl_cmd_core::CmdError>(
                PARENT.get(),
                replacement,
                actual().native_invocation_dialect(),
                &mut || Ok(()),
                &|_| {},
            )
            .unwrap();
        }
    } else {
        withdraw();
    }
    unsafe {
        obj::set_native_updater_string_rep(original, b"key", false);
    }
}
extern "C" fn free_namespace(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    if MODE.get() == 4 {
        // This callback already owns retirement of the old primary. Replace
        // its fields directly rather than recursively freeing that primary.
        unsafe {
            (*original).type_ptr = &MARKER;
            (*original).internal_rep = 0;
        }
    } else {
        withdraw();
    }
}
static NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledVariableName".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_name),
    set_from_any_proc: None,
};
static ROOT_NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledVariableRoot".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_root),
    set_from_any_proc: None,
};
static ELEMENT_NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledVariableElement".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update_element),
    set_from_any_proc: None,
};
static NAMESPACE_NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledNamespaceName".as_ptr(),
    free_int_rep_proc: Some(free_namespace),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static MARKER: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledVariableMarker".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn reset(mode: u8) {
    MODE.set(mode);
    GETTERS.set(0);
    ROOT_GETTERS.set(0);
    FREES.set(0);
    REFS.set(-1);
    PARENT.set(core::ptr::null_mut());
    ROOT.set(core::ptr::null_mut());
    ELEMENT.set(core::ptr::null_mut());
}
fn dynamic(original: obj::Owned) -> EvaluatedTarget {
    EvaluatedTarget {
        root: b"variable_name".to_vec(),
        element: None,
        original_name: Some(original),
        original_index: None,
        combined: true,
    }
}
fn frame(interp: &Interp) {
    interp.frames.borrow_mut().push(GLOBAL);
}
fn assert_first(interp: &Interp, mode: u8) {
    let first = interp.native_execution_refusal().unwrap();
    if mode == 1 {
        assert!(
            matches!(first,NativeExecutionError::HostCommandRefusal(ref reason) if reason.reason=="original compiled variable callback")
        );
    } else {
        assert!(matches!(first, NativeExecutionError::ValueAccessRefusal(_)));
    }
}
#[test]
fn original_compiled_variable_name_checks_first_string_before_cell_and_cache_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual C worker and software callbacks, no Native timing equivalence.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2] {
            let mut interp = fixture::native(profile);
            frame(&interp);
            let _current = fixture::install(&mut interp, 0);
            reset(mode);
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.result_obj();
            let original = BodyObjectOperation::capture(&interp).unwrap();
            let target = dynamic(obj::Owned::fresh(obj::alloc_typed(&NAME, 0)));
            let reached = interp.body_capture_target_using(&target, None, true, Some(&original));
            assert_eq!(GETTERS.get(), 1);
            assert_eq!(REFS.get(), 1);
            assert_eq!(interp.result_obj(), prior);
            assert!(obj::has_string_rep(
                target.original_name.as_ref().unwrap().as_ptr()
            ));
            if mode == 0 {
                assert!(reached.is_ok());
                original.check(&interp).unwrap();
            } else {
                assert!(matches!(reached, Err(Code::Error)));
                assert_first(&interp, mode);
                assert!(obj::native_variable_name::with_parsed(
                    target.original_name.as_ref().unwrap().as_ptr(),
                    |_| ()
                )
                .is_none());
            }
            MODE.set(0);
        }
    }
}
#[test]
fn original_compiled_parsed_element_projects_last_native_owners_before_uaf_and_primary_aba_callbacks(
) {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // C9 object-element purpose; allocation leases never add Native references.
    for profile in ["tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 3, 4, 5] {
            let mut interp = fixture::native(profile);
            frame(&interp);
            let _current = fixture::install(&mut interp, 0);
            reset(mode);
            let parent = fixture::string(b"variable_array(key)");
            let root = obj::Owned::fresh(obj::alloc_typed(&ROOT_NAME, 0));
            ROOT.set(root.as_ptr());
            let element = obj::Owned::fresh(obj::alloc_typed(&ELEMENT_NAME, 0));
            ELEMENT.set(element.as_ptr());
            PARENT.set(parent.as_ptr());
            let protocol = interp.native_c_variable_name_protocol().unwrap();
            assert!(protocol.new_element_bytes(b"key").is_none());
            obj::native_variable_name::install_parsed_with_stage_check::<tcl_cmd_core::CmdError>(
                parent.as_ptr(),
                NativeParsedVariableName {
                    protocol,
                    array: Some((root, NativeParsedVariableElement::Object(element))),
                },
                interp.native_invocation_dialect(),
                &mut || Ok(()),
                &|_| {},
            )
            .unwrap();
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.result_obj();
            let original = BodyObjectOperation::capture(&interp).unwrap();
            let target = dynamic(parent);
            let reached = interp.body_capture_target_using(&target, None, true, Some(&original));
            assert_eq!(GETTERS.get(), 1);
            assert_eq!(REFS.get(), 1);
            assert_eq!(interp.result_obj(), prior);
            if mode == 0 {
                assert!(reached.is_ok());
                assert_eq!(ROOT_GETTERS.get(), 1);
                original.check(&interp).unwrap();
            } else {
                assert!(matches!(reached, Err(Code::Error)));
                assert_eq!(ROOT_GETTERS.get(), usize::from(mode == 5));
                assert_first(&interp, mode);
                if mode == 3 || mode == 5 {
                    assert!(!obj::allocation_is_live(ROOT.get()));
                    assert!(!obj::allocation_is_live(ELEMENT.get()));
                }
                if mode == 4 || mode == 5 {
                    assert!(matches!(interp.native_execution_refusal(),Some(NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("changed original Parsed variable primary")))));
                }
            }
            MODE.set(0);
        }
    }
}
#[test]
fn original_compiled_namespace_cache_retirement_checks_same_body_before_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Existing namespace worker and real primary free, software source only.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [0, 1, 2, 4] {
            let mut interp = fixture::native(profile);
            frame(&interp);
            let _current = fixture::install(&mut interp, 0);
            reset(mode);
            let name = obj::Owned::fresh(obj::alloc_typed(&NAMESPACE_NAME, 0));
            unsafe {
                obj::set_native_updater_string_rep(name.as_ptr(), b"::", false);
            }
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.result_obj();
            let original = BodyObjectOperation::capture(&interp).unwrap();
            let actual = interp.clone();
            let reached = interp.native_namespace_object_lookup_using(
                name.as_ptr(),
                &|| original.check(&actual),
                &|error| original.refuse(error),
            );
            assert_eq!(FREES.get(), 1);
            assert_eq!(REFS.get(), 1);
            assert_eq!(interp.result_obj(), prior);
            if mode == 0 {
                assert_eq!(reached.unwrap(), Some(GLOBAL));
                assert!(obj::native_namespace_name::cache(name.as_ptr()).is_some());
                original.check(&interp).unwrap();
            } else {
                assert!(reached.is_err());
                assert!(obj::native_namespace_name::cache(name.as_ptr()).is_none());
                assert_first(&interp, mode);
            }
            MODE.set(0);
        }
    }
}
struct WrittenRole {
    selected: Rc<crate::interp::native_variable_names::OriginalCVariableCapture>,
    pending: bool,
    calls: Cell<usize>,
    published: Cell<*mut TclObj>,
    cleanup: RefCell<Option<Box<dyn FnOnce()>>>,
    currency: crate::interp::native_operation_currency::NativeOperationCurrency,
}
impl NativeVariableObserver<Interp> for WrittenRole {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _actual: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Write);
        self.calls.set(self.calls.get() + 1);
        let incoming = obj::new_string_bytes(b"REACHED");
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
fn original_compiled_variable_store_retains_successful_write_pending_role_and_legal_replacement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Pending role is software only, not a modern C setter schedule claim.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for pending in [false, true] {
            let mut interp = fixture::native(profile);
            frame(&interp);
            let _current = fixture::install(&mut interp, 0);
            reset(0);
            let target = dynamic(fixture::string(b"variable_name"));
            let initial = fixture::string(b"OLD");
            let original = BodyObjectOperation::capture(&interp).unwrap();
            let selected = Rc::new(
                interp
                    .body_capture_target_using(&target, None, true, Some(&original))
                    .unwrap(),
            );
            selected.receiver.store(initial.as_ptr()).unwrap();
            let currency =
                crate::interp::native_operation_currency::NativeOperationCurrency::issue(&interp)
                    .unwrap();
            let observer = Rc::new(WrittenRole {
                selected: selected.clone(),
                pending,
                calls: Cell::new(0),
                published: Cell::new(core::ptr::null_mut()),
                cleanup: Default::default(),
                currency,
            });
            let name = fixture::string(b"variable_name");
            let token = interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[NativeVariableTraceOperation::Write],
                    observer.clone(),
                )
                .unwrap();
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.result_obj();
            let value = fixture::string(b"INCOMING");
            let reached = interp.body_store_value_using(&target, None, &value, &original);
            assert_eq!(observer.calls.get(), 1);
            assert_eq!(interp.result_obj(), prior);
            assert_eq!(unsafe { (*initial.as_ptr()).ref_count }, 1);
            assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
            if pending {
                assert!(matches!(reached, Err(Code::Error)));
                assert_first(&interp, 2);
                assert_eq!(
                    selected.receiver.read_initial(),
                    Err(crate::frame::VarError::NameProtocolUnavailable)
                );
                assert_eq!(unsafe { (*observer.published.get()).ref_count }, 0);
                let cleanup = observer.cleanup.borrow_mut().take().unwrap();
                cleanup();
                assert!(!obj::allocation_is_live(observer.published.get()));
            } else {
                let current = reached.unwrap().unwrap();
                assert_eq!(current.as_ptr(), observer.published.get());
                assert_eq!(unsafe { (*current.as_ptr()).ref_count }, 2);
                original.check(&interp).unwrap();
                assert_eq!(
                    original.bytes(&interp, current.as_ptr()).unwrap().as_ref(),
                    b"REACHED"
                );
            }
            interp.remove_native_variable_observer(&token);
        }
    }
}
thread_local! {
    static OLD_FREES: Cell<usize> = const { Cell::new(0) };
    static LATER_FREES: Cell<usize> = const { Cell::new(0) };
}
extern "C" fn free_original_frame(_original: *mut TclObj) {
    OLD_FREES.set(OLD_FREES.get() + 1);
    let interp = actual();
    let mut frames = interp.frames.borrow_mut();
    assert!(frames.pop().is_some());
    frames.push(GLOBAL);
}
extern "C" fn free_later_host(_original: *mut TclObj) {
    LATER_FREES.set(LATER_FREES.get() + 1);
    actual().refuse_host_command("later compiled variable retirement");
}
static OLD_FRAME: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledVariableOldFrame".as_ptr(),
    free_int_rep_proc: Some(free_original_frame),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static LATER_HOST: obj::TclObjType = obj::TclObjType {
    name: c"originalCompiledVariableLaterHost".as_ptr(),
    free_int_rep_proc: Some(free_later_host),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
struct CountWrite(Cell<usize>);
impl NativeVariableObserver<Interp> for CountWrite {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _interp: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Write);
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}
#[test]
fn original_compiled_store_checks_cell_retirement_before_write_trace_and_later_incoming_host() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real last cell owner and existing incoming role; software ordering only.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = fixture::native(profile);
        frame(&interp);
        let _current = fixture::install(&mut interp, 0);
        reset(0);
        OLD_FREES.set(0);
        LATER_FREES.set(0);
        let target = dynamic(fixture::string(b"variable_name"));
        let original = BodyObjectOperation::capture(&interp).unwrap();
        let selected = interp
            .body_capture_target_using(&target, None, true, Some(&original))
            .unwrap();
        let old = obj::Owned::fresh(obj::alloc_typed(&OLD_FRAME, 0));
        selected.receiver.store(old.as_ptr()).unwrap();
        drop(old);
        let writes = Rc::new(CountWrite(Cell::new(0)));
        let name = fixture::string(b"variable_name");
        let token = interp
            .add_native_variable_observer(
                name.as_ptr(),
                &[NativeVariableTraceOperation::Write],
                writes.clone(),
            )
            .unwrap();
        interp.set_result_bytes(b"PRIOR");
        let prior = interp.result_obj();
        let value = obj::Owned::fresh(obj::alloc_typed(&LATER_HOST, 0));
        let reached = interp.body_store_value_using(&target, None, &value, &original);
        assert!(matches!(reached, Err(Code::Error)));
        assert_eq!(OLD_FREES.get(), 1);
        assert_eq!(writes.0.get(), 0);
        assert_eq!(LATER_FREES.get(), 0);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
        let first = interp.native_execution_refusal().unwrap();
        assert!(matches!(
            &first,
            NativeExecutionError::ValueAccessRefusal(_)
        ));
        drop(value);
        assert_eq!(LATER_FREES.get(), 1);
        assert_eq!(interp.native_execution_refusal(), Some(first));
        assert_eq!(interp.result_obj(), prior);
        interp.remove_native_variable_observer(&token);
    }
}
#[test]
fn original_compiled_error_presenter_keeps_first_frame_during_mandatory_private_owner_retirement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Direct genuine private +1 setup is distinct from a public native setter.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for result_retirement in [false, true] {
            let mut interp = fixture::native(profile);
            frame(&interp);
            let _current = fixture::install(&mut interp, 0);
            reset(0);
            OLD_FREES.set(0);
            LATER_FREES.set(0);
            if result_retirement {
                let old = obj::Owned::fresh(obj::alloc_typed(&OLD_FRAME, 0));
                interp.set_result(old.as_ptr());
                drop(old);
                let mut exc = interp.exc.borrow_mut();
                exc.info = Some(b"OLD INFO".to_vec());
                exc.code = b"OLD CODE".to_vec();
            } else {
                let mut exc = interp.exc.borrow_mut();
                exc.native.info = Some(obj::Owned::fresh(obj::alloc_typed(&OLD_FRAME, 0)));
                exc.native.code = Some(obj::Owned::fresh(obj::alloc_typed(&LATER_HOST, 0)));
            }
            let original = BodyObjectOperation::capture(&interp).unwrap();
            let actual = interp.clone();
            let stage = || original.check(&actual);
            let code = if result_retirement {
                interp.report_cmd_error_using(
                    tcl_cmd_core::CmdError::new_bytes(b"REACHED".to_vec()),
                    Some(&stage),
                )
            } else {
                interp.error_using(b"REACHED", Some(&stage))
            };
            assert_eq!(code, Code::Error);
            assert_eq!(OLD_FREES.get(), 1);
            assert!(matches!(
                interp.native_execution_refusal(),
                Some(NativeExecutionError::ValueAccessRefusal(_))
            ));
            assert_eq!(obj::bytes_of(interp.result_obj()).as_slice(), b"REACHED");
            let exc = interp.exc.borrow();
            if result_retirement {
                assert_eq!(LATER_FREES.get(), 0);
                assert_eq!(exc.info.as_deref(), Some(b"OLD INFO".as_slice()));
                assert_eq!(exc.code, b"OLD CODE");
            } else {
                assert_eq!(LATER_FREES.get(), 1);
                assert!(exc.native.info.is_none());
                assert!(exc.native.code.is_none());
                assert!(exc.info.is_none());
                assert_eq!(exc.code, b"NONE");
            }
        }
    }
}
struct C84FrameWrite(Cell<usize>);
impl NativeVariableObserver<Interp> for C84FrameWrite {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _interp: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Write);
        self.0.set(self.0.get() + 1);
        let interp = actual();
        let mut frames = interp.frames.borrow_mut();
        assert!(frames.pop().is_some());
        frames.push(GLOBAL);
        Ok(())
    }
}
#[test]
fn original_compiled_error_presenter_checks_c84_frame_after_reached_setter_flag_settlement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Selected C84 mandatory flag; callback/frame invariants are software only.
    let mut interp = fixture::native("tcl8.4");
    frame(&interp);
    let _current = fixture::install(&mut interp, 0);
    reset(0);
    let observer = Rc::new(C84FrameWrite(Cell::new(0)));
    let name = fixture::string(b"::errorCode");
    let token = interp
        .add_native_variable_observer(
            name.as_ptr(),
            &[NativeVariableTraceOperation::Write],
            observer.clone(),
        )
        .unwrap();
    let original = BodyObjectOperation::capture(&interp).unwrap();
    let actual = interp.clone();
    let mut details = tcl_cmd_core::CmdError::new_bytes(b"REACHED".to_vec()).into_byte_details();
    details.error_code = tcl_cmd_core::CmdErrorCodeUpdate::Set(b"B".to_vec());
    details.error_info = Some(b"LATER INFO".to_vec());
    let reached = interp.report_cmd_error_using(
        tcl_cmd_core::CmdError::from_byte_details(details),
        Some(&|| original.check(&actual)),
    );
    assert_eq!(reached, Code::Error);
    assert_eq!(observer.0.get(), 1);
    assert!(matches!(
        interp.native_execution_refusal(),
        Some(NativeExecutionError::ValueAccessRefusal(_))
    ));
    assert!(interp.exc.borrow().native.global_code_set);
    assert!(interp.exc.borrow().info.is_none());
    assert_eq!(obj::bytes_of(interp.result_obj()).as_slice(), b"REACHED");
    interp.remove_native_variable_observer(&token);
}
