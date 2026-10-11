// SPDX-License-Identifier: AGPL-3.0-or-later
//! Physical compatibility link storage and actual callback/refcount controls.
use super::*;
use crate::{
    interp::{Code, Interp},
    namespace::GLOBAL,
};
use std::cell::{Cell, RefCell};

thread_local! {
    static FIRST: RefCell<Option<tcl_runtime_api::NativeExecutionError>> = const { RefCell::new(None) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static GETTERS: Cell<usize> = const { Cell::new(0) };
    static EXPECTED: Cell<*mut TclObj> = const { Cell::new(std::ptr::null_mut()) };
    static PUBLISHED: Cell<bool> = const { Cell::new(false) };
}
struct Current(*mut Interp);
impl Current {
    fn enter(interp: &mut Interp) -> Self {
        let previous = crate::codegen_abi::current_interp();
        crate::codegen_abi::tcl_runtime_set_current_interp(interp);
        Self(previous)
    }
}
impl Drop for Current {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(self.0);
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
fn reset(mode: u8) {
    FIRST.with(|v| *v.borrow_mut() = None);
    MODE.with(|v| v.set(mode));
    CALLS.with(|v| v.set(0));
    GETTERS.with(|v| v.set(0));
    PUBLISHED.with(|v| v.set(false));
}
fn actual() -> Interp {
    unsafe { crate::codegen_abi::current_interp().as_ref() }
        .unwrap()
        .clone()
}
fn effect(interp: &mut Interp) {
    match MODE.with(Cell::get) {
        1 => {
            interp.refuse_host_command("first compatibility link retirement");
            FIRST.with(|v| *v.borrow_mut() = interp.native_execution_refusal());
        }
        2 => {
            let original = interp.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("compatibility-link-retirement".to_owned(), "1.0".to_owned())];
            interp.pin_context(&changed).unwrap();
            interp.pin_context(&original).unwrap();
        }
        3 => {
            assert!(interp.var_unset(b"root"));
            let replacement = Owned::fresh(obj::new_string_bytes(b"RECREATED"));
            interp.var_set(b"root", replacement.as_ptr()).unwrap();
        }
        _ => {}
    }
}
extern "C" fn retire(_: *mut TclObj) {
    CALLS.with(|v| v.set(v.get() + 1));
    let mut interp = actual();
    // Successful mutable namespace access proves no enclosing owner borrow.
    drop(interp.namespaces_mut());
    let current = interp.var_get(b"root").unwrap();
    PUBLISHED.with(|v| v.set(current == EXPECTED.with(Cell::get)));
    effect(&mut interp);
}
extern "C" fn update(original: *mut TclObj) {
    GETTERS.with(|v| v.set(v.get() + 1));
    let mut interp = actual();
    drop(interp.namespaces_mut());
    effect(&mut interp);
    unsafe { obj::set_native_updater_string_rep(original, b"k OLD", false) };
}
static OLD: obj::TclObjType = obj::TclObjType {
    name: c"compatibilityLinkRetirement".as_ptr(),
    free_int_rep_proc: Some(retire),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};
static LAZY: obj::TclObjType = obj::TclObjType {
    name: c"compatibilityLinkDictionary".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};
fn install_alias(interp: &Interp, element: Option<&[u8]>) {
    interp.namespaces_mut().var_table_mut(GLOBAL).insert_link(
        b"alias",
        super::super::Link {
            original_jim_target: None,
            native_scalar_entry: None,
            native_element_entry: None,
            array_identity: None,
            array_cell: None,
            home: super::super::VarHome::Namespace(GLOBAL),
            name: b"root".to_vec(),
            elem: element.map(<[u8]>::to_vec),
        },
    );
}
fn stored(interp: &mut Interp, value: &Owned) -> Result<(), Code> {
    let name = Owned::fresh(obj::new_string_bytes(b"alias"));
    interp.assign_original_named_variable(name.as_ptr(), value.as_ptr())
}

#[test]
fn captured_compatibility_link_publishes_before_retirement_outside_owner_borrows() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual Link(None) compatibility store, not original Jim recursive-name/COW parity.
    for mode in [0, 1, 2] {
        reset(mode);
        let mut interp = native();
        let _current = Current::enter(&mut interp);
        let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
        interp.var_set(b"root", old.as_ptr()).unwrap();
        drop(old);
        install_alias(&interp, None);
        let value = Owned::fresh(obj::new_string_bytes(b"NEW"));
        EXPECTED.with(|v| v.set(value.as_ptr()));
        let before = unsafe { (*value.as_ptr()).ref_count };
        let reached = stored(&mut interp, &value);
        assert_eq!(CALLS.with(Cell::get), 1);
        assert!(PUBLISHED.with(Cell::get));
        assert_eq!(
            interp.namespaces().var_table(GLOBAL).load_scalar(b"root"),
            Some(value.as_ptr())
        );
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, before + 1);
        if mode == 0 {
            assert_eq!(reached, Ok(()));
        } else {
            assert_eq!(reached, Err(Code::Error));
            assert!(interp.native_execution_refusal().is_some());
            if mode == 1 {
                assert_eq!(
                    interp.native_execution_refusal(),
                    FIRST.with(|v| v.borrow().clone())
                );
            }
        }
        assert_eq!(interp.entered_native_operation_count(), 0);
    }
}

