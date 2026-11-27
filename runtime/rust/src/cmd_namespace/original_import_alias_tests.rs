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

thread_local! {
    static OWNER: RefCell<Option<Interp>> = const { RefCell::new(None) };
    static READS: Cell<usize> = const { Cell::new(0) };
    static MODE: Cell<u8> = const { Cell::new(0) };
    static FIRST: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
    static DESTINATION: Cell<NsId> = const { Cell::new(GLOBAL) };
    static GETTER_RESULT: RefCell<Option<obj::NativeObjectLifetime>> = const { RefCell::new(None) };
}

extern "C" fn update(original: *mut TclObj) {
    READS.with(|reads| reads.set(reads.get() + 1));
    let mode = MODE.with(Cell::get);
    let bytes: &[u8] = if mode >= 3 {
        b"::src::branch"
    } else {
        b"MEMBER"
    };
    unsafe { obj::set_native_updater_string_rep(original, bytes, false) };
    if mode == 0 || mode == 4 {
        return;
    }
    let mut interp = OWNER.with(|owner| owner.borrow().as_ref().unwrap().clone());
    let result = GETTER_RESULT.with(|result| {
        result
            .borrow()
            .as_ref()
            .map(|original| original.checked_ptr().unwrap())
    });
    if let Some(result) = result {
        interp.set_result(result);
    }
    match mode {
        1 => {
            interp.refuse_host_command("original Jim import member first cause");
            FIRST.with(|first| *first.borrow_mut() = interp.native_execution_refusal());
        }
        2 => {
            let current = interp.runtime_context();
            let mut changed = current.clone();
            changed.packages = vec![("original-import-member".to_owned(), "1.0".to_owned())];
            interp.pin_context(&changed).unwrap();
            interp.pin_context(&current).unwrap();
        }
        3 => {
            let destination = DESTINATION.with(Cell::get);
            assert_ne!(interp.current_ns(), destination);
            interp.set_current_ns(destination);
        }
        _ => unreachable!(),
    }
}

static TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalJimImportMember".as_ptr(),
    free_int_rep_proc: None,
    dup_int_rep_proc: None,
    update_string_proc: Some(update),
    set_from_any_proc: None,
};

fn native() -> Interp {
    OWNER.with(|owner| owner.borrow_mut().take());
    READS.with(|reads| reads.set(0));
    MODE.with(|mode| mode.set(0));
    FIRST.with(|first| first.borrow_mut().take());
    DESTINATION.with(|destination| destination.set(GLOBAL));
    GETTER_RESULT.with(|result| result.borrow_mut().take());
    let mut interp = Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::resolve_environment("jim").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    crate::cmd_proc::install_stock_scripted_library(
        &mut interp,
        tcl_registry::native_scripted_distribution::NativeScriptedLibrary::NamespaceHelpers,
    );
    assert_eq!(
        interp.eval_str(b"namespace eval src {}; namespace eval dest {}"),
        Code::Ok
    );
    OWNER.with(|owner| *owner.borrow_mut() = Some(interp.clone()));
    interp
}

fn define(interp: &mut Interp, name: &[u8], members: &[*mut TclObj]) -> Owned {
    let head = Owned::fresh(obj::new_string_bytes(b"alias"));
    let name = Owned::fresh(obj::new_string_bytes(name));
    let mut argv = vec![head.as_ptr(), name.as_ptr()];
    argv.extend_from_slice(members);
    assert_eq!(interp.dispatch(&argv), Code::Ok);
    let query = tcl_runtime_api::Aliases::alias_prefix_original_value_at(
        interp,
        tcl_runtime_api::NsId(GLOBAL as u32),
        &name.as_ptr(),
    )
    .unwrap();
    let tcl_runtime_api::AliasPrefixLookup::Prefix(original) = query else {
        panic!("actual just-published alias prefix");
    };
    let stored = Owned::retain(original);
    assert_eq!(
        ValueOps::list_elements(interp, &stored.as_ptr()).unwrap(),
        members
    );
    interp.set_result_bytes(b"");
    stored
}

fn import(interp: &mut Interp, destination: NsId, pattern: &[u8]) -> Code {
    interp.set_current_ns(destination);
    let words = [b"namespace".as_slice(), b"import", pattern]
        .map(|word| Owned::fresh(obj::new_string_bytes(word)));
    let argv = words.each_ref().map(Owned::as_ptr);
    import_arguments(interp, destination, &argv)
}

