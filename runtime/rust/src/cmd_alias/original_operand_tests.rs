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
use crate::obj::Owned;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;
#[cfg(debug_assertions)]
use tcl_syntax::raw_string::NativeValueAccessRefusal;

#[derive(Clone, Copy)]
#[cfg(debug_assertions)]
pub(crate) enum Update {
    Host,
    Restore,
    Normal,
}
thread_local! {
    static INTERPRETER: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static FIRST_CAUSE: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
    #[cfg(debug_assertions)]
    static MODE: Cell<Update> = const { Cell::new(Update::Host) };
    static FIRST: Cell<usize> = const { Cell::new(0) };
    static LATER: Cell<usize> = const { Cell::new(0) };
}
#[cfg(debug_assertions)]
pub(crate) struct UpdaterOwner;
#[cfg(debug_assertions)]
impl UpdaterOwner {
    #[cfg(debug_assertions)]
    pub(crate) fn enter(interp: &Interp, mode: Update) -> Self {
        INTERPRETER.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
        MODE.with(|selected| selected.set(mode));
        FIRST.with(|count| count.set(0));
        FIRST_CAUSE.with(|cause| cause.borrow_mut().take());
        LATER.with(|count| count.set(0));
        Self
    }
}
#[cfg(debug_assertions)]
impl Drop for UpdaterOwner {
    fn drop(&mut self) {
        INTERPRETER.with(|slot| slot.borrow_mut().take());
    }
}
pub(crate) fn native(version: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
#[cfg(debug_assertions)]
fn restore(interp: &mut Interp) {
    let original = interp.runtime_context();
    let mut changed = original.clone();
    changed.packages = vec![("alias-trace-original-stage".to_owned(), "1.0".to_owned())];
    interp.pin_context(&changed).unwrap();
    interp.pin_context(&original).unwrap();
}
#[cfg(debug_assertions)]
extern "C" fn first_getter(original: *mut TclObj) {
    FIRST.with(|count| count.set(count.get() + 1));
    let mut interp = INTERPRETER.with(|slot| slot.borrow().as_ref().unwrap().clone());
    let reached = Owned::fresh(obj::new_string_bytes(b"REACHED"));
    interp
        .var_set(b"original_argument_effect", reached.as_ptr())
        .unwrap();
    match MODE.with(Cell::get) {
        Update::Host => {
            interp.refuse_host_command("first alias/trace original getter");
            FIRST_CAUSE.with(|cause| *cause.borrow_mut() = interp.native_execution_refusal());
        }
        Update::Restore => restore(&mut interp),
        Update::Normal => {}
    }
    unsafe { obj::set_native_updater_string_rep(original, b"reached\0\xff", false) };
}
#[cfg(debug_assertions)]
extern "C" fn later_getter(original: *mut TclObj) {
    LATER.with(|count| count.set(count.get() + 1));
    unsafe { obj::set_native_updater_string_rep(original, b"LATER", false) };
}
#[cfg(debug_assertions)]
static FIRST_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalAliasTraceGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(first_getter),
    set_from_any_proc: None,
};
#[cfg(debug_assertions)]
static LATER_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalAliasTraceLaterGetter".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(later_getter),
    set_from_any_proc: None,
};
#[cfg(debug_assertions)]
pub(crate) fn first() -> Owned {
    Owned::fresh(obj::alloc_typed(&FIRST_TYPE, 0))
}
#[cfg(debug_assertions)]
fn later() -> Owned {
    Owned::fresh(obj::alloc_typed(&LATER_TYPE, 0))
}
pub(crate) fn words(values: &[&[u8]]) -> Vec<Owned> {
    values
        .iter()
        .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)))
        .collect()
}
#[cfg(debug_assertions)]
fn expected(mode: Update) -> NativeExecutionError {
    match mode {
        Update::Host => FIRST_CAUSE.with(|cause| {
            cause
                .borrow()
                .as_ref()
                .expect("actual updater first cause")
                .clone()
        }),
        Update::Restore => NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("stale entered native operation"),
        ),
        Update::Normal => unreachable!(),
    }
}
/// Inspect the reached storage without starting another native name operation.
/// A pending first Host intentionally makes `var_get` terminal.
#[cfg(debug_assertions)]
pub(crate) fn reached_variable(interp: &Interp, name: &[u8]) -> *mut TclObj {
    crate::vars::get(
        &interp.frames.borrow(),
        &interp.namespaces(),
        interp.current_ns(),
        name,
    )
    .expect("actual earlier callback variable effect remains")
}
#[cfg(debug_assertions)]
pub(crate) fn seed(interp: &mut Interp) -> Owned {
    interp.set_result_bytes(b"PRIOR\0\xff");
    interp.set_return_state(2, Code::Other(7));
    interp.set_return_options(vec![(b"-original".to_vec(), b"OPTIONS".to_vec())]);
    Owned::retain(interp.get_obj_result())
}
#[cfg(debug_assertions)]
pub(crate) fn unchanged(interp: &Interp, result: &Owned, mode: Update) {
    assert_eq!(interp.native_execution_refusal(), Some(expected(mode)));
    assert_eq!(interp.get_obj_result(), result.as_ptr());
    assert_eq!(obj::bytes_of(result.as_ptr()), b"PRIOR\0\xff");
    assert_eq!(interp.pending_return_level(), 2);
    assert_eq!(interp.pending_return_code(), Code::Other(7));
    assert_eq!(interp.pending_return_option_objects().len(), 1);
    assert_eq!(FIRST.with(Cell::get), 1);
    assert_eq!(LATER.with(Cell::get), 0);
    assert_eq!(interp.entered_native_operation_count(), 0);
}

