// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native declaration references are independent of Rust lifetime transports.

use super::{ProcDef, Value};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use tcl_runtime_api::native_procedure_roles::{
    NativeProcedureRoleLedger, NativeProcedureRoleOwner,
};

/// Resources held once by an actual native procedure declaration.
#[derive(Default)]
pub(crate) struct NativeProcedureResources {
    roles: NativeProcedureRoleLedger,
    objects: RefCell<Option<Vec<Value>>>,
    foreign_body: RefCell<Option<crate::compiled::CompiledUnit>>,
    statics: RefCell<Option<Rc<crate::vars::StaticVariables>>>,
    location: RefCell<Option<NativeProcedureLocation>>,
    jim_namespace: RefCell<Option<Value>>,
    compiled_names: RefCell<Option<Vec<Option<tcl_core_types::NameBytes>>>>,
}

struct NativeProcedureLocation {
    name: String,
    command_namespace: tcl_core_types::NsId,
    simple: tcl_core_types::NameBytes,
    body_namespace: tcl_core_types::ByteNamespacePath,
    body_namespace_id: tcl_core_types::NsId,
}

impl NativeProcedureResources {
    pub(crate) fn references(&self) -> usize {
        self.roles.references()
    }

    fn retire(&self) {
        let foreign_body = self.foreign_body.borrow_mut().take();
        let statics = self.statics.borrow_mut().take();
        let namespace = self.jim_namespace.borrow_mut().take();
        // Take the complete owner list before running primary free hooks:
        // a body literal may itself own a procedure reference.
        let objects = self.objects.borrow_mut().take();
        drop(self.compiled_names.borrow_mut().take());
        drop(foreign_body);
        drop(statics);
        drop(namespace);
        if let Some(objects) = objects {
            for original in objects {
                let lifetime = original.native_lifetime_lease();
                drop(original);
                lifetime.value().retire_unowned_native_header();
            }
        }
    }
}

impl NativeProcedureRoleOwner for ProcDef {
    fn native_procedure_role_ledger(&self) -> &NativeProcedureRoleLedger {
        &self.native_resources.roles
    }

    fn retire_native_procedure_resources(declaration: &Rc<Self>) {
        declaration
            .body_src
            .clear_native_procedure_bytecode_context(declaration);
        declaration.native_resources.retire();
    }
}

/// One actual Proc reference, acquired by an intrep or a running body.
/// Cloning this capsule models a native reference increment.
pub(crate) struct NativeProcedureReference {
    role: tcl_runtime_api::native_procedure_roles::NativeProcedureReference<ProcDef>,
}

impl NativeProcedureReference {
    pub(crate) fn acquire(declaration: &Rc<ProcDef>) -> Self {
        Self {
            role: tcl_runtime_api::native_procedure_roles::NativeProcedureReference::acquire(
                declaration,
            )
            .expect("live native procedure declaration"),
        }
    }

    pub(crate) fn declaration(&self) -> &Rc<ProcDef> {
        self.role.owner()
    }
}

impl Clone for NativeProcedureReference {
    fn clone(&self) -> Self {
        Self {
            role: self.role.clone(),
        }
    }
}

/// One command binding. Cloned command views share its single native role.
pub struct NativeProcedureCommand {
    declaration: Rc<ProcDef>,
    binding: tcl_runtime_api::native_procedure_roles::NativeProcedureBinding<ProcDef>,
}

impl Clone for NativeProcedureCommand {
    fn clone(&self) -> Self {
        Self {
            declaration: self.declaration(),
            binding: self.binding.clone(),
        }
    }
}

fn retain_procedure_original(original: &mut Value, objects: &mut Vec<Value>) {
    let lifetime = original.native_lifetime_lease().into_value();
    objects.push(std::mem::replace(original, lifetime).into_native_reference());
}

