// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine Runtime query callbacks and full error transport.
use super::super::native_procedure_resources::NativeProcedureDefinition;
use super::super::{default_host, Code, NativeProcedureCommand, Param, ProcLocation, WeakInterp};
use super::*;
use crate::namespace::{NsId, GLOBAL};
use crate::obj::Owned;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::native_procedure_roles::NativeProcedureRoleOwner;

thread_local! {
    static OWNER:RefCell<Option<WeakInterp>>=const { RefCell::new(None) };
    static MODE:Cell<u8>=const {Cell::new(0)};
    static HOLDER:Cell<NsId>=const {Cell::new(GLOBAL)};
    static ALIAS:Cell<bool>=const {Cell::new(false)};
    static REACHED:Cell<*mut TclObj>=const {Cell::new(std::ptr::null_mut())};
    static STAGES:RefCell<Vec<u8>>=const {RefCell::new(Vec::new())};
}
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}
fn native(profile: &str) -> Interp {
    Interp::with_native_core(
        default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}
fn text(bytes: &[u8]) -> Owned {
    Owned::fresh(obj::new_string_bytes(bytes))
}
fn typed(label: u8) -> Owned {
    Owned::fresh(obj::alloc_typed(&TYPE, u64::from(label)))
}
fn builtin(_: &mut Interp, _: &[*mut TclObj]) -> Code {
    Code::Ok
}
fn arm(interpreter: &Interp, mode: u8, holder: NsId, alias: bool) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|value| value.set(mode));
    HOLDER.with(|value| value.set(holder));
    ALIAS.with(|value| value.set(alias));
    STAGES.with(|value| value.borrow_mut().clear());
    Cleanup
}
extern "C" fn update(original: *mut TclObj) {
    let label = unsafe { (*original).internal_rep as u8 };
    STAGES.with(|value| value.borrow_mut().push(label));
    unsafe { obj::set_string_rep(original, &[label]) };
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    let holder = HOLDER.with(Cell::get);
    let name = if ALIAS.with(Cell::get) {
        b"a".as_slice()
    } else {
        b"p".as_slice()
    };
    interpreter.set_result_bytes(b"REACHED_QUERY");
    REACHED.with(|value| value.set(interpreter.result_obj()));
    if STAGES.with(|value| value.borrow().len()) != 1 {
        return;
    }
    match MODE.with(Cell::get) {
        1 => interpreter.bind_command_replacement(holder, name, Command::Builtin(builtin)),
        2 => {
            let from = interpreter.namespaces().command_fqn_at(holder, name);
            let to = interpreter.namespaces().command_fqn_at(holder, b"moved");
            assert_eq!(
                interpreter.rename_command(&from, &to),
                crate::namespace::RenameOutcome::Renamed
            );
            interpreter.bind_command_replacement(holder, name, Command::Builtin(builtin));
        }
        3 => {
            interpreter.refuse_host_command("FIRST_QUERY_HOST");
        }
        4 => {
            let actual = interpreter.runtime_context();
            let mut changed = actual.clone();
            changed
                .packages
                .push(("query-context".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&actual).unwrap();
        }
        5 => {
            interpreter.delete_namespace_by_id(holder);
            let replacement = interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"Holder");
            assert_ne!(replacement, holder);
            interpreter.bind_command_replacement(replacement, name, Command::Builtin(builtin));
        }
        _ => {}
    }
}
static TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalNameQueryStage".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn declaration(
    interpreter: &Interp,
    holder: NsId,
    body: &Owned,
    defaults: &[Owned],
) -> NativeProcedureCommand {
    NativeProcedureCommand::new(
        defaults
            .iter()
            .enumerate()
            .map(|(i, default)| Param {
                name: format!("x{i}").into_bytes(),
                default: Some(default.clone()),
            })
            .collect(),
        body.clone(),
        NativeProcedureDefinition {
            location: ProcLocation {
                namespace: holder,
                qualified_name: interpreter.namespaces().command_fqn_at(holder, b"p"),
                jim_namespace: None,
            },
            jim_parameters: None,
            source: None,
            body_line_base: 0,
            native: None,
            statics: None,
        },
    )
}
#[test]
fn original_procedure_query_retains_selected_binding_through_reached_defaults_and_body() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual callbacks and header/reference roles; no Native Proc-query chronology claim.
    for profile in ["tcl8.4", "tcl9.1"] {
        for mode in [0, 1, 2, 3, 4, 5] {
            let mut interpreter = native(profile);
            let holder = interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"Holder");
            let defaults = [typed(b'A'), typed(b'B')];
            let body = typed(b'C');
            let command = declaration(&interpreter, holder, &body, &defaults);
            let selected = command.declaration();
            interpreter.bind_command_replacement(holder, b"p", Command::Proc(command));
            let _cleanup = arm(&interpreter, mode, holder, false);
            assert_eq!(selected.native_procedure_role_ledger().references(), 1);
            assert_eq!(unsafe { (*defaults[0].as_ptr()).ref_count }, 2);
            let reached = tcl_runtime_api::Procs::proc_info_bytes(&mut interpreter, b"::Holder::p");
            if mode == 0 || mode == 2 {
                let info = reached.unwrap().unwrap();
                assert_eq!(info.params[0].default.as_deref(), Some(b"A".as_slice()));
                assert_eq!(info.params[1].default.as_deref(), Some(b"B".as_slice()));
                assert_eq!(info.body, b"C");
                assert_eq!(STAGES.with(|value| value.borrow().clone()), b"ABC");
                assert_eq!(unsafe { (*defaults[0].as_ptr()).ref_count }, 2);
                if mode == 2 {
                    assert!(tcl_runtime_api::Procs::proc_info_bytes(
                        &mut interpreter,
                        b"::Holder::p"
                    )
                    .unwrap()
                    .is_none());
                    assert_eq!(selected.native_procedure_role_ledger().references(), 1);
                }
            } else {
                let error = reached.unwrap_err();
                let first = interpreter.native_execution_refusal().unwrap();
                assert_eq!(error.native_execution_refusal(), Some(&first));
                assert_eq!(STAGES.with(|value| value.borrow().clone()), b"A");
                assert!(unsafe { (*defaults[1].as_ptr()).bytes.is_null() });
                assert!(unsafe { (*body.as_ptr()).bytes.is_null() });
                if mode == 1 || mode == 5 {
                    assert_eq!(selected.native_procedure_role_ledger().references(), 0);
                    assert_eq!(unsafe { (*defaults[0].as_ptr()).ref_count }, 1);
                }
            }
            assert_eq!(interpreter.result_obj(), REACHED.with(Cell::get));
            assert_eq!(interpreter.result_bytes(), b"REACHED_QUERY");
        }
    }
}
#[test]
fn generic_proc_body_preserves_first_host_without_missing_procedure_or_default_getters() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // The actual shared consumer preserves outer execution failure before Guest formatting.
    let mut interpreter = native("tcl9.1");
    let body = typed(b'C');
    let defaults = [typed(b'A')];
    let command = declaration(&interpreter, GLOBAL, &body, &defaults);
    interpreter.bind_command_replacement(GLOBAL, b"p", Command::Proc(command));
    let _cleanup = arm(&interpreter, 3, GLOBAL, false);
    let name = text(b"p");
    let error = tcl_cmd_core::info::body(&mut interpreter, &name.as_ptr()).unwrap_err();
    let first = interpreter.native_execution_refusal().unwrap();
    assert_eq!(error.native_execution_refusal(), Some(&first));
    assert!(
        matches!(first,tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) if first.reason=="FIRST_QUERY_HOST")
    );
    assert_eq!(STAGES.with(|value| value.borrow().clone()), b"C");
    assert!(unsafe { (*defaults[0].as_ptr()).bytes.is_null() });
    assert_eq!(interpreter.result_obj(), REACHED.with(Cell::get));
}
#[test]
fn original_alias_query_checks_actual_generation_and_namespace_after_lazy_member_callbacks() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual stored List/target/name owners; query leases acquire no native references.
    for mode in [0, 1, 3, 4, 5] {
        let mut interpreter = native("tcl9.1");
        let holder = interpreter
            .namespaces_mut()
            .ensure_namespace(GLOBAL, b"Holder");
        let target = text(b"target");
        let first = typed(b'A');
        let later = typed(b'B');
        let original = Owned::fresh(interpreter.new_list_object(&[
            target.as_ptr(),
            first.as_ptr(),
            later.as_ptr(),
        ]));
        let name = text(b"::Holder::a");
        assert!(interpreter
            .install_alias_objects(
                b"::Holder::a",
                Rc::new(crate::interp::NativeAliasHeader::new(name)),
                Rc::new(crate::interp::NativeAliasHeader::new(original.clone())),
            )
            .unwrap()
            .is_none());
        let id = interpreter.find_command_id(GLOBAL, b"::Holder::a").unwrap();
        let count = unsafe { (*first.as_ptr()).ref_count };
        assert!(unsafe { (*first.as_ptr()).bytes.is_null() });
        assert!(unsafe { (*later.as_ptr()).bytes.is_null() });
        let _cleanup = arm(&interpreter, mode, holder, true);
        let reached = interpreter.original_alias_bytes_query(id);
        if mode == 0 {
            assert_eq!(
                reached.unwrap(),
                Some(vec![b"target".to_vec(), b"A".to_vec(), b"B".to_vec()])
            );
            assert_eq!(STAGES.with(|value| value.borrow().clone()), b"AB");
            assert_eq!(unsafe { (*first.as_ptr()).ref_count }, count);
        } else {
            let error = reached.unwrap_err();
            let first_cause = interpreter.native_execution_refusal().unwrap();
            assert_eq!(error.native_execution_refusal(), Some(&first_cause));
            assert_eq!(STAGES.with(|value| value.borrow().clone()), b"A");
            assert!(unsafe { (*later.as_ptr()).bytes.is_null() });
        }
        assert_eq!(interpreter.result_obj(), REACHED.with(Cell::get));
        assert_eq!(
            unsafe { (*original.as_ptr()).ref_count },
            if mode == 1 || mode == 5 { 1 } else { 2 }
        );
    }
}

