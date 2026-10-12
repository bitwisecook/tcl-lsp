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

//! Actual abstract bootstrap allocation; pure recipes never issue stock roles.
use super::*;
use crate::Code;
use tcl_runtime_api::{CompileError, CompileService, NativeCompilationEntry, SourceImage};

struct Capture(Rc<RefCell<Option<NativeCompilationEntry>>>);
impl CompileService for Capture {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile_script_bytes_with_entry(
        &self,
        _: tcl_runtime_api::ScriptCompileTargetBytes<'_>,
        _: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        *self.0.borrow_mut() = Some(entry.clone());
        self.compile("")
    }
    fn compile_script_with_entry(
        &self,
        _: tcl_runtime_api::ScriptCompileTarget<'_>,
        _: &'static tcl_dialect::DialectProfile,
        entry: &NativeCompilationEntry,
    ) -> Result<Self::Module, CompileError> {
        *self.0.borrow_mut() = Some(entry.clone());
        self.compile("")
    }
    fn script_command_plan_bytes_with_entry(
        &self,
        source: &SourceImage,
        _: &'static tcl_dialect::DialectProfile,
        _: &NativeCompilationEntry,
    ) -> Result<tcl_runtime_api::ScriptCommandPlan, CompileError> {
        Ok(tcl_runtime_api::ScriptCommandPlan::complete(source.len()))
    }
    fn compile(&self, _: &str) -> Result<Self::Module, CompileError> {
        Err(CompileError::Unsupported(
            "actual bootstrap entry capture only".into(),
        ))
    }
}
fn entry(vm: &mut Vm) -> NativeCompilationEntry {
    let retained = Rc::new(RefCell::new(None));
    vm.set_compiler(Box::new(Capture(Rc::clone(&retained))));
    assert!(vm.try_compile_function("set bootstrap_probe 1").is_err());
    let actual = retained
        .borrow_mut()
        .take()
        .expect("actual compilation ingress");
    actual
}
fn original(vm: &Vm) -> Option<OoId> {
    vm.oo
        .names
        .iter()
        .find_map(|(id, name)| (name.as_bytes() == b"oo::abstract").then_some(*id))
}
fn binding(
    entry: &NativeCompilationEntry,
) -> Option<&tcl_runtime_api::native_compilation::NativeCompilationBinding> {
    entry.commands.iter().find(|row| {
        row.slot.namespace == ["oo".to_owned()] && row.slot.simple.as_bytes() == b"abstract"
    })
}

#[test]
fn original_abstract_bootstrap_retains_actual_metaclass_and_class_side_visibility() {
    // Implementation contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let vm = crate::native_fixture::core(crate::environment::profile_for_dialect(version));
        let actual = original(&vm);
        if version == "tcl8.6" {
            assert!(actual.is_none());
            continue;
        }
        let actual = actual.unwrap();
        let class_root = vm.oo.class_root.unwrap();
        let class = &vm.oo.classes[&actual];
        let object = &vm.oo.objects[&actual];
        assert_eq!(object.class, class_root);
        assert_eq!(class.supers, [class_root]);
        assert!(class.constructor.owner().is_none());
        assert!(class.destructor.owner().is_none());
        assert!(object.namespace_token.is_some());
        assert!(object.methods.is_empty());
        assert!(object.unexported.is_empty());
        for name in ["create", "createWithNamespace", "new"] {
            let key = vm.oo.method_keys.get(&NameBytes::from(name)).unwrap();
            assert!(class.unexported.contains(key));
            let method = class.methods.owner(key).unwrap();
            let method = method.borrow();
            assert!(method.visibility_only);
            assert!(method.intrinsic.is_none());
            assert!(!method.exported);
        }
        let create = vm.oo.classes[&class_root].methods.owner("create").unwrap();
        assert_eq!(
            create.borrow().intrinsic,
            Some(NativeOoIntrinsicMethod::Create)
        );
        assert!(create.borrow().exported);
    }
}

