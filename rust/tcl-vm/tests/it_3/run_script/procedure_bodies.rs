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

//! The body a procedure runs is the one the compiler emitted for its `proc` command, through a compile service that caches injected bodies.

use super::*;

// The pre-compiled proc-body cache.

/// Compiler wrapper that replaces one exact compiler-authored procedure body
/// while preserving the compiler's profile, namespace, source provenance, and
/// dispatch-mode metadata. This keeps the test artifact on the same admission
/// path as production bytecode instead of presenting it as a foreign module.
struct CachedBodyCompileService {
    inner: BytecodeCompileService,
    qname: String,
    body_src: String,
    body: tcl_bytecode::FunctionAsm,
    /// Splice this procedure call into the compiled body instead.
    splice: Option<&'static [&'static str]>,
}

impl CachedBodyCompileService {
    fn inject(&self, mut module: tcl_bytecode::ModuleAsm) -> tcl_bytecode::ModuleAsm {
        if self.splice.is_some() {
            return module;
        }
        let mut ids = module.procedures.keys();
        let Some(id) = ids.next().copied() else {
            return module;
        };
        if ids.next().is_some() {
            return module;
        }
        let matches = module
            .procedure_provenance
            .get(&id)
            .is_some_and(|provenance| {
                provenance.name == self.qname && provenance.body == self.body_src
            });
        if matches && let Some(original) = module.procedures.get(&id) {
            let mut body = self.body.clone();
            body.plain_command_dispatch = original.plain_command_dispatch;
            module.procedures.insert(id, body);
        }
        module
    }

    /// A body compiled on its procedure's first call (C's `TclProcCompileProc`)
    /// is the module's top level; the exact body word selects the cached one.
    fn inject_body(
        &self,
        body: &[u8],
        mut module: tcl_bytecode::ModuleAsm,
    ) -> tcl_bytecode::ModuleAsm {
        if body == self.body_src.as_bytes() {
            let mut cached = match self.splice {
                Some(inline) => spliced_caller(&module.top_level, inline),
                None => self.body.clone(),
            };
            cached.plain_command_dispatch = module.top_level.plain_command_dispatch;
            module.top_level_body = cached.clone();
            module.top_level = cached;
        }
        module
    }
}

