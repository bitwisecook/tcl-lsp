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

//! The capability seam, proven with real hosts over the VM's real `ValueOps`.
//!
//! The same `tcl-cmd-core::platform` command body runs against a fully-capable
//! `NativeHost` and a sandboxed one (subprocess capability off — the posture a
//! WASM/WASI host has). The native host executes; the sandboxed host yields the
//! faithful "unsupported" error rather than panicking — the native-vs-WASM
//! divergence the architecture is built around.

use tcl_cmd_core::platform;
use tcl_vm::host_native::NativeHost;
use tcl_vm::{Value, Vm};

#[test]
fn exec_runs_on_native_host() {
    let mut vm = Vm::new();
    let host = NativeHost::new();
    let args = [Value::string("echo"), Value::string("hello")];
    let result = platform::exec(&mut vm, &host, &args).expect("native host should run exec");
    assert_eq!(&*result.to_str(), "hello");
}

#[test]
fn exec_unsupported_on_sandboxed_host() {
    let mut vm = Vm::new();
    // The conditional `process` capability is off — exactly what `Host::process`
    // returns `None` for on every WASM target.
    let host = NativeHost::sandboxed();
    let args = [Value::string("echo"), Value::string("hello")];
    let err =
        platform::exec(&mut vm, &host, &args).expect_err("a sandboxed host has no subprocess");
    assert!(
        err.message().contains("no subprocess support"),
        "expected the faithful unsupported error, got: {}",
        err.message()
    );
}

#[test]
fn pwd_reads_working_directory() {
    let mut vm = Vm::new();
    let host = NativeHost::new();
    let result = platform::pwd(&mut vm, &host).expect("pwd should resolve");
    // An absolute path on the native (unix) host.
    assert!(result.to_str().starts_with('/'), "got: {}", result.to_str());
}
