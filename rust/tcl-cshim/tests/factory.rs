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

//! A command that creates and deletes commands from inside its own
//! invocation — the factory shape C extensions use routinely — published
//! through the engine's registration door before the next statement runs.

use std::ffi::{c_int, c_void};

use tcl_cshim::{Interp, InterpState, Obj, ffi};
use tcl_engine_tclvm::TclVmEngine;

#[cfg(cshim_c_tests)]
unsafe extern "C" {
    fn IndexCache_Init(interp: *mut c_void) -> c_int;
    fn IndexCache_ChangeTable();
}

#[cfg(cshim_c_tests)]
unsafe extern "C" fn index_cache_init(interp: *mut InterpState) -> c_int {
    // SAFETY: the linked extension treats Tcl_Interp as an opaque pointer;
    // load_static supplies this live shim-owned interpreter.
    unsafe { IndexCache_Init(interp.cast()) }
}

#[cfg(cshim_c_tests)]
#[test]
fn index_cache_retains_original_table_and_survives_command_retirement() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: this linked static extension and table live for the entire process.
    unsafe { interp.load_static(index_cache_init) }.unwrap();
    interp.eval("set ::original a; set ::temporary a").unwrap();
    let original = interp.engine_mut().vm_mut().get_var("::original").unwrap();
    let identity = original.native_object_identity();
    interp.eval("original_index $::original").unwrap();
    let (cache, origin) = original.native_index_cache().unwrap();
    assert_eq!(origin, tcl_dialect::TclVersion::V9_0);
    assert_eq!(original.native_object_identity(), identity);
    assert_eq!(cache.word().unwrap().as_ref(), b"alpha");
    assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"a");
    interp.eval("temporary_index $::temporary").unwrap();
    assert!(
        interp
            .engine_mut()
            .vm_mut()
            .get_var("::temporary")
            .unwrap()
            .native_index_cache()
            .is_none()
    );
    // SAFETY: the extension mutates its own persistent table, with no concurrent use.
    unsafe { IndexCache_ChangeTable() };
    assert_eq!(cache.word().unwrap().as_ref(), b"changed");
    interp
        .eval("rename original_index {}; rename temporary_index {}")
        .unwrap();
    assert_eq!(cache.word().unwrap().as_ref(), b"changed");
    let rebuilt = tcl_vm::Value::from_native_index_cache(
        cache,
        origin,
        tcl_registry::InvocationDialect::for_version(origin),
        None,
    )
    .unwrap();
    let bytes = rebuilt
        .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::C(origin))
        .unwrap();
    assert_eq!(bytes.as_ref(), b"changed");
}

/// `made ?arg …?` — answers with its arguments as a list.
unsafe extern "C" fn made(
    _client_data: *mut c_void,
    interp: *mut InterpState,
    word_count: c_int,
    words: *const *mut Obj,
) -> c_int {
    // SAFETY: the shim passes a live interpreter and `word_count` objects.
    unsafe {
        let list = ffi::tcl_new_list_obj(
            isize::try_from(word_count).expect("small") - 1,
            words.add(1),
        );
        ffi::tcl_set_obj_result(interp, list);
    }
    ffi::TCL_OK
}

/// `factory NAME` — creates command NAME; `forget NAME` — deletes it.
unsafe extern "C" fn factory(
    client_data: *mut c_void,
    interp: *mut InterpState,
    word_count: c_int,
    words: *const *mut Obj,
) -> c_int {
    // SAFETY: as above; `client_data` is the non-null marker for `forget`.
    unsafe {
        if word_count != 2 {
            ffi::tcl_wrong_num_args(interp, 1, words, c"name".as_ptr());
            return ffi::TCL_ERROR;
        }
        let name = ffi::tcl_get_string(*words.add(1));
        if client_data.is_null() {
            ffi::tcl_create_obj_command(interp, name, made, std::ptr::null_mut(), None);
        } else {
            let deleted = ffi::tcl_delete_command(interp, name);
            ffi::tcl_set_obj_result(interp, ffi::tcl_new_int_obj(c_int::from(deleted == 0)));
        }
    }
    ffi::TCL_OK
}