mod jim_procedure_namespace_tests {
    use super::*;
    use tcl_runtime_api::{ProcBodyBytesQuery, Procs};

    thread_local! {
        static ACTUAL: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
        static MODE: Cell<u8> = const { Cell::new(0) };
        static HOLDER: Cell<NsId> = const { Cell::new(GLOBAL) };
        static READS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
        static FIRST: RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { RefCell::new(None) };
    }
    struct Cleanup;
    impl Drop for Cleanup {
        fn drop(&mut self) {
            ACTUAL.with(|actual| actual.borrow_mut().take());
        }
    }
    fn arm(actual: &Interp, mode: u8, holder: NsId) -> Cleanup {
        ACTUAL.with(|owner| *owner.borrow_mut() = Some(actual.downgrade()));
        MODE.with(|value| value.set(mode));
        HOLDER.with(|value| value.set(holder));
        READS.with(|reads| reads.borrow_mut().clear());
        FIRST.with(|first| first.borrow_mut().take());
        Cleanup
    }
    extern "C" fn update(original: *mut TclObj) {
        let label = u8::try_from(obj::internal_rep(original)).unwrap();
        READS.with(|reads| reads.borrow_mut().push(label));
        let bytes: &[u8] = if label == b'N' {
            b"::Holder::p"
        } else {
            b"return BODY\0counted"
        };
        unsafe { obj::set_native_updater_string_rep(original, bytes, false) };
        let Some(mut actual) =
            ACTUAL.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
        else {
            return;
        };
        actual.set_result_bytes(b"REACHED ORIGINAL QUERY");
        match MODE.with(Cell::get) {
            1 => {
                actual.refuse_host_command("Jim original procedure query first cause");
                FIRST.with(|first| *first.borrow_mut() = actual.native_execution_refusal());
            }
            2 => {
                let original = actual.runtime_context();
                let mut changed = original.clone();
                changed
                    .packages
                    .push(("jim-procedure-query-control".into(), "1.0".into()));
                actual.pin_context(&changed).unwrap();
                actual.pin_context(&original).unwrap();
            }
            3 => {
                actual.rename_command(b"::Holder::p", b"");
                actual.register_builtin(b"::Holder::p", builtin);
            }
            4 => {
                let declaration = actual.proc_def(b"::Holder::p").unwrap();
                let mut location = declaration.location();
                let replacement = text(b"Holder");
                crate::native_source::bind_context(
                    replacement.as_ptr(),
                    &actual.native_jim_object_context().unwrap(),
                )
                .unwrap();
                location.jim_namespace = Some(Rc::new(replacement));
                declaration.relocate(location);
            }
            5 => actual.set_current_ns(HOLDER.with(Cell::get)),
            6 => {
                let namespace = actual.current_ns();
                actual.frames.borrow_mut().push(namespace);
            }
            _ => {}
        }
    }
    extern "C" fn retire_name(_: *mut TclObj) {
        READS.with(|reads| reads.borrow_mut().push(b'R'));
        let actual = ACTUAL
            .with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
            .unwrap();
        actual.set_current_ns(HOLDER.with(Cell::get));
    }
    static RETIRING_NAME: obj::TclObjType = obj::TclObjType {
        name: c"originalJimQueryNameRetirement".as_ptr(),
        free_int_rep_proc: Some(retire_name),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };
    extern "C" fn c_import_name_updater(original: *mut TclObj) {
        READS.with(|reads| reads.borrow_mut().push(b'C'));
        unsafe { obj::set_native_updater_string_rep(original, b"::Destination::p", false) };
    }
    static C_IMPORT_NAME: obj::TclObjType = obj::TclObjType {
        name: c"originalCInfoBodyImportName".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(c_import_name_updater),
        set_from_any_proc: None,
    };
    static ORIGINAL: obj::TclObjType = obj::TclObjType {
        name: c"originalJimProcedureQueryNamespace".as_ptr(),
        free_int_rep_proc: None,
        dup_int_rep_proc: None,
        update_string_proc: Some(update),
        set_from_any_proc: None,
    };
    fn lazy(label: u8) -> Owned {
        Owned::fresh(obj::alloc_typed(&ORIGINAL, u64::from(label)))
    }
    fn define(actual: &mut Interp, body: &Owned) -> Rc<ProcDef> {
        let command = text(b"proc");
        let name = text(b"::Holder::p");
        let parameters = Owned::fresh(actual.new_list_object(&[]));
        let context = actual.native_jim_object_context().unwrap();
        let argv = [
            command.as_ptr(),
            name.as_ptr(),
            parameters.as_ptr(),
            body.as_ptr(),
        ];
        for original in argv {
            crate::native_source::bind_context(original, &context).unwrap();
        }
        assert_eq!(actual.dispatch(&argv), Code::Ok);
        let declaration = actual.proc_def(b"::Holder::p").unwrap();
        assert_ne!(declaration.namespace(), GLOBAL);
        assert!(actual
            .native_namespace_token_at(declaration.namespace())
            .is_none());
        assert!(declaration.location().jim_namespace.is_some());
        assert_eq!(declaration.body.checked_ptr().unwrap(), body.as_ptr());
        declaration
    }

