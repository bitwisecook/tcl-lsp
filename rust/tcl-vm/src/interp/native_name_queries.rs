// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reached name queries retain their selected command and namespace identities.

use super::native_operation_currency::NativeOperationScope;
use super::{Command, CommandId, NsId, Rc, Value, Vm};
use crate::command::ProcDef;
use tcl_cmd_core::CmdError;
use tcl_runtime_api::{ProcInfo, ProcParam};
use tcl_syntax::value::ValueError;

pub(super) struct QueryNamespace {
    id: NsId,
    dying: bool,
    dead: bool,
}
impl QueryNamespace {
    pub(super) fn capture(vm: &Vm, id: NsId) -> Result<Self, CmdError> {
        if !vm.native_namespace_token_has_owner(id) {
            return Err(
                ValueError::CommandProtocolUnavailable("original query namespace owner").into(),
            );
        }
        let world = vm.name_world.borrow();
        Ok(Self {
            id,
            dying: world.dying_namespaces.contains(&id),
            dead: world.dead_namespaces.contains(&id),
        })
    }
    pub(super) fn current(&self, vm: &Vm) -> bool {
        if !vm.native_namespace_token_has_owner(self.id) {
            return false;
        }
        let world = vm.name_world.borrow();
        world.dying_namespaces.contains(&self.id) == self.dying
            && world.dead_namespaces.contains(&self.id) == self.dead
    }
}

struct ProcedureQuery {
    generation: u64,
    namespace: QueryNamespace,
    declaration: Rc<ProcDef>,
}
impl ProcedureQuery {
    fn capture(vm: &Vm, name: &[u8]) -> Result<Option<Self>, CmdError> {
        let Some(key) = vm.resolve_command_from_token_bytes(vm.current_ns_id(), name, true) else {
            return Ok(None);
        };
        let Some(Command::Proc(procedure)) = vm.visible_command_at_key(&key) else {
            return Ok(None);
        };
        let generation =
            vm.visible_command_generation(&key)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original procedure query generation",
                ))?;
        let declaration = procedure.declaration();
        let namespace = QueryNamespace::capture(vm, declaration.actual_namespace_id())?;
        Ok(Some(Self {
            generation,
            namespace,
            declaration,
        }))
    }
    fn check(&self, vm: &mut Vm, scope: &NativeOperationScope) -> Result<(), CmdError> {
        scope.check(vm)?;
        let same = vm.command_token_at_generation(self.generation)
            .and_then(|token| vm.command_at_sidecar_key(&token.key))
            .is_some_and(|command| matches!(command, Command::Proc(procedure) if Rc::ptr_eq(&procedure.declaration(), &self.declaration)));
        if !same
            || self.declaration.native_reference_count() == 0
            || self.declaration.actual_namespace_id() != self.namespace.id
            || !self.namespace.current(vm)
        {
            return Err(vm.retain_name_query_refusal("original procedure query owner retired"));
        }
        Ok(())
    }
    fn bytes(
        &self,
        vm: &mut Vm,
        scope: &NativeOperationScope,
        value: &Value,
    ) -> Result<Vec<u8>, CmdError> {
        self.check(vm, scope)?;
        let reached = vm.native_name_operand_bytes_checked(value);
        self.check(vm, scope)?;
        reached.map(|bytes| bytes.to_vec())
    }
}