unsafe extern "C" fn init(interp: *mut InterpState) -> c_int {
    // SAFETY: the shim passes a live interpreter.
    unsafe {
        ffi::tcl_create_obj_command(
            interp,
            c"factory".as_ptr(),
            factory,
            std::ptr::null_mut(),
            None,
        );
        ffi::tcl_create_obj_command(
            interp,
            c"forget".as_ptr(),
            factory,
            std::ptr::from_ref(&init).cast_mut().cast::<c_void>(),
            None,
        );
    }
    ffi::TCL_OK
}

#[test]
fn smoke_a_command_created_mid_script_is_callable_by_the_next_statement() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: `init` is written against the shim's own exports.
    unsafe { interp.load_static(init) }.expect("loads");
    // A script's result crosses back as text (the engine's rule); the
    // factory's product is what is under test.
    let answer = interp
        .eval("factory made_one\nmade_one a {b c}")
        .expect("the factory's product is callable in the same script");
    assert_eq!(answer.as_str(), Some("a {b c}"));
    assert_eq!(
        interp.commands(),
        ["factory", "forget", "made_one"].map(tcl_core_types::NameBytes::from)
    );
}

#[test]
fn a_command_deleted_mid_script_is_gone_for_the_next_statement() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: as above.
    unsafe { interp.load_static(init) }.expect("loads");
    interp.eval("factory doomed").expect("creates");
    let error = interp
        .eval("forget doomed\ndoomed")
        .expect_err("the deletion is visible to the next statement");
    assert!(
        error.script_message_bytes() == Some("invalid command name \"doomed\"".as_bytes()),
        "{error:?}"
    );
    assert_eq!(
        interp.commands(),
        ["factory", "forget"].map(tcl_core_types::NameBytes::from)
    );
}

#[test]
fn a_command_may_delete_itself() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: as above.
    unsafe { interp.load_static(init) }.expect("loads");
    let answer = interp
        .eval("forget forget")
        .expect("deleting the running command is safe");
    assert_eq!(answer.as_str(), Some("1"));
    assert_eq!(
        interp.commands(),
        ["factory"].map(tcl_core_types::NameBytes::from)
    );
}

#[test]
fn script_retirement_runs_native_delete_before_unbinding_and_keeps_replacement() {
    struct Context {
        interp: *mut InterpState,
        calls: std::cell::Cell<usize>,
        saw_original: std::cell::Cell<bool>,
    }
    unsafe extern "C" fn replace_at_retirement(data: *mut c_void) {
        // SAFETY: the test owns this context and the interpreter until all
        // native retirement callbacks have returned.
        let context = unsafe { &*data.cast::<Context>() };
        context.calls.set(context.calls.get() + 1);
        context.saw_original.set(
            unsafe { &*context.interp }
                .command_names()
                .iter()
                .any(|name| name.as_bytes() == b"doomed"),
        );
        // SAFETY: the registered callback has its live scoped C interpreter.
        unsafe {
            ffi::tcl_create_obj_command(
                context.interp,
                c"doomed".as_ptr(),
                made,
                std::ptr::null_mut(),
                None,
            );
        }
    }
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: init uses the shim's supported registration ABI.
    unsafe { interp.load_static(init) }.unwrap();
    let context = Context {
        interp: interp.raw(),
        calls: std::cell::Cell::new(0),
        saw_original: std::cell::Cell::new(false),
    };
    // SAFETY: context outlives command registration, script execution and its
    // callback; the replacement deliberately carries no context pointer.
    unsafe {
        ffi::tcl_create_obj_command(
            interp.raw(),
            c"doomed".as_ptr(),
            made,
            std::ptr::from_ref(&context).cast_mut().cast(),
            Some(replace_at_retirement),
        );
    }
    interp.sync().unwrap();
    let result = interp.eval("rename doomed {}; doomed SURVIVED").unwrap();
    assert_eq!(result.as_str(), Some("SURVIVED"));
    assert_eq!(context.calls.get(), 1);
    assert!(context.saw_original.get());
    assert!(
        interp
            .commands()
            .iter()
            .any(|name| name.as_bytes() == b"doomed")
    );
}

