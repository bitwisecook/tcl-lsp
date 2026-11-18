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

//! Changing the release or profile of a live interpreter: cached bodies recompile, live continuations are refused, and imports recheck their source identity.

use super::*;

struct CountingCompilerSvc {
    calls: Rc<Cell<usize>>,
}

impl CompileService for CountingCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.calls.set(self.calls.get() + 1);
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .compile_script_bytes_for_profile(target, profile)
    }
    fn compile_substitution_with_entry(
        &self,
        target: tcl_runtime_api::native_substitution::NativeSubstitutionTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: tcl_runtime_api::native_substitution::NativeSubstitutionCompilationEntry<'_>,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.calls.set(self.calls.get() + 1);
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .compile_substitution_with_entry(target, profile, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.calls.set(self.calls.get() + 1);
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .compile_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_plain_script_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.calls.set(self.calls.get() + 1);
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .compile_plain_script_bytes_for_profile(target, profile)
    }
    fn compile_plain_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.calls.set(self.calls.get() + 1);
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }
    fn compile_procedure_bytes_for_profile(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.calls.set(self.calls.get() + 1);
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .compile_procedure_bytes_for_profile(target, profile, dispatch)
    }
    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
        self.calls.set(self.calls.get() + 1);
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
    }
    fn script_command_plan_bytes_for_profile(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .script_command_plan_bytes_for_profile(source, profile)
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
        tcl_codegen::compile_service::BytecodeCompileService::for_profile(profile)
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile_script_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        self.calls.set(self.calls.get() + 1);
        BytecodeCompileService::for_profile(profile)
            .compile_script_with_entry(target, profile, entry)
    }

    fn compile_procedure_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTarget<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &tcl_runtime_api::NativeCompilationEntry,
        dispatch: tcl_runtime_api::ProcedureDispatch,
    ) -> Result<Self::Module, tcl_vm::CompileError> {
        self.calls.set(self.calls.get() + 1);
        BytecodeCompileService::for_profile(profile)
            .compile_procedure_with_entry(target, profile, entry, dispatch)
    }

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.compile_for_profile(src, DialectProfile::plain_tcl())
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.calls.set(self.calls.get() + 1);
        BytecodeCompileService::for_profile(profile).compile(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        self.calls.set(self.calls.get() + 1);
        BytecodeCompileService::for_profile(profile)
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

/// Reproduce the stateful form of the import escape: importing while the 8.5
/// surface is active, then pinning that same interpreter to 8.4.  The local
/// clone and provenance remain in its table, but resolution must now reject
/// the final builtin identity.
#[test]
fn version_mutation_rechecks_an_existing_imports_source_identity() {
    let mut vm = tcl_vm::test_support::native_vm_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
        Box::new(std::io::stdout()),
    );
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile(),
    )));
    let setup = vm
        .eval_source(
            "namespace export lassign\n\
             namespace eval imported {namespace import ::lassign}\n\
             interp hide {} lassign hidden_lassign\n\
             interp expose {} hidden_lassign escaped\n\
             namespace eval imported {rename lassign imported_lassign}\n",
        )
        .expect("8.5 import setup compiles");
    assert!(setup.code.is_ok());
    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    let probe = vm
        .eval_source(
            "catch {namespace eval imported {set cmd imported_; append cmd lass ign; $cmd {a b} x}} imported; \\
             catch {set cmd es; append cmd caped; $cmd {a b} x} escaped; \\
             list $imported $escaped\n",
        )
        .expect("8.4 probe compiles");
    assert_eq!(
        probe.result.to_str().as_ref(),
        "{invalid command name \"imported_lassign\"} {invalid command name \"escaped\"}"
    );
}