impl CompileService for CachedBodyCompileService {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.inner
            .compile_script_bytes_for_profile(target, profile)
            .map(|module| self.inject(module))
    }
    fn compile_substitution_with_entry(
        &self,
        target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.inner
            .compile_substitution_with_entry(target, profile, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.inner
            .compile_script_bytes_with_entry(target, profile, entry)
            .map(|module| self.inject(module))
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.inner
            .compile_plain_script_bytes_for_profile(target, profile)
            .map(|module| self.inject(module))
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.inner
            .compile_plain_script_bytes_with_entry(target, profile, entry)
            .map(|module| self.inject(module))
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.inner
            .compile_procedure_bytes_for_profile(target, profile, dispatch)
            .map(|module| self.inject_body(target.source.bytes(), module))
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
            .map(|module| self.inject_body(target.source.bytes(), module))
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
            .map(|module| self.inject(module))
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
    }

    fn compile(&self, src: &str) -> Result<Self::Module, tcl_vm::CompileError> {
        self.inner.compile(src).map(|module| self.inject(module))
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        self.inner
            .compile_for_profile(src, profile)
            .map(|module| self.inject(module))
    }

    fn compile_script_for_profile(
        &self,
        target: tcl_vm::ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        self.inner
            .compile_script_for_profile(target, profile)
            .map(|module| self.inject(module))
    }

    fn compile_procedure_for_profile(
        &self,
        target: tcl_vm::ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_vm::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        self.inner
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

/// Build a VM whose pre-compiled body cache holds one entry: a hand-built
/// assembly returning `marker`, joined to the sole compiler-issued body ID and
/// admitted only alongside its exact compiler-authored `body_src` provenance.
fn vm_with_cached_body(qname: &str, body_src: &str, marker: &str) -> Vm {
    use tcl_bytecode::{FunctionAsm, Instruction, LiteralTable, LocalVarTable, Op};

    let mut literals = LiteralTable::new();
    literals.intern(marker);
    let mut instructions = vec![pushv(0), Instruction::new(Op::DONE, vec![])];
    let labels =
        tcl_bytecode::layout::resolve_layout(&mut instructions, &std::collections::HashMap::new());
    let cached = FunctionAsm {
        name: qname.to_owned(),
        literals,
        lvt: LocalVarTable::new(&[]),
        instructions,
        labels,
        loop_targets: std::collections::HashMap::new(),
        body_base_line: 0,
        proc_body_src: Some(body_src.to_owned()),
        error_regions: Vec::new(),
        splice_ranges: Vec::new(),
        ..FunctionAsm::default()
    };

    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(CachedBodyCompileService {
        inner: BytecodeCompileService::default(),
        qname: qname.to_owned(),
        body_src: body_src.to_owned(),
        body: cached,
        splice: None,
    }));
    vm
}

/// `proc` runs the body the compiler emitted for it.
///
/// An entered script carries no procedure bodies: as in C, a body is compiled
/// when its procedure is first called, and that compilation is what runs.
/// Both ordinary and qualified definitions run it.
#[test]
fn a_proc_runs_the_body_the_compiler_emitted_for_it() {
    let mut vm = vm_with_cached_body("::p", "return SOURCE", "PRECOMPILED");
    vm.eval_source("proc p {} {return SOURCE}").expect("define");
    assert_eq!(
        &*vm.eval_source("p").expect("call").result.to_str(),
        "PRECOMPILED"
    );

    let mut vm = vm_with_cached_body("::a::p", "return SOURCE", "PRECOMPILED");
    vm.eval_source("namespace eval a {}\nproc a::p {} {return SOURCE}")
        .expect("define");
    assert_eq!(
        &*vm.eval_source("a::p").expect("call").result.to_str(),
        "PRECOMPILED"
    );
}

/// The cached entry is taken only for the body word it was compiled from.
///
/// One name is reachable from several `proc` commands with different bodies,
/// and a compiled unit records only the first — so a name-only cache would run
/// a body the script never asked for. Matching the body word turns that into a
/// miss.
#[test]
fn a_proc_with_a_different_body_word_compiles_the_body_it_was_given() {
    let mut vm = vm_with_cached_body("::p", "return SOURCE", "PRECOMPILED");
    vm.eval_source("proc p {} {return OTHER}").expect("define");
    assert_eq!(
        &*vm.eval_source("p").expect("call").result.to_str(),
        "OTHER"
    );
}

/// The same rule, seen from Tcl: whichever `proc` command actually runs defines
/// the body that runs with it — the arm of an `if` the script took, the last
/// definition after a `rename`, the body a nested `proc` supplied.
///
/// tclsh 8.6.16 / 9.0.4 print `B` for each.
#[test]
fn the_proc_command_that_runs_is_the_one_whose_body_is_installed() {
    assert_eq!(
        run("if {0} { proc p {} {return A} } else { proc p {} {return B} }\nputs [p]").2,
        "B\n"
    );
    assert_eq!(
        run("eval {proc q {} {return A}}\nrename q {}\neval {proc q {} {return B}}\nputs [q]").2,
        "B\n"
    );
    assert_eq!(
        run("proc mk {} { proc r {} {return A} }\nmk\nrename r {}\n\
             proc mk2 {} { proc r {} {return B} }\nmk2\nputs [r]")
        .2,
        "B\n"
    );
}

// A spliced call leaves its body for the fallback invocation.

/// `caller`'s compiled body `vdouble [nextval]` with the call spliced: the
/// words go to anonymous locals, the spliced body runs `inline`, and the
/// fallback at the range's end invokes the words with the compiled call's own
/// instruction, as codegen lays a splice out.
fn spliced_caller(
    compiled: &tcl_bytecode::FunctionAsm,
    inline: &[&str],
) -> tcl_bytecode::FunctionAsm {
    use tcl_bytecode::{FunctionAsm, Instruction, LiteralTable, Op, Operand, SpliceRange};

    let invokes: Vec<_> = compiled
        .instructions
        .iter()
        .filter(|instruction| instruction.op == Op::INVOKE_STK1)
        .collect();
    let [next, call] = invokes.as_slice() else {
        panic!("vdouble [nextval] compiles to two invocations");
    };
    let mut literals = LiteralTable::new();
    let mut push = |text: &str| pushv(i32::try_from(literals.intern(text)).unwrap());
    let mut lvt = compiled.lvt.clone();
    let words = [lvt.intern_anonymous(), lvt.intern_anonymous()];
    let slot = |index: usize| Operand::Imm(i32::try_from(words[index]).unwrap());
    let mut instructions = vec![
        push("vdouble"),
        Instruction::new(Op::STORE_SCALAR1, vec![slot(0)]),
        Instruction::new(Op::POP, vec![]),
        push("nextval"),
        (*next).clone(),
        Instruction::new(Op::STORE_SCALAR1, vec![slot(1)]),
        Instruction::new(Op::POP, vec![]),
    ];
    let mut labels = std::collections::HashMap::new();
    labels.insert("splice_start".to_owned(), instructions.len());
    for word in inline {
        instructions.push(push(word));
    }
    instructions.push(Instruction::new(
        Op::INVOKE_STK1,
        vec![Operand::Imm(i32::try_from(inline.len()).unwrap())],
    ));
    instructions.push(Instruction::new(
        Op::JUMP4,
        vec![Operand::Label("splice_join".to_owned())],
    ));
    labels.insert("splice_end".to_owned(), instructions.len());
    instructions.push(Instruction::new(Op::LOAD_SCALAR1, vec![slot(0)]));
    instructions.push(Instruction::new(Op::LOAD_SCALAR1, vec![slot(1)]));
    instructions.push((*call).clone());
    labels.insert("splice_join".to_owned(), instructions.len());
    instructions.push(Instruction::new(Op::DONE, vec![]));
    let labels = tcl_bytecode::layout::resolve_layout(&mut instructions, &labels);
    let spliced = FunctionAsm {
        name: compiled.name.clone(),
        literals,
        lvt,
        instructions,
        labels,
        body_base_line: compiled.body_base_line,
        proc_body_src: compiled.proc_body_src.clone(),
        splice_ranges: vec![SpliceRange {
            start: "splice_start".to_owned(),
            end: "splice_end".to_owned(),
            entry_depth: 0,
            words: words.iter().map(|&slot| u32::try_from(slot).unwrap()).collect(),
            resumes_with_value: true,
        }],
        ..FunctionAsm::default()
    };
    assert!(spliced.splice_ranges_valid());
    spliced
}

fn vm_with_spliced_caller(inline: &'static [&'static str]) -> Vm {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(CachedBodyCompileService {
        inner: BytecodeCompileService::default(),
        qname: "::caller".to_owned(),
        body_src: "vdouble [nextval]".to_owned(),
        body: tcl_bytecode::FunctionAsm::default(),
        splice: Some(inline),
    }));
    vm
}

