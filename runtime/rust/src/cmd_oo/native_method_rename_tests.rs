// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original rename operands and native name roles, independent of method payload types.
use super::*;
use std::cell::Cell;

thread_local! {
    static OWNER: RefCell<Option<crate::interp::WeakInterp>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static TEXT: RefCell<[Vec<u8>; 3]> = RefCell::new([b"from".to_vec(), b"moved".to_vec(), b"from".to_vec()]);
    static UPDATES: RefCell<[usize; 3]> = const { RefCell::new([0; 3]) };
    static FREES: Cell<usize> = const { Cell::new(0) };
    static OUTSIDE: Cell<bool> = const { Cell::new(false) };
    static PUBLISHED: Cell<bool> = const { Cell::new(false) };
}
fn callback(mode: u8) {
    let mut interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    match mode {
        1 => { interpreter.refuse_host_command("first original rename name callback"); }
        2 => {
            let original = interpreter.runtime_context();
            let mut changed = original.clone();
            changed.packages.push(("original-method-rename".to_owned(), "1.0".to_owned()));
            interpreter.pin_context(&changed).unwrap();
            interpreter.pin_context(&original).unwrap();
        }
        3 => assert_eq!(interpreter.eval_str(b"oo::define RenameClass deletemethod from; oo::define RenameClass forward from list RECREATED"), Code::Ok),
        _ => {}
    }
}
extern "C" fn update_name(original: *mut TclObj) {
    let role = obj::internal_rep(original) as usize;
    UPDATES.with(|updates| updates.borrow_mut()[role] += 1);
    let bytes = TEXT.with(|text| text.borrow()[role].clone());
    unsafe { obj::set_native_updater_string_rep(original, &bytes, false) };
    if role == 1 {
        callback(MODE.with(Cell::get));
    }
}
extern "C" fn free_name(original: *mut TclObj) {
    let role = obj::internal_rep(original) as usize;
    if role != 2 {
        return;
    }
    FREES.with(|count| count.set(count.get() + 1));
    let interpreter = OWNER.with(|owner| owner.borrow().as_ref().unwrap().upgrade().unwrap());
    OUTSIDE.with(|outside| outside.set(interpreter.oo.try_borrow_mut().is_ok()));
    let target = interpreter.oo_resolve_object(b"RenameClass");
    let published = {
        let state = interpreter.oo.borrow();
        state.classes.get(&target).is_some_and(|class| {
            !class.methods.contains_key(b"from".as_slice())
                && class.methods.contains_key(b"moved".as_slice())
        })
    };
    PUBLISHED.with(|observed| observed.set(published));
    drop(interpreter);
    callback(MODE.with(Cell::get));
}
static NAME: obj::TclObjType = obj::TclObjType {
    name: c"originalMethodRenameName".as_ptr(),
    free_int_rep_proc: Some(free_name),
    dup_int_rep_proc: None,
    update_string_proc: Some(update_name),
    set_from_any_proc: None,
};
fn word(bytes: &[u8]) -> obj::Owned {
    obj::Owned::fresh(obj::new_string_bytes(bytes))
}
fn name(role: usize) -> obj::Owned {
    obj::Owned::fresh(obj::alloc_typed(&NAME, role as u64))
}
fn native(version: &str) -> Interp {
    let mut interpreter = Interp::with_native_core(
        crate::interp::default_host(),
        crate::environment::profile_for_dialect(version),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(
        interpreter.eval_str(b"oo::class create RenameClass; RenameClass create RenameObject"),
        Code::Ok
    );
    OWNER.with(|owner| *owner.borrow_mut() = Some(interpreter.downgrade()));
    MODE.with(|mode| mode.set(0));
    UPDATES.with(|updates| *updates.borrow_mut() = [0; 3]);
    FREES.with(|count| count.set(0));
    OUTSIDE.with(|value| value.set(false));
    PUBLISHED.with(|value| value.set(false));
    interpreter
}
fn enter(interpreter: &mut Interp, object: bool) -> (OoId, NsId, Vec<obj::Owned>) {
    let (head, target_name) = if object {
        (b"oo::objdefine".as_slice(), b"RenameObject".as_slice())
    } else {
        (b"oo::define".as_slice(), b"RenameClass".as_slice())
    };
    let target = interpreter.oo_resolve_object(target_name);
    let tape = vec![word(head), word(target_name), word(b"")];
    let pointers: Vec<_> = tape.iter().map(obj::Owned::as_ptr).collect();
    let definition = if object {
        DefTarget::Object(target)
    } else {
        DefTarget::Class(target)
    };
    let caller = interpreter
        .oo_enter_definition(&definition, &pointers)
        .unwrap();
    (target, caller, tape)
}
fn declaration(interpreter: &mut Interp, original: *mut TclObj) {
    let head = word(b"forward");
    let command = word(b"list");
    assert_eq!(
        def_forward(interpreter, &[head.as_ptr(), original, command.as_ptr()]),
        Code::Ok
    );
}
fn owner(
    interpreter: &Interp,
    target: OoId,
    object: bool,
    key: &[u8],
) -> Option<native_method_cache::MethodOwner> {
    let state = interpreter.oo.borrow();
    if object {
        state.objects[&target].methods.owner(key)
    } else {
        state.classes[&target].methods.owner(key)
    }
}
fn stream(version: &str) -> &'static str {
    match version {
        "tcl8.6" => include_str!("../../tests/data/native_method_rename_operands_records_original/510b/providers/8.6.18/execute.stdout"),
        "tcl9.0" => include_str!("../../tests/data/native_method_rename_operands_records_original/510b/providers/9.0.4/execute.stdout"),
        "tcl9.1" => include_str!("../../tests/data/native_method_rename_operands_records_original/510b/providers/9.1.0/execute.stdout"),
        _ => panic!("fixed supported original worker"),
    }
}
fn original_fields(
    version: &str,
    kind: &str,
    case: usize,
    stage: &str,
) -> BTreeMap<&'static str, &'static str> {
    let key = case.to_string();
    stream(version)
        .lines()
        .filter(|line| line.split('\t').next() == Some(kind))
        .map(|line| {
            line.split('\t')
                .skip(1)
                .filter_map(|field| field.split_once('='))
                .collect::<BTreeMap<_, _>>()
        })
        .find(|row| row.get("case") == Some(&key.as_str()) && row.get("stage") == Some(&stage))
        .expect("matched reached stock-worker original phase")
}
fn decode_hex(bytes: &str) -> Vec<u8> {
    bytes
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn original_named_declarations_keep_record_name_and_table_key_roles_separate() {
    // naming.tcloo.original-method-rename-operands-and-records
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-rename-operands-and-records.md
    // Genuine authored/generated declaration roles; shallow Rust observations are not Native copies.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version);
        let (target, caller, _tape) = enter(&mut interpreter, false);
        let original = word(b"forwarded");
        declaration(&mut interpreter, original.as_ptr());
        let record = owner(&interpreter, target, false, b"forwarded").unwrap();
        assert_eq!(record.original_name(), Some(original.as_ptr()));
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 3);
        let export = word(b"export");
        assert_eq!(
            def_export(
                &mut interpreter,
                &[export.as_ptr(), original.as_ptr()],
                true
            ),
            Code::Ok
        );
        let rename = word(b"renamemethod");
        let renamed = word(b"Forwarded");
        assert_eq!(
            def_renamemethod(
                &mut interpreter,
                &[rename.as_ptr(), original.as_ptr(), renamed.as_ptr()]
            ),
            Code::Ok
        );
        {
            let state = interpreter.oo.borrow();
            assert!(state.classes[&target]
                .exported
                .contains(b"Forwarded".as_slice()));
            assert!(!state.classes[&target]
                .exported
                .contains(b"forwarded".as_slice()));
        }
        assert_eq!(
            def_renamemethod(
                &mut interpreter,
                &[rename.as_ptr(), renamed.as_ptr(), original.as_ptr()]
            ),
            Code::Ok
        );
        let incoming = word(b"forwarded");
        declaration(&mut interpreter, incoming.as_ptr());
        assert!(Rc::ptr_eq(
            &record,
            &owner(&interpreter, target, false, b"forwarded").unwrap()
        ));
        assert_eq!(record.original_name(), Some(original.as_ptr()));
        assert_eq!(unsafe { (*incoming.as_ptr()).ref_count }, 1);
        let observed = interpreter.oo.borrow().classes[&target].methods.clone();
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 3);
        let copied = observed.duplicate_declarations().unwrap();
        assert_eq!(
            copied.owner(b"forwarded").unwrap().original_name(),
            Some(original.as_ptr())
        );
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 5);
        drop(copied);
        drop(observed);
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 3);
        let head = word(b"method");
        let method_name = word(b"body");
        let formals = word(b"");
        let body = word(b"return BODY");
        assert_eq!(
            def_method(
                &mut interpreter,
                &[
                    head.as_ptr(),
                    method_name.as_ptr(),
                    formals.as_ptr(),
                    body.as_ptr()
                ]
            ),
            Code::Ok
        );
        assert_eq!(
            owner(&interpreter, target, false, b"body")
                .unwrap()
                .original_name(),
            Some(method_name.as_ptr())
        );
        assert_eq!(unsafe { (*method_name.as_ptr()).ref_count }, 3);
        assert_eq!(interpreter.eval_str(b"oo::define RenameClass constructor {} {return}; oo::define RenameClass destructor {return}"), Code::Ok);
        {
            let state = interpreter.oo.borrow();
            let class = &state.classes[&target];
            assert_eq!(class.constructor.owner().unwrap().original_name(), None);
            assert_eq!(class.destructor.owner().unwrap().original_name(), None);
            let root = &state.classes[&state.object_root.unwrap()];
            assert!(root
                .methods
                .owner(b"unknown")
                .unwrap()
                .original_name()
                .is_some());
        }
        interpreter.oo_leave_definition(caller);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}

