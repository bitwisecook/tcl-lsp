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

thread_local! {
    static NAME_FREE_MODE: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    static NAME_FREE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static NAME_FREE_BORROWS_RELEASED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    static NAME_FREE_CONTEXT_ACCEPTED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    static NAME_CALLBACK_BOUND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static NAME_CALLBACK_TARGET_BORN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static NAME_CALLBACK_CODE: std::cell::Cell<Code> = const { std::cell::Cell::new(Code::Other(37)) };
}

struct CurrentNameCallbackInterp(*mut Interp);
impl CurrentNameCallbackInterp {
    fn enter(interp: &mut Interp) -> Self {
        let previous = crate::codegen_abi::current_interp();
        crate::codegen_abi::tcl_runtime_set_current_interp(interp);
        Self(previous)
    }
}
impl Drop for CurrentNameCallbackInterp {
    fn drop(&mut self) {
        crate::codegen_abi::tcl_runtime_set_current_interp(self.0);
    }
}

extern "C" fn name_free_callback(_original: *mut TclObj) {
    let call = NAME_FREE_CALLS.with(|calls| {
        let call = calls.get() + 1;
        calls.set(call);
        call
    });
    let Some(mut interp) = (unsafe { crate::codegen_abi::current_interp().as_ref() }).cloned()
    else {
        NAME_FREE_CONTEXT_ACCEPTED.with(|accepted| accepted.set(false));
        return;
    };
    let released =
        interp.frames.try_borrow_mut().is_ok() && interp.namespaces.try_borrow_mut().is_ok();
    NAME_FREE_BORROWS_RELEASED.with(|previous| previous.set(previous.get() && released));
    match NAME_FREE_MODE.with(std::cell::Cell::get) {
        1 => {
            interp.refuse_host_command("first original variable-name free callback");
        }
        2 => {
            let original = interp.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("original-name-callback".to_owned(), "1.0".to_owned())];
            let accepted =
                interp.pin_context(&changed).is_ok() && interp.pin_context(&original).is_ok();
            NAME_FREE_CONTEXT_ACCEPTED.with(|value| value.set(value.get() && accepted));
        }
        3 if call == 2 => {
            interp.refuse_host_command("first canonical variable-name free callback");
        }
        _ => {}
    }
}
static NAME_FREE_TYPE: obj::TclObjType = obj::TclObjType {
    name: c"originalVariableNameFree".as_ptr(),
    free_int_rep_proc: Some(name_free_callback),
    dup_int_rep_proc: None,
    update_string_proc: None,
    set_from_any_proc: None,
};

fn callback_name(bytes: &[u8]) -> Owned {
    let original = Owned::fresh(obj::alloc_typed(&NAME_FREE_TYPE, 0));
    // SAFETY: the fresh original header owns this exact resident spelling.
    unsafe { obj::set_native_updater_string_rep(original.as_ptr(), bytes, false) };
    original
}

fn reset_name_callback(mode: u8) {
    NAME_FREE_MODE.with(|value| value.set(mode));
    NAME_FREE_CALLS.with(|value| value.set(0));
    NAME_FREE_BORROWS_RELEASED.with(|value| value.set(true));
    NAME_FREE_CONTEXT_ACCEPTED.with(|value| value.set(true));
    NAME_CALLBACK_BOUND.with(|value| value.set(false));
    NAME_CALLBACK_TARGET_BORN.with(|value| value.set(false));
    NAME_CALLBACK_CODE.with(|value| value.set(Code::Other(37)));
}

#[cfg(debug_assertions)]
fn direct_name_callback_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let command = Owned::fresh(obj::new_string_bytes(b"upvar"));
    let level = Owned::fresh(obj::new_string_bytes(b"#0"));
    let local = Owned::fresh(obj::new_string_bytes(b"callback_local"));
    let code = interp.dispatch(&[command.as_ptr(), level.as_ptr(), argv[1], local.as_ptr()]);
    let target_born = interp
        .namespaces
        .borrow()
        .var_table(crate::namespace::GLOBAL)
        .trace_binding_identity(b"callback_target")
        .is_some();
    NAME_CALLBACK_TARGET_BORN.with(|value| value.set(target_born));
    let bound = interp
        .frames
        .borrow()
        .table(interp.current_level())
        .unwrap()
        .binding_id(b"callback_local")
        .is_some();
    NAME_CALLBACK_BOUND.with(|value| value.set(bound));
    NAME_CALLBACK_CODE.with(|value| value.set(code));
    code
}

#[cfg(debug_assertions)]
fn compiled_name_callback_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let slot = {
        let frames = interp.frames.borrow();
        frames
            .native_local_name_table()
            .expect("actual retained procedure name table")
            .names
            .iter()
            .position(|name| {
                name.as_ref()
                    .is_some_and(|name| obj::bytes_of(name.as_ptr()) == b"callback_local")
            })
            .expect("actual compiled local slot")
    };
    assert!(interp.frames.borrow().compiled_slot_var(slot).is_none());
    let level = Owned::fresh(obj::new_string_bytes(b"#0"));
    let (_, target) = match crate::cmd_eval::select_frame(
        interp,
        &[level.as_ptr()],
        tcl_registry::FrameEffectSpec::UPVAR,
    ) {
        Ok(selected) => selected,
        Err(code) => return code,
    };
    let code = interp.link_original_compiled_upvar(argv[1], target, slot);
    let bound = interp.frames.borrow().compiled_slot_var(slot).is_some();
    NAME_CALLBACK_BOUND.with(|value| value.set(bound));
    NAME_CALLBACK_CODE.with(|value| value.set(code));
    code
}

#[cfg(debug_assertions)]
#[test]
fn original_upvar_free_callback_checks_currency_before_cells_and_aliases() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual selected C worker; software callback currency, not native equivalence.
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..=2 {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"callbackUpvar", direct_name_callback_probe);
            assert_eq!(
                interp.eval_str(b"proc callback_run {other} {callbackUpvar $other}"),
                Code::Ok
            );
            let original = callback_name(b"::callback_target");
            let command = Owned::fresh(obj::new_string_bytes(b"callback_run"));
            reset_name_callback(mode);
            let _installed = CurrentNameCallbackInterp::enter(&mut interp);
            let code = interp.dispatch(&[command.as_ptr(), original.as_ptr()]);
            assert_eq!(
                NAME_FREE_CALLS.with(std::cell::Cell::get),
                1,
                "{profile}/{mode}"
            );
            assert!(NAME_FREE_BORROWS_RELEASED.with(std::cell::Cell::get));
            assert!(NAME_FREE_CONTEXT_ACCEPTED.with(std::cell::Cell::get));
            assert_eq!(obj::bytes_of(original.as_ptr()), b"::callback_target");
            assert_eq!(interp.current_level(), 0);
            assert_eq!(interp.current_ns.get(), crate::namespace::GLOBAL);
            assert_eq!(interp.entered_native_operation_count(), 0);
            if mode == 0 {
                assert_eq!(code, Code::Ok, "{profile}");
                assert_eq!(NAME_CALLBACK_CODE.with(std::cell::Cell::get), Code::Ok);
                assert!(NAME_CALLBACK_BOUND.with(std::cell::Cell::get));
                assert!(!interp.host_refusal_pending());
            } else {
                assert_eq!(code, Code::Error, "{profile}/{mode}");
                assert_eq!(NAME_CALLBACK_CODE.with(std::cell::Cell::get), Code::Error);
                assert!(!NAME_CALLBACK_BOUND.with(std::cell::Cell::get));
                assert!(interp.native_execution_refusal().is_some());
            }
            // Frame pop can reclaim the undefined target after its local alias.
            let target_born = NAME_CALLBACK_TARGET_BORN.with(std::cell::Cell::get);
            assert_eq!(
                target_born,
                mode == 0 || profile.starts_with("tcl9"),
                "C9 cell birth precedes the reached cache callback; C8 retirement precedes lookup"
            );
        }
    }
}

