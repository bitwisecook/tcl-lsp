// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reached original frame failures retain their typed result producer.

use crate::CmdError;
use tcl_syntax::native_frame_error::NativeFrameLevelFailure;

/// Present a reached original-object frame failure without discarding result
/// birth or rebuilding a primitive getter failure from its rendered text.
#[must_use]
pub fn present(failure: NativeFrameLevelFailure) -> CmdError {
    // naming.variable.original-upvar-and-exists-completion-and-name-windows
    // docs/design/analysis/name-resolution-proofs/variable.original-upvar-and-exists-completion-and-name-windows.md
    // C tclProc.c uses Tcl_AppendResult84/85 and Tcl_ObjPrintf86+.
    match failure {
        NativeFrameLevelFailure::Primitive(record) => {
            tcl_syntax::value::ValueError::NativeScalarGetter(record).into()
        }
        NativeFrameLevelFailure::BadLevel {
            name,
            lookup_code,
            string_result,
        } => {
            let mut message = b"bad level \"".to_vec();
            message.extend_from_slice(&name);
            message.push(b'"');
            let error = if lookup_code {
                let mut code = b"TCL LOOKUP LEVEL".to_vec();
                tcl_syntax::list::append_list_element(&mut code, &name, false);
                CmdError::with_error_code_bytes(message, code)
            } else {
                CmdError::new_bytes(message)
            };
            match string_result {
                Some(protocol) => error.with_native_string_result(protocol),
                None => error,
            }
        }
    }
}
