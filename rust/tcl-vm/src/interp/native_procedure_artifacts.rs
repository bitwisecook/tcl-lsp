// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Procedure artifact geometry and independently checked native namespace owners.

use super::{CompiledUnit, CompilerProvenance, ModuleAsm, NamespacePath, NsId, Rc, Vm};
use tcl_bytecode::ProcedureProvenance;
use tcl_runtime_api::CompiledNamespaceContext;

/// Source identity partitions by exact body geometry, never a displayed holder.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct ProcedureCacheKey {
    pub(super) name: String,
    parameters: String,
    body: String,
    body_namespace: NamespacePath,
}

impl ProcedureCacheKey {
    pub(super) fn from_runtime_key(
        name: &str,
        parameters: &str,
        body: &str,
        body_namespace: &NamespacePath,
    ) -> Self {
        Self {
            name: name.to_owned(),
            parameters: parameters.to_owned(),
            body: body.to_owned(),
            body_namespace: body_namespace.clone(),
        }
    }

    pub(super) fn from_module_entry(qname: &str, provenance: &ProcedureProvenance) -> Option<Self> {
        let entry_name = qname.strip_prefix("::")?;
        let provenance_name = provenance.name.strip_prefix("::")?;
        (entry_name == provenance_name).then_some(())?;
        let context = provenance.namespace_context.as_ref()?;
        Some(Self::from_runtime_key(
            entry_name,
            &provenance.parameters,
            &provenance.body,
            context.path(),
        ))
    }
}

pub(super) struct CachedProcedureArtifact {
    unit: CompiledUnit,
    namespace_context: CompiledNamespaceContext,
}

impl CachedProcedureArtifact {
    fn admitted_for(&self, vm: &Vm, selected: NsId) -> Option<CompiledUnit> {
        // Geometry cannot identify an old retained procedure owner after
        // another namespace has acquired the same path.
        if !matches!(self.namespace_context, CompiledNamespaceContext::Native(_))
            || vm.resolve_compiled_namespace_context(&self.namespace_context) != Some(selected)
        {
            return None;
        }
        Some(self.unit.clone())
    }
}

impl Vm {
    pub(super) fn module_proc(
        &self,
        qname: &str,
        parameters: &str,
        body: &str,
        selected: NsId,
    ) -> Option<CompiledUnit> {
        let key = ProcedureCacheKey::from_runtime_key(
            qname,
            parameters,
            body,
            &self.namespace_path_for_token(selected),
        );
        self.module_procs.get(&key)?.admitted_for(self, selected)
    }

