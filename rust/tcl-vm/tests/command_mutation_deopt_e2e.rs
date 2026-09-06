// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Runtime command-mutation invalidation (#1648).
//!
//! Tcl 9.0.4 is the oracle. Its command compile epoch makes a replacement of a
//! byte-compiled builtin visible across separately compiled proc/eval units;
//! the VM mirrors that by switching future units to ordinary dispatch.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::inlining::inline_module;
use tcl_compiler::lowering::{lower_script_module_for_bytecode, lower_to_ir_traced_with_dialect};
use tcl_dialect::{DialectProfile, TclVersion};
use tcl_lexer::LexerConfig;
use tcl_vm::{
    Code, Commands, CompileError, CompileService, Completion, FunctionHandle, NativeCommand,
    ProcedureCompileTarget, ProcedureDispatch, ScriptCompileTarget, Value, Vm,
};

struct OptimisedOnlyCompilerSvc(BytecodeCompileService);

struct DishonestPlainCompilerSvc(BytecodeCompileService);

struct ScriptOnlyCompilerSvc(BytecodeCompileService);

/// Test-only executable pipeline which opts into the public IR inliner before
/// CFG/codegen. Production `BytecodeCompileService` deliberately does not.
struct InliningCompilerSvc(BytecodeCompileService);

struct CountingCompilerSvc {
    inner: BytecodeCompileService,
    fast_calls: Rc<Cell<usize>>,
    plain_calls: Rc<Cell<usize>>,
}

struct NativeOverride;

struct CountCalls(Rc<Cell<usize>>);

struct SwitchProfileAndContinue(&'static DialectProfile);

struct SwitchProfileAndOk(&'static DialectProfile);

struct SwitchCompilerToCustom;

struct SwitchCompilerToDefault;

struct SwitchCompilerAndError;

struct SwitchCompilerAndReturn;

struct CompileOrInvokeHandle(RefCell<Option<FunctionHandle>>);

struct RunCompiledModule(Rc<tcl_bytecode::ModuleAsm>);

struct RunCompiledFunction(Rc<tcl_bytecode::FunctionAsm>);

impl NativeCommand for NativeOverride {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        Completion::new(Code::Ok, Value::string("NATIVE_OVERRIDE"), Value::empty())
    }
}

impl NativeCommand for CountCalls {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        self.0.set(self.0.get() + 1);
        Completion::new(Code::Ok, Value::empty(), Value::empty())
    }
}

impl NativeCommand for SwitchProfileAndContinue {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_dialect_profile(self.0);
        Completion::new(Code::Continue, Value::empty(), Value::empty())
    }
}

impl NativeCommand for SwitchProfileAndOk {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_dialect_profile(self.0);
        Completion::new(Code::Ok, Value::string("switched"), Value::empty())
    }
}

impl NativeCommand for SwitchCompilerToCustom {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_compiler(Box::new(custom_compile_service()));
        Completion::new(Code::Ok, Value::string("switched"), Value::empty())
    }
}

impl NativeCommand for SwitchCompilerToDefault {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_compiler(Box::new(BytecodeCompileService::default()));
        Completion::new(Code::Ok, Value::string("switched"), Value::empty())
    }
}

impl NativeCommand for SwitchCompilerAndError {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_compiler(Box::new(custom_compile_service()));
        Completion::new(
            Code::Error,
            Value::string("native tailcall error"),
            Value::empty(),
        )
    }
}

impl NativeCommand for SwitchCompilerAndReturn {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.set_compiler(Box::new(custom_compile_service()));
        Completion::new(
            Code::Return,
            Value::string("native tailcall return"),
            Value::empty(),
        )
    }
}

impl NativeCommand for CompileOrInvokeHandle {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let operation = args.first().map(|arg| arg.to_str().to_string());
        match operation.as_deref() {
            Some("compile") => match vm.compile_function("expr {1+2}") {
                Ok(handle) => {
                    *self.0.borrow_mut() = Some(handle);
                    Completion::new(Code::Ok, Value::empty(), Value::empty())
                }
                Err(error) => {
                    Completion::new(Code::Error, Value::string(error.message), Value::empty())
                }
            },
            Some("invoke") => {
                let handle = self.0.borrow().clone().expect("handle compiled first");
                vm.invoke_function(&handle)
            }
            _ => Completion::new(
                Code::Error,
                Value::string("expected compile or invoke"),
                Value::empty(),
            ),
        }
    }
}

impl NativeCommand for RunCompiledModule {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.run_module(&self.0)
    }
}

impl NativeCommand for RunCompiledFunction {
    fn invoke(&self, vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        vm.run_function(&self.0)
    }
}

impl CompileService for OptimisedOnlyCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.0.compile(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        if dispatch == ProcedureDispatch::Plain {
            return Err(CompileError(
                "plain procedure compilation unavailable".into(),
            ));
        }
        self.0
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

impl CompileService for DishonestPlainCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.0.compile(src)
    }

    fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
        // Deliberately violate the capability contract: the VM must validate
        // the returned artifact's explicit mode instead of trusting this
        // method's name or inferring safety from its dependency list.
        let mut module = self.0.compile(src)?;
        module.top_level.command_bindings.clear();
        for procedure in module.procedures.values_mut() {
            procedure.command_bindings.clear();
        }
        Ok(module)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        let mut module =
            self.0
                .compile_procedure_for_profile(target, profile, ProcedureDispatch::Optimised)?;
        if dispatch == ProcedureDispatch::Plain {
            module.top_level.command_bindings.clear();
        }
        Ok(module)
    }
}

impl CompileService for ScriptOnlyCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.0.compile(src)
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.0.compile_for_profile(src, profile)
    }

    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.0.compile_traced_for_profile(src, profile)
    }
}

impl CompileService for InliningCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.0.compile(src)
    }

    fn compile_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.compile_script_for_profile(
            ScriptCompileTarget {
                source: src,
                namespace: "",
            },
            profile,
        )
    }

    fn compile_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = LexerConfig::from_grammar(profile.grammar);
        let ir = lower_script_module_for_bytecode(
            target.source,
            target.namespace,
            registry,
            config,
            Some(profile),
            false,
        );
        let ir = inline_module(ir, registry);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry))
    }

    fn compile_traced_for_profile(
        &self,
        src: &str,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let config = LexerConfig::from_grammar(profile.grammar);
        let ir = lower_to_ir_traced_with_dialect(src, registry, config, Some(profile));
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, registry))
    }

    fn compile_plain_script_for_profile(
        &self,
        target: ScriptCompileTarget<'_>,
        profile: &'static DialectProfile,
    ) -> Result<Self::Module, CompileError> {
        self.0.compile_plain_script_for_profile(target, profile)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        self.0
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

impl CompileService for CountingCompilerSvc {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.fast_calls.set(self.fast_calls.get() + 1);
        self.inner.compile(src)
    }

    fn compile_traced(&self, src: &str) -> Result<Self::Module, CompileError> {
        self.plain_calls.set(self.plain_calls.get() + 1);
        self.inner.compile_traced(src)
    }

    fn compile_procedure_for_profile(
        &self,
        target: ProcedureCompileTarget<'_>,
        profile: &'static DialectProfile,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        match dispatch {
            ProcedureDispatch::Optimised => self.fast_calls.set(self.fast_calls.get() + 1),
            ProcedureDispatch::Plain => self.plain_calls.set(self.plain_calls.get() + 1),
        }
        self.inner
            .compile_procedure_for_profile(target, profile, dispatch)
    }
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn vm() -> (Vm, Capture) {
    let output = Capture::default();
    let mut vm = Vm::with_output(Box::new(output.clone()));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    (vm, output)
}

fn custom_compile_service() -> BytecodeCompileService {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let mut list = registry.get("list").expect("list spec").clone();
    // This owned surface deliberately redefines list's compiler semantics as
    // Expr. Keeping the builtin list fold would make the registry internally
    // contradictory and let a value-position fold mask the custom hooks.
    list.const_fold = None;
    list.lowering_hook = Some(tcl_registry::hooks::LoweringHookId::Expr);
    list.inline_codegen_hook = Some(tcl_registry::hooks::InlineCodegenHookId::Expr);
    registry.insert(list);
    BytecodeCompileService::new(registry)
}

fn eval_ok(vm: &mut Vm, source: &str) -> String {
    let completion = vm.eval_source(source).expect("source compiles");
    assert_eq!(completion.code, Code::Ok, "{}", completion.result.to_str());
    completion.result.to_str().to_string()
}

#[test]
fn explicit_inline_codegen_pipeline_invalidates_after_user_proc_redefinition() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = "proc target {} {return OLD}; proc caller {} {target}";
    let service = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let module = service
        .compile_for_profile(source, profile)
        .expect("explicit inline pipeline compiles");
    assert_eq!(
        module.procedures["::caller"].procedure_bindings,
        [tcl_runtime_api::ProcedureBindingIdentity::new(
            "target",
            "::target",
            "",
            "return OLD",
        )],
        "the consumed target call must survive codegen as an exact typed dependency",
    );

    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");
    assert_eq!(eval_ok(&mut vm, "caller"), "OLD");

    eval_ok(&mut vm, "rename target {}; proc target {} {return NEW}");
    assert_eq!(
        eval_ok(&mut vm, "caller"),
        "NEW",
        "the stale inlined body must deoptimise to the live procedure binding",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "{source}; puts [caller]; rename target {{}}; proc target {{}} {{return NEW}}; puts [caller]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            "OLD\nNEW",
            "Tcl 9.0.4 observes the redefined procedure",
        );
    }
}

#[test]
fn inlined_bare_procedure_call_revalidates_resolution_after_local_shadow() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = concat!(
        "namespace eval ::N {}\n",
        "proc target {} {return GLOBAL}\n",
        "proc ::N::caller {} {target}",
    );
    let service = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let module = service
        .compile_for_profile(source, profile)
        .expect("explicit inline pipeline compiles");
    assert_eq!(
        module.procedures["::N::caller"].procedure_bindings,
        [tcl_runtime_api::ProcedureBindingIdentity::in_namespace(
            "N",
            "target",
            "::target",
            "",
            "return GLOBAL",
        )],
        "the erased call must retain its source resolution and selected definition",
    );

    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");
    assert_eq!(eval_ok(&mut vm, "N::caller"), "GLOBAL");

    eval_ok(&mut vm, "proc ::N::target {} {return LOCAL}");
    assert_eq!(
        eval_ok(&mut vm, "N::caller"),
        "LOCAL",
        "a local shadow must invalidate the caller's inlined global target",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "{source}; puts [N::caller]; proc ::N::target {{}} {{return LOCAL}}; puts [N::caller]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            "GLOBAL\nLOCAL",
            "Tcl 9.0.4 resolves the newly-created namespace-local procedure",
        );
    }
}