    #[test]
    fn jim_non_global_declaration_body_byte_queries_use_its_actual_holder_and_context() {
        let mut actual = native("jimtcl");
        let body = text(b"return BODY\0counted");
        let declaration = define(&mut actual, &body);
        let holder = declaration.location().jim_namespace.unwrap();
        let count = unsafe { (*holder.as_ptr()).ref_count };
        let references = declaration.native_procedure_role_ledger().references();
        assert_ne!(actual.current_ns(), declaration.namespace());
        assert_eq!(
            Procs::proc_body_bytes(&mut actual, b"::Holder::p")
                .unwrap()
                .unwrap(),
            b"return BODY\0counted"
        );
        let info = Procs::proc_info_bytes(&mut actual, b"::Holder::p")
            .unwrap()
            .unwrap();
        assert_eq!(info.body, b"return BODY\0counted");
        assert!(info.params.is_empty());
        assert_eq!(unsafe { (*holder.as_ptr()).ref_count }, count);
        assert_eq!(
            declaration.native_procedure_role_ledger().references(),
            references
        );
        assert_eq!(
            declaration.location().jim_namespace.unwrap().as_ptr(),
            holder.as_ptr()
        );
        assert!(actual.native_execution_refusal().is_none());
    }

    #[test]
    fn jim_original_body_byte_query_reuses_original_command_cache_without_a_name_getter() {
        let mut actual = native("jimtcl");
        let body = text(b"return BODY");
        let _declaration = define(&mut actual, &body);
        let name = text(b"::Holder::p");
        crate::native_source::bind_context(
            name.as_ptr(),
            &actual.native_jim_object_context().unwrap(),
        )
        .unwrap();
        for cold in [true, false] {
            if !cold {
                obj::invalidate_string(name.as_ptr());
            }
            let result = Procs::proc_body_original_bytes(&mut actual, &name.as_ptr()).unwrap();
            assert!(
                matches!(result, Some(ProcBodyBytesQuery::Body(bytes)) if bytes == b"return BODY")
            );
            assert!(obj::with_jim_command_cache(name.as_ptr(), |_| ()).is_some());
            assert_eq!(obj::has_string_rep(name.as_ptr()), cold);
        }
        // This adapter still reports bytes; the direct native result-header boundary is separate.
        let result = Owned::fresh(tcl_cmd_core::info::body(&mut actual, &name.as_ptr()).unwrap());
        assert_eq!(obj::bytes_of(result.as_ptr()), b"return BODY");
        assert!(!obj::has_string_rep(name.as_ptr()));
        assert!(actual.native_execution_refusal().is_none());
    }

