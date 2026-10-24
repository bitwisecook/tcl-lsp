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

//! Genuine Dictionary scope publication and mandatory incoming retirement.

use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::native_variable_trace::{
    NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
};
use tcl_runtime_api::NativeExecutionError;

thread_local! {
    static WITHDRAW: Cell<bool> = const { Cell::new(false) };
    static STORE: Cell<bool> = const { Cell::new(false) };
    static SELECTED: RefCell<Option<Rc<crate::interp::native_variable_names::OriginalCVariableCapture>>> = const { RefCell::new(None) };
    static INCOMING: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static REPLACEMENT: Cell<*mut TclObj> = const { Cell::new(core::ptr::null_mut()) };
    static OLD_FREES: Cell<usize> = const { Cell::new(0) };
    static INCOMING_FREES: Cell<usize> = const { Cell::new(0) };
    static OLD_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static PUBLISHED_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static INCOMING_REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
    static CAUSE_AT_INCOMING_FREE: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
}

extern "C" fn retire_old(original: *mut TclObj) {
    OLD_FREES.set(OLD_FREES.get() + 1);
    OLD_REFS.set(unsafe { (*original).ref_count });
    PUBLISHED_REFS.set(unsafe { (*INCOMING.get()).ref_count });
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    if STORE.get() {
        let selected = SELECTED.with(|owner| owner.borrow().as_ref().unwrap().clone());
        assert_eq!(
            selected.receiver.read_initial().unwrap(),
            Some(INCOMING.get())
        );
        let replacement = obj::new_string_bytes(b"REACHED");
        REPLACEMENT.set(replacement);
        selected.receiver.store(replacement).unwrap();
    } else {
        assert_eq!(actual.result_obj(), INCOMING.get());
        actual.set_result_bytes(b"REACHED");
        REPLACEMENT.set(actual.result_obj());
    }
    // The incoming working owner, acquired by the real original algorithm,
    // remains the sole native reference after reentrant replacement.
    assert_eq!(unsafe { (*INCOMING.get()).ref_count }, 1);
    if WITHDRAW.get() {
        let mut frames = actual.frames.borrow_mut();
        assert!(frames.pop().is_some());
        frames.push(crate::namespace::GLOBAL);
    }
}

extern "C" fn retire_incoming(original: *mut TclObj) {
    INCOMING_FREES.set(INCOMING_FREES.get() + 1);
    INCOMING_REFS.set(unsafe { (*original).ref_count });
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    CAUSE_AT_INCOMING_FREE.with(|first| *first.borrow_mut() = actual.native_execution_refusal());
    if WITHDRAW.get() {
        actual.refuse_host_command("later Dictionary working owner cleanup");
    }
}

static OLD: obj::TclObjType = obj::TclObjType {
    name: c"originalDictionaryScopeRetiredPublication".as_ptr(),
    free_int_rep_proc: Some(retire_old),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static WORKING: obj::TclObjType = obj::TclObjType {
    name: c"originalDictionaryScopeWorkingCleanup".as_ptr(),
    free_int_rep_proc: Some(retire_incoming),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

struct WriteCount(Cell<usize>);
impl NativeVariableObserver<Interp> for WriteCount {
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _actual: &mut Interp,
        access: NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        assert_eq!(access.operation, NativeVariableTraceOperation::Write);
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        WITHDRAW.set(false);
        STORE.set(false);
        SELECTED.with(|selected| selected.borrow_mut().take());
        INCOMING.set(core::ptr::null_mut());
        REPLACEMENT.set(core::ptr::null_mut());
        CAUSE_AT_INCOMING_FREE.with(|first| first.borrow_mut().take());
    }
}
fn frame_first(first: &NativeExecutionError) {
    assert!(matches!(
        first,
        NativeExecutionError::ValueAccessRefusal(
            tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                "original compiled List frame or temporary"
            )
        )
    ));
}