#[test]
fn inlined_namespaced_body_revalidates_commands_in_its_defining_namespace() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = concat!(
        "namespace eval ::n {}\n",
        "proc ::n::callee {} {expr {40 + 2}}\n",
        "proc caller {} {::n::callee}\n",
    );
    let service = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let module = service
        .compile_for_profile(source, profile)
        .expect("explicit inline pipeline compiles");
    let caller = &module.procedures["::caller"];
    assert!(
        caller
            .procedure_bindings
            .iter()
            .any(|binding| binding.name == "::n::callee"),
        "the caller must contain the callee's inlined body",
    );
    assert!(
        caller.command_bindings.iter().any(|binding| {
            binding.resolution_namespace == "n"
                && binding.name == "expr"
                && binding.identity == "expr"
        }),
        "the copied expr specialisation must retain the callee's defining namespace",
    );

    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");
    assert_eq!(eval_ok(&mut vm, "caller"), "42");

    eval_ok(&mut vm, "proc ::n::expr args {return NAMESPACE_OVERRIDE}");
    assert_eq!(
        eval_ok(&mut vm, "caller"),
        "NAMESPACE_OVERRIDE",
        "a local command shadow must invalidate the copied namespaced binding",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "{source}; puts [caller]; proc ::n::expr args {{return NAMESPACE_OVERRIDE}}; puts [caller]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            "42\nNAMESPACE_OVERRIDE",
            "Tcl 9.0.4 resolves the newly-created local expr binding",
        );
    }
}

#[test]
fn active_inlined_namespaced_boundary_replays_in_its_defining_namespace() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = concat!(
        "namespace eval ::n {}\nnamespace eval ::m {}\nnamespace eval ::e {}\n",
        "proc mutate_n {name1 name2 op} {",
        "proc ::n::expr args {return N_OVERRIDE}}\n",
        "proc mutate_m {name1 name2 op} {",
        "proc ::m::expr args {return M_OVERRIDE}}\n",
        "proc mutate_e {name1 name2 op} {",
        "proc ::e::expr args {error E_FAILURE}}\n",
        "set ::trigger_n {}; trace add variable ::trigger_n read mutate_n\n",
        "set ::trigger_m {}; trace add variable ::trigger_m read mutate_m\n",
        "set ::trigger_e {}; trace add variable ::trigger_e read mutate_e\n",
        "proc ::n::callee {} {",
        "llength $::trigger_n; puts -nonewline {}; expr {40 + 2}}\n",
        "proc ::m::callee {} {",
        "llength $::trigger_m; puts -nonewline {}; expr {40 + 2}}\n",
        "proc ::e::callee {} {",
        "llength $::trigger_e; puts -nonewline {}; expr {40 + 2}}\n",
        "proc caller_n {} {::n::callee}\n",
        "proc caller_m {} {::m::callee}\n",
        "proc caller_e {} {::e::callee}\n",
    );
    let service = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let module = service
        .compile_for_profile(source, profile)
        .expect("explicit inline pipeline compiles");
    for (caller_name, callee_name, namespace) in [
        ("::caller_n", "::n::callee", "n"),
        ("::caller_m", "::m::callee", "m"),
        ("::caller_e", "::e::callee", "e"),
    ] {
        let caller = &module.procedures[caller_name];
        assert!(
            caller
                .procedure_bindings
                .iter()
                .any(|binding| binding.name == callee_name),
            "{caller_name} must contain the callee's inlined body",
        );
        let expr_boundary = caller
            .instructions
            .iter()
            .find(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.source_cmd_text == "expr {40 + 2}"
            })
            .expect("the inlined expr has a stale-command replay boundary");
        assert_eq!(
            expr_boundary.source_command_namespace, namespace,
            "the replay target must retain the callee's constructed namespace",
        );
    }

    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");
    assert_eq!(eval_ok(&mut vm, "caller_n"), "N_OVERRIDE");
    assert_eq!(
        eval_ok(&mut vm, "namespace current"),
        "::",
        "successful replay must restore the caller's namespace",
    );
    assert_eq!(
        eval_ok(&mut vm, "caller_m"),
        "M_OVERRIDE",
        "same-source replays in distinct inlined namespaces must not share a plain cache entry",
    );
    assert_eq!(
        eval_ok(&mut vm, "catch {caller_e} replay_error; set replay_error"),
        "E_FAILURE",
    );
    assert_eq!(
        eval_ok(&mut vm, "namespace current"),
        "::",
        "error unwind from a replay must restore the caller's namespace",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "{source}; puts [caller_n]; puts [caller_m]; catch {{caller_e}} replay_error; puts $replay_error; puts [namespace current]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            "N_OVERRIDE\nM_OVERRIDE\nE_FAILURE\n::",
            "Tcl 9.0.4 re-resolves expr in the callee's defining namespace",
        );
    }
}

#[test]
fn namespaced_boundary_replay_preserves_the_frame_namespace_pairing() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let source = concat!(
        "namespace eval ::n {}\n",
        "proc pick {} {return GLOBAL}\n",
        "proc ::n::pick {} {return NAMESPACE}\n",
        "proc replay_worker args {uplevel 1 {pick}}\n",
        "proc mutate {name1 name2 op} {",
        "interp alias {} ::n::expr {} ::replay_worker}\n",
        "set ::trigger {}; trace add variable ::trigger read mutate\n",
        "proc ::n::callee {} {",
        "llength $::trigger; puts -nonewline {}; expr {40 + 2}}\n",
        "proc caller {} {::n::callee}\n",
    );
    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(eval_ok(&mut vm, source), "");

    // The stale inlined boundary runs in ::n without adding a Tcl call frame.
    // Its alias target's `uplevel 1` must therefore observe ::n, just as Tcl
    // 9.0.4 does for the non-inlined ::n::callee frame.
    assert_eq!(eval_ok(&mut vm, "caller"), "NAMESPACE");
    assert_eq!(eval_ok(&mut vm, "namespace current"), "::");
}

#[test]
fn top_level_cross_namespace_call_keeps_its_proc_frame_and_matches_tcl90() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let body = "llength $::trigger; puts -nonewline {}; expr {40 + 2}";
    let source = format!("proc ::n::callee {{}} {{{body}}}\n::n::callee\n");
    let service = InliningCompilerSvc(BytecodeCompileService::for_profile(profile));
    let module = service
        .compile_for_profile(&source, profile)
        .expect("top-level inline pipeline compiles");
    assert!(
        module
            .top_level
            .procedure_bindings
            .iter()
            .all(|binding| binding.name != "::n::callee"),
        "a cross-namespace top-level call must keep its Tcl procedure frame",
    );

    let mut vm = Vm::new();
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(InliningCompilerSvc(
        BytecodeCompileService::for_profile(profile),
    )));
    assert_eq!(
        eval_ok(
            &mut vm,
            &format!(
                "namespace eval ::n {{}}\n\
                 proc ::n::callee {{}} {{{body}}}\n\
                 proc replay_worker args {{\
                     set ::replay_observed [uplevel #0 {{namespace current}}]; c\
                 }}\n\
                 proc mutate {{name1 name2 op}} {{\
                     interp alias {{}} ::n::expr {{}} ::replay_worker\
                 }}\n\
                 set ::replay_observed UNTOUCHED\n\
                 set ::trigger {{}}\n\
                 trace add variable ::trigger read mutate\n\
                 coroutine c eval {{yield READY; namespace current}}"
            ),
        ),
        "READY",
    );

    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(
        completion.result.to_str().as_ref(),
        "::",
        "the replacement command runs from ::n::callee's retained procedure frame",
    );
    assert_eq!(
        eval_ok(
            &mut vm,
            "list $::replay_observed [info commands c] [uplevel #0 {namespace current}]",
        ),
        ":: {} ::",
        "the replacement observes the global root, consumes the coroutine, and restores global context",
    );

    if let Some(tclsh) =
        tcl_test_support::locate_tclsh(TclVersion::V9_0).expect("the Tcl 9.0 oracle is valid")
    {
        assert_eq!(
            tclsh.patchlevel,
            tcl_test_support::reference_patchlevel(TclVersion::V9_0),
            "exact Tcl oracle pin",
        );
        let oracle_source = format!(
            "namespace eval ::n {{}}\n\
             proc ::n::callee {{}} {{{body}}}\n\
             proc replay_worker args {{\
                 set ::replay_observed [uplevel #0 {{namespace current}}]; c\
             }}\n\
             proc mutate {{name1 name2 op}} {{\
                 interp alias {{}} ::n::expr {{}} ::replay_worker\
             }}\n\
             set ::replay_observed UNTOUCHED\n\
             set ::trigger {{}}\n\
             trace add variable ::trigger read mutate\n\
             coroutine c eval {{yield READY; namespace current}}\n\
             set result [::n::callee]\n\
             puts [list $result $::replay_observed [info commands c] \
                 [uplevel #0 {{namespace current}}]]"
        );
        let outcome = tcl_test_support::run_script(&tclsh.path, oracle_source.as_bytes())
            .expect("Tcl 9.0.4 oracle runs");
        assert_eq!(
            outcome.strict_text().expect("Tcl 9.0.4 succeeds"),
            ":: :: {} ::",
            "Tcl 9.0.4 retains the callee frame and global root",
        );
    }
}

#[test]
fn builtin_replacement_crosses_proc_and_eval_unit_boundaries() {
    let (mut vm, output) = vm();
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return OVERRIDE}",
    );
    eval_ok(
        &mut vm,
        "proc p {} {set x [expr {1+2}]; return $x}; \
         puts [list proc [p]]; \
         puts [list eval [eval {set x [expr {1+2}]; set x}]]",
    );

    // Tcl 9.0.4: `proc OVERRIDE\neval OVERRIDE`.
    assert_eq!(
        String::from_utf8(output.0.borrow().clone()).expect("UTF-8 output"),
        "proc OVERRIDE\neval OVERRIDE\n"
    );
}

#[test]
fn cached_eval_is_not_reused_after_builtin_replacement() {
    let (mut vm, _output) = vm();
    assert_eq!(eval_ok(&mut vm, "expr {1+2}"), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return OVERRIDE}",
    );
    assert_eq!(eval_ok(&mut vm, "expr {1+2}"), "OVERRIDE");
}

#[test]
fn compiler_swap_invalidates_cached_eval_in_both_directions() {
    let (mut vm, _output) = vm();
    let source = "list {1 + 2}";
    assert_eq!(eval_ok(&mut vm, source), "{1 + 2}");

    vm.set_compiler(Box::new(custom_compile_service()));
    assert_eq!(eval_ok(&mut vm, source), "3");

    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_eq!(eval_ok(&mut vm, source), "{1 + 2}");
}

#[test]
fn owned_registry_controls_value_position_constant_folds() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(custom_compile_service()));

    // `set`'s value is emitted through the shared value-position fold and
    // inline-hook path. The owned registry maps list to Expr and removes
    // list's const-fold, so the nested call must follow the registry's
    // expression hook rather than being frozen with builtin list semantics.
    assert_eq!(
        eval_ok(&mut vm, "set result [list {1 + 2}]; set result"),
        "3"
    );
}

