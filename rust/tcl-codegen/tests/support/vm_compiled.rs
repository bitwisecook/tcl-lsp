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

//! Bytecode a live interpreter of a named release asks the compile service
//! for. A module lowered without an interpreter's compilation entry proves no
//! command and emits plain dispatch, so a test of what a release compiles
//! takes its unit from the request the interpreter makes.

use std::cell::RefCell;
use std::rc::Rc;
use tcl_bytecode::{FunctionAsm, ModuleAsm};
use tcl_codegen::compile_service::BytecodeCompileService;
use tcl_runtime_api::{
    CompileError, CompileService, NativeCompilationEntry, OriginalProcedureCompileRequest,
    ProcedureCompileTarget, ProcedureCompileTargetBytes, ProcedureDispatch, ScriptCommandPlan,
    ScriptCompileTarget, ScriptCompileTargetBytes, SourceImage,
};

type Profile = &'static tcl_dialect::DialectProfile;
type Compiled = Result<ModuleAsm, CompileError>;

#[derive(Default)]
struct Units {
    scripts: Vec<ModuleAsm>,
    procedures: Vec<(String, ModuleAsm)>,
}

struct Capture {
    compiler: BytecodeCompileService,
    units: Rc<RefCell<Units>>,
}

impl Capture {
    fn script(&self, compiled: Compiled) -> Compiled {
        if let Ok(module) = &compiled {
            self.units.borrow_mut().scripts.push(module.clone());
        }
        compiled
    }
}

impl CompileService for Capture {
    type Module = ModuleAsm;

    fn compile(&self, source: &str) -> Compiled {
        self.script(self.compiler.compile(source))
    }

    fn compile_for_profile(&self, source: &str, profile: Profile) -> Compiled {
        self.script(self.compiler.compile_for_profile(source, profile))
    }

    fn compile_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: Profile,
    ) -> Compiled {
        self.script(self.compiler.compile_script_for_profile(target, profile))
    }

    fn compile_script_with_entry(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: Profile,
        entry: &NativeCompilationEntry,
    ) -> Compiled {
        self.script(
            self.compiler
                .compile_script_with_entry(target, profile, entry),
        )
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: Profile,
        entry: &NativeCompilationEntry,
    ) -> Compiled {
        self.script(
            self.compiler
                .compile_script_bytes_with_entry(target, profile, entry),
        )
    }

    fn compile_plain_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: Profile,
        entry: &NativeCompilationEntry,
    ) -> Compiled {
        self.compiler
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }

    fn compile_procedure_with_entry(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: Profile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Compiled {
        self.compiler
            .compile_procedure_with_entry(target, profile, entry, dispatch)
    }

    fn compile_procedure_bytes_with_entry(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: Profile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Compiled {
        self.compiler
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
    }

    fn compile_original_procedure_bytes_with_entry(
        &self,
        request: &OriginalProcedureCompileRequest,
        profile: Profile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Compiled {
        let compiled = self
            .compiler
            .compile_original_procedure_bytes_with_entry(request, profile, entry, dispatch);
        if let Ok(module) = &compiled {
            self.units
                .borrow_mut()
                .procedures
                .push((request.reported_name().to_owned(), module.clone()));
        }
        compiled
    }

    fn script_command_plan_bytes_with_entry(
        &self,
        source: &SourceImage,
        profile: Profile,
        entry: &NativeCompilationEntry,
    ) -> Result<ScriptCommandPlan, CompileError> {
        self.compiler
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile_substitution_with_entry(
        &self,
        target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
        profile: Profile,
        entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Compiled {
        self.compiler
            .compile_substitution_with_entry(target, profile, entry)
    }
}

/// An interpreter of one release that keeps every unit it compiles.
pub struct Interpreter {
    vm: tcl_vm::Vm,
    units: Rc<RefCell<Units>>,
}

impl Interpreter {
    /// A fresh interpreter of `release` (`tcl8.6`, `tcl9.0`, …).
    pub fn of(release: &str) -> Self {
        let profile = tcl_dialect::DialectProfile::find(release)
            .unwrap_or_else(|| panic!("{release} is a catalogued release"));
        let units = Rc::new(RefCell::new(Units::default()));
        let mut vm = tcl_vm::Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(tcl_vm::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs {
                package_path: Vec::new(),
                default_library: None,
            },
        )
        .unwrap_or_else(|| panic!("{release} core registration"));
        vm.set_compiler(Box::new(Capture {
            compiler: BytecodeCompileService::for_profile(profile),
            units: Rc::clone(&units),
        }));
        Self { vm, units }
    }

    /// Evaluate `source` and return the unit compiled for it. The evaluation
    /// may fail; the unit is what the interpreter compiled before running it.
    pub fn script(&mut self, source: &str) -> ModuleAsm {
        let before = self.units.borrow().scripts.len();
        let _ = self.vm.try_eval_source(source);
        self.units
            .borrow()
            .scripts
            .get(before)
            .cloned()
            .unwrap_or_else(|| panic!("no unit compiled for {source:?}"))
    }

    /// The body unit of procedure `name`, compiling it by a call when no call
    /// has compiled it yet. The call's own outcome is not read.
    pub fn procedure(&mut self, name: &str) -> ModuleAsm {
        if self.compiled(name).is_none() {
            let _ = self.vm.try_eval_source(&format!(
                "{name} {{*}}[lrepeat [llength [info args {name}]] 0]"
            ));
        }
        self.compiled(name)
            .unwrap_or_else(|| panic!("procedure {name} was not compiled"))
    }

    fn compiled(&self, name: &str) -> Option<ModuleAsm> {
        let tail = tcl_syntax::naming::written_command_tail(name.as_bytes());
        self.units
            .borrow()
            .procedures
            .iter()
            .rev()
            .find(|(reported, _)| {
                tcl_syntax::naming::written_command_tail(reported.as_bytes()) == tail
            })
            .map(|(_, module)| module.clone())
    }
}

/// The top-level function `release` compiles for `source`.
pub fn top_level(release: &str, source: &str) -> FunctionAsm {
    Interpreter::of(release).script(source).top_level
}

/// The body function `release` compiles for procedure `name` of `source`.
pub fn procedure(release: &str, source: &str, name: &str) -> FunctionAsm {
    let mut interpreter = Interpreter::of(release);
    let _ = interpreter.vm.try_eval_source(source);
    interpreter.procedure(name).top_level
}