#[test]
fn original_rename_verdict_and_counted_headers_match_reached_stock_worker_stages() {
    // naming.tcloo.original-method-rename-operands-and-records
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-rename-operands-and-records.md
    // Match stock worker name/verdict/role phases only; the Runtime forward payload is independent.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for object in [false, true] {
            for scenario in 0..8 {
                let mut interpreter = native(version);
                let (target, caller, _tape) = enter(&mut interpreter, object);
                let (from, to, declared, occupied): (&[u8], &[u8], &[u8], &[u8]) = match scenario {
                    0 => (b"missing", b"unused", b"from", b"occupied"),
                    1 => (b"from", b"from", b"from", b"occupied"),
                    2 => (b"from", b"occupied", b"from", b"occupied"),
                    4 => (b"from", b"Upper", b"from", b"occupied"),
                    5 => (b"f\0x", b"t\0y", b"f\0x", b"occupied"),
                    6 => (b"from", b"o\0x", b"from", b"o\0x"),
                    7 => (b"::from", b"::moved", b"::from", b"occupied"),
                    _ => (b"from", b"moved", b"from", b"occupied"),
                };
                let declared_name = word(declared);
                let occupied_name = word(occupied);
                declaration(&mut interpreter, declared_name.as_ptr());
                declaration(&mut interpreter, occupied_name.as_ptr());
                // The original probe declares both Method records with an explicit public flag.
                let export = word(b"export");
                assert_eq!(
                    def_export(
                        &mut interpreter,
                        &[
                            export.as_ptr(),
                            declared_name.as_ptr(),
                            occupied_name.as_ptr()
                        ],
                    ),
                    Code::Ok
                );
                let record = Rc::downgrade(&owner(&interpreter, target, object, declared).unwrap());
                TEXT.with(|text| {
                    text.borrow_mut()[0] = from.to_vec();
                    text.borrow_mut()[1] = to.to_vec();
                });
                let source = name(0);
                let destination = name(1);
                let head = word(b"renamemethod");
                interpreter.error_with_code(b"BEFORE", b"SEEDED CODE");
                let code = def_renamemethod(
                    &mut interpreter,
                    &[head.as_ptr(), source.as_ptr(), destination.as_ptr()],
                );
                let case = 16 + usize::from(object) * 8 + scenario;
                let row = original_fields(
                    version,
                    "ROW",
                    case,
                    "after-rename-worker-before-presentation",
                );
                assert_eq!(
                    i32::from(code == Code::Error),
                    row["status"].parse::<i32>().unwrap()
                );
                assert_eq!(
                    UPDATES.with(|updates| updates.borrow()[0]),
                    row["n0_updates"].parse::<usize>().unwrap()
                );
                assert_eq!(
                    UPDATES.with(|updates| updates.borrow()[1]),
                    row["n1_updates"].parse::<usize>().unwrap()
                );
                assert_eq!(
                    unsafe { (*destination.as_ptr()).ref_count },
                    row["n1_refs"].parse::<isize>().unwrap()
                );
                let result = original_fields(version, "RESULT", case, "inner-rename");
                assert_eq!(interpreter.result_bytes(), decode_hex(result["hex"]));
                let error = original_fields(version, "ERRORCODE", case, "inner-rename");
                assert_eq!(interpreter.error_code(), decode_hex(error["hex"]));
                let source_record = record.upgrade().unwrap();
                if code == Code::Ok {
                    assert!(owner(&interpreter, target, object, from).is_none());
                    assert!(Rc::ptr_eq(
                        &source_record,
                        &owner(&interpreter, target, object, to).unwrap()
                    ));
                    assert_eq!(source_record.original_name(), Some(destination.as_ptr()));
                    assert_eq!(unsafe { (*declared_name.as_ptr()).ref_count }, 1);
                    let state = interpreter.oo.borrow();
                    let unexported = if object {
                        &state.objects[&target].unexported
                    } else {
                        &state.classes[&target].unexported
                    };
                    assert!(
                        !unexported.contains(to),
                        "rename preserves the original public flag including Upper"
                    );
                } else {
                    assert_eq!(source_record.original_name(), Some(declared_name.as_ptr()));
                    assert!(Rc::ptr_eq(
                        &source_record,
                        &owner(&interpreter, target, object, declared).unwrap()
                    ));
                    assert_eq!(unsafe { (*declared_name.as_ptr()).ref_count }, 3);
                }
                drop(source_record);
                interpreter.oo_leave_definition(caller);
                assert_eq!(interpreter.entered_native_operation_count(), 0);
                OWNER.with(|owner| *owner.borrow_mut() = None);
            }
        }
    }
}