#[cfg(debug_assertions)]
#[test]
fn original_alias_reads_stop_before_later_prefix_and_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Native415 observes no prefix String read at definition. Only name and
    // target are reached here; target failure retains post-publication effects.
    // Prefix refusal is exercised separately at actual invocation below.
    for mode in [Update::Host, Update::Restore] {
        for child in [false, true] {
            for failed_position in [3, 5] {
                let mut interp = native("tcl8.6");
                if child {
                    assert_eq!(interp.eval_str(b"interp create child"), Code::Ok);
                }
                let _updater = UpdaterOwner::enter(&interp, mode);
                let failed = first();
                let unread = later();
                let owned = words(&[
                    b"interp",
                    b"alias",
                    if child { b"child" } else { b"" },
                    b"original_alias",
                    b"",
                    b"list",
                ]);
                let mut argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
                argv[failed_position] = failed.as_ptr();
                argv.push(unread.as_ptr());
                let count = unsafe { (*failed.as_ptr()).ref_count };
                let before = interp.namespaces().native_command_generations();
                let child_before = child.then(|| {
                    interp
                        .with_child(b"child", |owner| {
                            owner.namespaces().native_command_generations()
                        })
                        .unwrap()
                });
                let prior = seed(&mut interp);
                assert_eq!(super::interp_cmd(&mut interp, &argv), Code::Error);
                unchanged(&interp, &prior, mode);
                assert_eq!(unsafe { (*failed.as_ptr()).ref_count }, count);
                assert_eq!(obj::bytes_of(failed.as_ptr()), b"reached\0\xff");
                assert!(!obj::has_string_rep(unread.as_ptr()));
                let after = interp.namespaces().native_command_generations();
                if child || failed_position == 3 {
                    assert_eq!(after, before);
                } else {
                    assert_ne!(after, before, "target getter follows reached publication");
                }
                if let Some(child_before) = child_before {
                    let child_after = interp
                        .with_child(b"child", |owner| {
                            owner.namespaces().native_command_generations()
                        })
                        .unwrap();
                    if failed_position == 3 {
                        assert_eq!(child_after, child_before);
                    } else {
                        assert_ne!(child_after, child_before);
                    }
                }
                if child {
                    interp
                        .with_child(b"child", |owner| {
                            assert!(owner.resolve_cmd_token(b"original_alias").is_none())
                        })
                        .unwrap();
                } else {
                    assert!(interp.resolve_cmd_token(b"original_alias").is_none());
                }
            }
        }
    }
}

