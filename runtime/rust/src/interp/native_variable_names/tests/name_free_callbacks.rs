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

//! A name object freed during an operation: the callback probes for `upvar`, compiled slots and canonical names.

use super::*;

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
                super::super::super::default_host(),
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
                super::super::super::default_host(),
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
    let operation =
        super::super::super::native_operation_currency::NativeOperationScope::enter(interp)
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
        super::super::super::default_host(),
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
