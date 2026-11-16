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
use crate::capi::{tcl_runtime_create_interp, tcl_runtime_delete_interp};
use crate::counters;
use crate::interp::Code;
use std::cell::RefCell;
use std::rc::Rc;
use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, Process, StdIo};
use tcl_runtime_api::codegen_abi::{NATIVE_PROC_STATUS_DECLINED, NATIVE_PROC_STATUS_RAN};
use tcl_runtime_api::guard::GuardDomain;

/// Run `body` under the alloc/free counters and assert zero residual — the
/// codegen ABI's references must balance exactly like the rest of the runtime.
fn leak_free(body: impl FnOnce()) {
    counters::reset();
    body();
    assert_eq!(
        counters::finalize(),
        0,
        "residual: {} objs, {} bufs",
        counters::live_objs(),
        counters::live_bufs()
    );
    assert_eq!(counters::double_free_count(), 0, "double frees detected");
}

/// Box a `&[u8]` as the emitter would (a fresh `rc 0` script/expr object).
unsafe fn box_str(s: &[u8]) -> *mut TclObj {
    // SAFETY: `s` is a valid readable slice.
    unsafe { tcl_obj_new_string(s.as_ptr(), s.len() as i32) }
}

#[cfg(have_tommath)]
unsafe fn owned_str(s: &[u8]) -> *mut TclObj {
    // SAFETY: `s` is a valid readable slice.
    unsafe { tcl_value_new_string(s.as_ptr(), s.len() as i32) }
}

/// Box one borrowed argv word and take the caller's owning reference.
unsafe fn owned_word(s: &[u8]) -> *mut TclObj {
    // SAFETY: `box_str` copies valid bytes into a fresh object.
    let word = unsafe { box_str(s) };
    // SAFETY: the test owns the fresh object and keeps this +1 until it
    // releases the argv after invoking.
    unsafe { obj::incr_ref_count(word) };
    word
}

fn empty_completion() -> TclCompletionAbi {
    TclCompletionAbi {
        code: 0,
        result: ptr::null_mut(),
        options: ptr::null_mut(),
    }
}

unsafe fn invoke(words: &[*mut TclObj]) -> TclCompletionAbi {
    let mut completion = empty_completion();
    // SAFETY: `words` is a Rust slice of live caller-owned object handles;
    // completion storage is local and correctly aligned.
    assert_eq!(
        unsafe {
            tcl_invoke_argv(
                words.as_ptr(),
                i32::try_from(words.len()).expect("test argv fits i32"),
                &mut completion,
            )
        },
        TCL_INVOKE_ABI_OK
    );
    completion
}

unsafe fn invoke_intrinsic(
    intrinsic: IntrinsicId,
    words: &[*mut TclObj],
) -> (i32, TclCompletionAbi) {
    let mut completion = empty_completion();
    // SAFETY: `words` is a Rust slice of live caller-owned object handles;
    // completion storage is local and correctly aligned.
    let status = unsafe {
        tcl_intrinsic_invoke_argv(
            intrinsic.stable_id(),
            words.as_ptr(),
            i32::try_from(words.len()).expect("test argv fits i32"),
            &mut completion,
        )
    };
    (status, completion)
}

unsafe fn prepare_intrinsic_guard(
    intrinsic: IntrinsicId,
    words: &[*mut TclObj],
    domains: i32,
) -> u64 {
    let interp = current_interp();
    assert!(
        !interp.is_null(),
        "test helper requires a current interpreter"
    );
    // SAFETY: tests install a live current interpreter before using this helper.
    let characters = unsafe { (*interp).native_invocation_dialect() }
        .characters
        .expect("test runtime has a selected character protocol");
    let identity = GuardIdentity::registry_intrinsic_with_semantics(
        intrinsic.stable_id(),
        intrinsic.guard_semantics_key_for_characters(characters),
    );
    // SAFETY: `words` is a Rust slice of live caller-owned object handles.
    unsafe {
        tcl_codegen_guard_prepare(
            intrinsic.stable_id(),
            words.as_ptr(),
            i32::try_from(words.len()).expect("test argv fits i32"),
            identity.namespace(),
            identity.value(),
            domains,
        )
    }
}

unsafe fn release_words(words: &[*mut TclObj]) {
    for &word in words {
        // SAFETY: balances `owned_word`'s caller-owned reference.
        unsafe { obj::decr_ref_count(word) };
    }
}

fn option(completion: &TclCompletionAbi, key: &[u8]) -> Vec<u8> {
    let value = crate::dict::dict_get(completion.options, key)
        .expect("completion options are a dict")
        .expect("expected completion option");
    obj_bytes(value)
}

fn scalar_interpreter(environment: &str) -> Interp {
    Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::resolve_environment(environment).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap()
}

thread_local! {
    static SCALAR_UPDATER_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

// native proc entries

thread_local! {
    /// The argv [`stub_body`] dispatches, and how many times a stub ran.
    static STUB_ARGV: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static STUB_CALLS: Cell<u32> = const { Cell::new(0) };
    /// `(line, source text)` [`stub_body`] logs on an error completion, as
    /// the emitter will on a statement's error edge. `None` logs nothing.
    static STUB_LOG: RefCell<Option<(i32, Vec<u8>)>> = const { RefCell::new(None) };
}

mod activation;
mod argv_invocation;
mod call_frames;
mod evaluation;
mod host_refusal;
mod procedure_definition;
mod values;
mod variables;

#[cfg(debug_assertions)]
pub(super) use procedure_definition::stub_declines;

#[test]
fn the_runtime_states_its_identity_to_a_host() {
    use tcl_runtime_api::codegen_abi::CODEGEN_ABI_VERSION;
    use tcl_runtime_api::manifest::EMBEDDED_STDLIB_REVISION;
    use tcl_runtime_api::ArtefactIdentityManifest;

    // SAFETY: every buffer is as long as the capacity it is passed with.
    unsafe {
        tcl_runtime_set_current_interp(ptr::null_mut());
        assert_eq!(
            tcl_runtime_identity(ptr::null_mut(), 0),
            0,
            "no interp is current, so there is nothing to state"
        );

        let interp = tcl_runtime_create_interp();
        tcl_runtime_set_current_interp(interp);
        (*interp).set_runtime_version(TclVersion::V8_6);
        let needed = tcl_runtime_identity(ptr::null_mut(), 0);
        assert!(needed > 0);

        let mut short = vec![0xAA_u8; usize::try_from(needed).unwrap() - 1];
        assert_eq!(tcl_runtime_identity(short.as_mut_ptr(), needed - 1), needed);
        assert!(
            short.iter().all(|byte| *byte == 0xAA),
            "a short buffer is not written"
        );

        let mut buffer = vec![0_u8; usize::try_from(needed).unwrap()];
        assert_eq!(tcl_runtime_identity(buffer.as_mut_ptr(), needed), needed);
        let manifest = ArtefactIdentityManifest::from_bytes(&buffer).expect("it decodes");
        assert_eq!(manifest, (*interp).held_identity());
        assert_eq!(manifest.environment, "tcl8.6");
        assert_eq!(manifest.release, "8.6");
        assert_eq!(manifest.abi_version, CODEGEN_ABI_VERSION);
        assert_eq!(
            manifest.intrinsic_table_hash,
            tcl_registry::intrinsic_table_hash()
        );
        assert_eq!(manifest.embedded_stdlib_revision, EMBEDDED_STDLIB_REVISION);
        assert!(manifest.packs.is_empty());

        tcl_runtime_set_current_interp(ptr::null_mut());
        tcl_runtime_delete_interp(interp);
    }
}