impl Vm {
    pub(super) fn retain_name_query_refusal(&mut self, reason: &'static str) -> CmdError {
        let error = CmdError::from(ValueError::CommandProtocolUnavailable(reason));
        if let Some(cause) = error.native_execution_refusal() {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone()));
        }
        error
    }
    pub(crate) fn original_proc_body_query(
        &mut self,
        name: &[u8],
    ) -> Result<Option<Vec<u8>>, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        let query = ProcedureQuery::capture(self, name)?;
        let reached: Result<Option<Vec<u8>>, CmdError> = query
            .as_ref()
            .map(|query| query.bytes(self, &scope, &query.declaration.body_src))
            .transpose();
        if let Err(error) = &reached {
            if let Some(cause) = error.native_execution_refusal() {
                self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                    cause.clone(),
                ));
            }
        }
        drop(query);
        scope.check(self)?;
        reached
    }
    pub(crate) fn original_proc_info_query(
        &mut self,
        name: &[u8],
    ) -> Result<Option<ProcInfo>, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        let query = ProcedureQuery::capture(self, name)?;
        let reached: Result<Option<ProcInfo>, CmdError> = (|| {
            let Some(query) = query.as_ref() else {
                return Ok(None);
            };
            let mut params = Vec::with_capacity(query.declaration.params.len());
            for parameter in &query.declaration.params {
                let default = parameter
                    .default
                    .as_ref()
                    .map(|value| query.bytes(self, &scope, value))
                    .transpose()?;
                params.push(ProcParam {
                    name: parameter.name.as_bytes().to_vec(),
                    default,
                });
            }
            let body = query.bytes(self, &scope, &query.declaration.body_src)?;
            Ok(Some(ProcInfo { body, params }))
        })();
        if let Err(error) = &reached {
            if let Some(cause) = error.native_execution_refusal() {
                self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                    cause.clone(),
                ));
            }
        }
        drop(query);
        scope.check(self)?;
        reached
    }

    pub(crate) fn original_alias_bytes_query(
        &mut self,
        id: CommandId,
    ) -> Result<Option<Vec<Vec<u8>>>, CmdError> {
        let scope = NativeOperationScope::enter(self)?;
        let key = self
            .command_sidecar_key(id.0)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "alias prefix command token",
            ))?;
        let token =
            self.command_token_identity(&key)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original alias query generation",
                ))?;
        let namespace_id = match &key {
            super::CommandSidecarKey::Visible(name) => {
                self.command_slot(name)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original alias query slot",
                    ))?
                    .namespace
            }
            super::CommandSidecarKey::Hidden(_) => super::ROOT_NS,
        };
        let namespace = QueryNamespace::capture(self, namespace_id)?;
        let check = |vm: &mut Vm| -> Result<(), CmdError> {
            scope.check(vm)?;
            let current = vm.command_token_at_generation(token.generation);
            if current.as_ref() != Some(&token) || !namespace.current(vm) {
                return Err(vm.retain_name_query_refusal("original alias query owner retired"));
            }
            Ok(())
        };
        let command = self.command_at_sidecar_key(&key);
        enum Words {
            Direct(Vec<crate::NativeObjectLifetimeLease>),
            List(crate::NativeObjectLifetimeLease),
        }
        let originals = match command.as_ref() {
            Some(Command::Alias { words, .. }) => Some(Words::Direct(
                words.iter().map(Value::native_lifetime_lease).collect(),
            )),
            Some(Command::CallerAlias(original)) => {
                Some(Words::List(original.native_lifetime_lease()))
            }
            _ => None,
        };
        drop(command);
        let reached: Result<Option<Vec<Vec<u8>>>, CmdError> = (|| {
            check(self)?;
            let mut result = Vec::new();
            match originals.as_ref() {
                None => return Ok(None),
                Some(Words::Direct(words)) => {
                    for word in words {
                        check(self)?;
                        let reached = self.native_name_operand_bytes_checked(word.value());
                        check(self)?;
                        result.push(reached?.to_vec());
                    }
                }
                Some(Words::List(original)) => {
                    let backing = {
                        let ops = super::native_numeric_ops::OriginalNumericOps::in_original_scope(
                            self, &scope,
                        )?;
                        let reached = ops.list_with_original_check(
                            original.value(),
                            tcl_syntax::native_string::NativeStringProtocol::Jim084,
                            || ops.with_vm_for_value(original.value(), |vm| check(vm)),
                        );
                        ops.current_for_value(original.value())?;
                        reached
                    };
                    check(self)?;
                    original.value().check_native_header()?;
                    let backing = backing?;
                    for word in &*backing {
                        check(self)?;
                        backing.check_generation()?;
                        original.value().check_native_header()?;
                        let reached = self.native_name_operand_bytes_checked(word);
                        check(self)?;
                        backing.check_generation()?;
                        original.value().check_native_header()?;
                        result.push(reached?.to_vec());
                    }
                }
            }
            Ok(Some(result))
        })();
        if let Err(error) = &reached {
            if let Some(cause) = error.native_execution_refusal() {
                self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(
                    cause.clone(),
                ));
            }
        }
        drop(originals);
        scope.check(self)?;
        reached
    }
}

#[cfg(test)]
mod tests;