impl NativeProcedureCommand {
    pub(crate) fn new(mut declaration: ProcDef) -> Self {
        declaration.native_resources = Rc::new(NativeProcedureResources::default());
        *declaration.native_resources.foreign_body.borrow_mut() = declaration.body.take();
        *declaration.native_resources.statics.borrow_mut() = declaration.statics.take();
        *declaration.native_resources.location.borrow_mut() = Some(NativeProcedureLocation {
            name: declaration.name.clone(),
            command_namespace: declaration.command_ns_id,
            simple: declaration.simple_name.clone(),
            body_namespace: declaration.namespace.clone(),
            body_namespace_id: declaration.ns_id,
        });
        *declaration.native_resources.jim_namespace.borrow_mut() =
            declaration.native_jim_namespace.take();
        let mut objects = Vec::new();
        retain_procedure_original(&mut declaration.body_src, &mut objects);
        for parameter in &mut declaration.params {
            if let Some(default) = &mut parameter.default {
                retain_procedure_original(default, &mut objects);
            }
        }
        if let Some(parameters) = &mut declaration.native_parameters {
            retain_procedure_original(parameters, &mut objects);
        }
        for words in [&mut declaration.usage_name, &mut declaration.call_identity]
            .into_iter()
            .flatten()
        {
            for word in words {
                retain_procedure_original(word, &mut objects);
            }
        }
        *declaration.native_resources.objects.borrow_mut() = Some(objects);
        let declaration = Rc::new(declaration);
        let binding =
            tcl_runtime_api::native_procedure_roles::NativeProcedureBinding::new(&declaration)
                .expect("new native procedure declaration");
        Self {
            declaration,
            binding,
        }
    }

    pub(crate) fn declaration(&self) -> Rc<ProcDef> {
        self.binding
            .owner()
            .unwrap_or_else(|| Rc::clone(&self.declaration))
    }

    pub(crate) fn retire(&self) {
        self.binding.retire();
    }

    /// Transfer the new client data into the same actual command binding.
    /// All import/query transports see it through their shared binding.
    pub(crate) fn replace_declaration(&self, replacement: &Self) {
        self.binding
            .replace_from(&replacement.binding)
            .expect("new native procedure binding");
    }

    #[cfg(test)]
    pub(crate) fn relocate(
        &self,
        name: String,
        namespace: tcl_core_types::NsId,
        simple: tcl_core_types::NameBytes,
        body_namespace: tcl_core_types::ByteNamespacePath,
        jim_namespace: Option<Value>,
    ) {
        self.relocate_with_body_namespace(
            name,
            namespace,
            simple,
            body_namespace,
            namespace,
            jim_namespace,
        );
    }

    pub(crate) fn relocate_with_body_namespace(
        &self,
        name: String,
        namespace: tcl_core_types::NsId,
        simple: tcl_core_types::NameBytes,
        body_namespace: tcl_core_types::ByteNamespacePath,
        body_namespace_id: tcl_core_types::NsId,
        jim_namespace: Option<Value>,
    ) {
        let declaration = self.declaration();
        *declaration.native_resources.location.borrow_mut() = Some(NativeProcedureLocation {
            name,
            command_namespace: namespace,
            simple,
            body_namespace,
            body_namespace_id,
        });
        if declaration
            .native_resources
            .jim_namespace
            .borrow()
            .is_some()
        {
            *declaration.native_resources.jim_namespace.borrow_mut() = jim_namespace;
        }
    }
}

impl Deref for NativeProcedureCommand {
    type Target = Rc<ProcDef>;
    fn deref(&self) -> &Self::Target {
        &self.declaration
    }
}

impl ProcDef {
    pub(crate) fn native_reference_count(&self) -> usize {
        self.native_resources.references()
    }

    pub(crate) fn retained_foreign_body(&self) -> Option<crate::compiled::CompiledUnit> {
        self.native_resources.foreign_body.borrow().clone()
    }

    pub(crate) fn retained_statics(&self) -> Option<Rc<crate::vars::StaticVariables>> {
        self.native_resources.statics.borrow().clone()
    }

    pub(crate) fn retained_jim_namespace(&self) -> Option<Value> {
        self.native_resources.jim_namespace.borrow().clone()
    }

    pub(crate) fn actual_name(&self) -> String {
        self.native_resources
            .location
            .borrow()
            .as_ref()
            .map_or_else(|| self.name.clone(), |location| location.name.clone())
    }

    pub(crate) fn actual_command_slot(
        &self,
    ) -> tcl_core_types::CommandSlot<tcl_core_types::NameBytes, tcl_core_types::NsId> {
        self.native_resources
            .location
            .borrow()
            .as_ref()
            .map_or_else(
                || tcl_core_types::CommandSlot {
                    namespace: self.command_ns_id,
                    simple: self.simple_name.clone(),
                },
                |location| tcl_core_types::CommandSlot {
                    namespace: location.command_namespace,
                    simple: location.simple.clone(),
                },
            )
    }