#[cfg(debug_assertions)]
#[test]
fn original_compiled_upvar_free_callback_refuses_later_slot_binding() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Genuine procedure artifact/slot and shared compiled worker, no codegen claim.
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in 0..=2 {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(profile),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"callbackCompiledUpvar", compiled_name_callback_probe);
            assert_eq!(interp.eval_str(b"proc callback_run {other} {set callback_local {}; unset callback_local; callbackCompiledUpvar $other}"), Code::Ok);
            let procedure = interp.proc_def(b"callback_run").unwrap();
            let artifact = interp
                .prepare_original_c_body(
                    procedure.body.as_ptr(),
                    procedure.namespace(),
                    Some(&procedure),
                )
                .unwrap()
                .expect("actual native procedure artifact");
            assert!(artifact
                .compiled_local_layout()
                .unwrap()
                .names
                .iter()
                .any(|name| {
                    name.as_ref()
                        .is_some_and(|name| name.as_bytes() == b"callback_local")
                }));
            let original = callback_name(b"::callback_target");
            let command = Owned::fresh(obj::new_string_bytes(b"callback_run"));
            reset_name_callback(mode);
            let _installed = CurrentNameCallbackInterp::enter(&mut interp);
            let code = interp.dispatch(&[command.as_ptr(), original.as_ptr()]);
            assert_eq!(
                NAME_FREE_CALLS.with(std::cell::Cell::get),
                1,
                "{profile}/{mode}"
            );
            assert!(NAME_FREE_BORROWS_RELEASED.with(std::cell::Cell::get));
            assert!(NAME_FREE_CONTEXT_ACCEPTED.with(std::cell::Cell::get));
            assert_eq!(interp.current_level(), 0);
            assert_eq!(interp.current_ns.get(), crate::namespace::GLOBAL);
            assert_eq!(interp.entered_native_operation_count(), 0);
            if mode == 0 {
                assert_eq!(code, Code::Ok, "{profile}");
                assert_eq!(NAME_CALLBACK_CODE.with(std::cell::Cell::get), Code::Ok);
                assert!(NAME_CALLBACK_BOUND.with(std::cell::Cell::get));
            } else {
                assert_eq!(code, Code::Error, "{profile}/{mode}");
                assert_eq!(NAME_CALLBACK_CODE.with(std::cell::Cell::get), Code::Error);
                assert!(!NAME_CALLBACK_BOUND.with(std::cell::Cell::get));
                assert!(interp.native_execution_refusal().is_some());
            }
        }
    }
}

fn canonical_name_callback_probe(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let (slot, canonical) = {
        let frames = interp.frames.borrow();
        let table = frames.native_local_name_table().unwrap();
        let slot = table
            .names
            .iter()
            .position(|name| {
                name.as_ref()
                    .is_some_and(|name| obj::bytes_of(name.as_ptr()) == b"value")
            })
            .unwrap();
        (slot, table.names[slot].as_ref().unwrap().as_ptr())
    };
    // The genuine ordinal/cell and inherited operation are captured before
    // either original primary can reach its free callback.
    let operation = super::super::native_operation_currency::NativeOperationScope::enter(interp)
        .expect("original canonical-name operation");
    let selected = crate::vars::prepare_original_c_indexed_name_cell(
        &mut interp.frames.borrow_mut(),
        &interp.namespaces.borrow(),
        slot,
    )
    .expect("actual selected canonical-name cell");
    obj::change_type(canonical, &NAME_FREE_TYPE, 0);
    let original = callback_name(b"value");
    reset_name_callback(3);
    let result = interp.install_original_c_name_cache_using(
        original.as_ptr(),
        &selected,
        operation.currency(),
        None,
    );
    assert!(result.is_err());
    assert!(obj::native_variable_name::with_local(original.as_ptr(), |_| ()).is_some());
    assert!(obj::obj_type_ptr(canonical).is_null());
    assert_eq!(obj::bytes_of(canonical), b"value");
    match result {
        Ok(()) => Code::Ok,
        Err(error) => interp.report_cmd_error(error),
    }
}

#[test]
fn original_canonical_name_free_callback_releases_borrows_and_stops_next_cache() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Reached original cache stays installed; the later canonical cache is refused.
    let mut interp = Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect("tcl9.1"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    interp.register_builtin(b"callbackCanonical", canonical_name_callback_probe);
    assert_eq!(
        interp.eval_str(b"proc callback_run {value} {callbackCanonical}"),
        Code::Ok
    );
    let _installed = CurrentNameCallbackInterp::enter(&mut interp);
    assert_eq!(interp.eval_str(b"callback_run VALUE"), Code::Error);
    assert_eq!(NAME_FREE_CALLS.with(std::cell::Cell::get), 2);
    assert!(NAME_FREE_BORROWS_RELEASED.with(std::cell::Cell::get));
    assert!(interp.native_execution_refusal().is_some());
    assert_eq!(interp.current_level(), 0);
    assert_eq!(interp.entered_native_operation_count(), 0);
}

fn exists_cache_purpose_probe(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    let version = interp.native_c_variable_name_protocol().unwrap().version();
    for name in [b"x".as_slice(), b"a(k)", b"x\0tail"] {
        let original = Owned::fresh(obj::new_string_bytes(name));
        let before = obj::obj_type_ptr(original.as_ptr());
        assert!(interp
            .exists_original_c_parts(original.as_ptr(), None)
            .unwrap());
        if version == tcl_dialect::TclVersion::V8_4 {
            assert_eq!(obj::obj_type_ptr(original.as_ptr()), before);
        } else {
            assert_ne!(obj::obj_type_ptr(original.as_ptr()), before);
        }
    }
    let member = Owned::fresh(obj::new_string_bytes(b"x"));
    let original = Owned::fresh(crate::list::new_list_obj(&[member.as_ptr()]));
    let before = obj::obj_type_ptr(original.as_ptr());
    assert!(interp
        .exists_original_c_parts(original.as_ptr(), None)
        .unwrap());
    if version == tcl_dialect::TclVersion::V8_4 {
        assert_eq!(obj::obj_type_ptr(original.as_ptr()), before);
        assert!(interp.read_original_c_variable(original.as_ptr()).is_ok());
        assert_ne!(
            obj::obj_type_ptr(original.as_ptr()),
            before,
            "actual object read still selects its original name cache"
        );
    } else {
        assert_ne!(obj::obj_type_ptr(original.as_ptr()), before);
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

#[test]
fn legacy_existence_preserves_original_name_caches_and_object_reads_still_adopt_them() {
    // naming.variable.original-upvar-and-exists-completion-and-name-windows
    // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
    // Native case16 binds the fresh scalar primary. The list-cache,
    // combined name and NUL controls separately test this source/API owner.
    for engine in ["tcl8.4", "tcl8.5"] {
        let mut interp = Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.register_builtin(b"existsCachePurpose", exists_cache_purpose_probe);
        let code =
            interp.eval_str(b"proc run {} {set x LOCAL; set a(k) VALUE; existsCachePurpose}; run");
        assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
        assert!(!interp.host_refusal_pending());
    }
}

thread_local! {
    static ALIAS_SIMPLE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn alias_simple_probe(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    for (case, name) in [b"v".as_slice(), b"d", b"n\0(k)"].into_iter().enumerate() {
        let local = if case == 2 {
            Owned::fresh(obj::new_string_bytes(name))
        } else {
            let member = Owned::fresh(obj::new_string_bytes(name));
            Owned::fresh(crate::list::new_list_obj(&[member.as_ptr()]))
        };
        let kind = if case == 2 { "NULL" } else { "list" };
        // SAFETY: local retains the original object throughout this callback.
        let references = unsafe { (*local.as_ptr()).ref_count };
        ALIAS_SIMPLE_ROWS.with(|rows| {
            rows.borrow_mut()
                .push(format!("before{case}|{kind}|{references}"))
        });
        let original_type = obj::obj_type_ptr(local.as_ptr());
        let target = Owned::fresh(obj::new_string_bytes(b"x"));
        let mut link = match interp.prepare_original_c_link_target(target.as_ptr(), 0) {
            Ok(Some(link)) => link,
            Ok(None) => panic!("actual C target unavailable"),
            Err(code) => return code,
        };
        interp.prepare_upvar_target(&mut link).unwrap();
        let code = interp.bind_original_c_alias_local(local.as_ptr(), link);
        if code != Code::Ok {
            return code;
        }
        assert_eq!(obj::obj_type_ptr(local.as_ptr()), original_type);
        // SAFETY: local retains the original object after alias installation.
        let references = unsafe { (*local.as_ptr()).ref_count };
        ALIAS_SIMPLE_ROWS.with(|rows| {
            rows.borrow_mut()
                .push(format!("after{case}|0|{kind}|{references}"))
        });
    }
    interp.set_result_bytes(b"");
    Code::Ok
}

thread_local! {
    static GLOBAL_CACHE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn global_cache_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let original = argv[1];
    let value = match interp.read_original_named_variable(original) {
        Ok(value) => value,
        Err(code) => return code,
    };
    let kind = obj::obj_type_ptr(original);
    assert!(
        !kind.is_null(),
        "successful original-name lookup owns a cache"
    );
    // SAFETY: original owns its selected descriptor throughout this callback.
    let name = unsafe { std::ffi::CStr::from_ptr((*kind).name).to_str().unwrap() };
    let bytes = interp.native_string_bytes(&value).unwrap();
    GLOBAL_CACHE_ROWS.with(|rows| {
        rows.borrow_mut().push(format!(
            "{name}|{}",
            String::from_utf8(bytes.to_vec()).unwrap(),
        ))
    });
    interp.set_result_bytes(b"");
    Code::Ok
}
#[test]
fn dynamic_global_uses_original_compiler_token_tail_and_name_cache() {
    // Native proof: naming.variable.original-global-cache-token
    // docs/design/analysis/name-resolution-proofs/variable-original-global-cache-token.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_str!("../../../tests/data/native_global_cache_token/8.4.20.txt"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tests/data/native_global_cache_token/8.5.19.txt"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tests/data/native_global_cache_token/8.6.18.txt"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tests/data/native_global_cache_token/9.0.4.txt"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tests/data/native_global_cache_token/9.1.0.txt"),
        ),
    ] {
        crate::counters::reset();
        GLOBAL_CACHE_ROWS.with(|rows| rows.borrow_mut().clear());
        {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"probe", global_cache_probe);
            let original = Owned::fresh(obj::new_string_bytes(b"v"));
            interp.var_set(b"name", original.as_ptr()).unwrap();
            drop(original);
            for (label, source) in [
                    ("simple", b"set v GLOBAL;proc p {} {set v LOCAL;probe $::name;unset v;global $::name;probe $::name;return $v};p".as_slice()),
                    ("qualified", b"namespace eval N {variable v QUALIFIED};set name ::N::v;p".as_slice()),
                ] {
                    let code = interp.eval_str(source);
                    GLOBAL_CACHE_ROWS.with(|rows| rows.borrow_mut().push(format!(
                        "{label}|{}|{}", code.as_int(), String::from_utf8(interp.result_bytes()).unwrap(),
                    )));
                    assert!(!interp.host_refusal_pending(), "{engine}");
                }
            let rows = GLOBAL_CACHE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
            assert_eq!(rows, expected, "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

#[test]
fn alias_local_simple_lookup_matches_all_15_native_primary_and_key_owners() {
    // Native proof: naming.alias.original-local-name-object
    // docs/design/analysis/name-resolution-proofs/alias-original-local-name-object.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_str!("../../../tests/data/native_alias_simple/8.4.20.txt"),
        ),
        (
            "tcl8.5",
            include_str!("../../../tests/data/native_alias_simple/8.5.19.txt"),
        ),
        (
            "tcl8.6",
            include_str!("../../../tests/data/native_alias_simple/8.6.18.txt"),
        ),
        (
            "tcl9.0",
            include_str!("../../../tests/data/native_alias_simple/9.0.4.txt"),
        ),
        (
            "tcl9.1",
            include_str!("../../../tests/data/native_alias_simple/9.1.0.txt"),
        ),
    ] {
        crate::counters::reset();
        ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().clear());
        {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"alias_probe", alias_simple_probe);
            let code = interp.eval_str(b"set x X; proc p {} {alias_probe; return $v}; p");
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert_eq!(interp.result_bytes(), b"X");
            ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow_mut().push("completion|0|X".into()));
            let rows = ALIAS_SIMPLE_ROWS.with(|rows| rows.borrow().join("\n") + "\n");
            assert_eq!(rows, expected, "{engine}");
            assert!(!interp.host_refusal_pending());
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

struct NamespaceOutputHost {
    inner: Rc<dyn tcl_platform::Host>,
    output: std::cell::RefCell<Vec<u8>>,
}
impl tcl_platform::StdIo for NamespaceOutputHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.output.borrow_mut().extend_from_slice(bytes);
    }
    fn write_stderr(&self, bytes: &[u8]) {
        self.inner.stdio().write_stderr(bytes);
    }
}
impl tcl_platform::Host for NamespaceOutputHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.inner.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.inner.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.inner.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.inner.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.inner.native_integer_formatter()
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.inner.system_encoding()
    }
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.inner.filesystem()
    }
    fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
        self.inner.sockets()
    }
    fn process(&self) -> Option<&dyn tcl_platform::Process> {
        self.inner.process()
    }
}