#[test]
fn foreign_module_procs_are_recompiled_by_the_current_service_in_both_orders() {
    let module = BytecodeCompileService::default()
        .compile("proc foreign_p {} {list {1 + 2}}; list {1 + 2}")
        .expect("default module compiles");

    for install_custom_first in [false, true] {
        let (mut vm, _output) = vm();
        if install_custom_first {
            vm.set_compiler(Box::new(custom_compile_service()));
        }

        // Public run_module preserves the supplied top-level bytecode's
        // semantics even when a different same-profile service is installed.
        let top = vm.run_module(&module);
        assert_eq!(top.code, Code::Ok);
        assert_eq!(top.result.to_str().as_ref(), "{1 + 2}");

        if !install_custom_first {
            vm.set_compiler(Box::new(custom_compile_service()));
        }
        // The source-bearing proc is foreign, not falsely stamped as custom;
        // first entry recompiles it through the current list->Expr service.
        assert_eq!(eval_ok(&mut vm, "foreign_p"), "3");
    }
}

#[test]
fn source_less_vm_runs_a_self_contained_foreign_module_proc_as_supplied() {
    let module = BytecodeCompileService::default()
        .compile("proc foreign_self {} {list {1 + 2}}; foreign_self")
        .expect("default module compiles");
    let mut vm = Vm::new();

    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "{1 + 2}");

    // Admission did not relabel the proc as VM-produced: installing a service
    // later still forces recompilation through that service.
    vm.set_compiler(Box::new(custom_compile_service()));
    assert_eq!(eval_ok(&mut vm, "foreign_self"), "3");
}

#[test]
fn source_less_vm_admits_multiple_exact_bodies_for_one_foreign_proc_name() {
    let service = BytecodeCompileService::default();
    let first = service
        .compile("proc same_name {} {return FIRST}; same_name")
        .expect("first module compiles");
    let second = service
        .compile("proc same_name {value} {return SECOND-$value}; same_name ok")
        .expect("second module compiles");
    let mut vm = Vm::new();

    let completion = vm.run_module(&first);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "FIRST");

    let completion = vm.run_module(&second);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "SECOND-ok");

    // Re-admitting the first module proves cache order does not select a body.
    let completion = vm.run_module(&first);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "FIRST");
}

#[test]
fn source_less_vm_uses_each_foreign_modules_same_source_proc_artifact() {
    let source = "proc same_source {} {list {1 + 2}}; same_source";
    let default = BytecodeCompileService::default()
        .compile(source)
        .expect("default module compiles");
    let custom = custom_compile_service()
        .compile(source)
        .expect("custom module compiles");
    let mut vm = Vm::new();

    for (module, expected) in [(&default, "{1 + 2}"), (&custom, "3"), (&default, "{1 + 2}")] {
        let completion = vm.run_module(module);
        assert_eq!(completion.code, Code::Ok, "{completion:?}");
        assert_eq!(completion.result.to_str().as_ref(), expected);
    }
}

#[test]
fn foreign_proc_provenance_name_must_match_its_module_entry() {
    let service = BytecodeCompileService::default();
    let mut malformed = service
        .compile("proc wrong {} {return POISON}")
        .expect("malformed source module starts valid");
    let provenance = malformed
        .procedure_provenance
        .get_mut("::wrong")
        .expect("wrong provenance exists");
    provenance.name = "::target".to_owned();
    provenance.body = "return CLAIMED".to_owned();
    let valid = service
        .compile("proc target {} {return CLAIMED}; target")
        .expect("valid module compiles");
    let mut vm = Vm::new();

    // A source-less VM cannot compile the malformed module's unmatched proc
    // definition, but merely admitting it must not poison another name.
    assert_eq!(vm.run_module(&malformed).code, Code::Error);
    let completion = vm.run_module(&valid);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "CLAIMED");
}

#[test]
fn compiler_swap_refreshes_function_proc_and_tcloo_bodies() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("list {1 + 2}")
        .expect("function compiles");
    eval_ok(
        &mut vm,
        "proc p {} {list {1 + 2}}; \
         oo::class create C {method m {} {list {1 + 2}}}; \
         set o [C new]",
    );

    vm.set_compiler(Box::new(custom_compile_service()));
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    assert_eq!(eval_ok(&mut vm, "$o m"), "3");

    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    assert_eq!(
        vm.invoke_function(&handle).result.to_str().as_ref(),
        "{1 + 2}"
    );
    assert_eq!(eval_ok(&mut vm, "p"), "{1 + 2}");
    assert_eq!(eval_ok(&mut vm, "$o m"), "{1 + 2}");
}

#[test]
fn suspended_coroutine_fails_closed_after_compiler_swap() {
    let (mut vm, _output) = vm();
    assert_eq!(
        eval_ok(
            &mut vm,
            "coroutine c apply {{} {yield ready; list {1 + 2}}}",
        ),
        "ready"
    );

    vm.set_compiler(Box::new(custom_compile_service()));
    let completion = vm.eval_source("c").expect("resume compiles");
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.to_str().as_ref(),
        "cannot continue bytecode after compile service changed"
    );

    let mut vm = Vm::new();
    vm.set_compiler(Box::new(custom_compile_service()));
    assert_eq!(
        eval_ok(
            &mut vm,
            "coroutine c apply {{} {yield ready; list {1 + 2}}}",
        ),
        "ready"
    );
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let completion = vm.eval_source("c").expect("resume compiles");
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.to_str().as_ref(),
        "cannot continue bytecode after compile service changed"
    );
}

#[test]
fn stale_coroutine_teardown_does_not_run_queued_injection() {
    let (mut vm, _output) = vm();
    let injections = Rc::new(Cell::new(0));
    let unsets = Rc::new(Cell::new(0));
    let execution_leaves = Rc::new(Cell::new(0));
    let coroutine_deletes = Rc::new(Cell::new(0));
    let lambda_deletes = Rc::new(Cell::new(0));
    vm.register_native_command(
        "count_injection",
        Rc::new(CountCalls(Rc::clone(&injections))),
    );
    vm.register_native_command("count_unset", Rc::new(CountCalls(Rc::clone(&unsets))));
    vm.register_native_command(
        "count_execution_leave",
        Rc::new(CountCalls(Rc::clone(&execution_leaves))),
    );
    vm.register_native_command(
        "count_coroutine_delete",
        Rc::new(CountCalls(Rc::clone(&coroutine_deletes))),
    );
    vm.register_native_command(
        "count_lambda_delete",
        Rc::new(CountCalls(Rc::clone(&lambda_deletes))),
    );
    eval_ok(
        &mut vm,
        "proc worker {} {set local 1; \
             trace add variable local unset count_unset; \
             yield ready; return done}; \
         trace add execution worker leave count_execution_leave; \
         coroutine c apply {{} {worker}}; \
         coroinject c count_injection; \
         trace add command c delete count_coroutine_delete; \
         set lambda [info commands ::tcl::apply::lambda*]; \
         trace add command $lambda delete count_lambda_delete",
    );
    assert_ne!(eval_ok(&mut vm, "info commands ::tcl::apply::lambda*"), "");

    vm.set_compiler(Box::new(custom_compile_service()));
    let completion = vm.eval_source("c resumed").expect("resume compiles");
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        completion.result.to_str().as_ref(),
        "cannot continue bytecode after compile service changed"
    );
    assert_eq!(
        injections.get(),
        0,
        "stale resume must not execute coroinject"
    );
    assert_eq!(unsets.get(), 1, "normal unwind must fire local unset trace");
    assert_eq!(
        execution_leaves.get(),
        1,
        "normal unwind must settle the suspended worker's execution leave once"
    );
    assert_eq!(
        coroutine_deletes.get(),
        1,
        "stale teardown must delete the coroutine command exactly once"
    );
    assert_eq!(
        lambda_deletes.get(),
        1,
        "stale unwind must delete the temporary lambda command exactly once"
    );
    assert_eq!(eval_ok(&mut vm, "info commands c"), "");
    assert_eq!(
        eval_ok(&mut vm, "info commands ::tcl::apply::lambda*"),
        "",
        "stale unwind must remove the coroutine's temporary lambda proc"
    );
}

#[test]
fn tailcall_non_ok_after_compiler_swap_rejects_the_stale_parent() {
    for (name, command) in [
        (
            "switch_compiler_and_error",
            Rc::new(SwitchCompilerAndError) as Rc<dyn NativeCommand>,
        ),
        (
            "switch_compiler_and_return",
            Rc::new(SwitchCompilerAndReturn) as Rc<dyn NativeCommand>,
        ),
    ] {
        let (mut vm, _output) = vm();
        vm.register_native_command(name, command);
        eval_ok(
            &mut vm,
            &format!(
                "proc tailcaller {{}} {{tailcall {name}}}; \
                 proc parent {{}} {{tailcaller; return unreachable}}"
            ),
        );

        let completion = vm.eval_source("parent").expect("source compiles");
        assert_eq!(completion.code, Code::Error, "{name}: {completion:?}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            "cannot continue bytecode after compile service changed",
            "{name}"
        );
    }
}

#[test]
fn final_native_compiler_swap_cannot_settle_old_deferred_bytecode() {
    for source in [
        "catch {switch_compiler}",
        "set head catch; $head {switch_compiler}",
        "try {switch_compiler}",
        "set head try; $head {switch_compiler}",
    ] {
        let (mut vm, _output) = vm();
        vm.register_native_command("switch_compiler", Rc::new(SwitchCompilerToCustom));
        let completion = vm.eval_source(source).expect("source compiles");
        assert_eq!(completion.code, Code::Error, "{source}: {completion:?}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            "cannot continue bytecode after compile service changed",
            "{source}"
        );

        let mut vm = Vm::new();
        vm.set_compiler(Box::new(custom_compile_service()));
        vm.register_native_command("switch_compiler", Rc::new(SwitchCompilerToDefault));
        let completion = vm.eval_source(source).expect("source compiles");
        assert_eq!(completion.code, Code::Error, "{source}: {completion:?}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            "cannot continue bytecode after compile service changed",
            "{source}"
        );
    }
}

#[test]
fn stale_coroutine_ancestor_cannot_leak_from_fresh_try_handler_or_finally() {
    for source in [
        "coroutine c apply {{} {set h try; $h {switch_compiler} on error {m o} {yield leaked}}}",
        "coroutine c apply {{} {set h try; $h {switch_compiler} finally {yield leaked}}}",
    ] {
        let (mut vm, _output) = vm();
        vm.register_native_command("switch_compiler", Rc::new(SwitchCompilerToCustom));
        let completion = vm.eval_source(source).expect("coroutine source compiles");
        assert_eq!(completion.code, Code::Error, "{source}: {completion:?}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            "cannot continue bytecode after compile service changed",
            "{source}"
        );
        assert_eq!(eval_ok(&mut vm, "info commands c"), "");
    }
}

