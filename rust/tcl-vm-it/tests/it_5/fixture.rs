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

/// The string protocol `vm`'s release selects.
pub fn strings(vm: &Vm) -> tcl_syntax::native_string::NativeStringProtocol {
    tcl_registry::InvocationDialect::of_static_profile(vm.dialect_profile())
        .native_string_protocol()
        .expect("a native string protocol")
}

/// `value`'s string bytes under `vm`'s release.
pub fn bytes(vm: &Vm, value: &tcl_vm::Value) -> Vec<u8> {
    value
        .native_string_bytes(strings(vm))
        .expect("native string bytes")
        .to_vec()
}

/// [`call`] with string words, returning the completion code and result bytes.
pub fn run(vm: &mut Vm, name: &str, words: &[&str]) -> (tcl_runtime_api::Code, Vec<u8>) {
    let words: Vec<_> = words
        .iter()
        .map(|word| tcl_vm::Value::string(*word))
        .collect();
    let completion = call(vm, name, &words);
    (completion.code, bytes(vm, &completion.result))
}

type Compiled = Rc<std::cell::RefCell<Vec<(bool, tcl_bytecode::FunctionAsm)>>>;

/// The release's own compiler, keeping each script and procedure body it
/// compiles, marked `true` for a procedure.
struct Recording {
    inner: tcl_codegen::compile_service::BytecodeCompileService,
    compiled: Compiled,
}

impl Recording {
    fn keep(
        &self,
        procedure: bool,
        module: Result<tcl_bytecode::ModuleAsm, tcl_runtime_api::CompileError>,
    ) -> Result<tcl_bytecode::ModuleAsm, tcl_runtime_api::CompileError> {
        if let Ok(module) = &module {
            self.compiled
                .borrow_mut()
                .push((procedure, module.top_level.clone()));
        }
        module
    }
}

macro_rules! delegate {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(fn $name(&self, $($arg: $ty),*) -> $ret {
            self.inner.$name($($arg),*)
        })*
    };
}

macro_rules! delegate_scripts {
    ($($name:ident($($arg:ident: $ty:ty),*);)*) => {
        $(fn $name(&self, $($arg: $ty),*) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            self.keep(false, self.inner.$name($($arg),*))
        })*
    };
}

impl tcl_runtime_api::CompileService for Recording {
    type Module = tcl_bytecode::ModuleAsm;