thread_local! {
    static INVOCATION_PRIOR: RefCell<Option<Owned>> = const { RefCell::new(None) };
}
#[cfg(debug_assertions)]
unsafe extern "C" fn consume_original_prefix(
    _: *mut std::ffi::c_void,
    interp: *mut Interp,
    count: std::ffi::c_int,
    argv: *const *mut TclObj,
) -> std::ffi::c_int {
    // SAFETY: actual registered command receives the live interp and argv.
    let interp = unsafe { &mut *interp };
    let argv = unsafe { std::slice::from_raw_parts(argv, usize::try_from(count).unwrap()) };
    let prior = seed(interp);
    INVOCATION_PRIOR.with(|slot| *slot.borrow_mut() = Some(prior));
    let arguments = match OriginalArguments::enter(interp, argv) {
        Ok(arguments) => arguments,
        Err(error) => return interp.report_cmd_error(error).as_int() as i32,
    };
    for index in [1, 2] {
        if let Err(error) = arguments.bytes(index) {
            return arguments.finish(interp.report_cmd_error(error)).as_int() as i32;
        }
    }
    arguments.finish(Code::Ok).as_int() as i32
}

#[cfg(debug_assertions)]
#[test]
fn original_alias_prefix_refusal_is_reached_only_at_actual_invocation() {
    // naming.alias.original-object-prefix-capture-query-invocation-lifetime
    // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
    for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [Update::Host, Update::Restore] {
            let mut interp = native(version);
            interp
                .create_obj_command(
                    b"consume_original_prefix",
                    crate::interp::ObjCommand::new(
                        consume_original_prefix,
                        std::ptr::null_mut(),
                        None,
                    ),
                )
                .expect("actual original command record");
            let _updater = UpdaterOwner::enter(&interp, mode);
            let failed = first();
            let unread = later();
            let owned = words(&[
                b"interp",
                b"alias",
                b"",
                b"original_alias",
                b"",
                b"consume_original_prefix",
            ]);
            let mut argv = owned.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            argv.extend([failed.as_ptr(), unread.as_ptr()]);
            assert_eq!(super::interp_cmd(&mut interp, &argv), Code::Ok);
            assert_eq!(FIRST.with(Cell::get), 0);
            assert_eq!(LATER.with(Cell::get), 0);
            assert!(!obj::has_string_rep(failed.as_ptr()));
            assert!(!obj::has_string_rep(unread.as_ptr()));
            let query = interp
                .alias_query_object(b"original_alias")
                .unwrap()
                .unwrap();
            assert_eq!(
                ValueOps::list_elements(&mut interp, &query.as_ptr()).unwrap(),
                vec![owned[5].as_ptr(), failed.as_ptr(), unread.as_ptr()]
            );
            assert_eq!(
                FIRST.with(Cell::get),
                0,
                "query retains the same unread prefix"
            );
            let count = unsafe { (*failed.as_ptr()).ref_count };
            let before = interp.namespaces().native_command_generations();
            assert_eq!(interp.dispatch(&[owned[3].as_ptr()]), Code::Error);
            let prior = INVOCATION_PRIOR.with(|slot| slot.borrow_mut().take()).unwrap_or_else(|| {
                    let mode = match mode {
                        Update::Host => "Host",
                        Update::Restore => "Restore",
                        Update::Normal => "Normal",
                    };
                    let result = interp.get_obj_result();
                    let resident_result = obj::has_string_rep(result).then(|| obj::bytes_of(result));
                    panic!(
                        "actual alias target callback was not reached: version={version}, mode={mode}, first={:?}, resident_result={resident_result:?}, first_getters={}, later_getters={}",
                        interp.native_execution_refusal(), FIRST.with(Cell::get), LATER.with(Cell::get),
                    );
                });
            unchanged(&interp, &prior, mode);
            assert_eq!(interp.namespaces().native_command_generations(), before);
            assert_eq!(unsafe { (*failed.as_ptr()).ref_count }, count);
            assert_eq!(obj::bytes_of(failed.as_ptr()), b"reached\0\xff");
            assert!(!obj::has_string_rep(unread.as_ptr()));
            assert!(interp.var_get(b"original_argument_effect").is_none());
            assert_eq!(
                obj::bytes_of(reached_variable(&interp, b"original_argument_effect")),
                b"REACHED"
            );
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_interpreter_options_and_names_keep_actual_read_order_and_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let cases: &[(&[&[u8]], usize)] = &[
        (&[b"interp", b"create", b"flag", b"LATER"], 2),
        (&[b"interp", b"target", b"", b"name"], 3),
        (&[b"interp", b"hide", b"", b"name", b"LATER"], 3),
        (&[b"interp", b"hide", b"", b"puts", b"token"], 4),
        (&[b"interp", b"invokehidden", b"", b"option", b"LATER"], 3),
        (
            &[
                b"interp",
                b"invokehidden",
                b"",
                b"-namespace",
                b"ns",
                b"LATER",
            ],
            4,
        ),
        (
            &[b"interp", b"invokehidden", b"", b"--", b"head", b"LATER"],
            4,
        ),
    ];
    for mode in [Update::Host, Update::Restore] {
        for &(texts, selected) in cases {
            let mut interp = native("tcl8.6");
            let _updater = UpdaterOwner::enter(&interp, mode);
            let owned = words(texts);
            let failed = first();
            let unread = later();
            let mut argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
            argv[selected] = failed.as_ptr();
            if selected + 1 < argv.len() {
                argv[selected + 1] = unread.as_ptr();
            }
            let before = interp.namespaces().native_command_generations();
            let prior = seed(&mut interp);
            assert_eq!(
                super::interp_cmd(&mut interp, &argv),
                Code::Error,
                "{texts:?}"
            );
            unchanged(&interp, &prior, mode);
            assert!(!obj::has_string_rep(unread.as_ptr()));
            assert_eq!(interp.namespaces().native_command_generations(), before);
            assert!(interp.child_names().is_empty());
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_interpreter_path_reads_selected_list_and_original_elements() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = native("tcl8.6");
    let child = Owned::fresh(obj::new_string_bytes(b"child\0ignored"));
    let path = Owned::fresh(interp.new_list_object(&[child.as_ptr()]));
    assert!(!obj::has_string_rep(path.as_ptr()));
    assert_eq!(
        super::interp_path(&mut interp, path.as_ptr()).unwrap(),
        vec![b"child".to_vec()]
    );
    assert!(!obj::has_string_rep(path.as_ptr()));
    assert_eq!(obj::bytes_of(child.as_ptr()), b"child\0ignored");
    let _updater = UpdaterOwner::enter(&interp, Update::Restore);
    let failed = first();
    let unread = later();
    let path = Owned::fresh(interp.new_list_object(&[failed.as_ptr(), unread.as_ptr()]));
    let prior = seed(&mut interp);
    assert_eq!(
        super::interp_path(&mut interp, path.as_ptr()),
        Err(Code::Error)
    );
    unchanged(&interp, &prior, Update::Restore);
    assert!(!obj::has_string_rep(path.as_ptr()));
    assert!(!obj::has_string_rep(unread.as_ptr()));
}

#[cfg(debug_assertions)]
fn original_child_worker(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    match MODE.with(Cell::get) {
        Update::Host => {
            interp.refuse_host_command("first alias/trace child worker");
        }
        Update::Restore => restore(interp),
        Update::Normal => {}
    }
    let result = Owned::fresh(obj::new_string_bytes(b"CHILD\0\xff"));
    interp.set_result(result.as_ptr());
    Code::Other(7)
}
#[cfg(debug_assertions)]
fn original_child_lazy_result(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    INTERPRETER.with(|slot| *slot.borrow_mut() = Some(interp.clone()));
    let result = first();
    interp.set_result(result.as_ptr());
    Code::Other(7)
}

#[cfg(debug_assertions)]
#[test]
fn original_hidden_child_result_getter_preserves_namespace_cleanup_and_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual child lazy result getter and mandatory cleanup, not a Native provider assertion.
    for mode in [Update::Host, Update::Restore] {
        let mut interp = native("tcl8.6");
        assert_eq!(interp.eval_str(b"interp create child"), Code::Ok);
        interp
            .with_child(b"child", |child| {
                child.register_builtin(b"original_lazy", original_child_lazy_result);
                assert!(matches!(
                    child.hide_command(b"original_lazy", b"original_lazy"),
                    crate::interp::CommandVisibilityOutcome::Moved
                ));
            })
            .unwrap();
        let _updater = UpdaterOwner::enter(&interp, mode);
        let owned = words(&[
            b"interp",
            b"invokehidden",
            b"child",
            b"-namespace",
            b"::child_work",
            b"original_lazy",
        ]);
        let argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
        let prior = seed(&mut interp);
        assert_eq!(super::interp_cmd(&mut interp, &argv), Code::Error);
        assert_eq!(interp.get_obj_result(), prior.as_ptr());
        assert_eq!(FIRST.with(Cell::get), 1);
        let cause = interp.native_execution_refusal().unwrap();
        assert_eq!(cause, expected(mode));
        interp
            .with_child(b"child", |child| {
                assert_eq!(child.native_execution_refusal(), Some(cause));
                assert_eq!(child.current_ns(), crate::namespace::GLOBAL);
                assert_eq!(obj::bytes_of(child.get_obj_result()), b"reached\0\xff");
                assert_eq!(child.entered_native_operation_count(), 0);
            })
            .unwrap();
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_child_alias_adapters_keep_guest_code_result_and_typed_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for hidden in [false, true] {
        for mode in [Update::Normal, Update::Host, Update::Restore] {
            let mut interp = native("tcl8.6");
            assert_eq!(interp.eval_str(b"interp create child"), Code::Ok);
            interp
                .with_child(b"child", |child| {
                    child.register_builtin(b"original_worker", original_child_worker);
                    if hidden {
                        child.hide_command(b"original_worker", b"original_worker");
                    }
                })
                .unwrap();
            let _updater = UpdaterOwner::enter(&interp, mode);
            let owned = words(&[
                b"interp",
                if hidden { b"invokehidden" } else { b"eval" },
                b"child",
                b"original_worker",
            ]);
            let argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
            let prior = seed(&mut interp);
            assert_eq!(
                super::interp_cmd(&mut interp, &argv),
                if matches!(mode, Update::Normal) {
                    Code::Other(7)
                } else {
                    Code::Error
                }
            );
            if matches!(mode, Update::Normal) {
                assert!(interp.native_execution_refusal().is_none());
                assert_eq!(obj::bytes_of(interp.get_obj_result()), b"CHILD\0\xff");
            } else {
                assert_eq!(interp.get_obj_result(), prior.as_ptr());
                let cause = interp.native_execution_refusal().unwrap();
                let child_cause = interp
                    .with_child(b"child", |child| child.native_execution_refusal())
                    .unwrap();
                assert_eq!(child_cause, Some(cause));
            }
            interp
                .with_child(b"child", |child| {
                    assert_eq!(child.current_ns(), crate::namespace::GLOBAL);
                    assert_eq!(child.entered_native_operation_count(), 0);
                })
                .unwrap();
            assert_eq!(interp.entered_native_operation_count(), 0);
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_child_handle_alias_reads_the_selected_operand_before_publication() {
    // naming.alias.original-object-prefix-capture-query-invocation-lifetime
    // docs/design/analysis/name-resolution-proofs/alias-original-object-prefix-capture-query-invocation-lifetime.md
    // This actual worker control distinguishes the C target pretest from
    // Jim's List-first capture. It does not equate child and core name roles.
    for version in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for mode in [Update::Host, Update::Restore] {
            let mut interp = native(version);
            let jim = version == "jim";
            let child = if jim {
                interp.create_jim_child()
            } else {
                interp.create_child(Some(b"child".to_vec()))
            };
            let _updater = UpdaterOwner::enter(&interp, mode);
            let failed = first();
            let unread = later();
            let owned = words(&[&child, b"alias", b"original_child_alias", b"list"]);
            let mut argv = owned.iter().map(Owned::as_ptr).collect::<Vec<_>>();
            argv[if jim { 2 } else { 3 }] = failed.as_ptr();
            argv.push(unread.as_ptr());
            let count = unsafe { (*failed.as_ptr()).ref_count };
            let before = interp
                .with_child(&child, |owner| {
                    owner.namespaces().native_command_generations()
                })
                .unwrap();
            let prior = seed(&mut interp);
            let code = {
                let arguments = OriginalArguments::enter(&interp, &argv).unwrap();
                let code =
                    super::child_alias_handle_original(&mut interp, &child, &argv, &arguments);
                arguments.finish(code)
            };
            assert_eq!(code, Code::Error);
            unchanged(&interp, &prior, mode);
            assert_eq!(unsafe { (*failed.as_ptr()).ref_count }, count);
            assert_eq!(obj::bytes_of(failed.as_ptr()), b"reached\0\xff");
            assert!(!obj::has_string_rep(unread.as_ptr()));
            interp
                .with_child(&child, |owner| {
                    assert_eq!(owner.namespaces().native_command_generations(), before);
                    assert!(owner.resolve_cmd_token(b"original_child_alias").is_none());
                    assert!(owner.native_execution_refusal().is_none());
                })
                .unwrap();
            assert!(interp.var_get(b"original_argument_effect").is_none());
            assert_eq!(
                obj::bytes_of(reached_variable(&interp, b"original_argument_effect")),
                b"REACHED",
            );
        }
    }
}