    #[test]
    fn jim_original_body_query_keeps_declaration_holder_and_first_cause_inside_body_getter() {
        for mode in [0, 1, 2, 3, 4, 5, 6] {
            let mut actual = native("jimtcl");
            let body = lazy(b'B');
            let declaration = define(&mut actual, &body);
            assert!(!obj::has_string_rep(body.as_ptr()));
            let name = text(b"::Holder::p");
            let _cleanup = arm(&actual, mode, declaration.namespace());
            let reached = tcl_cmd_core::info::body(&mut actual, &name.as_ptr());
            assert_eq!(READS.with(|reads| reads.borrow().clone()), b"B");
            if mode == 0 {
                let result = Owned::fresh(reached.unwrap());
                assert_eq!(obj::bytes_of(result.as_ptr()), b"return BODY\0counted");
                assert!(actual.native_execution_refusal().is_none());
            } else {
                let error = reached.unwrap_err();
                assert_eq!(
                    error.native_execution_refusal(),
                    actual.native_execution_refusal().as_ref()
                );
                assert!(actual.native_execution_refusal().is_some());
                assert_eq!(actual.result_bytes(), b"REACHED ORIGINAL QUERY");
                if mode == 1 {
                    assert_eq!(
                        FIRST.with(|first| first.borrow().clone()),
                        actual.native_execution_refusal()
                    );
                }
                if mode == 6 {
                    actual.frames.borrow_mut().pop();
                }
            }
            assert_eq!(actual.entered_native_operation_count(), 0);
        }
    }