#[test]
fn captured_compatibility_link_does_not_adopt_missing_or_recreated_target() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    reset(3);
    let mut interp = native();
    let _current = Current::enter(&mut interp);
    let old = Owned::fresh(obj::alloc_typed(&OLD, 0));
    interp.var_set(b"root", old.as_ptr()).unwrap();
    drop(old);
    install_alias(&interp, None);
    let value = Owned::fresh(obj::new_string_bytes(b"NEW"));
    EXPECTED.with(|v| v.set(value.as_ptr()));
    assert_eq!(stored(&mut interp, &value), Err(Code::Error));
    assert!(PUBLISHED.with(Cell::get));
    assert_eq!(CALLS.with(Cell::get), 1);
    assert_eq!(
        crate::interp::obj_bytes(interp.namespaces().var_table(GLOBAL).load_scalar(b"root").unwrap()),
        b"RECREATED"
    );
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert!(interp.native_execution_refusal().is_some());
    assert_eq!(interp.entered_native_operation_count(), 0);

    reset(0);
    let mut interp = native();
    install_alias(&interp, None);
    interp.set_result_bytes(b"PRIOR");
    assert_eq!(stored(&mut interp, &value), Err(Code::Error));
    assert!(interp
        .namespaces()
        .var_table(GLOBAL)
        .load_scalar(b"root")
        .is_none());
    assert_eq!(interp.result_bytes(), b"PRIOR");
    assert_eq!(CALLS.with(Cell::get), 0);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
}

#[test]
fn captured_compatibility_dictionary_getter_refusal_stops_copy_and_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2, 3] {
        reset(mode);
        let mut interp = native();
        let _current = Current::enter(&mut interp);
        let root = Owned::fresh(obj::alloc_typed(&LAZY, 0));
        interp
            .associate_native_jim_arguments(&[root.as_ptr()])
            .unwrap();
        interp.var_set(b"root", root.as_ptr()).unwrap();
        install_alias(&interp, Some(b"k"));
        let value = Owned::fresh(obj::new_string_bytes(b"NEW"));
        let refs = unsafe { (*value.as_ptr()).ref_count };
        assert_eq!(stored(&mut interp, &value), Err(Code::Error));
        assert_eq!(GETTERS.with(Cell::get), 1);
        assert!(obj::has_string_rep(root.as_ptr()));
        assert_eq!(obj::obj_type_ptr(root.as_ptr()), &LAZY as *const _);
        assert_eq!(unsafe { (*value.as_ptr()).ref_count }, refs);
        assert!(interp.native_execution_refusal().is_some());
        assert_eq!(interp.entered_native_operation_count(), 0);
        if mode != 3 {
            assert_eq!(
                interp.namespaces().var_table(GLOBAL).load_scalar(b"root"),
                Some(root.as_ptr())
            );
        }
    }
}

#[test]
fn captured_compatibility_dictionary_copy_keeps_original_and_new_owned_members() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Explicit existing compatibility always-copy policy; no original Jim COW claim.
    reset(0);
    let mut interp = native();
    let _current = Current::enter(&mut interp);
    let protocol = interp
        .native_invocation_dialect()
        .native_string_protocol()
        .unwrap();
    let key = Owned::fresh(obj::new_string_bytes(b"k"));
    let old = Owned::fresh(obj::new_string_bytes(b"OLD"));
    let root = Owned::fresh(
        crate::dict::new_dict_obj_native(&[(key.as_ptr(), old.as_ptr())], None, protocol).unwrap(),
    );
    interp.var_set(b"root", root.as_ptr()).unwrap();
    install_alias(&interp, Some(b"k"));
    let value = Owned::fresh(obj::new_string_bytes(b"NEW"));
    assert_eq!(stored(&mut interp, &value), Ok(()));
    let actual = interp.var_get(b"root").unwrap();
    assert_ne!(actual, root.as_ptr());
    assert_eq!(
        crate::dict::dict_get(root.as_ptr(), b"k").unwrap(),
        Some(old.as_ptr())
    );
    assert_eq!(
        crate::dict::dict_get(actual, b"k").unwrap(),
        Some(value.as_ptr())
    );
    assert!(!obj::has_string_rep(root.as_ptr()));
    assert!(!obj::has_string_rep(actual));
    assert_eq!(unsafe { (*root.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*old.as_ptr()).ref_count }, 2);
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 2);
    assert!(interp.native_execution_refusal().is_none());
    assert_eq!(interp.entered_native_operation_count(), 0);
}

#[test]
fn captured_compatibility_root_retirement_stops_list_adoption_on_recreated_cell() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine extension freeIntRep inside the existing Jim List conversion;
    // an independent native owner keeps the old allocation live, not its cell.
    reset(3);
    let mut interp = native();
    let _current = Current::enter(&mut interp);
    let root = Owned::fresh(obj::alloc_typed(&OLD, 0));
    unsafe { obj::set_native_updater_string_rep(root.as_ptr(), b"k OLD", false) };
    interp
        .associate_native_jim_arguments(&[root.as_ptr()])
        .unwrap();
    interp.var_set(b"root", root.as_ptr()).unwrap();
    install_alias(&interp, Some(b"k"));
    EXPECTED.with(|v| v.set(root.as_ptr()));
    let value = Owned::fresh(obj::new_string_bytes(b"NEW"));
    assert_eq!(stored(&mut interp, &value), Err(Code::Error));
    assert_eq!(CALLS.with(Cell::get), 1);
    assert!(PUBLISHED.with(Cell::get));
    assert!(obj::obj_type_ptr(root.as_ptr()).is_null());
    assert_eq!(crate::interp::obj_bytes(root.as_ptr()), b"k OLD");
    assert_eq!(
        crate::interp::obj_bytes(interp.namespaces().var_table(GLOBAL).load_scalar(b"root").unwrap()),
        b"RECREATED"
    );
    assert_eq!(unsafe { (*value.as_ptr()).ref_count }, 1);
    assert_eq!(unsafe { (*root.as_ptr()).ref_count }, 1);
    assert!(interp.native_execution_refusal().is_some());
    assert_eq!(interp.entered_native_operation_count(), 0);
}