#[test]
fn original_destination_callback_cannot_replace_source_identity_or_first_cause() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual software Host/context/source replacement; not an original provider callback verdict.
    for mode in [1, 2, 3] {
        let mut interpreter = native("tcl8.6");
        let (target, caller, _tape) = enter(&mut interpreter, false);
        let original = word(b"from");
        declaration(&mut interpreter, original.as_ptr());
        let record = Rc::downgrade(&owner(&interpreter, target, false, b"from").unwrap());
        TEXT.with(|text| {
            text.borrow_mut()[0] = b"from".to_vec();
            text.borrow_mut()[1] = b"moved".to_vec();
        });
        let source = name(0);
        let destination = name(1);
        let head = word(b"renamemethod");
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        MODE.with(|selected| selected.set(mode));
        assert_eq!(
            def_renamemethod(
                &mut interpreter,
                &[head.as_ptr(), source.as_ptr(), destination.as_ptr()]
            ),
            Code::Error
        );
        assert_eq!(UPDATES.with(|updates| updates.borrow()[1]), 1);
        assert_eq!(unsafe { (*destination.as_ptr()).ref_count }, 1);
        assert!(!interpreter.oo.borrow().classes[&target]
            .methods
            .contains_key(b"moved".as_slice()));
        if mode == 3 {
            assert!(record.upgrade().is_none());
            assert!(owner(&interpreter, target, false, b"from").is_some());
        } else {
            assert_eq!(interpreter.get_obj_result(), prior);
            assert!(interpreter.native_execution_refusal().is_some());
            assert!(record.upgrade().is_some());
        }
        interpreter.oo_leave_definition(caller);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        MODE.with(|selected| selected.set(0));
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}