    #[test]
    fn jim_original_body_query_stops_after_original_name_callback_before_cache_and_body() {
        for mode in [1, 2, 5, 6] {
            let mut actual = native("jimtcl");
            let body = lazy(b'B');
            let declaration = define(&mut actual, &body);
            let name = lazy(b'N');
            crate::native_source::bind_context(
                name.as_ptr(),
                &actual.native_jim_object_context().unwrap(),
            )
            .unwrap();
            let _cleanup = arm(&actual, mode, declaration.namespace());
            let error = tcl_cmd_core::info::body(&mut actual, &name.as_ptr()).unwrap_err();
            assert_eq!(READS.with(|reads| reads.borrow().clone()), b"N");
            assert!(!obj::has_string_rep(body.as_ptr()));
            assert!(obj::with_jim_command_cache(name.as_ptr(), |_| ()).is_none());
            assert_eq!(
                error.native_execution_refusal(),
                actual.native_execution_refusal().as_ref()
            );
            assert!(actual.native_execution_refusal().is_some());
            assert_eq!(actual.result_bytes(), b"REACHED ORIGINAL QUERY");
            if mode == 1 {
                assert_eq!(
                    FIRST.with(|first| first.borrow().clone()),
                    actual.native_execution_refusal()
                );
            }
            if mode == 6 {
                actual.frames.borrow_mut().pop();
            }
            assert_eq!(actual.entered_native_operation_count(), 0);
        }
    }

