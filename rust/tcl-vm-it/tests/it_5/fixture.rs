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

//! Interpreters over an explicit release's native core, built through
//! `tcl-vm`'s public constructor, with or without that release's source
//! compiler.

use std::io::Write;
use std::rc::Rc;

use tcl_vm::{Vm, host_native::NativeHost};

/// A native core of `profile` with no compiler, writing output nowhere.
pub fn core(profile: &'static tcl_dialect::DialectProfile) -> Vm {
    core_with_output(profile, Box::new(std::io::sink()))
}

/// A native core of `profile` with no compiler, writing output to `output`.
pub fn core_with_output(
    profile: &'static tcl_dialect::DialectProfile,
    output: Box<dyn Write>,
) -> Vm {
    core_with_output_and_host(profile, output, Rc::new(NativeHost::new()))
}

fn core_with_output_and_host(
    profile: &'static tcl_dialect::DialectProfile,
    output: Box<dyn Write>,
    host: Rc<dyn tcl_platform::Host>,
) -> Vm {
    Vm::with_native_core(
        output,
        host,
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("an actual native core for the profile")
}

fn with_compiler(mut vm: Vm, profile: &'static tcl_dialect::DialectProfile) -> Vm {
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    vm
}

/// [`core`] with `profile`'s source compiler installed.
pub fn interpreter(profile: &'static tcl_dialect::DialectProfile) -> Vm {
    interpreter_with_output(profile, Box::new(std::io::sink()))
}

/// [`core_with_output`] with `profile`'s source compiler installed.
pub fn interpreter_with_output(
    profile: &'static tcl_dialect::DialectProfile,
    output: Box<dyn Write>,
) -> Vm {
    with_compiler(core_with_output(profile, output), profile)
}

/// [`interpreter`] with every scripted distribution library loaded, as the
/// release's own shell starts.
pub fn distribution(profile: &'static tcl_dialect::DialectProfile) -> Vm {
    distribution_with_output(profile, Box::new(std::io::sink()))
}

/// [`distribution`] writing output to `output`.
pub fn distribution_with_output(
    profile: &'static tcl_dialect::DialectProfile,
    output: Box<dyn Write>,
) -> Vm {
    with_compiler(
        tcl_vm::test_support::native_vm_for_profile(profile, output),
        profile,
    )
}

struct EntryCapture(Rc<std::cell::RefCell<Option<tcl_runtime_api::NativeCompilationEntry>>>);

impl tcl_runtime_api::CompileService for EntryCapture {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, _: &str) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        Err(tcl_runtime_api::CompileError::Unsupported(
            "entry capture only".into(),
        ))
    }

    fn compile_script_with_entry(
        &self,
        _: tcl_runtime_api::ScriptCompileTarget<'_>,
        _: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        *self.0.borrow_mut() = Some(entry.clone());
        self.compile("")
    }
}

/// The compilation entry `vm` hands its compiler for a global-level script,
/// captured through the public compiler seam. The release's own
/// compiler is installed again afterwards.
pub fn root_compilation_entry(vm: &mut Vm) -> tcl_runtime_api::NativeCompilationEntry {
    let captured = Rc::new(std::cell::RefCell::new(None));
    vm.set_compiler(Box::new(EntryCapture(Rc::clone(&captured))));
    assert!(vm.try_compile_function("list").is_err());
    let profile = vm.dialect_profile();
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    captured
        .take()
        .expect("a global-level script reaches the compiler with its entry")
}

/// Bytes from the hex text the tclsh and jimsh captures record.
pub fn unhex(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

/// Invoke `name` with `arguments` as object words, as a host does; a host
/// refusal fails the test.
pub fn call(
    vm: &mut Vm,
    name: &str,
    arguments: &[tcl_vm::Value],
) -> tcl_runtime_api::Completion<tcl_vm::Value> {
    vm.try_invoke_command(name, arguments)
        .unwrap_or_else(|refusal| panic!("{name}: {refusal:?}"))
}

/// `package` invoked with object words.
pub fn package(
    vm: &mut Vm,
    arguments: &[tcl_vm::Value],
) -> tcl_runtime_api::Completion<tcl_vm::Value> {
    call(vm, "package", arguments)
}

/// The value of `key` in a return-options dictionary.
pub fn option(options: &tcl_vm::Value, key: &str) -> Option<tcl_vm::Value> {
    options
        .as_list()
        .ok()?
        .chunks(2)
        .find(|pair| pair[0].string_bytes().as_ref() == key.as_bytes())
        .map(|pair| pair[1].clone())
}

/// `package` invoked with object words, keeping a host refusal visible.
pub fn try_package(
    vm: &mut Vm,
    arguments: &[tcl_vm::Value],
) -> Result<tcl_runtime_api::Completion<tcl_vm::Value>, tcl_runtime_api::NativeExecutionError> {
    vm.try_invoke_command("package", arguments)
}

/// Evaluate `script` as a host does; a host refusal fails the test.
pub fn eval(vm: &mut Vm, script: &str) -> tcl_runtime_api::Completion<tcl_vm::Value> {
    vm.try_eval_source(script)
        .unwrap_or_else(|refusal| panic!("{script}: {refusal:?}"))
}