#[test]
fn original_old_name_retirement_observes_destination_then_stops_later_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Software free-hook/currentness control; native names own two real separate references.
    for mode in [0, 1, 2] {
        let mut interpreter = native("tcl8.6");
        let (target, caller, _tape) = enter(&mut interpreter, false);
        TEXT.with(|text| text.borrow_mut()[2] = b"from".to_vec());
        let original = name(2);
        declaration(&mut interpreter, original.as_ptr());
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 3);
        drop(original);
        let source = word(b"from");
        let destination = word(b"moved");
        let head = word(b"renamemethod");
        interpreter.set_result_bytes(b"PRIOR");
        let prior = interpreter.get_obj_result();
        MODE.with(|selected| selected.set(mode));
        assert_eq!(
            def_renamemethod(
                &mut interpreter,
                &[head.as_ptr(), source.as_ptr(), destination.as_ptr()]
            ),
            if mode == 0 { Code::Ok } else { Code::Error }
        );
        assert_eq!(FREES.with(Cell::get), 1);
        assert!(OUTSIDE.with(Cell::get));
        assert!(PUBLISHED.with(Cell::get));
        assert_eq!(
            owner(&interpreter, target, false, b"moved")
                .unwrap()
                .original_name(),
            Some(destination.as_ptr())
        );
        assert_eq!(unsafe { (*destination.as_ptr()).ref_count }, 3);
        if mode != 0 {
            assert_eq!(interpreter.get_obj_result(), prior);
            assert!(interpreter.native_execution_refusal().is_some());
        }
        interpreter.oo_leave_definition(caller);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        MODE.with(|selected| selected.set(0));
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}

