// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{cell::RefCell, rc::Rc};
use tcl_bytecode::ModuleAsm;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::{DialectProfile, NativeProcedureHeaderCompilation as Header};
use tcl_runtime_api::{
    CompileError, CompileService, NativeCompilationEntry, ProcedureCompileTargetBytes,
    ProcedureDispatch, ScriptCommandPlan, ScriptCompileTargetBytes, SourceImage,
    native_compilation::NativeCompilerHookPresence as Hook,
};
use tcl_vm::{Code, Value, Vm, host_native::NativeHost};

const SENTINEL: &[u8] = b"set cached_marker CURRENT";

#[derive(Default)]
struct Observations {
    last_entry: Option<NativeCompilationEntry>,
    sentinel_compilations: usize,
}

struct RecordingCompiler {
    inner: BytecodeCompileService,
    observations: Rc<RefCell<Observations>>,
}

impl RecordingCompiler {
    fn record(&self, source: &SourceImage, entry: &NativeCompilationEntry) {
        let mut observations = self.observations.borrow_mut();
        observations.last_entry = Some(entry.clone());
        if source.bytes() == SENTINEL {
            observations.sentinel_compilations += 1;
        }
    }
}

impl CompileService for RecordingCompiler {
    type Module = ModuleAsm;

    fn compile(&self, source: &str) -> Result<ModuleAsm, CompileError> {
        self.inner.compile(source)
    }

