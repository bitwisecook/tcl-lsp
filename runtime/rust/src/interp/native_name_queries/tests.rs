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
