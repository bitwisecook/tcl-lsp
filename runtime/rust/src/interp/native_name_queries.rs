// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original procedure and alias query owners across reached String callbacks.

use super::native_operation_currency::NativeOperationScope;
use super::{Command, ImportQueryPurpose, Interp, ProcDef};
use crate::obj::{self, NativeObjectLifetime, TclObj};
use std::rc::Rc;
use tcl_cmd_core::CmdError;
use tcl_runtime_api::{ProcInfo, ProcParam};
use tcl_syntax::value::ValueError;

struct ProcedureQuery {
    generation: u64,
    declaration: Rc<ProcDef>,
    namespace: tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken,
    lifecycle: tcl_syntax::native_namespace_name::NativeNamespaceLifecycle,
}
impl ProcedureQuery {
    fn capture(interpreter: &Interp, name: &[u8]) -> Result<Option<Self>, CmdError> {
        let Some(binding) = interpreter.import_query_binding(name) else {
            return Ok(None);
        };
        let Some(target) =
            interpreter.resolve_import_query(binding, ImportQueryPurpose::Introspection)
        else {
            return Ok(None);
        };
        let Command::Proc(procedure) = target.command else {
            return Ok(None);
        };
        let declaration = procedure.declaration();
        let namespace = interpreter
            .native_namespace_token_at(declaration.namespace())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original procedure query namespace",
            ))?;
        let lifecycle = namespace.lifecycle();
        Ok(Some(Self {
            generation: target.generation,
            declaration,
            namespace,
            lifecycle,
        }))
    }
    fn check(
        &self,
        interpreter: &mut Interp,
        scope: &NativeOperationScope,
    ) -> Result<(), CmdError> {
        scope.currency().ensure_current_or_refuse()?;
        let same = interpreter.raw_command_by_generation(self.generation)
            .is_some_and(|command| matches!(command, Command::Proc(procedure) if Rc::ptr_eq(&procedure.declaration(), &self.declaration)));
        if !same
            || self.namespace.lifecycle() != self.lifecycle
            || !interpreter
                .namespaces()
                .owns_namespace_name_token(&self.namespace)
            || self.declaration.namespace() as u64 != self.namespace.token()
        {
            return Err(
                interpreter.retain_name_query_refusal("original procedure query owner retired")
            );
        }
        self.declaration
            .check_native_liveness()
            .map_err(|error| interpreter.retain_name_query_error(error.into()))
    }
    fn bytes(
        &self,
        interpreter: &mut Interp,
        scope: &NativeOperationScope,
        original: *mut TclObj,
    ) -> Result<Vec<u8>, CmdError> {
        self.check(interpreter, scope)?;
        let arguments =
            super::native_arguments::OriginalArguments::enter(interpreter, &[original])?;
        let reached = arguments.bytes(0);
        self.check(interpreter, scope)?;
        drop(arguments);
        self.check(interpreter, scope)?;
        reached.map(|bytes| bytes.to_vec())
    }
}

