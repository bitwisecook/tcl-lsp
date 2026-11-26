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

//! `namespace` command-surface regression vectors.
//!
//! Every expectation is pinned byte-for-byte against tclsh 8.6.16 *and*
//! tclsh 9.0.4 unless a `9.0:` comment records a deliberate release
//! difference. The governing C functions are named per group.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_vm::CompileService;

mod command_delete_traces;
mod command_visibility;
mod ensembles;
mod export_import;
mod retained_namespaces;
mod retained_tokens;
mod teardown;
mod which_origin;

/// A `Write` sink backed by a shared buffer the test can read afterwards.
#[derive(Clone, Default)]
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

/// Compile and run `src` on a VM pinned to `release`; return the result
/// string (an uncaught error leaves its message there), or the compile
/// rejection.
fn run_at(src: &str, release: &str) -> String {
    let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(Capture::default()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm.try_eval_source(src).expect("the VM's own compile entry");
    let original = vm
        .capture_original_operand_projection()
        .expect("the actual VM result projection");
    let text = vm
        .original_operand_unicode(&original, &completion.result)
        .expect("the original result Unicode worker");
    drop(completion);
    vm.check_original_operand_projection(&original)
        .expect("result projection current after completion retirement");
    vm.finish_original_operand_projection(original)
        .expect("result projection settlement");
    text
}

/// [`run_at`] at the VM's default release (9.0), for the vectors tclsh 8.6.16
/// and 9.0.4 agree on.
fn run(src: &str) -> String {
    run_at(src, "tcl9.0")
}

/// Run each `(release, script)` step on **one** VM, re-pinning its profile
/// between steps; the last step's result is returned. Real tclsh cannot change
/// release mid-session, so this exists only for invariants about what the
/// emulated surface must do when an embedder re-pins it.
fn run_flipping(steps: &[(&str, &str)]) -> String {
    let mut vm = tcl_vm::test_support::native_vm_with_output(Box::new(Capture::default()));
    let mut last = String::new();
    for (release, src) in steps {
        let profile = tcl_registry::model::ingress::resolve_environment(release).analyser_profile();
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
        last = vm
            .try_eval_source(src)
            .expect("the VM's own compile entry")
            .result
            .to_str()
            .to_string();
    }
    last
}

#[test]
fn standalone_compound_projection_does_not_acquire_the_vm_result_scope() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(profile, Box::new(Capture::default()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let completion = vm
        .try_eval_source(
            r"set log {}
             proc rec {old new op} {lappend ::log [list $old $op]}
             namespace eval N {
                 proc q {} {return [namespace current]}
                 proc p {} {namespace delete ::N; set a [q]; set b $::log; list $a $b}
             }
             trace add command ::N::q delete rec
             set a [::N::p]
             list $a $log",
        )
        .expect("the same actual VM source entry");
    assert_eq!(
        completion
            .result
            .native_string_bytes(tcl_syntax::native_string::NativeStringProtocol::C(
                tcl_dialect::TclVersion::V9_0
            )),
        Err(tcl_syntax::native_string::NativeStringUnavailable::StringUpdater)
    );
}