#[test]
fn fresh_try_return_cannot_pop_a_stale_directly_invoked_proc() {
    for body in [
        "set h try; $h {switch_compiler} on error {m o} {return leaked}",
        "set h try; $h {switch_compiler} finally {return leaked}",
    ] {
        let (mut vm, _output) = vm();
        vm.register_native_command("switch_compiler", Rc::new(SwitchCompilerToCustom));
        eval_ok(&mut vm, &format!("proc p {{}} {{{body}}}"));

        let completion = vm.invoke_command("p", &[]);
        assert_eq!(completion.code, Code::Error, "{body}: {completion:?}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            "cannot continue bytecode after compile service changed",
            "{body}"
        );
    }
}

#[test]
fn terminal_profile_flips_fail_closed_in_catch_try_and_trace() {
    let v90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let v84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    for source in [
        "catch {switch_profile}",
        "set head catch; $head {switch_profile}",
        "try {switch_profile}",
        "set head try; $head {switch_profile}",
        "set x 0; trace add variable x write switch_profile; set x 1",
    ] {
        let mut vm = Vm::new();
        vm.set_dialect_profile(v90);
        vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v90)));
        vm.register_native_command("switch_profile", Rc::new(SwitchProfileAndOk(v84)));

        let completion = vm.eval_source(source).expect("Tcl 9.0 source compiles");
        assert_eq!(completion.code, Code::Error, "{source}: {completion:?}");
        assert_eq!(
            completion.result.to_str().as_ref(),
            "cannot continue bytecode after dialect profile changed",
            "{source}"
        );
    }
}

#[test]
fn runtime_foreach_revalidates_its_stored_body_between_iterations() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate_expr {} {\
             rename expr real_expr; \
             proc expr args {return FOREACH_OVERRIDE}\
         }",
    );

    let result = eval_ok(
        &mut vm,
        "set loop foreach; \
         set seen {}; \
         $loop i {0 1} {\
             if {$i == 0} {mutate_expr; continue}; \
             lappend seen [expr {1+2}]\
         }; \
         set seen",
    );

    // Tcl 9.0.4: iteration one replaces `expr`; the runtime fallback's stored
    // body must observe that replacement when it is activated for iteration two.
    assert_eq!(result, "FOREACH_OVERRIDE");
}

#[test]
fn runtime_lmap_revalidates_its_stored_body_between_iterations() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate_expr {} {\
             rename expr real_expr; \
             proc expr args {return LMAP_OVERRIDE}\
         }",
    );

    let result = eval_ok(
        &mut vm,
        "set loop lmap; \
         $loop i {0 1} {\
             if {$i == 0} {mutate_expr; continue}; \
             expr {1+2}\
         }",
    );

    // Tcl 9.0.4: `continue` omits iteration one and the replacement command's
    // result is the sole value collected from iteration two.
    assert_eq!(result, "LMAP_OVERRIDE");
}

#[test]
fn runtime_foreach_revalidates_after_a_loop_variable_write_trace() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate_expr_trace args {\
             rename expr real_expr; \
             proc expr args {return TRACE_OVERRIDE}\
         }",
    );

    let result = eval_ok(
        &mut vm,
        "trace add variable i write mutate_expr_trace; \
         set seen {}; \
         set loop foreach; \
         $loop i {0} {lappend seen [expr {1+2}]}; \
         set seen",
    );

    // The Rust-side loop-variable assignment runs its write trace after the
    // body was compiled but before its first deferred activation is pushed.
    assert_eq!(result, "TRACE_OVERRIDE");
}

#[test]
fn runtime_foreach_does_not_tag_old_profile_assembly_as_fresh() {
    let v90 = tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
    let v84 = tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile();
    let mut vm = Vm::new();
    vm.set_dialect_profile(v90);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(v90)));

    let body_calls = Rc::new(Cell::new(0));
    vm.register_native_command("count_body", Rc::new(CountCalls(Rc::clone(&body_calls))));
    vm.register_native_command("switch_profile", Rc::new(SwitchProfileAndContinue(v84)));

    let completion = vm
        .eval_source(
            "set loop foreach; \
             $loop i {0 1} {\
                 count_body; \
                 if {$i == 0} {switch_profile}; \
                 lassign {a b} x; \
                 set x\
             }",
        )
        .expect("Tcl 9.0 source compiles");

    // The host command ends iteration one with `continue` as it flips profile.
    // The stored Tcl 9.0 body must fail closed before entering iteration two,
    // rather than executing its old `lassign` lowering under Tcl 8.4.
    assert_eq!(completion.code, Code::Error, "{completion:?}");
    assert_eq!(
        completion.result.to_str().as_ref(),
        "cannot continue bytecode after dialect profile changed"
    );
    assert_eq!(body_calls.get(), 1, "stale body ran a second time");
}

#[test]
fn namespaced_builtin_shadow_invalidates_future_proc_compilation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "namespace eval n {proc expr args {return NAMESPACE}}",
    );
    eval_ok(
        &mut vm,
        "namespace eval n {proc p {} {set x [expr {1+2}]; return $x}}",
    );
    assert_eq!(eval_ok(&mut vm, "n::p"), "NAMESPACE");
}

#[test]
fn forgotten_import_revalidates_a_compiled_proc_binding() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "namespace eval :: {namespace export expr}; \
         namespace eval N {namespace import ::expr; proc p {} {expr {1+2}}}",
    );
    assert_eq!(eval_ok(&mut vm, "N::p"), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return OVERRIDE}; \
         namespace eval N {namespace forget expr}",
    );

    // Tcl 9.0.4: the imported command follows the original token through its
    // rename, then forgetting that alias makes the proc's unqualified lookup
    // reach the replacement global command.
    assert_eq!(eval_ok(&mut vm, "N::p"), "OVERRIDE");
}

#[test]
fn namespace_teardown_revalidates_a_cached_eval_binding() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "namespace eval N {proc expr args {return OVERRIDE}}",
    );
    assert_eq!(
        eval_ok(&mut vm, "namespace eval N {expr {1+2}}"),
        "OVERRIDE"
    );
    eval_ok(&mut vm, "namespace delete N; namespace eval N {}");

    // Tcl 9.0.4: recreating the namespace does not preserve the deleted
    // shadow command, so the same eval body now resolves the global builtin.
    assert_eq!(eval_ok(&mut vm, "namespace eval N {expr {1+2}}"), "3");
}

#[test]
fn untouched_builtin_keeps_normal_result() {
    let (mut vm, _output) = vm();
    eval_ok(&mut vm, "proc p {} {set x [expr {1+2}]; return $x}");
    assert_eq!(eval_ok(&mut vm, "p"), "3");
}

#[test]
fn active_proc_frame_redispatches_after_nested_command_mutation() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("proc p {} {mutate; set x [expr {1+2}]; return $x}")
        .expect("module compiles");
    assert!(
        compiled.procedures["::p"]
            .command_bindings
            .iter()
            .any(|binding| binding.name == "expr" && binding.identity == "expr"),
        "p must begin as an optimised active frame"
    );
    eval_ok(&mut vm, "proc p {} {mutate; set x [expr {1+2}]; return $x}");
    eval_ok(
        &mut vm,
        "proc mutate {} {eval {rename expr real_expr; proc expr args {return OVERRIDE}}}",
    );

    // Tcl 9.0.4: OVERRIDE. The mutation occurs through nested `eval` inside
    // `mutate`, after p's optimised frame is already active; p's next source
    // command must redispatch rather than run its stale expr opcode.
    assert_eq!(eval_ok(&mut vm, "p"), "OVERRIDE");
}

#[test]
fn fused_expr_keeps_its_entered_token_when_an_operand_mutates_expr() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate {} {\
             rename expr saved_expr; \
             proc expr args {return REPLACED}; \
             return 2}; \
         proc p {} {\
             set first [expr {[mutate]+1}]; \
             list $first [expr {2+2}]}",
    );

    // Tcl 9.0.4: `3 REPLACED`. The first expr command was entered before its
    // operand replaced the live spelling, so it finishes with the original
    // token; the next source boundary observes the replacement.
    assert_eq!(eval_ok(&mut vm, "p"), "3 REPLACED");
}

#[test]
fn active_return_expr_redispatches_after_nested_command_mutation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate {} {rename expr real_expr; proc expr args {return RETURN_OVERRIDE}}; \
         proc p {} {mutate; return [expr {1+2}]}",
    );

    // Tcl 9.0.4: RETURN_OVERRIDE. `return` consumes the nested expr command
    // into typed IR, but the live frame must still depend on expr's identity.
    assert_eq!(eval_ok(&mut vm, "p"), "RETURN_OVERRIDE");
}

#[test]
fn active_catch_body_redispatches_after_nested_command_mutation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate {} {rename expr real_expr; proc expr args {return CATCH_OVERRIDE}}; \
         proc p {} {mutate; set rc [catch {expr {1+2}} value]; list $rc $value}",
    );

    // Tcl 9.0.4: `0 CATCH_OVERRIDE`. The catch emitter consumes both catch
    // and its body command, so both bindings participate in revalidation.
    assert_eq!(eval_ok(&mut vm, "p"), "0 CATCH_OVERRIDE");
}

#[test]
fn active_nested_try_body_redispatches_after_nested_command_mutation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc mutate {} {rename expr real_expr; proc expr args {return TRY_OVERRIDE}}; \
         proc p {} {mutate; set rc [catch {try {expr {1+2}} on error {m} {set m handled}} value]; list $rc $value}",
    );

    // Tcl 9.0.4: `0 TRY_OVERRIDE`. The nested try body is another inline
    // compiler context, not a reason to lose expr's live binding identity.
    assert_eq!(eval_ok(&mut vm, "p"), "0 TRY_OVERRIDE");
}

#[test]
fn active_proc_frame_revalidates_a_cfg_consumed_while_head() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("proc p {} {mutate; while {0} {}}")
        .expect("module compiles");
    assert!(
        compiled.procedures["::p"]
            .command_bindings
            .iter()
            .any(|binding| binding.name == "while" && binding.identity == "while")
    );
    assert!(
        compiled.procedures["::p"]
            .instructions
            .iter()
            .any(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.source_cmd_text == "while {0} {}"
            })
    );

    eval_ok(&mut vm, "proc p {} {mutate; while {0} {}}");
    eval_ok(
        &mut vm,
        "proc mutate {} {rename while real_while; proc while args {return WHILE_OVERRIDE}}",
    );
    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    let result = eval_ok(&mut vm, "p");
    assert!(plain_calls.get() > 0, "the live while frame must deopt");
    assert_eq!(result, "WHILE_OVERRIDE");
}

