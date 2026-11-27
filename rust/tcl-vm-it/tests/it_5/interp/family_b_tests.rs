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

//! The Family-B contract the VM satisfies, through its public API.

use crate::prelude::*;
use tcl_runtime_api::guard::{GuardIdentity, GuardToken};

const GUARDED_IDENTITY: GuardIdentity = GuardIdentity::new(1, 41);

fn release_profile(name: &str) -> &'static tcl_dialect::DialectProfile {
    tcl_vm::test_support::profile(name)
}

/// The release's own engine and source compiler.
fn release_vm(version: tcl_dialect::TclVersion) -> Vm {
    crate::fixture::interpreter(release_profile(version.dialect_profile_name()))
}

/// Re-select both the actual engine and the logical profile of `name`.
fn select_release(vm: &mut Vm, name: &str) {
    let profile = release_profile(name);
    assert!(vm.set_native_engine_profile(profile), "{name}");
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
}

/// [`select_release`] for the child `child`.
fn select_child_release(vm: &mut Vm, child: &str, name: &str) {
    let profile = release_profile(name);
    assert!(vm.set_child_native_engine_profile(child, profile), "{name}");
    assert!(vm.set_child_dialect_profile(child, profile), "{name}");
}

fn compiled_vm() -> Vm {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::default(),
    ));
    vm
}

fn eval_value(vm: &mut Vm, script: &str) -> String {
    let completion = vm.eval_source(script).expect("script compiles");
    assert_eq!(completion.code, Code::Ok, "{script}: {completion:?}");
    completion.result.to_str().to_string()
}

fn guarded_builtin(_vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    Completion::new(Code::Ok, Value::empty(), Value::empty())
}

thread_local! {
    static PROBED_TOKEN: std::cell::Cell<Option<GuardToken>> =
        const { std::cell::Cell::new(None) };
}

mod hidden_commands;
mod host_activation;
mod intrinsic_guards;
mod variables;