    fn script_command_plan_bytes_with_entry(
        &self,
        source: &SourceImage,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ScriptCommandPlan, CompileError> {
        self.observations.borrow_mut().last_entry = Some(entry.clone());
        self.inner
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ModuleAsm, CompileError> {
        self.record(target.source, entry);
        self.inner
            .compile_script_bytes_with_entry(target, profile, entry)
    }

    fn compile_plain_script_bytes_with_entry(
        &self,
        target: ScriptCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<ModuleAsm, CompileError> {
        self.record(target.source, entry);
        self.inner
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }

    fn compile_procedure_bytes_with_entry(
        &self,
        target: ProcedureCompileTargetBytes<'_>,
        profile: &'static DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<ModuleAsm, CompileError> {
        self.inner
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
    }
}

fn native_vm(engine: &str) -> Vm {
    Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(NativeHost::new()),
        tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("actual native constructor")
}

fn invoke(vm: &mut Vm, command: &str, arguments: &[Value]) {
    let completion = vm
        .try_invoke_command(command, arguments)
        .expect("native invocation");
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
}

fn snapshot(vm: &mut Vm, observations: &Rc<RefCell<Observations>>) -> NativeCompilationEntry {
    let completion = vm
        .try_eval_source_bytes(b"set snapshot_marker CURRENT")
        .expect("native compiler entry");
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    observations
        .borrow()
        .last_entry
        .clone()
        .expect("actual entry observed")
}

fn run_cached_body(vm: &mut Vm, body: &Value) {
    invoke(vm, "if", &[Value::int(1), body.clone()]);
}

fn check_constructor_roster(profile: &'static DialectProfile) {
    let roster = tcl_registry::InvocationDialect::of_profile(profile)
        .native_bootstrap_protocol()
        .unwrap()
        .core_mathfunc_roster()
        .unwrap();
    assert_eq!(roster.len(), 29);
    let vm = native_vm("tcl8.6");
    for spec in tcl_syntax::expr::mathfunc::all() {
        assert_eq!(
            vm.registered_command_token_bytes(format!("::tcl::mathfunc::{}", spec.name).as_bytes())
                .is_some(),
            roster.contains(&spec.name),
            "C8.6 {}",
            spec.name
        );
    }
    for engine in ["tcl8.4", "tcl8.5", "tcl9.0", "tcl9.1", "jim"] {
        let vm = native_vm(engine);
        let mut previous = Vm::new();
        previous.set_dialect_profile(
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
        );
        for spec in tcl_syntax::expr::mathfunc::all() {
            let name = format!("::tcl::mathfunc::{}", spec.name);
            assert_eq!(
                vm.registered_command_token_bytes(name.as_bytes()).is_some(),
                previous
                    .registered_command_token_bytes(name.as_bytes())
                    .is_some(),
                "{engine} {}",
                spec.name
            );
        }
    }
    let mut distribution = Vm::new();
    distribution.set_dialect_profile(profile);
    for name in [
        b"::tcl::mathfunc::min".as_slice(),
        b"::tcl::mathfunc::max".as_slice(),
    ] {
        assert!(
            distribution.registered_command_token_bytes(name).is_some(),
            "distribution constructor retains mathfunc implementations"
        );
    }
}

fn check_empty_birth_currency(profile: &'static DialectProfile) {
    for (parameters, header, hook) in [
        ("args", Header::NoOp, Hook::Present),
        ("{args}", Header::Absent, Hook::Absent),
    ] {
        let mut vm = native_vm("tcl8.6");
        let observations = Rc::new(RefCell::new(Observations::default()));
        vm.set_compiler(Box::new(RecordingCompiler {
            inner: BytecodeCompileService::for_profile(profile),
            observations: Rc::clone(&observations),
        }));
        let cached_body = Value::new_native_string_bytes(SENTINEL);
        run_cached_body(&mut vm, &cached_body);
        run_cached_body(&mut vm, &cached_body);
        assert_eq!(
            observations.borrow().sentinel_compilations,
            1,
            "same retained source object reuses its bytecode"
        );

        invoke(
            &mut vm,
            "proc",
            &[
                Value::string("::tcl::mathfunc::min"),
                Value::string(parameters),
                Value::empty(),
            ],
        );
        run_cached_body(&mut vm, &cached_body);
        assert_eq!(
            observations.borrow().sentinel_compilations,
            1,
            "empty-slot birth preserves cached-body currency"
        );
        let entry = snapshot(&mut vm, &observations);
        let born = entry
            .commands
            .iter()
            .find(|row| row.slot.simple == "min" && row.slot.namespace == ["tcl", "mathfunc"])
            .unwrap();
        assert_eq!(born.procedure_header, Some(header));
        assert_eq!(born.compiler_hook, hook);
        assert!(born.compiler.is_none());
        let generation = born.implementation_generation;
        invoke(
            &mut vm,
            "proc",
            &[
                Value::string("::tcl::mathfunc::min"),
                Value::string("args"),
                Value::string("return REPLACED"),
            ],
        );
        run_cached_body(&mut vm, &cached_body);
        assert_eq!(
            observations.borrow().sentinel_compilations,
            1 + usize::from(hook == Hook::Present),
            "retiring an actual Present hook invalidates the cached body"
        );
        let entry = snapshot(&mut vm, &observations);
        let replaced = entry
            .commands
            .iter()
            .find(|row| row.slot.simple == "min" && row.slot.namespace == ["tcl", "mathfunc"])
            .unwrap();
        assert_ne!(replaced.implementation_generation, generation);
        assert_eq!(replaced.procedure_header, Some(Header::Absent));
        assert_eq!(replaced.compiler_hook, Hook::Absent);
    }
}

fn check_authentic_max_birth_currency(profile: &'static DialectProfile) {
    let mut vm = native_vm("tcl8.6");
    let observations = Rc::new(RefCell::new(Observations::default()));
    vm.set_compiler(Box::new(RecordingCompiler {
        inner: BytecodeCompileService::for_profile(profile),
        observations: Rc::clone(&observations),
    }));
    let cached_body = Value::new_native_string_bytes(SENTINEL);
    run_cached_body(&mut vm, &cached_body);
    let source = format!(
        "namespace eval ::tcl::mathfunc {{\n{}\n}}\nset marker after",
        include_str!("data/native_c86_init_max.tcl")
    );
    let completion = vm
        .try_eval_source_bytes(source.as_bytes())
        .expect("authentic init definition compiler entry");
    assert_eq!(completion.code, Code::Ok, "{completion:?}");
    assert_eq!(completion.result.string_bytes().as_ref(), b"after");
    run_cached_body(&mut vm, &cached_body);
    assert_eq!(
        observations.borrow().sentinel_compilations,
        1,
        "authentic max birth preserves cached-body currency"
    );
    let entry = snapshot(&mut vm, &observations);
    let born = entry
        .commands
        .iter()
        .find(|row| row.slot.simple == "max" && row.slot.namespace == ["tcl", "mathfunc"])
        .unwrap();
    assert_eq!(born.procedure_header, Some(Header::Absent));
    assert_eq!(born.compiler_hook, Hook::Absent);
    assert!(born.compiler.is_none());
}

#[test]
fn native_constructor_mathfunc_roster_and_library_birth_currency() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
    check_constructor_roster(profile);
    check_empty_birth_currency(profile);
    check_authentic_max_birth_currency(profile);
}
