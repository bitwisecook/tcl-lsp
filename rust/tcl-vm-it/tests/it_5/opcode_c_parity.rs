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

//! Per-opcode parity tests for the variable/array/dict/introspection,
//! coroutine and `TclOO` instructions the compiler does not (yet) emit, plus
//! the iRules dialect operators it does.
//!
//! Each test hand-assembles a [`FunctionAsm`] — the artifact `tcl-compiler`'s
//! `codegen_module` produces — and runs it through [`Vm::run_function`], so the
//! opcode under test is exercised directly rather than through whatever
//! sequence codegen happens to choose. Expectations are pinned to
//! `tclExecute.c`'s `TEBCresume` cases (stack order, result value, error
//! message) for Tcl 9.0.4; the iRules operators, which have no C counterpart,
//! are pinned against the core commands they are defined in terms of.
//!
//! The opcodes that only mean something *inside* a coroutine or a method call
//! (`yield`, `yieldToInvoke`, `tclooSelf`, `tclooNext`, …) run through
//! [`vm_with_seeded_proc`], which installs a hand-built stream as a real proc's
//! compiled body so a `coroutine`/method call reaches it.

use std::collections::HashMap;

use tcl_bytecode::{
    FunctionAsm, Instruction, LiteralTable, LocalVarTable, Op, Operand, bytecode_imm, layout,
};
use tcl_vm::{Code, Completion, Value, Vm};

mod context_queries;
mod coroutines_and_tcloo;
mod expr_and_return;
mod irules_operators;
mod lists_and_dicts;
mod strings;
mod variable_access;
mod variable_state;

/// A hand-built instruction stream plus its literal pool and LVT.
#[derive(Default)]
struct Asm {
    lits: LiteralTable,
    lvt: LocalVarTable,
    instrs: Vec<Instruction>,
}

impl Asm {
    fn new() -> Self {
        Self::default()
    }

    /// Push a literal string. Marked verbatim (as codegen marks braced words)
    /// so the VM pushes it exactly as interned instead of re-substituting it.
    fn push(&mut self, text: &str) -> &mut Self {
        let idx = self.lits.intern(text);
        let mut instr = Instruction::new(Op::PUSH1, vec![Operand::Imm(bytecode_imm(idx))]);
        instr.literal_rep = tcl_bytecode::LiteralRep::Verbatim;
        self.instrs.push(instr);
        self
    }

    /// Intern `name` in the LVT and return its slot, for an LVT-operand opcode.
    fn slot(&mut self, name: &str) -> i32 {
        bytecode_imm(self.lvt.intern(name))
    }

    /// Emit `op` with the given operands.
    fn op(&mut self, op: Op, operands: &[i32]) -> &mut Self {
        self.instrs.push(Instruction::new(
            op,
            operands.iter().copied().map(Operand::Imm).collect(),
        ));
        self
    }

    /// The stream behind C's `global` prologue (`push ::`, then `push name;
    /// nsupvar slot` per variable, `pop`) for every slot the stream does not
    /// link itself.
    fn global_linked_body(self) -> FunctionAsm {
        let linked: Vec<i32> = self
            .instrs
            .iter()
            .filter(|instr| matches!(instr.op, Op::VARIABLE | Op::UPVAR | Op::NSUPVAR))
            .filter_map(|instr| match instr.operands.first() {
                Some(Operand::Imm(slot)) => Some(*slot),
                _ => None,
            })
            .collect();
        let mut prologue = Asm {
            lits: self.lits,
            lvt: self.lvt,
            instrs: Vec::new(),
        };
        prologue.push("::");
        for (slot, name) in prologue.lvt.native_slot_names().into_iter().enumerate() {
            let slot = bytecode_imm(slot);
            if let Some(name) = name
                && !linked.contains(&slot)
            {
                let text = String::from_utf8(name.as_bytes().to_vec()).expect("test variable name");
                prologue.push(&text).op(Op::NSUPVAR, &[slot]);
            }
        }
        prologue.op(Op::POP, &[]);
        prologue.instrs.extend(self.instrs);
        prologue.build()
    }

    /// Finish the function, assigning byte offsets the way `codegen_module`
    /// does (`resolve_layout`) so the executor's offset→index map is valid.
    fn build(mut self) -> FunctionAsm {
        let labels = HashMap::new();
        let resolved = layout::resolve_layout(&mut self.instrs, &labels);
        FunctionAsm {
            name: "test".into(),
            literals: self.lits,
            lvt: self.lvt,
            instructions: self.instrs,
            labels: resolved,
            loop_targets: HashMap::new(),
            body_base_line: 0,
            native_compilation_failure: None,
            native_compilation_preflight: tcl_runtime_api::NativeCompilationPreflight::NotRequired,
            native_math_table_prerequisite: None,
            native_compiler_prerequisites: Vec::new(),
            required_compiled_local_layout: None,
            proc_body_src: None,
            error_regions: Vec::new(),
            splice_ranges: Vec::new(),
            plain_command_dispatch: false,
            command_bindings: Vec::new(),
            procedure_bindings: Vec::new(),
            site_claims: Vec::new(),
        }
    }
}