/// The procedure frame of a spliced call that fails comes from the fallback:
/// the words, `[nextval]` included, were evaluated once, and the result,
/// `errorInfo` and `-errorstack` are the uninlined call's.
///
/// tclsh 8.6.18, 9.0.4 and 9.1.0 print `2 1`, then `{bad 1} 1 {bad 1
/// ... "caller"} {INNER {returnImm {bad 1} {}} CALL {vdouble 1} CALL caller}`.
#[test]
fn a_spliced_call_that_fails_runs_its_words_once_through_the_fallback() {
    let script = r#"proc caller {} {vdouble [nextval]}
proc nextval {} {incr ::n}
proc vdouble x {expr {$x*2}}
proc fails {} {error "inline failed"}
set ::n 0
set r [list [caller] $::n]
proc vdouble x {error "bad $x"}
set ::n 0
catch caller m o
lappend r $m $::n [dict get $o -errorinfo] [dict get $o -errorstack]"#;
    let expected = r#"2 1 {bad 1} 1 {bad 1
    while executing
"error "bad $x""
    (procedure "vdouble" line 1)
    invoked from within
"vdouble [nextval]"
    (procedure "caller" line 1)
    invoked from within
"caller"} {INNER {returnImm {bad 1} {}} CALL {vdouble 1} CALL caller}"#;
    // The spliced body fails in a builtin it runs, then in a procedure it calls.
    for inline in [&["error", "inline failed"][..], &["fails"][..]] {
        let mut vm = vm_with_spliced_caller(inline);
        let completion = vm.eval_source(script).expect("script");
        assert_eq!(&*completion.result.to_str(), expected, "{inline:?}");
    }
}