impl Interp {
    fn retain_name_query_error(&mut self, error: CmdError) -> CmdError {
        if let Some(cause) = error.native_execution_refusal() {
            self.refuse_native_execution(cause.clone());
        }
        error
    }
    fn retain_name_query_refusal(&mut self, reason: &'static str) -> CmdError {
        self.retain_name_query_error(ValueError::CommandProtocolUnavailable(reason).into())
    }
    pub(crate) fn original_proc_body_query(
        &mut self,
        name: &[u8],
    ) -> Result<Option<Vec<u8>>, CmdError> {
        let scope = NativeOperationScope::enter(self).map_err(CmdError::from_execution_refusal)?;
        let query = ProcedureQuery::capture(self, name)?;
        let reached: Result<Option<Vec<u8>>, CmdError> = query
            .as_ref()
            .map(|query| {
                query.check(self, &scope)?;
                query.bytes(self, &scope, query.declaration.body.checked_ptr()?)
            })
            .transpose();
        if let Err(error) = &reached {
            self.retain_name_query_error(error.clone());
        }
        drop(query);
        scope.currency().ensure_current_or_refuse()?;
        reached
    }
    pub(crate) fn original_proc_info_query(
        &mut self,
        name: &[u8],
    ) -> Result<Option<ProcInfo>, CmdError> {
        let scope = NativeOperationScope::enter(self).map_err(CmdError::from_execution_refusal)?;
        let query = ProcedureQuery::capture(self, name)?;
        let reached: Result<Option<ProcInfo>, CmdError> = (|| {
            let Some(query) = query.as_ref() else {
                return Ok(None);
            };
            let mut params = Vec::with_capacity(query.declaration.params.len());
            for parameter in &query.declaration.params {
                query.check(self, &scope)?;
                let default = parameter
                    .default
                    .as_ref()
                    .map(|value| query.bytes(self, &scope, value.checked_ptr()?))
                    .transpose()?;
                params.push(ProcParam {
                    name: parameter.name.clone(),
                    default,
                });
            }
            query.check(self, &scope)?;
            let body = query.bytes(self, &scope, query.declaration.body.checked_ptr()?)?;
            Ok(Some(ProcInfo { body, params }))
        })();
        if let Err(error) = &reached {
            self.retain_name_query_error(error.clone());
        }
        drop(query);
        scope.currency().ensure_current_or_refuse()?;
        reached
    }
    pub(crate) fn original_alias_bytes_query(
        &mut self,
        id: u32,
    ) -> Result<Option<Vec<Vec<u8>>>, CmdError> {
        let scope = NativeOperationScope::enter(self).map_err(CmdError::from_execution_refusal)?;
        let (_, generation) =
            self.command_identity(id)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "alias prefix command token",
                ))?;
        let namespace_id = self
            .namespaces()
            .native_command_slot_at_node(generation)
            .map(|(namespace, _)| namespace)
            .or_else(|| {
                self.hidden
                    .borrow()
                    .values()
                    .any(|binding| binding.generation == generation)
                    .then_some(crate::namespace::GLOBAL)
            })
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original alias query namespace",
            ))?;
        let namespace = self.native_namespace_token_at(namespace_id).ok_or(
            ValueError::CommandProtocolUnavailable("original alias query namespace owner"),
        )?;
        let lifecycle = namespace.lifecycle();
        let command = self.raw_command_by_generation(generation);
        let Some(Command::Alias {
            original_prefix, ..
        }) = command.as_ref()
        else {
            return Ok(None);
        };
        let original = NativeObjectLifetime::retain(original_prefix.as_ptr());
        drop(command);
        let check = |interpreter: &mut Interp| -> Result<(), CmdError> {
            scope.currency().ensure_current_or_refuse()?;
            obj::check_native_liveness(original.as_ptr())?;
            let same = interpreter.raw_command_by_generation(generation).is_some_and(|command| matches!(command, Command::Alias { original_prefix, .. } if original_prefix.as_ptr() == original.as_ptr()));
            if !same
                || namespace.lifecycle() != lifecycle
                || !interpreter
                    .namespaces()
                    .owns_namespace_name_token(&namespace)
                || interpreter
                    .namespaces()
                    .native_command_slot_at_node(generation)
                    .is_some_and(|(current, _)| current != namespace_id)
            {
                return Err(
                    interpreter.retain_name_query_refusal("original alias query owner retired")
                );
            }
            Ok(())
        };
        let reached: Result<Option<Vec<Vec<u8>>>, CmdError> = (|| {
            check(self)?;
            let protocol = self
                .native_invocation_dialect()
                .native_string_protocol()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "original alias List protocol",
                ))?;
            let members = crate::list::list_elements_native_with_currency(
                original.as_ptr(),
                protocol,
                Some(scope.currency()),
            );
            check(self)?;
            let members = members?;
            let backing = crate::list::native_list_backing(original.as_ptr()).ok_or(
                ValueError::CommandProtocolUnavailable("original alias List backing"),
            )?;
            let members = members
                .into_iter()
                .map(|member| {
                    obj::check_native_liveness(member)?;
                    Ok(NativeObjectLifetime::retain(member))
                })
                .collect::<Result<Vec<_>, ValueError>>()?;
            let mut result = Vec::with_capacity(members.len());
            for member in &members {
                backing.check_generation()?;
                check(self)?;
                let arguments =
                    super::native_arguments::OriginalArguments::enter(self, &[member.as_ptr()])?;
                let reached = arguments.bytes(0);
                check(self)?;
                drop(arguments);
                check(self)?;
                backing.check_generation()?;
                result.push(reached?.to_vec());
            }
            Ok(Some(result))
        })();
        if let Err(error) = &reached {
            self.retain_name_query_error(error.clone());
        }
        drop(original);
        scope.currency().ensure_current_or_refuse()?;
        reached
    }
}

#[cfg(test)]
mod tests;
