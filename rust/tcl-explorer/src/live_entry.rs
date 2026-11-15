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

//! Disassembly through a live interpreter's compilation entry.
//!
//! Native command compilers (`catch`, `info exists`, `expr` in a command
//! word, `if`) are admitted only through a live interpreter entry, so static
//! codegen alone shows them as generic invocations. This recompiles each body
//! of a static module through a fresh interpreter, as C's
//! `tcl::unsupported::disassemble` does: the script is compiled, never run,
//! and each procedure is defined from its static declaration and compiled as
//! on its first call. A body the interpreter cannot compile keeps its static
//! disassembly.

use tcl_codegen::codegen::ModuleAsm;
use tcl_compiler::ir::Module;

/// Replace the bodies of `module`, compiled statically from `ir`, with the
/// bytecode a fresh interpreter for `ir`'s dialect compiles for them. A dialect
/// with no native core leaves `module` unchanged.
pub fn recompile_through_live_entry(source: &str, ir: &Module, module: &mut ModuleAsm) {
    let Some(mut vm) = ir.dialect_profile.and_then(live_interpreter) else {
        return;
    };
    if let Ok(handle) = vm.try_compile_function(source) {
        let mut top = (*handle.bytecode()).clone();
        top.name.clone_from(&module.top_level.name);
        module.top_level = top;
    }
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
/// `None` for a dialect with no native core.
fn live_interpreter(dialect: &'static tcl_dialect::DialectProfile) -> Option<tcl_vm::Vm> {
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
    let Some((namespace, _)) = name.rsplit_once("::") else {
        return true;
    };
    if namespace.is_empty() {
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
