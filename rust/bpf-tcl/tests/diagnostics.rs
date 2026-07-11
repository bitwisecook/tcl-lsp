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

//! The typed front-end rejects out-of-subset constructs with the right codes.

use bpf_tcl_ir::{BpfDiag, compile_module};

#[test]
fn rejects_plain_set() {
    let e = compile_module("when SOCKET_FILTER { set x 1\n accept }\n").unwrap_err();
    assert_eq!(e.code, BpfDiag::OutOfSubset);
}

#[test]
fn rejects_unknown_event() {
    let e = compile_module("when TURBOENCABULATOR { accept }\n").unwrap_err();
    assert_eq!(e.code, BpfDiag::BadEvent);
}

#[test]
fn rejects_unbounded_while() {
    let e = compile_module(
        "when SOCKET_FILTER { seti32 n {0}\n while {1} { seti32 n {1} }\n accept }\n",
    )
    .unwrap_err();
    assert_eq!(e.code, BpfDiag::UnboundedLoop);
}

#[test]
fn rejects_pointer_arithmetic() {
    let e =
        compile_module("when SOCKET_FILTER { setbuf pkt ctx\n setint x {$pkt + 1}\n accept }\n")
            .unwrap_err();
    assert_eq!(e.code, BpfDiag::TypeMismatch);
}

#[test]
fn rejects_undefined_var() {
    let e = compile_module("when SOCKET_FILTER { setint x {$nope + 1}\n accept }\n").unwrap_err();
    assert_eq!(e.code, BpfDiag::UndefinedVar);
}

#[test]
fn rejects_unknown_command() {
    let e = compile_module("when SOCKET_FILTER { frobnicate 1 2\n accept }\n").unwrap_err();
    assert_eq!(e.code, BpfDiag::UnknownCommand);
}

/// Concurrency primitives (coroutines, the `thread` package, the event loop)
/// cannot exist on eBPF — a program is a single bounded run to a verdict — so
/// they are rejected as `OutOfSubset` with a concurrency-specific message,
/// distinct from the generic "unknown command".
#[test]
fn rejects_concurrency_primitives() {
    for body in [
        "coroutine c mygen",
        "yield 1",
        "yieldto foo",
        "coroinject c foo",
        "coroprobe c bar",
        "thread::create {}",
        "thread::send $id {}",
        "tsv::set arr k 1",
        "tpool::create",
    ] {
        let src = format!("when SOCKET_FILTER {{ {body}\n accept }}\n");
        let Err(e) = compile_module(&src) else {
            panic!("expected `{body}` to be rejected");
        };
        assert_eq!(
            e.code,
            BpfDiag::OutOfSubset,
            "`{body}` should be OutOfSubset, not {:?}",
            e.code
        );
        assert!(
            e.msg.contains("concurrency is not supported"),
            "`{body}` message: {}",
            e.msg
        );
    }
}