unsafe extern "C" fn original_getter(
    client_data: *mut c_void,
    interp: *mut InterpState,
    count: c_int,
    words: *const *mut Obj,
) -> c_int {
    if count != 3 {
        return ffi::TCL_ERROR;
    }
    // SAFETY: native invocation owns both live argument references throughout.
    unsafe {
        let first = *words.add(1);
        let second = *words.add(2);
        if first != second || ffi::tcl_is_shared(first) == 0 {
            return ffi::TCL_ERROR;
        }
        let code = if client_data.is_null() {
            let mut value = 0_i64;
            ffi::tcl_get_wide_int_from_obj(interp, first, &raw mut value)
        } else {
            let mut value = 0;
            ffi::tcl_get_int_from_obj(interp, first, &raw mut value)
        };
        ffi::tcl_set_obj_result(interp, ffi::tcl_new_int_obj(code));
        // The test observes failure-cache publication while the C caller handles
        // the primitive error itself, independently of a script catch boundary.
        ffi::TCL_OK
    }
}

unsafe extern "C" fn original_getter_init(interp: *mut InterpState) -> c_int {
    // SAFETY: installation receives a live shim interpreter.
    unsafe {
        ffi::tcl_create_obj_command(
            interp,
            c"original_wide".as_ptr(),
            original_getter,
            std::ptr::null_mut(),
            None,
        );
        ffi::tcl_create_obj_command(
            interp,
            c"original_int".as_ptr(),
            original_getter,
            std::ptr::dangling_mut::<c_void>(),
            None,
        );
    }
    ffi::TCL_OK
}

#[test]
fn callback_getters_preserve_aliases_and_publish_original_failure_caches() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: the initializer exclusively uses the shim ABI.
    unsafe { interp.load_static(original_getter_init) }.unwrap();
    interp.eval("set ::original 0x10").unwrap();
    let original = interp.engine_mut().vm_mut().get_var("::original").unwrap();
    let identity = original.native_object_identity();
    interp
        .eval("original_wide $::original $::original")
        .unwrap();
    assert_eq!(original.native_object_identity(), identity);
    assert!(matches!(
        original.native_scalar_cache(),
        Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Int(16)
        ))
    ));
    assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"0x10");

    interp.eval("set ::original 18446744073709551616").unwrap();
    let original = interp.engine_mut().vm_mut().get_var("::original").unwrap();
    let answer = interp.eval("original_int $::original $::original").unwrap();
    assert_eq!(answer.as_str(), Some("1"));
    assert!(matches!(
        original.native_scalar_cache(),
        Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Big { .. }
        ))
    ));
    assert_eq!(
        original.resident_string_bytes().unwrap().as_ref(),
        b"18446744073709551616"
    );
}

unsafe extern "C" fn original_member(
    _client_data: *mut c_void,
    interp: *mut InterpState,
    count: c_int,
    words: *const *mut Obj,
) -> c_int {
    if count != 2 {
        return ffi::TCL_ERROR;
    }
    // SAFETY: the callback receives one live original List and writable locals.
    unsafe {
        let mut count = 0;
        let mut members = std::ptr::null_mut();
        let code =
            ffi::tcl_list_obj_get_elements(interp, *words.add(1), &raw mut count, &raw mut members);
        if code != ffi::TCL_OK {
            return code;
        }
        if count != 2 || *members != *members.add(1) {
            return ffi::TCL_ERROR;
        }
        let mut number = 0;
        let code = ffi::tcl_get_wide_int_from_obj(interp, *members, &raw mut number);
        if code != ffi::TCL_OK {
            return code;
        }
        ffi::tcl_set_obj_result(interp, *members.add(1));
    }
    ffi::TCL_OK
}