#[test]
fn active_proc_frame_revalidates_a_cfg_consumed_eval_head() {
    let (mut vm, _output) = vm();
    let source = "proc p {} {mutate; eval {set ::body_ran 1}}";
    let compiled = BytecodeCompileService::default()
        .compile(source)
        .expect("module compiles");
    assert!(
        compiled.procedures["::p"]
            .command_bindings
            .iter()
            .any(|binding| binding.name == "eval" && binding.identity == "eval")
    );
    assert!(
        compiled.procedures["::p"]
            .instructions
            .iter()
            .any(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.source_cmd_text == "eval {set ::body_ran 1}"
            })
    );

    eval_ok(
        &mut vm,
        "set ::body_ran 0; proc p {} {mutate; eval {set ::body_ran 1}}",
    );
    eval_ok(
        &mut vm,
        "proc mutate {} {rename eval real_eval; proc eval args {return EVAL_OVERRIDE}}",
    );
    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));

    assert_eq!(eval_ok(&mut vm, "p"), "EVAL_OVERRIDE");
    assert_eq!(eval_ok(&mut vm, "set ::body_ran"), "0");
    assert!(plain_calls.get() > 0, "the live eval frame must deopt");
}

#[test]
fn stale_constant_if_boundary_replays_only_the_owning_command() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "set ::mutations 0; set ::body_ran 0; \
         proc mutate {} {incr ::mutations; rename if real_if; proc if args {return IF_OVERRIDE}}; \
         proc p {} {mutate; if {1} {set ::body_ran 1}}",
    );
    assert_eq!(eval_ok(&mut vm, "p"), "IF_OVERRIDE");
    assert_eq!(eval_ok(&mut vm, "list $::mutations $::body_ran"), "1 0");
}

#[test]
fn replacement_delete_trace_reentry_cannot_acknowledge_the_preinstall_epoch() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc generator {} {yield ready; set first [expr {1+1}]; yield $first; expr {2+2}}; \
         coroutine c generator; \
         proc resume_on_delete args {set ::during_delete [c]}; \
         trace add command expr delete resume_on_delete; \
         proc expr args {return EXPR_OVERRIDE}",
    );

    assert_eq!(
        eval_ok(&mut vm, "list $::during_delete [c]"),
        "2 EXPR_OVERRIDE"
    );
}

#[test]
fn active_proc_frame_redispatches_after_namespace_path_mutation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "namespace eval Override {proc expr args {return PATH_OVERRIDE}}; \
         namespace eval N { \
             proc mutate {} {namespace path ::Override}; \
             proc p {} {mutate; expr {1+2}} \
         }",
    );

    // Tcl 9.0.4: PATH_OVERRIDE. `p` entered with unqualified `expr`
    // resolving to the global builtin, but its preceding command changes N's
    // resolution path while the optimised frame is live. The next source
    // command must revalidate before executing its stale expression opcodes.
    assert_eq!(eval_ok(&mut vm, "N::p"), "PATH_OVERRIDE");
}

#[test]
fn fused_expr_alias_uses_the_alias_binding_identity() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("interp alias {} e {} expr; proc p {} {set x [e {1+2}]; return $x}")
        .expect("module compiles");
    assert!(
        compiled.procedures["::p"]
            .command_bindings
            .iter()
            .any(|binding| binding.name == "e" && binding.identity == "expr"),
        "the fused alias must retain its source and target identities"
    );
    eval_ok(
        &mut vm,
        "interp alias {} e {} expr; \
         proc p {} {set x [e {1+2}]; return $x}",
    );
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    eval_ok(
        &mut vm,
        "interp alias {} e {} {}; proc e args {return ALIAS_OVERRIDE}",
    );

    // Tcl 9.0.4: ALIAS_OVERRIDE. The optimised AssignExpr assumption belongs
    // to `e -> expr`, not to the unrelated live spelling `expr`.
    assert_eq!(eval_ok(&mut vm, "p"), "ALIAS_OVERRIDE");
}

#[test]
fn fused_expr_alias_tracks_its_source_implementation() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "interp alias {} e {} expr; \
         proc p {} {set x [e {1+2}]; return $x}",
    );
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return TARGET_OVERRIDE}",
    );

    // The source spelling `e` is unchanged, but it no longer reaches the
    // registry `expr` implementation. Its target identity is part of the same
    // dependency stamp, so the cached proc redispatches through the alias.
    assert_eq!(eval_ok(&mut vm, "p"), "TARGET_OVERRIDE");
}

#[test]
fn reusable_module_is_revalidated_after_command_mutation() {
    let (mut vm, _output) = vm();
    let module = BytecodeCompileService::default()
        .compile("expr {1+2}")
        .expect("module compiles");
    assert_eq!(vm.run_module(&module).result.to_str().as_ref(), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return MODULE_OVERRIDE}",
    );

    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "MODULE_OVERRIDE");
}

#[test]
fn reusable_function_handle_is_revalidated_after_command_mutation() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return HANDLE_OVERRIDE}",
    );

    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "HANDLE_OVERRIDE");
}

#[test]
fn native_reentry_refreshes_function_handle_in_the_invoking_namespace() {
    let (mut vm, _output) = vm();
    vm.register_native_command(
        "host_handle",
        Rc::new(CompileOrInvokeHandle(RefCell::new(None))),
    );
    eval_ok(
        &mut vm,
        "namespace eval ::a {}; namespace eval ::b {proc expr args {return B}}",
    );

    // The command epoch is unchanged between these two native re-entries. The
    // reusable artifact must still refresh because its unqualified `expr` was
    // specialised in ::a and is now being invoked under ::b's local shadow.
    assert_eq!(
        eval_ok(
            &mut vm,
            "namespace eval ::a {host_handle compile}; \
             namespace eval ::b {host_handle invoke}",
        ),
        "B",
    );
}

#[test]
fn native_reentry_recompiles_module_in_the_running_namespace() {
    let profile = DialectProfile::plain_tcl();
    let module = BytecodeCompileService::default()
        .compile_script_for_profile(
            ScriptCompileTarget {
                source: "expr {1+2}",
                namespace: "a",
            },
            profile,
        )
        .expect("namespaced module compiles");
    assert_eq!(module.source_namespace, "a");

    let (mut vm, _output) = vm();
    vm.register_native_command("host_run", Rc::new(RunCompiledModule(Rc::new(module))));
    eval_ok(
        &mut vm,
        "namespace eval ::a {}; namespace eval ::b {proc expr args {return B}}",
    );

    assert_eq!(
        eval_ok(&mut vm, "namespace eval ::b {host_run}"),
        "B",
        "run_module must not execute ::a-specialised bytecode under ::b",
    );
}

#[test]
fn native_reentry_rejects_a_bare_function_from_another_namespace() {
    let profile = DialectProfile::plain_tcl();
    let function = BytecodeCompileService::default()
        .compile_script_for_profile(
            ScriptCompileTarget {
                source: "expr {1+2}",
                namespace: "a",
            },
            profile,
        )
        .expect("namespaced function compiles")
        .top_level;

    let (mut vm, _output) = vm();
    vm.register_native_command(
        "host_run_function",
        Rc::new(RunCompiledFunction(Rc::new(function))),
    );
    eval_ok(
        &mut vm,
        "namespace eval ::a {}; namespace eval ::b {proc expr args {return B}}",
    );

    assert_eq!(
        eval_ok(
            &mut vm,
            "namespace eval ::b {catch {host_run_function} message; set message}",
        ),
        "stale profile-less bytecode has no source for plain dispatch",
        "source-less FunctionAsm must fail closed outside its recorded resolution namespace",
    );
}

#[test]
fn reusable_consumers_revalidate_command_execution_traces() {
    fn install_trace(vm: &mut Vm) {
        eval_ok(
            vm,
            "set ::traced 0; proc trace_expr {command op} {\
             if {$command eq {expr 1+2}} {incr ::traced}}; \
             trace add execution expr enter trace_expr",
        );
    }

    let module = BytecodeCompileService::default()
        .compile("expr {1+2}")
        .expect("module compiles");
    let (mut module_vm, _output) = vm();
    assert_eq!(module_vm.run_module(&module).result.to_str().as_ref(), "3");
    install_trace(&mut module_vm);
    assert_eq!(module_vm.run_module(&module).result.to_str().as_ref(), "3");
    assert_eq!(eval_ok(&mut module_vm, "set ::traced"), "1");

    let (mut handle_vm, _output) = vm();
    let handle = handle_vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    assert_eq!(
        handle_vm.invoke_function(&handle).result.to_str().as_ref(),
        "3"
    );
    install_trace(&mut handle_vm);
    assert_eq!(
        handle_vm.invoke_function(&handle).result.to_str().as_ref(),
        "3"
    );
    assert_eq!(eval_ok(&mut handle_vm, "set ::traced"), "1");

    let (mut eval_vm, _output) = vm();
    assert_eq!(eval_ok(&mut eval_vm, "expr {1+2}"), "3");
    install_trace(&mut eval_vm);
    assert_eq!(eval_ok(&mut eval_vm, "expr {1+2}"), "3");
    assert_eq!(eval_ok(&mut eval_vm, "set ::traced"), "1");
}

#[test]
fn mutation_recovery_fails_closed_without_plain_dispatch_capability() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return SHOULD_NOT_RUN_OPTIMISED}",
    );
    vm.set_compiler(Box::new(OptimisedOnlyCompilerSvc(
        BytecodeCompileService::default(),
    )));

    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Error, "{completion:?}");
    assert!(
        completion
            .result
            .to_str()
            .contains("does not support plain command dispatch"),
        "{}",
        completion.result.to_str()
    );
}

#[test]
fn non_expr_codegen_hook_redispatches_after_replacement() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("proc p {} {mutate; llength {a b c}}")
        .expect("module compiles");
    assert!(
        compiled.procedures["::p"]
            .command_bindings
            .iter()
            .any(|binding| binding.name == "llength" && binding.identity == "llength")
    );
    eval_ok(
        &mut vm,
        "proc mutate {} {rename llength real_llength; proc llength args {return LIST_OVERRIDE}}; \
         proc p {} {mutate; llength {a b c}}",
    );

    // `llength` is registry-hooked independently of AssignExpr; the same
    // binding-generation contract protects it.
    assert_eq!(eval_ok(&mut vm, "p"), "LIST_OVERRIDE");
}