#[test]
fn original_self_method_report_keeps_current_entered_name_and_unnamed_roles_distinct() {
    // naming.tcloo.original-method-rename-operands-and-records
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-rename-operands-and-records.md
    // Source-selected SELF_METHOD name role; no measured custom MethodType payload/body equivalence.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interpreter = native(version);
        let (target, caller, _tape) = enter(&mut interpreter, false);
        let head = word(b"method");
        let original = word(b"from");
        let formals = word(b"");
        let body = word(b"oo::define RenameClass renamemethod from moved; self method");
        assert_eq!(
            def_method(
                &mut interpreter,
                &[
                    head.as_ptr(),
                    original.as_ptr(),
                    formals.as_ptr(),
                    body.as_ptr()
                ]
            ),
            Code::Ok
        );
        let record = Rc::downgrade(&owner(&interpreter, target, false, b"from").unwrap());
        interpreter.oo_leave_definition(caller);
        assert_eq!(interpreter.eval_str(b"RenameObject from"), Code::Ok);
        let current = owner(&interpreter, target, false, b"moved").unwrap();
        assert!(Rc::ptr_eq(&record.upgrade().unwrap(), &current));
        assert_eq!(
            interpreter.get_obj_result(),
            current.original_name().unwrap()
        );
        assert_eq!(interpreter.result_bytes(), b"moved");
        assert_eq!(unsafe { (*original.as_ptr()).ref_count }, 1);
        assert_eq!(interpreter.entered_native_operation_count(), 0);
        // Unnamed slots retain no named-record role. Foundation-owned report
        // headers are a separate consumer purpose, not supplied by this owner.
        assert_eq!(interpreter.eval_str(b"oo::define RenameClass constructor {} {return}; oo::define RenameClass destructor {return}"), Code::Ok);
        let state = interpreter.oo.borrow();
        assert_eq!(
            state.classes[&target]
                .constructor
                .owner()
                .unwrap()
                .original_name(),
            None
        );
        assert_eq!(
            state.classes[&target]
                .destructor
                .owner()
                .unwrap()
                .original_name(),
            None
        );
        drop(state);
        OWNER.with(|owner| *owner.borrow_mut() = None);
    }
}
