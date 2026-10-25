// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tcl_compiler::compile_service::{BytecodeCompileService, NativeCompiledProcedure};
use tcl_compiler::ir::{Module, Procedure, ProcedureId};
use tcl_runtime_api::{CompileError, CompileService, NativeCompilationEntry, ProcedureDispatch};

type Captured = (NativeCompiledProcedure, NativeCompilationEntry);
type Observations = Rc<RefCell<Vec<Captured>>>;
type Runtime = (tcl_vm::Vm, Observations, Rc<Cell<usize>>);

struct ObserveCompilation {
    compiler: BytecodeCompileService,
    observed: Observations,
    attempted: Rc<Cell<usize>>,
    refuse_original: bool,
}

impl CompileService for ObserveCompilation {
    type Module = tcl_bytecode::ModuleAsm;

    fn compile(&self, source: &str) -> Result<Self::Module, CompileError> {
        self.compiler.compile(source)
    }

    fn script_command_plan_bytes_with_entry(
        &self,
        source: &tcl_runtime_api::SourceImage,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, CompileError> {
        self.compiler
            .script_command_plan_bytes_with_entry(source, profile, entry)
    }

    fn compile_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        self.compiler
            .compile_script_bytes_with_entry(target, profile, entry)
    }

    fn compile_plain_script_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        self.compiler
            .compile_plain_script_bytes_with_entry(target, profile, entry)
    }

    fn compile_procedure_bytes_with_entry(
        &self,
        target: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        self.compiler
            .compile_procedure_bytes_with_entry(target, profile, entry, dispatch)
    }

    fn compile_original_procedure_bytes_with_entry(
        &self,
        request: &tcl_runtime_api::OriginalProcedureCompileRequest,
        profile: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
        dispatch: ProcedureDispatch,
    ) -> Result<Self::Module, CompileError> {
        self.attempted.set(self.attempted.get() + 1);
        if self.refuse_original {
            return Err(CompileError::Unsupported(
                "body compiler refusal control".to_owned(),
            ));
        }
        let compiled = self
            .compiler
            .compile_original_procedure_bytes_with_entry(request, profile, entry, dispatch)?;
        let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
        let original = NativeCompiledProcedure::lower(request, registry, profile, entry, dispatch)?;
        self.observed.borrow_mut().push((original, entry.clone()));
        Ok(compiled)
    }
}

pub struct CompiledFixture {
    original: NativeCompiledProcedure,
    entry: NativeCompilationEntry,
    owner: Option<tcl_vm::Vm>,
}

impl CompiledFixture {
    pub fn module(&self) -> &Module {
        self.original.module()
    }

    pub fn body(&self) -> (ProcedureId, &Procedure) {
        self.original
            .original_compiled_body(
                self.module(),
                self.module().resolved_registry(),
                &self.entry,
            )
            .expect("original live compiled procedure body")
    }

    pub fn refuses_module(&self, module: &Module) -> bool {
        self.original
            .original_compiled_body(module, module.resolved_registry(), &self.entry)
            .is_none()
    }

    pub fn refuses_foreign_entry(&self, other: &Self) -> bool {
        self.original
            .original_compiled_body(
                self.module(),
                self.module().resolved_registry(),
                &other.entry,
            )
            .is_none()
    }

    pub fn drop_owner(&mut self) {
        drop(self.owner.take());
    }
}

fn runtime(refuse_original: bool) -> Runtime {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl9.0").unit_profile();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let attempted = Rc::new(Cell::new(0));
    let mut owner = tcl_vm::Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("actual native command registration");
    owner.set_compiler(Box::new(ObserveCompilation {
        compiler: BytecodeCompileService::for_profile(profile),
        observed: Rc::clone(&observed),
        attempted: Rc::clone(&attempted),
        refuse_original,
    }));
    (owner, observed, attempted)
}

pub fn compiled_fixture(source: &str, name: &str, arguments: &[&str]) -> CompiledFixture {
    let (mut owner, observed, _) = runtime(false);
    let declared = owner
        .try_eval_source(source)
        .expect("actual fixture declaration and calls");
    assert!(declared.is_ok(), "{declared:?}");
    if observed.borrow().is_empty() {
        let values: Vec<_> = arguments
            .iter()
            .map(|argument| tcl_vm::Value::new_native_string_bytes(argument.as_bytes()))
            .collect();
        let activated = owner
            .try_invoke_command(name, &values)
            .expect("actual original procedure activation");
        assert!(activated.is_ok(), "{activated:?}");
    }
    let mut captured = observed.borrow_mut();
    let (original, entry) = captured
        .pop()
        .expect("actual original procedure compilation");
    assert!(
        captured
            .iter()
            .all(|(previous, _)| original.same_declaration(previous)),
        "every compilation belongs to the same original declaration"
    );
    CompiledFixture {
        original,
        entry,
        owner: Some(owner),
    }
}

pub fn assert_declaration_and_refusal_boundaries() {
    let source = "proc p {} {set x 1}";
    let (mut owner, observed, attempted) = runtime(true);
    let declared = owner
        .try_eval_source(source)
        .expect("actual original declaration");
    assert!(declared.is_ok(), "{declared:?}");
    assert_eq!(attempted.get(), 0, "declaration has not compiled the body");
    assert!(observed.borrow().is_empty());
    let refused = owner.try_invoke_command("p", &[]);
    assert!(
        refused.is_err(),
        "body compiler host refusal must remain explicit"
    );
    assert_eq!(
        attempted.get(),
        1,
        "actual original body compiler was reached"
    );
    assert!(
        observed.borrow().is_empty(),
        "a refused compiler issued no compiled fixture"
    );
}