fn trace_source_observation(engine: &str, source: &[u8], expected: &[u8]) {
    crate::counters::reset();
    {
        let host = Rc::new(NamespaceOutputHost {
            inner: super::super::default_host(),
            output: std::cell::RefCell::new(Vec::new()),
        });
        let mut interp = Interp::with_native_core(
            host.clone(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let code = interp.eval_str(source);
        assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
        assert!(!interp.host_refusal_pending(), "{engine}");
        let actual = host.output.borrow();
        // The version identifies the native capture, while this control
        // compares trace setup and callback observations after that line.
        let after_version = |bytes: &[u8]| {
            let end = bytes.iter().position(|byte| *byte == b'\n').unwrap();
            bytes[end + 1..].to_vec()
        };
        assert_eq!(after_version(&actual), after_version(expected), "{engine}");
    }
    assert_eq!(crate::counters::finalize(), 0, "{engine}");
    assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
}

fn jim_trace_setup_observation(source: &[u8], expected: &[u8]) {
    // Retain the original probe, but stop at its recorded SETUP frontier
    // before the unrelated binary reporter, which this backend lacks.
    let source = std::str::from_utf8(source).unwrap();
    let (prefix, _) = source
        .split_once("binary scan $probeSetupResult H* probeSetupHex\n")
        .unwrap();
    let captured = std::str::from_utf8(expected).unwrap();
    let setup = captured
        .lines()
        .find(|line| line.starts_with("SETUP "))
        .unwrap();
    let hex = setup.strip_prefix("SETUP 1 ").unwrap();
    let expected_message: Vec<_> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert!(!captured
        .lines()
        .any(|line| line.starts_with("OBSERVATION ")));
    crate::counters::reset();
    {
        let mut interp = Interp::with_native_core(
            super::super::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(prefix.as_bytes()), Code::Ok);
        assert!(!interp.host_refusal_pending());
        for (name, expected) in [
            (b"probeSetupCode".as_slice(), b"1".as_slice()),
            (b"probeSetupResult".as_slice(), expected_message.as_slice()),
        ] {
            let value = interp.var_get(name).unwrap();
            assert_eq!(
                interp.native_string_bytes(&value).unwrap().as_ref(),
                expected
            );
        }
    }
    assert_eq!(crate::counters::finalize(), 0);
    assert_eq!(crate::counters::double_free_count(), 0);
}

#[test]
fn active_read_unset_retires_old_registrations_matches_five_native_source_controls() {
    // Native proof: naming.variable.unset-active-read-callback-retirement
    // docs/design/analysis/name-resolution-proofs/variable-unset-active-read-callback-retirement.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../tests/data/native_active_read_unset/8.4.20.txt").as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../tests/data/native_active_read_unset/8.5.19.txt").as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../tests/data/native_active_read_unset/8.6.18.txt").as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../tests/data/native_active_read_unset/9.0.4.txt").as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../tests/data/native_active_read_unset/9.1.0.txt").as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../tests/data/native_active_read_unset/source.tcl"),
            expected,
        );
    }
}

#[test]
fn active_read_unset_probe_preserves_actual_jim_unsupported_setup() {
    // Native proof: naming.variable.unset-active-read-callback-retirement
    // docs/design/analysis/name-resolution-proofs/variable-unset-active-read-callback-retirement.md
    // Jim's recorded setup failure supplies no read/unset callback answer.
    jim_trace_setup_observation(
        include_bytes!("../../../tests/data/native_active_read_unset/source.tcl"),
        include_bytes!("../../../tests/data/native_active_read_unset/jim.txt"),
    );
}

#[test]
fn recreated_element_read_trace_matches_five_original_source_controls() {
    // Native proof: naming.variable.recreated-element-independent-read-trace
    // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../tests/data/native_recreated_element_trace/8.4.20.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../tests/data/native_recreated_element_trace/8.5.19.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../tests/data/native_recreated_element_trace/8.6.18.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../tests/data/native_recreated_element_trace/9.0.4.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../tests/data/native_recreated_element_trace/9.1.0.txt")
                .as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../tests/data/native_recreated_element_trace/source.tcl"),
            expected,
        );
    }
}

#[test]
fn recreated_element_trace_probe_preserves_actual_jim_unsupported_setup() {
    // Native proof: naming.variable.recreated-element-independent-read-trace
    // docs/design/analysis/name-resolution-proofs/variable-recreated-element-independent-read-trace.md
    // The captured Jim setup failure supplies no independent callback answer.
    jim_trace_setup_observation(
        include_bytes!("../../../tests/data/native_recreated_element_trace/source.tcl"),
        include_bytes!("../../../tests/data/native_recreated_element_trace/jim.txt"),
    );
}