#[test]
fn original_compiled_dictionary_publication_retains_frame_before_working_cleanup_and_keeps_replacement(
) {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual original cell/result and working reference roles are exercised.
    // Callback ordering here is software transport, not external Native parity.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for store in [false, true] {
            for withdraw in [false, true] {
                let mut actual = fixture::native(profile);
                assert_eq!(
                    actual.eval_str(
                        b"proc p {} {set d {K A}; dict update d K v {set v B}; return $d}"
                    ),
                    Code::Ok
                );
                assert_eq!(actual.eval_str(b"p"), Code::Ok);
                let procedure = actual.proc_def(b"p").unwrap();
                let artifact = cache(procedure.body.as_ptr()).unwrap();
                let instruction = artifact
                    .scripts
                    .values()
                    .flat_map(|script| &script.commands)
                    .find_map(|command| match &command.operation {
                        Operation::DictionaryScope(scope) => Some(scope),
                        _ => None,
                    })
                    .expect("actual compiled Dictionary Update owner");
                assert!(matches!(instruction.recipe.kind, Kind::Update { .. }));
                let protocol = actual
                    .native_invocation_dialect()
                    .native_compiled_variable_protocol()
                    .filter(|protocol| protocol.has_indexed_locals())
                    .expect("actual native indexed-local protocol");
                {
                    let mut frames = actual.frames.borrow_mut();
                    frames.push(crate::namespace::GLOBAL);
                    frames.install_formal_cells(&[], protocol);
                    frames
                        .install_native_compiled_local_layout(
                            artifact.compiled_local_layout().unwrap(),
                        )
                        .unwrap();
                }
                let _current = fixture::install(&mut actual, 0);
                let _reset = Reset;
                let target = local_target(instruction.root_name.as_deref().unwrap());
                let old = obj::Owned::fresh(obj::alloc_typed(&OLD, 0));
                if store {
                    actual
                        .body_store_value(&target, instruction.root_slot, &old)
                        .unwrap();
                } else {
                    actual.set_result(old.as_ptr());
                }
                let old_pointer = old.as_ptr();
                drop(old);
                assert_eq!(unsafe { (*old_pointer).ref_count }, 1);
                let selected = store.then(|| {
                    Rc::new(
                        actual
                            .body_capture_target(&target, instruction.root_slot, false)
                            .unwrap(),
                    )
                });
                SELECTED.with(|owner| *owner.borrow_mut() = selected.clone());
                let writes = Rc::new(WriteCount(Cell::new(0)));
                let name = fixture::string(&target.root);
                let token = if store {
                    Some(
                        actual
                            .add_native_variable_observer(
                                name.as_ptr(),
                                &[NativeVariableTraceOperation::Write],
                                writes.clone(),
                            )
                            .unwrap(),
                    )
                } else {
                    None
                };
                if store {
                    actual.set_result_bytes(b"PRIOR");
                }
                let prior_result = actual.result_obj();
                let original = BodyObjectOperation::capture(&actual).unwrap();
                let incoming = obj::alloc_typed(&WORKING, 0);
                INCOMING.set(incoming);
                OLD_FREES.set(0);
                INCOMING_FREES.set(0);
                OLD_REFS.set(-1);
                PUBLISHED_REFS.set(-1);
                INCOMING_REFS.set(-1);
                STORE.set(store);
                WITHDRAW.set(withdraw);
                let reached = if store {
                    actual
                        .publish_compiled_dictionary(
                            &original,
                            &target,
                            selected.as_ref().unwrap(),
                            RuntimeAppendValue::retain(incoming),
                        )
                        .map(|()| Code::Ok)
                } else {
                    actual
                        .publish_compiled_dictionary_result(&original, obj::Owned::fresh(incoming))
                };
                assert_eq!(OLD_FREES.get(), 1);
                assert_eq!(INCOMING_FREES.get(), 1);
                assert_eq!(OLD_REFS.get(), 0);
                assert_eq!(PUBLISHED_REFS.get(), 2);
                assert_eq!(INCOMING_REFS.get(), 0);
                assert!(!obj::allocation_is_live(old_pointer));
                assert!(!obj::allocation_is_live(incoming));
                if store {
                    assert_eq!(actual.result_obj(), prior_result);
                } else {
                    assert_eq!(actual.result_obj(), REPLACEMENT.get());
                }
                if withdraw {
                    assert_eq!(reached, Err(Code::Error));
                    let before_cleanup = CAUSE_AT_INCOMING_FREE
                        .with(|first| first.borrow().clone())
                        .expect("frame cause before actual incoming retirement");
                    frame_first(&before_cleanup);
                    let first = actual.native_execution_refusal().unwrap();
                    assert_eq!(first, before_cleanup);
                    assert!(original.check(&actual).is_err());
                    assert_eq!(writes.0.get(), 0);
                } else {
                    assert_eq!(reached, Ok(Code::Ok));
                    assert!(CAUSE_AT_INCOMING_FREE.with(|first| first.borrow().is_none()));
                    assert!(actual.native_execution_refusal().is_none());
                    original.check(&actual).unwrap();
                    assert_eq!(writes.0.get(), if store { 1 } else { 0 });
                    let replacement = if store {
                        selected
                            .as_ref()
                            .unwrap()
                            .receiver
                            .read_initial()
                            .unwrap()
                            .unwrap()
                    } else {
                        actual.result_obj()
                    };
                    assert_eq!(replacement, REPLACEMENT.get());
                    assert_eq!(unsafe { (*replacement).ref_count }, 1);
                    assert_eq!(
                        original.bytes(&actual, replacement).unwrap().as_ref(),
                        b"REACHED"
                    );
                }
                if let Some(token) = token {
                    actual.remove_native_variable_observer(&token);
                }
            }
        }
    }
}