#[test]
#[allow(clippy::too_many_lines)] // One table keeps the Tcl 9.0.4 differential cases uniform.
fn argument_substitution_mutation_keeps_entered_token_then_deopts_at_next_command() {
    struct Case {
        name: &'static str,
        source: &'static str,
        want: &'static str,
    }

    let cases = [
        Case {
            name: "direct replacement",
            source: concat!(
                "set ::replacement_called 0\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {",
                "set ::replacement_called 1; return REPLACED}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "llength [mutate]; ",
                "list $::replacement_called [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "0 REPLACED\n",
        },
        Case {
            name: "direct rename away",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "return {a b c}}\n",
                "proc p {} {",
                "llength [mutate]; ",
                "set code [catch {llength {d e}} result]; ",
                "list $code $result}\n",
                "puts [p]\n",
            ),
            want: "1 {invalid command name \"llength\"}\n",
        },
        Case {
            name: "direct forced import",
            source: concat!(
                "set ::import_called 0\n",
                "namespace eval alt {",
                "proc llength args {",
                "set ::import_called 1; return IMPORTED}; ",
                "namespace export llength}\n",
                "namespace eval N {",
                "namespace import ::llength; ",
                "proc mutate {} {",
                "namespace import -force ::alt::llength; ",
                "return {a b c}}; ",
                "proc p {} {",
                "llength [mutate]; ",
                "list $::import_called [llength {d e}]}}\n",
                "puts [N::p]\n",
            ),
            want: "0 IMPORTED\n",
        },
        Case {
            name: "nested replacement",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "3 REPLACED\n",
        },
        Case {
            name: "nested rename away",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "set code [catch {llength {d e}} result]; ",
                "list $first $code $result}\n",
                "puts [p]\n",
            ),
            want: "3 1 {invalid command name \"llength\"}\n",
        },
        Case {
            name: "nested forced import",
            source: concat!(
                "namespace eval alt {",
                "proc llength args {return IMPORTED}; ",
                "namespace export llength}\n",
                "namespace eval N {",
                "namespace import ::llength; ",
                "proc mutate {} {",
                "namespace import -force ::alt::llength; ",
                "return {a b c}}; ",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}}\n",
                "puts [N::p]\n",
            ),
            want: "3 IMPORTED\n",
        },
        Case {
            name: "ordinary generic command resolves after substitution",
            source: concat!(
                "proc foo x {return OLD:$x}\n",
                "proc mutate {} {",
                "rename foo saved_foo; ",
                "proc foo x {return NEW:$x}; ",
                "return X}\n",
                "proc p {} {foo [mutate]}\n",
                "puts [p]\n",
            ),
            want: "NEW:X\n",
        },
        Case {
            name: "wrong-arity builtin remains generic",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return X}\n",
                "proc p {} {llength [mutate] extra}\n",
                "puts [p]\n",
            ),
            want: "NEW\n",
        },
        Case {
            name: "outer generic command does not replace inner marker",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return {a b c}}\n",
                "proc p {} {puts [llength [mutate]]}\n",
                "p\n",
            ),
            want: "3\n",
        },
        Case {
            name: "dynamic head remains late resolved",
            source: concat!(
                "set ::replacement_called 0\n",
                "set ::payload {a b c}\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {",
                "set ::replacement_called 1; return REPLACED}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set command llength; ",
                "$command [mutate]; ",
                "list $::replacement_called [$command {d e}]}\n",
                "puts [p]\n",
            ),
            want: "1 REPLACED\n",
        },
        Case {
            name: "direct inner command revalidates while outer token stays entered",
            source: concat!(
                "set ::replacement_called 0\n",
                "set ::payload {a b c}\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {",
                "set ::replacement_called 1; return REPLACED}; ",
                "return ignored}\n",
                "proc p {} {",
                "llength [mutate; llength {d e}; set ::payload]}\n",
                "puts [list [p] $::replacement_called]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "nested inner command revalidates while outer token stays entered",
            source: concat!(
                "set ::replacement_called 0\n",
                "set ::payload {a b c}\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {",
                "set ::replacement_called 1; return REPLACED}; ",
                "return ignored}\n",
                "proc p {} {",
                "set outer [llength [mutate; llength {d e}; set ::payload]]; ",
                "list $outer $::replacement_called}\n",
                "puts [p]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "expanded nested replacement resolves after expansion",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}; ",
                "return abc}\n",
                "proc p {} {",
                "set first [llength {*}[mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "REPLACED REPLACED\n",
        },
        Case {
            name: "execution trace plain-dispatch resolves after substitution",
            source: concat!(
                "proc callback args {}\n",
                "trace add execution llength enter callback\n",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "REPLACED REPLACED\n",
        },
        Case {
            name: "pre-existing execution trace deoptimises specialised opcode",
            source: concat!(
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {llength {a b}}} {incr ::traced}}\n",
                "trace add execution llength enter callback\n",
                "proc p {} {llength {a b}}\n",
                "puts [list [p] $::traced]\n",
            ),
            want: "2 1\n",
        },
        Case {
            name: "first argument substitution cannot pre-empt command entry",
            source: concat!(
                "set ::x A\n",
                "proc noop {} {return B}\n",
                "proc callback args {",
                "trace remove variable ::x read callback; ",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}}\n",
                "trace add variable ::x read callback\n",
                "proc p {} {",
                "set first [llength $::x[noop]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "1 REPLACED\n",
        },
        Case {
            name: "lassign typed hook retains its entered command",
            source: concat!(
                "proc mutate {} {",
                "rename lassign saved_lassign; ",
                "proc lassign args {return NEW}; ",
                "return {a b}}\n",
                "proc p {} {",
                "set a 0; set b 0; ",
                "set first [lassign [mutate] a b]; ",
                "list $a $b $first [lassign {x} a]}\n",
                "puts [p]\n",
            ),
            want: "a b {} NEW\n",
        },
        Case {
            name: "append typed hook retains its entered command",
            source: concat!(
                "proc mutate {} {",
                "rename append saved_append; ",
                "proc append args {return NEW}; ",
                "return X}\n",
                "proc p {} {",
                "set x A; ",
                "set first [append x [mutate]]; ",
                "list $first $x [append x B]}\n",
                "puts [p]\n",
            ),
            want: "AX AX NEW\n",
        },
        Case {
            name: "concat typed hook retains its entered command",
            source: concat!(
                "proc mutate {} {",
                "rename concat saved_concat; ",
                "proc concat args {return NEW}; ",
                "return {a b}}\n",
                "proc p {} {",
                "set first [concat [mutate] c]; ",
                "list $first [concat d e]}\n",
                "puts [p]\n",
            ),
            want: "{a b c} NEW\n",
        },
        Case {
            name: "lrange typed hook survives inline-hook distrust",
            source: concat!(
                "proc mutate {} {",
                "rename lrange saved_lrange; ",
                "proc lrange args {return NEW}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [lrange [mutate] 0 1]; ",
                "list $first [lrange {d e} 0 0]}\n",
                "puts [p]\n",
            ),
            want: "{a b} NEW\n",
        },
        Case {
            name: "linsert typed hook survives inline-hook distrust",
            source: concat!(
                "proc mutate {} {",
                "rename linsert saved_linsert; ",
                "proc linsert args {return NEW}; ",
                "return {a b}}\n",
                "proc p {} {",
                "set first [linsert [mutate] 1 X]; ",
                "list $first [linsert {d e} 1 Y]}\n",
                "puts [p]\n",
            ),
            want: "{a X b} NEW\n",
        },
        Case {
            name: "namespace-local proc never acquires builtin entry timing",
            source: concat!(
                "namespace eval N {",
                "proc llength args {return OLD}; ",
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return {a b}}; ",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {c d}]}}\n",
                "puts [N::p]\n",
            ),
            want: "NEW NEW\n",
        },
        Case {
            name: "trace added during namespace-local proc argument applies immediately",
            source: concat!(
                "namespace eval N {",
                "set traced 0; ",
                "proc llength args {return OLD}; ",
                "proc callback {command op} {incr ::N::traced}; ",
                "proc mutate {} {",
                "trace add execution llength enter callback; ",
                "return {a b}}; ",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first $::N::traced}}\n",
                "puts [N::p]\n",
            ),
            want: "OLD 1\n",
        },
        Case {
            name: "coroutine initial command uses traced plain admission",
            source: concat!(
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {expr 1+2}} {incr ::traced}}\n",
                "trace add execution expr enter callback\n",
                "set result [coroutine c expr {1+2}]\n",
                "puts [list $result $::traced]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "cached proc tracks first trace add and last removal",
            source: concat!(
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {llength {a b}}} {incr ::traced}}\n",
                "proc p {} {llength {a b}}\n",
                "set before [p]\n",
                "trace add execution llength enter callback\n",
                "set during [p]\n",
                "trace remove execution llength enter callback\n",
                "set after [p]\n",
                "puts [list $before $during $after $::traced]\n",
            ),
            want: "2 2 2 1\n",
        },
        Case {
            name: "execution trace on alias source deoptimises fused target",
            source: concat!(
                "interp alias {} e {} expr\n",
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {e 1+2}} {incr ::traced}}\n",
                "trace add execution e enter callback\n",
                "proc p {} {e {1+2}}\n",
                "puts [list [p] $::traced]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "execution trace on alias target deoptimises fused source",
            source: concat!(
                "interp alias {} e {} expr\n",
                "set ::traced 0\n",
                "proc callback {command op} {",
                "if {$command eq {expr 1+2}} {incr ::traced}}\n",
                "trace add execution expr enter callback\n",
                "proc p {} {e {1+2}}\n",
                "puts [list [p] $::traced]\n",
            ),
            want: "3 1\n",
        },
        Case {
            name: "trace added after entry does not change entered token",
            source: concat!(
                "proc callback args {}\n",
                "proc mutate {} {",
                "trace add execution llength enter callback; ",
                "rename llength saved_llength; ",
                "proc llength args {return NEWA}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "3 NEWA\n",
        },
        Case {
            name: "trace added after entry applies only to later calls",
            source: concat!(
                "set ::traced 0\n",
                "proc callback args {incr ::traced}\n",
                "proc mutate {} {",
                "trace add execution llength enter callback; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "set before $::traced; ",
                "set second [llength {d e}]; ",
                "list $first $before $second $::traced}\n",
                "puts [p]\n",
            ),
            want: "3 0 2 1\n",
        },
        Case {
            name: "trace removed after entry keeps late resolution",
            source: concat!(
                "proc callback args {}\n",
                "trace add execution llength enter callback\n",
                "proc mutate {} {",
                "trace remove execution llength enter callback; ",
                "rename llength saved_llength; ",
                "proc llength args {return NEWB}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set first [llength [mutate]]; ",
                "list $first [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "NEWB NEWB\n",
        },
        Case {
            name: "catch body retains eligible specialised entry",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "set rc [catch {llength [mutate]} result]; ",
                "list $rc $result}\n",
                "puts [p]\n",
            ),
            want: "0 3\n",
        },
        Case {
            name: "try handler retains eligible specialised entry",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return NEW}; ",
                "return {a b c}}\n",
                "proc p {} {",
                "try {error boom} on error {m} {llength [mutate]}}\n",
                "puts [p]\n",
            ),
            want: "3\n",
        },
        Case {
            name: "suspended command retains entered token across mutation",
            source: concat!(
                "proc p {} {llength [yield PAUSE]}\n",
                "puts [coroutine c p]\n",
                "rename llength saved_llength\n",
                "proc llength args {return NEW}\n",
                "puts [c {a b c}]\n",
            ),
            want: "PAUSE\n3\n",
        },
        Case {
            name: "failed substitution prunes entered token before catch continuation",
            source: concat!(
                "proc mutate {} {",
                "rename llength saved_llength; ",
                "proc llength args {return REPLACED}; ",
                "error BOOM}\n",
                "proc p {} {",
                "set code [catch {set first [llength [mutate]]} result]; ",
                "list $code $result [llength {d e}]}\n",
                "puts [p]\n",
            ),
            want: "1 BOOM REPLACED\n",
        },
        Case {
            name: "invoke replace keeps specialised entered command",
            source: concat!(
                "proc mutate {} {",
                "rename string saved_string; ",
                "proc string args {return STRING_REPLACED}; ",
                "return x}\n",
                "proc p {} {",
                "set first [string equal -nocase [mutate] X]; ",
                "list $first [string equal x x]}\n",
                "puts [p]\n",
            ),
            want: "1 STRING_REPLACED\n",
        },
        Case {
            name: "invoke replace resolves internal implementation after substitution",
            source: concat!(
                "namespace eval ::tcl::string {}\n",
                "proc mutate {} {",
                "rename ::tcl::string::equal saved_equal; ",
                "proc ::tcl::string::equal args {return INTERNAL_REPLACED}; ",
                "return x}\n",
                "proc p {} {",
                "string equal -nocase [mutate] X}\n",
                "puts [p]\n",
            ),
            // A later ordinary `string equal` currently belongs to the
            // registry/ensemble deduplication tracked by #1607; this vector
            // isolates INVOKE_REPLACE's same-invocation lookup timing.
            want: "INTERNAL_REPLACED\n",
        },
    ];

    // Tcl 9.0.4 resolves the specialised command token before evaluating its
    // remaining words. The token therefore survives mutations performed by an
    // argument substitution, while the following source command is the replay
    // boundary and resolves the replacement, missing name, or forced import.
    // Revalidating the outer operation itself would disagree with C Tcl and
    // would also risk evaluating a substitution twice.
    let oracle = tcl_test_support::locate_tclsh(TclVersion::V9_0)
        .expect("the Tcl 9.0 oracle override is valid");
    let marker_module = BytecodeCompileService::default()
        .compile("proc p {} {set first [llength [mutate]]; return $first}")
        .expect("literal nested command compiles");
    assert!(
        marker_module.procedures["::p"]
            .instructions
            .iter()
            .any(|instruction| {
                matches!(
                    instruction.op,
                    tcl_bytecode::Op::PUSH1 | tcl_bytecode::Op::PUSH4
                ) && instruction.entered_command.as_ref().is_some_and(|entered| {
                    entered.binding.name == "llength"
                        && entered.binding.identity == "llength"
                        && !entered.end.is_empty()
                })
            }),
        "compiler lost the typed literal entered-command marker"
    );
    assert!(
        !marker_module.procedures["::p"]
            .instructions
            .iter()
            .any(|instruction| {
                instruction.op == tcl_bytecode::Op::START_CMD
                    && instruction.entered_command.is_some()
            }),
        "entered-command metadata belongs to the literal head, not a nested substitution"
    );
    let expanded_marker_module = BytecodeCompileService::default()
        .compile("proc p {} {set first [llength {*}[mutate]]; return $first}")
        .expect("expanded literal nested command compiles");
    assert!(
        !expanded_marker_module.procedures["::p"]
            .instructions
            .iter()
            .any(
                |instruction| instruction.entered_command.as_ref().is_some_and(|entered| {
                    entered.binding.name == "llength" && !entered.end.is_empty()
                })
            ),
        "expanded commands resolve after expansion and must not retain a token"
    );
    let invoke_replace_marker_module = BytecodeCompileService::default()
        .compile("proc p {} {set first [string equal -nocase [mutate] X]; return $first}")
        .expect("literal string ensemble command compiles");
    assert!(
        invoke_replace_marker_module.procedures["::p"]
            .instructions
            .iter()
            .any(|instruction| {
                matches!(
                    instruction.op,
                    tcl_bytecode::Op::PUSH1 | tcl_bytecode::Op::PUSH4
                ) && instruction.entered_command.as_ref().is_some_and(|entered| {
                    entered.binding.name == "string"
                        && entered.binding.identity == "string"
                        && !entered.end.is_empty()
                })
            }),
        "INVOKE_REPLACE must retain the typed string ensemble token"
    );
    for case in cases {
        let (mut vm, output) = vm();
        eval_ok(&mut vm, case.source);
        let actual = String::from_utf8(output.0.borrow().clone()).expect("UTF-8 VM output");
        assert_eq!(actual, case.want, "{}: TclVM", case.name);

        if let Some(tclsh) = &oracle {
            assert_eq!(
                tclsh.patchlevel,
                tcl_test_support::reference_patchlevel(TclVersion::V9_0),
                "exact Tcl oracle pin"
            );
            let outcome = tcl_test_support::run_script(&tclsh.path, case.source.as_bytes())
                .expect("Tcl 9.0.4 oracle runs");
            let oracle_output = outcome.strict_text().expect("Tcl 9.0.4 succeeds");
            assert_eq!(
                format!("{oracle_output}\n"),
                case.want,
                "{}: Tcl 9.0.4 oracle",
                case.name
            );
        }
    }
}