#[test]
fn generic_namespace_declarations_match_all_25_native_execution_results() {
    let source = include_bytes!(
        "../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/source.tcl"
    );
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!(
                "../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl8.4.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!(
                "../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl8.5.txt"
            )
            .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!(
                "../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl8.6.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!(
                "../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl9.0.txt"
            )
            .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!(
                "../../../../../rust/tcl-vm/tests/data/native_namespace_handler_order/tcl9.1.txt"
            )
            .as_slice(),
        ),
    ] {
        crate::counters::reset();
        {
            let host = Rc::new(NamespaceOutputHost {
                inner: super::super::default_host(),
                output: std::cell::RefCell::new(Vec::new()),
            });
            let mut interp = Interp::with_native_core(
                host.clone(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let code = interp.eval_str(source);
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert!(!interp.host_refusal_pending(), "{engine}");
            assert_eq!(host.output.borrow().as_slice(), expected, "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

// Native proof: naming.variable.namespace-alias-settlement-after-store
// docs/design/analysis/name-resolution-proofs/variable.namespace-alias-settlement-after-store.md
#[test]
fn namespace_alias_settlement_matches_all_15_native_callback_and_error_results() {
    let source = include_bytes!("../../../tests/data/native_namespace_alias_settlement/source.tcl");
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../tests/data/native_namespace_alias_settlement/8.4.20.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../tests/data/native_namespace_alias_settlement/8.5.19.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../tests/data/native_namespace_alias_settlement/8.6.18.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../tests/data/native_namespace_alias_settlement/9.0.4.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../tests/data/native_namespace_alias_settlement/9.1.0.txt")
                .as_slice(),
        ),
    ] {
        crate::counters::reset();
        {
            let host = Rc::new(NamespaceOutputHost {
                inner: super::super::default_host(),
                output: std::cell::RefCell::new(Vec::new()),
            });
            let mut interp = Interp::with_native_core(
                host.clone(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let code = interp.eval_str(source);
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert!(!interp.host_refusal_pending(), "{engine}");
            assert_eq!(host.output.borrow().as_slice(), expected, "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

#[test]
fn qualified_variable_scope_matches_all_six_native_sequences() {
    let source = include_bytes!("../../../tests/data/native_variable_qualified_target/source.tcl");
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../tests/data/native_variable_qualified_target/tcl8.4.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../tests/data/native_variable_qualified_target/tcl8.5.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../tests/data/native_variable_qualified_target/tcl8.6.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../tests/data/native_variable_qualified_target/tcl9.0.txt")
                .as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../tests/data/native_variable_qualified_target/tcl9.1.txt")
                .as_slice(),
        ),
        (
            "jim",
            include_bytes!("../../../tests/data/native_variable_qualified_target/jim.txt")
                .as_slice(),
        ),
    ] {
        crate::counters::reset();
        {
            let host = Rc::new(NamespaceOutputHost {
                inner: super::super::default_host(),
                output: std::cell::RefCell::new(Vec::new()),
            });
            let mut interp = Interp::with_native_core(
                host.clone(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let code = interp.eval_str(source);
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert!(!interp.host_refusal_pending(), "{engine}");
            assert_eq!(host.output.borrow().as_slice(), expected, "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

fn namespace_cache_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let original = argv[1];
    assert_eq!(
        super::super::obj_bytes(interp.read_original_c_variable(original).unwrap()),
        b"LOCAL"
    );
    let local_slot = interp.original_c_local_index(original).unwrap();
    let before = interp
        .frames
        .borrow()
        .native_compiled_cell_identity(local_slot)
        .unwrap();
    let level = interp.current_level();
    let alias_slot = interp
        .frames
        .borrow()
        .native_compiled_name_index(level, b"alias")
        .unwrap();
    let code = interp.link_original_compiled_namespace_variable(
        original,
        crate::namespace::GLOBAL,
        alias_slot,
        true,
    );
    assert_eq!(code, Code::Ok, "{:?}", interp.result_bytes());
    assert!(obj::native_variable_name::with_local(original, |_| ()).is_none());
    assert!(
        obj::native_variable_name::with_parsed(original, |cache| cache.array.is_none()).unwrap()
    );
    assert_eq!(
        interp
            .frames
            .borrow()
            .native_compiled_cell_identity(local_slot),
        Some(before)
    );
    assert_eq!(
        super::super::obj_bytes(interp.var_get(b"v").unwrap()),
        b"LOCAL"
    );
    let selected = interp
        .prepare_original_c_name_in(
            original,
            NativeVariableNameLookupPurpose::Read,
            Some(crate::namespace::GLOBAL),
        )
        .unwrap()
        .unwrap();
    let (receiver, _) = interp
        .capture_original_c_selection(original, &selected, NativeVariableNameLookupPurpose::Read)
        .unwrap()
        .unwrap();
    assert_eq!(
        super::super::obj_bytes(receiver.read().unwrap().unwrap()),
        b"GLOBAL"
    );
    interp.set_result_bytes(b"");
    Code::Ok
}

#[test]
fn namespace_opcodes_bypass_original_local_cache_and_retain_the_namespace_cell() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        crate::counters::reset();
        {
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.register_builtin(b"namespace_cache_probe", namespace_cache_probe);
            let original = Owned::fresh(obj::new_string_bytes(b"v"));
            interp.var_set(b"name", original.as_ptr()).unwrap();
            let code = interp.eval_str(b"set v GLOBAL; proc p {} {set v LOCAL; namespace_cache_probe $::name; return $alias}; p");
            assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
            assert_eq!(interp.result_bytes(), b"GLOBAL", "{engine}");
            assert!(
                obj::native_variable_name::with_parsed(original.as_ptr(), |cache| cache
                    .array
                    .is_none())
                .unwrap()
            );
            assert!(!interp.host_refusal_pending(), "{engine}");
        }
        assert_eq!(crate::counters::finalize(), 0, "{engine}");
        assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
    }
}

thread_local! {
    static CACHE_LOOKUP_CASE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CACHE_LOOKUP_CALLBACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CACHE_LOOKUP_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}
struct CacheLookupObserver;
impl tcl_runtime_api::native_variable_trace::NativeVariableObserver<Interp>
    for CacheLookupObserver
{
    type Error = tcl_cmd_core::CmdError;
    fn observe(
        &self,
        _interp: &mut Interp,
        _access: tcl_runtime_api::native_variable_trace::NativeVariableTraceAccess<'_>,
    ) -> Result<(), Self::Error> {
        CACHE_LOOKUP_CALLBACKS.with(|count| count.set(count.get() + 1));
        Ok(())
    }
}
fn cache_lookup_probe(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    use tcl_runtime_api::native_variable_trace::NativeVariableTraceOperation as Op;
    let original = argv[1];
    let case = CACHE_LOOKUP_CASE.with(std::cell::Cell::get);
    let version = interp.native_c_variable_name_protocol().unwrap().version();
    interp.read_original_c_variable(original).unwrap();
    let before = interp
        .original_c_local_index(original)
        .expect("genuine local cache");
    let before_slot_cell = interp
        .frames
        .borrow()
        .native_compiled_cell_identity(before)
        .expect("actual installed compiled cell");
    let selection = interp.prepare_original_c_name(original, false).unwrap();
    let before_cell = interp
        .capture_original_c_selection(original, &selection, NativeVariableNameLookupPurpose::Read)
        .unwrap()
        .unwrap()
        .1
        .binding_id;
    if matches!(case, 2 | 3) {
        interp
            .add_native_variable_observer(
                original,
                &[Op::Read, Op::Write],
                Rc::new(CacheLookupObserver),
            )
            .unwrap();
    }
    if case == 4 {
        assert!(interp.var_unset(b"x"));
    }
    obj::invalidate_string(original);
    let result = if matches!(case, 1 | 3) {
        let next = Owned::fresh(obj::new_string_bytes(b"NEXT"));
        interp.store_original_c_variable(original, next.as_ptr())
    } else {
        interp.read_original_c_variable(original).map(|_| ())
    };
    assert_eq!(interp.original_c_local_index(original), Some(before));
    assert_eq!(
        interp.frames.borrow().native_compiled_cell_identity(before),
        Some(before_slot_cell),
        "same original cell after {version:?}/{case}"
    );
    // Unset withdraws the value, while the actual compiled Var allocation
    // and the original cache keep their identity through undefinedness.
    assert_eq!(interp.trace_identity(b"x").binding_id, before_cell);
    if case == 4 {
        let frames = interp.frames.borrow();
        assert!(frames
            .table(frames.current_level())
            .unwrap()
            .binding_id(b"x")
            .is_none());
    }
    if version >= tcl_dialect::TclVersion::V8_5 && case >= 2 {
        assert!(result.is_err());
        assert!(
            interp.host_refusal_pending(),
            "native updater abort must remain a host refusal"
        );
        assert!(!obj::has_string_rep(original));
        assert_eq!(CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get), 0);
    } else {
        assert_eq!(result.is_err(), case == 4);
        assert_eq!(
            obj::has_string_rep(original),
            version == tcl_dialect::TclVersion::V8_4
        );
        assert_eq!(
            CACHE_LOOKUP_CALLBACKS.with(std::cell::Cell::get),
            usize::from(matches!(case, 2 | 3))
        );
    }
    let version_label = match version {
        tcl_dialect::TclVersion::V8_4 => "8.4.20",
        tcl_dialect::TclVersion::V8_5 => "8.5.19",
        tcl_dialect::TclVersion::V8_6 => "8.6.18",
        tcl_dialect::TclVersion::V9_0 => "9.0.4",
        tcl_dialect::TclVersion::V9_1 => "9.1.0",
    };
    let row = if interp.host_refusal_pending() {
        format!("{version_label}|{case}|updater-unavailable")
    } else {
        format!(
            "{version_label}|{case}|{}|{}|localVarName|1",
            usize::from(result.is_err()),
            usize::from(obj::has_string_rep(original))
        )
    };
    CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().push(row));
    if case != 4 {
        // Inspect reached storage without entering another read after refusal.
        let frames = interp.frames.borrow();
        let stored = frames.compiled_slot_var(before).unwrap();
        let crate::frame::Var::Scalar(value) = &*stored else {
            panic!("actual compiled scalar contents");
        };
        assert!(obj::has_string_rep(*value));
        assert_eq!(
            obj::bytes_of(*value),
            if matches!(case, 1 | 3) {
                b"NEXT".as_slice()
            } else {
                b"VALUE".as_slice()
            }
        );
    }
    result.map_or_else(|code| code, |()| Code::Ok)
}

#[test]
fn original_local_cache_getter_order_matches_all_25_native_paths() {
    // Native proof: naming.variable.original-local-cache-before-getter
    // docs/design/analysis/name-resolution-proofs/variable-original-local-cache-before-getter.md
    // Native proof: naming.variable.original-local-cache-unavailable-updater
    // docs/design/analysis/name-resolution-proofs/variable-original-local-cache-unavailable-updater.md
    use super::*;
    let expected = include_str!(
        "../../../../../rust/tcl-syntax/tests/data/native_variable_name/cache_lookup/paths.txt"
    );
    assert_eq!(expected.lines().count(), 25);
    CACHE_LOOKUP_ROWS.with(|rows| rows.borrow_mut().clear());
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for case in 0..5 {
            CACHE_LOOKUP_CASE.with(|mode| mode.set(case));
            CACHE_LOOKUP_CALLBACKS.with(|count| count.set(0));
            let mut interp = Interp::with_native_core(
                super::super::default_host(),
                crate::environment::profile_for_dialect(environment),
                tcl_registry::special_vars::NativeBootstrapInputs {
                    package_path: Vec::new(),
                    default_library: None,
                },
            )
            .unwrap();
            interp.register_builtin(b"probe", cache_lookup_probe);
            let code = interp.eval_str(b"proc p {} {set x VALUE;probe x};p");
            assert_eq!(
                code == Code::Ok,
                case < 2 || (environment == "tcl8.4" && case < 4),
                "{environment}/{case}"
            );
        }
    }
    CACHE_LOOKUP_ROWS.with(|rows| {
        assert_eq!(
            rows.borrow().as_slice(),
            expected.lines().collect::<Vec<_>>()
        )
    });
}

#[test]
fn original_link_lookup_and_unset_preserve_target_name_cache() {
    use super::*;
    for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        let target = Owned::fresh(obj::new_string_bytes(b"target"));
        let prepared = interp
            .prepare_original_c_link_target(target.as_ptr(), 0)
            .unwrap();
        let prepared = prepared.expect("actual selected target");
        assert_eq!(prepared.name, b"target");
        assert_eq!(prepared.elem, None);
        assert!(
            obj::native_variable_name::with_parsed(target.as_ptr(), |cache| cache.array.is_none())
                .unwrap()
        );
        let value = Owned::fresh(obj::new_string_bytes(b"7"));
        interp
            .assign_original_named_variable(target.as_ptr(), value.as_ptr())
            .unwrap();
        interp
            .unset_original_c_variable(target.as_ptr(), true)
            .unwrap();
        assert!(
            obj::native_variable_name::with_parsed(target.as_ptr(), |cache| cache.array.is_none())
                .unwrap()
        );
        assert!(interp
            .read_original_named_variable(target.as_ptr())
            .is_err());
    }
}

#[test]
fn element_table_and_var_roles_match_all_25_actual_callback_windows() {
    // Native proof: naming.variable.original-element-unset-callback-roles
    // docs/design/analysis/name-resolution-proofs/variable-original-element-unset-callback-roles.md
    use super::*;
    use crate::frame::NativeElementEntryObserver;
    use crate::namespace::GLOBAL;
    use std::cell::RefCell;
    use tcl_runtime_api::native_variable_trace::{
        NativeVariableObserver, NativeVariableTraceAccess, NativeVariableTraceOperation,
    };
    struct Windows {
        version: &'static str,
        index_k: *mut TclObj,
        index_j: *mut TclObj,
        old_k: NativeElementEntryObserver,
        old_j: NativeElementEntryObserver,
        object_table: bool,
        rows: Rc<RefCell<Vec<String>>>,
        sequence: Rc<RefCell<usize>>,
    }
    impl Windows {
        fn record(&self, interp: &Interp, phase: &str) {
            let current = interp
                .namespaces
                .borrow()
                .var_table(GLOBAL)
                .capture_array_cell(b"arr");
            let (present_k, _, defined_k, dead_k, refs_k) =
                self.old_k.observe(current.as_ref(), self.object_table);
            let (present_j, _, defined_j, dead_j, refs_j) =
                self.old_j.observe(current.as_ref(), self.object_table);
            // SAFETY: the probe's two original index owners outlive every callback.
            let (key_k, key_j) = unsafe { ((*self.index_k).ref_count, (*self.index_j).ref_count) };
            self.rows.borrow_mut().push(format!("{}|window|{phase}|{key_k}|{key_j}|{dead_k}|{dead_j}|{defined_k}|{defined_j}|{refs_k}|{refs_j}|{present_k}|{present_j}", self.version));
        }
    }
    impl NativeVariableObserver<Interp> for Windows {
        type Error = tcl_cmd_core::CmdError;
        fn observe(
            &self,
            interp: &mut Interp,
            access: NativeVariableTraceAccess<'_>,
        ) -> Result<(), Self::Error> {
            let phase = if access.name2.is_empty() {
                "root".to_owned()
            } else {
                *self.sequence.borrow_mut() += 1;
                format!(
                    "element{}-{}",
                    *self.sequence.borrow(),
                    std::str::from_utf8(access.name2).unwrap()
                )
            };
            self.record(interp, &phase);
            Ok(())
        }
    }
    let rows = Rc::new(RefCell::new(Vec::new()));
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        let root = Owned::fresh(obj::new_string_bytes(b"arr"));
        let index_k = Owned::fresh(obj::new_string_bytes(b"k"));
        let index_j = Owned::fresh(obj::new_string_bytes(b"j"));
        let one = Owned::fresh(obj::new_string_bytes(b"ONE"));
        for index in [&index_k, &index_j] {
            let capture = interp
                .capture_original_c_parts_report(
                    root.as_ptr(),
                    Some(index.as_ptr()),
                    NativeVariableNameLookupPurpose::Write,
                )
                .unwrap()
                .unwrap();
            capture.receiver.store(one.as_ptr()).unwrap();
        }
        let array = interp
            .namespaces
            .borrow()
            .var_table(GLOBAL)
            .capture_array_cell(b"arr")
            .unwrap();
        let names = [
            Owned::fresh(obj::new_string_bytes(b"arr(k)")),
            Owned::fresh(obj::new_string_bytes(b"arr(j)")),
        ];
        let mut aliases = Vec::new();
        for name in &names {
            let mut target = interp
                .prepare_original_c_link_target(name.as_ptr(), 0)
                .unwrap()
                .unwrap();
            crate::vars::prepare_upvar_target(
                &mut interp.frames.borrow_mut(),
                &mut interp.namespaces.borrow_mut(),
                &mut target,
            )
            .unwrap();
            aliases.push(target);
        }
        let windows = Rc::new(Windows {
            version,
            index_k: index_k.as_ptr(),
            index_j: index_j.as_ptr(),
            old_k: array.observe_element_entry(b"k").unwrap(),
            old_j: array.observe_element_entry(b"j").unwrap(),
            object_table: interp
                .native_c_variable_name_protocol()
                .unwrap()
                .element_table_retains_original(),
            rows: Rc::clone(&rows),
            sequence: Rc::new(RefCell::new(0)),
        });
        for name in std::iter::once(&root).chain(names.iter()) {
            interp
                .add_native_variable_observer(
                    name.as_ptr(),
                    &[NativeVariableTraceOperation::Unset],
                    windows.clone(),
                )
                .unwrap();
        }
        windows.record(&interp, "before");
        assert!(interp.var_unset(b"arr"));
        windows.record(&interp, "after");
        drop(aliases);
    }
    let expected = include_str!("../../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_callbacks.txt").lines().filter(|line| line.contains("|window|")).collect::<Vec<_>>();
    assert_eq!(rows.borrow().len(), 25);
    assert_eq!(*rows.borrow(), expected);
}

#[test]
fn element_entry_lifecycle_matches_all_70_actual_c_windows() {
    // Native proof: naming.variable.original-element-alias-recreation
    // docs/design/analysis/name-resolution-proofs/variable-original-element-alias-recreation.md
    // Native proof: naming.variable.original-element-alias-header-roles
    // docs/design/analysis/name-resolution-proofs/variable-original-element-alias-header-roles.md
    use super::*;
    use crate::namespace::GLOBAL;
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        for whole in [false, true] {
            let mut interp = Interp::new();
            interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
            let protocol = interp.native_c_variable_name_protocol().unwrap();
            let original = Owned::fresh(obj::new_string_bytes(b"arr(k)"));
            let fresh = Owned::fresh(obj::new_string_bytes(b"arr(k)"));
            let one = Owned::fresh(obj::new_string_bytes(b"ONE"));
            let two = Owned::fresh(obj::new_string_bytes(b"TWO"));
            interp
                .assign_original_named_variable(original.as_ptr(), one.as_ptr())
                .unwrap();
            let array = interp
                .namespaces
                .borrow()
                .var_table(GLOBAL)
                .capture_array_cell(b"arr")
                .unwrap();
            let observer = array.observe_element_entry(b"k").unwrap();
            let root = obj::native_variable_name::with_parsed(original.as_ptr(), |cache| {
                cache.array.as_ref().unwrap().0.as_ptr()
            })
            .unwrap();
            let key = observer.key();
            // This is the native probe's declared external key observer,
            // needed only for the separately allocated C85/86 table key.
            let _key_pin = (protocol.version() >= tcl_dialect::TclVersion::V8_5
                && protocol.version() < tcl_dialect::TclVersion::V9_0)
                .then(|| Owned::retain(key.unwrap()));
            let observe = |interp: &Interp, phase: &str, rows: &mut Vec<String>| {
                let current = interp
                    .namespaces
                    .borrow()
                    .var_table(GLOBAL)
                    .capture_array_cell(b"arr");
                let (present, same, defined, dead, refs) =
                    observer.observe(current.as_ref(), protocol.element_table_retains_original());
                // SAFETY: original parser storage or the declared observer pin owns each header.
                let (name_refs, root_refs, key_refs) = unsafe {
                    (
                        (*original.as_ptr()).ref_count,
                        (*root).ref_count,
                        key.map_or(-1, |key| (*key).ref_count),
                    )
                };
                rows.push(format!("{version}|window|{}|{phase}|{name_refs}|{root_refs}|{key_refs}|{present}|{same}|{defined}|{dead}|{refs}", if whole { "whole" } else { "element" }));
            };
            observe(&interp, "created", &mut rows);
            let mut first = interp
                .prepare_original_c_link_target(original.as_ptr(), 0)
                .unwrap()
                .unwrap();
            crate::vars::prepare_upvar_target(
                &mut interp.frames.borrow_mut(),
                &mut interp.namespaces.borrow_mut(),
                &mut first,
            )
            .unwrap();
            let mut second = interp
                .prepare_original_c_link_target(original.as_ptr(), 0)
                .unwrap()
                .unwrap();
            crate::vars::prepare_upvar_target(
                &mut interp.frames.borrow_mut(),
                &mut interp.namespaces.borrow_mut(),
                &mut second,
            )
            .unwrap();
            observe(&interp, "linked", &mut rows);
            let target: &[u8] = if whole { b"arr" } else { b"arr(k)" };
            assert!(interp.var_unset(target));
            observe(&interp, "unset-target", &mut rows);
            interp
                .assign_original_named_variable(fresh.as_ptr(), two.as_ptr())
                .unwrap();
            observe(&interp, "recreated", &mut rows);
            assert!(interp.var_unset(target));
            observe(&interp, "unset-recreated", &mut rows);
            drop(second);
            observe(&interp, "one-alias", &mut rows);
            drop(first);
            observe(&interp, "no-alias", &mut rows);
        }
    }
    let expected = include_str!("../../../../../rust/tcl-syntax/tests/data/native_variable_name/element_alias_lifecycle.txt").lines().filter(|line| line.contains("|window|") && !line.starts_with("Jim|")).collect::<Vec<_>>();
    assert_eq!(rows.len(), 70);
    assert_eq!(rows, expected);
}

#[test]
fn search_free_slots_match_all_20_actual_c_windows() {
    // Native proof: naming.variable.array-search-free-slot-retirement
    // docs/design/analysis/name-resolution-proofs/variable-array-search-free-slot-retirement.md
    use super::*;
    use tcl_cmd_core::native_array_search::NativeArraySearchBackend;
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        let protocol = interp.array_search_protocol().unwrap();
        let name = Owned::fresh(obj::new_string_bytes(b"a(k)"));
        let member = Owned::fresh(obj::new_string_bytes(b"v"));
        interp
            .assign_original_named_variable(name.as_ptr(), member.as_ptr())
            .unwrap();
        let array = Owned::fresh(obj::new_string_bytes(b"a"));
        let command = Owned::fresh(obj::new_string_bytes(b"array"));
        let start = Owned::fresh(obj::new_string_bytes(b"startsearch"));
        assert_eq!(
            interp.dispatch(&[command.as_ptr(), start.as_ptr(), array.as_ptr()]),
            Code::Ok
        );
        let handle = Owned::retain(interp.get_obj_result());
        let anymore = Owned::fresh(obj::new_string_bytes(b"anymore"));
        assert_eq!(
            interp.dispatch(&[
                command.as_ptr(),
                anymore.as_ptr(),
                array.as_ptr(),
                handle.as_ptr()
            ]),
            Code::Ok
        );
        let observe = |phase: &str, original: &Owned| {
            let kind = obj::obj_type_ptr(original.as_ptr());
            // SAFETY: the test owns this original header and live descriptor.
            let free = !kind.is_null() && unsafe { (*kind).free_int_rep_proc.is_some() };
            format!(
                "{version}|{phase}|{}|{}|{}",
                if obj::native_array_search_cache_in(original.as_ptr(), protocol)
                    .unwrap()
                    .is_some()
                {
                    "array search"
                } else if matches!(
                    obj::native_object_snapshot(original.as_ptr())
                        .unwrap()
                        .cache,
                    tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
                ) {
                    "string"
                } else {
                    "none"
                },
                usize::from(free),
                usize::from(obj::has_string_rep(original.as_ptr()))
            )
        };
        rows.push(observe("converted", &handle));
        let copy = Owned::fresh(obj::duplicate(handle.as_ptr()));
        rows.push(observe("duplicate", &copy));
        assert!(interp
            .read_original_named_variable(handle.as_ptr())
            .is_err());
        rows.push(observe("missing", &handle));
        assert!(interp.read_original_named_variable(copy.as_ptr()).is_err());
        rows.push(observe("duplicate-missing", &copy));
    }
    let expected = include_str!(
        "../../../../../rust/tcl-syntax/tests/data/native_variable_name/array_search_free_slot.txt"
    )
    .lines()
    .filter(|line| !line.is_empty())
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 20);
    assert_eq!(rows, expected);
}
use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;

#[test]
fn scalar_alias_entries_and_array_parts_match_all_95_native_windows() {
    // Native proof: naming.variable.original-scalar-alias-entry-lifetime
    // docs/design/analysis/name-resolution-proofs/variable-original-scalar-alias-entry-lifetime.md
    // Native proof: naming.variable.original-parsed-array-child-references
    // docs/design/analysis/name-resolution-proofs/variable-original-parsed-array-child-references.md
    use crate::namespace::GLOBAL;
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        for mode in 0..2 {
            let name = Owned::fresh(obj::new_string_bytes(b"k"));
            let value = Owned::fresh(obj::new_string_bytes(b"ONE"));
            interp
                .assign_original_named_variable(name.as_ptr(), value.as_ptr())
                .unwrap();
            let before = interp
                .namespaces
                .borrow()
                .var_table(GLOBAL)
                .native_name_cell_identity(b"k")
                .unwrap();
            let observe = |interp: &Interp, phase: &str, fresh: Option<&Owned>| {
                let namespaces = interp.namespaces.borrow();
                let table = namespaces.var_table(GLOBAL);
                let found = table.native_name_cell_identity(b"k");
                assert!(matches!(
                    obj::native_object_snapshot(name.as_ptr()).unwrap().cache,
                    Cache::ParsedVariableName { .. }
                ));
                // SAFETY: these original object headers are owned throughout each window.
                let (references, fresh_references) = unsafe {
                    (
                        (*name.as_ptr()).ref_count,
                        fresh.map_or(-1, |fresh| (*fresh.as_ptr()).ref_count),
                    )
                };
                format!(
                        "{version}|alias|{mode}|{phase}|parsedVarName|{references}|{fresh_references}|{}|{}|{}",
                        usize::from(found.is_some()),
                        usize::from(found == Some(before)),
                        usize::from(table.original_native_key_is(b"k", name.as_ptr()))
                    )
            };
            rows.push(observe(&interp, "created", None));
            interp.frames.borrow_mut().push(GLOBAL);
            let target = crate::vars::link_target_at(
                &interp.frames.borrow(),
                &interp.namespaces.borrow(),
                b"k",
                None,
                0,
            )
            .unwrap();
            interp.make_upvar(target, b"a");
            rows.push(observe(&interp, "linked", None));
            assert!(interp.var_unset(b"::k"));
            rows.push(observe(&interp, "unset-target", None));
            let fresh = Owned::fresh(obj::new_string_bytes(b"k"));
            let written =
                Owned::fresh(obj::new_string_bytes(if mode == 0 { b"::k" } else { b"a" }));
            interp
                .assign_original_named_variable(written.as_ptr(), value.as_ptr())
                .unwrap();
            rows.push(observe(&interp, "recreated", Some(&fresh)));
            assert!(interp.var_unset(b"::k"));
            rows.push(observe(&interp, "unset-recreated-target", Some(&fresh)));
            assert!(!interp.var_unset(b"a"));
            rows.push(observe(&interp, "unset-alias", Some(&fresh)));
            interp.frames.borrow_mut().pop();
            rows.push(observe(&interp, "frame-popped", Some(&fresh)));
            assert!(interp.read_original_named_variable(fresh.as_ptr()).is_err());
            rows.push(observe(&interp, "fresh-read", Some(&fresh)));
        }
        let name = Owned::fresh(obj::new_string_bytes(b"arr(k)"));
        let value = Owned::fresh(obj::new_string_bytes(b"VALUE"));
        interp
            .assign_original_named_variable(name.as_ptr(), value.as_ptr())
            .unwrap();
        let observe = |phase: &str| {
            obj::native_variable_name::with_parsed(name.as_ptr(), |cache| {
                let (root, element) = cache.array.as_ref().unwrap();
                // SAFETY: cache children remain owned by the original parsed name.
                let (root_refs, element_refs) = unsafe {
                    (
                        (*root.as_ptr()).ref_count,
                        match element {
                            NativeParsedVariableElement::Object(element) => {
                                (*element.as_ptr()).ref_count
                            }
                            NativeParsedVariableElement::Bytes(_) => -1,
                        },
                    )
                };
                format!("{version}|parts|{phase}|{root_refs}|{element_refs}")
            })
            .unwrap()
        };
        rows.push(observe("stored"));
        let copy = Owned::fresh(obj::duplicate(name.as_ptr()));
        rows.push(observe("duplicated"));
        drop(copy);
        assert!(interp.var_unset(b"arr"));
        rows.push(observe("unset"));
    }
    let expected = include_str!(
        "../../../../../rust/tcl-syntax/tests/data/native_variable_name/scalar_alias_entries.txt"
    )
    .lines()
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 95);
    assert_eq!(rows, expected);
}