unsafe extern "C" fn original_member_init(interp: *mut InterpState) -> c_int {
    // SAFETY: callback and interpreter obey the shim C ABI.
    unsafe {
        ffi::tcl_create_obj_command(
            interp,
            c"original_member".as_ptr(),
            original_member,
            std::ptr::null_mut(),
            None,
        );
    }
    ffi::TCL_OK
}

#[test]
fn original_string_cache_keeps_unicode_and_allocation_until_reached_conversion() {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    use tcl_syntax::native_string::NativeStringProtocol;
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: the initializer exclusively registers the static callbacks above.
    unsafe { interp.load_static(original_getter_init) }.unwrap();
    interp.eval("set ::original 0x10").unwrap();
    let original = interp.engine_mut().vm_mut().get_var("::original").unwrap();
    let protocol = NativeStringProtocol::C(tcl_dialect::TclVersion::V9_0);
    let units = original.native_unicode_units(protocol).unwrap();
    assert_eq!(units.as_ref(), &[48, 120, 49, 48]);
    let resident = original.resident_string_bytes().unwrap();
    interp
        .eval("original_wide $::original $::original")
        .unwrap();
    assert!(std::rc::Rc::ptr_eq(
        &resident,
        &original.resident_string_bytes().unwrap()
    ));
    assert!(matches!(
        original.native_scalar_cache(),
        Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Int(16)
        ))
    ));
    let cache = Cache::String {
        protocol,
        num_chars: Some(2),
        unicode: Some(std::rc::Rc::from([49, 50])),
    };
    let pure = tcl_vm::Value::from_native_string_cache(
        cache,
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0),
        None,
    )
    .unwrap();
    let identity = pure.native_object_identity();
    interp
        .engine_mut()
        .vm_mut()
        .set_var("::original", pure.clone())
        .unwrap();
    assert!(pure.resident_string_bytes().is_none());
    interp
        .eval("original_wide $::original $::original")
        .unwrap();
    assert_eq!(pure.native_object_identity(), identity);
    assert_eq!(pure.resident_string_bytes().unwrap().as_ref(), b"12");
    assert!(matches!(
        pure.native_scalar_cache(),
        Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Int(12)
        ))
    ));
}

#[test]
fn list_members_and_original_callback_results_keep_the_same_physical_objects() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: the initializer only registers the callback above.
    unsafe { interp.load_static(original_member_init) }.unwrap();
    interp
        .eval("set ::child 0x10; set ::original [list $::child $::child]; set initialized DONE")
        .unwrap();
    let child = interp.engine_mut().vm_mut().get_var("::child").unwrap();
    let parent = interp.engine_mut().vm_mut().get_var("::original").unwrap();
    assert!(parent.resident_string_bytes().is_none());
    interp
        .eval("set ::returned [original_member $::original]")
        .unwrap();
    let returned = interp.engine_mut().vm_mut().get_var("::returned").unwrap();
    assert_eq!(
        returned.native_object_identity(),
        child.native_object_identity()
    );
    let (members, _) = parent.cached_list_representation().unwrap();
    assert!(
        members
            .iter()
            .all(|value| value.native_object_identity() == child.native_object_identity())
    );
    assert!(matches!(
        child.native_scalar_cache(),
        Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
            tcl_syntax::number::Number::Int(16)
        ))
    ));
    assert!(
        parent.resident_string_bytes().is_none(),
        "child conversion does not materialize the parent"
    );
}

#[derive(Default)]
struct RetainedOriginal {
    object: *mut Obj,
    string: *mut std::ffi::c_char,
}