#[test]
fn generated_procedure_body_keeps_proc_locals_and_entered_append_token() {
    let (mut vm, output) = vm();
    eval_ok(
        &mut vm,
        concat!(
            "proc mutate {} {",
            "rename append saved_append; ",
            "proc append args {return NEW}; ",
            "return X}\n",
            "set generated {",
            "set x A; ",
            "set first [append x [mutate]]; ",
            "list $first $x [append x B]}\n",
            "proc generated_proc {} $generated\n",
            "puts [generated_proc]\n",
        ),
    );
    assert_eq!(
        String::from_utf8(output.0.borrow().clone()).unwrap(),
        "AX AX NEW\n"
    );
}

#[test]
fn apply_and_tcloo_bodies_compile_with_parameter_lvt_context() {
    let (mut vm, output) = vm();
    eval_ok(
        &mut vm,
        concat!(
            "puts [apply {{x suffix} {append x $suffix; return $x}} A X]\n",
            "oo::class create C {method join {x suffix} {append x $suffix; return $x}}\n",
            "puts [[C new] join A X]\n",
        ),
    );
    assert_eq!(
        String::from_utf8(output.0.borrow().clone()).unwrap(),
        "AX\nAX\n"
    );
}

#[test]
fn precompiled_proc_cache_requires_exact_body_and_parameters() {
    let (mut vm, output) = vm();
    eval_ok(
        &mut vm,
        concat!(
            "proc p {x} {return FIRST:$x}\n",
            "rename p old_p\n",
            "proc p {} {return SECOND}\n",
            "puts [list [old_p A] [p]]\n",
        ),
    );
    assert_eq!(
        String::from_utf8(output.0.borrow().clone()).unwrap(),
        "FIRST:A SECOND\n"
    );
}

#[test]
fn exact_static_proc_provenance_preserves_the_precompiled_body() {
    let fast_calls = Rc::new(Cell::new(0));
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::new(Cell::new(0)),
    }));
    eval_ok(&mut vm, "proc p {x} {return $x}");
    assert_eq!(fast_calls.get(), 1, "the outer module compiled once");

    let completion = vm.invoke_command("p", &[Value::string("kept")]);
    assert!(completion.code.is_ok(), "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "kept");
    assert_eq!(
        fast_calls.get(),
        1,
        "exact provenance should admit the module's precompiled proc body"
    );
}

#[test]
fn non_expr_lowering_hook_redispatches_after_replacement() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("proc p {} {mutate; set x OLD}")
        .expect("module compiles");
    assert!(
        compiled.procedures["::p"]
            .command_bindings
            .iter()
            .any(|binding| binding.name == "set" && binding.identity == "set"),
        "typed lowering must retain the registry command it specialised"
    );
    eval_ok(&mut vm, "proc p {} {mutate; set x OLD}");
    eval_ok(
        &mut vm,
        "proc mutate {} {rename set real_set; proc set args {return LOWERING_OVERRIDE}}",
    );

    // Tcl 9.0.4: LOWERING_OVERRIDE. Typed lowering is a registry-command
    // assumption just like a codegen hook; an active frame must not execute
    // the stale AssignConst after the nested mutation.
    assert_eq!(eval_ok(&mut vm, "p"), "LOWERING_OVERRIDE");
}

#[test]
fn escaped_head_stays_on_live_dispatch_after_replacement() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile(r"proc p {} {mutate; se\x74 x OLD}")
        .expect("module compiles");
    assert!(
        !compiled.procedures["::p"]
            .command_bindings
            .iter()
            .any(|binding| binding.name == "set"),
        "an unresolved escaped head must not acquire forged lowering provenance",
    );
    eval_ok(&mut vm, r"proc p {} {mutate; se\x74 x OLD}");
    eval_ok(
        &mut vm,
        "proc mutate {} {rename set real_set; proc set args {return ESCAPED_OVERRIDE}}",
    );

    // Tcl 9.0.4: ESCAPED_OVERRIDE. This spelling is intentionally not a typed
    // lowering candidate yet, so its generic invoke resolves the replacement
    // live without needing a speculative binding reconstructed from source.
    assert_eq!(eval_ok(&mut vm, "p"), "ESCAPED_OVERRIDE");
}

#[test]
fn constant_fold_dependency_redispatches_after_replacement() {
    let (mut vm, _output) = vm();
    let compiled = BytecodeCompileService::default()
        .compile("set x [list a b]; set x")
        .expect("module compiles");
    assert!(
        compiled
            .top_level
            .command_bindings
            .iter()
            .any(|binding| binding.name == "list" && binding.identity == "list"),
        "constant-folded commands must carry the same binding dependency as hooks"
    );
    let handle = vm
        .compile_function("set x [list a b]; set x")
        .expect("function compiles");
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "a b");
    eval_ok(
        &mut vm,
        "rename list real_list; proc list args {return FOLD_OVERRIDE}",
    );

    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "FOLD_OVERRIDE");
}

#[test]
fn active_constant_fold_replays_after_an_earlier_argument_mutates_its_binding() {
    let (mut vm, _output) = vm();

    // Compile the mutator and consumer in separate units. The consumer cannot
    // see through the already-installed `mutate` proc while deciding whether
    // its later constant command substitution is foldable.
    eval_ok(
        &mut vm,
        "proc mutate {} {\
             rename list saved_list; \
             proc list args {return REPLACED}; \
             return X\
         }",
    );
    eval_ok(&mut vm, "proc p {} {concat [mutate] [list a b]}");

    // Tcl 9.0.4: the first substitution replaces `list`; the folded second
    // substitution must redispatch at its own boundary inside the same active
    // outer command.
    assert_eq!(eval_ok(&mut vm, "p"), "X REPLACED");
}

#[test]
fn top_level_fold_replays_after_a_specialised_write_trace_mutates_its_binding() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        r"proc replace_list {name1 name2 op} {
             rename list saved_list
             proc list args {return TRACE_REPLACED}
         }
         set trigger 0
         trace add variable trigger write replace_list",
    );
    let handle = vm
        .compile_function("set a([incr trigger]) [list x y]; set a(1)")
        .expect("function compiles");

    // No generic invoke exists in this top-level unit. The specialised INCR
    // fires a write trace and advances the command epoch before the folded
    // list executes, so its nested replay marker must survive peephole passes.
    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "TRACE_REPLACED");
}