fn observe(version: &str, case: &str, phase: &str, original: &Owned) -> String {
    let snapshot = obj::native_object_snapshot(original.as_ptr()).unwrap();
    let kind = match snapshot.cache {
        Cache::None => "none",
        Cache::Numeric(_) => "int",
        Cache::List { .. } => "list",
        Cache::ByteArray { .. } => "bytearray",
        Cache::ParsedVariableName { .. } => "parsedVarName",
        other => panic!("unexpected actual original primary: {other:?}"),
    };
    let descriptor = obj::obj_type_ptr(original.as_ptr());
    // SAFETY: both the retained original header and its descriptor are live.
    let (references, free_hook) = unsafe {
        (
            (*original.as_ptr()).ref_count,
            !descriptor.is_null() && (*descriptor).free_int_rep_proc.is_some(),
        )
    };
    let hex = snapshot
        .resident
        .as_deref()
        .map_or_else(String::new, |bytes| {
            bytes.iter().map(|byte| format!("{byte:02x}")).collect()
        });
    format!(
        "{version}|{case}|{phase}|{kind}|{references}|{}|{}|{hex}",
        usize::from(free_hook),
        usize::from(snapshot.resident.is_some())
    )
}

#[test]
fn original_parsed_headers_match_all_150_actual_c_windows() {
    // Native proof: naming.variable.original-scalar-parsed-header
    // docs/design/analysis/name-resolution-proofs/variable-original-scalar-parsed-header.md
    // Native proof: naming.variable.original-counted-array-parsed-header
    // docs/design/analysis/name-resolution-proofs/variable-original-counted-array-parsed-header.md
    let mut rows = Vec::new();
    for (environment, version) in [
        ("tcl8.4", "8.4.20"),
        ("tcl8.5", "8.5.19"),
        ("tcl8.6", "8.6.18"),
        ("tcl9.0", "9.0.4"),
        ("tcl9.1", "9.1.0"),
    ] {
        let mut interp = Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect(environment));
        let dialect = interp.native_invocation_dialect();
        let protocol = dialect.native_string_protocol().unwrap();
        for case in ["string", "int", "list", "bytearray", "array"] {
            let original = match case {
                "string" => Owned::fresh(obj::new_string_bytes(b"missing")),
                "int" => {
                    let original = Owned::fresh(obj::new_wide_int_obj(42));
                    if version == "8.4.20" {
                        obj::adopt_native_scalar_cache(
                            original.as_ptr(),
                            tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(42),
                            dialect.native_scalar_getter_protocol().unwrap(),
                        )
                        .unwrap();
                    }
                    original
                }
                "list" => {
                    let member = Owned::fresh(obj::new_string_bytes(b"missing"));
                    Owned::fresh(crate::list::new_list_obj_native(
                        &[member.as_ptr()],
                        protocol,
                    ))
                }
                "bytearray" => Owned::fresh(interp.new_native_byte_array(b"missing").unwrap()),
                "array" => Owned::fresh(obj::new_string_bytes(b"arr(k\0z)")),
                _ => unreachable!(),
            };
            rows.push(observe(version, case, "before", &original));
            assert!(interp
                .read_original_named_variable(original.as_ptr())
                .is_err());
            rows.push(observe(version, case, "missing", &original));
            let value = Owned::fresh(obj::new_string_bytes(b"VALUE"));
            interp
                .store_original_named_variable(original.as_ptr(), value.as_ptr())
                .unwrap();
            rows.push(observe(version, case, "stored", &original));
            assert_eq!(
                interp
                    .read_original_named_variable(original.as_ptr())
                    .unwrap(),
                value.as_ptr()
            );
            rows.push(observe(version, case, "read", &original));
            let copy = Owned::fresh(obj::duplicate(original.as_ptr()));
            rows.push(observe(version, case, "duplicate", &copy));
            drop(copy);
            for name in [b"missing".as_slice(), b"42".as_slice(), b"arr".as_slice()] {
                interp.var_unset(name);
            }
            assert!(interp
                .read_original_named_variable(original.as_ptr())
                .is_err());
            rows.push(observe(version, case, "after-unset-missing", &original));
        }
    }
    let expected = include_str!(
        "../../../../../rust/tcl-syntax/tests/data/native_variable_name/parsed_headers.txt"
    )
    .lines()
    .collect::<Vec<_>>();
    assert_eq!(rows.len(), 150);
    assert_eq!(rows, expected);
}
#[test]
fn list_assignment_arity_matches_five_original_source_controls() {
    // Native proof: naming.list.original-lassign-target-arity
    // docs/design/analysis/name-resolution-proofs/list-original-lassign-target-arity.md
    // Literal, dynamic and proc source entry are observations, not admission proofs.
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../tests/data/native_list_assignment_arity/8.4.20.txt")
                .as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../tests/data/native_list_assignment_arity/8.5.19.txt")
                .as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../tests/data/native_list_assignment_arity/8.6.18.txt")
                .as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../tests/data/native_list_assignment_arity/9.0.4.txt").as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../tests/data/native_list_assignment_arity/9.1.0.txt").as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../tests/data/native_list_assignment_arity/source.tcl"),
            expected,
        );
    }
}

