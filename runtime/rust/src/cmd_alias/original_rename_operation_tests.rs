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

#[cfg(debug_assertions)]
use super::*;
#[cfg(debug_assertions)]
use crate::obj::Owned;
use std::cell::{Cell, RefCell};
use tcl_runtime_api::NativeExecutionError;
#[cfg(debug_assertions)]
use tcl_syntax::raw_string::NativeValueAccessRefusal;

#[derive(Clone, Copy)]
#[cfg(debug_assertions)]
enum Callback {
    Normal,
    Host,
    Restore,
}
thread_local! {
    #[cfg(debug_assertions)]
    static MODE: Cell<Callback> = const { Cell::new(Callback::Normal) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
    static FIRST: RefCell<Option<NativeExecutionError>> = const { RefCell::new(None) };
}
#[cfg(debug_assertions)]
fn victim(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(b"VICTIM");
    Code::Ok
}
#[cfg(debug_assertions)]
fn observer(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    CALLS.with(|calls| calls.set(calls.get() + 1));
    let effect = Owned::fresh(obj::new_string_bytes(b"REACHED\0\xff"));
    interp
        .var_set(b"original_rename_effect", effect.as_ptr())
        .unwrap();
    match MODE.with(Cell::get) {
        Callback::Normal => interp.set_result_bytes(b"CALLBACK"),
        Callback::Host => {
            interp.refuse_host_command("first original rename observer");
            FIRST.with(|first| *first.borrow_mut() = interp.native_execution_refusal());
        }
        Callback::Restore => {
            let original = interp.runtime_context();
            let mut changed = original.clone();
            changed.packages = vec![("original-rename-observer".to_owned(), "1.0".to_owned())];
            interp.pin_context(&changed).unwrap();
            interp.pin_context(&original).unwrap();
        }
    }
    Code::Ok
}

#[cfg(debug_assertions)]
#[test]
fn original_rename_observers_keep_reached_effects_and_stop_later_result_publication() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Actual command-trace callbacks and native entry; no external Native order claim.
    for version in ["tcl8.6", "tcl9.1"] {
        for mode in [Callback::Normal, Callback::Host, Callback::Restore] {
            for deleting in [false, true] {
                let mut interp = super::original_operand_tests::native(version);
                interp.register_builtin(b"original_rename_victim", victim);
                interp.register_builtin(b"original_rename_observer", observer);
                assert_eq!(interp.eval_str(b"trace add command original_rename_victim {rename delete} original_rename_observer"), Code::Ok);
                MODE.with(|selected| selected.set(mode));
                CALLS.with(|calls| calls.set(0));
                FIRST.with(|first| first.borrow_mut().take());
                let owned = super::original_operand_tests::words(&[
                    b"rename",
                    b"original_rename_victim",
                    if deleting {
                        b""
                    } else {
                        b"original_rename_moved"
                    },
                ]);
                let argv: Vec<_> = owned.iter().map(Owned::as_ptr).collect();
                let counts: Vec<_> = argv.iter().map(|&p| unsafe { (*p).ref_count }).collect();
                let prior = super::original_operand_tests::seed(&mut interp);
                let code = super::rename(&mut interp, &argv);
                assert_eq!(CALLS.with(Cell::get), 1);
                assert!(interp
                    .resolve_cmd_token(b"original_rename_victim")
                    .is_none());
                assert_eq!(
                    interp.resolve_cmd_token(b"original_rename_moved").is_some(),
                    !deleting
                );
                if !matches!(mode, Callback::Normal) {
                    assert!(interp.var_get(b"original_rename_effect").is_none());
                }
                let effect = super::original_operand_tests::reached_variable(
                    &interp,
                    b"original_rename_effect",
                );
                assert_eq!(obj::bytes_of(effect), b"REACHED\0\xff");
                assert_eq!(
                    argv.iter()
                        .map(|&p| unsafe { (*p).ref_count })
                        .collect::<Vec<_>>(),
                    counts
                );
                assert_eq!(interp.entered_native_operation_count(), 0);
                if matches!(mode, Callback::Normal) {
                    assert_eq!(code, Code::Ok);
                    assert!(interp.native_execution_refusal().is_none());
                    assert_eq!(obj::bytes_of(interp.get_obj_result()), b"");
                } else {
                    assert_eq!(code, Code::Error);
                    let expected = match mode {
                        Callback::Host => FIRST.with(|first| {
                            first.borrow().clone().expect("actual callback first Host")
                        }),
                        Callback::Restore => NativeExecutionError::ValueAccessRefusal(
                            NativeValueAccessRefusal::CommandProtocolUnavailable(
                                "stale entered native operation",
                            ),
                        ),
                        Callback::Normal => unreachable!(),
                    };
                    assert_eq!(interp.native_execution_refusal(), Some(expected));
                    assert_eq!(interp.get_obj_result(), prior.as_ptr());
                    assert_eq!(obj::bytes_of(prior.as_ptr()), b"PRIOR\0\xff");
                    assert_eq!(interp.pending_return_level(), 2);
                    assert_eq!(interp.pending_return_code(), Code::Other(7));
                    assert_eq!(interp.pending_return_option_objects().len(), 1);
                }
            }
        }
    }
}