    /// Retain candidates without executing literal registrations or declaring a proc.
    pub(super) fn merge_procedure_artifacts(
        &mut self,
        module: &ModuleAsm,
        compiler: CompilerProvenance,
    ) {
        for (qname, asm) in &module.procedures {
            let Some(provenance) = module.procedure_provenance.get(qname) else {
                continue;
            };
            let Some(key) = ProcedureCacheKey::from_module_entry(qname, provenance) else {
                continue;
            };
            let context = provenance
                .namespace_context
                .as_ref()
                .expect("checked context");
            let native_owner = match context {
                CompiledNamespaceContext::Native(_) => {
                    let Some(owner) = self.resolve_compiled_namespace_context(context) else {
                        continue;
                    };
                    Some(owner)
                }
                CompiledNamespaceContext::ConstructedPath(_) => None,
            };
            let mut unit = self.unentered_compiled_unit(
                Rc::new(asm.clone()),
                context.path().clone(),
                compiler,
            );
            unit.native_cache = native_owner.map(|owner| self.native_cache_stamp(owner));
            // A newer compiler artifact replaces only the same source and exact
            // geometry; another colliding display remains an independent entry.
            self.module_procs.insert(
                key,
                CachedProcedureArtifact {
                    unit,
                    namespace_context: context.clone(),
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_bytecode::FunctionAsm;
    use tcl_cmd_core::namespace::NamespaceDeleteBackend;

    fn native_vm(engine: &str) -> Vm {
        Vm::with_native_core(
            Box::new(std::io::sink()),
            Rc::new(crate::host_native::NativeHost::new()),
            tcl_registry::model::ingress::resolve_environment(engine).unit_profile(),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap()
    }

    fn declare(vm: &mut Vm, path: &NamespacePath) -> NsId {
        vm.declare_namespace_path_with_origin(path.clone(), true);
        vm.definition_namespace_token_at_path(path, true)
    }

    fn context(vm: &Vm, namespace: NsId) -> CompiledNamespaceContext {
        CompiledNamespaceContext::Native(
            vm.native_compilation_entry_for_namespace_token(Some(namespace), true)
                .retained_namespace_context(u64::from(namespace.0))
                .unwrap(),
        )
    }

    fn artifact(vm: &Vm, context: Option<CompiledNamespaceContext>) -> ModuleAsm {
        let name = "::a:::b::p".to_owned();
        ModuleAsm {
            profile: vm.source_profile(),
            source: tcl_lexer::SourceImage::document("proc p {} {return SAME}"),
            source_namespace: NamespacePath::root(),
            plain_command_dispatch: false,
            top_level: FunctionAsm::default(),
            top_level_body: FunctionAsm::default(),
            procedures: std::collections::HashMap::from([(name.clone(), FunctionAsm::default())]),
            procedure_provenance: std::collections::HashMap::from([(
                name.clone(),
                ProcedureProvenance {
                    name,
                    namespace_context: context,
                    parameters: String::new(),
                    body: "return SAME".to_owned(),
                },
            )]),
        }
    }

    #[test]
    fn procedure_artifacts_partition_colliding_display_paths_without_owner_donation() {
        let mut vm = native_vm("tcl8.6");
        let left_path = NamespacePath::from_segments(["a:", "b"]);
        let right_path = NamespacePath::from_segments(["a", ":b"]);
        let left = declare(&mut vm, &left_path);
        let right = declare(&mut vm, &right_path);
        assert_eq!(vm.ns_name_bytes(left), vm.ns_name_bytes(right));
        for path in [&left_path, &right_path] {
            let module = artifact(
                &vm,
                Some(CompiledNamespaceContext::ConstructedPath(path.clone())),
            );
            vm.merge_procs(&module);
        }
        assert_eq!(vm.module_procs.len(), 2);
        for selected in [left, right] {
            assert!(
                vm.module_proc("a:::b::p", "", "return SAME", selected)
                    .is_none()
            );
        }
        let native = artifact(&vm, Some(context(&vm, left)));
        vm.merge_foreign_procs(&native);
        let selected = vm.module_proc("a:::b::p", "", "return SAME", left).unwrap();
        assert_eq!(selected.source_namespace, left_path);
        assert!(
            selected
                .compiler
                .is_current_foreign_admission(vm.compiler_generation)
        );
        assert!(!selected.compiler.is_current_service(vm.compiler_generation));
        assert!(
            vm.module_proc("a:::b::p", "", "return SAME", right)
                .is_none()
        );
    }

    #[test]
    fn procedure_artifact_missing_namespace_never_reconstructs_its_display_holder() {
        let mut vm = native_vm("tcl8.6");
        let module = artifact(&vm, None);
        vm.merge_procs(&module);
        vm.merge_foreign_procs(&module);
        assert!(vm.module_procs.is_empty());
    }

    #[test]
    fn native_procedure_artifact_requires_selected_retained_namespace_incarnation() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut vm = native_vm(engine);
            let path = NamespacePath::from_segments(["N"]);
            let old = declare(&mut vm, &path);
            let original = artifact(&vm, Some(context(&vm, old)));
            vm.merge_procs(&original);
            assert!(vm.module_proc("a:::b::p", "", "return SAME", old).is_some());
            vm.push_ns_eval_token_frame(old, Vec::new());
            vm.delete_selected_namespace(old).unwrap();
            let new = declare(&mut vm, &path);
            assert_ne!(old, new, "{engine}");
            assert_eq!(
                vm.namespace_path_for_token(old),
                vm.namespace_path_for_token(new)
            );
            assert!(
                vm.module_proc("a:::b::p", "", "return SAME", new).is_none(),
                "{engine}"
            );
            assert!(
                vm.module_proc("a:::b::p", "", "return SAME", old).is_some(),
                "{engine}"
            );
            vm.pop_call_frame();
            assert!(
                vm.module_proc("a:::b::p", "", "return SAME", old).is_none(),
                "{engine}"
            );
            vm.merge_procs(&original);
            assert!(
                vm.module_proc("a:::b::p", "", "return SAME", new).is_none(),
                "{engine}"
            );
        }
    }
}