#[test]
fn list_assignment_arity_matches_current_jim_original_source_controls() {
    // Native proof: naming.list.original-lassign-target-arity
    // docs/design/analysis/name-resolution-proofs/list-original-lassign-target-arity.md
    trace_source_observation(
        "jim",
        include_bytes!("../../../tests/data/native_list_assignment_arity/source.tcl"),
        include_bytes!("../../../tests/data/native_list_assignment_arity/jim.txt"),
    );
}
#[test]
fn variable_read_diagnostics_match_five_original_source_controls() {
    // Native proof: naming.variable.original-read-diagnostic-input
    // docs/design/analysis/name-resolution-proofs/variable-original-read-diagnostic-input.md
    // These source-entry outcomes grant no private layout or compiler admission.
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../tests/data/native_variable_read_input/8.4.20.txt").as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../tests/data/native_variable_read_input/8.5.19.txt").as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../tests/data/native_variable_read_input/8.6.18.txt").as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../tests/data/native_variable_read_input/9.0.4.txt").as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../tests/data/native_variable_read_input/9.1.0.txt").as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../tests/data/native_variable_read_input/source.tcl"),
            expected,
        );
    }
}

#[test]
fn variable_read_input_preserves_current_jim_dictionary_results() {
    // Native proof: naming.variable.original-read-diagnostic-input
    // docs/design/analysis/name-resolution-proofs/variable-original-read-diagnostic-input.md
    // Jim's array-valued root reads succeed; its scalar element reads fail.
    trace_source_observation(
        "jim",
        include_bytes!("../../../tests/data/native_variable_read_input/source.tcl"),
        include_bytes!("../../../tests/data/native_variable_read_input/jim.txt"),
    );
}