#[test]
fn original_abstract_bootstrap_entry_joins_real_token_without_factory_role_donation() {
    // Implementation contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for version in ["tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(version));
        let original = original(&vm).unwrap();
        let actual = entry(&mut vm);
        let binding = binding(&actual).expect("actual engine published command");
        let observed = actual
            .original_oo_class(binding.token, binding.implementation_generation)
            .unwrap();
        assert_eq!(observed.object, original.0);
        assert_eq!(observed.maker, vm.oo.class_root.unwrap().0);
        assert_eq!(observed.superclasses, [vm.oo.class_root.unwrap().0]);
        assert_eq!(observed.bootstrap_role, None);
        assert_eq!(observed.constructor, NativeOoLifecycleObservation::Absent);
        assert_eq!(observed.destructor, NativeOoLifecycleObservation::Absent);
        let CommandSidecarKey::Visible(key) = &vm.oo.command_keys[&original] else {
            panic!("visible stock command")
        };
        let current = vm
            .native_compilation_binding(key, &Command::Object(original), actual.interpreter)
            .unwrap();
        assert_eq!(current.token, binding.token);
        assert_eq!(
            current.implementation_generation,
            binding.implementation_generation
        );
        let registry = tcl_registry::CommandRegistry::build_default();
        assert!(
            registry
                .native_class_factory_recipe(
                    "oo::abstract",
                    vm.actual_native_invocation_dialect(),
                    tcl_dialect::model::InvocationRealm::default()
                )
                .is_none()
        );
        assert!(
            actual
                .original_oo_class(binding.token, binding.implementation_generation + 1)
                .is_none()
        );
    }
}

#[test]
fn original_abstract_bootstrap_preserves_surface_absence_without_erasing_physical_class() {
    // Implementation contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.1"));
    let actual = original(&vm).unwrap();
    assert!(binding(&entry(&mut vm)).is_some());
    vm.set_dialect_profile(crate::environment::profile_for_dialect("tcl8.6"));
    assert_eq!(original(&vm), Some(actual));
    assert!(binding(&entry(&mut vm)).is_none());
}

#[test]
fn original_abstract_command_replacement_keeps_new_binding_separate_from_old_class() {
    // Implementation contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.1"));
    let original = original(&vm).unwrap();
    let before = entry(&mut vm);
    let old_token = binding(&before).unwrap().token;
    assert_eq!(
        vm.invoke_command(
            "rename",
            &[
                Value::string("::oo::abstract"),
                Value::string("::MovedAbstract")
            ]
        )
        .code,
        Code::Ok
    );
    vm.register_command(
        "::oo::abstract",
        Command::Builtin(|_, _| super::super::ok(Value::empty())),
    );
    let after = entry(&mut vm);
    let replacement = binding(&after).unwrap();
    assert_ne!(replacement.token, old_token);
    assert!(
        after
            .original_oo_class(replacement.token, replacement.implementation_generation)
            .is_none()
    );
    let moved = after
        .commands
        .iter()
        .find(|row| row.token == old_token)
        .unwrap();
    assert_eq!(
        after
            .original_oo_class(moved.token, moved.implementation_generation)
            .unwrap()
            .object,
        original.0
    );
}

#[test]
fn original_abstract_metaclass_manufactures_only_through_the_actual_public_maker() {
    // Implementation contract: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.1"));
    let result = vm.invoke_command(
        "::oo::abstract",
        &[Value::string("create"), Value::string("AbstractActual")],
    );
    assert_eq!(result.code, Code::Ok);
    assert_eq!(result.result.string_bytes().as_ref(), b"::AbstractActual");
    assert_eq!(
        vm.invoke_command("AbstractActual", &[Value::string("new")])
            .code,
        Code::Error
    );
    assert_eq!(
        vm.invoke_command(
            "oo::class",
            &[Value::string("create"), Value::string("ConcreteActual")]
        )
        .code,
        Code::Ok
    );
    assert_eq!(
        vm.invoke_command(
            "oo::define",
            &[
                Value::string("ConcreteActual"),
                Value::string("superclass"),
                Value::string("AbstractActual")
            ]
        )
        .code,
        Code::Ok
    );
    assert_eq!(
        vm.invoke_command(
            "ConcreteActual",
            &[Value::string("create"), Value::string("ActualObject")]
        )
        .code,
        Code::Ok
    );
}