/// Run a hand-built function on `vm`.
fn run(vm: &mut Vm, asm: Asm) -> Completion<Value> {
    if asm.lvt.is_empty() {
        return vm.run_function(&asm.build());
    }
    // C keeps LVT slots only in a procedure frame, so an LVT stream runs as a proc body whose slots `global` links.
    vm.set_compiler(Box::new(compiler::seeded_proc(asm.global_linked_body())));
    vm.eval_source(compiler::SEEDED_PROC_DEF)
        .expect("proc defines");
    vm.eval_source("p").expect("compiles")
}

/// Run a hand-built function on a fresh `Vm`, returning `(vm, completion)`.
fn run_fresh(asm: Asm) -> (Vm, Completion<Value>) {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    let c = run(&mut vm, asm);
    (vm, c)
}

/// The result string of a completion that must have succeeded.
fn ok_str(c: &Completion<Value>) -> String {
    assert_eq!(
        c.code,
        Code::Ok,
        "expected success, got {}",
        c.result.to_str()
    );
    c.result.to_str().to_string()
}

/// The `-errorcode` an error completion publishes through its options dict
/// (what `catch … opts; dict get $opts -errorcode` reads back).
fn err_code(c: &Completion<Value>) -> String {
    let words = c.options.as_list().expect("options are a dict");
    words
        .windows(2)
        .find(|pair| &*pair[0].to_str() == "-errorcode")
        .map_or_else(String::new, |pair| pair[1].to_str().to_string())
}

/// The message of a completion that must have failed.
fn err_str(c: &Completion<Value>) -> String {
    assert_eq!(
        c.code,
        Code::Error,
        "expected an error, got {}",
        c.result.to_str()
    );
    c.result.to_str().to_string()
}

/// Evaluate `src`, asserting it compiled, and return its completion.
fn eval(vm: &mut Vm, src: &str) -> Completion<Value> {
    vm.eval_source(src).expect("compiles")
}

/// A `tcl-compiler`-backed compile service, so the tests that need real Tcl
/// setup (traces, `namespace import`, command dispatch) can `eval_source`.
mod compiler {
    use tcl_bytecode::{FunctionAsm, ModuleAsm};
    use tcl_codegen::compile_service::BytecodeCompileService;
    use tcl_dialect::DialectProfile;
    use tcl_runtime_api::{
        CompileError, CompileService, ProcedureCompileTarget, ProcedureDispatch,
    };

    pub const SEEDED_METHOD_BODY: &str = "string length native_opcode_body";

    pub const SEEDED_PROC_BODY: &str = "this body is replaced by the seeded one";

    pub const SEEDED_PROC_DEF: &str = "proc p {} {this body is replaced by the seeded one}";

    /// Test compiler that replaces exactly one compiler-produced procedure
    /// body with a hand-assembled opcode stream. Delegating every compilation
    /// mode to [`BytecodeCompileService`] keeps registry and dialect selection
    /// identical to production while making the replacement honest current-
    /// service bytecode at the VM provenance boundary.
    pub struct SeededProcCompileService {
        inner: BytecodeCompileService,
        body: FunctionAsm,
        consumed: std::cell::Cell<usize>,
    }

    /// A seed that never applies would let a seeded test pass vacuously.
    impl Drop for SeededProcCompileService {
        fn drop(&mut self) {
            if !std::thread::panicking() {
                assert_eq!(self.consumed.get(), 1, "the seeded body is compiled once");
            }
        }
    }

    impl SeededProcCompileService {
        fn inject(&self, src: &[u8], mut module: ModuleAsm) -> ModuleAsm {
            // C compiles a procedure body when it is first called, so the
            // seed replaces that compilation.
            if src == SEEDED_PROC_BODY.as_bytes() || src == SEEDED_METHOD_BODY.as_bytes() {
                let mut body = self.body.clone();
                body.plain_command_dispatch = module.plain_command_dispatch;
                body.command_bindings.clear();
                module.top_level = body;
                self.consumed.set(self.consumed.get() + 1);
            }
            module
        }
    }

    impl CompileService for SeededProcCompileService {
        type Module = ModuleAsm;
        fn compile_substitution_with_entry(
            &self,
            target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
            profile: &'static tcl_dialect::DialectProfile,
            entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            self.inner
                .compile_substitution_with_entry(target, profile, entry)
        }