    delegate_scripts! {
        compile_substitution_with_entry(target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>, profile: &'static tcl_dialect::DialectProfile, entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>);
        compile_script_bytes_for_profile(target: tcl_runtime_api::ScriptCompileTargetBytes<'_>, profile: &'static tcl_dialect::DialectProfile);
        compile_plain_script_bytes_for_profile(target: tcl_runtime_api::ScriptCompileTargetBytes<'_>, profile: &'static tcl_dialect::DialectProfile);
        compile_script_bytes_with_entry(target: tcl_runtime_api::ScriptCompileTargetBytes<'_>, profile: &'static tcl_dialect::DialectProfile, entry: &tcl_runtime_api::NativeCompilationEntry);
        compile_plain_script_bytes_with_entry(target: tcl_runtime_api::ScriptCompileTargetBytes<'_>, profile: &'static tcl_dialect::DialectProfile, entry: &tcl_runtime_api::NativeCompilationEntry);
        compile(src: &str);
        compile_for_profile(src: &str, profile: &'static tcl_dialect::DialectProfile);
        compile_script_for_profile(target: tcl_runtime_api::ScriptCompileTarget<'_>, profile: &'static tcl_dialect::DialectProfile);
        compile_traced(src: &str);
        compile_traced_for_profile(src: &str, profile: &'static tcl_dialect::DialectProfile);
        compile_plain_dispatch_for_profile(src: &str, profile: &'static tcl_dialect::DialectProfile);
        compile_plain_script_for_profile(target: tcl_runtime_api::ScriptCompileTarget<'_>, profile: &'static tcl_dialect::DialectProfile);
        compile_script_with_entry(target: tcl_runtime_api::ScriptCompileTarget<'_>, profile: &'static tcl_dialect::DialectProfile, entry: &tcl_runtime_api::NativeCompilationEntry);
    }

    delegate! {
        script_command_plan_bytes_for_profile(source: &tcl_lexer::SourceImage, profile: &'static tcl_dialect::DialectProfile) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError>;
        script_command_plan_bytes_with_entry(source: &tcl_lexer::SourceImage, profile: &'static tcl_dialect::DialectProfile, entry: &tcl_runtime_api::NativeCompilationEntry) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError>;
        script_command_plan_for_profile(src: &str, profile: &'static tcl_dialect::DialectProfile) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError>;
        compile_failure_for_profile(src: &str, profile: &'static tcl_dialect::DialectProfile, unit: tcl_runtime_api::CompileUnit) -> Option<tcl_runtime_api::CompileFailure>;
    }

    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.keep(
            true,
            self.inner
                .compile_procedure_bytes_for_profile(target, profile, dispatch),
        )
    }

    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.keep(
            true,
            self.inner
                .compile_procedure_bytes_with_entry(target, profile, entry, dispatch),
        )
    }

    fn compile_original_procedure_bytes_with_entry(
        &self,
        request: &tcl_runtime_api::native_procedure_compilation::OriginalProcedureCompileRequest,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.keep(
            true,
            self.inner
                .compile_original_procedure_bytes_with_entry(request, profile, entry, dispatch),
        )
    }

    fn compile_procedure_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.keep(
            true,
            self.inner
                .compile_procedure_with_entry(target, profile, entry, dispatch),
        )
    }

    fn compile_procedure_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.keep(
            true,
            self.inner
                .compile_procedure_for_profile(target, profile, dispatch),
        )
    }
}

/// The body `profile`'s compiler produces when the VM first calls
/// `proc name params body`, the first procedure it compiles, captured
/// through the public compiler seam.
pub fn procedure_body(
    profile: &'static tcl_dialect::DialectProfile,
    name: &str,
    params: &str,
    body: &str,
) -> tcl_bytecode::FunctionAsm {
    let (mut vm, compiled) = recording(profile);
    let words = [name, params, body].map(tcl_vm::Value::string);
    assert_eq!(
        call(&mut vm, "proc", &words).code,
        tcl_runtime_api::Code::Ok
    );
    let arguments: Vec<_> = params
        .split_whitespace()
        .map(|_| tcl_vm::Value::string("1"))
        .collect();
    let _ = vm.try_invoke_command(name, &arguments);
    first(&compiled, true).unwrap_or_else(|| panic!("{name}: a compiled procedure body"))
}

fn recording(profile: &'static tcl_dialect::DialectProfile) -> (Vm, Compiled) {
    let mut vm = core(profile);
    let compiled = Rc::new(std::cell::RefCell::new(Vec::new()));
    vm.set_compiler(Box::new(Recording {
        inner: tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
        compiled: Rc::clone(&compiled),
    }));
    (vm, compiled)
}

fn first(compiled: &Compiled, procedure: bool) -> Option<tcl_bytecode::FunctionAsm> {
    let mut compiled = compiled.borrow_mut();
    let index = compiled.iter().position(|(kind, _)| *kind == procedure)?;
    Some(compiled.swap_remove(index).1)
}

/// The top level `profile`'s compiler produces when the VM evaluates
/// `script`, the first script it compiles.
pub fn script_body(
    profile: &'static tcl_dialect::DialectProfile,
    script: &str,
) -> tcl_bytecode::FunctionAsm {
    let (mut vm, compiled) = recording(profile);
    let _ = vm.try_eval_source(script);
    first(&compiled, false).unwrap_or_else(|| panic!("{script}: a compiled script"))
}
