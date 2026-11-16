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

use super::super::{default_host, Code, Interp};
use crate::obj;

fn public_roots(interpreter: &Interp) -> (Option<usize>, Option<usize>) {
    let namespaces = interpreter.namespaces.borrow();
    let table = namespaces
        .original_variable_table_at(crate::namespace::GLOBAL)
        .expect("live root namespace");
    (
        table
            .published_scalar_header(b"errorInfo")
            .map(|v| v as usize),
        table
            .published_scalar_header(b"errorCode")
            .map(|v| v as usize),
    )
}

#[test]
fn allocation_lease_cannot_certify_a_retired_error_header() {
    let original = obj::Owned::fresh(obj::new_string_bytes(b"original error header"));
    let pointer = original.as_ptr();
    let before = unsafe { (*pointer).ref_count };
    let lease = obj::NativeObjectLifetime::retain(pointer);
    assert_eq!(unsafe { (*pointer).ref_count }, before);
    assert_eq!(lease.checked_ptr().unwrap(), pointer);
    drop(original);
    assert!(lease.checked_ptr().is_err());
}

#[test]
fn c9_next_entry_publishes_original_error_headers_without_stringifying_code() {
    let environment = tcl_registry::model::ingress::resolve_known_environment("tcl9.0").unwrap();
    let mut interpreter = Interp::with_native_core(
        default_host(),
        environment.unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    assert_eq!(interpreter.runtime_version(), tcl_dialect::TclVersion::V9_0);
    assert_eq!(public_roots(&interpreter), (None, None));
    let script = b"set go 1\nwhile {$go} {string index\nlappend never x}\nstring length abc\n";
    let mut previous: Option<(obj::NativeObjectLifetime, obj::NativeObjectLifetime)> = None;
    for _ in 0..3 {
        assert_eq!(interpreter.eval_str(script), Code::Error);
        assert!(interpreter.native_execution_refusal().is_none());
        let current = {
            let exception = interpreter.exc.borrow();
            assert!(exception.native.legacy_copy);
            let info = exception
                .native
                .info
                .as_ref()
                .expect("private errorInfo")
                .as_ptr();
            let code = exception
                .native
                .code
                .as_ref()
                .expect("private errorCode")
                .as_ptr();
            // Both private native owners remain held throughout these raw reads.
            unsafe {
                assert!((*info).ref_count > 0);
                assert!((*code).ref_count > 0);
                assert!((*code).bytes.is_null());
                assert!(std::ptr::eq((*code).type_ptr, &crate::list::TCL_LIST_TYPE));
            }
            (
                obj::NativeObjectLifetime::retain(info),
                obj::NativeObjectLifetime::retain(code),
            )
        };
        let globals = public_roots(&interpreter);
        match previous.take() {
            Some((info, code)) => {
                let info = info
                    .checked_ptr()
                    .expect("original live errorInfo incarnation");
                let code = code
                    .checked_ptr()
                    .expect("original live errorCode incarnation");
                assert_eq!(globals, (Some(info as usize), Some(code as usize)));
                assert_ne!(globals.0, Some(current.0.as_ptr() as usize));
                assert_ne!(globals.1, Some(current.1.as_ptr() as usize));
            }
            None => assert_eq!(globals, (None, None)),
        }
        previous = Some(current);
    }
}