        fn compile_script_bytes_for_profile(
            &self,
            target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
            profile: &'static tcl_dialect::DialectProfile,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            self.inner
                .compile_script_bytes_for_profile(target, profile)
                .map(|module| self.inject(target.source.bytes(), module))
        }
        fn compile_script_bytes_with_entry(
            &self,
            target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
            profile: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            self.inner
                .compile_script_bytes_with_entry(target, profile, entry)
                .map(|module| self.inject(target.source.bytes(), module))
        }
        fn compile_plain_script_bytes_for_profile(
            &self,
            target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
            profile: &'static tcl_dialect::DialectProfile,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            self.inner
                .compile_plain_script_bytes_for_profile(target, profile)
                .map(|module| self.inject(target.source.bytes(), module))
        }
        fn compile_plain_script_bytes_with_entry(
            &self,
            target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
            profile: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            self.inner
                .compile_plain_script_bytes_with_entry(target, profile, entry)
                .map(|module| self.inject(target.source.bytes(), module))
        }
        fn compile_procedure_bytes_for_profile(
            &self,
            target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
            profile: &'static tcl_dialect::DialectProfile,
            dispatch: tcl_runtime_api::ProcedureDispatch,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            self.inner
                .compile_procedure_bytes_for_profile(target, profile, dispatch)
                .map(|module| self.inject(target.source.bytes(), module))
        }
        fn compile_procedure_bytes_with_entry(
            &self,
            target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
            profile: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
            dispatch: tcl_runtime_api::ProcedureDispatch,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            self.inner
                .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
                .map(|module| self.inject(target.source.bytes(), module))
        }
        fn script_command_plan_bytes_for_profile(
            &self,
            source: &tcl_runtime_api::SourceImage,
            profile: &'static tcl_dialect::DialectProfile,
        ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
            self.inner
                .script_command_plan_bytes_for_profile(source, profile)
        }
        fn script_command_plan_bytes_with_entry(
            &self,
            source: &tcl_runtime_api::SourceImage,
            profile: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
            self.inner
                .script_command_plan_bytes_with_entry(source, profile, entry)
        }

        fn compile_script_with_entry(
            &self,
            target: tcl_runtime_api::ScriptCompileTarget<'_>,
            profile: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<Self::Module, tcl_vm::CompileError> {
            self.inner
                .compile_script_with_entry(target, profile, entry)
                .map(|module| self.inject(target.source.as_bytes(), module))
        }

        fn compile_procedure_with_entry(
            &self,
            target: tcl_runtime_api::ProcedureCompileTarget<'_>,
            profile: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
            dispatch: tcl_runtime_api::ProcedureDispatch,
        ) -> Result<Self::Module, tcl_vm::CompileError> {
            self.inner
                .compile_procedure_with_entry(target, profile, entry, dispatch)
                .map(|module| self.inject(target.source.as_bytes(), module))
        }

        fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
            self.inner
                .compile(src)
                .map(|module| self.inject(src.as_bytes(), module))
        }

        fn compile_for_profile(
            &self,
            src: &str,
            profile: &'static DialectProfile,
        ) -> Result<Self::Module, CompileError> {
            self.inner
                .compile_for_profile(src, profile)
                .map(|module| self.inject(src.as_bytes(), module))
        }

        fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
            self.inner
                .compile_traced(src)
                .map(|module| self.inject(src.as_bytes(), module))
        }

        fn compile_traced_for_profile(
            &self,
            src: &str,
            profile: &'static DialectProfile,
        ) -> Result<Self::Module, CompileError> {
            self.inner
                .compile_traced_for_profile(src, profile)
                .map(|module| self.inject(src.as_bytes(), module))
        }

        fn compile_plain_dispatch_for_profile(
            &self,
            src: &str,
            profile: &'static DialectProfile,
        ) -> Result<Self::Module, CompileError> {
            self.inner
                .compile_plain_dispatch_for_profile(src, profile)
                .map(|module| self.inject(src.as_bytes(), module))
        }

        fn compile_procedure_for_profile(
            &self,
            target: ProcedureCompileTarget<'_>,
            profile: &'static DialectProfile,
            dispatch: ProcedureDispatch,
        ) -> Result<Self::Module, CompileError> {
            self.inner
                .compile_procedure_for_profile(target, profile, dispatch)
                .map(|module| self.inject(target.source.as_bytes(), module))
        }
    }

    pub fn svc() -> BytecodeCompileService {
        BytecodeCompileService::default()
    }

    pub fn seeded_proc(body: FunctionAsm) -> SeededProcCompileService {
        SeededProcCompileService {
            inner: svc(),
            body,
            consumed: std::cell::Cell::new(0),
        }
    }
}