    pub(crate) fn actual_namespace_id(&self) -> tcl_core_types::NsId {
        self.native_resources
            .location
            .borrow()
            .as_ref()
            .map_or(self.ns_id, |location| location.body_namespace_id)
    }

    pub(crate) fn actual_namespace(&self) -> tcl_core_types::ByteNamespacePath {
        self.native_resources
            .location
            .borrow()
            .as_ref()
            .map_or_else(
                || self.namespace.clone(),
                |location| location.body_namespace.clone(),
            )
    }

    /// Actual legacy recompile copies only formal records and their defaults.
    /// Native duplication of the source withdraws its Bytecode primary.
    pub(crate) fn duplicate_for_native_recompilation(
        &self,
        strings: tcl_syntax::native_string::NativeStringProtocol,
    ) -> NativeProcedureCommand {
        let mut duplicate = self.clone();
        duplicate.name = self.actual_name();
        let slot = self.actual_command_slot();
        duplicate.command_ns_id = slot.namespace;
        duplicate.simple_name = slot.simple;
        duplicate.namespace = self.actual_namespace();
        duplicate.ns_id = self.actual_namespace_id();
        duplicate.body_src = self.body_src.duplicate_native_object_in(strings);
        duplicate.body = None;
        NativeProcedureCommand::new(duplicate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Param;
    use tcl_core_types::{ByteNamespacePath, ROOT_NS};
    use tcl_registry::native_procedure::{
        NativeProcedureActivationProtocol, NativeProcedureCompilationPurpose,
        NativeProcedureRecompileAction,
    };

    fn declaration(body: Value, default: Value) -> NativeProcedureCommand {
        NativeProcedureCommand::new(ProcDef {
            native_resources: Rc::default(),
            name: "p".into(),
            command_ns_id: ROOT_NS,
            simple_name: "p".into(),
            namespace: ByteNamespacePath::root(),
            ns_id: ROOT_NS,
            params: vec![Param {
                name: "x".into(),
                default: Some(default),
            }],
            parameter_grammar: tcl_dialect::ParameterGrammar::Tcl,
            has_args: false,
            native_parameters: None,
            native_jim_namespace: None,
            native_header: tcl_dialect::NativeProcedureHeaderCompilation::Absent,

            statics: None,
            body: None,
            body_src: body,
            usage_name: None,
            call_identity: None,
        })
    }

    #[test]
    fn command_views_do_not_acquire_native_declaration_or_member_references() {
        let command = declaration(
            Value::new_native_string_bytes(b"return $x".as_slice()),
            Value::new_native_string_bytes(b"DEFAULT".as_slice()),
        );
        let proc = command.declaration();
        assert_eq!(proc.native_reference_count(), 1);
        assert_eq!(proc.body_src.native_object_reference_count(), 1);
        assert_eq!(
            proc.params[0]
                .default
                .as_ref()
                .unwrap()
                .native_object_reference_count(),
            1
        );
        let queries: Vec<_> = (0..20).map(|_| command.clone()).collect();
        let declarations: Vec<_> = queries
            .iter()
            .map(NativeProcedureCommand::declaration)
            .collect();
        assert_eq!(proc.native_reference_count(), 1);
        assert_eq!(proc.body_src.native_object_reference_count(), 1);
        assert_eq!(
            proc.params[0]
                .default
                .as_ref()
                .unwrap()
                .native_object_reference_count(),
            1
        );
        let local = NativeProcedureReference::acquire(&proc);
        let duplicate = local.clone();
        assert_eq!(proc.native_reference_count(), 3);
        drop(duplicate);
        drop(local);
        assert_eq!(proc.native_reference_count(), 1);
        drop(declarations);
        drop(queries);
        assert_eq!(proc.native_reference_count(), 1);
    }

    #[test]
    fn real_binding_retirement_releases_storage_despite_surviving_transports() {
        let member = Value::new_native_string_bytes(b"MEMBER".as_slice());
        let command = declaration(
            Value::list(vec![member.clone()]),
            Value::new_native_string_bytes(b"DEFAULT".as_slice()),
        );
        let query = command.clone();
        let proc = query.declaration();
        assert_eq!(member.native_object_reference_count(), 2);
        command.retire();
        assert_eq!(proc.native_reference_count(), 0);
        assert_eq!(proc.body_src.native_object_reference_count(), 0);
        assert!(proc.body_src.resident_string_bytes().is_none());
        assert_eq!(member.native_object_reference_count(), 1);
        // Explicit retirement is idempotent; query drops are memory-only.
        query.retire();
        drop(query);
        drop(command);
        assert_eq!(member.native_object_reference_count(), 1);
    }

    #[test]
    fn selected_legacy_replacement_updates_the_same_binding_and_retires_old_roles() {
        use tcl_dialect::TclVersion;
        use tcl_syntax::native_string::NativeStringProtocol;
        let command = declaration(
            Value::new_native_string_bytes(b"return $x".as_slice()),
            Value::new_native_string_bytes(b"DEFAULT".as_slice()),
        );
        let import = command.clone();
        let old = command.declaration();
        let native_local = NativeProcedureReference::acquire(&old);
        assert_eq!(old.native_reference_count(), 2);
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let protocol = NativeProcedureActivationProtocol::C(version);
            assert_eq!(
                protocol.recompilation_action(
                    NativeProcedureCompilationPurpose::CommandBody,
                    old.native_reference_count()
                ),
                Some(if matches!(version, TclVersion::V8_4 | TclVersion::V8_5) {
                    NativeProcedureRecompileAction::ReplaceSharedDeclaration
                } else {
                    NativeProcedureRecompileAction::RetainDeclaration
                })
            );
        }
        let replacement =
            old.duplicate_for_native_recompilation(NativeStringProtocol::C(TclVersion::V8_5));
        let fresh = replacement.declaration();
        command.replace_declaration(&replacement);
        assert!(Rc::ptr_eq(&import.declaration(), &fresh));
        assert_eq!(old.native_reference_count(), 1);
        assert_eq!(fresh.native_reference_count(), 1);
        assert!(!old.body_src.is_same_object(&fresh.body_src));
        let old_default = old.params[0].default.as_ref().unwrap();
        let new_default = fresh.params[0].default.as_ref().unwrap();
        assert!(old_default.is_same_object(new_default));
        assert_eq!(new_default.native_object_reference_count(), 2);
        drop(native_local);
        assert_eq!(old.native_reference_count(), 0);
        assert_eq!(new_default.native_object_reference_count(), 1);
    }

    #[test]
    fn relocation_preserves_declaration_and_object_roles() {
        let command = declaration(
            Value::new_native_string_bytes(b"return $x".as_slice()),
            Value::new_native_string_bytes(b"DEFAULT".as_slice()),
        );
        let old = command.declaration();
        command.relocate(
            "q".into(),
            ROOT_NS,
            "q".into(),
            ByteNamespacePath::root(),
            None,
        );
        assert!(Rc::ptr_eq(&command.declaration(), &old));
        assert_eq!(old.actual_name(), "q");
        assert_eq!(old.actual_command_slot().simple.as_bytes(), b"q");
        assert_eq!(old.native_reference_count(), 1);
        assert_eq!(old.body_src.native_object_reference_count(), 1);
        assert_eq!(
            old.params[0]
                .default
                .as_ref()
                .unwrap()
                .native_object_reference_count(),
            1
        );
    }
}

impl ProcDef {
    pub(crate) fn retain_native_compiled_names(
        &self,
        names: Vec<Option<tcl_core_types::NameBytes>>,
    ) {
        *self.native_resources.compiled_names.borrow_mut() = Some(names);
    }

    pub(crate) fn native_compiled_name(
        &self,
        index: usize,
    ) -> Result<Vec<u8>, tcl_syntax::native_string::NativeStringUnavailable> {
        let unavailable = tcl_syntax::native_string::NativeStringUnavailable::StringUpdater;
        if self.native_resources.roles.is_retired() {
            return Err(unavailable);
        }
        let names = self.native_resources.compiled_names.borrow();
        let names = names.as_ref().ok_or(unavailable)?;
        Ok(names
            .get(index)
            .and_then(Option::as_ref)
            .map_or_else(Vec::new, |name| name.as_bytes().to_vec()))
    }
}
