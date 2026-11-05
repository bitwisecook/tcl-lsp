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

//! Test fixtures that hold a live VM, which `tcl-compiler` cannot depend on:
//! copies of `tcl_compiler::environment_ingress`'s own test fixtures, for the
//! emitter tests that moved here with `codegen`.

/// Capture the real interpreter's original command and fixed-math entry.
/// This attests registration only: it grants no literal-pool object class,
/// operand effects or successful execution of the fixture being analysed.
pub(crate) fn captured_native_entry(
    profile: &'static tcl_dialect::DialectProfile,
) -> CapturedNativeEntry {
    let (owner, entry) = captured_native_entry_with_owner(profile);
    CapturedNativeEntry { entry, owner }
}

/// The independently captured entry and its actual interpreter lifetime.
/// Entry clones remain weak observations; this holder grants no Native purpose.
pub(crate) struct CapturedNativeEntry {
    entry: tcl_runtime_api::NativeCompilationEntry,
    owner: tcl_vm::Vm,
}

impl CapturedNativeEntry {
    pub(crate) fn as_entry(&self) -> &tcl_runtime_api::NativeCompilationEntry {
        &self.entry
    }

    pub(crate) fn retain<T>(self, value: T) -> RetainedNativeFixture<T> {
        RetainedNativeFixture::new(value, self.owner)
    }
}

impl std::ops::Deref for CapturedNativeEntry {
    type Target = tcl_runtime_api::NativeCompilationEntry;
    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl std::ops::DerefMut for CapturedNativeEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entry
    }
}

/// Keep the original interpreter alive when a test consumes a weak observation.
/// Command snapshots alone deliberately retain no registration-world owner.
pub(crate) fn captured_native_entry_with_owner(
    profile: &'static tcl_dialect::DialectProfile,
) -> (tcl_vm::Vm, tcl_runtime_api::NativeCompilationEntry) {
    captured_native_entry_with_owner_using(profile, |_| {})
}

/// The same capture worker after actual public command setup. The entry is
/// captured from its real owner; no command row or trace flag is manufactured.
pub(crate) fn captured_native_entry_with_owner_using(
    profile: &'static tcl_dialect::DialectProfile,
    setup: impl FnOnce(&mut tcl_vm::Vm),
) -> (tcl_vm::Vm, tcl_runtime_api::NativeCompilationEntry) {
    captured_native_entry_with_request(profile, setup, |vm| {
        assert!(vm.try_compile_function("set entry_probe 1").is_err());
    })
}

/// Capture the actual procedure activation after public namespace and definition setup.
pub(crate) fn captured_native_namespace_entry_with_owner(
    profile: &'static tcl_dialect::DialectProfile,
    namespace: &str,
) -> (tcl_vm::Vm, tcl_runtime_api::NativeCompilationEntry) {
    let procedure = format!("{namespace}::entry_probe");
    let (owner, entry) = captured_native_entry_with_request(
        profile,
        |vm| {
            let _ = vm.invoke_command(
                "namespace",
                &[
                    tcl_vm::Value::string("eval"),
                    tcl_vm::Value::string(namespace),
                    tcl_vm::Value::string(""),
                ],
            );
            vm.try_define_procedure(&procedure, &[], "set entry_probe 1")
                .expect("authentic namespace procedure definition");
        },
        |vm| {
            let _ = vm.invoke_command(&procedure, &[]);
        },
    );
    assert_eq!(
        tcl_compiler::command_binding::SourceNamespaceKey::from_native_entry(&entry)
            .unwrap()
            .display(),
        Some(namespace.to_owned()),
        "actual selected procedure namespace",
    );
    (owner, entry)
}

pub(crate) fn captured_native_entry_with_request(
    profile: &'static tcl_dialect::DialectProfile,
    setup: impl FnOnce(&mut tcl_vm::Vm),
    request: impl FnOnce(&mut tcl_vm::Vm),
) -> (tcl_vm::Vm, tcl_runtime_api::NativeCompilationEntry) {
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Capture(Rc<RefCell<Option<tcl_runtime_api::NativeCompilationEntry>>>);
    impl tcl_runtime_api::CompileService for Capture {
        type Module = tcl_bytecode::ModuleAsm;
        fn compile_script_bytes_with_entry(
            &self,
            _: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
            _: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            *self.0.borrow_mut() = Some(entry.clone());
            self.compile("")
        }
        fn compile_procedure_bytes_with_entry(
            &self,
            _: tcl_runtime_api::ProcedureCompileTargetBytes<'_>,
            _: &'static tcl_dialect::DialectProfile,
            entry: &tcl_runtime_api::NativeCompilationEntry,
            _: tcl_runtime_api::ProcedureDispatch,
        ) -> Result<Self::Module, tcl_runtime_api::CompileError> {
            *self.0.borrow_mut() = Some(entry.clone());
            self.compile("")
        }
        fn script_command_plan_bytes_with_entry(
            &self,
            source: &tcl_runtime_api::SourceImage,
            _: &'static tcl_dialect::DialectProfile,
            _: &tcl_runtime_api::NativeCompilationEntry,
        ) -> Result<tcl_runtime_api::ScriptCommandPlan, tcl_runtime_api::CompileError> {
            Ok(tcl_runtime_api::ScriptCommandPlan::complete(source.len()))
        }

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

    let captured = Rc::new(RefCell::new(None));
    let mut vm = tcl_vm::Vm::with_native_core(
        Box::new(std::io::sink()),
        Rc::new(tcl_vm::host_native::NativeHost::new()),
        profile,
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("authentic native registration before entry capture");
    setup(&mut vm);
    vm.set_compiler(Box::new(Capture(Rc::clone(&captured))));
    request(&mut vm);
    let entry = captured
        .borrow_mut()
        .take()
        .expect("actual interpreter compilation entry");
    (vm, entry)
}

/// Test value retires before its actual interpreter, which stays live through
/// every subsequent query. This does not strengthen any registration observation.
pub(crate) struct RetainedNativeFixture<T> {
    value: T,
    _owner: tcl_vm::Vm,
}

impl<T> RetainedNativeFixture<T> {
    pub(crate) fn new(value: T, owner: tcl_vm::Vm) -> Self {
        Self {
            value,
            _owner: owner,
        }
    }
}

impl<T> std::ops::Deref for RetainedNativeFixture<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> std::ops::DerefMut for RetainedNativeFixture<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

pub(crate) type RetainedNativeUnit =
    RetainedNativeFixture<tcl_compiler::compilation_unit::CompilationUnit>;
