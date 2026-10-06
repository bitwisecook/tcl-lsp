// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original native rename operands select actual slots and procedure holders.

use super::{CommandSlot, NativeCommandLookupUnavailable, ROOT_NS, Vm};
use tcl_core_types::{ByteCommandSlot, NameBytes};
use tcl_syntax::naming::NativeNameContext;

impl Vm {
    pub(crate) fn native_rename_reported_operand(
        &self,
        original: &[u8],
    ) -> Result<Vec<u8>, NativeCommandLookupUnavailable> {
        let policy = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?;
        if policy.recipe().is_jim084() {
            return Ok(original.to_vec());
        }
        let path = self.ns_path(self.current_ns_id());
        Ok(policy
            .recipe()
            .rename_source_input(NativeNameContext::new(&path), original)
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?
            .selected()
            .to_vec())
    }

    pub(super) fn native_rename_destination_slot(
        &self,
        original: &[u8],
    ) -> Result<ByteCommandSlot, NativeCommandLookupUnavailable> {
        let policy = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?;
        let world = self.name_world.borrow();
        let namespace = self.current_ns_id();
        let path = self.ns_path(namespace);
        let context = if policy.recipe().is_jim084() {
            let holder = world
                .jim_namespace_objects
                .get(&namespace)
                .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
            NativeNameContext::with_jim_namespace(&path, holder.as_bytes())
        } else {
            NativeNameContext::new(&path)
        };
        policy
            .recipe()
            .rename_destination_slot(context, original)
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)
    }

    pub(super) fn native_rename_source_key(
        &self,
        original: &[u8],
    ) -> Result<Option<String>, NativeCommandLookupUnavailable> {
        let policy = self
            .name_policy_protocol()
            .ok_or(NativeCommandLookupUnavailable::ProtocolUnavailable)?;
        if !policy.recipe().is_jim084() {
            let path = self.ns_path(self.current_ns_id());
            let selected = policy
                .recipe()
                .rename_source_input(NativeNameContext::new(&path), original)
                .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
            return self.resolve_command_bytes_checked(
                self.current_ns_id(),
                selected.selected(),
                true,
            );
        }
        let world = self.name_world.borrow();
        let namespace = self.current_ns_id();
        let path = self.ns_path(namespace);
        let holder = world
            .jim_namespace_objects
            .get(&namespace)
            .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let selected = policy
            .recipe()
            .rename_source_input(
                NativeNameContext::with_jim_namespace(&path, holder.as_bytes()),
                original,
            )
            .map_err(|_| NativeCommandLookupUnavailable::NamespaceContextUnavailable)?;
        let simple = NameBytes::from(
            selected
                .jim_flat_key()
                .ok_or(NativeCommandLookupUnavailable::NamespaceContextUnavailable)?,
        );
        drop(world);
        self.command_at_exact_slot_checked(
            &CommandSlot {
                namespace: ROOT_NS,
                simple,
            },
            true,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use std::rc::Rc;
    use tcl_core_types::Code;
    use tcl_syntax::value::ValueOps;

    fn actual_jim() -> Vm {
        let profile = tcl_registry::model::ingress::resolve_environment("jim").unit_profile();
        let mut vm = Vm::new();
        assert!(vm.set_native_engine_profile(profile));
        vm.set_dialect_profile(profile);
        vm.set_compiler(Box::new(
            tcl_compiler::compile_service::BytecodeCompileService::for_profile(profile),
        ));
        vm
    }

    #[test]
    fn opaque_jim_rename_moves_the_same_declaration_and_original_namespace_role() {
        let mut vm = actual_jim();
        let source = Value::from_native_string_bytes(b"old\xff".as_slice());
        let parameters = Value::list(Vec::new());
        let body = Value::from_native_string_bytes(b"".as_slice());
        assert_eq!(
            vm.try_invoke_command("proc", &[source.clone(), parameters, body])
                .unwrap()
                .code,
            Code::Ok
        );
        let declaration = vm.proc_def_bytes(b"old\xff").unwrap();
        let qualified = Value::from_native_string_bytes(b"n\xff::p\0z".as_slice());
        assert_eq!(
            vm.try_invoke_command("rename", &[source, qualified.clone()])
                .unwrap()
                .code,
            Code::Ok
        );
        let moved = vm.proc_def_bytes(b"n\xff::p\0z").unwrap();
        assert!(Rc::ptr_eq(&declaration, &moved));
        assert!(vm.proc_def_bytes(b"old\xff").is_none());
        assert_eq!(
            moved.actual_command_slot().simple.as_bytes(),
            b"n\xff::p\0z"
        );
        let namespace = moved.retained_jim_namespace().unwrap();
        assert_eq!(
            vm.native_string_bytes(&namespace).unwrap().as_ref(),
            b"n\xff"
        );
        let unqualified = Value::from_native_string_bytes(b"again\xff".as_slice());
        assert_eq!(
            vm.try_invoke_command("rename", &[qualified, unqualified.clone()])
                .unwrap()
                .code,
            Code::Ok
        );
        let again = vm.proc_def_bytes(b"again\xff").unwrap();
        assert!(Rc::ptr_eq(&declaration, &again));
        assert!(
            again
                .retained_jim_namespace()
                .unwrap()
                .is_same_object(&namespace)
        );
        assert_eq!(
            vm.try_invoke_command("rename", &[unqualified, Value::empty()])
                .unwrap()
                .code,
            Code::Ok
        );
        assert!(vm.proc_def_bytes(b"again\xff").is_none());
    }
}
