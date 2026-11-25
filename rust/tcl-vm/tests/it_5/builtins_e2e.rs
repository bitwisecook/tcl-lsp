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

//! End-to-end coverage of the builtin command library: list/string/dict/array,
//! namespace, `info`, regexp/regsub, expr, binary/scan, channels, traces, and
//! friends — each compiled as real Tcl via `tcl-compiler` and run through
//! `tcl-vm`, asserting observable behaviour.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;
use std::rc::Rc;

use tcl_codegen::codegen::codegen_module;
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

mod control_flow_and_errors;
mod host_and_io;
mod introspection;
mod lists_arrays_dicts;
mod namespaces_and_variables;
mod option_words;
mod packages;
mod strings_and_substitution;

#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The suite's release: the default VM's Tcl 9.0.
const RELEASE: tcl_dialect::TclVersion = tcl_dialect::TclVersion::V9_0;

fn run(src: &str) -> (bool, String, String) {
    // A unit compiled without a Tcl version selects no native plan, so the
    // script is compiled for the release it runs on.
    let profile = tcl_registry::model::ingress::resolve_environment(RELEASE.dialect_profile_name())
        .analyser_profile();
    let service = BytecodeCompileService::for_profile(profile);
    let asm = tcl_vm::CompileService::compile_for_profile(&service, src, profile)
        .expect("test script compiles for its selected profile");
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm =
        tcl_vm::test_support::native_vm_for_release(RELEASE, Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(service));
    let c = vm.run_module(&asm);
    let out = String::from_utf8(buf.borrow().clone()).expect("utf-8");
    (c.code.is_ok(), c.result.to_str().to_string(), out)
}

/// Assert a script runs OK and prints `expected`.
fn out_eq(src: &str, expected: &str) {
    let (ok, result, out) = run(src);
    assert!(ok, "script errored: {result}");
    assert_eq!(out, expected, "for script:\n{src}");
}