unsafe extern "C" fn retain_original(
    data: *mut c_void,
    interp: *mut InterpState,
    count: c_int,
    words: *const *mut Obj,
) -> c_int {
    if count != 2 {
        return ffi::TCL_ERROR;
    }
    // SAFETY: client data lives until its registered delete callback; argv lives
    // for the call, and retaining the object takes an explicit C reference.
    unsafe {
        let state = &mut *data.cast::<RetainedOriginal>();
        let object = *words.add(1);
        if state.object.is_null() {
            ffi::tcl_incr_ref_count(object);
            state.object = object;
            state.string = ffi::tcl_get_string(object);
        } else if state.object != object
            || state.string != ffi::tcl_get_string(object)
            || (&*object).to_value().as_int() != Some(16)
        {
            return ffi::TCL_ERROR;
        }
        ffi::tcl_set_obj_result(interp, object);
    }
    ffi::TCL_OK
}

unsafe extern "C" fn release_original(data: *mut c_void) {
    // SAFETY: this owns the registered allocation and its one retained reference.
    unsafe {
        let state = Box::from_raw(data.cast::<RetainedOriginal>());
        if !state.object.is_null() {
            ffi::tcl_decr_ref_count(state.object);
        }
    }
}

unsafe extern "C" fn retain_original_init(interp: *mut InterpState) -> c_int {
    let data = Box::into_raw(Box::<RetainedOriginal>::default()).cast::<c_void>();
    // SAFETY: native state owns this client allocation through its delete callback.
    unsafe {
        ffi::tcl_create_obj_command(
            interp,
            c"retain_original".as_ptr(),
            retain_original,
            data,
            Some(release_original),
        );
    }
    ffi::TCL_OK
}

#[test]
fn retained_native_objects_refresh_cache_without_changing_identity_or_resident_pointer() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: initialization and delete callback own all retained native references.
    unsafe { interp.load_static(retain_original_init) }.unwrap();
    interp
        .eval("set ::original 0x10; retain_original $::original")
        .unwrap();
    let original = interp.engine_mut().vm_mut().get_var("::original").unwrap();
    let dialect = interp.engine_mut().vm_mut().native_scalar_carrier_dialect();
    original
        .native_scalar_probe(
            dialect,
            tcl_syntax::scalar_getter::NativeScalarGetterKind::Wide,
        )
        .unwrap()
        .unwrap();
    interp.eval("retain_original $::original").unwrap();
}

unsafe extern "C" fn native_completion(
    mode: *mut c_void,
    interp: *mut InterpState,
    _count: c_int,
    _words: *const *mut Obj,
) -> c_int {
    let bits = mode.addr();
    // SAFETY: each new object belongs to this live invocation. The List takes
    // two references to the SAME child before the result takes the List.
    unsafe {
        let child = ffi::tcl_new_string_obj(b"v\0\xff".as_ptr().cast(), 3);
        let members = [child, child];
        let result = ffi::tcl_new_list_obj(2, members.as_ptr());
        ffi::tcl_set_obj_result(interp, result);
        if bits & 256 != 0 {
            ffi::tcl_set_obj_error_code(interp, ffi::tcl_new_string_obj(c"CUSTOM E".as_ptr(), -1));
        }
    }
    c_int::try_from(bits & 255).expect("native completion code")
}

unsafe extern "C" fn native_completion_init(interp: *mut InterpState) -> c_int {
    for (bits, name) in [
        (256, c"native_code_0"),
        (257, c"native_code_1"),
        (258, c"native_code_2"),
        (259, c"native_code_3"),
        (260, c"native_code_4"),
        (263, c"native_code_7"),
        (0, c"native_plain_0"),
        (1, c"native_plain_1"),
        (2, c"native_plain_2"),
        (3, c"native_plain_3"),
        (4, c"native_plain_4"),
        (7, c"native_plain_7"),
    ] {
        // SAFETY: the code bits are opaque client metadata, never dereferenced.
        unsafe {
            ffi::tcl_create_obj_command(
                interp,
                name.as_ptr(),
                native_completion,
                std::ptr::without_provenance_mut(bits),
                None,
            );
        }
    }
    ffi::TCL_OK
}