    #[test]
    fn jim_original_body_query_checks_name_primary_retirement_before_cache_publication() {
        let mut actual = native("jimtcl");
        let body = lazy(b'B');
        let declaration = define(&mut actual, &body);
        let name = Owned::fresh(obj::alloc_typed(&RETIRING_NAME, 0));
        unsafe { obj::set_native_updater_string_rep(name.as_ptr(), b"::Holder::p", false) };
        let _cleanup = arm(&actual, 0, declaration.namespace());
        let error = tcl_cmd_core::info::body(&mut actual, &name.as_ptr()).unwrap_err();
        assert_eq!(READS.with(|reads| reads.borrow().clone()), b"R");
        assert!(!obj::has_string_rep(body.as_ptr()));
        assert!(obj::with_jim_command_cache(name.as_ptr(), |_| ()).is_none());
        assert_eq!(
            error.native_execution_refusal(),
            actual.native_execution_refusal().as_ref()
        );
        assert!(actual.native_execution_refusal().is_some());
        assert_eq!(actual.entered_native_operation_count(), 0);
    }

    #[test]
    fn jim_original_body_query_does_not_follow_alias_invocations_or_alias_cycles() {
        let mut actual = native("jimtcl");
        let body = text(b"return BODY");
        let _declaration = define(&mut actual, &body);
        assert_eq!(
            actual.eval_str(
                b"alias targetAlias ::Holder::p; alias cycleA cycleB; alias cycleB cycleA"
            ),
            Code::Ok
        );
        for original in [b"targetAlias".as_slice(), b"cycleA", b"cycleB"] {
            let name = text(original);
            let result = Procs::proc_body_original_bytes(&mut actual, &name.as_ptr()).unwrap();
            assert!(
                matches!(result, Some(ProcBodyBytesQuery::NotProcedure(bytes)) if bytes == original)
            );
        }
        assert!(actual.native_execution_refusal().is_none());
    }

    #[test]
    fn original_body_query_preserves_finite_c_import_generation_ownership() {
        for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut actual = native(profile);
            assert_eq!(actual.eval_str(b"namespace eval Source {namespace export *; proc p {} {return BODY}}; namespace eval Middle {namespace export *; namespace import ::Source::p}; namespace eval Destination {namespace import ::Middle::p}"), Code::Ok);
            READS.with(|reads| reads.borrow_mut().clear());
            let name = Owned::fresh(obj::alloc_typed(&C_IMPORT_NAME, 0));
            assert!(Procs::proc_body_original_bytes(&mut actual, &name.as_ptr())
                .unwrap()
                .is_none());
            assert!(READS.with(|reads| reads.borrow().is_empty()));
            for first in [true, false] {
                if !first {
                    obj::invalidate_string(name.as_ptr());
                }
                let result =
                    Owned::fresh(tcl_cmd_core::info::body(&mut actual, &name.as_ptr()).unwrap());
                assert_eq!(obj::bytes_of(result.as_ptr()), b"return BODY");
                assert!(obj::has_string_rep(name.as_ptr()));
                assert!(obj::native_command_name_cache(name.as_ptr()).is_none());
            }
            assert_eq!(READS.with(|reads| reads.borrow().clone()), b"CC");
            assert!(actual.native_execution_refusal().is_none());
        }
    }
}