fn import_arguments(interp: &mut Interp, destination: NsId, argv: &[*mut TclObj]) -> Code {
    DESTINATION.with(|selected| selected.set(destination));
    // The original helper reads its caller with uplevel. Enter the retained
    // namespace token through the existing namespace activation owner.
    let body = Owned::fresh(interp.new_list_object(argv));
    let code = interp.ns_eval_in_token(
        destination,
        b"",
        None,
        false,
        Some((
            tcl_native_model::native_eval_object::EvalObjectPurpose::NamespaceBody,
            body.as_ptr(),
        )),
        None,
    );
    assert_eq!(interp.current_ns(), destination);
    assert_eq!(interp.entered_native_operation_count(), 0);
    code
}

fn destination(interp: &mut Interp) -> super::original_argument_tests::JimFixtureNamespace {
    super::original_argument_tests::jim_fixture_namespace(interp, b"dest")
}

// The pinned Jim helper publishes the generated destination at the flat
// root table. Issue that exact slot before the tested getter/effect, then
// inspect only retained storage after any operational refusal.
fn destination_slot(interp: &Interp, destination: NsId) -> Vec<u8> {
    let namespaces = interp.namespaces();
    let mut generated = namespaces
        .jim_namespace_bytes(destination)
        .expect("retained reached destination namespace")
        .to_vec();
    generated.extend_from_slice(b"::branch");
    let slot = interp
        .name_policy_protocol()
        .unwrap()
        .recipe()
        .alias_publication_slot(tcl_syntax::naming::NativeNameContext::root(), &generated)
        .unwrap();
    assert!(slot.namespace.is_root());
    slot.simple.as_bytes().to_vec()
}

#[test]
fn original_jim_import_refuses_a_pattern_getter_that_switches_the_current_namespace() {
    for change in [false, true] {
        let mut interp = native();
        let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
            OWNER.with(|owner| owner.borrow_mut().take());
        });
        assert_eq!(
            interp.eval_str(b"proc src::branch {} {return TARGET}"),
            Code::Ok
        );
        let destination_owner = destination(&mut interp);
        let destination = destination_owner.token;
        let slot = destination_slot(&interp, destination);
        interp.set_current_ns(destination);
        MODE.with(|mode| mode.set(if change { 3 } else { 4 }));
        let pattern = Owned::fresh(obj::alloc_typed(&TYPE, 0));
        let head = Owned::fresh(obj::new_string_bytes(b"namespace"));
        let selector = Owned::fresh(obj::new_string_bytes(b"import"));
        let code = import_arguments(
            &mut interp,
            destination,
            &[head.as_ptr(), selector.as_ptr(), pattern.as_ptr()],
        );
        assert_eq!(code, if change { Code::Error } else { Code::Ok });
        assert_eq!(READS.with(Cell::get), 1);
        assert_eq!(
            interp.namespaces().command_in(GLOBAL, &slot).is_some(),
            !change
        );
        assert_eq!(interp.native_execution_refusal().is_some(), change);
        assert_eq!(interp.entered_native_operation_count(), 0);
        interp.set_current_ns(GLOBAL);
    }
}

#[test]
fn original_jim_import_prefix_keeps_the_enumerated_hash_key_after_source_deletion() {
    let mut interp = native();
    assert_eq!(
        interp.eval_str(b"namespace eval src {proc x {} {return OLD}}"),
        Code::Ok,
        "Jim source setup: native refusal {:?}; result {:?}",
        interp.native_execution_refusal(),
        interp.result_bytes()
    );
    let key = interp
        .namespaces()
        .original_jim_command_key(GLOBAL, b"src::x")
        .unwrap();
    let before = unsafe { (*key).ref_count };
    let imported = interp.eval_str(b"namespace eval dest {namespace import ::src::x}");
    assert_eq!(
        imported,
        Code::Ok,
        "Jim import: native refusal before projection {:?}; result {:?}; actual completion {:?}",
        interp.native_execution_refusal(),
        interp.result_bytes(),
        crate::completion::capture_bytes(&mut interp, imported)
    );
    let name = Owned::fresh(obj::new_string_bytes(b"dest::x"));
    let query = tcl_runtime_api::Aliases::alias_prefix_original_value_at(
        &mut interp,
        tcl_runtime_api::NsId(GLOBAL as u32),
        &name.as_ptr(),
    )
    .unwrap();
    let tcl_runtime_api::AliasPrefixLookup::Prefix(prefix) = query else {
        panic!("actual imported alias prefix");
    };
    assert_eq!(
        ValueOps::list_elements(&mut interp, &prefix).unwrap(),
        vec![key]
    );
    assert_eq!(unsafe { (*key).ref_count }, before + 1);
    assert_eq!(interp.eval_str(b"rename src::x {}"), Code::Ok);
    assert!(obj::check_native_liveness(key).is_ok());
    assert_eq!(unsafe { (*key).ref_count }, 1);
    assert!(interp
        .namespaces()
        .original_jim_command_enumeration_keys(&[b"src::x".to_vec()])
        .is_err());
    assert_eq!(
        interp.eval_str(b"namespace eval src {proc x {} {return NEW}}"),
        Code::Ok
    );
    let replacement = interp
        .namespaces()
        .original_jim_command_key(GLOBAL, b"src::x")
        .unwrap();
    assert_ne!(replacement, key);
    assert_eq!(
        ValueOps::list_elements(&mut interp, &prefix).unwrap(),
        vec![key]
    );
    assert!(!interp.host_refusal_pending());
    OWNER.with(|owner| owner.borrow_mut().take());
}