/// Changing a VM's profile is not merely a lookup-cache invalidation: bodies
/// compiled while a newer surface was active can contain direct bytecode for a
/// command that the older profile would reject.  Keep the compiler installed
/// at its original 8.5 construction profile to prove the VM selects the new
/// profile for every recompile itself.
#[test]
#[allow(clippy::too_many_lines)] // One stateful profile lifecycle must stay in one causal vector.
fn profile_mutation_recompiles_cached_bodies_and_rejects_live_continuations() {
    let v85 = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
    let v84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let v86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(v86, Box::new(std::io::stdout()));
    // TclOO is an 8.6 command surface, so the class factory is
    // only reachable there; the resulting object command is script-created and
    // stays callable on every release, which is what the flips below exercise.
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v85)));
    let oo_setup = vm
        .eval_source(
            "oo::class create C {method m {} {lassign {a b} x; return $x}}\n\
             C create o",
        )
        .expect("8.6 TclOO setup compiles");
    assert!(oo_setup.code.is_ok(), "{}", oo_setup.result.to_str());
    vm.set_dialect_profile(v85);

    // Prime every cache/body owner that retains executable bytecode: an
    // ordinary proc, TclOO method, child proc reached through an alias,
    // reusable host FunctionHandle, eval cache, lexer/parser cache, and a
    // parked coroutine continuation.
    let setup = vm
        .eval_source(
            "proc p {} {lassign {a b} x; return $x}\n\
             expr {0b10 + 1}",
        )
        .expect("8.5 setup compiles");
    assert!(setup.code.is_ok(), "{}", setup.result.to_str());
    let handle = vm
        .compile_function("lassign {a b} handle_x; set handle_x")
        .expect("8.5 handle compiles");
    // A reusable handle is VM-affine: another VM can have the same profile
    // generation but must never adopt its bytecode.
    let mut foreign_vm =
        tcl_vm::test_support::native_vm_for_profile(v85, Box::new(std::io::stdout()));
    foreign_vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v85)));
    assert_eq!(
        foreign_vm.invoke_function(&handle).result.to_str().as_ref(),
        "FunctionHandle belongs to a different Vm"
    );
    // AOT modules carry the profile they were lowered for and are rejected at
    // the entry boundary instead of being stamped with the VM's generation.
    let aot = BytecodeCompileService::for_profile(v85)
        .compile("set profile_aot 1")
        .expect("AOT module compiles");
    foreign_vm.set_dialect_profile(v84);
    assert_eq!(
        foreign_vm.run_module(&aot).result.to_str().as_ref(),
        "bytecode compiled for dialect profile tcl8.5 cannot run under tcl8.4"
    );
    let generic_registry = tcl_registry::CommandRegistry::build_default();
    let generic_ir =
        tcl_compiler::lowering::lower_to_ir_for_bytecode("lassign {a b} x", &generic_registry);
    let generic_cfg = build_cfg_codegen(&generic_ir, false);
    let generic_aot = codegen_module(&generic_cfg, &generic_ir, &generic_registry)
        .expect("original source namespace");
    assert_eq!(
        foreign_vm.run_module(&generic_aot).result.to_str().as_ref(),
        "bytecode compiled for dialect profile tcl cannot run under tcl8.4"
    );
    let traced_registry = tcl_registry::model::ingress::static_context_for_profile(v85).commands();
    let traced_config = tcl_lexer::LexerConfig::from_grammar(v85.grammar);
    let traced_ir = tcl_compiler::lowering::lower_to_ir_traced_with_dialect(
        "lassign {a b} x",
        traced_registry,
        traced_config,
        Some(v85),
    );
    let traced_cfg = build_cfg_codegen(&traced_ir, false);
    let traced_aot = codegen_module(&traced_cfg, &traced_ir, traced_registry)
        .expect("original source namespace");
    assert!(std::ptr::eq(traced_aot.profile, v85));
    assert_eq!(
        foreign_vm.run_module(&traced_aot).result.to_str().as_ref(),
        "bytecode compiled for dialect profile tcl8.5 cannot run under tcl8.4"
    );

    // The reported escape, exactly: a body compiled under 8.5 must not retain
    // its inline lassign lowering after a direct 8.4 flip, and the reverse
    // flip must compile it back onto the newer surface.
    vm.set_dialect_profile(v84);
    let exact_84 = vm
        .eval_source("catch {p} message; set message")
        .expect("8.4 exact repro compiles");
    assert_eq!(
        exact_84.result.to_str().as_ref(),
        "invalid command name \"lassign\""
    );
    // 8.4 compiles the script whole, and `expr`'s compile procedure refuses
    // the `0b10` it has no grammar for (tclsh 8.4.20).
    assert_eq!(
        &*vm.eval_source("expr {0b10 + 1}")
            .expect_err("8.4 refuses to compile the expression")
            .message_bytes()
            .unwrap(),
        b"syntax error in expression \"0b10 + 1\": extra tokens at end of expression"
    );
    assert!(
        vm.eval_source("{*}[list set ::profile_mutation_lexer 1]")
            .is_err()
    );
    vm.set_dialect_profile(v85);
    assert_eq!(
        vm.eval_source("list [p] [expr {0b10 + 1}]")
            .expect("8.5 reverse repro compiles")
            .result
            .to_str()
            .as_ref(),
        "a 3"
    );

    // `coroutine` begins in 8.6. Retain the 8.5-built proc/handle above, then
    // add a live continuation and child-owned profile while the newer surface
    // is selected.
    vm.set_dialect_profile(v86);
    let newer_setup = vm
        .eval_source(
            "coroutine c apply {{} {yield ready; lassign {a b} x; return $x}}\n\
             interp create child\n\
             child eval {proc p {} {lassign {a b} x; return $x}}\n\
             interp alias {} child_p child p",
        )
        .expect("8.6 coroutine setup compiles");
    assert!(newer_setup.code.is_ok(), "{}", newer_setup.result.to_str());
    // This source is accepted and expanded on 8.5+; retaining its compiled
    // module exercises the lexer-sensitive eval cache that 8.4 must discard
    // and re-lex.
    let lexer_86 = vm
        .eval_source("{*}[list set ::profile_mutation_lexer 1]")
        .expect("8.5 lexer source compiles");
    assert_eq!(lexer_86.code, Code::Ok);
    assert_eq!(lexer_86.result.to_str().as_ref(), "1");

    assert!(vm.set_child_dialect_profile("child", v84));
    vm.set_dialect_profile(v84);

    let stale = vm
        .eval_source(
            "catch {p} proc_msg; catch {o m} method_msg; catch {child_p} alias_msg; \\
             list $proc_msg $method_msg $alias_msg",
        )
        .expect("8.4 probes compile");
    assert_eq!(
        stale.result.to_str().as_ref(),
        "{invalid command name \"lassign\"} {invalid command name \"lassign\"} {invalid command name \"lassign\"}"
    );
    let handle_result = vm.invoke_function(&handle);
    assert_eq!(handle_result.code, Code::Error);
    assert_eq!(
        handle_result.result.to_str().as_ref(),
        "invalid command name \"lassign\""
    );

    // The exact same source must not be served from its newer-profile eval cache:
    // expression grammar and lexer grammar changed with the profile too.
    assert_eq!(
        &*vm.eval_source("expr {0b10 + 1}")
            .expect_err("8.4 refuses to compile the expression")
            .message_bytes()
            .unwrap(),
        b"syntax error in expression \"0b10 + 1\": extra tokens at end of expression"
    );
    assert!(
        vm.eval_source("{*}[list set ::profile_mutation_lexer 1]")
            .is_err()
    );

    // A coroutine cannot be recompiled in place after it yielded: it owns a
    // live PC/operand stack. The frame generation guard therefore rejects the
    // stale continuation before it reaches its old inline lassign opcode.
    let continuation = vm
        .eval_source("catch {c} continuation_msg; set continuation_msg")
        .expect("8.4 continuation probe compiles");
    assert_eq!(
        continuation.code,
        Code::Ok,
        "catch must absorb the stale frame"
    );
    assert_eq!(
        continuation.result.to_str().as_ref(),
        "cannot continue bytecode after dialect profile changed"
    );

    // The reverse mutation gets fresh 8.5 bodies (including the child and
    // alias) rather than retaining the failed 8.4 recompilation.
    assert!(vm.set_child_dialect_profile("child", v85));
    vm.set_dialect_profile(v85);
    let restored = vm
        .eval_source(
            "proc q {} {lassign {a b} x; return $x}; \\
             list [p] [o m] [child_p] [q] [expr {0b10 + 1}]",
        )
        .expect("8.5 restoration compiles");
    assert_eq!(restored.result.to_str().as_ref(), "a a a a 3");

    // And the live-coroutine surface returns normally once a newly-created
    // continuation is compiled under a release that supports it.
    vm.set_dialect_profile(v86);
    let resumed = vm
        .eval_source("coroutine c2 apply {{} {yield ready; lassign {a b} x; return $x}}; c2")
        .expect("8.6 coroutine restoration compiles");
    assert_eq!(resumed.result.to_str().as_ref(), "a");
}

