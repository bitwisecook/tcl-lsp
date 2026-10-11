// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual command/Proc owners and original operation settlement. Software controls.

use super::{native_procedure_resources::NativeProcedureDefinition, *};
use crate::namespace::{NsId, GLOBAL};
use crate::obj::{self, Owned};
use tcl_runtime_api::native_procedure_roles::NativeProcedureRoleOwner;

thread_local! {
    static OWNER: RefCell<Option<WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static HOLDER: Cell<NsId> = const { Cell::new(GLOBAL) };
    static REBORN: Cell<NsId> = const { Cell::new(GLOBAL) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static COUNTS: RefCell<Vec<obj::TclSize>> = const { RefCell::new(Vec::new()) };
    static EMPTY: Cell<bool> = const { Cell::new(false) };
    static REACHED: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
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
fn body() -> Owned {
    let body = Owned::fresh(obj::alloc_typed(&BODY, 0));
    unsafe { obj::set_string_rep(body.as_ptr(), b"return OLD") };
    body
}
fn builtin(_: &mut Interp, _: &[*mut TclObj]) -> Code {
    Code::Ok
}
fn arm(interpreter: &Interp, mode: u8, holder: NsId) -> Cleanup {
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|value| value.set(mode));
    HOLDER.with(|value| value.set(holder));
    REBORN.with(|value| value.set(holder));
    FREES.with(|value| value.set(0));
    EMPTY.with(|value| value.set(false));
    COUNTS.with(|value| value.borrow_mut().clear());
    Cleanup
}
extern "C" fn retire(original: *mut TclObj) {
    FREES.with(|value| value.set(value.get() + 1));
    COUNTS.with(|value| value.borrow_mut().push(unsafe { (*original).ref_count }));
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    assert!(interpreter.frames.try_borrow_mut().is_ok());
    let holder = HOLDER.with(Cell::get);
    EMPTY.with(|value| {
        value.set(
            interpreter
                .namespaces()
                .command_in(holder, b"retiring")
                .is_none(),
        )
    });
    // This actual callback owns the reached result. A later refused publication
    // must not replace it with a Guest diagnostic or an empty success.
    interpreter.set_result_bytes(b"REACHED");
    REACHED.with(|value| value.set(interpreter.result_ptr()));
    match MODE.with(Cell::get) {
        1 => {
            interpreter.refuse_host_command("original command payload retirement");
        }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("command-retirement".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => {
            interpreter.bind_command_replacement(holder, b"retiring", Command::Builtin(builtin));
        }
        4 => {
            interpreter.delete_namespace_by_id(holder);
            let reborn = interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"Holder");
            REBORN.with(|value| value.set(reborn));
        }
        _ => {}
    }
}
static BODY: obj::TclObjType = obj::TclObjType {
    name: c"originalCommandPayload".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
fn declaration(interpreter: &Interp, holder: NsId, body: Owned) -> NativeProcedureCommand {
    NativeProcedureCommand::new(
        Vec::new(),
        body,
        NativeProcedureDefinition {
            location: ProcLocation {
                namespace: holder,
                qualified_name: interpreter.namespaces().command_fqn_at(holder, b"retiring"),
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
fn check_result(interpreter: &Interp) {
    assert_eq!(interpreter.result_ptr(), REACHED.with(Cell::get));
}

#[test]
fn original_proc_binding_retires_resources_outside_table_borrow_while_query_views_survive() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Query metadata is not a native Proc reference. C resource-role software control only.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..3 {
            crate::counters::reset();
            {
                let mut interpreter = native(profile);
                let command = declaration(&interpreter, GLOBAL, body());
                let query = command.clone();
                let owner = query.declaration();
                assert_eq!(owner.native_procedure_role_ledger().references(), 1);
                interpreter.bind_command_replacement(GLOBAL, b"retiring", Command::Proc(command));
                let generation = interpreter
                    .namespaces()
                    .command_generation(GLOBAL, b"retiring")
                    .unwrap();
                let _cleanup = arm(&interpreter, mode, GLOBAL);
                assert_eq!(interpreter.delete_command_generation(generation), mode == 0);
                assert_eq!(FREES.with(Cell::get), 1);
                assert_eq!(COUNTS.with(|value| value.borrow().clone()), [0]);
                assert!(EMPTY.with(Cell::get));
                assert!(owner.native_procedure_role_ledger().is_retired());
                assert_eq!(owner.native_procedure_role_ledger().references(), 0);
                assert!(query.declaration().body.checked_ptr().is_err());
                assert_eq!(interpreter.native_execution_refusal().is_some(), mode != 0);
                check_result(&interpreter);
                drop(query);
                drop(owner);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}

#[test]
fn original_proc_replacement_checks_retirement_and_holder_before_incoming_adoption() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Callback recreation has no non-Obj payload disposal purpose; no second delete or Native grant.
    for profile in ["tcl8.4", "tcl8.6", "tcl9.1"] {
        for mode in 0..5 {
            crate::counters::reset();
            {
                let mut interpreter = native(profile);
                let holder = interpreter
                    .namespaces_mut()
                    .ensure_namespace(GLOBAL, b"Holder");
                let command = declaration(&interpreter, holder, body());
                interpreter.bind_command_replacement(holder, b"retiring", Command::Proc(command));
                let _cleanup = arm(&interpreter, mode, holder);
                interpreter.bind_command_replacement(
                    holder,
                    b"retiring",
                    Command::Builtin(builtin),
                );
                assert_eq!(FREES.with(Cell::get), 1);
                assert!(EMPTY.with(Cell::get));
                if mode == 0 {
                    assert!(matches!(
                        interpreter.namespaces().command_in(holder, b"retiring"),
                        Some(Command::Builtin(_))
                    ));
                    assert!(interpreter.native_execution_refusal().is_none());
                } else {
                    assert!(interpreter.native_execution_refusal().is_some());
                    assert_eq!(
                        interpreter
                            .namespaces()
                            .command_in(holder, b"retiring")
                            .is_some(),
                        mode == 3
                    );
                    if mode == 3 {
                        assert!(matches!(interpreter.native_execution_refusal(), Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("callback-recreated command payload disposal purpose unavailable")))));
                    }
                    if mode == 4 {
                        assert_ne!(REBORN.with(Cell::get), holder);
                    }
                }
                check_result(&interpreter);
            }
            assert_eq!(crate::counters::finalize(), 0);
            assert_eq!(crate::counters::double_free_count(), 0);
        }
    }
}

#[test]
fn original_jim_registration_temporary_retires_before_outer_publication_settlement() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Occupied Jim key remains independent; the real incoming temporary has one native hold.
    for mode in 0..3 {
        crate::counters::reset();
        {
            let mut interpreter = native("jim");
            interpreter.bind_command_replacement(GLOBAL, b"retiring", Command::Builtin(builtin));
            let original_key = interpreter
                .namespaces()
                .original_jim_command_key(GLOBAL, b"retiring")
                .unwrap();
            let key = body();
            unsafe { obj::set_string_rep(key.as_ptr(), b"retiring") };
            let report = interpreter.namespaces().jim_command_table_key_at(
                GLOBAL,
                b"retiring",
                tcl_syntax::naming::NativeNamePurpose::CommandCApiPublication,
            );
            let _cleanup = arm(&interpreter, mode, GLOBAL);
            interpreter.bind_command_replacement_with_original_jim_key(
                GLOBAL,
                b"retiring",
                Command::Builtin(builtin),
                report,
                Some(Rc::new(key)),
            );
            assert_eq!(FREES.with(Cell::get), 1);
            assert_eq!(COUNTS.with(|value| value.borrow().clone()), [0]);
            assert!(
                !EMPTY.with(Cell::get),
                "new publication precedes actual unused incoming temporary retirement"
            );
            assert_eq!(
                interpreter
                    .namespaces()
                    .original_jim_command_key(GLOBAL, b"retiring")
                    .unwrap(),
                original_key
            );
            assert_eq!(interpreter.native_execution_refusal().is_some(), mode != 0);
            check_result(&interpreter);
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}

#[test]
fn original_ensemble_replacement_uses_shared_retirement_before_token_activation() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real Proc release from the ordinary shared replacement worker, not Native ensemble chronology.
    for mode in 0..3 {
        crate::counters::reset();
        {
            let mut interpreter = native("tcl9.1");
            let command = declaration(&interpreter, GLOBAL, body());
            interpreter.bind_command_replacement(GLOBAL, b"retiring", Command::Proc(command));
            let _cleanup = arm(&interpreter, mode, GLOBAL);
            interpreter.create_ensemble_in_slot(
                GLOBAL,
                b"retiring",
                crate::ensemble::EnsembleConfig {
                    originals: Default::default(),
                    ns: GLOBAL,
                    map: None,
                    subcommands: None,
                    prefixes: true,
                    parameters: Vec::new(),
                    unknown: Vec::new(),
                },
            );
            assert_eq!(FREES.with(Cell::get), 1);
            assert!(EMPTY.with(Cell::get));
            assert_eq!(
                matches!(
                    interpreter.namespaces().command_in(GLOBAL, b"retiring"),
                    Some(Command::Ensemble(_))
                ),
                mode == 0
            );
            assert_eq!(interpreter.native_execution_refusal().is_some(), mode != 0);
            check_result(&interpreter);
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}

#[test]
fn original_namespace_drain_finishes_each_existing_proc_role_after_first_refusal() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Already extracted owners require mandatory release after Host; no later dispatch is admitted.
    for profile in ["tcl8.4", "tcl8.6", "tcl9.1"] {
        crate::counters::reset();
        {
            let mut interpreter = native(profile);
            let holder = interpreter
                .namespaces_mut()
                .ensure_namespace(GLOBAL, b"Holder");
            let a = declaration(&interpreter, holder, body());
            let b = declaration(&interpreter, holder, body());
            let old_a = a.declaration();
            let old_b = b.declaration();
            interpreter.bind_command_replacement(holder, b"a", Command::Proc(a));
            interpreter.bind_command_replacement(holder, b"b", Command::Proc(b));
            let _cleanup = arm(&interpreter, 1, holder);
            interpreter.delete_namespace_by_id(holder);
            assert_eq!(FREES.with(Cell::get), 2);
            assert_eq!(COUNTS.with(|value| value.borrow().clone()), [0, 0]);
            assert!(old_a.native_procedure_role_ledger().is_retired());
            assert!(old_b.native_procedure_role_ledger().is_retired());
            assert!(interpreter.native_execution_refusal().is_some());
            assert!(interpreter.namespaces().command_names(holder).is_empty());
            check_result(&interpreter);
        }
        assert_eq!(crate::counters::finalize(), 0);
        assert_eq!(crate::counters::double_free_count(), 0);
    }
}

thread_local! {
    static HEADER_MODE: Cell<u8> = const { Cell::new(0) };
    static HEADER_STAGE: Cell<u8> = const { Cell::new(0) };
    static HEADER_GETTERS: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static HEADER_ORIGINAL: RefCell<Option<Weak<ProcDef>>> = const { RefCell::new(None) };
    static HEADER_GENERATION: Cell<u64> = const { Cell::new(0) };
}
extern "C" fn header_formals(original: *mut TclObj) {
    HEADER_GETTERS.with(|value| value.borrow_mut().push(b'A'));
    unsafe { obj::set_string_rep(original, b"args") };
    if HEADER_STAGE.with(Cell::get) == 0 {
        observe_header_binding();
    }
}
fn observe_header_binding() {
    let Some(mut interpreter) =
        OWNER.with(|owner| owner.borrow().as_ref().and_then(WeakInterp::upgrade))
    else {
        return;
    };
    assert!(interpreter.namespaces.try_borrow_mut().is_ok());
    let command = match interpreter.namespaces().command_in(GLOBAL, b"headerProbe") {
        Some(Command::Proc(command)) => command,
        _ => panic!("actual published original Proc must precede formal String"),
    };
    let declaration = command.declaration();
    assert_eq!(declaration.native_procedure_role_ledger().references(), 1);
    HEADER_ORIGINAL.with(|value| *value.borrow_mut() = Some(Rc::downgrade(&declaration)));
    HEADER_GENERATION.with(|value| {
        value.set(
            interpreter
                .namespaces()
                .command_generation(GLOBAL, b"headerProbe")
                .unwrap(),
        )
    });
    drop(command);
    interpreter.set_result_bytes(b"HEADER_REACHED");
    REACHED.with(|value| value.set(interpreter.result_ptr()));
    match HEADER_MODE.with(Cell::get) {
        1 | 5 => {
            interpreter.bind_command_replacement(GLOBAL, b"headerProbe", Command::Builtin(builtin))
        }
        2 => assert_eq!(
            interpreter.rename_command(b"headerProbe", b"headerMoved"),
            crate::namespace::RenameOutcome::Renamed
        ),
        3 => {
            interpreter.refuse_host_command("original compiler header observer");
        }
        4 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed
                .packages
                .push(("header-context".into(), "1.0".into()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        6 => {
            let selected = match interpreter
                .namespaces()
                .command_in(GLOBAL, b"headerProbe")
                .unwrap()
            {
                Command::Proc(command) => command,
                _ => unreachable!(),
            };
            let replacement = declaration_for_header(&interpreter);
            selected.replace_declaration(&replacement);
        }
        _ => {}
    }
    if HEADER_MODE.with(Cell::get) == 5 {
        interpreter.refuse_host_command("original compiler header observer");
    }
}
fn declaration_for_header(interpreter: &Interp) -> NativeProcedureCommand {
    NativeProcedureCommand::new(
        Vec::new(),
        text(b"return replacement"),
        NativeProcedureDefinition {
            location: ProcLocation {
                namespace: GLOBAL,
                qualified_name: interpreter
                    .namespaces()
                    .command_fqn_at(GLOBAL, b"headerProbe"),
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
extern "C" fn header_body(original: *mut TclObj) {
    HEADER_GETTERS.with(|value| value.borrow_mut().push(b'B'));
    unsafe { obj::set_string_rep(original, b"") };
    if HEADER_STAGE.with(Cell::get) == 1 {
        observe_header_binding();
    }
}
static HEADER_FORMALS: obj::TclObjType = obj::TclObjType {
    name: c"originalCompilerFormals".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(header_formals),
    set_from_any_proc: None,
};
static HEADER_BODY: obj::TclObjType = obj::TclObjType {
    name: c"originalCompilerBody".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(header_body),
    set_from_any_proc: None,
};

#[test]
fn original_compiler_header_keeps_the_published_proc_binding_between_actual_getters() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Fixed-slot continuation is software safety; no Native pointer rename chronology claim.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for stage in 0..2 {
            for mode in 0..7 {
                crate::counters::reset();
                {
                    let mut interpreter = native(profile);
                    let formals = Owned::fresh(obj::alloc_typed(&HEADER_FORMALS, 0));
                    let original_body = Owned::fresh(obj::alloc_typed(&HEADER_BODY, 0));
                    let _cleanup = arm(&interpreter, 0, GLOBAL);
                    HEADER_MODE.with(|value| value.set(mode));
                    HEADER_STAGE.with(|value| value.set(stage));
                    HEADER_GETTERS.with(|value| value.borrow_mut().clear());
                    HEADER_ORIGINAL.with(|value| value.borrow_mut().take());
                    let generation = interpreter.install_proc_chosen_storage(
                        b"headerProbe",
                        vec![Param {
                            name: b"args".to_vec(),
                            default: None,
                        }],
                        Some(formals.as_ptr()),
                        (original_body.as_ptr(), text(b"")),
                        None,
                        None,
                    );
                    assert_eq!(generation.is_some(), mode == 0);
                    assert_eq!(
                        HEADER_GETTERS.with(|value| value.borrow().clone()),
                        if mode == 0 || stage == 1 {
                            b"AB".to_vec()
                        } else {
                            b"A".to_vec()
                        }
                    );
                    check_result(&interpreter);
                    let original_generation = HEADER_GENERATION.with(Cell::get);
                    if mode == 0 {
                        assert_eq!(generation, Some(original_generation));
                        let current = match interpreter
                            .namespaces()
                            .command_in(GLOBAL, b"headerProbe")
                            .unwrap()
                        {
                            Command::Proc(command) => command.declaration(),
                            _ => unreachable!(),
                        };
                        assert_eq!(
                            current.compiler_header.get(),
                            tcl_dialect::NativeProcedureHeaderCompilation::NoOp
                        );
                        assert_eq!(
                            interpreter
                                .namespaces()
                                .native_compiler_recipe(original_generation),
                            Some(crate::namespace::NativeCompilerRecipe::ProcedureNoOp)
                        );
                    } else {
                        let refusal = interpreter.native_execution_refusal().unwrap();
                        if mode == 3 || mode == 5 {
                            assert!(
                                matches!(refusal, tcl_runtime_api::NativeExecutionError::HostCommandRefusal(first) if first.reason == "original compiler header observer")
                            );
                        } else if mode != 4 {
                            assert!(matches!(refusal, tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable("original procedure publication binding changed"))));
                        }
                        if mode == 2 || mode == 6 {
                            assert_eq!(
                                interpreter
                                    .namespaces()
                                    .native_compiler_recipe(original_generation),
                                Some(crate::namespace::NativeCompilerRecipe::Absent)
                            );
                        }
                        if mode == 1 || mode == 5 {
                            let replacement = interpreter
                                .namespaces()
                                .command_generation(GLOBAL, b"headerProbe")
                                .unwrap();
                            assert_ne!(replacement, original_generation);
                            assert_eq!(
                                interpreter.namespaces().native_compiler_recipe(replacement),
                                Some(crate::namespace::NativeCompilerRecipe::Absent)
                            );
                        }
                    }
                    if mode == 2 || mode == 3 || mode == 4 {
                        let original = HEADER_ORIGINAL
                            .with(|value| value.borrow().as_ref().unwrap().upgrade().unwrap());
                        assert_eq!(
                            original.compiler_header.get(),
                            tcl_dialect::NativeProcedureHeaderCompilation::Absent
                        );
                    }
                    assert_eq!(unsafe { (*formals.as_ptr()).ref_count }, 1);
                    assert_eq!(unsafe { (*original_body.as_ptr()).ref_count }, 1);
                    HEADER_ORIGINAL.with(|value| value.borrow_mut().take());
                }
                assert_eq!(crate::counters::finalize(), 0);
                assert_eq!(crate::counters::double_free_count(), 0);
            }
        }
    }
}

#[test]
fn original_publication_receipt_refuses_a_replacement_from_registration_temporary_cleanup() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual unused Jim registration temporary reaches free after insertion, before receipt settlement.
    crate::counters::reset();
    {
        let mut interpreter = native("jim");
        interpreter.bind_command_replacement(GLOBAL, b"retiring", Command::Builtin(builtin));
        let key = body();
        unsafe { obj::set_string_rep(key.as_ptr(), b"retiring") };
        let report = interpreter.namespaces().jim_command_table_key_at(
            GLOBAL,
            b"retiring",
            tcl_syntax::naming::NativeNamePurpose::CommandCApiPublication,
        );
        let _cleanup = arm(&interpreter, 3, GLOBAL);
        let reached = interpreter.bind_command_replacement_with_original_jim_key(
            GLOBAL,
            b"retiring",
            Command::Builtin(builtin),
            report,
            Some(Rc::new(key)),
        );
        assert!(matches!(reached, OriginalCommandPublication::Refused));
        assert_eq!(FREES.with(Cell::get), 1);
        assert_eq!(COUNTS.with(|value| value.borrow().clone()), [0]);
        assert!(matches!(
            interpreter.namespaces().command_in(GLOBAL, b"retiring"),
            Some(Command::Builtin(_))
        ));
        assert!(matches!(
            interpreter.native_execution_refusal(),
            Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                    "original command publication binding changed"
                )
            ))
        ));
        check_result(&interpreter);
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}
