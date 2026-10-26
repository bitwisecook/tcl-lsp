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

//! Actual compiled coroutine List concat and terminal result retirement.
use super::*;
use crate::cmd_control::original_list_conversion_tests as fixture;
use std::cell::Cell;
thread_local! {
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static REFS: Cell<obj::TclSize> = const { Cell::new(-1) };
}
extern "C" fn retire(original: *mut TclObj) {
    FREES.set(FREES.get() + 1);
    REFS.set(unsafe { (*original).ref_count });
    let mut actual = unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone();
    match MODE.get() {
        1 => {
            actual.refuse_host_command("original coroutine reached retirement");
        }
        2 => {
            let before = actual.runtime_context();
            let mut changed = before.clone();
            changed.packages = vec![("original-coroutine-list".into(), "1".into())];
            actual.pin_context(&changed).unwrap();
            actual.pin_context(&before).unwrap();
        }
        3 => {
            let mut frames = actual.frames.borrow_mut();
            assert!(frames.pop().is_some());
            frames.push(crate::namespace::GLOBAL);
        }
        _ => {}
    }
}
static ORIGINAL: obj::TclObjType = obj::TclObjType {
    name: c"originalCoroutineListRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        MODE.set(0);
    }
}
fn assert_first(actual: &Interp, mode: u8) {
    let first = actual
        .native_execution_refusal()
        .expect("actual original coroutine cause");
    if mode == 1 {
        assert!(
            matches!(first,tcl_runtime_api::NativeExecutionError::HostCommandRefusal(ref reason) if reason.reason=="original coroutine reached retirement")
        );
    } else {
        assert!(matches!(
            first,
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(_)
        ));
    }
}
#[test]
fn original_coroutine_concat_keeps_frame_inside_source_conversion_before_tailcall_publication() {
    // The real C91 expanded-tailcall recipe; software callbacks, no Native chronology claim.
    for mode in [0, 1, 2, 3] {
        let mut actual = fixture::native("tcl9.1");
        assert_eq!(
            actual.eval_str(b"proc target args {return DONE}; proc p {} {tailcall target {*}$x}"),
            Code::Ok
        );
        assert_eq!(actual.eval_str(b"p"), Code::Error);
        let artifact = cache(actual.proc_def(b"p").unwrap().body.as_ptr())
            .expect("actual expanded tailcall artifact");
        let command = &artifact.scripts[&artifact.region].commands[0];
        let Operation::Coroutine(recipe) = &command.operation else {
            panic!("real native coroutine instruction")
        };
        assert!(recipe
            .steps
            .iter()
            .any(|step| matches!(step, CoroutineStep::Concat)));
        assert!(matches!(
            recipe.steps.last(),
            Some(CoroutineStep::TailcallList)
        ));
        {
            let mut frames = actual.frames.borrow_mut();
            frames.push(crate::namespace::GLOBAL);
            frames
                .install_native_compiled_local_layout(artifact.compiled_local_layout().unwrap())
                .unwrap();
        }
        let _installed = fixture::install(&mut actual, 0);
        let _reset = Reset;
        let source = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        unsafe {
            obj::set_native_updater_string_rep(source.as_ptr(), b"B", false);
        }
        let slot = command
            .words
            .iter()
            .find_map(|word| {
                word.arena.locals.iter().find_map(|(span, slot)| {
                    (word.arena.original.bytes(*span) == Some(b"x".as_slice())).then_some(*slot)
                })
            })
            .expect("actual expanded argument local");
        let target = EvaluatedTarget {
            root: b"x".to_vec(),
            element: None,
            original_name: None,
            original_index: None,
            combined: false,
        };
        actual
            .body_store_value(&target, Some(slot), &source)
            .unwrap();
        let prior = fixture::string(b"PRIOR");
        actual.set_result(prior.as_ptr());
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
        MODE.set(mode);
        FREES.set(0);
        REFS.set(-1);
        let reached = actual.execute_body_coroutine(
            &artifact,
            command,
            recipe,
            &mut BodyExecution::default(),
        );
        assert_eq!(FREES.get(), 1);
        assert_eq!(REFS.get(), 3);
        let pending = {
            let mut frames = actual.frames.borrow_mut();
            frames.take_tailcall()
        };
        if mode == 0 {
            assert_eq!(reached, Ok(Code::Return));
            assert!(pending.is_some());
            scope.currency().ensure_current_or_refuse().unwrap();
        } else {
            assert_eq!(reached, Err(Code::Error));
            assert!(pending.is_none());
            assert_first(&actual, mode);
            assert!(scope.currency().ensure_current_or_refuse().is_err());
        }
        assert_eq!(actual.result_obj(), prior.as_ptr());
        MODE.set(0);
        drop(pending);
        actual.frames.borrow_mut().pop();
    }
}
#[test]
fn original_coroutine_name_checks_reached_last_result_retirement_before_success() {
    for mode in [0, 1, 2, 3] {
        let mut actual = fixture::native("tcl9.1");
        assert_eq!(actual.eval_str(b"proc p {} {info coroutine}"), Code::Ok);
        assert_eq!(actual.eval_str(b"p"), Code::Ok);
        let artifact = cache(actual.proc_def(b"p").unwrap().body.as_ptr())
            .expect("actual info coroutine artifact");
        let command = &artifact.scripts[&artifact.region].commands[0];
        let Operation::Coroutine(recipe) = &command.operation else {
            panic!("actual coroutine Name instruction")
        };
        assert!(matches!(recipe.steps.as_slice(), [CoroutineStep::Name]));
        actual.frames.borrow_mut().push(crate::namespace::GLOBAL);
        let _installed = fixture::install(&mut actual, 0);
        let _reset = Reset;
        let prior = obj::Owned::fresh(obj::alloc_typed(&ORIGINAL, 0));
        actual.set_result(prior.as_ptr());
        let _allocation = obj::NativeObjectLifetime::retain(prior.as_ptr());
        drop(prior);
        let scope =
            crate::interp::native_operation_currency::NativeOperationScope::enter(&actual).unwrap();
        MODE.set(mode);
        FREES.set(0);
        REFS.set(-1);
        let reached = actual.execute_body_coroutine(
            &artifact,
            command,
            recipe,
            &mut BodyExecution::default(),
        );
        assert_eq!(FREES.get(), 1);
        assert_eq!(REFS.get(), 0);
        assert!(obj::bytes_of(actual.result_obj()).is_empty());
        if mode == 0 {
            assert_eq!(reached, Ok(Code::Ok));
            scope.currency().ensure_current_or_refuse().unwrap();
        } else {
            assert_eq!(reached, Err(Code::Error));
            assert_first(&actual, mode);
            assert!(scope.currency().ensure_current_or_refuse().is_err());
        }
        MODE.set(0);
        actual.frames.borrow_mut().pop();
    }
}