#[test]
fn tcloo_method_profile_refresh_is_persisted_after_the_first_call() {
    let calls = Rc::new(Cell::new(0));
    // TclOO's class factory is 8.6+; the object it makes is
    // script-created and survives the flip to 8.4 below.
    let mut vm = tcl_vm::test_support::native_vm_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Box::new(std::io::stdout()),
    );
    vm.set_compiler(Box::new(CountingCompilerSvc {
        calls: Rc::clone(&calls),
    }));
    let setup = vm
        .eval_source("oo::class create C {method m {} {lassign {a b} x; return $x}}; C create o")
        .expect("class setup compiles");
    assert!(setup.code.is_ok(), "{}", setup.result.to_str());

    vm.set_dialect_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    );
    let before = calls.get();
    let first = vm.invoke_command("o", &[tcl_vm::Value::string("m")]);
    let after_first = calls.get();
    let second = vm.invoke_command("o", &[tcl_vm::Value::string("m")]);
    assert_eq!(first.code, Code::Error);
    assert_eq!(second.code, Code::Error);
    assert_eq!(
        after_first,
        before + 1,
        "first call recompiles the method once"
    );
    assert_eq!(
        calls.get(),
        after_first,
        "second call reuses the refreshed method"
    );
}

/// An imported proc owns its local binding even after its source has been
/// renamed. Refreshing trace-stale bytecode must update that exact resolved
/// import, never the `ProcDef` source name, which may now name a replacement.
#[test]
fn imported_proc_refresh_preserves_renamed_source_replacement() {
    let src = concat!(
        "namespace eval src {proc p {} {return original}; namespace export p}\n",
        "namespace eval dst {namespace import ::src::p}\n",
        "rename ::src::p ::src::q\n",
        "proc ::src::p {} {return replacement}\n",
        "proc noop args {}\n",
        "trace add execution ::dst::p enterstep noop\n",
        "puts [list [::dst::p] [::src::p] [::src::q] [namespace origin ::dst::p]]\n",
    );
    let want = "original replacement original ::src::q";
    assert_eq!(
        profile_output(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        ),
        want
    );
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] imported proc refresh");
        }
    }
}