#[test]
fn top_level_dict_fold_replay_skips_stale_verification_and_payload() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        r"proc replace_dict {name1 name2 op} {
             rename dict saved_dict
             proc dict args {return NOT_A_DICT}
         }
         set trigger 0
         trace add variable trigger write replace_dict",
    );
    let handle = vm
        .compile_function("set a([incr trigger]) [dict create k v]; set a(1)")
        .expect("function compiles");

    // Replaying the replacement resumes after the entire stale folded range,
    // including VERIFY_DICT; the replacement's non-dict result is legitimate.
    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "NOT_A_DICT");
}

#[test]
fn restored_builtin_identity_does_not_leave_the_vm_permanently_untrusted() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    eval_ok(
        &mut vm,
        "rename expr held_expr; proc expr args {return OVERRIDE}; \
         rename expr {}; rename held_expr expr",
    );
    vm.set_compiler(Box::new(OptimisedOnlyCompilerSvc(
        BytecodeCompileService::default(),
    )));

    // Tcl 9.0.4 returns 3 after the original token is renamed back. The live
    // binding once again satisfies the handle's dependency, so invocation
    // must not demand a plain compiler merely because an epoch changed.
    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "3");
}

#[test]
fn proc_reoptimises_after_running_plain_during_a_builtin_replacement() {
    let (mut vm, _output) = vm();
    eval_ok(&mut vm, "proc p {} {expr {1+2}}");
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    eval_ok(
        &mut vm,
        "rename expr held_expr; proc expr args {return OVERRIDE}",
    );

    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(eval_ok(&mut vm, "p"), "OVERRIDE");
    assert!(plain_calls.get() > 0, "the replacement must deopt the proc");

    eval_ok(&mut vm, "rename expr {}; rename held_expr expr");
    fast_calls.set(0);
    plain_calls.set(0);
    assert_eq!(eval_ok(&mut vm, "p"), "3");
    assert_eq!(fast_calls.get(), 1, "the proc must re-enter fast bytecode");
    assert_eq!(plain_calls.get(), 0);
}

#[test]
fn function_handle_reoptimises_after_running_plain_during_a_builtin_replacement() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
    eval_ok(
        &mut vm,
        "rename expr held_expr; proc expr args {return OVERRIDE}",
    );

    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(
        vm.invoke_function(&handle).result.to_str().as_ref(),
        "OVERRIDE"
    );
    assert!(
        plain_calls.get() > 0,
        "the replacement must deopt the handle"
    );

    eval_ok(&mut vm, "rename expr {}; rename held_expr expr");
    fast_calls.set(0);
    plain_calls.set(0);
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
    assert_eq!(
        fast_calls.get(),
        1,
        "the handle must re-enter fast bytecode"
    );
    assert_eq!(plain_calls.get(), 0);
}

#[test]
fn proc_reoptimises_after_running_plain_under_a_removed_step_trace() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "proc p {} {expr {1+2}}; proc step_callback args {}",
    );
    assert_eq!(vm.dispatch("p", &[]).result.to_str().as_ref(), "3");
    eval_ok(&mut vm, "trace add execution p enterstep step_callback");

    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(vm.dispatch("p", &[]).result.to_str().as_ref(), "3");
    assert!(
        plain_calls.get() > 0,
        "the active step trace must deopt the proc"
    );

    eval_ok(&mut vm, "trace remove execution p enterstep step_callback");
    fast_calls.set(0);
    plain_calls.set(0);
    assert_eq!(vm.dispatch("p", &[]).result.to_str().as_ref(), "3");
    assert_eq!(fast_calls.get(), 1, "trace removal must restore fast code");
    assert_eq!(plain_calls.get(), 0);
}

#[test]
fn tcloo_method_reoptimises_after_running_plain_during_a_builtin_replacement() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "oo::class create C {method m {} {expr {1+2}}}; C create o",
    );
    assert_eq!(
        vm.dispatch("o", &[Value::string("m")])
            .result
            .to_str()
            .as_ref(),
        "3"
    );
    eval_ok(
        &mut vm,
        "rename expr held_expr; proc expr args {return METHOD_OVERRIDE}",
    );

    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(eval_ok(&mut vm, "o m"), "METHOD_OVERRIDE");
    assert!(
        plain_calls.get() > 0,
        "the replacement must deopt the cached method"
    );

    eval_ok(&mut vm, "rename expr {}; rename held_expr expr");
    fast_calls.set(0);
    plain_calls.set(0);
    assert_eq!(
        vm.dispatch("o", &[Value::string("m")])
            .result
            .to_str()
            .as_ref(),
        "3"
    );
    assert_eq!(
        fast_calls.get(),
        1,
        "the method must re-enter fast bytecode"
    );
    assert_eq!(plain_calls.get(), 0);
}

#[test]
fn tcloo_method_reoptimises_after_running_plain_under_a_removed_step_trace() {
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "oo::class create C {method m {} {expr {1+2}}}; \
         C create o; proc traced {} {}; proc step_callback args {}",
    );
    assert_eq!(
        vm.dispatch("o", &[Value::string("m")])
            .result
            .to_str()
            .as_ref(),
        "3"
    );
    eval_ok(
        &mut vm,
        "trace add execution traced enterstep step_callback",
    );

    let fast_calls = Rc::new(Cell::new(0));
    let plain_calls = Rc::new(Cell::new(0));
    vm.set_compiler(Box::new(CountingCompilerSvc {
        inner: BytecodeCompileService::default(),
        fast_calls: Rc::clone(&fast_calls),
        plain_calls: Rc::clone(&plain_calls),
    }));
    assert_eq!(
        vm.dispatch("o", &[Value::string("m")])
            .result
            .to_str()
            .as_ref(),
        "3"
    );
    assert!(
        plain_calls.get() > 0,
        "the active step trace must deopt the cached method"
    );

    eval_ok(
        &mut vm,
        "trace remove execution traced enterstep step_callback",
    );
    fast_calls.set(0);
    plain_calls.set(0);
    assert_eq!(
        vm.dispatch("o", &[Value::string("m")])
            .result
            .to_str()
            .as_ref(),
        "3"
    );
    assert_eq!(
        fast_calls.get(),
        1,
        "trace removal must restore fast method code"
    );
    assert_eq!(plain_calls.get(), 0);
}

#[test]
fn hide_and_expose_lifecycle_mutations_revalidate_artifacts() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    eval_ok(
        &mut vm,
        "interp hide {} expr held; proc expr args {return HIDDEN_OVERRIDE}",
    );
    assert_eq!(
        vm.invoke_function(&handle).result.to_str().as_ref(),
        "HIDDEN_OVERRIDE"
    );

    eval_ok(&mut vm, "rename expr {}; interp expose {} held expr");
    assert_eq!(vm.invoke_function(&handle).result.to_str().as_ref(), "3");
}

#[test]
fn unrelated_mutation_does_not_deoptimise_a_reusable_artifact() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    eval_ok(&mut vm, "proc unrelated {} {return X}");
    vm.set_compiler(Box::new(OptimisedOnlyCompilerSvc(
        BytecodeCompileService::default(),
    )));

    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.to_str().as_ref(), "3");
}

#[test]
fn native_replacement_fails_closed_when_no_plain_compiler_exists() {
    let module = BytecodeCompileService::default()
        .compile("expr {1+2}")
        .expect("module compiles");
    let mut vm = Vm::new();
    vm.register_native_command("expr", Rc::new(NativeOverride));

    let completion = vm.run_module(&module);
    assert_eq!(completion.code, Code::Error, "{completion:?}");
    assert!(
        completion
            .result
            .to_str()
            .contains("requires a plain-dispatch CompileService"),
        "{}",
        completion.result.to_str()
    );
}

#[test]
fn profileless_function_fails_closed_after_binding_replacement() {
    let module = BytecodeCompileService::default()
        .compile("expr {1+2}")
        .expect("module compiles");
    let (mut vm, _output) = vm();
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return FUNCTION_OVERRIDE}",
    );

    let completion = vm.run_function(&module.top_level);
    assert_eq!(completion.code, Code::Error, "{completion:?}");
    assert!(
        completion
            .result
            .to_str()
            .contains("has no source for plain dispatch"),
        "{}",
        completion.result.to_str()
    );
}

#[test]
fn mutation_recovery_rejects_a_dishonest_plain_compiler() {
    let (mut vm, _output) = vm();
    let handle = vm
        .compile_function("expr {1+2}")
        .expect("function compiles");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return SHOULD_NOT_RUN_OPTIMISED}",
    );
    vm.set_compiler(Box::new(DishonestPlainCompilerSvc(
        BytecodeCompileService::default(),
    )));

    let completion = vm.invoke_function(&handle);
    assert_eq!(completion.code, Code::Error, "{completion:?}");
    assert!(
        completion
            .result
            .to_str()
            .contains("returned optimised bytecode"),
        "{}",
        completion.result.to_str()
    );
}

#[test]
fn procedure_recovery_rejects_a_dishonest_plain_compiler() {
    let (mut vm, _output) = vm();
    eval_ok(&mut vm, "proc p {} {expr {1+2}}");
    eval_ok(
        &mut vm,
        "rename expr real_expr; proc expr args {return SHOULD_NOT_RUN_OPTIMISED}",
    );
    vm.set_compiler(Box::new(DishonestPlainCompilerSvc(
        BytecodeCompileService::default(),
    )));

    let completion = vm.eval_source("p").expect("caller compiles");
    assert_eq!(completion.code, Code::Error, "{completion:?}");
    assert!(
        completion
            .result
            .to_str()
            .contains("procedure plain-dispatch capability returned optimised bytecode"),
        "{}",
        completion.result.to_str()
    );
}

#[test]
fn procedure_consumers_preserve_a_missing_typed_compiler_capability() {
    const CAUSE: &str = "CompileService does not support procedure-body compilation";

    let mut host_vm = Vm::new();
    host_vm.set_compiler(Box::new(ScriptOnlyCompilerSvc(
        BytecodeCompileService::default(),
    )));
    let host_error = host_vm
        .define_procedure("host_p", &[], "return HOST")
        .expect_err("host procedure needs the typed capability");
    assert_eq!(host_error.message, format!("procedure \"host_p\": {CAUSE}"));

    for source in [
        "set body {return PROC}; proc p {} $body",
        "set lambda [list {} {return APPLY}]; apply $lambda",
        "oo::class create C {method m {} {return METHOD}}; set o [C new]; $o m",
    ] {
        let mut vm = Vm::new();
        vm.set_compiler(Box::new(ScriptOnlyCompilerSvc(
            BytecodeCompileService::default(),
        )));
        let completion = vm.eval_source(source).expect("outer script compiles");
        assert_eq!(completion.code, Code::Error, "{source}: {completion:?}");
        assert_eq!(completion.result.to_str().as_ref(), CAUSE, "{source}");
    }
}