#[test]
fn native_completion_preserves_raw_codes_options_and_new_object_aliases() {
    let mut interp = Interp::new(TclVmEngine::new());
    // SAFETY: the initializer only registers linked static callbacks.
    unsafe { interp.load_static(native_completion_init) }.unwrap();
    for seeded in [false, true] {
        for code in [0, 1, 2, 3, 4, 7] {
            let completion = interp
                .engine_mut()
                .vm_mut()
                .try_invoke_command(
                    &format!("native_{}_{code}", if seeded { "code" } else { "plain" }),
                    &[],
                )
                .unwrap();
            assert_eq!(completion.code, tcl_vm::Code::from_int(code));
            let (members, _) = completion.result.cached_list_representation().unwrap();
            assert_eq!(members.len(), 2);
            assert_eq!(
                members[0].native_object_identity(),
                members[1].native_object_identity()
            );
            assert_eq!(
                members[0].resident_string_bytes().unwrap().as_ref(),
                b"v\0\xff"
            );
            let options = completion
                .options
                .cached_dictionary_representation()
                .unwrap();
            let field = |name: &[u8]| {
                options
                    .iter()
                    .find(|(key, _)| {
                        key.resident_string_bytes()
                            .is_some_and(|bytes| bytes.as_ref() == name)
                    })
                    .map(|(_, value)| value)
                    .unwrap()
            };
            assert_eq!(
                field(b"-code").native_scalar_cache(),
                Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                    tcl_syntax::number::Number::Int(if code == 2 { 0 } else { i64::from(code) })
                ))
            );
            assert_eq!(
                field(b"-level").native_scalar_cache(),
                Some(tcl_syntax::scalar_getter::NativeScalarCache::Number(
                    tcl_syntax::number::Number::Int(i64::from(code == 2))
                ))
            );
            if seeded || code == 1 {
                assert_eq!(
                    field(b"-errorcode")
                        .resident_string_bytes()
                        .unwrap()
                        .as_ref(),
                    if seeded {
                        b"CUSTOM E".as_slice()
                    } else {
                        b"NONE".as_slice()
                    }
                );
            } else {
                assert!(!options.iter().any(|(key, _)| {
                    key.resident_string_bytes()
                        .is_some_and(|bytes| bytes.as_ref() == b"-errorcode")
                }));
            }
            if code == 1 {
                assert_eq!(
                    field(b"-errorinfo").native_object_identity(),
                    completion.result.native_object_identity()
                );
                assert!(
                    field(b"-errorstack")
                        .resident_string_bytes()
                        .unwrap()
                        .is_empty()
                );
            } else {
                assert!(!options.iter().any(|(key, _)| {
                    key.resident_string_bytes()
                        .is_some_and(|bytes| bytes.as_ref() == b"-errorinfo")
                }));
            }
        }
    }
}

#[test]
fn zero_argument_callback_refuses_a_foreign_native_issuer_before_native_execution() {
    let mut engine = TclVmEngine::new();
    engine.vm_mut().set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
    );
    let mut interp = Interp::new(engine);
    // SAFETY: all linked procedures and their opaque code metadata are static.
    unsafe { interp.load_static(native_completion_init) }.unwrap();
    // SAFETY: the shim state stays alive throughout this test.
    let original_result = unsafe { ffi::tcl_get_obj_result(interp.raw()) };
    assert!(
        interp
            .engine_mut()
            .vm_mut()
            .try_invoke_command("native_code_0", &[])
            .is_err()
    );
    // The callback would replace this result with its new List if it ran.
    assert_eq!(
        unsafe { ffi::tcl_get_obj_result(interp.raw()) },
        original_result
    );
}