/// A profile flip re-pins the whole release: everything compiled afterwards
/// answers as a fresh interpreter of the new release. tclsh 8.5.19:
/// `expr {0b10 + 1}` is 3, `lmap` and `try` are `invalid command name`;
/// tclsh 8.4.20: `expr {0b10 + 1}` is `syntax error in expression
/// "0b10 + 1": extra tokens at end of expression`.
#[test]
fn a_profile_flip_answers_as_a_fresh_interpreter_of_the_new_release() {
    let v86 = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let v85 = tcl_registry::model::ingress::resolve_environment("tcl8.5").analyser_profile();
    let v84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let mut vm = tcl_vm::test_support::native_vm_for_profile(v86, Box::new(std::io::stdout()));
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v86)));
    let probe = "list [list [catch {expr {0b10 + 1}} m] $m] \
                 [list [catch {lmap x {1} {set x}} m] $m] \
                 [list [catch {try {set a 1}} m] $m]";
    vm.set_dialect_profile(v85);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v85)));
    let answer = vm.eval_source(probe).expect("8.5 compiles");
    assert_eq!(
        answer.result.to_str().as_ref(),
        "{0 3} {1 {invalid command name \"lmap\"}} {1 {invalid command name \"try\"}}"
    );
    vm.set_dialect_profile(v84);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v84)));
    let answer = vm
        .eval_source("list [catch {expr {0b10 + 1}} m] $m")
        .expect("8.4 compiles");
    assert_eq!(
        answer.result.to_str().as_ref(),
        "1 {syntax error in expression \"0b10 + 1\": extra tokens at end of expression}"
    );
}