#[test]
fn original_jim_import_uses_rooted_hash_key_spelling_in_helper_operations() {
    let mut interp = native();
    assert_eq!(
            interp.eval_str(b"namespace eval src {proc ::src::x {} {return OLD}}; namespace eval dest {namespace import ::src::x}; info alias dest::x"),
            Code::Ok
        );
    assert_eq!(interp.result_bytes(), b"::src::x");
    assert_eq!(
        interp.eval_str(b"namespace eval src {namespace import ::src::x}"),
        Code::Ok
    );
    assert!(!interp.host_refusal_pending());
    OWNER.with(|owner| owner.borrow_mut().take());
}

#[test]
fn original_jim_import_uses_the_cached_alias_list_without_member_getters() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    // Software helper control: cached List and original member identity;
    // no original Enum name header or C import equivalence is inferred.
    let mut interp = native();
    let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
        OWNER.with(|owner| owner.borrow_mut().take());
    });
    let target = Owned::fresh(obj::new_string_bytes(b"list"));
    let member = Owned::fresh(obj::alloc_typed(&TYPE, 0));
    let stored = define(
        &mut interp,
        b"src::branch",
        &[target.as_ptr(), member.as_ptr()],
    );
    assert_eq!(READS.with(Cell::get), 0);
    assert_eq!(
        ValueOps::native_string_bytes(&mut interp, &stored.as_ptr())
            .unwrap()
            .as_ref(),
        b"list MEMBER"
    );
    assert_eq!(READS.with(Cell::get), 1);
    obj::invalidate_string(member.as_ptr());
    READS.with(|reads| reads.set(0));
    MODE.with(|mode| mode.set(1));
    let refs = unsafe { (*member.as_ptr()).ref_count };
    let destination_owner = destination(&mut interp);
    let destination = destination_owner.token;
    let destination_slot = destination_slot(&interp, destination);
    assert_eq!(import(&mut interp, destination, b"::src::branch"), Code::Ok);
    assert_eq!(READS.with(Cell::get), 0);
    assert!(!obj::has_string_rep(member.as_ptr()));
    assert_eq!(unsafe { (*member.as_ptr()).ref_count }, refs);
    let name = Owned::fresh(obj::new_string_bytes(b"src::branch"));
    let tcl_runtime_api::AliasPrefixLookup::Prefix(after) =
        tcl_runtime_api::Aliases::alias_prefix_original_value_at(
            &mut interp,
            tcl_runtime_api::NsId(GLOBAL as u32),
            &name.as_ptr(),
        )
        .unwrap()
    else {
        panic!("retained source alias");
    };
    assert_eq!(after, stored.as_ptr());
    assert_eq!(
        ValueOps::list_elements(&mut interp, &stored.as_ptr()).unwrap(),
        vec![target.as_ptr(), member.as_ptr()]
    );
    assert!(matches!(
        interp.namespaces().command_in(GLOBAL, &destination_slot),
        Some(Command::Alias { .. })
    ));
    assert!(interp.native_execution_refusal().is_none());
    OWNER.with(|owner| owner.borrow_mut().take());
}

