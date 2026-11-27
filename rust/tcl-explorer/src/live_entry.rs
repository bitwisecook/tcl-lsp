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

//! Procedure disassembly through a live interpreter's compilation entry.
//!
//! Native command compilers (`catch`, `info exists`, `expr` in a command
//! word, `if`) are admitted only through a live interpreter entry, so static
//! codegen alone shows them as generic invocations. This recompiles each
//! procedure body of a static module through a fresh interpreter, as C's
//! `tcl::unsupported::disassemble proc` does: nothing runs, and each procedure
//! is defined from its static declaration and compiled as on its first call.
//! The top level keeps its static disassembly, and so does a body the
//! interpreter cannot compile.

use tcl_codegen::codegen::ModuleAsm;
use tcl_compiler::ir::Module;

/// Replace the procedure bodies of `module`, compiled statically from `ir`,
/// with the bytecode a fresh interpreter for `ir`'s Tcl release compiles for
/// them. Any other dialect leaves `module` unchanged.
pub fn recompile_through_live_entry(ir: &Module, module: &mut ModuleAsm) {
    let Some(mut vm) = ir.dialect_profile.and_then(live_interpreter) else {
        return;
    };
    for (id, asm) in &mut module.procedures {
        let Some(procedure) = ir.procedures.get(id) else {
            continue;
        };
        let Some(body) = procedure.body_source.as_deref() else {
            continue;
        };
        let name = procedure.qualified_name.as_str();
        if !ensure_namespace(&mut vm, name)
            || vm
                .try_define_procedure(name, &[procedure.params_raw.as_str()], body)
                .is_err()
        {
            continue;
        }
        if let Ok(live) = vm.procedure_bytecode(name) {
            let mut live = (*live).clone();
            live.name.clone_from(&asm.name);
            *asm = live;
        }
    }
}

/// A fresh interpreter for `dialect` with its compile service installed, or
/// `None` unless `dialect` is a plain Tcl release, the only family whose
/// compiler tclsh's disassembly is the reference for.
fn live_interpreter(dialect: &'static tcl_dialect::DialectProfile) -> Option<tcl_vm::Vm> {
    if tcl_native_model::InvocationDialect::of_profile(dialect).family()
        != Some(tcl_dialect::model::Family::Tcl)
    {
        return None;
    }
    let profile = tcl_registry::model::resolve_environment(dialect.name).unit_profile();
    let mut vm = tcl_vm::Vm::with_native_core(
        Box::new(std::io::sink()),
        std::rc::Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )?;
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile),
    ));
    Some(vm)
}

/// Create the namespace `name` is qualified into. Only a plain path is
/// created; any other name keeps its static disassembly.
fn ensure_namespace(vm: &mut tcl_vm::Vm, name: &str) -> bool {
    let (namespace, _) = tcl_syntax::naming::key_holder_and_tail(name);
    if namespace.is_empty() || namespace == "::" {
        return true;
    }
    if !namespace
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
    {
        return false;
    }
    vm.try_compile_function(&format!("namespace eval {namespace} {{}}"))
        .is_ok_and(|handle| vm.invoke_function(&handle).code == tcl_vm::Code::Ok)
}