#[test]
fn retired_namespace_c86_source_entries_preserve_scalar_and_element_sites() {
    trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/input.tcl"
            ),
            include_bytes!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-01/8.6.18/stdout.txt"
            ),
        );
}

#[test]
fn retired_namespace_c86_variable_heads_preserve_original_root_tuple() {
    trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/variable-head-input.tcl"
            ),
            include_bytes!(
                "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.6.18/stdout.txt"
            ),
        );
}

#[test]
fn retired_namespace_original_object_vector_keeps_root_despite_compiled_cell() {
    fn original_vector(interp: &mut Interp, _args: &[*mut TclObj]) -> Code {
        let head = Owned::fresh(obj::new_string_bytes(b"set"));
        let name = Owned::fresh(obj::new_string_bytes(b"foo(3)"));
        let value = Owned::fresh(obj::new_string_bytes(b"3"));
        let code =
            interp.eval_original_object_vector(&[head.as_ptr(), name.as_ptr(), value.as_ptr()]);
        let root = obj::native_variable_name::with_parsed(name.as_ptr(), |cache| {
            cache.array.as_ref().map(|(root, _)| root.as_ptr())
        })
        .flatten()
        .expect("original parsed root retained after failure");
        assert!(interp.original_c_local_index(root).is_some());
        code
    }
    let mut interp = Interp::with_native_core(
        super::super::default_host(),
        crate::environment::profile_for_dialect("tcl8.6"),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    interp.register_builtin(b"original_vector_write", original_vector);
    assert_eq!(
        interp.eval_str(
            br#"
            namespace eval test_ns_var {variable foo 2}
            proc p {} {
                variable ::test_ns_var::foo
                set live [set foo]
                namespace delete ::test_ns_var
                set code [catch {original_vector_write} result]
                list $code $result $::errorCode
            }
            p
        "#
        ),
        Code::Ok,
        "{:?}",
        interp.native_execution_refusal()
    );
    let protocol =
        tcl_syntax::native_string::NativeStringProtocol::C(tcl_dialect::TclVersion::V8_6);
    let capture = include_str!(
            "../../../../../experiments/name-handling/results/runtime-c86-real-suites/retired-namespace-variable-diagnostic-question/capture-02/8.6.18/stdout.txt"
        );
    let line = capture
        .lines()
        .find(|line| line.starts_with("ROW variable-head-element "))
        .unwrap();
    let row = tcl_syntax::list::split_native_list_bytes(line.as_bytes(), protocol).unwrap();
    let expected = tcl_syntax::list::split_native_list_bytes(&row[3], protocol).unwrap();
    let result = interp.result_bytes();
    let actual = tcl_syntax::list::split_native_list_bytes(&result, protocol).unwrap();
    assert_eq!(actual.len(), 3);
    for (actual, expected) in actual.iter().zip(&expected[2..]) {
        assert_eq!(actual.as_ref(), expected.as_ref());
    }
    assert!(!interp.host_refusal_pending());
}

#[test]
fn detached_element_c86_original_var17_write_preserves_late_diagnostic() {
    trace_source_observation(
        "tcl8.6",
        include_bytes!(
            "../../../tests/data/native_detached_element_write_sites/original-var17.tcl"
        ),
        include_bytes!(
            "../../../tests/data/native_detached_element_write_sites/original-var17.txt"
        ),
    );
}

#[test]
fn detached_element_c86_recreated_root_keeps_old_link_write_detached() {
    trace_source_observation(
        "tcl8.6",
        include_bytes!(
            "../../../tests/data/native_detached_element_write_sites/root-recreated-old-link.tcl"
        ),
        include_bytes!(
            "../../../tests/data/native_detached_element_write_sites/root-recreated-old-link.txt"
        ),
    );
}

#[test]
fn detached_element_c86_recreated_root_admits_new_link_write() {
    trace_source_observation(
            "tcl8.6",
            include_bytes!(
                "../../../tests/data/native_detached_element_write_sites/root-recreated-current-link.tcl"
            ),
            include_bytes!(
                "../../../tests/data/native_detached_element_write_sites/root-recreated-current-link.txt"
            ),
        );
}

#[test]
fn array_root_write_diagnostics_match_five_original_source_controls() {
    // Native proof: naming.variable.original-array-root-write-diagnostic
    // docs/design/analysis/name-resolution-proofs/variable-original-array-root-write-diagnostic.md
    // Full source output is compared; no private receiver or compiler admission is inferred.
    for (engine, expected) in [
        (
            "tcl8.4",
            include_bytes!("../../../tests/data/native_variable_array_write/8.4.20.txt").as_slice(),
        ),
        (
            "tcl8.5",
            include_bytes!("../../../tests/data/native_variable_array_write/8.5.19.txt").as_slice(),
        ),
        (
            "tcl8.6",
            include_bytes!("../../../tests/data/native_variable_array_write/8.6.18.txt").as_slice(),
        ),
        (
            "tcl9.0",
            include_bytes!("../../../tests/data/native_variable_array_write/9.0.4.txt").as_slice(),
        ),
        (
            "tcl9.1",
            include_bytes!("../../../tests/data/native_variable_array_write/9.1.0.txt").as_slice(),
        ),
    ] {
        trace_source_observation(
            engine,
            include_bytes!("../../../tests/data/native_variable_array_write/source.tcl"),
            expected,
        );
    }
}

#[test]
fn array_root_write_preserves_current_jim_dictionary_replacement() {
    // Native proof: naming.variable.original-array-root-write-diagnostic
    // docs/design/analysis/name-resolution-proofs/variable-original-array-root-write-diagnostic.md
    // Jim replaces its dictionary-valued root with the scalar result.
    trace_source_observation(
        "jim",
        include_bytes!("../../../tests/data/native_variable_array_write/source.tcl"),
        include_bytes!("../../../tests/data/native_variable_array_write/jim.txt"),
    );
}