#[cfg(debug_assertions)]
#[test]
fn original_jim_import_retains_reached_list_getter_first_cause_before_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for mode in [1, 2] {
        let mut interp = native();
        let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
            GETTER_RESULT.with(|result| result.borrow_mut().take());
            OWNER.with(|owner| owner.borrow_mut().take());
        });
        let target = Owned::fresh(obj::new_string_bytes(b"list"));
        let member = Owned::fresh(obj::alloc_typed(&TYPE, 0));
        let stored = define(
            &mut interp,
            b"src::branch",
            &[target.as_ptr(), member.as_ptr()],
        );
        let backing = list::native_list_backing(stored.as_ptr()).unwrap();
        let destination_owner = destination(&mut interp);
        let destination = destination_owner.token;
        let destination_slot = destination_slot(&interp, destination);
        let prior = Owned::fresh(obj::new_string_bytes(b"PRIOR\0\xff"));
        interp.set_result(prior.as_ptr());
        // Successful helper commands precede this List getter. The updater
        // installs the retained result immediately before its Host effect.
        GETTER_RESULT.with(|result| {
            *result.borrow_mut() = Some(obj::NativeObjectLifetime::retain(prior.as_ptr()));
        });
        let refs = unsafe { (*member.as_ptr()).ref_count };
        let current = interp.runtime_context();
        MODE.with(|selected| selected.set(mode));
        assert_eq!(
            import(&mut interp, destination, b"::src::branch"),
            Code::Error
        );
        assert_eq!(READS.with(Cell::get), 1);
        assert!(interp
            .namespaces()
            .command_in(GLOBAL, &destination_slot)
            .is_none());
        assert_eq!(interp.result_obj(), prior.as_ptr());
        assert_eq!(obj::bytes_of(prior.as_ptr()), b"PRIOR\0\xff");
        assert_eq!(interp.runtime_context(), current);
        assert_eq!(unsafe { (*member.as_ptr()).ref_count }, refs);
        assert_eq!(
            &*backing.elements().unwrap(),
            &[target.as_ptr(), member.as_ptr()]
        );
        let cause = interp
            .native_execution_refusal()
            .expect("first original operational cause");
        if mode == 1 {
            assert_eq!(Some(cause), FIRST.with(|first| first.borrow().clone()));
        } else {
            assert_eq!(
                cause,
                NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "stale entered native operation"
                    )
                )
            );
        }
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}

#[test]
fn original_jim_import_follows_original_alias_lists_and_refuses_destination_loops() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    for closes_loop in [false, true] {
        let mut interp = native();
        let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
            OWNER.with(|owner| owner.borrow_mut().take());
        });
        let destination_owner = destination(&mut interp);
        let destination = destination_owner.token;
        let destination_slot = destination_slot(&interp, destination);
        let end = Owned::fresh(obj::new_string_bytes(if closes_loop {
            b"dest::branch"
        } else {
            b"list"
        }));
        let middle = define(&mut interp, b"chain", &[end.as_ptr()]);
        let chain = Owned::fresh(obj::new_string_bytes(b"chain"));
        let source = define(&mut interp, b"src::branch", &[chain.as_ptr()]);
        assert_eq!(
            import(&mut interp, destination, b"::src::branch"),
            if closes_loop { Code::Error } else { Code::Ok }
        );
        assert_eq!(
            interp
                .namespaces()
                .command_in(GLOBAL, &destination_slot)
                .is_some(),
            !closes_loop
        );
        if closes_loop {
            assert_eq!(
                interp.result_bytes(),
                b"import pattern \"::src::branch\" would create a loop"
            );
        }
        assert!(interp.native_execution_refusal().is_none());
        assert!(obj::has_string_rep(source.as_ptr()));
        assert!(obj::has_string_rep(middle.as_ptr()));
        OWNER.with(|owner| owner.borrow_mut().take());
    }
}

#[test]
fn original_jim_import_keeps_root_alias_lookup_separate_from_destination_collisions() {
    // naming.alias.jim-original-core-prefix-object-storage
    // docs/design/analysis/name-resolution-proofs/alias-jim-original-core-prefix-object-storage.md
    let mut interp = native();
    let _cleanup = super::original_argument_tests::FixtureCleanup(|| {
        OWNER.with(|owner| owner.borrow_mut().take());
    });
    assert_eq!(interp.eval_str(b"namespace eval dest::src {}"), Code::Ok);
    let destination_owner = destination(&mut interp);
    let destination = destination_owner.token;
    let destination_slot = destination_slot(&interp, destination);
    let list = Owned::fresh(obj::new_string_bytes(b"list"));
    let source = define(&mut interp, b"src::branch", &[list.as_ptr()]);
    let loop_destination = Owned::fresh(obj::new_string_bytes(b"dest::branch"));
    let collision = define(
        &mut interp,
        b"dest::src::branch",
        &[loop_destination.as_ptr()],
    );
    assert_eq!(import(&mut interp, destination, b"::src::branch"), Code::Ok);
    assert!(interp
        .namespaces()
        .command_in(GLOBAL, &destination_slot)
        .is_some());
    assert!(obj::has_string_rep(source.as_ptr()));
    assert!(!obj::has_string_rep(collision.as_ptr()));
    assert!(interp.native_execution_refusal().is_none());
    let name = Owned::fresh(obj::new_string_bytes(b"src::branch"));
    assert!(tcl_runtime_api::Aliases::alias_prefix_original_value_at(
        &mut interp,
        tcl_runtime_api::NsId(u32::MAX),
        &name.as_ptr()
    )
    .is_err());
    assert_eq!(interp.current_ns(), destination);
    OWNER.with(|owner| owner.borrow_mut().take());
}
